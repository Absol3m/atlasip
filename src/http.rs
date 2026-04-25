use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{RwLock, Semaphore},
    task::JoinSet,
    time::timeout,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

use crate::{
    bgp::BgpClient,
    cache::LookupCache,
    config::AppConfig,
    dns,
    export::{self, ExportFormat},
    metrics::RequestMetrics,
    models::{BulkLookupRequest, ConfigUpdateRequest, ExportQuery, IpRecord},
    rdap::{ParsedRdap, RdapClient},
    retry::retry_async,
    utils,
    whois::{enrich_from_whois_raw, ParsedWhois, WhoisClient},
};

/// Maximum number of lookup results kept in the session store.
/// Oldest entries are dropped first when the limit is exceeded.
const MAX_SESSION_RECORDS: usize = 10_000;

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
    /// TTL-based lookup cache (P0-PERF-004, P3-PERF-018).
    pub cache: LookupCache,
    /// Per-source request metrics (P3-PERF-016).
    pub metrics: RequestMetrics,
    /// Semaphore that caps concurrent lookups to `max_concurrent_lookups`
    /// (P2-PERF-011).
    pub semaphore: Arc<Semaphore>,
}

impl AppState {
    /// Create a new `AppState` starting with default configuration.
    pub fn new() -> Self {
        Self::with_config(AppConfig::default())
    }

    /// Create a new `AppState` seeded with the supplied `config`.
    ///
    /// The cache TTL and semaphore capacity are derived from the config so
    /// they stay consistent even when a custom config is provided.
    pub fn with_config(config: AppConfig) -> Self {
        let cache_ttl    = Duration::from_secs(config.cache_ttl_secs);
        let max_parallel = config.max_concurrent_lookups as usize;
        Self {
            cache:     LookupCache::new(cache_ttl),
            metrics:   RequestMetrics::new(),
            semaphore: Arc::new(Semaphore::new(max_parallel)),
            config:    Arc::new(RwLock::new(config)),
            records:   Arc::new(RwLock::new(Vec::new())),
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
        // ── Existing routes (backward-compatible) ────────────────────────────
        .route("/health",                get(health))
        .route("/lookup/ip/:ip",         get(lookup_ip))
        .route("/lookup/hostname/:host", get(lookup_hostname))
        .route("/lookup/bulk",           post(lookup_bulk))
        .route("/export",                get(export_records))
        .route("/config",                get(get_config).post(update_config))
        .route("/metrics",               get(get_metrics))
        // ── v0.5 headless / Chrome-extension API (spec §3) ──────────────────
        // These routes are stable aliases exposed for the extension and external
        // integrations.  They share the same pipeline as the /lookup/* routes.
        .route("/analyze/ip/:ip",        get(analyze_ip))
        .route("/analyze/domain/:domain", get(analyze_domain))
        .route("/reverse/:ip",           get(reverse_ip))
        .route("/reverse-ip/:ip",        get(reverse_ip_domains))
        .layer(cors)
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Lookup pipeline (spec §2.1 — RDAP prioritaire, fallback WHOIS)
// ---------------------------------------------------------------------------

/// Run the full lookup pipeline for a single `target` (IP or hostname).
///
/// A hard global deadline (`config.global_timeout_ms`) wraps the entire
/// pipeline (P0-PERF-001).  Individual steps use `retry_async` when
/// `config.auto_retry_failed` is `true` (P1-PERF-006).
///
/// Exposed as `pub(crate)` so `cli.rs` can reuse the same pipeline.
pub(crate) async fn perform_lookup(
    order:   u32,
    target:  &str,
    config:  &AppConfig,
    cache:   &LookupCache,
    metrics: &RequestMetrics,
) -> IpRecord {
    match timeout(
        Duration::from_millis(config.global_timeout_ms),
        perform_lookup_inner(order, target, config, cache, metrics),
    )
    .await
    {
        Ok(record) => record,
        Err(_) => {
            let mut rec = IpRecord::new(order, target);
            error!(
                target: "atlasip::lookup",
                %target,
                timeout_ms = config.global_timeout_ms,
                "global lookup timeout exceeded"
            );
            rec.lookup_errors.push(format!(
                "Global lookup timeout ({} ms) exceeded",
                config.global_timeout_ms
            ));
            rec
        }
    }
}

/// Inner pipeline — no global timeout (handled by the caller).
async fn perform_lookup_inner(
    order:   u32,
    target:  &str,
    config:  &AppConfig,
    cache:   &LookupCache,
    metrics: &RequestMetrics,
) -> IpRecord {
    // ── Cache check (P0-PERF-004) ────────────────────────────────────────────
    if let Some(mut cached) = cache.get(target).await {
        metrics.record_cache_hit();
        // Update order so bulk-lookup sort remains deterministic.
        cached.order = order;
        info!(target: "atlasip::cache", %target, "serving from cache");
        return cached;
    }

    let mut record = IpRecord::new(order, target);

    // ── Step 1: DNS ─────────────────────────────────────────────────────────
    let dns_start = Instant::now();

    let ip: String = if utils::is_ip(target) {
        record.ip = target.to_owned();
        // PTR is non-blocking; failure is recorded but does not abort.
        let dns_ok = match dns::reverse_lookup(target, config.dns_timeout_ms).await {
            Ok(Some(ptr)) => {
                record.resolved_name = Some(ptr);
                true
            }
            Ok(None) => true,
            Err(e) => {
                record.lookup_errors.push(format!("PTR: {e}"));
                false
            }
        };
        metrics.record_dns(dns_start.elapsed().as_micros() as u64, dns_ok);
        target.to_owned()
    } else {
        record.host_name = Some(target.to_owned());
        let dns_result = dns::full_dns_lookup(
            target,
            config.dns_timeout_ms,
            &config.dns_mode,
            &config.doh_endpoint,
            &config.dot_server,
        ).await;

        // Propagate DNS records (A, AAAA, CNAME, TXT with TTL).
        record.dns_records = dns_result.records;
        if let Some(ptr) = dns_result.ptr {
            record.resolved_name = Some(ptr);
        }
        for e in dns_result.errors {
            record.lookup_errors.push(e);
        }

        match dns_result.resolved_ip {
            Some(ip) => {
                metrics.record_dns(dns_start.elapsed().as_micros() as u64, true);
                record.ip = ip.clone();
                ip
            }
            None => {
                metrics.record_dns(dns_start.elapsed().as_micros() as u64, false);
                // Only surface an error when DNS returned nothing at all (NXDOMAIN,
                // timeout, network failure).  If other record types (MX, NS, TXT…)
                // were found the query succeeded — the domain just has no address
                // record, which is normal for mail-only or delegation-only zones.
                if record.dns_records.is_empty() {
                    let msg = "DNS: no A/AAAA record resolved".to_owned();
                    warn!("{msg} for {target}");
                    record.lookup_errors.push(msg);
                }
                return record; // No IP → RDAP/WHOIS/GeoIP cannot run.
            }
        }
    };

    // ── Steps 2 + 3: RDAP and WHOIS run in parallel ──────────────────────────
    //
    // RDAP is the primary source (spec §2.1).
    // WHOIS always runs alongside to provide enrichment:
    //   • RDAP ok  → WHOIS brut fills empty fields (*_enriched flags).
    //   • RDAP fail → WHOIS becomes primary source (no enrichment flags).
    let max_retries = if config.auto_retry_failed { 2 } else { 0 };

    let rdap_metrics = metrics.clone();
    let rdap_future = {
        let ip_clone = ip.clone();
        let cfg      = config.clone();
        async move {
            let start = Instant::now();
            let result: Result<_, anyhow::Error> = retry_async(
                "RDAP",
                max_retries,
                Duration::from_millis(200),
                || {
                    let ip  = ip_clone.clone();
                    let cfg = cfg.clone();
                    async move {
                        let client = RdapClient::new(cfg.rdap_timeout_ms)
                            .map_err(|e| anyhow::anyhow!("RDAP client init: {e}"))?;
                        client.query(&ip).await.map_err(|e| anyhow::anyhow!("{e}"))
                    }
                },
            )
            .await;
            rdap_metrics.record_rdap(start.elapsed().as_micros() as u64, result.is_ok());
            result
        }
    };

    let whois_metrics = metrics.clone();
    let whois_future = {
        let ip_clone = ip.clone();
        let cfg      = config.clone();
        async move {
            let start = Instant::now();
            let result: Result<_, anyhow::Error> = retry_async(
                "WHOIS",
                max_retries,
                Duration::from_millis(200),
                || {
                    let ip  = ip_clone.clone();
                    let cfg = cfg.clone();
                    async move {
                        let client = WhoisClient::new(cfg.whois_timeout_ms);
                        client.query(&ip).await.map_err(|e| anyhow::anyhow!("{e}"))
                    }
                },
            )
            .await;
            whois_metrics.record_whois(start.elapsed().as_micros() as u64, result.is_ok());
            result
        }
    };

    let bgp_future = {
        let ip_clone = ip.clone();
        // Cap at 1 500 ms per request so 2 sequential rounds (ip → prefixes+peers)
        // stay within the 5 s global timeout even on a slow RIPEstat response.
        let timeout = config.rdap_timeout_ms.min(1_500);
        let proxy   = config.proxy.clone();
        async move {
            let client = BgpClient::new(timeout, &proxy)?;
            client.lookup(&ip_clone).await
        }
    };

    let (rdap_result, whois_result, bgp_result) =
        tokio::join!(rdap_future, whois_future, bgp_future);

    let rdap_ok = match rdap_result {
        Ok(rdap) => {
            info!(target: "atlasip::rdap", %ip, url = %rdap.url, "RDAP ok");
            let parsed = RdapClient::parse(&rdap.json);
            record.raw_rdap     = Some(rdap.json);
            record.whois_source = Some("RDAP".to_owned());
            apply_rdap(&mut record, parsed);
            true
        }
        Err(e) => {
            let msg = format!("RDAP: {e}");
            warn!("{msg}");
            record.lookup_errors.push(msg);
            false
        }
    };

    match whois_result {
        Ok(whois) => {
            info!(target: "atlasip::whois", %ip, server = %whois.server, "WHOIS ok");
            record.raw_whois = Some(whois.raw.clone());
            if rdap_ok {
                // RDAP already populated the record — use WHOIS brut only to
                // fill empty fields and set *_enriched flags (spec R1–R3).
                enrich_from_whois_raw(&mut record, &whois.raw);
            } else {
                // RDAP failed: WHOIS is the primary source; no enrichment flags.
                metrics.record_fallback();
                record.whois_source = Some(whois.server);
                let parsed = WhoisClient::parse(&whois.raw);
                apply_whois(&mut record, parsed);
            }
        }
        Err(e) => {
            let msg = format!("WHOIS: {e}");
            // Demote to warn when RDAP already succeeded; otherwise error.
            if rdap_ok { warn!("{msg}"); } else { error!("{msg}"); }
            record.lookup_errors.push(msg);
        }
    }

    // ── Step 4: BGP (RIPEstat, non-blocking) ────────────────────────────────
    match bgp_result {
        Ok(bgp) => {
            record.bgp = Some(bgp);
        }
        Err(e) => {
            warn!(target: "atlasip::bgp", %ip, "BGP lookup failed: {e}");
        }
    }

    // ── Step 5: GeoIP (MaxMind GeoLite2, non-blocking) ──────────────────────
    if let Some(geo) = crate::geoip::lookup(&ip) {
        record.geo_lat     = Some(geo.lat);
        record.geo_lon     = Some(geo.lon);
        record.geo_city    = geo.city;
        record.geo_country = geo.country;
    }

    // ── Cache insert (P0-PERF-004) ───────────────────────────────────────────
    // Only cache when at least one network source returned data.
    if rdap_ok || record.raw_whois.is_some() {
        cache.insert(target, record.clone()).await;
    }

    record
}

/// Merge parsed RDAP fields into an `IpRecord`.
/// Fields already set on the record are left unchanged (first-wins).
fn apply_rdap(rec: &mut IpRecord, p: ParsedRdap) {
    fill_opt(&mut rec.country,       p.country);
    fill_opt(&mut rec.owner_name,    p.owner_name);
    fill_opt(&mut rec.address,       p.address);
    fill_opt(&mut rec.phone,         p.phone);
    fill_opt(&mut rec.fax,           p.fax);
    fill_opt(&mut rec.from_ip,       p.from_ip);
    fill_opt(&mut rec.to_ip,         p.to_ip);
    fill_opt(&mut rec.status,        p.status);
    fill_opt(&mut rec.network_name,  p.network_name);
    fill_opt(&mut rec.contact_name,  p.contact_name);
    fill_opt(&mut rec.allocated,     p.allocated);
    fill_opt(&mut rec.cidr,          p.cidr);
    fill_opt(&mut rec.postal_code,   p.postal_code);
    fill_opt(&mut rec.abuse_contact, p.abuse_contact);
    extend_unique(&mut rec.emails,       p.emails);
    extend_unique(&mut rec.abuse_emails, p.abuse_emails);
}

/// Merge parsed WHOIS fields into an `IpRecord`.
fn apply_whois(rec: &mut IpRecord, p: ParsedWhois) {
    fill_opt(&mut rec.country,       p.country);
    fill_opt(&mut rec.owner_name,    p.owner_name);
    fill_opt(&mut rec.address,       p.address);
    fill_opt(&mut rec.phone,         p.phone);
    fill_opt(&mut rec.fax,           p.fax);
    fill_opt(&mut rec.from_ip,       p.from_ip);
    fill_opt(&mut rec.to_ip,         p.to_ip);
    fill_opt(&mut rec.status,        p.status);
    fill_opt(&mut rec.network_name,  p.network_name);
    fill_opt(&mut rec.contact_name,  p.contact_name);
    fill_opt(&mut rec.allocated,     p.allocated);
    fill_opt(&mut rec.cidr,          p.cidr);
    fill_opt(&mut rec.postal_code,   p.postal_code);
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
            Json(serde_json::json!({ "error": crate::i18n::t("errors.error.invalid_ip_format").replace("{ip}", &ip) })),
        )
            .into_response());
    }

    // Safe to unwrap: is_ip() already validated the parse.
    let ip_addr: std::net::IpAddr = ip.parse().unwrap();
    if !utils::is_public_ip(&ip_addr) {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": crate::i18n::t("errors.error.private_ip") })),
        )
            .into_response());
    }

    let config = state.config.read().await.clone();
    let order = {
        let records = state.records.read().await;
        records.len() as u32 + 1
    };

    let _permit = state.semaphore.acquire().await.unwrap();
    let record = perform_lookup(order, &ip, &config, &state.cache, &state.metrics).await;
    drop(_permit);

    push_record(&state.records, record.clone()).await;
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
                        "error": crate::i18n::t("errors.error.private_hostname")
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

    let _permit = state.semaphore.acquire().await.unwrap();
    let record = perform_lookup(order, &host, &config, &state.cache, &state.metrics).await;
    drop(_permit);

    push_record(&state.records, record.clone()).await;
    info!("Lookup complete for {host}: {} errors", record.lookup_errors.len());

    Ok(Json(record).into_response())
}

/// POST /lookup/bulk — body: `{ "targets": [...] }` (spec §2.6)
///
/// Runs all lookups concurrently via `tokio::task::JoinSet`, bounded by the
/// semaphore in `AppState` (P2-PERF-011).
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

    let sem     = Arc::clone(&state.semaphore);
    let cache   = state.cache.clone();
    let metrics = state.metrics.clone();

    let mut set: JoinSet<IpRecord> = JoinSet::new();
    for (i, target) in targets.into_iter().enumerate() {
        let cfg     = config.clone();
        let cache   = cache.clone();
        let metrics = metrics.clone();
        let sem     = Arc::clone(&sem);
        let order   = base_order + i as u32 + 1;
        set.spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            perform_lookup(order, &target, &cfg, &cache, &metrics).await
        });
    }

    let mut results: Vec<IpRecord> = Vec::new();
    while let Some(res) = set.join_next().await {
        match res {
            Ok(record) => results.push(record),
            Err(e)     => error!("Bulk lookup task panicked: {e}"),
        }
    }

    // Restore deterministic output order.
    results.sort_by_key(|r| r.order);

    {
        let mut store = state.records.write().await;
        store.extend(results.clone());
        let overflow = store.len().saturating_sub(MAX_SESSION_RECORDS);
        if overflow > 0 {
            store.drain(0..overflow);
        }
    }
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

    let config  = state.config.read().await.clone();
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

    if let Some(v) = body.locale                { config.locale                = v; }
    if let Some(v) = body.proxy_type            { config.proxy_type            = v; }
    if let Some(v) = body.proxy_host            { config.proxy_host            = v; }
    if let Some(v) = body.proxy_port            { config.proxy_port            = v; }
    if let Some(v) = body.dns_timeout_ms        { config.dns_timeout_ms        = v; }
    if let Some(v) = body.whois_timeout_ms      { config.whois_timeout_ms      = v; }
    if let Some(v) = body.rdap_timeout_ms       { config.rdap_timeout_ms       = v; }
    if let Some(v) = body.default_export_format { config.default_export_format = v; }
    if let Some(v) = body.csv_with_header       { config.csv_with_header       = v; }
    if let Some(v) = body.maxmind_account_id    { config.maxmind_account_id    = Some(v); }
    if let Some(v) = body.maxmind_license_key   { config.maxmind_license_key   = Some(v); }

    Ok(Json(config.clone()))
}

/// GET /metrics → `MetricsSnapshot` (P3-PERF-016)
async fn get_metrics(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.metrics.snapshot())
}

// ---------------------------------------------------------------------------
// v0.5 headless / Chrome-extension routes (spec §3)
// ---------------------------------------------------------------------------

/// GET /analyze/ip/:ip
///
/// Stable alias for `/lookup/ip/:ip`.  Full RDAP+WHOIS+DNS pipeline.
/// Intended as the primary endpoint for the Chrome extension.
async fn analyze_ip(
    state: State<AppState>,
    path:  Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    lookup_ip(state, path).await
}

/// GET /analyze/domain/:domain
///
/// Stable alias for `/lookup/hostname/:domain`.
async fn analyze_domain(
    state: State<AppState>,
    path:  Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    lookup_hostname(state, path).await
}

/// GET /reverse/:ip
///
/// Lightweight PTR-only lookup that uses the DNS strategy configured in
/// `AppConfig.dns_mode` (System / DoH / Automatic / Disabled).
///
/// Response:
/// ```json
/// { "ip": "8.8.8.8", "ptr": "dns.google", "dns_mode": "automatic" }
/// ```
/// When no PTR record exists, `ptr` is `null`.
async fn reverse_ip(
    State(state): State<AppState>,
    Path(ip):     Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate the IP address first.
    if ip.parse::<std::net::IpAddr>().is_err() {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "error": crate::i18n::t("errors.error.invalid_ip_reverse").replace("{ip}", &ip) })),
        )
            .into_response());
    }

    let cfg = state.config.read().await;
    let ptr = dns::reverse_lookup_smart(
        &ip,
        &cfg.dns_mode,
        cfg.dns_system_timeout_ms,
        &cfg.doh_endpoint,
        &cfg.dot_server,
        cfg.dns_timeout_ms,
    )
    .await
    .unwrap_or(None);

    let dns_mode_label = format!("{:?}", cfg.dns_mode).to_lowercase();
    drop(cfg);

    Ok(Json(serde_json::json!({
        "ip":       ip,
        "ptr":      ptr,
        "dns_mode": dns_mode_label,
    }))
    .into_response())
}

/// GET /reverse-ip/:ip
/// GET /reverse-ip/:ip[?hostname=<ptr>]
///
/// Multi-source OSINT reverse IP: queries all available passive sources in
/// parallel, deduplicates results and attributes each domain to its source(s).
///
/// Sources:
///   - hackertarget — passive DNS reverse IP
///   - crtsh        — certificate transparency (requires PTR hostname)
///
/// Response:
/// ```json
/// {
///   "ip": "1.2.3.4",
///   "results": [{ "domain": "example.com", "sources": ["hackertarget"] }],
///   "count": 12,
///   "source_errors": [{ "source": "crtsh", "error": "timeout" }]
/// }
/// ```
async fn reverse_ip_domains(
    State(state): State<AppState>,
    Path(ip):     Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, ApiError> {
    if ip.parse::<std::net::IpAddr>().is_err() {
        return Ok((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({
                "error": crate::i18n::t("errors.error.invalid_ip_reverse").replace("{ip}", &ip)
            })),
        )
            .into_response());
    }

    let (timeout_ms, proxy) = {
        let cfg = state.config.read().await;
        (cfg.rdap_timeout_ms, cfg.proxy.clone())
    };

    let hostname = params.get("hostname").cloned();

    let client: reqwest::Client = apply_proxy(
        reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .user_agent("AtlasIP/0.7 (OSINT; https://github.com/Absol3m/atlasip)"),
        &proxy,
    )?.build()?;

    // ── Run all sources in parallel ──────────────────────────────────────────
    let (ht_result, crtsh_result) = tokio::join!(
        query_hackertarget(&client, &ip),
        query_crtsh(&client, hostname.as_deref()),
    );

    // ── Merge + deduplicate: domain → set of sources ─────────────────────────
    let mut map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut source_errors: Vec<serde_json::Value> = Vec::new();

    let mut ingest = |result: Result<Vec<String>, String>, source: &str| {
        match result {
            Ok(domains) => {
                for d in domains {
                    map.entry(d).or_default().push(source.to_owned());
                }
            }
            Err(e) => {
                source_errors.push(serde_json::json!({ "source": source, "error": e }));
            }
        }
    };

    ingest(ht_result,    "hackertarget");
    ingest(crtsh_result, "crtsh");

    let mut results: Vec<serde_json::Value> = map
        .into_iter()
        .map(|(domain, sources)| serde_json::json!({ "domain": domain, "sources": sources }))
        .collect();
    results.sort_by(|a, b| {
        a["domain"].as_str().unwrap_or("").cmp(b["domain"].as_str().unwrap_or(""))
    });

    let count = results.len();
    Ok(Json(serde_json::json!({
        "ip":            ip,
        "results":       results,
        "count":         count,
        "source_errors": source_errors,
    }))
    .into_response())
}

/// Query HackerTarget reverse IP — returns plain-text, one domain per line.
async fn query_hackertarget(
    client: &reqwest::Client,
    ip: &str,
) -> Result<Vec<String>, String> {
    let url  = format!("https://api.hackertarget.com/reverseiplookup/?q={ip}");
    let body = client.get(&url).send().await
        .map_err(|e| e.to_string())?
        .text().await
        .map_err(|e| e.to_string())?;

    if body.starts_with("error") || body.trim() == "No Records Found" {
        return Ok(Vec::new());
    }
    Ok(body.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("error"))
        .map(str::to_owned)
        .collect())
}

/// Query crt.sh certificate transparency for `hostname` (PTR of the IP).
/// Returns unique domain names extracted from certificate SANs.
/// Returns `Ok(vec![])` when no hostname is available.
async fn query_crtsh(
    client: &reqwest::Client,
    hostname: Option<&str>,
) -> Result<Vec<String>, String> {
    let host = match hostname {
        Some(h) if !h.is_empty() => h,
        _ => return Ok(Vec::new()),
    };

    let url = format!("https://crt.sh/?q={host}&output=json");
    let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Ok(Vec::new()); // crt.sh down or rate-limited — non-blocking
    }

    let entries: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;

    let mut domains: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in &entries {
        if let Some(names) = entry["name_value"].as_str() {
            for name in names.split('\n') {
                let name = name.trim().trim_start_matches("*.").to_owned();
                if !name.is_empty() && name.contains('.') {
                    domains.insert(name);
                }
            }
        }
    }

    Ok(domains.into_iter().collect())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Push `record` into `store`, evicting the oldest entries when the session
/// cap is reached.  Holds the write lock only for the duration of the push.
async fn push_record(store: &Arc<RwLock<Vec<IpRecord>>>, record: IpRecord) {
    let mut s = store.write().await;
    s.push(record);
    let overflow = s.len().saturating_sub(MAX_SESSION_RECORDS);
    if overflow > 0 {
        s.drain(0..overflow);
    }
}

/// Apply `ProxyConfig` entries to a `reqwest::ClientBuilder`.
/// Returns the builder unchanged when no proxy is configured.
pub(crate) fn apply_proxy(
    mut builder: reqwest::ClientBuilder,
    proxy: &crate::config::ProxyConfig,
) -> anyhow::Result<reqwest::ClientBuilder> {
    if let Some(url) = &proxy.http {
        builder = builder.proxy(reqwest::Proxy::http(url)?);
    }
    if let Some(url) = &proxy.https {
        builder = builder.proxy(reqwest::Proxy::https(url)?);
    }
    if let Some(url) = &proxy.socks4 {
        builder = builder.proxy(reqwest::Proxy::all(url)?);
    }
    if let Some(url) = &proxy.socks5 {
        builder = builder.proxy(reqwest::Proxy::all(url)?);
    }
    Ok(builder)
}

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
        assert_eq!(json["locale"], "en-US");
        assert_eq!(json["dns_timeout_ms"], 2000);
    }

    #[tokio::test]
    async fn test_post_config_updates_fields() {
        let state = test_state();
        let app = build_router(state.clone());

        let body = serde_json::json!({ "locale": "en-US", "dns_timeout_ms": 1000 });
        let req = Request::builder()
            .method("POST")
            .uri("/config")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["locale"], "en-US");
        assert_eq!(json["dns_timeout_ms"], 1000);
        // Other fields must remain unchanged.
        assert_eq!(json["proxy_type"], "none");
    }

    // ── /metrics ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_metrics_returns_json() {
        let app = build_router(test_state());
        let req = Request::builder().uri("/metrics").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json["rdap"].is_object(),      "expected rdap object");
        assert!(json["whois"].is_object(),     "expected whois object");
        assert!(json["dns"].is_object(),       "expected dns object");
        assert!(json["fallbacks"].is_number(), "expected fallbacks counter");
        assert!(json["cache_hits"].is_number(),"expected cache_hits counter");
        // Fresh state — all counters must be zero.
        assert_eq!(json["fallbacks"],  0);
        assert_eq!(json["cache_hits"], 0);
        assert_eq!(json["rdap"]["requests"], 0);
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

    // ── Global timeout recorded in lookup_errors ─────────────────────────────

    #[tokio::test]
    async fn test_global_timeout_recorded() {
        // Build a config with very short timeouts — all steps will fail quickly.
        let mut config = crate::config::AppConfig::default();
        config.dns_timeout_ms   = 1;
        config.rdap_timeout_ms  = 1;
        config.whois_timeout_ms = 1;

        let cache   = LookupCache::new(Duration::from_secs(3600));
        let metrics = RequestMetrics::new();
        let record  = perform_lookup(1, "8.8.8.8", &config, &cache, &metrics).await;
        // With 1 ms timeouts the pipeline must produce at least one error.
        assert!(
            !record.lookup_errors.is_empty(),
            "Expected lookup errors with 1 ms timeouts, got none"
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
