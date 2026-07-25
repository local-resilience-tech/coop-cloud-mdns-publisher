use crate::avahi::{AvahiClient, AvahiState};

pub async fn handle_status() {
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

    println!("{} — {}", status.version, status.state.label());
    println!("Hostname: {}", status.fqdn);

    if status.state != AvahiState::Running {
        std::process::exit(1);
    }
}
