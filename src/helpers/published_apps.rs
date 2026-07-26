use std::collections::HashMap;
use std::time::{Duration, Instant};

use coop_cloud_docker_apps::CoopCloudApp;

use crate::avahi::PublishedRecord;

enum AppState {
    /// Record is active and the app is present.
    Published,
    /// App was absent at the given instant; record is kept until the grace period expires.
    PendingWithdrawal(Instant),
}

struct PublishedApp {
    #[allow(dead_code)] // held to keep the Avahi record alive via Drop
    record: PublishedRecord,
    state: AppState,
}

/// Tracks which apps currently have active mDNS records.
pub struct PublishedApps {
    entries: HashMap<String, PublishedApp>,
    grace_period: Duration,
}

impl PublishedApps {
    pub fn new(grace_period: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            grace_period,
        }
    }

    /// Apps in `current` that do not yet have a published record.
    pub fn to_publish<'a>(&self, current: &'a [CoopCloudApp]) -> Vec<&'a CoopCloudApp> {
        current
            .iter()
            .filter(|a| !self.entries.contains_key(&a.name))
            .collect()
    }

    /// Names of apps whose grace period has expired and whose records should now be withdrawn.
    /// Also transitions newly absent apps to `PendingWithdrawal` and restores reappearing apps
    /// to `Published`.
    pub fn to_withdraw(&mut self, current: &[CoopCloudApp]) -> Vec<String> {
        let current_names: std::collections::HashSet<&str> =
            current.iter().map(|a| a.name.as_str()).collect();

        let grace_period = self.grace_period;
        let mut to_withdraw = Vec::new();
        for (name, entry) in self.entries.iter_mut() {
            if current_names.contains(name.as_str()) {
                // App is present; clear any pending withdrawal.
                entry.state = AppState::Published;
            } else {
                match entry.state {
                    AppState::Published => {
                        // First poll where app is absent — start the grace period.
                        entry.state = AppState::PendingWithdrawal(Instant::now());
                    }
                    AppState::PendingWithdrawal(since) => {
                        if since.elapsed() >= grace_period {
                            to_withdraw.push(name.clone());
                        }
                    }
                }
            }
        }
        to_withdraw
    }

    pub fn insert(&mut self, name: String, record: PublishedRecord) {
        self.entries.insert(
            name,
            PublishedApp {
                record,
                state: AppState::Published,
            },
        );
    }

    pub fn remove(&mut self, name: &str) {
        self.entries.remove(name);
    }
}
