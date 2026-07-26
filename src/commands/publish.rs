use std::time::Duration;

use coop_cloud_docker_apps::{CoopCloudApp, coop_cloud_apps};
use tokio::time;
use tracing::{error, info, warn};

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
            error!("{}\nIs avahi-daemon installed and running?", e);
            std::process::exit(1);
        }
    };

    let status = match client.status().await {
        Ok(s) => s,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    if status.state != AvahiState::Running {
        error!(state = %status.state.label(), "Avahi is not running");
        std::process::exit(1);
    }

    let address = match &status.local_address {
        Some(a) => a.clone(),
        None => {
            error!("Could not determine local IP address from Avahi");
            std::process::exit(1);
        }
    };

    let hostname = match client.get_host_name().await {
        Ok(h) => h,
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    };

    let mut published = PublishedApps::new(WITHDRAWAL_GRACE_PERIOD, address, hostname);

    info!("Press Ctrl+C to stop publishing.");

    let mut interval = time::interval(POLL_INTERVAL);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                // Re-check the host's IP and hostname; if either changed, replace published
                // (dropping all records) so every app is re-published below.
                match (client.status().await, client.get_host_name().await) {
                    (Ok(s), Ok(new_hostname)) => {
                        if let Some(new_address) = s.local_address {
                            if !published.matches(&new_address, &new_hostname) {
                                info!(
                                    old_address = %published.address(),
                                    new_address = %new_address,
                                    old_hostname = %published.hostname(),
                                    new_hostname = %new_hostname,
                                    "Host identity changed; re-publishing all records",
                                );
                                published = PublishedApps::new(WITHDRAWAL_GRACE_PERIOD, new_address, new_hostname);
                            }
                        }
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        warn!("Could not re-check host identity: {e}");
                    }
                }

                let apps = coop_cloud_apps();

                for name in published.to_withdraw(&apps) {
                    info!(name = %name, "Withdrawn (grace period elapsed)");
                    published.remove(&name);
                }

                for app in published.to_publish(&apps) {
                    let record_name = host_name_for_app(app, published.hostname());
                    match client.publish_address(&record_name, published.address()).await {
                        Ok(group) => {
                            info!(record = %record_name, address = %published.address(), "Published");
                            published.insert(app.name.clone(), group);
                        }
                        Err(e) => {
                            warn!("Could not publish {record_name}: {e}");
                        }
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down; records will be withdrawn.");
                break;
            }
        }
    }
}

fn host_name_for_app(app: &CoopCloudApp, hostname: &str) -> String {
    format!("{}-{}.local", app.name, hostname)
}
