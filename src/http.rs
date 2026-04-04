use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tokio::{sync::RwLock, task::JoinSet};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

use crate::{
    config::AppConfig,
    dns,
    export::{self, ExportFormat},
    models::{BulkLookupRequest, ConfigUpdateRequest, ExportQuery, IpRecord},
    rdap::{ParsedRdap, RdapClient},
    utils,
    whois::{ParsedWhois, WhoisClient},
};

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

/// State shared across every Axum handler.
/// Both fields are wrapped in `Arc<RwLock<…>>` so the server can serve
/// concurrent requests safely while still allowing mutation.
#[derive(Clone)]
pub struct AppState {
    /// Runtime configuration (language, timeouts, proxy, …).
    pub config: Arc<RwLock<AppConfig>>,
    /// In-memory lookup results, accumulated across all requests.
    pub records: Arc<RwLock<Vec<IpRecord>>>,
}

impl AppState {
    /// Create a new `AppState` starting with default configuration.
    /// Once `AppConfig::load()` is implemented, call it here.
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(AppConfig::default())),
            records: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Wrapper that converts any `anyhow::Error` into a JSON HTTP 500 response.
struct ApiError(anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({ "error": self.0.to_string() }));
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(e: E) -> Self {
        ApiError(e.into())
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build and return the Axum router with all API routes (spec §2.6).
/// CORS is set to permissive (required for the browser extension).
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(Any);

    Router::new()
        .route("/health",               get(health))
        .route("/lookup/ip/:ip",        get(lookup_ip))
        .route("/lookup/hostname/:host", get(lookup_hostname))
        .route("/lookup/bulk",          post(lookup_bulk))
        .route("/export",               get(export_records))
        .route("/config",               get(get_config).post(update_config))
        .layer(cors)
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Lookup pipeline (spec §2.1 — RDAP prioritaire, fallback WHOIS)
// ---------------------------------------------------------------------------

/// Run the full lookup pipeline for a single `target` (IP or hostname).
///
/// Steps:
/// 1. If `target` is a hostname, resolve it to an IP via DNS.
/// 2. Perform a reverse PTR lookup on the resolved IP.
/// 3. Try RDAP (priority). On any failure, fall back to WHOIS.
/// 4. Return a populated `IpRecord`.
///
/// Exposed as `pub(crate)` so `cli.rs` can reuse the same pipeline.
pub(crate) async fn perform_lookup(order: u32, target: &str, config: &AppConfig) -> IpRecord {
    let mut record = IpRecord::new(order, target);

    // ── Step 1: hostname resolution ─────────────────────────────────────────
    let ip: String = if utils::is_ip(target) {
        // The target itself is an IP; record it and move on.
        target.to_owned()
    } else {
        // Remember the original hostname.
        record.host_name = Some(target.to_owned());
        match dns::resolve_hostname(target, config.dns_timeout_ms).await {
            Ok(ip) => {
                record.ip = ip.clone();
                ip
            }
            Err(e) => {
                let msg = format!("DNS forward lookup failed: {e}");
                warn!("{msg}");
                record.lookup_errors.push(msg);
                return record;
            }
        }
    };

    // ── Step 2: reverse PTR lookup ──────────────────────────────────────────
    match dns::reverse_lookup(&ip, config.dns_timeout_ms).await {
        Ok(Some(ptr)) => {
            record.resolved_name = Some(ptr);
        }
        Ok(None) => {}
        Err(e) => {
            // PTR failure is non-fatal.
            record.lookup_errors.push(format!("DNS reverse lookup: {e}"));
        }
    }

    // ── Step 3: RDAP (priority) ─────────────────────────────────────────────
    let rdap_ok = match RdapClient::new(config.rdap_timeout_ms) {
        Ok(client) => match client.query(&ip).await {
            Ok(result) => {
                info!("RDAP success for {ip} via {}", result.url);
                let parsed = RdapClient::parse(&result.json);
                record.raw_rdap = Some(result.json);
                record.whois_source = Some("RDAP".to_owned());
                apply_rdap(&mut record, parsed);
                true
            }
            Err(e) => {
                let msg = format!("RDAP query failed: {e}");
                warn!("{msg}");
                record.lookup_errors.push(msg);
                false
            }
        },
        Err(e) => {
            let msg = format!("RDAP client init failed: {e}");
            error!("{msg}");
            record.lookup_errors.push(msg);
            false
        }
    };

    // ── Step 4: WHOIS fallback ──────────────────────────────────────────────
    if !rdap_ok {
        let client = WhoisClient::new(config.whois_timeout_ms);
        match client.query(&ip).await {
            Ok(result) => {
                info!("WHOIS success for {ip} via {}", result.server);
                record.whois_source = Some(result.server.clone());
                let parsed = WhoisClient::parse(&result.raw);
                record.raw_whois = Some(result.raw);
                apply_whois(&mut record, parsed);
            }
            Err(e) => {
                let msg = format!("WHOIS query failed: {e}");
                error!("{msg}");
                record.lookup_errors.push(msg);
            }
        }
    }

    record
}

/// Merge parsed RDAP fields into an `IpRecord`.
/// Fields already set on the record are left unchanged (first-wins).
fn apply_rdap(rec: &mut IpRecord, p: ParsedRdap) {
    fill_opt(&mut rec.country,      p.country);
    fill_opt(&mut rec.owner_name,   p.owner_name);
    fill_opt(&mut rec.address,      p.address);
    fill_opt(&mut rec.phone,        p.phone);
    fill_opt(&mut rec.fax,          p.fax);
    fill_opt(&mut rec.from_ip,      p.from_ip);
    fill_opt(&mut rec.to_ip,        p.to_ip);
    fill_opt(&mut rec.status,       p.status);
    fill_opt(&mut rec.network_name, p.network_name);
    fill_opt(&mut rec.contact_name, p.contact_name);
    fill_opt(&mut rec.allocated,    p.allocated);
    fill_opt(&mut rec.cidr,         p.cidr);
    fill_opt(&mut rec.postal_code,  p.postal_code);
    fill_opt(&mut rec.abuse_contact, p.abuse_contact);
    extend_unique(&mut rec.emails,       p.emails);
    extend_unique(&mut rec.abuse_emails, p.abuse_emails);
}

/// Merge parsed WHOIS fields into an `IpRecord`.
fn apply_whois(rec: &mut IpRecord, p: ParsedWhois) {
    fill_opt(&mut rec.country,      p.country);
    fill_opt(&mut rec.owner_name,   p.owner_name);
    fill_opt(&mut rec.address,      p.address);
    fill_opt(&mut rec.phone,        p.phone);
    fill_opt(&mut rec.fax,          p.fax);
    fill_opt(&mut rec.from_ip,      p.from_ip);
    fill_opt(&mut rec.to_ip,        p.to_ip);
    fill_opt(&mut rec.status,       p.status);
    fill_opt(&mut rec.network_name, p.network_name);
    fill_opt(&mut rec.contact_name, p.contact_name);
    fill_opt(&mut rec.allocated,    p.allocated);
    fill_opt(&mut rec.cidr,         p.cidr);
    fill_opt(&mut rec.postal_code,  p.postal_code);
    fill_opt(&mut rec.abuse_contact, p.abuse_contact);
    extend_unique(&mut rec.emails,       p.emails);
    extend_unique(&mut rec.abuse_emails, p.abuse_emails);
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /health → `{ "status": "ok" }` (spec §2.6)
async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /lookup/ip/:ip → `IpRecord` (spec §2.6)
async fn lookup_ip(
    State(state): State<AppState>,
    Path(ip): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let ip = ip.trim().to_owned();
    if !utils::is_ip(&ip) {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": format!("'{ip}' is not a valid IP address") })),
        )
            .into_response());
    }

    // Safe to unwrap: is_ip() already validated the parse.
    let ip_addr: std::net::IpAddr = ip.parse().unwrap();
    if !utils::is_public_ip(&ip_addr) {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": "Private or reserved IP not allowed" })),
        )
            .into_response());
    }

    let config = state.config.read().await.clone();
    let order = {
        let records = state.records.read().await;
        records.len() as u32 + 1
    };

    let record = perform_lookup(order, &ip, &config).await;

    state.records.write().await.push(record.clone());
    info!("Lookup complete for {ip}: {} errors", record.lookup_errors.len());

    Ok(Json(record).into_response())
}

/// GET /lookup/hostname/:host → `IpRecord` (spec §2.6)
async fn lookup_hostname(
    State(state): State<AppState>,
    Path(host): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let host = host.trim().to_owned();
    let config = state.config.read().await.clone();

    // Pre-resolve to check if the target IP is public before launching the
    // full pipeline.  A DNS failure here is non-fatal: perform_lookup will
    // handle it with proper error recording.
    if let Ok(ip_str) = dns::resolve_hostname(&host, config.dns_timeout_ms).await {
        if let Ok(ip_addr) = ip_str.parse::<std::net::IpAddr>() {
            if !utils::is_public_ip(&ip_addr) {
                return Ok((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({
                        "error": "Hostname resolves to a private or reserved IP"
                    })),
                )
                    .into_response());
            }
        }
    }

    let order = {
        let records = state.records.read().await;
        records.len() as u32 + 1
    };

    let record = perform_lookup(order, &host, &config).await;

    state.records.write().await.push(record.clone());
    info!("Lookup complete for {host}: {} errors", record.lookup_errors.len());

    Ok(Json(record).into_response())
}

/// POST /lookup/bulk — body: `{ "targets": [...] }` (spec §2.6)
///
/// Runs all lookups concurrently via `tokio::task::JoinSet`.
/// Results are sorted by their original input order.
async fn lookup_bulk(
    State(state): State<AppState>,
    Json(body): Json<BulkLookupRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let targets = utils::clean_targets(&body.targets.join("\n"));
    if targets.is_empty() {
        return Ok(Json(Vec::<IpRecord>::new()).into_response());
    }

    let config = state.config.read().await.clone();
    let base_order = {
        let records = state.records.read().await;
        records.len() as u32
    };

    let mut set: JoinSet<IpRecord> = JoinSet::new();
    for (i, target) in targets.into_iter().enumerate() {
        let cfg = config.clone();
        let order = base_order + i as u32 + 1;
        set.spawn(async move { perform_lookup(order, &target, &cfg).await });
    }

    let mut results: Vec<IpRecord> = Vec::new();
    while let Some(res) = set.join_next().await {
        match res {
            Ok(record) => results.push(record),
            Err(e) => error!("Bulk lookup task panicked: {e}"),
        }
    }

    // Restore deterministic output order.
    results.sort_by_key(|r| r.order);

    let mut store = state.records.write().await;
    store.extend(results.clone());
    info!("Bulk lookup complete: {} records", results.len());

    Ok(Json(results).into_response())
}

/// GET /export?format=csv|tsv|txt|html|xml|whois&ids=uuid,uuid,… (spec §2.6)
async fn export_records(
    State(state): State<AppState>,
    Query(params): Query<ExportQuery>,
) -> Result<Response, ApiError> {
    let fmt = ExportFormat::from_str(&params.format)?;

    // Filter by IDs if provided (comma-separated UUIDs).
    let all_records = state.records.read().await;
    let records: Vec<IpRecord> = match &params.ids {
        Some(ids_str) if !ids_str.is_empty() => {
            let ids: Vec<&str> = ids_str.split(',').map(str::trim).collect();
            all_records
                .iter()
                .filter(|r| ids.contains(&r.id.to_string().as_str()))
                .cloned()
                .collect()
        }
        _ => all_records.clone(),
    };
    drop(all_records);

    let config = state.config.read().await.clone();
    let content = export::export(&records, fmt, config.csv_with_header)?;

    let content_type = export_content_type(fmt);
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(content))
        .map_err(anyhow::Error::from)?;

    Ok(response)
}

/// GET /config → current `AppConfig` as JSON (spec §2.6)
async fn get_config(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let config = state.config.read().await.clone();
    Ok(Json(config))
}

/// POST /config — partial update, returns the updated `AppConfig` (spec §2.6)
async fn update_config(
    State(state): State<AppState>,
    Json(body): Json<ConfigUpdateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut config = state.config.write().await;

    if let Some(v) = body.language            { config.language            = v; }
    if let Some(v) = body.proxy_type          { config.proxy_type          = v; }
    if let Some(v) = body.proxy_host          { config.proxy_host          = v; }
    if let Some(v) = body.proxy_port          { config.proxy_port          = v; }
    if let Some(v) = body.dns_timeout_ms      { config.dns_timeout_ms      = v; }
    if let Some(v) = body.whois_timeout_ms    { config.whois_timeout_ms    = v; }
    if let Some(v) = body.rdap_timeout_ms     { config.rdap_timeout_ms     = v; }
    if let Some(v) = body.default_export_format { config.default_export_format = v; }
    if let Some(v) = body.csv_with_header     { config.csv_with_header     = v; }

    // TODO: persist to disk once AppConfig::save() is implemented.

    Ok(Json(config.clone()))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Set `dest` to `src` only if `dest` is currently `None`.
fn fill_opt(dest: &mut Option<String>, src: Option<String>) {
    if dest.is_none() {
        *dest = src;
    }
}

/// Append each item of `src` to `dest` if it is not already present.
fn extend_unique(dest: &mut Vec<String>, src: Vec<String>) {
    for item in src {
        if !dest.contains(&item) {
            dest.push(item);
        }
    }
}

/// Map an `ExportFormat` to an HTTP `Content-Type` header value.
fn export_content_type(fmt: ExportFormat) -> &'static str {
    match fmt {
        ExportFormat::Csv                          => "text/csv; charset=utf-8",
        ExportFormat::Tsv                          => "text/tab-separated-values; charset=utf-8",
        ExportFormat::Txt
        | ExportFormat::TxtHorizontal              => "text/plain; charset=utf-8",
        ExportFormat::HtmlVertical
        | ExportFormat::HtmlHorizontal             => "text/html; charset=utf-8",
        ExportFormat::Xml                          => "application/xml; charset=utf-8",
        ExportFormat::WhoisRaw                     => "text/plain; charset=utf-8",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    fn test_state() -> AppState {
        AppState::new()
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // ── /health ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_health_returns_ok() {
        let app = build_router(test_state());
        let req = Request::builder().uri("/health").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["status"], "ok");
    }

    // ── /config ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_config_returns_defaults() {
        let app = build_router(test_state());
        let req = Request::builder().uri("/config").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["language"], "fr");
        assert_eq!(json["dns_timeout_ms"], 3000);
    }

    #[tokio::test]
    async fn test_post_config_updates_fields() {
        let state = test_state();
        let app = build_router(state.clone());

        let body = serde_json::json!({ "language": "en", "dns_timeout_ms": 1000 });
        let req = Request::builder()
            .method("POST")
            .uri("/config")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["language"], "en");
        assert_eq!(json["dns_timeout_ms"], 1000);
        // Other fields must remain unchanged.
        assert_eq!(json["proxy_type"], "none");
    }

    // ── /lookup/ip ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_lookup_ip_invalid_returns_422() {
        let app = build_router(test_state());
        let req = Request::builder()
            .uri("/lookup/ip/not-an-ip")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_lookup_ip_google_dns() {
        let app = build_router(test_state());
        let req = Request::builder()
            .uri("/lookup/ip/8.8.8.8")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["ip"], "8.8.8.8");
        assert!(json["id"].is_string(), "Expected a UUID");
        assert!(json["order"].is_number());
        // Either RDAP or WHOIS must have returned the range.
        assert!(
            json["from_ip"] != serde_json::Value::Null
                || !json["lookup_errors"].as_array().unwrap().is_empty(),
            "Expected either IP range data or errors, got: {json}"
        );
    }

    // ── /lookup/bulk ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_lookup_bulk_empty_targets() {
        let app = build_router(test_state());
        let body = serde_json::json!({ "targets": [] });
        let req = Request::builder()
            .method("POST")
            .uri("/lookup/bulk")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json, serde_json::json!([]));
    }

    #[tokio::test]
    async fn test_lookup_bulk_deduplicates() {
        let app = build_router(test_state());
        // Same IP twice — should be deduplicated by clean_targets.
        let body = serde_json::json!({ "targets": ["1.1.1.1", "1.1.1.1", "8.8.8.8"] });
        let req = Request::builder()
            .method("POST")
            .uri("/lookup/bulk")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2, "Expected 2 unique results after dedup");
    }

    // ── Private/reserved IP guard ────────────────────────────────────────────

    #[tokio::test]
    async fn test_lookup_ip_private_returns_422() {
        let app = build_router(test_state());
        let req = Request::builder()
            .uri("/lookup/ip/192.168.1.1")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let json = body_json(resp).await;
        assert!(
            json["error"].as_str().unwrap().contains("Private or reserved"),
            "Unexpected error message: {json}"
        );
    }

    #[tokio::test]
    async fn test_lookup_ip_loopback_returns_422() {
        let app = build_router(test_state());
        let req = Request::builder()
            .uri("/lookup/ip/127.0.0.1")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_lookup_ip_public_returns_200() {
        let app = build_router(test_state());
        let req = Request::builder()
            .uri("/lookup/ip/8.8.8.8")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// localhost resolves to 127.0.0.1 (loopback) → 422.
    #[tokio::test]
    async fn test_lookup_hostname_localhost_returns_422() {
        let app = build_router(test_state());
        let req = Request::builder()
            .uri("/lookup/hostname/localhost")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let json = body_json(resp).await;
        assert!(
            json["error"].as_str().unwrap().contains("private or reserved"),
            "Unexpected error message: {json}"
        );
    }

    /// A private IP in a bulk list must be silently dropped by clean_targets.
    #[tokio::test]
    async fn test_lookup_bulk_skips_private_ip() {
        let app = build_router(test_state());
        let body = serde_json::json!({ "targets": ["8.8.8.8", "192.168.1.1", "10.0.0.1"] });
        let req = Request::builder()
            .method("POST")
            .uri("/lookup/bulk")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let arr = json.as_array().unwrap();
        // Only 8.8.8.8 should remain after filtering private IPs.
        assert_eq!(arr.len(), 1, "Expected only 1 public IP, got: {json}");
        assert_eq!(arr[0]["ip"], "8.8.8.8");
    }
}
