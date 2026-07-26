mod avahi;
mod commands;
mod helpers;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[derive(Parser)]
#[command(name = "ccmdns", about = "Co-op Cloud mDNS publisher")]
struct Cli {
    /// Write logs to a file in this directory (in addition to stdout)
    #[arg(long, value_name = "DIR")]
    log_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List installed co-op cloud apps
    Apps,
    /// Publish mDNS A records for each installed app
    Publish,
    /// Check whether Avahi is installed and running
    Status,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let stdout_layer = fmt::layer().with_writer(std::io::stdout);

    let file_layer = cli.log_dir.as_ref().map(|dir| {
        let appender = rolling::RollingFileAppender::builder()
            .filename_prefix("ccmdns.log")
            .rotation(rolling::Rotation::DAILY)
            .build(dir)
            .unwrap_or_else(|e| panic!("failed to open log directory {}: {e}", dir.display()));
        let (non_blocking, guard) = tracing_appender::non_blocking(appender);
        std::mem::forget(guard);
        fmt::layer().with_writer(non_blocking).with_ansi(false)
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    match cli.command {
        Commands::Apps => commands::apps::handle_apps(),
        Commands::Publish => commands::publish::handle_publish().await,
        Commands::Status => commands::status::handle_status().await,
    }
}
