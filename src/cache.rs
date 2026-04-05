//! TTL-based in-memory lookup cache (P0-PERF-004, P3-PERF-018).
//!
//! # Design
//! - Keys are IP addresses or hostnames, normalised to lowercase ASCII.
//! - Entries expire after `ttl`.  Expired entries are not returned; they are
//!   lazily evicted on the next `insert()` so no background task is needed.
//! - The handle is wrapped in `Arc` — cheap to clone and share across handlers.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;
use tracing::debug;

use crate::models::IpRecord;

// ---------------------------------------------------------------------------
// Internal entry
// ---------------------------------------------------------------------------

struct CacheEntry {
    record:      IpRecord,
    inserted_at: Instant,
}

// ---------------------------------------------------------------------------
// Public handle
// ---------------------------------------------------------------------------

/// Shared TTL cache for `IpRecord` lookups.
///
/// Cheap to clone — all state lives behind `Arc<RwLock<…>>`.
#[derive(Clone)]
pub struct LookupCache {
    inner: Arc<RwLock<HashMap<String, CacheEntry>>>,
    ttl:   Duration,
}

impl LookupCache {
    /// Create a new cache with the given `ttl`.
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Look up `key`.
    ///
    /// Returns `None` when the entry does not exist **or** has expired.
    pub async fn get(&self, key: &str) -> Option<IpRecord> {
        let key = key.to_ascii_lowercase();
        let map = self.inner.read().await;
        if let Some(entry) = map.get(&key) {
            if entry.inserted_at.elapsed() < self.ttl {
                debug!(target: "atlasip::cache", key = %key, "cache hit");
                return Some(entry.record.clone());
            }
            debug!(target: "atlasip::cache", key = %key, "cache stale (expired)");
        }
        None
    }

    /// Store `record` under `key`.
    ///
    /// All expired entries are evicted as a side-effect so the map does not
    /// grow unboundedly over long sessions.
    pub async fn insert(&self, key: &str, record: IpRecord) {
        let key = key.to_ascii_lowercase();
        let mut map = self.inner.write().await;

        // Lazy eviction: drop every expired entry before inserting a new one.
        let ttl = self.ttl;
        map.retain(|_, v| v.inserted_at.elapsed() < ttl);

        debug!(target: "atlasip::cache", key = %key, "cache insert");
        map.insert(key, CacheEntry { record, inserted_at: Instant::now() });
    }

    /// Return the number of non-expired entries currently held.
    pub async fn len(&self) -> usize {
        let ttl = self.ttl;
        self.inner
            .read()
            .await
            .values()
            .filter(|e| e.inserted_at.elapsed() < ttl)
            .count()
    }

    /// `true` when no non-expired entries are held.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::IpRecord;

    fn record(ip: &str) -> IpRecord {
        IpRecord::new(1, ip)
    }

    // ── Hit / miss ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_miss() {
        let cache = LookupCache::new(Duration::from_secs(3600));
        assert!(cache.get("8.8.8.8").await.is_none());
    }

    #[tokio::test]
    async fn test_insert_then_hit() {
        let cache = LookupCache::new(Duration::from_secs(3600));
        cache.insert("8.8.8.8", record("8.8.8.8")).await;
        let hit = cache.get("8.8.8.8").await;
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().ip, "8.8.8.8");
    }

    // ── TTL expiry ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_ttl_expired_returns_none() {
        // TTL = 1 ms — immediately expired.
        let cache = LookupCache::new(Duration::from_millis(1));
        cache.insert("1.1.1.1", record("1.1.1.1")).await;
        // Sleep longer than TTL to guarantee expiry.
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(cache.get("1.1.1.1").await.is_none());
    }

    #[tokio::test]
    async fn test_ttl_not_expired_returns_record() {
        let cache = LookupCache::new(Duration::from_secs(60));
        cache.insert("9.9.9.9", record("9.9.9.9")).await;
        assert!(cache.get("9.9.9.9").await.is_some());
    }

    // ── Key normalisation ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_key_case_insensitive() {
        let cache = LookupCache::new(Duration::from_secs(3600));
        cache.insert("Google.Com", record("google.com")).await;
        assert!(cache.get("google.com").await.is_some());
        assert!(cache.get("GOOGLE.COM").await.is_some());
    }

    // ── Len / is_empty ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_len_counts_live_entries_only() {
        // One short TTL entry + one long TTL would require two caches.
        // Instead, verify len after insert and after expiry.
        let cache = LookupCache::new(Duration::from_millis(5));
        assert_eq!(cache.len().await, 0);
        cache.insert("a", record("1.2.3.4")).await;
        assert_eq!(cache.len().await, 1);
        tokio::time::sleep(Duration::from_millis(10)).await;
        // Entry is now expired — len must return 0.
        assert_eq!(cache.len().await, 0);
    }

    // ── Multiple IPs ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_multiple_entries() {
        let cache = LookupCache::new(Duration::from_secs(3600));
        for i in 0..10u8 {
            cache.insert(&format!("10.0.0.{i}"), record(&format!("10.0.0.{i}"))).await;
        }
        assert_eq!(cache.len().await, 10);
        for i in 0..10u8 {
            assert!(cache.get(&format!("10.0.0.{i}")).await.is_some());
        }
    }

    // ── Lazy eviction on insert ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_lazy_eviction_on_insert() {
        let cache = LookupCache::new(Duration::from_millis(5));
        cache.insert("old", record("1.1.1.1")).await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        // This insert must evict the expired "old" entry.
        cache.insert("new", record("2.2.2.2")).await;
        // "old" is gone even though we only inserted "new".
        assert!(cache.get("old").await.is_none());
        assert!(cache.get("new").await.is_some());
    }
}
