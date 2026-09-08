use crate::config::Asset;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct AssetPollInfo {
    pub name: String,
    pub next_poll: SystemTime,
}

pub struct Poller {
    poll_times: Vec<AssetPollInfo>,
}

impl Poller {
    pub fn new(assets: &[Asset]) -> Self {
        let now = SystemTime::now();
        let poll_times = assets
            .iter()
            .map(|asset| AssetPollInfo {
                name: asset.name.clone(),
                next_poll: now,
            })
            .collect();

        Poller { poll_times }
    }

    pub fn should_poll(&self, asset_name: &str) -> bool {
        self.poll_times
            .iter()
            .find(|p| p.name == asset_name)
            .map(|p| SystemTime::now() >= p.next_poll)
            .unwrap_or(false)
    }

    pub fn mark_polled(&mut self, asset_name: &str, assets: &[Asset]) {
        if let Some(asset) = assets.iter().find(|a| a.name == asset_name)
            && let Some(poll_info) = self.poll_times.iter_mut().find(|p| p.name == asset_name)
        {
            let interval = parse_interval(&asset.poll_interval);
            poll_info.next_poll = SystemTime::now() + interval;
        }
    }

    pub fn time_until_poll(&self, asset_name: &str) -> Option<std::time::Duration> {
        self.poll_times
            .iter()
            .find(|p| p.name == asset_name)
            .and_then(|p| p.next_poll.duration_since(SystemTime::now()).ok())
    }
}

fn parse_interval(interval_str: &str) -> std::time::Duration {
    let (num_str, unit) = interval_str.split_at(interval_str.len() - 1);
    let num: u64 = num_str.parse().unwrap_or(1);

    match unit {
        "s" => std::time::Duration::from_secs(num),
        "m" => std::time::Duration::from_secs(num * 60),
        "h" => std::time::Duration::from_secs(num * 3600),
        "d" => std::time::Duration::from_secs(num * 86400),
        _ => std::time::Duration::from_secs(3600),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_asset(name: &str, poll_interval: &str) -> Asset {
        Asset {
            name: name.to_string(),
            url: "https://example.com".to_string(),
            price_path: "price".to_string(),
            unit: "EUR".to_string(),
            symbol: "B".to_string(),
            poll_interval: poll_interval.to_string(),
        }
    }

    #[test]
    fn parse_interval_seconds() {
        assert_eq!(parse_interval("30s"), Duration::from_secs(30));
        assert_eq!(parse_interval("1s"), Duration::from_secs(1));
    }

    #[test]
    fn parse_interval_minutes() {
        assert_eq!(parse_interval("1m"), Duration::from_secs(60));
        assert_eq!(parse_interval("5m"), Duration::from_secs(300));
    }

    #[test]
    fn parse_interval_hours() {
        assert_eq!(parse_interval("1h"), Duration::from_secs(3600));
        assert_eq!(parse_interval("2h"), Duration::from_secs(7200));
    }

    #[test]
    fn parse_interval_days() {
        assert_eq!(parse_interval("1d"), Duration::from_secs(86400));
    }

    #[test]
    fn parse_interval_invalid_defaults_to_1h() {
        assert_eq!(parse_interval("xyz"), Duration::from_secs(3600));
        assert_eq!(parse_interval("10x"), Duration::from_secs(3600));
    }

    #[test]
    fn parse_interval_invalid_number_defaults_to_1() {
        // "xm" -> num parse fails -> 1, unit m -> 60s
        assert_eq!(parse_interval("xm"), Duration::from_secs(60));
    }

    #[test]
    fn new_poller_marks_all_due_immediately() {
        let assets = vec![make_asset("Bitcoin", "1m"), make_asset("Gold", "1h")];
        let poller = Poller::new(&assets);

        assert!(poller.should_poll("Bitcoin"));
        assert!(poller.should_poll("Gold"));
        assert!(!poller.should_poll("Unknown"));
    }

    #[test]
    fn mark_polled_sets_next_poll_in_future() {
        let assets = vec![make_asset("Bitcoin", "1h")];
        let mut poller = Poller::new(&assets);

        assert!(poller.should_poll("Bitcoin"));

        poller.mark_polled("Bitcoin", &assets);

        // After marking, should not be due yet
        assert!(!poller.should_poll("Bitcoin"));

        let remaining = poller.time_until_poll("Bitcoin").expect("should have time");
        // Allow some slack for test execution time
        assert!(remaining > Duration::from_secs(3500));
        assert!(remaining <= Duration::from_secs(3600));
    }

    #[test]
    fn mark_polled_unknown_asset_is_noop() {
        let assets = vec![make_asset("Bitcoin", "1m")];
        let mut poller = Poller::new(&assets);

        poller.mark_polled("DoesNotExist", &assets);
        // Bitcoin still due
        assert!(poller.should_poll("Bitcoin"));
    }

    #[test]
    fn time_until_poll_none_for_unknown() {
        let assets = vec![make_asset("Bitcoin", "1m")];
        let poller = Poller::new(&assets);

        assert!(poller.time_until_poll("Unknown").is_none());
    }

    #[test]
    fn time_until_poll_zero_or_none_when_due() {
        let assets = vec![make_asset("Bitcoin", "1m")];
        let poller = Poller::new(&assets);

        // Just created -> due now -> duration_since fails or is ~0
        let remaining = poller.time_until_poll("Bitcoin");
        // Either None (already past) or very small
        if let Some(d) = remaining {
            assert!(d < Duration::from_secs(1));
        }
    }
}
