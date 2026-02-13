use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_ssh_config_path")]
    pub ssh_config_path: PathBuf,
    #[serde(default = "default_socket_dir")]
    pub socket_dir: PathBuf,
    #[serde(default)]
    pub auto_restore: bool,
    #[serde(default = "default_max_recent")]
    pub max_recent_hosts: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_true")]
    pub show_all_hosts: bool,
}

fn default_ssh_config_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".ssh/config")
}

fn default_socket_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".config/stm/sockets")
}

fn default_max_recent() -> usize {
    10
}

fn default_true() -> bool {
    true
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            ssh_config_path: default_ssh_config_path(),
            socket_dir: default_socket_dir(),
            auto_restore: false,
            max_recent_hosts: default_max_recent(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_all_hosts: true,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".config/stm/config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => toml::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    #[allow(dead_code)]
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

/// Ensure config directory and example config exist.
pub fn ensure_config_dir() -> anyhow::Result<PathBuf> {
    let config_dir = dirs::home_dir().unwrap_or_default().join(".config/stm");
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(!config.general.auto_restore);
        assert_eq!(config.general.max_recent_hosts, 10);
        assert!(config.ui.show_all_hosts);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = AppConfig::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(
            config.general.max_recent_hosts,
            deserialized.general.max_recent_hosts
        );
        assert_eq!(
            config.general.auto_restore,
            deserialized.general.auto_restore
        );
    }

    #[test]
    fn test_partial_config_parse() {
        let toml_str = r#"
[general]
auto_restore = true
"#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(config.general.auto_restore);
        assert_eq!(config.general.max_recent_hosts, 10); // default
    }

    #[test]
    fn test_empty_config_parse() {
        let config: AppConfig = toml::from_str("").unwrap();
        assert!(!config.general.auto_restore);
    }
}
