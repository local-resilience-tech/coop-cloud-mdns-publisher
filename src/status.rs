use zbus::{Connection, proxy};

#[proxy(
    interface = "org.freedesktop.Avahi.Server",
    default_service = "org.freedesktop.Avahi",
    default_path = "/"
)]
trait AvahiServer {
    async fn get_state(&self) -> zbus::Result<i32>;
    async fn get_version_string(&self) -> zbus::Result<String>;
    async fn get_host_name_fqdn(&self) -> zbus::Result<String>;
}

pub async fn handle_status() {
    let conn = match Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: could not connect to system D-Bus: {e}");
            std::process::exit(1);
        }
    };

    let proxy = match AvahiServerProxy::new(&conn).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: could not reach Avahi on D-Bus: {e}");
            eprintln!("Is avahi-daemon installed and running?");
            std::process::exit(1);
        }
    };

    let version = match proxy.get_version_string().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: Avahi did not respond: {e}");
            std::process::exit(1);
        }
    };

    let state = match proxy.get_state().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not get Avahi state: {e}");
            std::process::exit(1);
        }
    };

    let state_label = match state {
        0 => "Invalid",
        1 => "Registering",
        2 => "Running",
        3 => "Collision",
        4 => "Failure",
        _ => "Unknown",
    };

    let fqdn = proxy.get_host_name_fqdn().await.unwrap_or_else(|_| "(unknown)".to_string());

    println!("{version} — {state_label}");
    println!("Hostname: {fqdn}");

    if state != 2 {
        std::process::exit(1);
    }
}
