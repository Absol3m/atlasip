use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::{net::IpAddr, time::Duration};

// ---------------------------------------------------------------------------
// IANA RDAP bootstrap URLs (RFC 9224)
// ---------------------------------------------------------------------------

const BOOTSTRAP_IPV4: &str = "https://data.iana.org/rdap/ipv4.json";
const BOOTSTRAP_IPV6: &str = "https://data.iana.org/rdap/ipv6.json";
const BOOTSTRAP_DNS: &str = "https://data.iana.org/rdap/dns.json";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Raw RDAP response for one query.
#[derive(Debug, Clone)]
pub struct RdapResult {
    /// The parsed JSON body returned by the RDAP server.
    pub json: Value,
    /// The URL that was queried.
    pub url: String,
}

/// All fields extracted from an RDAP JSON response (spec §2.1 & §3).
#[derive(Debug, Default, Clone)]
pub struct ParsedRdap {
    pub country: Option<String>,
    pub owner_name: Option<String>,
    pub address: Option<String>,
    pub emails: Vec<String>,
    pub abuse_emails: Vec<String>,
    pub phone: Option<String>,
    pub fax: Option<String>,
    pub from_ip: Option<String>,
    pub to_ip: Option<String>,
    pub status: Option<String>,
    pub network_name: Option<String>,
    pub contact_name: Option<String>,
    pub allocated: Option<String>,
    pub cidr: Option<String>,
    pub postal_code: Option<String>,
    pub abuse_contact: Option<String>,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Whether a bootstrap lookup is for an IP or a domain name.
enum QueryKind {
    Ip,
    Domain,
}

/// Return `true` if `ip_str` falls within the CIDR `cidr_str` (e.g. `"8.0.0.0/8"`).
/// Supports both IPv4 and IPv6. Returns `false` on any parse error.
fn ip_in_cidr(ip_str: &str, cidr_str: &str) -> bool {
    let (net_str, prefix_str) = match cidr_str.rsplit_once('/') {
        Some(pair) => pair,
        // No slash → treat as host route (exact match).
        None => return ip_str == cidr_str,
    };

    let network: IpAddr = match net_str.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let prefix_len: u32 = match prefix_str.parse() {
        Ok(l) => l,
        Err(_) => return false,
    };
    let target: IpAddr = match ip_str.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };

    match (network, target) {
        (IpAddr::V4(net), IpAddr::V4(host)) => {
            if prefix_len >= 32 {
                return net == host;
            }
            let mask = u32::MAX << (32 - prefix_len);
            (u32::from(net) & mask) == (u32::from(host) & mask)
        }
        (IpAddr::V6(net), IpAddr::V6(host)) => {
            if prefix_len >= 128 {
                return net == host;
            }
            let mask = u128::MAX << (128 - prefix_len);
            (u128::from(net) & mask) == (u128::from(host) & mask)
        }
        _ => false, // mixed address families
    }
}

/// Extract a human-readable CIDR string from `cidr0_cidrs` (preferred) or
/// from `startAddress` / `endAddress` as a fallback.
fn extract_cidr(json: &Value) -> Option<String> {
    if let Some(cidrs) = json["cidr0_cidrs"].as_array() {
        let parts: Vec<String> = cidrs
            .iter()
            .filter_map(|c| {
                let prefix = c["v4prefix"]
                    .as_str()
                    .or_else(|| c["v6prefix"].as_str())?;
                let length = c["length"].as_u64()?;
                Some(format!("{prefix}/{length}"))
            })
            .collect();
        if !parts.is_empty() {
            return Some(parts.join(", "));
        }
    }
    None
}

/// Walk the `events` array and return the date of the first event whose
/// `eventAction` matches `action` (e.g. `"registration"`).
fn extract_event_date(json: &Value, action: &str) -> Option<String> {
    json["events"].as_array()?.iter().find_map(|ev| {
        if ev["eventAction"].as_str()? == action {
            ev["eventDate"].as_str().map(str::to_owned)
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------------------
// vCard helpers
// ---------------------------------------------------------------------------

/// Return the raw `vcardArray[1]` property list for an entity, or an empty
/// slice if the structure is absent or malformed.
fn vcard_props(entity: &Value) -> &[Value] {
    entity["vcardArray"]
        .as_array()
        .and_then(|a| a.get(1))
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// Extract a simple text value from a vCard property tuple
/// `[name, params, type, value]`.
fn vcard_text(prop: &Value) -> Option<&str> {
    prop.as_array()?.get(3)?.as_str()
}

/// Extract the `type` parameter from a vCard property (e.g. `"voice"`, `"fax"`).
fn vcard_type_param(prop: &Value) -> Option<&str> {
    let params = prop.as_array()?.get(1)?;
    params["type"].as_str().or_else(|| {
        // type can also be an array: ["voice", "text"]
        params["type"].as_array()?.first()?.as_str()
    })
}

/// Format a structured vCard `adr` array into a single-line address string.
/// The adr value is: [pobox, ext, street, locality, region, postal_code, country].
fn format_adr(value: &Value) -> (Option<String>, Option<String>) {
    // Value may be a plain string (label format) or a 7-element array.
    if let Some(s) = value.as_str() {
        let clean = s.replace('\n', ", ").trim().to_owned();
        return (Some(clean), None);
    }

    if let Some(parts) = value.as_array() {
        let get = |i: usize| {
            parts
                .get(i)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        let postal_code = get(5);
        let components: Vec<String> = [get(2), get(3), get(4), postal_code.clone(), get(6)]
            .into_iter()
            .flatten()
            .collect();
        let addr = if components.is_empty() {
            None
        } else {
            Some(components.join(", "))
        };
        return (addr, postal_code);
    }

    (None, None)
}

/// Try to extract a 2-letter country code from the vCard `adr` property of
/// an entity (the 7th structured field is the country).
/// Used as a fallback when no top-level `country` field is present (ARIN).
fn extract_country_from_vcard(entity: &Value) -> Option<String> {
    for prop in vcard_props(entity) {
        let is_adr = prop.as_array()?.first()?.as_str()? == "adr";
        if !is_adr {
            continue;
        }
        // Structured adr: ["pobox","ext","street","city","region","postal","country"]
        if let Some(parts) = prop.as_array()?.get(3).and_then(|v| v.as_array()) {
            if let Some(country) = parts.get(6).and_then(|v| v.as_str()) {
                let c = country.trim();
                if !c.is_empty() {
                    return Some(c.to_owned());
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Entity processing
// ---------------------------------------------------------------------------

/// Parse the vCard of a single entity and merge relevant fields into `parsed`
/// according to the entity's `roles`.
fn process_entity(entity: &Value, parsed: &mut ParsedRdap) {
    let roles: Vec<&str> = entity["roles"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|r| r.as_str()).collect())
        .unwrap_or_default();

    let is_registrant = roles.contains(&"registrant");
    let is_abuse = roles.contains(&"abuse");

    let mut name: Option<String> = None;
    let mut address: Option<String> = None;
    let mut postal_code: Option<String> = None;
    let mut emails: Vec<String> = Vec::new();
    let mut phone: Option<String> = None;
    let mut fax: Option<String> = None;

    for prop in vcard_props(entity) {
        let prop_name = prop.as_array().and_then(|a| a.first()).and_then(|v| v.as_str());
        match prop_name {
            Some("fn") => {
                name = vcard_text(prop).map(str::to_owned);
            }
            Some("org") => {
                // Use org as name if fn is not available.
                if name.is_none() {
                    name = vcard_text(prop).map(str::to_owned);
                }
            }
            Some("adr") => {
                let value = prop.as_array().and_then(|a| a.get(3));
                if let Some(v) = value {
                    let (addr, zip) = format_adr(v);
                    address = address.or(addr);
                    postal_code = postal_code.or(zip);
                }
            }
            Some("email") => {
                if let Some(email) = vcard_text(prop) {
                    emails.push(email.to_owned());
                }
            }
            Some("tel") => {
                let tel_type = vcard_type_param(prop).unwrap_or("voice");
                let number = vcard_text(prop).map(str::to_owned);
                if tel_type.eq_ignore_ascii_case("fax") {
                    fax = fax.or(number);
                } else {
                    phone = phone.or(number);
                }
            }
            _ => {}
        }
    }

    // Merge into parsed based on roles.
    if is_registrant {
        parsed.owner_name = parsed.owner_name.take().or(name.clone());
        parsed.address = parsed.address.take().or(address.clone());
        parsed.postal_code = parsed.postal_code.take().or(postal_code.clone());
        parsed.emails.extend(emails.clone());
        parsed.phone = parsed.phone.take().or(phone.clone());
        parsed.fax = parsed.fax.take().or(fax.clone());
        // Fallback: extract country code from the last field of the adr vCard
        // when the top-level `country` field is absent (e.g. ARIN responses).
        if parsed.country.is_none() {
            parsed.country = extract_country_from_vcard(entity);
        }
    }

    if is_abuse {
        parsed.abuse_emails.extend(emails.clone());
        parsed.abuse_contact = parsed.abuse_contact.take().or(name.clone());
        // Fallback: use first abuse email as abuse_contact if no name.
        if parsed.abuse_contact.is_none() {
            parsed.abuse_contact = emails.first().cloned();
        }
    }

    // contact_name: first non-registrant named entity.
    if !is_registrant && parsed.contact_name.is_none() {
        parsed.contact_name = name.clone();
    }

    // Postal code: fill if still missing.
    if parsed.postal_code.is_none() {
        parsed.postal_code = postal_code;
    }

    // Recurse into nested entities (e.g. an org entity containing people).
    if let Some(sub_entities) = entity["entities"].as_array() {
        for sub in sub_entities {
            process_entity(sub, parsed);
        }
    }
}

// ---------------------------------------------------------------------------
// RdapClient
// ---------------------------------------------------------------------------

pub struct RdapClient {
    http: Client,
    timeout_ms: u64,
}

impl RdapClient {
    /// Build a new client with the given per-request timeout.
    pub fn new(timeout_ms: u64) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .user_agent("AtlasIP/0.1 (RDAP client; https://github.com/atlasip)")
            .build()
            .context("Failed to build RDAP HTTP client")?;
        Ok(Self { http, timeout_ms })
    }

    /// Look up `target` (IP or hostname) via RDAP.
    ///
    /// Flow (spec §2.1):
    /// 1. Detect query type (IP vs domain).
    /// 2. Resolve the authoritative RDAP server via the IANA bootstrap registry.
    /// 3. Fetch the RDAP object and return the raw JSON.
    pub async fn query(&self, target: &str) -> Result<RdapResult> {
        let target = target.trim();
        match target.parse::<IpAddr>() {
            Ok(ip) => self.query_ip(target, ip).await,
            Err(_) => self.query_domain(target).await,
        }
    }

    async fn query_ip(&self, target: &str, ip: IpAddr) -> Result<RdapResult> {
        let bootstrap_url = match ip {
            IpAddr::V4(_) => BOOTSTRAP_IPV4,
            IpAddr::V6(_) => BOOTSTRAP_IPV6,
        };
        let base = self
            .find_rdap_base(bootstrap_url, target, QueryKind::Ip)
            .await?;
        let url = format!("{base}ip/{target}");
        self.fetch(&url).await
    }

    async fn query_domain(&self, target: &str) -> Result<RdapResult> {
        // Match on TLD (last label) for the DNS bootstrap.
        let tld = target.rsplit('.').next().unwrap_or(target);
        let base = self
            .find_rdap_base(BOOTSTRAP_DNS, tld, QueryKind::Domain)
            .await?;
        let url = format!("{base}domain/{target}");
        self.fetch(&url).await
    }

    /// Fetch the IANA bootstrap file at `bootstrap_url` and find the RDAP
    /// base URL whose prefix set matches `target`.
    async fn find_rdap_base(
        &self,
        bootstrap_url: &str,
        target: &str,
        kind: QueryKind,
    ) -> Result<String> {
        let body: Value = self
            .http
            .get(bootstrap_url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch IANA bootstrap: {bootstrap_url}"))?
            .error_for_status()
            .with_context(|| format!("IANA bootstrap returned error: {bootstrap_url}"))?
            .json()
            .await
            .context("Failed to parse IANA bootstrap JSON")?;

        let services = body["services"]
            .as_array()
            .context("Invalid IANA bootstrap: missing 'services' array")?;

        for service in services {
            let entry = match service.as_array() {
                Some(a) if a.len() >= 2 => a,
                _ => continue,
            };
            let empty = vec![];
            let prefixes = entry[0].as_array().unwrap_or(&empty);
            let urls = entry[1].as_array().unwrap_or(&empty);

            let matched = prefixes.iter().any(|p| {
                let prefix = p.as_str().unwrap_or("");
                match kind {
                    QueryKind::Ip => ip_in_cidr(target, prefix),
                    QueryKind::Domain => prefix.eq_ignore_ascii_case(target),
                }
            });

            if matched {
                let raw_url = urls
                    .first()
                    .and_then(|u| u.as_str())
                    .with_context(|| {
                        format!("Bootstrap entry for '{target}' has no RDAP URL")
                    })?;
                // Ensure the base URL ends with '/'.
                let base = if raw_url.ends_with('/') {
                    raw_url.to_owned()
                } else {
                    format!("{raw_url}/")
                };
                return Ok(base);
            }
        }

        anyhow::bail!(
            "No authoritative RDAP server found for '{target}' in {bootstrap_url}"
        )
    }

    /// GET `url`, assert HTTP 2xx, parse body as JSON.
    async fn fetch(&self, url: &str) -> Result<RdapResult> {
        let json: Value = self
            .http
            .get(url)
            .header(
                "Accept",
                "application/rdap+json, application/json;q=0.9",
            )
            .send()
            .await
            .with_context(|| format!("RDAP request failed: {url}"))?
            .error_for_status()
            .with_context(|| format!("RDAP server returned an error for: {url}"))?
            .json()
            .await
            .with_context(|| format!("Failed to parse RDAP JSON from: {url}"))?;

        Ok(RdapResult {
            json,
            url: url.to_owned(),
        })
    }

    /// Parse a raw RDAP JSON object into structured fields (spec §3).
    ///
    /// Handles both IP network objects and domain objects.
    pub fn parse(json: &Value) -> ParsedRdap {
        let mut parsed = ParsedRdap::default();

        // --- Top-level IP network fields ---
        parsed.country = json["country"].as_str().map(str::to_owned);
        parsed.network_name = json["name"].as_str().map(str::to_owned);
        parsed.from_ip = json["startAddress"].as_str().map(str::to_owned);
        parsed.to_ip = json["endAddress"].as_str().map(str::to_owned);
        parsed.cidr = extract_cidr(json);

        // Status: join the status array into a comma-separated string.
        if let Some(statuses) = json["status"].as_array() {
            let s: Vec<&str> = statuses.iter().filter_map(|v| v.as_str()).collect();
            if !s.is_empty() {
                parsed.status = Some(s.join(", "));
            }
        }

        // Allocated date from the "registration" event.
        parsed.allocated = extract_event_date(json, "registration");

        // --- Entities (registrant, abuse, tech, …) ---
        if let Some(entities) = json["entities"].as_array() {
            for entity in entities {
                process_entity(entity, &mut parsed);
            }
        }

        parsed
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Unit tests (no network) ---

    #[test]
    fn test_ip_in_cidr_v4_match() {
        assert!(ip_in_cidr("8.8.8.8", "8.0.0.0/8"));
        assert!(ip_in_cidr("8.8.8.8", "8.8.0.0/16"));
        assert!(ip_in_cidr("8.8.8.8", "8.8.8.8/32"));
    }

    #[test]
    fn test_ip_in_cidr_v4_no_match() {
        assert!(!ip_in_cidr("1.1.1.1", "8.0.0.0/8"));
        assert!(!ip_in_cidr("9.0.0.1", "8.0.0.0/8"));
    }

    #[test]
    fn test_ip_in_cidr_v6_match() {
        assert!(ip_in_cidr("2001:4860:4860::8888", "2001:4860::/32"));
    }

    #[test]
    fn test_ip_in_cidr_bad_input() {
        assert!(!ip_in_cidr("not-an-ip", "8.0.0.0/8"));
        assert!(!ip_in_cidr("8.8.8.8", "not-a-cidr/8"));
    }

    #[test]
    fn test_extract_cidr_from_cidr0() {
        let json = serde_json::json!({
            "cidr0_cidrs": [
                {"v4prefix": "8.8.8.0", "length": 24},
                {"v4prefix": "8.8.4.0", "length": 24}
            ]
        });
        let cidr = extract_cidr(&json).unwrap();
        assert_eq!(cidr, "8.8.8.0/24, 8.8.4.0/24");
    }

    #[test]
    fn test_extract_cidr_missing_returns_none() {
        let json = serde_json::json!({ "name": "TEST-NET" });
        assert!(extract_cidr(&json).is_none());
    }

    #[test]
    fn test_extract_event_date() {
        let json = serde_json::json!({
            "events": [
                {"eventAction": "last changed", "eventDate": "2020-01-01T00:00:00Z"},
                {"eventAction": "registration", "eventDate": "1992-04-30T00:00:00Z"}
            ]
        });
        let date = extract_event_date(&json, "registration").unwrap();
        assert_eq!(date, "1992-04-30T00:00:00Z");
    }

    #[test]
    fn test_parse_full_rdap_response() {
        // Minimal RDAP-like JSON similar to what ARIN returns for 8.8.8.0/24.
        let json = serde_json::json!({
            "objectClassName": "ip network",
            "handle": "NET-8-8-8-0-1",
            "name": "LVLT-GOGL-8-8-8",
            "country": "US",
            "startAddress": "8.8.8.0",
            "endAddress": "8.8.8.255",
            "cidr0_cidrs": [{"v4prefix": "8.8.8.0", "length": 24}],
            "status": ["active"],
            "events": [
                {"eventAction": "registration", "eventDate": "2014-03-14T00:00:00Z"}
            ],
            "entities": [
                {
                    "roles": ["registrant"],
                    "vcardArray": ["vcard", [
                        ["version", {}, "text", "4.0"],
                        ["fn", {}, "text", "Google LLC"],
                        ["adr", {"label": "1600 Amphitheatre Pkwy\nMountain View\nCA 94043\nUnited States"},
                         "text", ["", "", "1600 Amphitheatre Pkwy", "Mountain View", "CA", "94043", "US"]],
                        ["email", {}, "text", "arin-contact@google.com"],
                        ["tel", {"type": "voice"}, "text", "+1-650-253-0000"]
                    ]]
                },
                {
                    "roles": ["abuse"],
                    "vcardArray": ["vcard", [
                        ["version", {}, "text", "4.0"],
                        ["fn", {}, "text", "Google LLC - Abuse"],
                        ["email", {}, "text", "network-abuse@google.com"],
                        ["tel", {"type": "voice"}, "text", "+1-650-253-0000"]
                    ]]
                }
            ]
        });

        let p = RdapClient::parse(&json);

        assert_eq!(p.country.as_deref(), Some("US"));
        assert_eq!(p.network_name.as_deref(), Some("LVLT-GOGL-8-8-8"));
        assert_eq!(p.from_ip.as_deref(), Some("8.8.8.0"));
        assert_eq!(p.to_ip.as_deref(), Some("8.8.8.255"));
        assert_eq!(p.cidr.as_deref(), Some("8.8.8.0/24"));
        assert_eq!(p.status.as_deref(), Some("active"));
        assert_eq!(p.allocated.as_deref(), Some("2014-03-14T00:00:00Z"));
        assert_eq!(p.owner_name.as_deref(), Some("Google LLC"));
        assert_eq!(p.address.as_deref(), Some("1600 Amphitheatre Pkwy, Mountain View, CA, 94043, US"));
        assert_eq!(p.postal_code.as_deref(), Some("94043"));
        assert!(p.emails.contains(&"arin-contact@google.com".to_owned()));
        assert_eq!(p.phone.as_deref(), Some("+1-650-253-0000"));
        assert!(p.abuse_emails.contains(&"network-abuse@google.com".to_owned()));
        assert_eq!(p.abuse_contact.as_deref(), Some("Google LLC - Abuse"));
    }

    // --- Integration tests (real network) ---

    #[tokio::test]
    #[ignore = "requires ARIN RDAP network access"]
    async fn test_query_google_dns_ip() {
        let client = RdapClient::new(5000).unwrap();
        let result = client.query("8.8.8.8").await.unwrap();
        let parsed = RdapClient::parse(&result.json);

        // 8.8.8.8 is Google's public DNS — served by ARIN.
        // ARIN does not include a top-level `country` field, so we only assert
        // the fields that ARIN reliably returns.
        assert_eq!(parsed.from_ip.as_deref(), Some("8.8.8.0"), "Expected startAddress");
        assert_eq!(parsed.to_ip.as_deref(), Some("8.8.8.255"), "Expected endAddress");
        assert!(parsed.owner_name.is_some(), "Expected an owner name (Google LLC)");
        assert!(parsed.cidr.is_some(), "Expected a CIDR (cidr0_cidrs)");
    }

    #[tokio::test]
    async fn test_query_cloudflare_ip() {
        let client = RdapClient::new(5000).unwrap();
        // 1.1.1.1 is served by APNIC — returns country at top-level.
        let result = client.query("1.1.1.1").await.unwrap();
        let parsed = RdapClient::parse(&result.json);

        assert!(parsed.from_ip.is_some(), "Expected startAddress");
        assert!(parsed.network_name.is_some(), "Expected network name");
    }

    #[tokio::test]
    #[ignore = "requires ARIN RDAP network access"]
    async fn test_query_ipv6() {
        let client = RdapClient::new(5000).unwrap();
        // Google's public IPv6 DNS — also served by ARIN (no top-level country).
        let result = client.query("2001:4860:4860::8888").await.unwrap();
        let parsed = RdapClient::parse(&result.json);

        assert!(parsed.from_ip.is_some(), "Expected startAddress for IPv6 block");
        assert!(parsed.network_name.is_some(), "Expected network name");
    }

    #[tokio::test]
    async fn test_query_ripe_ip_has_country() {
        let client = RdapClient::new(5000).unwrap();
        // 194.2.0.0 is in RIPE space — RIPE always returns `country` at top-level.
        let result = client.query("194.2.0.1").await.unwrap();
        let parsed = RdapClient::parse(&result.json);

        assert!(parsed.country.is_some(), "Expected RIPE to return a country");
        assert!(parsed.from_ip.is_some());
    }
}
