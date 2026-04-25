use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;

use crate::{config::ProxyConfig, models::{BgpInfo, BgpPeer}};

// ---------------------------------------------------------------------------
// Client  (data source: RIPEstat — https://stat.ripe.net/docs/data_api)
// ---------------------------------------------------------------------------

pub struct BgpClient {
    client: reqwest::Client,
}

impl BgpClient {
    pub fn new(timeout_ms: u64, proxy: &ProxyConfig) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .user_agent("AtlasIP/0.7 (OSINT; https://github.com/Absol3m/atlasip)");

        if let Some(url) = &proxy.http   { builder = builder.proxy(reqwest::Proxy::http(url)?);  }
        if let Some(url) = &proxy.https  { builder = builder.proxy(reqwest::Proxy::https(url)?); }
        if let Some(url) = &proxy.socks4 { builder = builder.proxy(reqwest::Proxy::all(url)?);   }
        if let Some(url) = &proxy.socks5 { builder = builder.proxy(reqwest::Proxy::all(url)?);   }

        Ok(Self { client: builder.build()? })
    }

    /// Full BGP enrichment for `ip`: resolves ASN via prefix-overview, then
    /// fetches announced prefixes + neighbours in parallel.
    pub async fn lookup(&self, ip: &str) -> Result<BgpInfo> {
        let (asn, mut info) = self.query_prefix_overview(ip).await?;

        let asn = match asn {
            Some(n) => n,
            None => return Ok(info),
        };

        let (prefixes_result, peers_result) =
            tokio::join!(self.query_announced_prefixes(asn), self.query_neighbours(asn));

        if let Ok((v4, v6)) = prefixes_result {
            info.prefixes_v4 = v4;
            info.prefixes_v6 = v6;
        }
        if let Ok(peers) = peers_result {
            info.peers = peers;
        }

        Ok(info)
    }

    /// GET /data/prefix-overview/data.json?resource={ip}
    /// Returns the most-specific prefix announced for `ip` plus its origin ASN.
    async fn query_prefix_overview(&self, ip: &str) -> Result<(Option<u32>, BgpInfo)> {
        let url = format!(
            "https://stat.ripe.net/data/prefix-overview/data.json?resource={ip}"
        );
        let resp: serde_json::Value = self.client.get(&url).send().await?.json().await?;

        if resp["status"].as_str() != Some("ok") {
            anyhow::bail!("RIPEstat prefix-overview returned non-ok status");
        }

        let asns = resp["data"]["asns"].as_array();
        let first_asn = asns.and_then(|a| a.first());

        let asn = first_asn
            .and_then(|a| a["asn"].as_u64())
            .map(|n| n as u32);

        let holder = first_asn
            .and_then(|a| a["holder"].as_str())
            .map(str::to_owned);

        let (as_name, as_country) = parse_holder(holder.as_deref());

        // The prefix announced for this specific IP (most-specific match).
        let prefix_v4 = resp["data"]["prefix"]
            .as_str()
            .filter(|p| !p.contains(':'))
            .map(str::to_owned);
        let prefix_v6 = resp["data"]["prefix"]
            .as_str()
            .filter(|p| p.contains(':'))
            .map(str::to_owned);

        let prefixes_v4 = prefix_v4.into_iter().collect();
        let prefixes_v6 = prefix_v6.into_iter().collect();

        Ok((
            asn,
            BgpInfo {
                asn,
                as_name,
                as_country,
                prefixes_v4,
                prefixes_v6,
                peers: Vec::new(),
            },
        ))
    }

    /// GET /data/announced-prefixes/data.json?resource=AS{asn}
    /// Returns all IPv4 + IPv6 prefixes announced by this ASN.
    async fn query_announced_prefixes(&self, asn: u32) -> Result<(Vec<String>, Vec<String>)> {
        let url = format!(
            "https://stat.ripe.net/data/announced-prefixes/data.json?resource=AS{asn}"
        );
        let resp: serde_json::Value = self.client.get(&url).send().await?.json().await?;

        if resp["status"].as_str() != Some("ok") {
            anyhow::bail!("RIPEstat announced-prefixes returned non-ok status");
        }

        let (mut v4, mut v6) = (Vec::new(), Vec::new());
        if let Some(arr) = resp["data"]["prefixes"].as_array() {
            for entry in arr {
                if let Some(prefix) = entry["prefix"].as_str() {
                    if prefix.contains(':') {
                        v6.push(prefix.to_owned());
                    } else {
                        v4.push(prefix.to_owned());
                    }
                }
            }
        }

        Ok((v4, v6))
    }

    /// GET /data/asn-neighbours/data.json?resource=AS{asn}
    /// Returns BGP neighbours (peers) visible from RIPE RIS route collectors.
    async fn query_neighbours(&self, asn: u32) -> Result<Vec<BgpPeer>> {
        let url = format!(
            "https://stat.ripe.net/data/asn-neighbours/data.json?resource=AS{asn}"
        );
        let resp: serde_json::Value = self.client.get(&url).send().await?.json().await?;

        if resp["status"].as_str() != Some("ok") {
            anyhow::bail!("RIPEstat asn-neighbours returned non-ok status");
        }

        let mut map: HashMap<u32, BgpPeer> = HashMap::new();
        if let Some(arr) = resp["data"]["neighbours"].as_array() {
            for entry in arr {
                if let Some(peer_asn) = entry["asn"].as_u64().map(|n| n as u32) {
                    map.entry(peer_asn).or_insert(BgpPeer {
                        asn: peer_asn,
                        name: None,
                        country: None,
                    });
                }
            }
        }

        let mut peers: Vec<BgpPeer> = map.into_values().collect();
        peers.sort_by_key(|p| p.asn);
        Ok(peers)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a RIPEstat `holder` string like `"GOOGLE - Google LLC, US"` into
/// `(as_name, as_country)`.  The short name is the part before ` - ` (or the
/// whole string), and the country is the last comma-separated token when it
/// looks like an ISO-3166 alpha-2/3 code (all-uppercase, 2-3 chars).
pub(crate) fn parse_holder(holder: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(h) = holder else {
        return (None, None);
    };

    // Extract a potential ISO country code from the trailing ", XX" or ", XXX".
    let as_country = h
        .rsplit(',')
        .next()
        .map(str::trim)
        .filter(|s| s.len() >= 2 && s.len() <= 3 && s.chars().all(|c| c.is_uppercase()))
        .map(str::to_owned);

    // Short name: the part before " - " if present, otherwise strip the country
    // suffix (", XX") so we don't return "TELIANET, SE" as the name.
    let base = if let Some((short, _)) = h.split_once(" - ") {
        short.trim()
    } else if as_country.is_some() {
        // Remove trailing ", COUNTRY" from the full string.
        h.rsplit_once(',').map(|(left, _)| left.trim()).unwrap_or(h.trim())
    } else {
        h.trim()
    };

    (Some(base.to_owned()), as_country)
}

/// Split a flat slice of prefix JSON objects into (v4, v6) string vecs.
pub(crate) fn split_prefixes(prefixes: Option<&[serde_json::Value]>) -> (Vec<String>, Vec<String>) {
    let Some(arr) = prefixes else {
        return (Vec::new(), Vec::new());
    };
    let mut v4 = Vec::new();
    let mut v6 = Vec::new();
    for p in arr {
        if let Some(prefix) = p["prefix"].as_str() {
            if prefix.contains(':') {
                v6.push(prefix.to_owned());
            } else {
                v4.push(prefix.to_owned());
            }
        }
    }
    (v4, v6)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── parse_holder ─────────────────────────────────────────────────────────

    #[test]
    fn test_parse_holder_full_format() {
        let (name, country) = parse_holder(Some("GOOGLE - Google LLC, US"));
        assert_eq!(name.as_deref(), Some("GOOGLE"));
        assert_eq!(country.as_deref(), Some("US"));
    }

    #[test]
    fn test_parse_holder_no_dash() {
        let (name, country) = parse_holder(Some("TELIANET, SE"));
        assert_eq!(name.as_deref(), Some("TELIANET"));
        assert_eq!(country.as_deref(), Some("SE"));
    }

    #[test]
    fn test_parse_holder_no_country() {
        let (name, country) = parse_holder(Some("CERN - European Organization for Nuclear Research"));
        assert_eq!(name.as_deref(), Some("CERN"));
        assert!(country.is_none(), "no country code should be extracted");
    }

    #[test]
    fn test_parse_holder_none() {
        let (name, country) = parse_holder(None);
        assert!(name.is_none());
        assert!(country.is_none());
    }

    #[test]
    fn test_parse_holder_three_letter_country() {
        let (name, country) = parse_holder(Some("TEST-AS - Test ISP, EUR"));
        assert_eq!(name.as_deref(), Some("TEST-AS"));
        assert_eq!(country.as_deref(), Some("EUR"));
    }

    // ── split_prefixes ───────────────────────────────────────────────────────

    #[test]
    fn test_split_prefixes_none_input() {
        let (v4, v6) = split_prefixes(None);
        assert!(v4.is_empty());
        assert!(v6.is_empty());
    }

    #[test]
    fn test_split_prefixes_empty_array() {
        let arr: Vec<serde_json::Value> = vec![];
        let (v4, v6) = split_prefixes(Some(&arr));
        assert!(v4.is_empty());
        assert!(v6.is_empty());
    }

    #[test]
    fn test_split_prefixes_v4_only() {
        let arr = vec![
            json!({ "prefix": "8.8.8.0/24" }),
            json!({ "prefix": "8.8.4.0/24" }),
        ];
        let (v4, v6) = split_prefixes(Some(&arr));
        assert_eq!(v4, vec!["8.8.8.0/24", "8.8.4.0/24"]);
        assert!(v6.is_empty());
    }

    #[test]
    fn test_split_prefixes_v6_only() {
        let arr = vec![
            json!({ "prefix": "2001:4860::/32" }),
            json!({ "prefix": "2404:6800::/32" }),
        ];
        let (v4, v6) = split_prefixes(Some(&arr));
        assert!(v4.is_empty());
        assert_eq!(v6, vec!["2001:4860::/32", "2404:6800::/32"]);
    }

    #[test]
    fn test_split_prefixes_mixed() {
        let arr = vec![
            json!({ "prefix": "8.8.8.0/24" }),
            json!({ "prefix": "2001:4860::/32" }),
            json!({ "prefix": "1.2.3.0/24" }),
        ];
        let (v4, v6) = split_prefixes(Some(&arr));
        assert_eq!(v4.len(), 2);
        assert_eq!(v6.len(), 1);
        assert!(v4.contains(&"8.8.8.0/24".to_owned()));
        assert!(v6.contains(&"2001:4860::/32".to_owned()));
    }

    #[test]
    fn test_split_prefixes_skips_missing_field() {
        let arr = vec![
            json!({ "ip": "8.8.8.0" }),
            json!({ "prefix": "8.8.8.0/24" }),
        ];
        let (v4, _) = split_prefixes(Some(&arr));
        assert_eq!(v4, vec!["8.8.8.0/24"]);
    }

    // ── BgpInfo / BgpPeer data model ─────────────────────────────────────────

    #[test]
    fn test_bgp_info_default_is_empty() {
        let info = BgpInfo::default();
        assert!(info.asn.is_none());
        assert!(info.as_name.is_none());
        assert!(info.as_country.is_none());
        assert!(info.prefixes_v4.is_empty());
        assert!(info.prefixes_v6.is_empty());
        assert!(info.peers.is_empty());
    }

    #[test]
    fn test_bgp_info_serialization_roundtrip() {
        let original = BgpInfo {
            asn: Some(15169),
            as_name: Some("GOOGLE".into()),
            as_country: Some("US".into()),
            prefixes_v4: vec!["8.8.8.0/24".into()],
            prefixes_v6: vec!["2001:4860::/32".into()],
            peers: vec![BgpPeer { asn: 1, name: Some("TEST".into()), country: Some("FR".into()) }],
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: BgpInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.asn, Some(15169));
        assert_eq!(decoded.as_name.as_deref(), Some("GOOGLE"));
        assert_eq!(decoded.prefixes_v4, vec!["8.8.8.0/24"]);
        assert_eq!(decoded.prefixes_v6, vec!["2001:4860::/32"]);
        assert_eq!(decoded.peers.len(), 1);
    }

    #[test]
    fn test_bgp_peer_optional_fields() {
        let peer = BgpPeer { asn: 42, name: None, country: None };
        let json = serde_json::to_string(&peer).unwrap();
        let decoded: BgpPeer = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.asn, 42);
        assert!(decoded.name.is_none());
        assert!(decoded.country.is_none());
    }

    // ── RIPEstat response parsing ────────────────────────────────────────────

    #[test]
    fn test_parse_prefix_overview_extracts_asn() {
        let resp = json!({
            "status": "ok",
            "data": {
                "prefix": "8.8.8.0/24",
                "asns": [{
                    "asn": 15169,
                    "holder": "GOOGLE - Google LLC, US"
                }]
            }
        });
        let asns = resp["data"]["asns"].as_array();
        let first = asns.and_then(|a| a.first());
        let asn = first.and_then(|a| a["asn"].as_u64()).map(|n| n as u32);
        let (name, country) = parse_holder(first.and_then(|a| a["holder"].as_str()));

        assert_eq!(asn, Some(15169));
        assert_eq!(name.as_deref(), Some("GOOGLE"));
        assert_eq!(country.as_deref(), Some("US"));

        let prefix = resp["data"]["prefix"].as_str().filter(|p| !p.contains(':'));
        assert_eq!(prefix, Some("8.8.8.0/24"));
    }

    #[test]
    fn test_parse_prefix_overview_no_asns() {
        let resp = json!({ "status": "ok", "data": { "asns": [] } });
        let asns = resp["data"]["asns"].as_array();
        let asn = asns.and_then(|a| a.first()).and_then(|a| a["asn"].as_u64());
        assert!(asn.is_none());
    }

    #[test]
    fn test_parse_neighbours_deduplicates() {
        let resp = json!({
            "status": "ok",
            "data": {
                "neighbours": [
                    { "asn": 1234, "type": "left" },
                    { "asn": 5678, "type": "right" },
                    { "asn": 1234, "type": "uncertain" }
                ]
            }
        });

        let mut map: std::collections::HashMap<u32, BgpPeer> = std::collections::HashMap::new();
        if let Some(arr) = resp["data"]["neighbours"].as_array() {
            for entry in arr {
                if let Some(asn) = entry["asn"].as_u64().map(|n| n as u32) {
                    map.entry(asn).or_insert(BgpPeer { asn, name: None, country: None });
                }
            }
        }
        assert_eq!(map.len(), 2, "duplicate ASN 1234 must be deduped");
    }

    // ── Network integration ──────────────────────────────────────────────────

    #[tokio::test]
    #[ignore = "requires network access to stat.ripe.net"]
    async fn test_bgp_lookup_google_dns() {
        let client = BgpClient::new(5_000, &ProxyConfig::default()).expect("client init should not fail");
        let result = client.lookup("8.8.8.8").await;

        assert!(result.is_ok(), "RIPEstat lookup failed: {:?}", result.err());
        let info = result.unwrap();

        assert_eq!(info.asn, Some(15169), "8.8.8.8 must be AS15169 (Google)");
        assert!(
            info.as_name.as_deref().map(|n| n.to_uppercase().contains("GOOGLE")).unwrap_or(false),
            "AS name should contain GOOGLE, got: {:?}", info.as_name
        );
        assert!(!info.prefixes_v4.is_empty(), "Google must announce at least one IPv4 prefix");
        assert!(!info.peers.is_empty(), "Google must have BGP peers");

        let asns: Vec<u32> = info.peers.iter().map(|p| p.asn).collect();
        let mut sorted = asns.clone();
        sorted.sort();
        assert_eq!(asns, sorted, "peers must be sorted by ASN");
    }

    #[tokio::test]
    #[ignore = "requires network access to stat.ripe.net"]
    async fn test_bgp_lookup_cloudflare() {
        let client = BgpClient::new(5_000, &ProxyConfig::default()).expect("client init should not fail");
        let info = client.lookup("1.1.1.1").await.expect("RIPEstat lookup failed");

        assert_eq!(info.asn, Some(13335), "1.1.1.1 must be AS13335 (Cloudflare)");
        assert!(!info.prefixes_v4.is_empty());
    }
}
