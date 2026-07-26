use tracing::{error, info};

use crate::avahi::{AvahiClient, AvahiState};

pub async fn handle_status() {
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

    info!(version = %status.version, state = %status.state.label(), "Avahi status");
    info!(hostname = %status.fqdn, "Hostname");
    if let Some(addr) = &status.local_address {
        info!(address = %addr, "Address");
    }

    if status.state != AvahiState::Running {
        std::process::exit(1);
    }
}
