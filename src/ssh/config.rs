use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct SshHost {
    pub name: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<PathBuf>,
    pub proxy_jump: Option<String>,
}

impl SshHost {
    /// Returns the effective hostname (hostname or name fallback).
    pub fn effective_hostname(&self) -> &str {
        self.hostname.as_deref().unwrap_or(&self.name)
    }

    /// Returns the effective port (port or 22 fallback).
    pub fn effective_port(&self) -> u16 {
        self.port.unwrap_or(22)
    }

    /// Returns the display string like "user@hostname" or just "hostname".
    pub fn display_target(&self) -> String {
        match &self.user {
            Some(user) => format!("{}@{}", user, self.effective_hostname()),
            None => self.effective_hostname().to_string(),
        }
    }
}

/// Parse an SSH config file into a list of host entries.
/// Skips wildcard-only hosts (e.g., `Host *`).
/// Handles `Include` directives by resolving paths relative to `~/.ssh/`.
pub fn parse_ssh_config(path: &Path) -> anyhow::Result<Vec<SshHost>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read SSH config at {}: {}", path.display(), e))?;
    parse_ssh_config_content(&content, path.parent())
}

fn parse_ssh_config_content(
    content: &str,
    config_dir: Option<&Path>,
) -> anyhow::Result<Vec<SshHost>> {
    let mut hosts = Vec::new();
    let mut current_host: Option<SshHost> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split on first whitespace or '='
        let (keyword, value) = match split_config_line(line) {
            Some(pair) => pair,
            None => continue,
        };

        let keyword_lower = keyword.to_lowercase();

        if keyword_lower == "host" {
            // Save previous host if any
            if let Some(host) = current_host.take() {
                if !is_wildcard_only(&host.name) {
                    hosts.push(host);
                }
            }

            // Start new host block
            // Skip patterns that are purely wildcards
            if !is_wildcard_only(value) {
                current_host = Some(SshHost {
                    name: value.to_string(),
                    ..Default::default()
                });
            }
        } else if keyword_lower == "match" {
            // Save previous host, skip Match blocks
            if let Some(host) = current_host.take() {
                if !is_wildcard_only(&host.name) {
                    hosts.push(host);
                }
            }
        } else if keyword_lower == "include" {
            // Save previous host before include
            if let Some(host) = current_host.take() {
                if !is_wildcard_only(&host.name) {
                    hosts.push(host);
                }
            }

            let include_hosts = resolve_include(value, config_dir)?;
            hosts.extend(include_hosts);
        } else if let Some(ref mut host) = current_host {
            match keyword_lower.as_str() {
                "hostname" => host.hostname = Some(value.to_string()),
                "user" => host.user = Some(value.to_string()),
                "port" => {
                    if let Ok(port) = value.parse::<u16>() {
                        host.port = Some(port);
                    }
                }
                "identityfile" => {
                    host.identity_file = Some(expand_tilde(value));
                }
                "proxyjump" => host.proxy_jump = Some(value.to_string()),
                _ => {} // Ignore unknown directives
            }
        }
    }

    // Don't forget the last host
    if let Some(host) = current_host {
        if !is_wildcard_only(&host.name) {
            hosts.push(host);
        }
    }

    Ok(hosts)
}

/// Split a config line into (keyword, value), handling both whitespace and '=' separators.
fn split_config_line(line: &str) -> Option<(&str, &str)> {
    // Try splitting on '=' first
    if let Some(eq_pos) = line.find('=') {
        let keyword = line[..eq_pos].trim();
        let value = line[eq_pos + 1..].trim();
        if !keyword.is_empty() && !value.is_empty() {
            return Some((keyword, value));
        }
    }

    // Split on whitespace
    let mut parts = line.splitn(2, |c: char| c.is_whitespace());
    let keyword = parts.next()?.trim();
    let value = parts.next()?.trim();

    if keyword.is_empty() || value.is_empty() {
        return None;
    }

    Some((keyword, value))
}

/// Check if a host pattern is wildcard-only (e.g., "*", "* !bastion").
fn is_wildcard_only(name: &str) -> bool {
    let parts: Vec<&str> = name.split_whitespace().collect();
    parts
        .iter()
        .all(|p| p.starts_with('*') || p.starts_with('!'))
}

/// Expand `~` at the start of a path to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Resolve an Include directive, which can be a glob or a path.
fn resolve_include(pattern: &str, config_dir: Option<&Path>) -> anyhow::Result<Vec<SshHost>> {
    let expanded = if pattern.starts_with('~') || pattern.starts_with('/') {
        expand_tilde(pattern)
    } else {
        // Relative paths are relative to the config directory (usually ~/.ssh/)
        match config_dir {
            Some(dir) => dir.join(pattern),
            None => PathBuf::from(pattern),
        }
    };

    let pattern_str = expanded.to_string_lossy().to_string();

    let mut all_hosts = Vec::new();

    // Handle glob patterns
    if pattern_str.contains('*') || pattern_str.contains('?') {
        if let Ok(paths) = glob_paths(&pattern_str) {
            for path in paths {
                if path.is_file() {
                    match parse_ssh_config(&path) {
                        Ok(hosts) => all_hosts.extend(hosts),
                        Err(_) => continue, // Skip unreadable includes
                    }
                }
            }
        }
    } else if expanded.is_file() {
        all_hosts = parse_ssh_config(&expanded)?;
    }

    Ok(all_hosts)
}

/// Simple glob matching for Include directives.
fn glob_paths(pattern: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut results = Vec::new();

    // Find the directory part (everything before the first glob character)
    let path = Path::new(pattern);
    if let Some(parent) = path.parent() {
        let file_pattern = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        if parent.is_dir() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if matches_simple_glob(&name, &file_pattern) {
                        results.push(entry.path());
                    }
                }
            }
        }
    }

    results.sort();
    Ok(results)
}

/// Very simple glob matcher supporting only `*` wildcards.
fn matches_simple_glob(name: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if let Some(suffix) = pattern.strip_prefix('*') {
        return name.ends_with(suffix);
    }

    if let Some(prefix) = pattern.strip_suffix('*') {
        return name.starts_with(prefix);
    }

    name == pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_host() {
        let config = r#"
Host myserver
    HostName 192.168.1.100
    User admin
    Port 2222
    IdentityFile ~/.ssh/id_rsa
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "myserver");
        assert_eq!(hosts[0].hostname.as_deref(), Some("192.168.1.100"));
        assert_eq!(hosts[0].user.as_deref(), Some("admin"));
        assert_eq!(hosts[0].port, Some(2222));
        assert!(hosts[0].identity_file.is_some());
    }

    #[test]
    fn test_multiple_hosts() {
        let config = r#"
Host prod
    HostName prod.example.com
    User deploy

Host staging
    HostName staging.example.com
    User deploy
    Port 2222
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].name, "prod");
        assert_eq!(hosts[1].name, "staging");
    }

    #[test]
    fn test_skip_wildcard_host() {
        let config = r#"
Host *
    ServerAliveInterval 60
    ServerAliveCountMax 3

Host myserver
    HostName 10.0.0.1
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "myserver");
    }

    #[test]
    fn test_comments_and_blank_lines() {
        let config = r#"
# This is a comment
Host server1
    # Another comment
    HostName 10.0.0.1

    User root
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].user.as_deref(), Some("root"));
    }

    #[test]
    fn test_case_insensitive_keywords() {
        let config = r#"
host myserver
    hostname 10.0.0.1
    USER admin
    PORT 22
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("10.0.0.1"));
        assert_eq!(hosts[0].user.as_deref(), Some("admin"));
        assert_eq!(hosts[0].port, Some(22));
    }

    #[test]
    fn test_equals_separator() {
        let config = r#"
Host myserver
    HostName=10.0.0.1
    User=admin
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].hostname.as_deref(), Some("10.0.0.1"));
        assert_eq!(hosts[0].user.as_deref(), Some("admin"));
    }

    #[test]
    fn test_invalid_port_ignored() {
        let config = r#"
Host myserver
    HostName 10.0.0.1
    Port not_a_number
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].port, None);
    }

    #[test]
    fn test_proxy_jump() {
        let config = r#"
Host internal
    HostName 10.0.0.50
    ProxyJump bastion
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].proxy_jump.as_deref(), Some("bastion"));
    }

    #[test]
    fn test_effective_hostname_fallback() {
        let host = SshHost {
            name: "myalias".to_string(),
            ..Default::default()
        };
        assert_eq!(host.effective_hostname(), "myalias");
    }

    #[test]
    fn test_effective_port_default() {
        let host = SshHost::default();
        assert_eq!(host.effective_port(), 22);
    }

    #[test]
    fn test_display_target_with_user() {
        let host = SshHost {
            name: "myserver".to_string(),
            hostname: Some("10.0.0.1".to_string()),
            user: Some("admin".to_string()),
            ..Default::default()
        };
        assert_eq!(host.display_target(), "admin@10.0.0.1");
    }

    #[test]
    fn test_display_target_without_user() {
        let host = SshHost {
            name: "myserver".to_string(),
            hostname: Some("10.0.0.1".to_string()),
            ..Default::default()
        };
        assert_eq!(host.display_target(), "10.0.0.1");
    }

    #[test]
    fn test_wildcard_negation_skip() {
        let config = r#"
Host * !bastion
    ForwardAgent no

Host bastion
    HostName bastion.example.com
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "bastion");
    }

    #[test]
    fn test_empty_config() {
        let config = "";
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert!(hosts.is_empty());
    }

    #[test]
    fn test_tilde_expansion() {
        let path = expand_tilde("~/foo/bar");
        assert!(!path.to_string_lossy().starts_with('~'));
        assert!(path.to_string_lossy().ends_with("foo/bar"));
    }

    #[test]
    fn test_no_tilde() {
        let path = expand_tilde("/absolute/path");
        assert_eq!(path, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_match_block_handled() {
        let config = r#"
Host server1
    HostName 10.0.0.1

Match host *.example.com
    ForwardAgent yes

Host server2
    HostName 10.0.0.2
"#;
        let hosts = parse_ssh_config_content(config, None).unwrap();
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].name, "server1");
        assert_eq!(hosts[1].name, "server2");
    }
}
