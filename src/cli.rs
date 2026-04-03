use clap::{Parser, Subcommand};

/// AtlasIP — Modern IP analysis tool.
#[derive(Parser, Debug)]
#[command(name = "atlasip", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Analyse one or more IP addresses / hostnames.
    Lookup {
        /// Target IP address or hostname (omit to read from --file).
        target: Option<String>,

        /// Path to a file containing one target per line.
        #[arg(long)]
        file: Option<std::path::PathBuf>,

        /// Output format: table | json | csv (spec §2.5).
        #[arg(long, default_value = "table")]
        format: String,

        /// Write output to this file instead of stdout.
        #[arg(long, short)]
        output: Option<std::path::PathBuf>,

        /// Skip WHOIS; use RDAP only.
        #[arg(long)]
        rdap_only: bool,

        /// Skip RDAP; use WHOIS only.
        #[arg(long)]
        whois_only: bool,

        /// Skip DNS resolution.
        #[arg(long)]
        no_dns: bool,
    },

    /// Start the local HTTP API server (127.0.0.1:8080).
    Serve {
        /// Override the bind address.
        #[arg(long, default_value = "127.0.0.1:8080")]
        bind: String,
    },
}

/// Entry point for CLI mode. Called by `main` when not in server mode.
pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Command::Lookup {
            target,
            file,
            format,
            output,
            rdap_only,
            whois_only,
            no_dns,
        } => {
            // TODO: collect targets from `target` and/or `file`, clean them
            // with utils::clean_targets, run the lookup pipeline for each,
            // render with export::export (or table renderer), write to
            // `output` or stdout.
            todo!("cli lookup")
        }
        Command::Serve { bind } => {
            // TODO: load AppConfig, build AppState, call http::build_router,
            // bind with tokio::net::TcpListener, serve with axum::serve.
            todo!("cli serve")
        }
    }
}
