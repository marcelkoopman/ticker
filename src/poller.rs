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
