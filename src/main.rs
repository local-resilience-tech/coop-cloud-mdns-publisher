mod apps;
mod status;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ccmdns", about = "Co-op Cloud mDNS publisher")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List installed co-op cloud apps
    Apps,
    /// Check whether Avahi is installed and running
    Status,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Apps => apps::handle_apps(),
        Commands::Status => status::handle_status().await,
    }
}
