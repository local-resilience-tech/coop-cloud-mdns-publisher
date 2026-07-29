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

struct PublishedApp<R> {
    #[allow(dead_code)] // held to keep the record alive via Drop
    record: R,
    state: AppState,
}

/// Tracks which apps currently have active mDNS records for a specific host IP address and hostname.
///
/// The type parameter `R` is the record handle (e.g. [`PublishedRecord`] in production,
/// `()` in tests).
pub struct PublishedApps<R = PublishedRecord> {
    entries: HashMap<String, PublishedApp<R>>,
    grace_period: Duration,
    address: String,
    hostname: String,
}

impl<R> PublishedApps<R> {
    pub fn new(grace_period: Duration, address: String, hostname: String) -> Self {
        Self {
            entries: HashMap::new(),
            grace_period,
            address,
            hostname,
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns `true` if both `addr` and `hostname` match what this instance was created for.
    pub fn matches(&self, addr: &str, hostname: &str) -> bool {
        self.address == addr && self.hostname == hostname
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

    pub fn insert(&mut self, name: String, record: R) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenience alias — no real record needed in tests.
    type TestApps = PublishedApps<()>;

    fn app(name: &str) -> CoopCloudApp {
        CoopCloudApp {
            name: name.to_string(),
            recipe: name.to_string(),
            version: Some("1.0.0".to_string()),
            url: None,
            lores: None,
        }
    }

    fn instant_grace() -> Duration {
        Duration::ZERO
    }

    fn long_grace() -> Duration {
        Duration::from_secs(300)
    }

    #[test]
    fn to_publish_returns_all_when_empty() {
        let pa = TestApps::new(
            long_grace(),
            "127.0.0.1".to_string(),
            "testhost".to_string(),
        );
        let apps = vec![app("foo"), app("bar")];
        let result: Vec<&str> = pa
            .to_publish(&apps)
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(result, vec!["foo", "bar"]);
    }

    #[test]
    fn to_publish_excludes_already_published() {
        let mut pa = TestApps::new(
            long_grace(),
            "127.0.0.1".to_string(),
            "testhost".to_string(),
        );
        pa.insert("foo".to_string(), ());
        let apps = vec![app("foo"), app("bar")];
        let result: Vec<&str> = pa
            .to_publish(&apps)
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(result, vec!["bar"]);
    }

    #[test]
    fn to_withdraw_not_triggered_within_grace_period() {
        let mut pa = TestApps::new(
            long_grace(),
            "127.0.0.1".to_string(),
            "testhost".to_string(),
        );
        pa.insert("foo".to_string(), ());
        // First call with empty list — starts grace period.
        let withdrawn = pa.to_withdraw(&[]);
        assert!(withdrawn.is_empty());
        // Second call still within grace period — not yet withdrawn.
        let withdrawn = pa.to_withdraw(&[]);
        assert!(withdrawn.is_empty());
    }

    #[test]
    fn to_withdraw_triggers_after_grace_period() {
        let mut pa = TestApps::new(
            instant_grace(),
            "127.0.0.1".to_string(),
            "testhost".to_string(),
        );
        pa.insert("foo".to_string(), ());
        // First call starts the pending state.
        pa.to_withdraw(&[]);
        // Second call — elapsed >= Duration::ZERO, so it should withdraw.
        let withdrawn = pa.to_withdraw(&[]);
        assert_eq!(withdrawn, vec!["foo"]);
    }

    #[test]
    fn reappearing_app_cancels_pending_withdrawal() {
        let mut pa = TestApps::new(
            long_grace(),
            "127.0.0.1".to_string(),
            "testhost".to_string(),
        );
        pa.insert("foo".to_string(), ());
        // App goes absent — starts grace period.
        pa.to_withdraw(&[]);
        // App comes back — grace period should be cancelled.
        let withdrawn = pa.to_withdraw(&[app("foo")]);
        assert!(withdrawn.is_empty());
        // App goes absent again — should start a fresh grace period, not withdraw immediately.
        let withdrawn = pa.to_withdraw(&[]);
        assert!(withdrawn.is_empty());
    }

    #[test]
    fn remove_prevents_future_withdrawal() {
        let mut pa = TestApps::new(
            instant_grace(),
            "127.0.0.1".to_string(),
            "testhost".to_string(),
        );
        pa.insert("foo".to_string(), ());
        let to_withdraw = pa.to_withdraw(&[]);
        // Grace period elapsed on second call; but we remove before that.
        pa.remove("foo");
        let withdrawn = pa.to_withdraw(&[]);
        assert!(withdrawn.is_empty());
        let _ = to_withdraw; // suppress unused warning
    }
}
