mod apps;

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
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Apps => apps::handle_apps(),
    }
}
