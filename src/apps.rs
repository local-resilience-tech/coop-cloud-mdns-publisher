use coop_cloud_docker_apps::coop_cloud_apps;

pub fn handle_apps() {
    let apps = coop_cloud_apps();
    if apps.is_empty() {
        println!("No co-op cloud apps installed.");
    } else {
        for app in apps {
            println!("{} ({})", app.name, app.version);
        }
    }
}
