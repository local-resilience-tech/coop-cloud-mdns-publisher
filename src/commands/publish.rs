use std::time::Duration;

use coop_cloud_docker_apps::coop_cloud_apps;
use tokio::time;

use crate::avahi::{AvahiClient, AvahiState};

use crate::helpers::published_apps::PublishedApps;

/// How often to re-check the set of deployed co-op cloud apps.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// How long an app must be continuously absent before its mDNS record is withdrawn.
const WITHDRAWAL_GRACE_PERIOD: Duration = Duration::from_secs(300);

pub async fn handle_publish() {
    let client = match AvahiClient::connect().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("Is avahi-daemon installed and running?");
            std::process::exit(1);
        }
    };

    let status = match client.status().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    if status.state != AvahiState::Running {
        eprintln!(
            "error: Avahi is not running (state: {})",
            status.state.label()
        );
        std::process::exit(1);
    }

    let address = match &status.local_address {
        Some(a) => a.clone(),
        None => {
            eprintln!("error: could not determine local IP address from Avahi");
            std::process::exit(1);
        }
    };

    let hostname = match client.get_host_name().await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

    let mut published = PublishedApps::new(WITHDRAWAL_GRACE_PERIOD);

    println!("Press Ctrl+C to stop publishing.");

    let mut interval = time::interval(POLL_INTERVAL);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let apps = coop_cloud_apps();

                for name in published.to_withdraw(&apps) {
                    println!("Withdrawn (grace period elapsed): {name}");
                    published.remove(&name);
                }

                for app in published.to_publish(&apps) {
                    let record_name = format!("{}-{}.local", app.name, hostname);
                    match client.publish_address(&record_name, &address).await {
                        Ok(group) => {
                            println!("Published: {} → {}", record_name, address);
                            published.insert(app.name.clone(), group);
                        }
                        Err(e) => {
                            eprintln!("warning: could not publish {record_name}: {e}");
                        }
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Shutting down; records will be withdrawn.");
                break;
            }
        }
    }
}
