mod cli;
mod config;
mod dns;
mod export;
mod http;
mod i18n;
mod models;
mod rdap;
mod utils;
mod whois;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise structured logging.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "atlasip=info".into()),
        )
        .init();

    let cli = cli::Cli::parse();
    cli::run(cli).await
}
