use coop_cloud_docker_apps::coop_cloud_apps;

use crate::avahi::{AvahiClient, AvahiState, PublishedRecord};

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
        eprintln!("error: Avahi is not running (state: {})", status.state.label());
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

    let apps = coop_cloud_apps();
    if apps.is_empty() {
        println!("No co-op cloud apps installed; nothing to publish.");
        return;
    }

    // Keep entry groups alive for as long as the process runs.
    let mut _groups: Vec<PublishedRecord> = Vec::new();

    for app in &apps {
        let record_name = format!("{}-{}.local", app.name, hostname);
        match client.publish_address(&record_name, &address).await {
            Ok(group) => {
                println!("Published: {} → {}", record_name, address);
                _groups.push(group);
            }
            Err(e) => {
                eprintln!("warning: could not publish {record_name}: {e}");
            }
        }
    }

    println!("Press Ctrl+C to stop publishing.");
    tokio::signal::ctrl_c().await.ok();
    println!("Shutting down; records will be withdrawn.");
}
