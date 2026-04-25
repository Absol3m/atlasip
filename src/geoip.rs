use std::{
    fs,
    io,
    net::IpAddr,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};

const DB_FILENAME: &str = "GeoLite2-City.mmdb";
const MAX_AGE_SECS: u64 = 24 * 3_600;

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

pub fn db_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| PathBuf::from(".config"))
        });
    base.join("atlasip").join("geoip")
}

pub fn db_path() -> PathBuf {
    db_dir().join(DB_FILENAME)
}

// ---------------------------------------------------------------------------
// Freshness check
// ---------------------------------------------------------------------------

pub fn needs_update() -> bool {
    let path = db_path();
    if !path.exists() {
        return true;
    }
    match fs::metadata(&path).and_then(|m| m.modified()) {
        Ok(modified) => {
            SystemTime::now()
                .duration_since(modified)
                .unwrap_or(Duration::MAX)
                .as_secs()
                > MAX_AGE_SECS
        }
        Err(_) => true,
    }
}

pub fn db_status() -> &'static str {
    let path = db_path();
    if !path.exists() {
        "missing"
    } else if needs_update() {
        "outdated"
    } else {
        "ok"
    }
}

// ---------------------------------------------------------------------------
// Download + extraction
// ---------------------------------------------------------------------------

pub async fn download(account_id: &str, license_key: &str) -> Result<()> {
    const URL: &str =
        "https://download.maxmind.com/geoip/databases/GeoLite2-City/download?suffix=tar.gz";

    tracing::info!("Downloading GeoLite2-City database…");

    let bytes = reqwest::Client::new()
        .get(URL)
        .basic_auth(account_id, Some(license_key))
        .send()
        .await
        .context("HTTP GET failed")?
        .error_for_status()
        .context("MaxMind returned a non-2xx status — check your Account ID and license key")?
        .bytes()
        .await
        .context("Failed to read response body")?;

    extract_mmdb(bytes.as_ref())
}

fn extract_mmdb(data: &[u8]) -> Result<()> {
    let decoder = flate2::read::GzDecoder::new(io::Cursor::new(data));
    let mut archive = tar::Archive::new(decoder);

    let dest = db_path();
    fs::create_dir_all(dest.parent().unwrap_or(Path::new(".")))
        .context("Failed to create geoip directory")?;

    for entry in archive.entries().context("Failed to read tar archive")? {
        let mut entry = entry.context("Failed to read archive entry")?;
        let path = entry.path().context("Failed to read entry path")?;

        if path.extension().map_or(false, |e| e == "mmdb") {
            // Read bytes and write via fs::write so the mtime reflects the
            // download time, not the original archive timestamp.  This is
            // important because needs_update() uses mtime to decide freshness.
            let mut buf = Vec::new();
            io::Read::read_to_end(&mut entry, &mut buf)
                .context("Failed to read .mmdb data from archive")?;
            fs::write(&dest, &buf).context("Failed to write .mmdb file")?;
            tracing::info!("GeoLite2-City database saved to {}", dest.display());
            return Ok(());
        }
    }

    anyhow::bail!("No .mmdb file found in MaxMind archive")
}

// ---------------------------------------------------------------------------
// Lookup
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GeoRecord {
    pub lat:     f64,
    pub lon:     f64,
    pub city:    Option<String>,
    pub country: Option<String>,
}

pub fn lookup(ip_str: &str) -> Option<GeoRecord> {
    let path = db_path();
    if !path.exists() {
        return None;
    }

    let ip: IpAddr = ip_str.parse().ok()?;
    let reader = maxminddb::Reader::open_readfile(&path).ok()?;
    let city: maxminddb::geoip2::City = reader.lookup(ip).ok()?.decode().ok()??;

    let lat = city.location.latitude?;
    let lon = city.location.longitude?;

    let city_name = city.city.names.english.map(|s| s.to_string());

    let country = city.country.iso_code.map(|s| s.to_string());

    Some(GeoRecord { lat, lon, city: city_name, country })
}
