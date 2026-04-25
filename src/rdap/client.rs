use anyhow::{Context, Result};
use icann_rdap_client::prelude::{
    create_client, ClientConfig, MemoryBootstrapStore, rdap_bootstrapped_request, QueryType,
};
use serde_json::Value;

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

/// Extract a human-readable CIDR string from `cidr0_cidrs`.
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
            ev["eventDate"]
                .as_str()
                .map(|d| crate::utils::normalize_date(d))
        } else {
            None
        }
    })
}

// ---------------------------------------------------------------------------
// vCard helpers
// ---------------------------------------------------------------------------

fn vcard_props(entity: &Value) -> &[Value] {
    entity["vcardArray"]
        .as_array()
        .and_then(|a| a.get(1))
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn vcard_text(prop: &Value) -> Option<&str> {
    prop.as_array()?.get(3)?.as_str()
}

fn vcard_type_param(prop: &Value) -> Option<&str> {
    let params = prop.as_array()?.get(1)?;
    params["type"].as_str().or_else(|| {
        params["type"].as_array()?.first()?.as_str()
    })
}

/// Format a structured vCard `adr` array into a single-line address string.
/// Returns `(address, postal_code)`.
fn format_adr(value: &Value) -> (Option<String>, Option<String>) {
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
/// an entity. Used as a fallback when no top-level `country` field is present (ARIN).
fn extract_country_from_vcard(entity: &Value) -> Option<String> {
    for prop in vcard_props(entity) {
        let is_adr = prop.as_array()?.first()?.as_str()? == "adr";
        if !is_adr {
            continue;
        }
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
    // `org` role: used by APNIC/AFRINIC as the network owner entity.
    let is_org       = roles.contains(&"org");
    let is_abuse     = roles.contains(&"abuse");
    let is_secondary = roles.contains(&"administrative") || roles.contains(&"technical");

    let mut name: Option<String>   = None;
    let mut address: Option<String>= None;
    let mut postal_code: Option<String> = None;
    let mut emails: Vec<String>    = Vec::new();
    let mut phone: Option<String>  = None;
    let mut fax: Option<String>    = None;

    for prop in vcard_props(entity) {
        let prop_name = prop.as_array().and_then(|a| a.first()).and_then(|v| v.as_str());
        match prop_name {
            Some("fn") => {
                name = vcard_text(prop).map(str::to_owned);
            }
            Some("org") => {
                if name.is_none() {
                    name = vcard_text(prop).map(str::to_owned);
                }
            }
            Some("adr") => {
                let value = prop.as_array().and_then(|a| a.get(3));
                if let Some(v) = value {
                    let (addr, zip) = format_adr(v);
                    address    = address.or(addr);
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
                let number   = vcard_text(prop).map(str::to_owned);
                if tel_type.eq_ignore_ascii_case("fax") {
                    fax   = fax.or(number);
                } else {
                    phone = phone.or(number);
                }
            }
            _ => {}
        }
    }

    if is_registrant {
        parsed.owner_name  = parsed.owner_name.take().or(name.clone());
        parsed.address     = parsed.address.take().or(address.clone());
        parsed.postal_code = parsed.postal_code.take().or(postal_code.clone());
        parsed.emails.extend(emails.clone());
        parsed.phone = parsed.phone.take().or(phone.clone());
        parsed.fax   = parsed.fax.take().or(fax.clone());
        if parsed.country.is_none() {
            parsed.country = extract_country_from_vcard(entity);
        }
    }

    if is_org && !is_registrant {
        parsed.owner_name.get_or_insert_with(|| name.clone().unwrap_or_default());
        if parsed.owner_name.as_deref() == Some("") {
            parsed.owner_name = name.clone();
        }
        if parsed.address.is_none()     { parsed.address = address.clone(); }
        if parsed.postal_code.is_none() { parsed.postal_code = postal_code.clone(); }
        if parsed.phone.is_none()       { parsed.phone = phone.clone(); }
        if parsed.fax.is_none()         { parsed.fax   = fax.clone(); }
        parsed.emails.extend(emails.clone());
        if parsed.country.is_none() {
            parsed.country = extract_country_from_vcard(entity);
        }
    }

    if is_abuse {
        parsed.abuse_emails.extend(emails.clone());
        parsed.abuse_contact = parsed.abuse_contact.take().or(name.clone());
        if parsed.abuse_contact.is_none() {
            parsed.abuse_contact = emails.first().cloned();
        }
    }

    if is_secondary {
        if parsed.contact_name.is_none() {
            parsed.contact_name = name.clone();
        }
    }

    if !is_registrant && !is_org && !is_abuse && parsed.contact_name.is_none() {
        parsed.contact_name = name.clone();
    }

    if parsed.postal_code.is_none() {
        parsed.postal_code = postal_code;
    }

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
    timeout_ms: u64,
}

impl RdapClient {
    pub fn new(timeout_ms: u64) -> Result<Self> {
        Ok(Self { timeout_ms })
    }

    /// Look up `target` (IP or hostname) via RDAP using the ICANN bootstrapped client.
    pub async fn query(&self, target: &str) -> Result<RdapResult> {
        let target = target.trim();

        let timeout_secs = ((self.timeout_ms + 999) / 1000).max(1);
        let config = ClientConfig::builder()
            .timeout_secs(timeout_secs)
            .build();
        let http = create_client(&config)
            .context(crate::i18n::t("errors.error.rdap.client_build"))?;

        let store = MemoryBootstrapStore::new();
        let query: QueryType = target
            .parse()
            .with_context(|| format!("Cannot create RDAP query for '{target}'"))?;

        let response = rdap_bootstrapped_request(&query, &http, &store, |_| {})
            .await
            .with_context(|| {
                crate::i18n::t("errors.error.rdap.request_failed")
                    .replace("{url}", target)
            })?;

        let url = {
            let hd = &response.http_data;
            match hd.request_uri() {
                Some(uri) => format!("https://{}{}", hd.host(), uri),
                None      => hd.host().to_string(),
            }
        };
        let json = serde_json::to_value(&response.rdap)
            .context("Failed to serialize RDAP response")?;

        Ok(RdapResult { json, url })
    }

    /// Parse a raw RDAP JSON object into structured fields (spec §3).
    pub fn parse(json: &Value) -> ParsedRdap {
        let mut parsed = ParsedRdap::default();

        parsed.country      = json["country"].as_str().map(str::to_owned);
        parsed.network_name = json["name"].as_str().map(str::to_owned);
        parsed.from_ip      = json["startAddress"].as_str().map(str::to_owned);
        parsed.to_ip        = json["endAddress"].as_str().map(str::to_owned);
        parsed.cidr         = extract_cidr(json);

        if let Some(statuses) = json["status"].as_array() {
            let s: Vec<&str> = statuses.iter().filter_map(|v| v.as_str()).collect();
            if !s.is_empty() {
                parsed.status = Some(s.join(", "));
            }
        }

        parsed.allocated = extract_event_date(json, "registration");

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

    #[test]
    fn test_parse_org_role_entity() {
        let json = serde_json::json!({
            "objectClassName": "ip network",
            "name": "APNIC-NET",
            "country": "AU",
            "startAddress": "1.1.1.0",
            "endAddress": "1.1.1.255",
            "cidr0_cidrs": [{"v4prefix": "1.1.1.0", "length": 24}],
            "status": ["active"],
            "events": [
                {"eventAction": "registration", "eventDate": "2011-08-11"}
            ],
            "entities": [
                {
                    "roles": ["org"],
                    "vcardArray": ["vcard", [
                        ["version", {}, "text", "4.0"],
                        ["fn", {}, "text", "APNIC Pty Ltd"],
                        ["adr", {}, "text",
                         ["", "", "6 Cordelia St", "South Brisbane", "QLD", "4101", "AU"]],
                        ["email", {}, "text", "helpdesk@apnic.net"]
                    ]]
                },
                {
                    "roles": ["abuse"],
                    "vcardArray": ["vcard", [
                        ["version", {}, "text", "4.0"],
                        ["fn", {}, "text", "IRT-APNICRANDNET-AU"],
                        ["email", {}, "text", "abuse@apnic.net"]
                    ]]
                }
            ]
        });

        let p = RdapClient::parse(&json);

        assert_eq!(p.country.as_deref(), Some("AU"));
        assert_eq!(p.owner_name.as_deref(), Some("APNIC Pty Ltd"));
        assert!(p.emails.contains(&"helpdesk@apnic.net".to_owned()));
        assert!(p.abuse_emails.contains(&"abuse@apnic.net".to_owned()));
        assert_eq!(p.abuse_contact.as_deref(), Some("IRT-APNICRANDNET-AU"));
        assert_eq!(p.allocated.as_deref(), Some("2011-08-11T00:00:00Z"));
    }

    #[test]
    fn test_parse_date_already_iso8601() {
        let json = serde_json::json!({
            "events": [
                {"eventAction": "registration", "eventDate": "1992-04-30T00:00:00Z"}
            ],
            "entities": []
        });
        let p = RdapClient::parse(&json);
        assert_eq!(p.allocated.as_deref(), Some("1992-04-30T00:00:00Z"));
    }

    #[test]
    fn test_registrant_wins_over_org() {
        let json = serde_json::json!({
            "entities": [
                {
                    "roles": ["registrant"],
                    "vcardArray": ["vcard", [
                        ["version", {}, "text", "4.0"],
                        ["fn", {}, "text", "Registrant Name"],
                        ["email", {}, "text", "reg@example.com"]
                    ]]
                },
                {
                    "roles": ["org"],
                    "vcardArray": ["vcard", [
                        ["version", {}, "text", "4.0"],
                        ["fn", {}, "text", "Org Name"],
                        ["email", {}, "text", "org@example.com"]
                    ]]
                }
            ]
        });

        let p = RdapClient::parse(&json);
        assert_eq!(p.owner_name.as_deref(), Some("Registrant Name"),
            "registrant entity (first in document) should set owner_name");
    }

    #[test]
    fn test_org_only_entity_fills_owner_name() {
        let json = serde_json::json!({
            "entities": [
                {
                    "roles": ["org"],
                    "vcardArray": ["vcard", [
                        ["version", {}, "text", "4.0"],
                        ["fn", {}, "text", "APNIC Pty Ltd"],
                        ["email", {}, "text", "helpdesk@apnic.net"]
                    ]]
                }
            ]
        });

        let p = RdapClient::parse(&json);
        assert_eq!(p.owner_name.as_deref(), Some("APNIC Pty Ltd"));
    }

    #[tokio::test]
    #[ignore = "requires RDAP network access"]
    async fn test_query_cloudflare_ip() {
        let client = RdapClient::new(5000).unwrap();
        let result = client.query("1.1.1.1").await.unwrap();
        let parsed = RdapClient::parse(&result.json);

        assert!(parsed.from_ip.is_some(), "Expected startAddress");
        assert!(parsed.network_name.is_some(), "Expected network name");
    }

    #[tokio::test]
    #[ignore = "requires RDAP network access"]
    async fn test_query_ripe_ip_has_country() {
        let client = RdapClient::new(5000).unwrap();
        let result = client.query("194.2.0.1").await.unwrap();
        let parsed = RdapClient::parse(&result.json);

        assert!(parsed.country.is_some(), "Expected RIPE to return a country");
        assert!(parsed.from_ip.is_some());
    }
}
