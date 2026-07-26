use coop_cloud_docker_apps::coop_cloud_apps;
use tracing::info;

pub fn handle_apps() {
    let apps = coop_cloud_apps();
    if apps.is_empty() {
        info!("No co-op cloud apps installed.");
    } else {
        for app in apps {
            info!(name = %app.name, version = %app.version, "App");
        }
    }
}
