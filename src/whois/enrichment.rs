//! WHOIS brut enrichment layer (backend_enrichment_v1.txt — P0-ENRICH-001…003).
//!
//! # Rules (strict, from spec)
//! - RDAP is the primary source.  [`enrich_from_whois_raw`] **never overwrites**
//!   a field that is already populated.
//! - Every field filled by this module sets the corresponding `*_enriched` flag
//!   on [`IpRecord`] to `true`, so the UI can distinguish RDAP-native vs WHOIS
//!   enriched values (R3).
//! - All enrichment actions are logged at `debug` level (R5).

use tracing::debug;

use crate::{
    models::IpRecord,
    utils,
    whois::WhoisClient,
};

// ---------------------------------------------------------------------------
// Extra fields not covered by ParsedWhois
// ---------------------------------------------------------------------------

/// Fields extracted from WHOIS raw text that are *not* part of the standard
/// [`ParsedWhois`] (i.e. enrichment-specific extras).
struct EnrichmentExtras {
    /// Lines from `Remarks:`, `Comment:`, or `Note:` keys.
    remarks: Option<String>,
    /// Last-updated date from `Updated:`, `Changed:`, or `last-modified:`.
    updated_at: Option<String>,
    /// Country extracted from the `Ctry:` key (P1-ENRICH-006 fallback).
    ctry: Option<String>,
}

/// Parse enrichment-specific extra fields from a raw WHOIS text.
fn parse_extras(raw: &str) -> EnrichmentExtras {
    let mut remarks_lines: Vec<String> = Vec::new();
    let mut updated_at: Option<String> = None;
    let mut ctry: Option<String> = None;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('%') || line.starts_with('#') {
            continue;
        }
        let (key, value) = match line.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };
        if value.is_empty() {
            continue;
        }
        match key.to_ascii_lowercase().as_str() {
            // ── Remarks (P2-ENRICH-008) ───────────────────────────────────
            "remarks" | "comment" | "note" => {
                remarks_lines.push(value.to_owned());
            }
            // ── Updated date (P2-ENRICH-009) ──────────────────────────────
            "updated" | "changed" | "last-modified" | "last-update" => {
                updated_at.get_or_insert_with(|| utils::normalize_date(value));
            }
            // ── Country fallback: Ctry: (P1-ENRICH-006) ───────────────────
            "ctry" => {
                ctry.get_or_insert_with(|| value.to_owned());
            }
            _ => {}
        }
    }

    EnrichmentExtras {
        remarks: if remarks_lines.is_empty() {
            None
        } else {
            Some(remarks_lines.join(" | "))
        },
        updated_at,
        ctry,
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Enrich `record` with data extracted from `whois_raw`.
///
/// This function implements the RDAP-wins fusion rules from the spec:
/// - Only fields that are **currently `None` / empty** on `record` are filled.
/// - Every field filled here sets the corresponding `*_enriched` flag.
/// - All enrichment actions are logged at `debug` level.
///
/// # Spec references
/// - P0-ENRICH-001 — function signature and module
/// - P0-ENRICH-002 — no field left empty if WHOIS can provide a value
/// - P0-ENRICH-003 — `*_enriched` flags
/// - P1-ENRICH-004 — address
/// - P1-ENRICH-005 — phone / fax
/// - P1-ENRICH-006 — country
/// - P2-ENRICH-007 — owner / network_name
/// - P2-ENRICH-008 — remarks
/// - P2-ENRICH-009 — dates (allocated, updated_at)
pub fn enrich_from_whois_raw(record: &mut IpRecord, whois_raw: &str) {
    let parsed = WhoisClient::parse(whois_raw);
    let extras = parse_extras(whois_raw);

    // ── P1-ENRICH-004 — Address ───────────────────────────────────────────
    if record.address.is_none() {
        if let Some(v) = parsed.address {
            debug!("WHOIS enrichment [{}]: address = {:?}", record.ip, v);
            record.address = Some(v);
            record.address_enriched = true;
        }
    }

    // ── P1-ENRICH-006 — Country (Country: > Ctry: fallback) ──────────────
    if record.country.is_none() {
        let country = parsed.country.or(extras.ctry);
        if let Some(v) = country {
            debug!("WHOIS enrichment [{}]: country = {:?}", record.ip, v);
            record.country = Some(v);
            record.country_enriched = true;
        }
    }

    // ── P1-ENRICH-005 — Phone ─────────────────────────────────────────────
    if record.phone.is_none() {
        if let Some(v) = parsed.phone {
            debug!("WHOIS enrichment [{}]: phone = {:?}", record.ip, v);
            record.phone = Some(v);
            record.phone_enriched = true;
        }
    }

    // ── P1-ENRICH-005 — Fax ──────────────────────────────────────────────
    if record.fax.is_none() {
        if let Some(v) = parsed.fax {
            debug!("WHOIS enrichment [{}]: fax = {:?}", record.ip, v);
            record.fax = Some(v);
            record.fax_enriched = true;
        }
    }

    // ── P2-ENRICH-007 — Owner / OrgName ──────────────────────────────────
    if record.owner_name.is_none() {
        if let Some(v) = parsed.owner_name {
            debug!("WHOIS enrichment [{}]: owner_name = {:?}", record.ip, v);
            record.owner_name = Some(v);
            record.owner_enriched = true;
        }
    }

    // ── P2-ENRICH-007 — NetName (complement, no dedicated flag) ──────────
    if record.network_name.is_none() {
        if let Some(v) = parsed.network_name {
            debug!("WHOIS enrichment [{}]: network_name = {:?}", record.ip, v);
            record.network_name = Some(v);
        }
    }

    // ── P2-ENRICH-008 — Remarks ───────────────────────────────────────────
    if record.remarks.is_none() {
        if let Some(v) = extras.remarks {
            debug!("WHOIS enrichment [{}]: remarks = {:?}", record.ip, v);
            record.remarks = Some(v);
            record.remarks_enriched = true;
        }
    }

    // ── P2-ENRICH-009 — Dates (allocated = created, updated_at) ──────────
    let mut dates_enriched = false;
    if record.allocated.is_none() {
        if let Some(v) = parsed.allocated {
            debug!("WHOIS enrichment [{}]: allocated = {:?}", record.ip, v);
            record.allocated = Some(v);
            dates_enriched = true;
        }
    }
    if record.updated_at.is_none() {
        if let Some(v) = extras.updated_at {
            debug!("WHOIS enrichment [{}]: updated_at = {:?}", record.ip, v);
            record.updated_at = Some(v);
            dates_enriched = true;
        }
    }
    if dates_enriched {
        record.dates_enriched = true;
    }

    // ── Supplementary fields (no dedicated enrichment flag) ───────────────
    // Emails: only add if RDAP gave nothing (R1 — non-empty list wins).
    if record.emails.is_empty() {
        record.emails = parsed.emails;
    }
    if record.abuse_emails.is_empty() {
        record.abuse_emails = parsed.abuse_emails;
    }

    fill_opt(&mut record.postal_code,  parsed.postal_code);
    fill_opt(&mut record.abuse_contact, parsed.abuse_contact);
    fill_opt(&mut record.contact_name, parsed.contact_name);
    fill_opt(&mut record.from_ip,      parsed.from_ip);
    fill_opt(&mut record.to_ip,        parsed.to_ip);
    fill_opt(&mut record.cidr,         parsed.cidr);
    fill_opt(&mut record.status,       parsed.status);
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[inline]
fn fill_opt(dest: &mut Option<String>, src: Option<String>) {
    if dest.is_none() {
        *dest = src;
    }
}

// ---------------------------------------------------------------------------
// Tests (P0-ENRICH — QA backend enrichment)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::IpRecord;

    fn make_record() -> IpRecord {
        IpRecord::new(1, "8.8.8.8")
    }

    // ── RDAP riche + WHOIS riche → RDAP gagne pour les champs déjà remplis ─

    #[test]
    fn test_rdap_wins_all_fields_already_set() {
        let mut rec = make_record();
        rec.country    = Some("US".into());
        rec.address    = Some("1600 Amphitheatre Pkwy, Mountain View, CA".into());
        rec.phone      = Some("+1-650-253-0000".into());
        rec.fax        = Some("+1-650-253-0001".into());
        rec.owner_name = Some("Google LLC".into());
        rec.allocated  = Some("2020-01-01T00:00:00Z".into());

        let whois_raw = "\
country: NL
address: Some Other Street
phone: +31205354444
fax-no: +31205354445
orgname: RIPE NCC
created: 2003-03-17T12:15:57Z
remarks: Do not overwrite
";
        enrich_from_whois_raw(&mut rec, whois_raw);

        // RDAP values must be unchanged.
        assert_eq!(rec.country.as_deref(),    Some("US"));
        assert_eq!(rec.phone.as_deref(),      Some("+1-650-253-0000"));
        assert_eq!(rec.fax.as_deref(),        Some("+1-650-253-0001"));
        assert_eq!(rec.owner_name.as_deref(), Some("Google LLC"));
        assert_eq!(rec.allocated.as_deref(),  Some("2020-01-01T00:00:00Z"));
        assert!(rec.address.as_deref().unwrap().contains("1600 Amphitheatre"));

        // No enrichment flags must be set (RDAP provided everything).
        assert!(!rec.country_enriched);
        assert!(!rec.address_enriched);
        assert!(!rec.phone_enriched);
        assert!(!rec.fax_enriched);
        assert!(!rec.owner_enriched);
        assert!(!rec.dates_enriched);
        // Remarks was None → gets enriched.
        assert!(rec.remarks_enriched);
        assert!(rec.remarks.is_some());
    }

    // ── RDAP pauvre + WHOIS riche → champs complétés par WHOIS ─────────────

    #[test]
    fn test_rdap_poor_whois_rich_fills_gaps() {
        let mut rec = make_record();
        // Simulate a minimal RDAP result: only owner_name known.
        rec.owner_name = Some("Google LLC".into());

        let whois_raw = "\
# ARIN WHOIS
NetRange:       8.8.8.0 - 8.8.8.255
CIDR:           8.8.8.0/24
NetName:        GOGL
Country:        US
Address:        1600 Amphitheatre Parkway
City:           Mountain View
StateProv:      CA
PostalCode:     94043
OrgAbusePhone:  +1-650-253-0000
RegDate:        2023-12-28
Updated:        2024-01-15
";
        enrich_from_whois_raw(&mut rec, whois_raw);

        // WHOIS filled the gaps.
        assert_eq!(rec.country.as_deref(), Some("US"));
        assert!(rec.address.as_deref().unwrap().contains("1600 Amphitheatre"));
        assert_eq!(rec.phone.as_deref(), Some("+1-650-253-0000"));
        assert_eq!(rec.cidr.as_deref(), Some("8.8.8.0/24"));
        assert_eq!(rec.postal_code.as_deref(), Some("94043"));
        assert!(rec.allocated.is_some());
        assert!(rec.updated_at.is_some());

        // Enrichment flags set correctly.
        assert!(rec.country_enriched);
        assert!(rec.address_enriched);
        assert!(rec.phone_enriched);
        assert!(rec.dates_enriched);

        // owner_name came from RDAP → not enriched.
        assert!(!rec.owner_enriched);
        assert_eq!(rec.owner_name.as_deref(), Some("Google LLC"));
    }

    // ── RDAP pauvre + WHOIS pauvre → champs restent None ────────────────────

    #[test]
    fn test_rdap_poor_whois_poor_fields_remain_none() {
        let mut rec = make_record();

        enrich_from_whois_raw(&mut rec, "% No objects found.\n");

        assert!(rec.country.is_none());
        assert!(rec.address.is_none());
        assert!(rec.phone.is_none());
        assert!(rec.fax.is_none());
        assert!(!rec.country_enriched);
        assert!(!rec.address_enriched);
    }

    // ── Tests Address (P1-ENRICH-004) ───────────────────────────────────────

    #[test]
    fn test_address_multiline_whois() {
        let mut rec = make_record();
        let raw = "\
address: P.O. Box 10096
address: 1001 EB
address: Amsterdam
address: NETHERLANDS
";
        enrich_from_whois_raw(&mut rec, raw);
        let addr = rec.address.unwrap();
        assert!(addr.contains("P.O. Box 10096"));
        assert!(addr.contains("Amsterdam"));
        assert!(rec.address_enriched);
    }

    #[test]
    fn test_address_not_set_when_whois_has_none() {
        let mut rec = make_record();
        enrich_from_whois_raw(&mut rec, "netname: TEST\ncountry: US\n");
        assert!(rec.address.is_none());
        assert!(!rec.address_enriched);
    }

    #[test]
    fn test_address_partial_arin() {
        let mut rec = make_record();
        // Partial: only City + Country, no street.
        let raw = "City: Mountain View\nCountry: US\n";
        enrich_from_whois_raw(&mut rec, raw);
        // ARIN path: city present → address built from available components.
        assert!(rec.address.is_some());
        assert!(rec.address_enriched);
    }

    // ── Tests Phone / Fax (P1-ENRICH-005) ───────────────────────────────────

    #[test]
    fn test_phone_multiple_sources() {
        let mut rec = make_record();
        // OrgTechPhone takes first match, OrgAbusePhone is ignored.
        let raw = "OrgTechPhone: +1-800-123-4567\nOrgAbusePhone: +1-800-999-0000\n";
        enrich_from_whois_raw(&mut rec, raw);
        assert_eq!(rec.phone.as_deref(), Some("+1-800-123-4567"));
        assert!(rec.phone_enriched);
    }

    #[test]
    fn test_phone_not_enriched_when_whois_has_none() {
        let mut rec = make_record();
        enrich_from_whois_raw(&mut rec, "netname: TEST\n");
        assert!(rec.phone.is_none());
        assert!(!rec.phone_enriched);
    }

    #[test]
    fn test_fax_exotic_format() {
        let mut rec = make_record();
        enrich_from_whois_raw(&mut rec, "fax-no: +55 11 5509-3501 ext 42\n");
        assert_eq!(rec.fax.as_deref(), Some("+55 11 5509-3501 ext 42"));
        assert!(rec.fax_enriched);
    }

    // ── Tests Country (P1-ENRICH-006) ────────────────────────────────────────

    #[test]
    fn test_country_rdap_wins_over_whois() {
        let mut rec = make_record();
        rec.country = Some("US".into());

        enrich_from_whois_raw(&mut rec, "Country: NL\n");

        assert_eq!(rec.country.as_deref(), Some("US"));
        assert!(!rec.country_enriched);
    }

    #[test]
    fn test_country_filled_from_whois_when_rdap_empty() {
        let mut rec = make_record();
        enrich_from_whois_raw(&mut rec, "Country: BR\n");
        assert_eq!(rec.country.as_deref(), Some("BR"));
        assert!(rec.country_enriched);
    }

    #[test]
    fn test_country_ctry_fallback() {
        let mut rec = make_record();
        // `Ctry:` is used when `Country:` is absent (P1-ENRICH-006).
        enrich_from_whois_raw(&mut rec, "Ctry: JP\n");
        assert_eq!(rec.country.as_deref(), Some("JP"));
        assert!(rec.country_enriched);
    }

    // ── Tests Flags (P0-ENRICH-003) ──────────────────────────────────────────

    #[test]
    fn test_enriched_flag_true_only_when_whois_filled() {
        let mut rec = make_record();
        // No RDAP data at all → all fields open for enrichment.
        let raw = "\
Country: DE
phone: +49 30 1234567
fax-no: +49 30 1234568
orgname: Deutsche Telekom
address: Telekom Allee 1
address: 53113 Bonn
address: Germany
created: 2000-01-01
updated: 2023-06-15
remarks: Test remark
";
        enrich_from_whois_raw(&mut rec, raw);
        assert!(rec.country_enriched);
        assert!(rec.phone_enriched);
        assert!(rec.fax_enriched);
        assert!(rec.owner_enriched);
        assert!(rec.address_enriched);
        assert!(rec.dates_enriched);
        assert!(rec.remarks_enriched);
    }

    #[test]
    fn test_enriched_flag_false_when_rdap_provided() {
        let mut rec = make_record();
        rec.country    = Some("DE".into());
        rec.phone      = Some("+49 30 000".into());
        rec.fax        = Some("+49 30 001".into());
        rec.owner_name = Some("Deutsche Telekom".into());
        rec.address    = Some("Telekom Allee 1".into());
        rec.allocated  = Some("2000-01-01T00:00:00Z".into());

        enrich_from_whois_raw(&mut rec, "Country: US\nphone: +1-650-253-0000\n");

        assert!(!rec.country_enriched);
        assert!(!rec.phone_enriched);
        assert!(!rec.fax_enriched);
        assert!(!rec.owner_enriched);
        assert!(!rec.address_enriched);
        assert!(!rec.dates_enriched);
    }

    // ── Tests Remarks (P2-ENRICH-008) ────────────────────────────────────────

    #[test]
    fn test_remarks_multiple_lines_joined() {
        let mut rec = make_record();
        let raw = "remarks: Line one\nremarks: Line two\ncomment: Also a comment\n";
        enrich_from_whois_raw(&mut rec, raw);
        let r = rec.remarks.unwrap();
        assert!(r.contains("Line one"));
        assert!(r.contains("Line two"));
        assert!(r.contains("Also a comment"));
        assert!(rec.remarks_enriched);
    }

    // ── Tests Dates (P2-ENRICH-009) ──────────────────────────────────────────

    #[test]
    fn test_dates_allocated_and_updated_at() {
        let mut rec = make_record();
        enrich_from_whois_raw(
            &mut rec,
            "RegDate: 2010-05-20\nUpdated: 2023-11-01\n",
        );
        assert_eq!(rec.allocated.as_deref(), Some("2010-05-20T00:00:00Z"));
        assert_eq!(rec.updated_at.as_deref(), Some("2023-11-01T00:00:00Z"));
        assert!(rec.dates_enriched);
    }

    #[test]
    fn test_dates_not_set_when_whois_has_none() {
        let mut rec = make_record();
        enrich_from_whois_raw(&mut rec, "netname: TEST\n");
        assert!(rec.allocated.is_none());
        assert!(rec.updated_at.is_none());
        assert!(!rec.dates_enriched);
    }
}
