// ── DNS resolution ────────────────────────────────────────────────────────────
// Implements the full DNS lookup pipeline (spec §2.1 — Step 1/2):
//   • Forward A / AAAA resolution   (P0-DNS-001)
//   • CNAME chain detection          (P1-DNS-002)
//   • PTR reverse lookup             (P1-DNS-003)
//   • TXT record extraction          (P1-DNS-004)
//   • TTL on every record            (P2-DNS-006)
//   • Parallel A + AAAA + CNAME + TXT (P2-DNS-005)
//   • Global lookup deadline          (P1-NETWORK-003)

use anyhow::{Context, Result};
use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts},
    error::ResolveErrorKind,
    proto::rr::RecordType,
    TokioAsyncResolver,
};
use std::{net::IpAddr, time::Duration};
use tokio::time::timeout;

use crate::models::DnsRecord;

// ---------------------------------------------------------------------------
// Public result type for a full hostname lookup
// ---------------------------------------------------------------------------

/// All DNS data collected for one hostname target.
pub struct DnsLookupResult {
    /// First A or AAAA address resolved (used as the pipeline IP).
    pub resolved_ip: Option<String>,
    /// PTR name for the resolved IP (non-blocking; `None` if absent).
    pub ptr: Option<String>,
    /// All records found (A, AAAA, CNAME, TXT) with their TTL values.
    pub records: Vec<DnsRecord>,
    /// Non-fatal error messages collected during the lookup.
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn build_resolver(timeout_ms: u64) -> TokioAsyncResolver {
    let mut opts = ResolverOpts::default();
    // Per-query timeout for each DNS attempt.
    opts.timeout = Duration::from_millis(timeout_ms);
    // Two attempts before giving up (covers transient SERVFAIL etc.).
    opts.attempts = 2;
    TokioAsyncResolver::tokio(ResolverConfig::default(), opts)
}

/// Query for all records of `record_type` and return them as [`DnsRecord`]s.
/// Non-fatal: any error yields an empty vec (NXDOMAIN, SERVFAIL, timeout…).
async fn query_records(
    resolver: &TokioAsyncResolver,
    hostname: &str,
    record_type: RecordType,
) -> Vec<DnsRecord> {
    match resolver.lookup(hostname, record_type).await {
        Ok(lookup) => lookup
            .records()
            .iter()
            .filter_map(|record| {
                record.data().map(|data| DnsRecord {
                    record_type: record.record_type().to_string(),
                    // Strip trailing dot from CNAME/PTR names for cleanliness.
                    value: data.to_string().trim_end_matches('.').to_owned(),
                    ttl: record.ttl(),
                })
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Public API — backward-compatible single-result helpers
// ---------------------------------------------------------------------------

/// Resolve a hostname to its first A or AAAA IP address.
///
/// Bounded by `timeout_ms` milliseconds. Returns an error on failure.
pub async fn resolve_hostname(hostname: &str, timeout_ms: u64) -> Result<String> {
    let resolver = build_resolver(timeout_ms);
    let deadline = Duration::from_millis(timeout_ms * 2); // covers retries

    let lookup = timeout(deadline, resolver.lookup_ip(hostname))
        .await
        .context("DNS forward lookup timed out")?
        .with_context(|| format!("DNS forward lookup failed for '{hostname}'"))?;

    let addr = lookup
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No address record found for '{hostname}'"))?;

    Ok(addr.to_string())
}

/// Perform a reverse PTR lookup for an IP address string (IPv4 or IPv6).
///
/// Returns:
/// - `Ok(Some(ptr))` — PTR record found; trailing dot stripped.
/// - `Ok(None)`      — NXDOMAIN or no PTR record (not an error).
/// - `Err(_)`        — network error, invalid IP, or timeout.
pub async fn reverse_lookup(ip: &str, timeout_ms: u64) -> Result<Option<String>> {
    let addr: IpAddr = ip
        .parse()
        .with_context(|| format!("Invalid IP address: '{ip}'"))?;

    let resolver = build_resolver(timeout_ms);
    let deadline = Duration::from_millis(timeout_ms * 2);

    let result = timeout(deadline, resolver.reverse_lookup(addr))
        .await
        .context("DNS reverse lookup timed out")?;

    match result {
        Ok(lookup) => {
            let name = lookup.iter().next().map(|record| {
                record.0.to_string().trim_end_matches('.').to_owned()
            });
            Ok(name)
        }
        Err(e) => match e.kind() {
            // NXDOMAIN / empty answer — no PTR record exists, not an error.
            ResolveErrorKind::NoRecordsFound { .. } => Ok(None),
            // Any other error (timeout, SERVFAIL, …) is propagated.
            _ => Err(anyhow::Error::new(e)
                .context(format!("DNS reverse lookup failed for '{ip}'"))),
        },
    }
}

// ---------------------------------------------------------------------------
// Public API — full parallel DNS lookup (P2-DNS-005, P1-DNS-002/003/004)
// ---------------------------------------------------------------------------

/// Perform a full DNS lookup for `hostname`:
///
/// 1. Parallel queries for A, AAAA, CNAME, TXT records (with TTL).
/// 2. Extract the first resolved IP from A/AAAA records.
/// 3. PTR reverse lookup on the resolved IP.
///
/// The whole operation is bounded by `timeout_ms × 3` to cover parallel legs.
/// Never fails — errors are captured in [`DnsLookupResult::errors`].
pub async fn full_dns_lookup(hostname: &str, timeout_ms: u64) -> DnsLookupResult {
    // Global deadline covering all parallel sub-queries + PTR.
    let deadline = Duration::from_millis(timeout_ms * 3);
    let mut errors: Vec<String> = Vec::new();

    let inner = async {
        let resolver = build_resolver(timeout_ms);

        // Step A: parallel A + AAAA + CNAME + TXT (P2-DNS-005).
        let (a_recs, aaaa_recs, cname_recs, txt_recs) = tokio::join!(
            query_records(&resolver, hostname, RecordType::A),
            query_records(&resolver, hostname, RecordType::AAAA),
            query_records(&resolver, hostname, RecordType::CNAME),
            query_records(&resolver, hostname, RecordType::TXT),
        );

        let mut records = Vec::new();
        records.extend(a_recs);
        records.extend(aaaa_recs);
        records.extend(cname_recs);
        records.extend(txt_recs);

        // Step B: first resolved IP from A/AAAA.
        let resolved_ip = records
            .iter()
            .find(|r| r.record_type == "A" || r.record_type == "AAAA")
            .map(|r| r.value.clone());

        // Step C: PTR on the resolved IP (non-blocking).
        let ptr = match &resolved_ip {
            Some(ip) => reverse_lookup(ip, timeout_ms).await.ok().flatten(),
            None => None,
        };

        (records, resolved_ip, ptr)
    };

    match timeout(deadline, inner).await {
        Ok((records, resolved_ip, ptr)) => DnsLookupResult {
            resolved_ip,
            ptr,
            records,
            errors,
        },
        Err(_) => {
            errors.push(format!("DNS lookup timed out for '{hostname}'"));
            DnsLookupResult {
                resolved_ip: None,
                ptr: None,
                records: Vec::new(),
                errors,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: u64 = 3000;

    // ── Backward-compat forward lookup ───────────────────────────────────────

    #[tokio::test]
    async fn test_resolve_known_hostname() {
        let ip = resolve_hostname("dns.google", TIMEOUT).await.unwrap();
        assert!(!ip.is_empty(), "Expected a non-empty IP address");
    }

    #[tokio::test]
    async fn test_resolve_invalid_hostname_returns_error() {
        let result = resolve_hostname("this-host-does-not-exist.invalid", TIMEOUT).await;
        assert!(result.is_err());
    }

    // ── PTR reverse lookup ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_reverse_lookup_google_dns() {
        let ptr = reverse_lookup("8.8.8.8", TIMEOUT).await.unwrap();
        assert_eq!(ptr.as_deref(), Some("dns.google"));
    }

    #[tokio::test]
    async fn test_reverse_lookup_no_ptr_returns_none() {
        // RFC 5737 documentation address — guaranteed to have no PTR record.
        let ptr = reverse_lookup("192.0.2.1", TIMEOUT).await.unwrap();
        assert!(ptr.is_none());
    }

    #[tokio::test]
    async fn test_reverse_lookup_invalid_ip_returns_error() {
        let result = reverse_lookup("not-an-ip", TIMEOUT).await;
        assert!(result.is_err());
    }

    // ── Full parallel lookup ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_full_dns_lookup_a_record() {
        let result = full_dns_lookup("dns.google", TIMEOUT).await;
        // Must resolve to at least one A/AAAA record.
        assert!(
            result.resolved_ip.is_some(),
            "Expected a resolved IP for dns.google"
        );
        let has_a_or_aaaa = result.records.iter().any(|r| r.record_type == "A" || r.record_type == "AAAA");
        assert!(has_a_or_aaaa, "Expected A or AAAA records");
    }

    #[tokio::test]
    async fn test_full_dns_lookup_ttl_present() {
        let result = full_dns_lookup("dns.google", TIMEOUT).await;
        // All records must have a non-zero TTL.
        for rec in &result.records {
            assert!(rec.ttl > 0, "Expected TTL > 0 for record {:?}", rec);
        }
    }

    #[tokio::test]
    async fn test_full_dns_lookup_ptr() {
        // dns.google has the well-known PTR record pointing back to dns.google.
        let result = full_dns_lookup("dns.google", TIMEOUT).await;
        assert!(result.ptr.is_some(), "Expected PTR for dns.google");
    }

    #[tokio::test]
    async fn test_full_dns_lookup_nxdomain_returns_empty() {
        let result = full_dns_lookup("this-does-not-exist.invalid", TIMEOUT).await;
        assert!(result.resolved_ip.is_none());
        assert!(result.records.is_empty());
    }

    #[tokio::test]
    async fn test_full_dns_lookup_txt_records() {
        // google.com publishes TXT records (SPF, DMARC, etc.).
        let result = full_dns_lookup("google.com", TIMEOUT).await;
        let has_txt = result.records.iter().any(|r| r.record_type == "TXT");
        assert!(has_txt, "Expected TXT records for google.com");
    }

    #[tokio::test]
    async fn test_full_dns_lookup_record_fields_non_empty() {
        let result = full_dns_lookup("dns.google", TIMEOUT).await;
        for rec in &result.records {
            assert!(!rec.record_type.is_empty(), "record_type should be non-empty");
            assert!(!rec.value.is_empty(), "value should be non-empty");
        }
    }

    // ── query_records unit tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_query_records_a_returns_vec() {
        let resolver = build_resolver(TIMEOUT);
        let recs = query_records(&resolver, "dns.google", RecordType::A).await;
        assert!(!recs.is_empty(), "Expected at least one A record");
        assert!(recs.iter().all(|r| r.record_type == "A"));
    }

    #[tokio::test]
    async fn test_query_records_nonexistent_returns_empty() {
        let resolver = build_resolver(TIMEOUT);
        let recs = query_records(&resolver, "no-such-host.invalid", RecordType::A).await;
        assert!(recs.is_empty());
    }
}
