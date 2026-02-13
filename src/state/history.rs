use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::ssh::tunnel::Tunnel;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct History {
    pub hosts: HashMap<String, HostHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostHistory {
    pub last_used: DateTime<Utc>,
    pub use_count: u32,
    pub tunnels: Vec<SavedTunnel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedTunnel {
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
}

impl From<&Tunnel> for SavedTunnel {
    fn from(t: &Tunnel) -> Self {
        Self {
            local_port: t.local_port,
            remote_host: t.remote_host.clone(),
            remote_port: t.remote_port,
        }
    }
}

impl History {
    pub fn history_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".config/stm/history.json")
    }

    pub fn load() -> Self {
        let path = Self::history_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::history_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn record_connection(&mut self, host_name: &str) {
        let entry = self
            .hosts
            .entry(host_name.to_string())
            .or_insert(HostHistory {
                last_used: Utc::now(),
                use_count: 0,
                tunnels: Vec::new(),
            });
        entry.last_used = Utc::now();
        entry.use_count += 1;
    }

    pub fn save_tunnels(&mut self, host_name: &str, tunnels: &[Tunnel]) {
        if let Some(entry) = self.hosts.get_mut(host_name) {
            entry.tunnels = tunnels.iter().map(SavedTunnel::from).collect();
        }
    }

    pub fn get_saved_tunnels(&self, host_name: &str) -> Vec<SavedTunnel> {
        self.hosts
            .get(host_name)
            .map(|h| h.tunnels.clone())
            .unwrap_or_default()
    }

    #[allow(dead_code)]
    pub fn recent_hosts(&self) -> Vec<String> {
        let mut entries: Vec<_> = self.hosts.iter().collect();
        entries.sort_by(|a, b| b.1.last_used.cmp(&a.1.last_used));
        entries.into_iter().map(|(name, _)| name.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_roundtrip() {
        let mut history = History::default();
        history.record_connection("myhost");
        history.record_connection("myhost");

        let json = serde_json::to_string(&history).unwrap();
        let restored: History = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.hosts["myhost"].use_count, 2);
    }

    #[test]
    fn test_save_tunnels() {
        let mut history = History::default();
        history.record_connection("myhost");

        let tunnels = vec![Tunnel::new(5432, "localhost".to_string(), 5432)];
        history.save_tunnels("myhost", &tunnels);

        let saved = history.get_saved_tunnels("myhost");
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].local_port, 5432);
    }

    #[test]
    fn test_recent_hosts_ordering() {
        let mut history = History::default();
        history.record_connection("old");
        // small delay to ensure ordering
        history.record_connection("new");

        let recent = history.recent_hosts();
        assert_eq!(recent[0], "new");
    }

    #[test]
    fn test_empty_history() {
        let history = History::default();
        assert!(history.recent_hosts().is_empty());
        assert!(history.get_saved_tunnels("nonexistent").is_empty());
    }
}
