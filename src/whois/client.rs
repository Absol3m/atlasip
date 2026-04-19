use anyhow::{Context, Result};
use std::{net::IpAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Root WHOIS server — queried first to discover the authoritative RIR server.
const IANA_WHOIS: &str = "whois.iana.org";

const WHOIS_PORT: u16 = 43;

/// Hard cap on response size to prevent memory exhaustion.
const MAX_RESPONSE_BYTES: usize = 512 * 1024; // 512 KB

// ---------------------------------------------------------------------------
// Rate-limit / access-denied detection (P0-WHOIS-001, P0-NETWORK-001)
// ---------------------------------------------------------------------------

/// Return `Some(reason)` if the WHOIS response indicates the server has
/// rate-limited or blocked the query.  Used to set `status = failed`
/// rather than silently returning empty results (spec §2.4).
pub fn detect_rate_limit(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    // Common rate-limit / denial phrases across ARIN, RIPE, APNIC, LACNIC.
    let triggers: &[&str] = &[
        "rate limit exceeded",
        "query rate limit",
        "exceeded the maximum",
        "too many requests",
        "access denied",
        "access restricted",
        "blocked",
        "permission denied",
    ];
    if triggers.iter().any(|t| lower.contains(t)) {
        // Return the first non-comment content line as the reason.
        let reason = raw
            .lines()
            .map(str::trim)
            .find(|l| {
                !l.is_empty()
                    && !l.starts_with('%')
                    && !l.starts_with('#')
                    && triggers
                        .iter()
                        .any(|t| l.to_ascii_lowercase().contains(t))
            })
            .map(str::to_owned)
            .unwrap_or_else(|| crate::i18n::t("errors.error.whois.rate_limit_detected"));
        return Some(reason);
    }
    None
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Raw WHOIS response as returned by the wire.
#[derive(Debug, Clone)]
pub struct WhoisResult {
    /// Full text of the WHOIS response.
    pub raw: String,
    /// Hostname of the server that answered (used as `whois_source`).
    pub server: String,
}

/// All fields extracted from a raw WHOIS text response (spec §3 — IpRecord).
#[derive(Debug, Default, Clone)]
pub struct ParsedWhois {
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
// WhoisClient
// ---------------------------------------------------------------------------

pub struct WhoisClient {
    timeout_ms: u64,
}

impl WhoisClient {
    pub fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }

    /// Look up `target` (IP or hostname) via WHOIS.
    ///
    /// Flow (spec §2.1 — fallback WHOIS brut):
    /// 1. Query `whois.iana.org` to get a `refer:` pointing to the
    ///    authoritative RIR or registrar WHOIS server.
    /// 2. Query the referred server with the original target.
    /// 3. If the referral fails or returns no useful data, return the IANA
    ///    response as-is.
    ///
    /// Rate-limit / access-denied responses are propagated as errors so the
    /// caller can set `status = failed` (spec §2.4 — P0-WHOIS-001).
    pub async fn query(&self, target: &str) -> Result<WhoisResult> {
        let target = target.trim();

        // For domain targets, IANA needs only the TLD to return a referral.
        // For IPs, query the IP directly.
        let iana_query = match target.parse::<IpAddr>() {
            Ok(_) => target.to_owned(),
            Err(_) => target.rsplit('.').next().unwrap_or(target).to_owned(),
        };

        let iana_raw = self.raw_query(IANA_WHOIS, &iana_query).await?;

        // Follow one level of referral if present.
        if let Some(refer_server) = extract_refer(&iana_raw) {
            match self.raw_query(&refer_server, target).await {
                Ok(detailed) if has_useful_content(&detailed) => {
                    // Check for rate-limit before returning.
                    if let Some(reason) = detect_rate_limit(&detailed) {
                        anyhow::bail!("{}", crate::i18n::t("errors.error.whois.rate_limit").replace("{server}", &refer_server).replace("{reason}", &reason));
                    }
                    return Ok(WhoisResult {
                        raw: detailed,
                        server: refer_server,
                    });
                }
                _ => {
                    // Referral server unreachable or returned nothing useful.
                    // Fall through and return the IANA response.
                }
            }
        }

        Ok(WhoisResult {
            raw: iana_raw,
            server: IANA_WHOIS.to_owned(),
        })
    }

    /// Open a TCP connection to `server:43`, send `query\r\n`, read until EOF.
    async fn raw_query(&self, server: &str, query: &str) -> Result<String> {
        let addr = format!("{server}:{WHOIS_PORT}");
        let deadline = Duration::from_millis(self.timeout_ms);

        let mut stream = timeout(deadline, TcpStream::connect(&addr))
            .await
            .with_context(|| crate::i18n::t("errors.error.whois.connect_timeout").replace("{addr}", &addr))?
            .with_context(|| crate::i18n::t("errors.error.whois.connect_failed").replace("{addr}", &addr))?;

        let request = format!("{query}\r\n");
        timeout(deadline, stream.write_all(request.as_bytes()))
            .await
            .context(crate::i18n::t("errors.error.whois.write_timeout"))?
            .context(crate::i18n::t("errors.error.whois.write_failed"))?;

        let mut buf: Vec<u8> = Vec::with_capacity(8192);
        timeout(deadline, stream.read_to_end(&mut buf))
            .await
            .context(crate::i18n::t("errors.error.whois.read_timeout"))?
            .context(crate::i18n::t("errors.error.whois.read_failed"))?;

        buf.truncate(MAX_RESPONSE_BYTES);

        // WHOIS responses may be ISO-8859-1 or UTF-8 — use lossy conversion.
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    /// Parse a raw WHOIS text response into structured fields.
    ///
    /// Handles:
    /// - **RIPE-style** (RIPE, APNIC, AFRINIC): lowercase RPSL keys,
    ///   `inetnum:`, `netname:`, multi-line `address:`, `e-mail:`.
    /// - **ARIN-style**: CamelCase keys, `NetRange:`, `NetName:`, split
    ///   address (`Address:` + `City:` + `StateProv:` + `PostalCode:`).
    /// - **LACNIC-style**: `owner:`, `ownerid:`, `responsible:`, `inetnum:`
    ///   (P0-WHOIS-002 — multi-format support).
    ///
    /// Dates are normalised to ISO 8601 (P1-RDAP-004).
    /// Emails are fully de-duplicated (P1-WHOIS-003).
    pub fn parse(raw: &str) -> ParsedWhois {
        let mut parsed = ParsedWhois::default();

        // First contiguous block of `address:` lines (RIPE-style).
        let mut first_addr_block: Vec<String> = Vec::new();
        let mut collecting_addr = false;
        let mut addr_done = false;

        // ARIN splits the address across multiple keys.
        let mut arin_city: Option<String> = None;
        let mut arin_state: Option<String> = None;

        for line in raw.lines() {
            let line = line.trim();

            // Blank line → end of RPSL object; stop any in-progress address block.
            if line.is_empty() {
                if collecting_addr {
                    collecting_addr = false;
                    addr_done = true;
                }
                continue;
            }

            // Comment lines (RIPE uses `%`, ARIN uses `#`).
            if line.starts_with('%') || line.starts_with('#') {
                continue;
            }

            let (key, value) = match line.split_once(':') {
                Some((k, v)) => (k.trim(), v.trim()),
                None => continue,
            };

            if value.is_empty() {
                if collecting_addr {
                    // A non-address key stops the current address block.
                    collecting_addr = false;
                    addr_done = true;
                }
                continue;
            }

            let key_lc = key.to_ascii_lowercase();

            // Any key that is not `address` ends the address collection.
            if key_lc != "address" && collecting_addr {
                collecting_addr = false;
                addr_done = true;
            }

            match key_lc.as_str() {
                // ── Country ──────────────────────────────────────────────────
                // `Country:` is the canonical key; `Ctry:` is a common alias.
                "country" | "ctry" => {
                    parsed.country.get_or_insert_with(|| value.to_owned());
                }

                // ── Network name ──────────────────────────────────────────────
                // RIPE: `netname:` / ARIN: `NetName:`
                "netname" | "net-name" => {
                    parsed.network_name.get_or_insert_with(|| value.to_owned());
                }

                // ── Owner / organisation name ─────────────────────────────────
                // Priority order (highest → lowest):
                //   orgname / org-name  >  owner (LACNIC)  >
                //   organization  >  descr
                //
                // RIPE response: `descr:` appears before `org-name:`.
                // ARIN response: `Organization:` appears before `OrgName:`.
                // LACNIC response: `owner:` is the primary org key.
                // In all cases the more-specific key must win.
                "orgname" | "org-name" => {
                    // Unconditional overwrite — this is the canonical org name.
                    parsed.owner_name = Some(value.to_owned());
                }
                // LACNIC: `owner:` is equivalent to orgname.
                "owner" => {
                    // Use get_or_insert so orgname still wins if seen first.
                    parsed.owner_name.get_or_insert_with(|| value.to_owned());
                }
                "organization" | "organisation" => {
                    // Lower priority than orgname / owner — only set if not yet known.
                    parsed.owner_name.get_or_insert_with(|| value.to_owned());
                }
                // `descr:` used as owner_name fallback of last resort.
                "descr" => {
                    if parsed.owner_name.is_none() {
                        parsed.owner_name = Some(value.to_owned());
                    }
                }

                // ── Address ────────────────────────────────────────────────────
                // RIPE: consecutive `address:` lines form the postal address.
                // ARIN: a single `Address:` line is the street; city/state follow.
                "address" => {
                    if !addr_done {
                        first_addr_block.push(value.to_owned());
                        collecting_addr = true;
                    }
                }

                // ARIN address components.
                "city" => {
                    arin_city.get_or_insert_with(|| value.to_owned());
                }
                "stateprov" | "statename" | "state-province" => {
                    arin_state.get_or_insert_with(|| value.to_owned());
                }

                // ── Postal code ───────────────────────────────────────────────
                "postalcode" | "postal-code" => {
                    parsed.postal_code.get_or_insert_with(|| value.to_owned());
                }

                // ── Emails ────────────────────────────────────────────────────
                // RIPE general contact email.
                "e-mail" | "email" => {
                    push_unique(&mut parsed.emails, value);
                }
                // RIPE / generic abuse mailbox.
                "abuse-mailbox" | "abuse-email" => {
                    push_unique(&mut parsed.abuse_emails, value);
                }
                // ARIN org-level emails.
                "orgabuseemail" | "orgabusemail" => {
                    push_unique(&mut parsed.abuse_emails, value);
                }
                "orgtechemail" | "orgadminemail" => {
                    push_unique(&mut parsed.emails, value);
                }

                // ── Phone / fax (P2-WHOIS-007) ────────────────────────────────
                // RIPE: `phone:` / ARIN: `OrgAbusePhone:`, `OrgTechPhone:`
                // LACNIC: `phone:` (same key, handled generically)
                "phone" | "orgabusephone" | "orgtechphone" | "orgadminphone" => {
                    parsed.phone.get_or_insert_with(|| value.to_owned());
                }
                "fax-no" | "fax" | "fax-number" => {
                    parsed.fax.get_or_insert_with(|| value.to_owned());
                }

                // ── IP range ──────────────────────────────────────────────────
                // RIPE: `inetnum: x.x.x.x - y.y.y.y`
                // ARIN: `NetRange: x.x.x.x - y.y.y.y`
                "inetnum" | "inet6num" | "netrange" => {
                    let (from, to) = split_ip_range(value);
                    parsed.from_ip.get_or_insert_with(|| from);
                    if let Some(t) = to {
                        parsed.to_ip.get_or_insert_with(|| t);
                    }
                }

                // ── CIDR ──────────────────────────────────────────────────────
                // ARIN: `CIDR: x.x.x.x/24`
                "cidr" => {
                    parsed.cidr.get_or_insert_with(|| value.to_owned());
                }
                // RIPE: `route:` or `route6:` carries the announced prefix.
                "route" | "route6" => {
                    parsed.cidr.get_or_insert_with(|| value.to_owned());
                }

                // ── Status ────────────────────────────────────────────────────
                "status" => {
                    parsed.status.get_or_insert_with(|| value.to_owned());
                }

                // ── Allocation date (normalised to ISO 8601) ─────────────────
                // ARIN: `RegDate:` / RIPE/LACNIC: `created:` (P1-RDAP-004).
                "regdate" | "created" | "registration-date" => {
                    parsed.allocated.get_or_insert_with(|| {
                        crate::utils::normalize_date(value)
                    });
                }

                // ── Contact name ──────────────────────────────────────────────
                // RIPE: `person:` or `role:` (contact object)
                "person" | "role" => {
                    parsed.contact_name.get_or_insert_with(|| value.to_owned());
                }
                // ARIN: `OrgTechName:` / `OrgAdminName:`
                "orgtechname" | "orgadminname" => {
                    parsed.contact_name.get_or_insert_with(|| value.to_owned());
                }
                // LACNIC: `responsible:` is the contact person / NOC name.
                "responsible" => {
                    parsed.contact_name.get_or_insert_with(|| value.to_owned());
                }

                // ── Abuse contact ─────────────────────────────────────────────
                // RIPE: `abuse-c:` (a handle, not a name — best we can do here)
                "abuse-c" => {
                    parsed.abuse_contact.get_or_insert_with(|| value.to_owned());
                }
                // ARIN: `OrgAbuseName:`
                "orgabusename" => {
                    // Prefer this over the bare handle from `abuse-c:`.
                    parsed.abuse_contact = Some(value.to_owned());
                }

                _ => {}
            }
        }

        // ── Post-processing: build the final address string ─────────────────

        if arin_city.is_some() || arin_state.is_some() {
            // ARIN format: combine street + city + state + postal + country.
            let street = first_addr_block.first().cloned();
            let components: Vec<String> = [
                street,
                arin_city,
                arin_state,
                parsed.postal_code.clone(),
                parsed.country.clone(),
            ]
            .into_iter()
            .flatten()
            .collect();
            if !components.is_empty() {
                parsed.address = Some(components.join(", "));
            }
        } else if !first_addr_block.is_empty() {
            // RIPE format: join the consecutive address lines.
            parsed.address = Some(first_addr_block.join(", "));
        }

        parsed
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Look for a `refer:`, `whois:`, or `ReferralServer:` field in a WHOIS
/// response and return the hostname of the referred server.
fn extract_refer(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let line = line.trim();
        let (key, value) = match line.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        match key.to_ascii_lowercase().as_str() {
            // IANA responses: `refer: whois.ripe.net`
            "refer" | "whois" => {
                if !value.is_empty() {
                    return Some(value.to_owned());
                }
            }
            // ARIN responses: `ReferralServer: rwhois://rwhois.example.net:4321`
            "referralserver" => {
                // Strip scheme (rwhois:// or whois://) and port.
                let host = value
                    .trim_start_matches("rwhois://")
                    .trim_start_matches("whois://")
                    .split(':')
                    .next()
                    .unwrap_or(value)
                    .trim();
                if !host.is_empty() {
                    return Some(host.to_owned());
                }
            }
            _ => {}
        }
    }
    None
}

/// Return `true` if the response contains at least one non-comment,
/// non-empty content line (i.e. is worth returning to the caller).
fn has_useful_content(raw: &str) -> bool {
    raw.lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('%') && !t.starts_with('#')
        })
        .count()
        > 3
}

/// Split `"x.x.x.x - y.y.y.y"` into `(from, Some(to))`.
/// For bare CIDRs or single IPs, returns `(value, None)`.
fn split_ip_range(s: &str) -> (String, Option<String>) {
    match s.split_once(" - ") {
        Some((from, to)) => (from.trim().to_owned(), Some(to.trim().to_owned())),
        None => (s.trim().to_owned(), None),
    }
}

/// Append `value` to `vec` only if it is not already present.
fn push_unique(vec: &mut Vec<String>, value: &str) {
    let owned = value.to_owned();
    if !vec.contains(&owned) {
        vec.push(owned);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Unit tests (no network) ──────────────────────────────────────────────

    #[test]
    fn test_extract_refer_iana_style() {
        let raw = "% IANA WHOIS server\nrefer: whois.arin.net\ninetnum: 8.0.0.0\n";
        assert_eq!(extract_refer(raw), Some("whois.arin.net".to_owned()));
    }

    #[test]
    fn test_extract_refer_whois_key() {
        let raw = "whois: whois.ripe.net\nstatus: ASSIGNED\n";
        assert_eq!(extract_refer(raw), Some("whois.ripe.net".to_owned()));
    }

    #[test]
    fn test_extract_refer_rwhois_scheme() {
        let raw = "ReferralServer: rwhois://rwhois.example.net:4321\n";
        assert_eq!(
            extract_refer(raw),
            Some("rwhois.example.net".to_owned())
        );
    }

    #[test]
    fn test_extract_refer_none() {
        let raw = "% No referral here\nnetname: TEST\n";
        assert_eq!(extract_refer(raw), None);
    }

    #[test]
    fn test_split_ip_range_dash() {
        let (from, to) = split_ip_range("8.8.8.0 - 8.8.8.255");
        assert_eq!(from, "8.8.8.0");
        assert_eq!(to, Some("8.8.8.255".to_owned()));
    }

    #[test]
    fn test_split_ip_range_cidr() {
        let (from, to) = split_ip_range("8.8.8.0/24");
        assert_eq!(from, "8.8.8.0/24");
        assert_eq!(to, None);
    }

    #[test]
    fn test_parse_arin_style() {
        let raw = "\
# ARIN WHOIS
NetRange:       8.8.8.0 - 8.8.8.255
CIDR:           8.8.8.0/24
NetName:        GOGL
Organization:   Google LLC (GOGL)
RegDate:        2023-12-28
Updated:        2023-12-28

OrgName:        Google LLC
OrgId:          GOGL
Address:        1600 Amphitheatre Parkway
City:           Mountain View
StateProv:      CA
PostalCode:     94043
Country:        US
OrgTechEmail:   arin-contact@google.com
OrgAbuseEmail:  network-abuse@google.com
OrgAbuseName:   Abuse
OrgAbusePhone:  +1-650-253-0000
";
        let p = WhoisClient::parse(raw);

        assert_eq!(p.from_ip.as_deref(), Some("8.8.8.0"));
        assert_eq!(p.to_ip.as_deref(), Some("8.8.8.255"));
        assert_eq!(p.cidr.as_deref(), Some("8.8.8.0/24"));
        assert_eq!(p.network_name.as_deref(), Some("GOGL"));
        assert_eq!(p.owner_name.as_deref(), Some("Google LLC"));
        assert_eq!(p.country.as_deref(), Some("US"));
        assert_eq!(p.postal_code.as_deref(), Some("94043"));
        // Date is normalised to ISO 8601 (P1-RDAP-004).
        assert_eq!(p.allocated.as_deref(), Some("2023-12-28T00:00:00Z"));
        assert!(p.emails.contains(&"arin-contact@google.com".to_owned()));
        assert!(p.abuse_emails.contains(&"network-abuse@google.com".to_owned()));
        assert_eq!(p.abuse_contact.as_deref(), Some("Abuse"));
        assert_eq!(p.phone.as_deref(), Some("+1-650-253-0000"));
        // Address should combine street + city + state + postal + country.
        let addr = p.address.unwrap();
        assert!(addr.contains("1600 Amphitheatre Parkway"), "addr={addr}");
        assert!(addr.contains("Mountain View"), "addr={addr}");
        assert!(addr.contains("CA"), "addr={addr}");
    }

    #[test]
    fn test_parse_ripe_style() {
        let raw = "\
% RIPE Database
inetnum:        193.0.0.0 - 193.0.7.255
netname:        RIPE-NCC
descr:          RIPE Network Coordination Centre
country:        NL
status:         ASSIGNED PA
created:        2003-03-17T12:15:57Z

org-name:       Reseaux IP Europeens Network Coordination Centre (RIPE NCC)
address:        P.O. Box 10096
address:        1001 EB
address:        Amsterdam
address:        NETHERLANDS
phone:          +31205354444
fax-no:         +31205354445
abuse-mailbox:  abuse@ripe.net

role:           Managing Director
address:        RIPE NCC HQ
address:        Amsterdam
";
        let p = WhoisClient::parse(raw);

        assert_eq!(p.from_ip.as_deref(), Some("193.0.0.0"));
        assert_eq!(p.to_ip.as_deref(), Some("193.0.7.255"));
        assert_eq!(p.network_name.as_deref(), Some("RIPE-NCC"));
        assert_eq!(p.country.as_deref(), Some("NL"));
        assert_eq!(p.status.as_deref(), Some("ASSIGNED PA"));
        assert_eq!(p.allocated.as_deref(), Some("2003-03-17T12:15:57Z"));
        assert_eq!(p.owner_name.as_deref(), Some("Reseaux IP Europeens Network Coordination Centre (RIPE NCC)"));
        assert_eq!(p.phone.as_deref(), Some("+31205354444"));
        assert_eq!(p.fax.as_deref(), Some("+31205354445"));
        assert!(p.abuse_emails.contains(&"abuse@ripe.net".to_owned()));
        // Only the first contiguous address block should be taken (org section).
        let addr = p.address.as_deref().unwrap();
        assert!(addr.contains("P.O. Box 10096"), "addr={addr}");
        assert!(addr.contains("Amsterdam"), "addr={addr}");
        // The role's address lines must NOT be included.
        assert!(!addr.contains("RIPE NCC HQ"), "addr={addr}");
    }

    #[test]
    fn test_parse_empty_response() {
        let p = WhoisClient::parse("% No objects found.\n");
        assert!(p.country.is_none());
        assert!(p.from_ip.is_none());
        assert!(p.emails.is_empty());
    }

    // ── LACNIC format (P0-WHOIS-002) ────────────────────────────────────────

    #[test]
    fn test_parse_lacnic_style() {
        let raw = "\
% Joint Whois - whois.lacnic.net
% This server accepts single ASN, IPv4 or IPv6 queries

inetnum:     200.160.0.0/20
status:      reallocated
owner:       NIC.br
ownerid:     005.506.560/0001-36
country:     BR
owner-c:     NICBR
responsible: Carlos Afonso
address:     Av. das Nações Unidas, 11541
address:     Brooklin
phone:       +55 11 5509-3500
fax-no:      +55 11 5509-3501
e-mail:      noc@nic.br
abuse-mailbox: cert@cert.br
created:     19970401
changed:     20230101
";
        let p = WhoisClient::parse(raw);

        assert_eq!(p.from_ip.as_deref(), Some("200.160.0.0/20"),
            "LACNIC inetnum (CIDR) should be in from_ip");
        assert_eq!(p.country.as_deref(), Some("BR"));
        assert_eq!(p.owner_name.as_deref(), Some("NIC.br"));
        assert_eq!(p.contact_name.as_deref(), Some("Carlos Afonso"),
            "responsible: should map to contact_name");
        assert_eq!(p.phone.as_deref(), Some("+55 11 5509-3500"));
        assert_eq!(p.fax.as_deref(), Some("+55 11 5509-3501"));
        assert!(p.emails.contains(&"noc@nic.br".to_owned()),
            "e-mail should be in emails");
        assert!(p.abuse_emails.contains(&"cert@cert.br".to_owned()),
            "abuse-mailbox should be in abuse_emails");
    }

    // ── Rate-limit detection (P0-WHOIS-001 / spec §2.4) ─────────────────────

    #[test]
    fn test_detect_rate_limit_arin() {
        let raw = "% Error: rate limit exceeded. Please try again later.\n";
        assert!(detect_rate_limit(raw).is_some());
    }

    #[test]
    fn test_detect_rate_limit_access_denied() {
        let raw = "% Access Denied.\ninetnum: 8.0.0.0 - 8.255.255.255\n";
        assert!(detect_rate_limit(raw).is_some());
    }

    #[test]
    fn test_detect_rate_limit_none_for_normal_response() {
        let raw = "netname: GOGL\ncountry: US\nstatus: ASSIGNED\n";
        assert!(detect_rate_limit(raw).is_none());
    }

    #[test]
    fn test_detect_rate_limit_too_many_requests() {
        let raw = "% Too many requests. Please slow down.\n";
        assert!(detect_rate_limit(raw).is_some());
    }

    // ── Date normalisation in WHOIS (P1-RDAP-004) ────────────────────────────

    #[test]
    fn test_parse_whois_date_normalization() {
        let raw = "netname: TEST\nregdate: 2023-12-28\nstatus: ASSIGNED\n";
        let p = WhoisClient::parse(raw);
        assert_eq!(p.allocated.as_deref(), Some("2023-12-28T00:00:00Z"));
    }

    #[test]
    fn test_parse_whois_date_already_iso() {
        let raw = "netname: TEST\ncreated: 2003-03-17T12:15:57Z\nstatus: ASSIGNED\n";
        let p = WhoisClient::parse(raw);
        assert_eq!(p.allocated.as_deref(), Some("2003-03-17T12:15:57Z"));
    }

    // ── Multi-email extraction (P1-WHOIS-003) ────────────────────────────────

    #[test]
    fn test_parse_multi_email() {
        let raw = "\
inetnum: 10.0.0.0 - 10.255.255.255
e-mail: first@example.com
e-mail: second@example.com
e-mail: first@example.com
";
        let p = WhoisClient::parse(raw);
        // De-duplicated; order preserved.
        assert_eq!(p.emails.len(), 2, "Expected de-duplicated emails");
        assert!(p.emails.contains(&"first@example.com".to_owned()));
        assert!(p.emails.contains(&"second@example.com".to_owned()));
    }

    // ── Integration tests (real network) ────────────────────────────────────

    #[tokio::test]
    async fn test_query_google_dns() {
        let client = WhoisClient::new(5000);
        let result = client.query("8.8.8.8").await.unwrap();

        // Should have been answered by ARIN.
        assert!(
            result.server.contains("arin.net"),
            "Expected ARIN server, got: {}",
            result.server
        );
        // Raw response must be non-empty and contain the IP block.
        assert!(result.raw.contains("8.8.8"), "Raw WHOIS missing IP data");

        let parsed = WhoisClient::parse(&result.raw);
        assert_eq!(parsed.from_ip.as_deref(), Some("8.8.8.0"));
        assert_eq!(parsed.to_ip.as_deref(), Some("8.8.8.255"));
        assert_eq!(parsed.cidr.as_deref(), Some("8.8.8.0/24"));
        assert_eq!(parsed.country.as_deref(), Some("US"));
        assert!(parsed.owner_name.is_some(), "Expected an owner name");
    }

    #[tokio::test]
    async fn test_query_ripe_ip() {
        let client = WhoisClient::new(5000);
        let result = client.query("193.0.6.1").await.unwrap();

        assert!(
            result.server.contains("ripe.net"),
            "Expected RIPE server, got: {}",
            result.server
        );

        let parsed = WhoisClient::parse(&result.raw);
        assert_eq!(parsed.country.as_deref(), Some("NL"));
        assert!(parsed.network_name.is_some());
        assert!(parsed.from_ip.is_some());
    }

    #[tokio::test]
    async fn test_query_invalid_gives_error_or_empty() {
        // Querying a documentation IP (RFC 5737) should not panic.
        let client = WhoisClient::new(5000);
        let result = client.query("192.0.2.1").await;
        // Either an error or an empty parsed result is acceptable.
        if let Ok(r) = result {
            let p = WhoisClient::parse(&r.raw);
            // No owner info expected for this reserved range.
            let _ = p; // just assert it doesn't panic
        }
    }
}
