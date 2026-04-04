use crate::models::{IpRecord, COLUMNS};
use std::fmt;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum ExportError {
    InvalidFormat(String),
    SerializationError(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportError::InvalidFormat(s) => write!(f, "Unknown export format: {s}"),
            ExportError::SerializationError(s) => write!(f, "Serialization error: {s}"),
        }
    }
}

// anyhow's blanket `impl<E: Error + Send + Sync + 'static> From<E> for anyhow::Error`
// covers ExportError automatically — no manual impl needed.
impl std::error::Error for ExportError {}

// ---------------------------------------------------------------------------
// Format enum
// ---------------------------------------------------------------------------

/// All export formats supported by AtlasIP (spec §2.3 & §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Tsv,
    /// Plain-text vertical "fiche" (one field per line).
    Txt,
    /// Plain-text horizontal table (columns separated by " | ").
    TxtHorizontal,
    /// HTML fiche: `<div class="record">…</div>` per record.
    HtmlVertical,
    /// HTML table: `<table><thead>…</thead><tbody>…</tbody></table>`.
    HtmlHorizontal,
    Xml,
    WhoisRaw,
}

impl ExportFormat {
    /// Parse a format string coming from the CLI or API query parameter.
    ///
    /// Accepted values (case-insensitive):
    /// `csv`, `tsv`, `txt` / `txt-v`, `txt-h`, `html` / `html-h`,
    /// `html-v`, `xml`, `whois`.
    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "csv" => Ok(Self::Csv),
            "tsv" => Ok(Self::Tsv),
            "txt" | "txt-v" => Ok(Self::Txt),
            "txt-h" => Ok(Self::TxtHorizontal),
            "html" | "html-h" => Ok(Self::HtmlHorizontal),
            "html-v" => Ok(Self::HtmlVertical),
            "xml" => Ok(Self::Xml),
            "whois" => Ok(Self::WhoisRaw),
            _ => Err(ExportError::InvalidFormat(s.to_owned()).into()),
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Render `records` in the requested `format`.
///
/// `with_header` controls whether a header row is emitted for CSV/TSV.
pub fn export(
    records: &[IpRecord],
    format: ExportFormat,
    with_header: bool,
) -> anyhow::Result<String> {
    match format {
        ExportFormat::Csv => export_csv(records, with_header),
        ExportFormat::Tsv => export_tsv(records, with_header),
        ExportFormat::Txt => export_txt_vertical(records),
        ExportFormat::TxtHorizontal => export_txt_horizontal(records),
        ExportFormat::HtmlVertical => export_html_vertical(records),
        ExportFormat::HtmlHorizontal => export_html_horizontal(records),
        ExportFormat::Xml => export_xml(records),
        ExportFormat::WhoisRaw => export_whois_raw(records),
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_xml(s: &str) -> String {
    // XML escaping rules are identical to HTML for the five special characters.
    escape_html(s)
}

/// Convert a COLUMNS label to a lowercase-underscore XML element name.
/// e.g. "IP Address" → "ip_address", "Owner Name" → "owner_name".
fn col_to_xml_tag(col: &str) -> String {
    col.to_ascii_lowercase().replace(' ', "_")
}

// ---------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------

fn export_csv(records: &[IpRecord], with_header: bool) -> anyhow::Result<String> {
    let mut out = String::new();

    if with_header {
        out.push_str(&COLUMNS.join(","));
        out.push('\n');
    }

    for rec in records {
        let row: Vec<String> = rec
            .to_row()
            .into_iter()
            .map(|v| csv_quote(&v))
            .collect();
        out.push_str(&row.join(","));
        out.push('\n');
    }

    Ok(out)
}

/// Wrap a CSV field in double-quotes if it contains a comma, double-quote,
/// or newline; escape inner double-quotes by doubling them.
fn csv_quote(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

// ---------------------------------------------------------------------------
// TSV
// ---------------------------------------------------------------------------

fn export_tsv(records: &[IpRecord], with_header: bool) -> anyhow::Result<String> {
    let mut out = String::new();

    if with_header {
        out.push_str(&COLUMNS.join("\t"));
        out.push('\n');
    }

    for rec in records {
        out.push_str(&rec.to_row().join("\t"));
        out.push('\n');
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// TXT — vertical "fiche"
// ---------------------------------------------------------------------------

fn export_txt_vertical(records: &[IpRecord]) -> anyhow::Result<String> {
    let fiches: Vec<String> = records
        .iter()
        .map(|rec| {
            let row = rec.to_row();
            COLUMNS
                .iter()
                .zip(row.iter())
                .map(|(col, val)| format!("{col}: {val}"))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect();

    Ok(fiches.join("\n\n"))
}

// ---------------------------------------------------------------------------
// TXT — horizontal table
// ---------------------------------------------------------------------------

fn export_txt_horizontal(records: &[IpRecord]) -> anyhow::Result<String> {
    // Compute column widths: max(header length, widest value).
    let n = COLUMNS.len();
    let rows: Vec<Vec<String>> = records.iter().map(|r| r.to_row()).collect();

    let widths: Vec<usize> = (0..n)
        .map(|i| {
            let header_w = COLUMNS[i].len();
            let data_w = rows.iter().map(|r| r[i].len()).max().unwrap_or(0);
            header_w.max(data_w)
        })
        .collect();

    let mut out = String::new();

    // Header row.
    let header: Vec<String> = COLUMNS
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{:<width$}", col, width = widths[i]))
        .collect();
    out.push_str(&header.join(" | "));
    out.push('\n');

    // Separator.
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    out.push_str(&sep.join("-+-"));
    out.push('\n');

    // Data rows.
    for row in &rows {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, val)| format!("{:<width$}", val, width = widths[i]))
            .collect();
        out.push_str(&cells.join(" | "));
        out.push('\n');
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// HTML — vertical fiche
// ---------------------------------------------------------------------------

fn export_html_vertical(records: &[IpRecord]) -> anyhow::Result<String> {
    let mut out = String::new();

    for rec in records {
        let row = rec.to_row();
        out.push_str(&format!(
            "<div class=\"record\"><h2>{}</h2><ul>\n",
            escape_html(&rec.ip)
        ));
        for (col, val) in COLUMNS.iter().zip(row.iter()) {
            out.push_str(&format!(
                "  <li><b>{}:</b> {}</li>\n",
                escape_html(col),
                escape_html(val)
            ));
        }
        out.push_str("</ul></div>\n");
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// HTML — horizontal table
// ---------------------------------------------------------------------------

fn export_html_horizontal(records: &[IpRecord]) -> anyhow::Result<String> {
    let mut out = String::from("<table>\n<thead>\n<tr>");

    for col in COLUMNS {
        out.push_str(&format!("<th>{}</th>", escape_html(col)));
    }
    out.push_str("</tr>\n</thead>\n<tbody>\n");

    for rec in records {
        out.push_str("<tr>");
        for val in rec.to_row() {
            out.push_str(&format!("<td>{}</td>", escape_html(&val)));
        }
        out.push_str("</tr>\n");
    }

    out.push_str("</tbody>\n</table>");
    Ok(out)
}

// ---------------------------------------------------------------------------
// XML
// ---------------------------------------------------------------------------

fn export_xml(records: &[IpRecord]) -> anyhow::Result<String> {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<records>\n");

    for rec in records {
        out.push_str("  <record>\n");
        for (col, val) in COLUMNS.iter().zip(rec.to_row().iter()) {
            let tag = col_to_xml_tag(col);
            out.push_str(&format!(
                "    <{tag}>{}</{tag}>\n",
                escape_xml(val)
            ));
        }
        out.push_str("  </record>\n");
    }

    out.push_str("</records>");
    Ok(out)
}

// ---------------------------------------------------------------------------
// WHOIS raw
// ---------------------------------------------------------------------------

fn export_whois_raw(records: &[IpRecord]) -> anyhow::Result<String> {
    let parts: Vec<&str> = records
        .iter()
        .filter_map(|r| r.raw_whois.as_deref())
        .collect();

    Ok(parts.join("\n\n-----\n\n"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::IpRecord;

    fn make_record_a() -> IpRecord {
        let mut r = IpRecord::new(1, "8.8.8.8");
        r.country = Some("US".into());
        r.owner_name = Some("Google LLC".into());
        r.emails = vec!["dns-admin@google.com".into()];
        r.abuse_emails = vec!["abuse@google.com".into()];
        r.network_name = Some("GOOGLE".into());
        r.cidr = Some("8.8.8.0/24".into());
        r.raw_whois = Some("% ARIN WHOIS data\nnetname: GOOGLE".into());
        r
    }

    fn make_record_b() -> IpRecord {
        let mut r = IpRecord::new(2, "1.1.1.1");
        r.country = Some("AU".into());
        r.owner_name = Some("Cloudflare & Co <test>".into()); // special chars for HTML/XML
        r.raw_whois = Some("% APNIC WHOIS\nnetname: APNIC-LABS".into());
        r
    }

    // -----------------------------------------------------------------------

    #[test]
    fn test_export_csv() {
        let records = vec![make_record_a()];
        let out = export(&records, ExportFormat::Csv, true).unwrap();
        let lines: Vec<&str> = out.lines().collect();

        // Header present and has correct number of columns.
        assert_eq!(lines[0].split(',').count(), COLUMNS.len());
        assert!(lines[0].starts_with("Order,"));

        // Data row: order=1, ip=8.8.8.8.
        assert!(lines[1].starts_with("1,8.8.8.8,"));
    }

    #[test]
    fn test_export_csv_no_header() {
        let records = vec![make_record_a()];
        let out = export(&records, ExportFormat::Csv, false).unwrap();
        let first = out.lines().next().unwrap();
        // First line is data, not header.
        assert!(first.starts_with("1,"));
    }

    #[test]
    fn test_export_csv_quotes() {
        // Values containing commas must be quoted.
        let mut r = IpRecord::new(1, "1.2.3.4");
        r.address = Some("123 Main St, Suite 4".into());
        let out = export(&[r], ExportFormat::Csv, false).unwrap();
        assert!(out.contains("\"123 Main St, Suite 4\""));
    }

    #[test]
    fn test_export_tsv() {
        let records = vec![make_record_a(), make_record_b()];
        let out = export(&records, ExportFormat::Tsv, true).unwrap();
        let lines: Vec<&str> = out.lines().collect();

        assert_eq!(lines.len(), 3); // header + 2 records
        assert_eq!(lines[0].split('\t').count(), COLUMNS.len());
        assert!(lines[1].contains("8.8.8.8"));
        assert!(lines[2].contains("1.1.1.1"));
    }

    #[test]
    fn test_export_txt_vertical() {
        let records = vec![make_record_a()];
        let out = export(&records, ExportFormat::Txt, true).unwrap();

        assert!(out.contains("IP Address: 8.8.8.8"));
        assert!(out.contains("Country: US"));
        assert!(out.contains("Owner Name: Google LLC"));
    }

    #[test]
    fn test_export_txt_vertical_two_records() {
        let records = vec![make_record_a(), make_record_b()];
        let out = export(&records, ExportFormat::Txt, true).unwrap();
        // Records are separated by blank line.
        assert!(out.contains("\n\n"));
        assert!(out.contains("8.8.8.8"));
        assert!(out.contains("1.1.1.1"));
    }

    #[test]
    fn test_export_txt_horizontal() {
        let records = vec![make_record_a(), make_record_b()];
        let out = export(&records, ExportFormat::TxtHorizontal, true).unwrap();
        let lines: Vec<&str> = out.lines().collect();

        // header + separator + 2 data rows.
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("IP Address"));
        // Separator is dashes.
        assert!(lines[1].chars().all(|c| c == '-' || c == '+' || c == ' '));
        assert!(lines[2].contains("8.8.8.8"));
        assert!(lines[3].contains("1.1.1.1"));
    }

    #[test]
    fn test_export_html_vertical() {
        let records = vec![make_record_b()]; // has special chars
        let out = export(&records, ExportFormat::HtmlVertical, true).unwrap();

        assert!(out.contains("<div class=\"record\">"));
        assert!(out.contains("<h2>1.1.1.1</h2>"));
        assert!(out.contains("<ul>"));
        assert!(out.contains("<li>"));
        // Special characters must be escaped.
        assert!(out.contains("&amp;"));
        assert!(out.contains("&lt;"));
        assert!(out.contains("&gt;"));
        assert!(!out.contains("Cloudflare & Co"));
    }

    #[test]
    fn test_export_html_horizontal() {
        let records = vec![make_record_a(), make_record_b()];
        let out = export(&records, ExportFormat::HtmlHorizontal, true).unwrap();

        assert!(out.contains("<table>"));
        assert!(out.contains("<thead>"));
        assert!(out.contains("<tbody>"));
        assert!(out.contains("<th>IP Address</th>"));
        assert!(out.contains("<td>8.8.8.8</td>"));
        // Special characters escaped.
        assert!(!out.contains("Cloudflare & Co"));
        assert!(out.contains("&amp;"));
    }

    #[test]
    fn test_export_xml() {
        let records = vec![make_record_a(), make_record_b()];
        let out = export(&records, ExportFormat::Xml, true).unwrap();

        assert!(out.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(out.contains("<records>"));
        assert!(out.contains("<record>"));
        assert!(out.contains("<ip_address>8.8.8.8</ip_address>"));
        assert!(out.contains("<country>US</country>"));
        assert!(out.contains("<owner_name>Google LLC</owner_name>"));
        // Special characters escaped.
        assert!(!out.contains("Cloudflare & Co"));
        assert!(out.contains("&amp;"));
    }

    #[test]
    fn test_export_whois_raw() {
        let records = vec![make_record_a(), make_record_b()];
        let out = export(&records, ExportFormat::WhoisRaw, true).unwrap();

        assert!(out.contains("ARIN WHOIS"));
        assert!(out.contains("APNIC WHOIS"));
        assert!(out.contains("\n\n-----\n\n"));
    }

    #[test]
    fn test_export_whois_raw_skips_none() {
        let mut r = IpRecord::new(1, "10.0.0.1");
        r.raw_whois = None;
        let r2 = make_record_a();
        let out = export(&[r, r2], ExportFormat::WhoisRaw, true).unwrap();
        // Only one block — the None record is silently skipped.
        assert!(!out.contains("-----"));
        assert!(out.contains("ARIN WHOIS"));
    }

    #[test]
    fn test_from_str_valid() {
        assert_eq!(ExportFormat::from_str("csv").unwrap(), ExportFormat::Csv);
        assert_eq!(ExportFormat::from_str("CSV").unwrap(), ExportFormat::Csv);
        assert_eq!(ExportFormat::from_str("tsv").unwrap(), ExportFormat::Tsv);
        assert_eq!(ExportFormat::from_str("txt").unwrap(), ExportFormat::Txt);
        assert_eq!(ExportFormat::from_str("txt-v").unwrap(), ExportFormat::Txt);
        assert_eq!(ExportFormat::from_str("txt-h").unwrap(), ExportFormat::TxtHorizontal);
        assert_eq!(ExportFormat::from_str("html").unwrap(), ExportFormat::HtmlHorizontal);
        assert_eq!(ExportFormat::from_str("html-h").unwrap(), ExportFormat::HtmlHorizontal);
        assert_eq!(ExportFormat::from_str("html-v").unwrap(), ExportFormat::HtmlVertical);
        assert_eq!(ExportFormat::from_str("xml").unwrap(), ExportFormat::Xml);
        assert_eq!(ExportFormat::from_str("whois").unwrap(), ExportFormat::WhoisRaw);
    }

    #[test]
    fn test_from_str_invalid() {
        assert!(ExportFormat::from_str("pdf").is_err());
        assert!(ExportFormat::from_str("").is_err());
    }
}
