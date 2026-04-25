// ── DNS resolution ────────────────────────────────────────────────────────────
// Implements the full DNS lookup pipeline (spec §2.1 — Step 1/2):
//   • Forward A / AAAA / CNAME / TXT / MX / NS / SOA   (P0-DNS-001, #14)
//   • PTR reverse lookup                                 (P1-DNS-003)
//   • TTL on every record                                (P2-DNS-006)
//   • Parallel queries                                   (P2-DNS-005)
//   • DNSSEC validation status via DoH AD flag           (#14)
//   • DNS-over-TLS transport                             (#14)
//   • Global lookup deadline                             (P1-NETWORK-003)
//   • DNS-over-HTTPS reverse lookup                      (spec §4)
//   • Intelligent fallback (System → DoH)                (spec §5)

use anyhow::{Context, Result};
use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts, CLOUDFLARE, GOOGLE, QUAD9},
    net::runtime::TokioRuntimeProvider,
    proto::rr::{RData, RecordType},
    TokioResolver,
};
use std::{net::IpAddr, time::Duration};
use tokio::time::timeout;

use crate::{config::DnsMode, models::DnsRecord};

// ---------------------------------------------------------------------------
// DoH response structures
// ---------------------------------------------------------------------------

/// Top-level Cloudflare / RFC-8484 JSON DNS response.
#[derive(Debug, serde::Deserialize)]
struct DohResponse {
    #[serde(rename = "Answer")]
    answer: Option<Vec<DohAnswer>>,
    /// Authenticated Data bit — true when DNSSEC validation passed.
    #[serde(rename = "AD", default)]
    ad: bool,
}

/// Individual answer record in a DoH JSON response.
#[derive(Debug, serde::Deserialize)]
struct DohAnswer {
    /// Raw string value of the record (PTR name with trailing dot).
    data: String,
}

// ---------------------------------------------------------------------------
// DoH helpers
// ---------------------------------------------------------------------------

/// Convert an IP address string to its PTR query name.
fn ip_to_ptr_name(ip: &str) -> Option<String> {
    let addr: IpAddr = ip.parse().ok()?;
    match addr {
        IpAddr::V4(v4) => {
            let [a, b, c, d] = v4.octets();
            Some(format!("{d}.{c}.{b}.{a}.in-addr.arpa"))
        }
        IpAddr::V6(v6) => {
            let hex: String = v6.octets().iter().map(|b| format!("{b:02x}")).collect();
            let reversed: String = hex
                .chars()
                .rev()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(".");
            Some(format!("{reversed}.ip6.arpa"))
        }
    }
}

// ---------------------------------------------------------------------------
// Resolver builders
// ---------------------------------------------------------------------------

fn build_resolver(timeout_ms: u64) -> TokioResolver {
    let mut opts = ResolverOpts::default();
    opts.timeout  = Duration::from_millis(timeout_ms);
    opts.attempts = 2;
    // Try system DNS first; fall back to Cloudflare when the system config
    // contains entries hickory cannot parse (e.g. macOS scoped IPv6 link-local
    // addresses like fe80::1%en0).
    let builder = TokioResolver::builder_tokio().unwrap_or_else(|_| {
        tracing::warn!(
            "system DNS config could not be parsed (scoped IPv6?); \
             falling back to Cloudflare 1.1.1.1"
        );
        TokioResolver::builder_with_config(
            ResolverConfig::udp_and_tcp(&CLOUDFLARE),
            TokioRuntimeProvider::default(),
        )
    });
    builder
        .with_options(opts)
        .build()
        .expect("failed to build DNS resolver")
}

/// Build a DNS-over-TLS resolver for the named server.
/// Accepted values: `"cloudflare"`, `"google"`, `"quad9"` (default: cloudflare).
fn build_dot_resolver(server: &str, timeout_ms: u64) -> TokioResolver {
    let config = match server {
        "google" => ResolverConfig::tls(&GOOGLE),
        "quad9"  => ResolverConfig::tls(&QUAD9),
        _        => ResolverConfig::tls(&CLOUDFLARE),
    };
    let mut opts = ResolverOpts::default();
    opts.timeout  = Duration::from_millis(timeout_ms);
    opts.attempts = 2;
    TokioResolver::builder_with_config(config, TokioRuntimeProvider::default())
        .with_options(opts)
        .build()
        .expect("failed to build DoT resolver")
}

// ---------------------------------------------------------------------------
// Internal record query helper
// ---------------------------------------------------------------------------

/// Query for all records of `record_type`.  Never fails — errors yield an empty vec.
async fn query_records(
    resolver: &TokioResolver,
    hostname: &str,
    record_type: RecordType,
    dnssec_validated: bool,
) -> Vec<DnsRecord> {
    match resolver.lookup(hostname, record_type).await {
        Ok(lookup) => lookup
            .answers()
            .iter()
            .map(|record| DnsRecord {
                record_type: record.record_type().to_string(),
                value: record.data.to_string().trim_end_matches('.').to_owned(),
                ttl: record.ttl,
                dnssec_validated,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Public API — DoH PTR reverse lookup (spec §4)
// ---------------------------------------------------------------------------

pub async fn reverse_dns_doh(ip: &str, endpoint: &str, timeout_ms: u64) -> Result<Option<String>> {
    let ptr_name = ip_to_ptr_name(ip)
        .ok_or_else(|| anyhow::anyhow!("{}", crate::i18n::t("errors.error.dns.invalid_ip_ptr").replace("{ip}", ip)))?;

    let url = format!("{endpoint}?name={ptr_name}&type=PTR");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .context(crate::i18n::t("errors.error.dns.doh_client_build"))?;

    let body: DohResponse = client
        .get(&url)
        .header("Accept", "application/dns-json")
        .send()
        .await
        .with_context(|| crate::i18n::t("errors.error.dns.doh_request").replace("{ip}", ip))?
        .json()
        .await
        .context(crate::i18n::t("errors.error.dns.doh_parse"))?;

    let ptr = body
        .answer
        .and_then(|answers| answers.into_iter().next())
        .map(|a| a.data.trim_end_matches('.').to_owned())
        .filter(|s| !s.is_empty());

    Ok(ptr)
}

// ---------------------------------------------------------------------------
// Internal — DoH DNSSEC check (returns AD flag for a hostname)
// ---------------------------------------------------------------------------

/// Query the DoH endpoint for an A record and return the AD (Authenticated Data) flag.
/// Used to stamp all records from a `full_dns_lookup` with their DNSSEC status.
async fn dnssec_check_doh(hostname: &str, endpoint: &str, timeout_ms: u64) -> bool {
    let url = format!("{endpoint}?name={hostname}&type=A&do=1");

    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
    else {
        return false;
    };

    let Ok(resp) = client
        .get(&url)
        .header("Accept", "application/dns-json")
        .send()
        .await
    else {
        return false;
    };

    resp.json::<DohResponse>().await.map(|r| r.ad).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Public API — intelligent fallback reverse lookup (spec §5)
// ---------------------------------------------------------------------------

pub async fn reverse_lookup_smart(
    ip: &str,
    dns_mode: &DnsMode,
    system_timeout_ms: u64,
    doh_endpoint: &str,
    dot_server: &str,
    doh_timeout_ms: u64,
) -> Result<Option<String>> {
    match dns_mode {
        DnsMode::Disabled   => Ok(None),
        DnsMode::SystemOnly => reverse_lookup(ip, system_timeout_ms).await,
        DnsMode::DohOnly    => reverse_dns_doh(ip, doh_endpoint, doh_timeout_ms).await,
        DnsMode::DotOnly    => {
            let resolver = build_dot_resolver(dot_server, doh_timeout_ms);
            let ptr_name = ip_to_ptr_name(ip)
                .ok_or_else(|| anyhow::anyhow!("{}", crate::i18n::t("errors.error.invalid_ip_reverse").replace("{ip}", ip)))?;
            let deadline = Duration::from_millis(doh_timeout_ms * 2);
            let result = timeout(deadline, resolver.reverse_lookup(&ptr_name))
                .await
                .context(crate::i18n::t("errors.error.dns.reverse_timeout"))?;
            match result {
                Ok(lookup) => Ok(lookup.answers().iter().find_map(|r| {
                    if let RData::PTR(ptr) = &r.data {
                        Some(ptr.0.to_string().trim_end_matches('.').to_owned())
                    } else {
                        None
                    }
                })),
                Err(e) if e.is_no_records_found() => Ok(None),
                Err(e) => Err(anyhow::Error::new(e)
                    .context(crate::i18n::t("errors.error.dns.reverse_failed").replace("{ip}", ip))),
            }
        }
        DnsMode::Automatic  => {
            match reverse_lookup(ip, system_timeout_ms).await {
                Ok(result) => Ok(result),
                Err(_) => {
                    tracing::debug!(target: "atlasip::dns", %ip, "system DNS failed; falling back to DoH");
                    reverse_dns_doh(ip, doh_endpoint, doh_timeout_ms).await
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public result type for a full hostname lookup
// ---------------------------------------------------------------------------

pub struct DnsLookupResult {
    /// First A or AAAA address resolved (used as the pipeline IP).
    pub resolved_ip: Option<String>,
    /// PTR name for the resolved IP.
    pub ptr: Option<String>,
    /// All records found (A, AAAA, CNAME, TXT, MX, NS, SOA) with TTL + DNSSEC status.
    pub records: Vec<DnsRecord>,
    /// Non-fatal error messages collected during the lookup.
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API — full parallel DNS lookup
// ---------------------------------------------------------------------------

/// Perform a full DNS lookup for `hostname`:
///
/// 1. Parallel queries: A, AAAA, CNAME, TXT, MX, NS, SOA.
/// 2. Concurrent DNSSEC check via DoH AD flag (stamped on all records).
/// 3. Extract the first resolved IP from A/AAAA records.
/// 4. PTR reverse lookup on the resolved IP.
///
/// Transport is selected by `dns_mode`:
/// - `Automatic` / `SystemOnly` → hickory system resolver.
/// - `DohOnly` → hickory system resolver for records + DNSSEC via DoH.
/// - `DotOnly` → hickory DoT resolver with `dot_server`.
/// - `Disabled` → returns empty result.
pub async fn full_dns_lookup(
    hostname: &str,
    timeout_ms: u64,
    dns_mode: &DnsMode,
    doh_endpoint: &str,
    dot_server: &str,
) -> DnsLookupResult {
    if matches!(dns_mode, DnsMode::Disabled) {
        return DnsLookupResult { resolved_ip: None, ptr: None, records: Vec::new(), errors: Vec::new() };
    }

    let deadline = Duration::from_millis(timeout_ms * 3);
    let mut errors: Vec<String> = Vec::new();

    let inner = async {
        let resolver = match dns_mode {
            DnsMode::DotOnly => build_dot_resolver(dot_server, timeout_ms),
            _                => build_resolver(timeout_ms),
        };

        // DNSSEC check runs concurrently with record queries.
        let dnssec_fut = dnssec_check_doh(hostname, doh_endpoint, timeout_ms);

        let (
            a_recs, aaaa_recs, cname_recs, txt_recs,
            mx_recs, ns_recs, soa_recs,
            dnssec_validated,
        ) = tokio::join!(
            query_records(&resolver, hostname, RecordType::A,     false),
            query_records(&resolver, hostname, RecordType::AAAA,  false),
            query_records(&resolver, hostname, RecordType::CNAME, false),
            query_records(&resolver, hostname, RecordType::TXT,   false),
            query_records(&resolver, hostname, RecordType::MX,    false),
            query_records(&resolver, hostname, RecordType::NS,    false),
            query_records(&resolver, hostname, RecordType::SOA,   false),
            dnssec_fut,
        );

        // Re-stamp all records with the DNSSEC result now that we have it.
        let mut records: Vec<DnsRecord> = [a_recs, aaaa_recs, cname_recs, txt_recs, mx_recs, ns_recs, soa_recs]
            .into_iter()
            .flatten()
            .map(|mut r| { r.dnssec_validated = dnssec_validated; r })
            .collect();

        // Sort for stable display order: A → AAAA → CNAME → MX → NS → SOA → TXT.
        let type_order = |t: &str| match t {
            "A"     => 0u8, "AAAA"  => 1, "CNAME" => 2, "MX" => 3,
            "NS"    => 4,   "SOA"   => 5, "TXT"   => 6, _    => 7,
        };
        records.sort_by_key(|r| type_order(&r.record_type));

        let resolved_ip = records
            .iter()
            .find(|r| r.record_type == "A" || r.record_type == "AAAA")
            .map(|r| r.value.clone());

        let ptr = match &resolved_ip {
            Some(ip) => reverse_lookup(ip, timeout_ms).await.ok().flatten(),
            None     => None,
        };

        (records, resolved_ip, ptr)
    };

    match timeout(deadline, inner).await {
        Ok((records, resolved_ip, ptr)) => DnsLookupResult { resolved_ip, ptr, records, errors },
        Err(_) => {
            errors.push(crate::i18n::t("errors.error.dns.lookup_timeout").replace("{hostname}", hostname));
            DnsLookupResult { resolved_ip: None, ptr: None, records: Vec::new(), errors }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API — backward-compatible single-result helpers
// ---------------------------------------------------------------------------

pub async fn resolve_hostname(hostname: &str, timeout_ms: u64) -> Result<String> {
    let resolver = build_resolver(timeout_ms);
    let deadline = Duration::from_millis(timeout_ms * 2);

    let lookup = timeout(deadline, resolver.lookup_ip(hostname))
        .await
        .context(crate::i18n::t("errors.error.dns.forward_timeout"))?
        .with_context(|| crate::i18n::t("errors.error.dns.forward_failed").replace("{hostname}", hostname))?;

    let addr = lookup
        .iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("{}", crate::i18n::t("errors.error.dns.no_address").replace("{hostname}", hostname)))?;

    Ok(addr.to_string())
}

pub async fn reverse_lookup(ip: &str, timeout_ms: u64) -> Result<Option<String>> {
    let ptr_name = ip_to_ptr_name(ip)
        .ok_or_else(|| anyhow::anyhow!("{}", crate::i18n::t("errors.error.invalid_ip_reverse").replace("{ip}", ip)))?;

    let resolver = build_resolver(timeout_ms);
    let deadline = Duration::from_millis(timeout_ms * 2);

    let result = timeout(deadline, resolver.reverse_lookup(&ptr_name))
        .await
        .context(crate::i18n::t("errors.error.dns.reverse_timeout"))?;

    match result {
        Ok(lookup) => Ok(lookup.answers().iter().find_map(|record| {
            if let RData::PTR(ptr) = &record.data {
                Some(ptr.0.to_string().trim_end_matches('.').to_owned())
            } else {
                None
            }
        })),
        Err(e) if e.is_no_records_found() => Ok(None),
        Err(e) => Err(anyhow::Error::new(e)
            .context(crate::i18n::t("errors.error.dns.reverse_failed").replace("{ip}", ip))),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: u64 = 3000;
    const DOH: &str    = "https://cloudflare-dns.com/dns-query";
    const DOT: &str    = "cloudflare";

    // ── Backward-compat forward lookup ───────────────────────────────────────

    #[tokio::test]
    async fn test_resolve_known_hostname() {
        let ip = resolve_hostname("dns.google", TIMEOUT).await.unwrap();
        assert!(!ip.is_empty());
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
        let result = full_dns_lookup("dns.google", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        assert!(result.resolved_ip.is_some());
        assert!(result.records.iter().any(|r| r.record_type == "A" || r.record_type == "AAAA"));
    }

    #[tokio::test]
    async fn test_full_dns_lookup_mx_records() {
        let result = full_dns_lookup("google.com", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        assert!(result.records.iter().any(|r| r.record_type == "MX"), "Expected MX records for google.com");
    }

    #[tokio::test]
    async fn test_full_dns_lookup_ns_records() {
        let result = full_dns_lookup("google.com", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        assert!(result.records.iter().any(|r| r.record_type == "NS"), "Expected NS records");
    }

    #[tokio::test]
    async fn test_full_dns_lookup_soa_record() {
        let result = full_dns_lookup("google.com", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        assert!(result.records.iter().any(|r| r.record_type == "SOA"), "Expected SOA record");
    }

    #[tokio::test]
    async fn test_full_dns_lookup_txt_records() {
        let result = full_dns_lookup("google.com", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        assert!(result.records.iter().any(|r| r.record_type == "TXT"));
    }

    #[tokio::test]
    async fn test_full_dns_lookup_sort_order() {
        let result = full_dns_lookup("google.com", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        let types: Vec<&str> = result.records.iter().map(|r| r.record_type.as_str()).collect();
        // A records must appear before NS/SOA/TXT if present.
        if let (Some(a_pos), Some(ns_pos)) = (
            types.iter().position(|&t| t == "A"),
            types.iter().position(|&t| t == "NS"),
        ) {
            assert!(a_pos < ns_pos);
        }
    }

    #[tokio::test]
    async fn test_full_dns_lookup_dnssec_flag_is_bool() {
        let result = full_dns_lookup("cloudflare.com", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        // All records should have the same dnssec_validated value (AD bit per response).
        let flags: std::collections::HashSet<bool> = result.records.iter().map(|r| r.dnssec_validated).collect();
        assert!(flags.len() <= 1, "all records in a response should share the same DNSSEC status");
    }

    #[tokio::test]
    async fn test_full_dns_lookup_ttl_present() {
        let result = full_dns_lookup("dns.google", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        for rec in &result.records {
            assert!(rec.ttl > 0, "TTL > 0 for {:?}", rec);
        }
    }

    #[tokio::test]
    async fn test_full_dns_lookup_ptr() {
        let result = full_dns_lookup("dns.google", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        assert!(result.ptr.is_some());
    }

    #[tokio::test]
    async fn test_full_dns_lookup_nxdomain_returns_empty() {
        let result = full_dns_lookup("this-does-not-exist.invalid", TIMEOUT, &DnsMode::Automatic, DOH, DOT).await;
        assert!(result.resolved_ip.is_none());
        assert!(result.records.is_empty());
    }

    #[tokio::test]
    async fn test_full_dns_lookup_disabled_mode() {
        let result = full_dns_lookup("google.com", TIMEOUT, &DnsMode::Disabled, DOH, DOT).await;
        assert!(result.records.is_empty());
        assert!(result.resolved_ip.is_none());
    }

    // ── PTR name helpers ─────────────────────────────────────────────────────

    #[test]
    fn test_ip_to_ptr_name_ipv4() {
        assert_eq!(ip_to_ptr_name("8.8.8.8").unwrap(), "8.8.8.8.in-addr.arpa");
        assert_eq!(ip_to_ptr_name("1.2.3.4").unwrap(), "4.3.2.1.in-addr.arpa");
    }

    #[test]
    fn test_ip_to_ptr_name_ipv6() {
        let ptr = ip_to_ptr_name("2001:db8::1").unwrap();
        assert!(ptr.ends_with(".ip6.arpa"), "unexpected: {ptr}");
    }

    #[test]
    fn test_ip_to_ptr_name_invalid() {
        assert!(ip_to_ptr_name("not-an-ip").is_none());
    }

    // ── DoH reverse lookup ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_reverse_dns_doh_known_ip() {
        let result = reverse_dns_doh("1.1.1.1", DOH, TIMEOUT).await.unwrap();
        assert!(result.is_some());
        let ptr = result.unwrap();
        assert!(!ptr.is_empty());
        assert!(!ptr.ends_with('.'));
    }

    #[tokio::test]
    async fn test_reverse_dns_doh_no_ptr_returns_none() {
        let result = reverse_dns_doh("192.0.2.1", DOH, TIMEOUT).await.unwrap();
        assert!(result.is_none());
    }

    // ── Smart fallback ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_smart_disabled_returns_none() {
        let result = reverse_lookup_smart("8.8.8.8", &DnsMode::Disabled, 300, DOH, DOT, TIMEOUT).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_smart_doh_only() {
        let result = reverse_lookup_smart("1.1.1.1", &DnsMode::DohOnly, 300, DOH, DOT, TIMEOUT).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_smart_automatic_falls_back_on_bad_timeout() {
        let result = reverse_lookup_smart("1.1.1.1", &DnsMode::Automatic, 1, DOH, DOT, TIMEOUT).await;
        assert!(result.is_ok());
    }
}
