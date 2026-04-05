mod cache;
mod cli;
mod config;
mod dns;
mod export;
mod http;
mod i18n;
mod metrics;
mod models;
mod rdap;
mod retry;
mod utils;
mod whois;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Structured logging (P3-PERF-015).
    // Set RUST_LOG_FORMAT=json to emit newline-delimited JSON logs.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "atlasip=info".into());

    if std::env::var("RUST_LOG_FORMAT").as_deref() == Ok("json") {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .init();
    }

    let cli = cli::Cli::parse();
    cli::run(cli).await
}
