use std::collections::HashMap;
use std::time::Duration;

use coop_cloud_docker_apps::{CoopCloudApp, coop_cloud_apps};
use tokio::time;

use crate::avahi::{AvahiClient, AvahiState, PublishedRecord};

/// How often to re-check the set of deployed co-op cloud apps.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Tracks which apps currently have active mDNS records.
struct PublishedApps(HashMap<String, PublishedRecord>);

impl PublishedApps {
    fn new() -> Self {
        Self(HashMap::new())
    }

    /// Apps in `current` that do not yet have a published record.
    fn to_publish<'a>(&self, current: &'a [CoopCloudApp]) -> Vec<&'a CoopCloudApp> {
        current
            .iter()
            .filter(|a| !self.0.contains_key(&a.name))
            .collect()
    }

    /// Names of apps that have a published record but are absent from `current`.
    fn to_withdraw(&self, current: &[CoopCloudApp]) -> Vec<String> {
        let current_names: std::collections::HashSet<&str> =
            current.iter().map(|a| a.name.as_str()).collect();
        self.0
            .keys()
            .filter(|name| !current_names.contains(name.as_str()))
            .cloned()
            .collect()
    }

    fn insert(&mut self, name: String, record: PublishedRecord) {
        self.0.insert(name, record);
    }

    fn remove(&mut self, name: &str) {
        self.0.remove(name);
    }
}

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

    let mut published = PublishedApps::new();

    println!("Press Ctrl+C to stop publishing.");

    let mut interval = time::interval(POLL_INTERVAL);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let apps = coop_cloud_apps();

                for name in published.to_withdraw(&apps) {
                    println!("Withdrawn: {name}");
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
