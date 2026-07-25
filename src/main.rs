use clap::{Parser, Subcommand};
use coop_cloud_docker_apps::coop_cloud_apps;

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
        Commands::Apps => {
            let apps = coop_cloud_apps();
            if apps.is_empty() {
                println!("No co-op cloud apps installed.");
            } else {
                for app in apps {
                    println!("{} ({})", app.name, app.version);
                }
            }
        }
    }
}
