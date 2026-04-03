use anyhow::{Context, Result};
use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts},
    error::ResolveErrorKind,
    TokioAsyncResolver,
};
use std::{net::IpAddr, time::Duration};
use tokio::time::timeout;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a Hickory async resolver with the given timeout applied both to
/// individual queries (`opts.timeout`) and the number of retry attempts.
fn build_resolver(timeout_ms: u64) -> TokioAsyncResolver {
    let mut opts = ResolverOpts::default();
    // Per-query timeout (each attempt).
    opts.timeout = Duration::from_millis(timeout_ms);
    // Two attempts before giving up (total wall-clock ≤ 2 × timeout_ms).
    opts.attempts = 2;
    TokioAsyncResolver::tokio(ResolverConfig::default(), opts)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Resolve a hostname to its first A or AAAA IP address.
///
/// The lookup is bounded by `timeout_ms` milliseconds at the Tokio level
/// (independent of the per-attempt timeout configured in the resolver).
/// Returns an error if:
/// - `timeout_ms` elapses before any response arrives,
/// - the name does not resolve,
/// - the DNS server returns an error.
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
                // PTR names from the wire end with a trailing dot — remove it.
                record.0.to_string().trim_end_matches('.').to_owned()
            });
            Ok(name)
        }
        Err(e) => match e.kind() {
            // NXDOMAIN or empty answer — no PTR record exists, not an error.
            ResolveErrorKind::NoRecordsFound { .. } => Ok(None),
            // Any other error (timeout, SERVFAIL, …) is propagated.
            _ => Err(anyhow::Error::new(e)
                .context(format!("DNS reverse lookup failed for '{ip}'"))),
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: u64 = 3000;

    #[tokio::test]
    async fn test_resolve_known_hostname() {
        // dns.google is Google's public DNS — extremely stable.
        let ip = resolve_hostname("dns.google", TIMEOUT).await.unwrap();
        // Should return one of 8.8.8.8 / 8.8.4.4 (or IPv6 equivalent).
        assert!(!ip.is_empty(), "Expected a non-empty IP address");
    }

    #[tokio::test]
    async fn test_resolve_invalid_hostname_returns_error() {
        let result = resolve_hostname("this-host-does-not-exist.invalid", TIMEOUT).await;
        assert!(result.is_err(), "Expected an error for a non-existent hostname");
    }

    #[tokio::test]
    async fn test_reverse_lookup_google_dns() {
        // 8.8.8.8 has a well-known PTR record: dns.google.
        let ptr = reverse_lookup("8.8.8.8", TIMEOUT).await.unwrap();
        assert_eq!(ptr.as_deref(), Some("dns.google"));
    }

    #[tokio::test]
    async fn test_reverse_lookup_no_ptr_returns_none() {
        // RFC 5737 documentation address — guaranteed to have no PTR record.
        let ptr = reverse_lookup("192.0.2.1", TIMEOUT).await.unwrap();
        assert!(ptr.is_none(), "Expected None for an IP with no PTR record");
    }

    #[tokio::test]
    async fn test_reverse_lookup_invalid_ip_returns_error() {
        let result = reverse_lookup("not-an-ip", TIMEOUT).await;
        assert!(result.is_err(), "Expected an error for an invalid IP string");
    }
}
