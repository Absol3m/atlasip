//! Per-source request metrics (P3-PERF-016).
//!
//! Tracks latency, total requests, errors, and fallbacks for each lookup source
//! (RDAP, WHOIS, DNS).  All counters use `AtomicU64` so recording is lock-free
//! and never blocks an async task.
//!
//! The `MetricsSnapshot` type is `Serialize` and is returned by `GET /metrics`.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use serde::Serialize;

// ---------------------------------------------------------------------------
// Per-source counters
// ---------------------------------------------------------------------------

struct SourceCounters {
    /// Total number of requests issued.
    requests: AtomicU64,
    /// Number of requests that produced an error.
    errors: AtomicU64,
    /// Accumulated latency in **microseconds** across all requests.
    /// Divide by `requests` to get the mean.
    latency_us: AtomicU64,
}

impl SourceCounters {
    fn new() -> Self {
        Self {
            requests:   AtomicU64::new(0),
            errors:     AtomicU64::new(0),
            latency_us: AtomicU64::new(0),
        }
    }

    /// Record one completed request.
    ///
    /// * `latency_us` — wall-clock duration of the request in microseconds.
    /// * `ok`         — `true` if the request succeeded.
    fn record(&self, latency_us: u64, ok: bool) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.latency_us.fetch_add(latency_us, Ordering::Relaxed);
        if !ok {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn snapshot(&self) -> SourceSnapshot {
        let requests   = self.requests.load(Ordering::Relaxed);
        let errors     = self.errors.load(Ordering::Relaxed);
        let latency_us = self.latency_us.load(Ordering::Relaxed);
        SourceSnapshot {
            requests,
            errors,
            error_rate: if requests > 0 {
                errors as f64 / requests as f64
            } else {
                0.0
            },
            mean_latency_ms: if requests > 0 {
                // Convert μs → ms.
                latency_us as f64 / requests as f64 / 1_000.0
            } else {
                0.0
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Public handle
// ---------------------------------------------------------------------------

/// Shared metrics handle.  Cheap to clone — all state lives behind `Arc`.
#[derive(Clone)]
pub struct RequestMetrics {
    inner: Arc<Inner>,
}

struct Inner {
    rdap:      SourceCounters,
    whois:     SourceCounters,
    dns:       SourceCounters,
    /// How many lookups fell back to WHOIS as the primary source (RDAP failed).
    fallbacks:  AtomicU64,
    /// How many lookup responses were served from the TTL cache.
    cache_hits: AtomicU64,
}

impl RequestMetrics {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                rdap:       SourceCounters::new(),
                whois:      SourceCounters::new(),
                dns:        SourceCounters::new(),
                fallbacks:  AtomicU64::new(0),
                cache_hits: AtomicU64::new(0),
            }),
        }
    }

    /// Record one RDAP request.
    pub fn record_rdap(&self, latency_us: u64, ok: bool) {
        self.inner.rdap.record(latency_us, ok);
    }

    /// Record one WHOIS request.
    pub fn record_whois(&self, latency_us: u64, ok: bool) {
        self.inner.whois.record(latency_us, ok);
    }

    /// Record one DNS request (or batch of queries for a single hostname).
    pub fn record_dns(&self, latency_us: u64, ok: bool) {
        self.inner.dns.record(latency_us, ok);
    }

    /// Record that WHOIS was used as the primary source because RDAP failed.
    pub fn record_fallback(&self) {
        self.inner.fallbacks.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a cache hit (result served without any network request).
    pub fn record_cache_hit(&self) {
        self.inner.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Return a point-in-time snapshot of all counters.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            rdap:       self.inner.rdap.snapshot(),
            whois:      self.inner.whois.snapshot(),
            dns:        self.inner.dns.snapshot(),
            fallbacks:  self.inner.fallbacks.load(Ordering::Relaxed),
            cache_hits: self.inner.cache_hits.load(Ordering::Relaxed),
        }
    }
}

// ---------------------------------------------------------------------------
// Serialisable snapshot (returned by GET /metrics)
// ---------------------------------------------------------------------------

/// Metrics for a single lookup source.
#[derive(Debug, Serialize)]
pub struct SourceSnapshot {
    /// Total requests made since startup.
    pub requests: u64,
    /// Requests that produced an error.
    pub errors: u64,
    /// `errors / requests` (0.0 when no requests have been made).
    pub error_rate: f64,
    /// Mean wall-clock latency in milliseconds.
    pub mean_latency_ms: f64,
}

/// Full metrics snapshot for all sources.
#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub rdap:       SourceSnapshot,
    pub whois:      SourceSnapshot,
    pub dns:        SourceSnapshot,
    /// Count of lookups where RDAP failed and WHOIS was used as primary.
    pub fallbacks:  u64,
    /// Count of lookups served directly from the TTL cache.
    pub cache_hits: u64,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_snapshot_is_zero() {
        let m = RequestMetrics::new();
        let s = m.snapshot();
        assert_eq!(s.rdap.requests, 0);
        assert_eq!(s.rdap.errors, 0);
        assert_eq!(s.rdap.error_rate, 0.0);
        assert_eq!(s.rdap.mean_latency_ms, 0.0);
        assert_eq!(s.fallbacks, 0);
        assert_eq!(s.cache_hits, 0);
    }

    #[test]
    fn test_record_rdap_success() {
        let m = RequestMetrics::new();
        m.record_rdap(50_000, true); // 50 ms
        let s = m.snapshot();
        assert_eq!(s.rdap.requests, 1);
        assert_eq!(s.rdap.errors, 0);
        assert_eq!(s.rdap.error_rate, 0.0);
        assert!((s.rdap.mean_latency_ms - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_record_rdap_error() {
        let m = RequestMetrics::new();
        m.record_rdap(10_000, false);
        let s = m.snapshot();
        assert_eq!(s.rdap.errors, 1);
        assert_eq!(s.rdap.error_rate, 1.0);
    }

    #[test]
    fn test_error_rate_partial() {
        let m = RequestMetrics::new();
        m.record_whois(1_000, true);
        m.record_whois(1_000, false);
        m.record_whois(1_000, true);
        m.record_whois(1_000, false);
        let s = m.snapshot();
        assert_eq!(s.whois.requests, 4);
        assert_eq!(s.whois.errors, 2);
        assert!((s.whois.error_rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_mean_latency_ms() {
        let m = RequestMetrics::new();
        // 100 ms + 200 ms → mean 150 ms
        m.record_dns(100_000, true);
        m.record_dns(200_000, true);
        let s = m.snapshot();
        assert!((s.dns.mean_latency_ms - 150.0).abs() < 0.1);
    }

    #[test]
    fn test_fallback_and_cache_hit_counters() {
        let m = RequestMetrics::new();
        m.record_fallback();
        m.record_fallback();
        m.record_cache_hit();
        let s = m.snapshot();
        assert_eq!(s.fallbacks, 2);
        assert_eq!(s.cache_hits, 1);
    }

    #[test]
    fn test_clone_shares_state() {
        let m1 = RequestMetrics::new();
        let m2 = m1.clone();
        m1.record_rdap(5_000, true);
        // m2 points to the same Arc — should see the update.
        assert_eq!(m2.snapshot().rdap.requests, 1);
    }
}
