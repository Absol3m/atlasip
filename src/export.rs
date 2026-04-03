use crate::models::IpRecord;
use anyhow::Result;

/// All export formats supported by AtlasIP (spec §2.3 & §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Tsv,
    Txt,
    HtmlVertical,
    HtmlHorizontal,
    Xml,
    WhoisRaw,
}

impl ExportFormat {
    /// Parse a format string coming from the CLI or API query parameter.
    pub fn from_str(s: &str) -> Result<Self> {
        // TODO: match "csv", "tsv", "txt", "html", "html-v", "html-h", "xml",
        // "whois" to the corresponding variant; return error for unknown values.
        todo!("ExportFormat::from_str")
    }
}

/// Render `records` in the requested `format`.
/// `with_header` applies only to CSV/TSV.
pub fn export(records: &[IpRecord], format: ExportFormat, with_header: bool) -> Result<String> {
    match format {
        ExportFormat::Csv => export_csv(records, with_header),
        ExportFormat::Tsv => export_tsv(records, with_header),
        ExportFormat::Txt => export_txt(records),
        ExportFormat::HtmlVertical => export_html_vertical(records),
        ExportFormat::HtmlHorizontal => export_html_horizontal(records),
        ExportFormat::Xml => export_xml(records),
        ExportFormat::WhoisRaw => export_whois_raw(records),
    }
}

fn export_csv(records: &[IpRecord], with_header: bool) -> Result<String> {
    // TODO: serialize records to CSV using the ordered column list from the
    // spec; include header row when `with_header` is true.
    todo!("export_csv")
}

fn export_tsv(records: &[IpRecord], with_header: bool) -> Result<String> {
    // TODO: same as CSV but tab-separated.
    todo!("export_tsv")
}

fn export_txt(records: &[IpRecord]) -> Result<String> {
    // TODO: plain-text tabular output.
    todo!("export_txt")
}

fn export_html_vertical(records: &[IpRecord]) -> Result<String> {
    // TODO: one record per table row (horizontal layout table).
    todo!("export_html_vertical")
}

fn export_html_horizontal(records: &[IpRecord]) -> Result<String> {
    // TODO: one record per column (vertical layout table).
    todo!("export_html_horizontal")
}

fn export_xml(records: &[IpRecord]) -> Result<String> {
    // TODO: serialize records as XML with one <record> element per IpRecord.
    todo!("export_xml")
}

fn export_whois_raw(records: &[IpRecord]) -> Result<String> {
    // TODO: concatenate raw_whois fields, separated by dashed lines.
    todo!("export_whois_raw")
}
