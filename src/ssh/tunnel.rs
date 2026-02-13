use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tunnel {
    pub id: Uuid,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

impl Tunnel {
    pub fn new(local_port: u16, remote_host: String, remote_port: u16) -> Self {
        Self {
            id: Uuid::new_v4(),
            local_port,
            remote_host,
            remote_port,
            enabled: false,
            created_at: Utc::now(),
        }
    }

    /// Returns the forward spec string for SSH -L option.
    pub fn forward_spec(&self) -> String {
        format!(
            "{}:{}:{}",
            self.local_port, self.remote_host, self.remote_port
        )
    }
}

/// Check if a local port is available.
pub fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Add a tunnel via SSH ControlMaster.
pub async fn add_tunnel(
    socket_path: &Path,
    ssh_target: &str,
    tunnel: &Tunnel,
) -> anyhow::Result<()> {
    let socket = socket_path.to_string_lossy().to_string();
    let spec = tunnel.forward_spec();

    let output = Command::new("ssh")
        .args(["-S", &socket, "-O", "forward", "-L", &spec, ssh_target])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .await?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Failed to add tunnel: {}", stderr.trim()))
    }
}

/// Remove a tunnel via SSH ControlMaster.
pub async fn remove_tunnel(
    socket_path: &Path,
    ssh_target: &str,
    tunnel: &Tunnel,
) -> anyhow::Result<()> {
    let socket = socket_path.to_string_lossy().to_string();
    let spec = tunnel.forward_spec();

    let output = Command::new("ssh")
        .args(["-S", &socket, "-O", "cancel", "-L", &spec, ssh_target])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .await?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!(
            "Failed to remove tunnel: {}",
            stderr.trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_spec() {
        let tunnel = Tunnel::new(5432, "localhost".to_string(), 5432);
        assert_eq!(tunnel.forward_spec(), "5432:localhost:5432");
    }

    #[test]
    fn test_forward_spec_different_ports() {
        let tunnel = Tunnel::new(8080, "10.0.0.1".to_string(), 80);
        assert_eq!(tunnel.forward_spec(), "8080:10.0.0.1:80");
    }

    #[test]
    fn test_tunnel_new_defaults() {
        let tunnel = Tunnel::new(3000, "localhost".to_string(), 3000);
        assert!(!tunnel.enabled);
        assert_eq!(tunnel.remote_host, "localhost");
    }

    #[test]
    fn test_port_check() {
        // Port 0 asks OS for available port - should always work
        assert!(is_port_available(0));
    }
}
