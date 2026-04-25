use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// BGP types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgpPeer {
    pub asn: u32,
    pub name: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BgpInfo {
    pub asn: Option<u32>,
    pub as_name: Option<String>,
    pub as_country: Option<String>,
    pub prefixes_v4: Vec<String>,
    pub prefixes_v6: Vec<String>,
    pub peers: Vec<BgpPeer>,
}

// ---------------------------------------------------------------------------
// DNS record type
// ---------------------------------------------------------------------------

/// A single DNS resource record (A, AAAA, CNAME, TXT, MX, NS, SOA…) with TTL.
/// Returned by the DNS lookup step and embedded in [`IpRecord`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    /// Record type as a string (e.g. "A", "AAAA", "CNAME", "TXT", "MX", "NS", "SOA").
    pub record_type: String,
    /// Human-readable record value (IP, hostname, or text content).
    pub value: String,
    /// Time-to-live in seconds as returned by the authoritative server.
    pub ttl: u32,
    /// Whether this record was DNSSEC-validated (AD bit set in the resolving response).
    #[serde(default)]
    pub dnssec_validated: bool,
}

/// Ordered column names as shown in the UI table (spec §2.2).
/// Used by export renderers to produce consistent column ordering.
pub const COLUMNS: &[&str] = &[
    "Order",
    "IP Address",
    "Country",
    "Owner Name",
    "Address",
    "Email",
    "Abuse Email",
    "Phone",
    "Fax",
    "From IP",
    "To IP",
    "Status",
    "Whois Source",
    "Network Name",
    "Contact Name",
    "Allocated",
    "Host Name",
    "Resolved Name",
    "CIDR",
    "Postal Code",
    "Abuse Contact",
];

/// Core record representing the result of an IP/hostname lookup.
/// All fields match the spec (section 3 — Modèle de Données).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpRecord {
    pub id: Uuid,
    pub order: u32,
    pub ip: String,
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
    pub whois_source: Option<String>,
    pub network_name: Option<String>,
    pub contact_name: Option<String>,
    pub allocated: Option<String>,
    pub host_name: Option<String>,
    pub resolved_name: Option<String>,
    pub cidr: Option<String>,
    pub postal_code: Option<String>,
    pub abuse_contact: Option<String>,
    pub raw_whois: Option<String>,
    pub raw_rdap: Option<serde_json::Value>,
    // ── GeoIP (MaxMind GeoLite2) ──────────────────────────────────────────────
    pub geo_lat:     Option<f64>,
    pub geo_lon:     Option<f64>,
    pub geo_city:    Option<String>,
    pub geo_country: Option<String>,
    /// DNS records (A, AAAA, CNAME, TXT) with TTL, populated for hostname
    /// targets. Not included in table columns — JSON/export only.
    pub dns_records: Vec<DnsRecord>,
    pub lookup_errors: Vec<String>,
    /// Remarks / comments extracted from WHOIS (P2-ENRICH-008).
    /// Not included in the main grid — exposed for detail views.
    pub remarks: Option<String>,
    /// Last-updated date extracted from WHOIS brut (P2-ENRICH-009).
    pub updated_at: Option<String>,
    // ── WHOIS enrichment flags (P0-ENRICH-003) ────────────────────────────
    // true  → field was empty after RDAP and was filled by WHOIS brut.
    // false → field came from RDAP (or was never filled).
    pub address_enriched: bool,
    pub country_enriched: bool,
    pub phone_enriched: bool,
    pub fax_enriched: bool,
    pub owner_enriched: bool,
    pub remarks_enriched: bool,
    pub dates_enriched: bool,
    /// BGP/ASN enrichment from BGPView (no account or API key required).
    #[serde(default)]
    pub bgp: Option<BgpInfo>,
}

impl IpRecord {
    /// Create a new empty IpRecord for a given IP string.
    pub fn new(order: u32, ip: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            order,
            ip: ip.into(),
            country: None,
            owner_name: None,
            address: None,
            emails: Vec::new(),
            abuse_emails: Vec::new(),
            phone: None,
            fax: None,
            from_ip: None,
            to_ip: None,
            status: None,
            whois_source: None,
            network_name: None,
            contact_name: None,
            allocated: None,
            host_name: None,
            resolved_name: None,
            cidr: None,
            postal_code: None,
            abuse_contact: None,
            raw_whois: None,
            raw_rdap: None,
            geo_lat:     None,
            geo_lon:     None,
            geo_city:    None,
            geo_country: None,
            dns_records: Vec::new(),
            lookup_errors: Vec::new(),
            remarks: None,
            updated_at: None,
            address_enriched: false,
            country_enriched: false,
            phone_enriched: false,
            fax_enriched: false,
            owner_enriched: false,
            remarks_enriched: false,
            dates_enriched: false,
            bgp: None,
        }
    }

    /// Return field values in the same order as [`COLUMNS`].
    /// Multi-value fields (emails, abuse_emails) are joined with "; ".
    pub fn to_row(&self) -> Vec<String> {
        vec![
            self.order.to_string(),
            self.ip.clone(),
            self.country.clone().unwrap_or_default(),
            self.owner_name.clone().unwrap_or_default(),
            self.address.clone().unwrap_or_default(),
            self.emails.join("; "),
            self.abuse_emails.join("; "),
            self.phone.clone().unwrap_or_default(),
            self.fax.clone().unwrap_or_default(),
            self.from_ip.clone().unwrap_or_default(),
            self.to_ip.clone().unwrap_or_default(),
            self.status.clone().unwrap_or_default(),
            self.whois_source.clone().unwrap_or_default(),
            self.network_name.clone().unwrap_or_default(),
            self.contact_name.clone().unwrap_or_default(),
            self.allocated.clone().unwrap_or_default(),
            self.host_name.clone().unwrap_or_default(),
            self.resolved_name.clone().unwrap_or_default(),
            self.cidr.clone().unwrap_or_default(),
            self.postal_code.clone().unwrap_or_default(),
            self.abuse_contact.clone().unwrap_or_default(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_defaults() {
        let rec = IpRecord::new(1, "8.8.8.8");
        assert_eq!(rec.ip, "8.8.8.8");
        assert_eq!(rec.order, 1);
        assert!(rec.country.is_none());
        assert!(rec.emails.is_empty());
        assert!(rec.lookup_errors.is_empty());
    }

    #[test]
    fn test_to_row_length_matches_columns() {
        let rec = IpRecord::new(0, "1.1.1.1");
        assert_eq!(rec.to_row().len(), COLUMNS.len());
    }

    #[test]
    fn test_to_row_values() {
        let mut rec = IpRecord::new(3, "1.1.1.1");
        rec.country = Some("AU".into());
        rec.emails = vec!["a@b.com".into(), "c@d.com".into()];
        let row = rec.to_row();
        assert_eq!(row[0], "3");
        assert_eq!(row[1], "1.1.1.1");
        assert_eq!(row[2], "AU");
        assert_eq!(row[5], "a@b.com; c@d.com");
    }

    #[test]
    fn test_to_row_empty_optional() {
        let rec = IpRecord::new(0, "::1");
        let row = rec.to_row();
        // all optional columns default to empty string
        assert!(row[2..].iter().all(|v| v.is_empty()));
    }
}

// ---------------------------------------------------------------------------
// API DTOs
// ---------------------------------------------------------------------------

/// Request body for POST /lookup/bulk
#[derive(Debug, Deserialize)]
pub struct BulkLookupRequest {
    pub targets: Vec<String>,
}

/// Query parameters for GET /export
#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: String,
    pub ids: Option<String>,
}

/// Request body for POST /config
#[derive(Debug, Deserialize)]
pub struct ConfigUpdateRequest {
    pub locale: Option<String>,
    pub proxy_type: Option<String>,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<u16>,
    pub dns_timeout_ms: Option<u64>,
    pub whois_timeout_ms: Option<u64>,
    pub rdap_timeout_ms: Option<u64>,
    pub default_export_format: Option<String>,
    pub csv_with_header: Option<bool>,
    // ── GeoIP credentials ─────────────────────────────────────────────────
    pub maxmind_account_id: Option<String>,
    pub maxmind_license_key: Option<String>,
}
