use std::path::PathBuf;
use tokio::process::{Child, Command};

use crate::ssh::config::SshHost;

pub struct ConnectionManager {
    child: Option<Child>,
    socket_path: PathBuf,
    host: SshHost,
}

impl ConnectionManager {
    pub fn new(host: SshHost, socket_dir: &std::path::Path) -> Self {
        let socket_name = format!("{}-{}", host.effective_hostname(), host.effective_port());
        let socket_path = socket_dir.join(socket_name);

        Self {
            child: None,
            socket_path,
            host,
        }
    }

    pub fn host(&self) -> &SshHost {
        &self.host
    }

    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Build the SSH target string (e.g., "user@hostname" or just "hostname").
    fn ssh_target(&self) -> String {
        let hostname = self.host.effective_hostname();
        match &self.host.user {
            Some(user) => format!("{user}@{hostname}"),
            None => hostname.to_string(),
        }
    }

    /// Spawn a ControlMaster SSH connection.
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        // Ensure socket directory exists
        if let Some(parent) = self.socket_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let target = self.ssh_target();
        let socket = self.socket_path.to_string_lossy().to_string();

        let mut cmd = Command::new("ssh");
        cmd.args([
            "-M", // ControlMaster mode
            "-S",
            &socket, // Socket path
            "-N",    // No remote command
            "-o",
            "ControlPersist=yes", // Keep master alive
            "-o",
            "ServerAliveInterval=15", // Keepalive
            "-o",
            "ServerAliveCountMax=3", // Max missed keepalives
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "BatchMode=yes", // No interactive prompts
        ]);

        // Add port if non-default
        if let Some(port) = self.host.port {
            cmd.args(["-p", &port.to_string()]);
        }

        // Add identity file if specified
        if let Some(ref identity) = self.host.identity_file {
            cmd.args(["-i", &identity.to_string_lossy()]);
        }

        // Add proxy jump if specified
        if let Some(ref proxy) = self.host.proxy_jump {
            cmd.args(["-J", proxy]);
        }

        cmd.arg(&target);

        // Suppress stdin/stdout/stderr
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::piped());

        cmd.kill_on_drop(true);

        let child = cmd.spawn()?;
        self.child = Some(child);

        // Wait briefly for the connection to establish, then verify
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Check if connection was established
        match self.check().await {
            Ok(true) => Ok(()),
            Ok(false) => {
                // Try to get stderr output for error details
                let err_msg = self.collect_stderr().await;
                self.cleanup().await;
                Err(anyhow::anyhow!(
                    "Connection failed: {}",
                    err_msg.unwrap_or_else(|| "unknown error".to_string())
                ))
            }
            Err(e) => {
                self.cleanup().await;
                Err(e)
            }
        }
    }

    /// Check if the ControlMaster connection is alive.
    pub async fn check(&self) -> anyhow::Result<bool> {
        let socket = self.socket_path.to_string_lossy().to_string();
        let target = self.ssh_target();

        let output = Command::new("ssh")
            .args(["-S", &socket, "-O", "check", &target])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .await?;

        Ok(output.status.success())
    }

    /// Disconnect the ControlMaster connection.
    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        let socket = self.socket_path.to_string_lossy().to_string();
        let target = self.ssh_target();

        // Send exit signal to ControlMaster
        let _ = Command::new("ssh")
            .args(["-S", &socket, "-O", "exit", &target])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .await;

        self.cleanup().await;
        Ok(())
    }

    async fn cleanup(&mut self) {
        // Kill child process if still running
        if let Some(ref mut child) = self.child {
            let _ = child.kill().await;
        }
        self.child = None;

        // Remove socket file
        let _ = tokio::fs::remove_file(&self.socket_path).await;
    }

    async fn collect_stderr(&mut self) -> Option<String> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            if let Ok(output) = child.wait_with_output().await {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                if !stderr.is_empty() {
                    return Some(stderr.trim().to_string());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_target_with_user() {
        let host = SshHost {
            name: "myhost".to_string(),
            hostname: Some("10.0.0.1".to_string()),
            user: Some("admin".to_string()),
            ..Default::default()
        };
        let dir = PathBuf::from("/tmp/sockets");
        let mgr = ConnectionManager::new(host, &dir);
        assert_eq!(mgr.ssh_target(), "admin@10.0.0.1");
    }

    #[test]
    fn test_ssh_target_without_user() {
        let host = SshHost {
            name: "myhost".to_string(),
            hostname: Some("10.0.0.1".to_string()),
            ..Default::default()
        };
        let dir = PathBuf::from("/tmp/sockets");
        let mgr = ConnectionManager::new(host, &dir);
        assert_eq!(mgr.ssh_target(), "10.0.0.1");
    }

    #[test]
    fn test_socket_path() {
        let host = SshHost {
            name: "myhost".to_string(),
            hostname: Some("10.0.0.1".to_string()),
            port: Some(2222),
            ..Default::default()
        };
        let dir = PathBuf::from("/tmp/sockets");
        let mgr = ConnectionManager::new(host, &dir);
        assert_eq!(
            mgr.socket_path(),
            &PathBuf::from("/tmp/sockets/10.0.0.1-2222")
        );
    }

    #[test]
    fn test_socket_path_default_port() {
        let host = SshHost {
            name: "myhost".to_string(),
            hostname: Some("10.0.0.1".to_string()),
            ..Default::default()
        };
        let dir = PathBuf::from("/tmp/sockets");
        let mgr = ConnectionManager::new(host, &dir);
        assert_eq!(
            mgr.socket_path(),
            &PathBuf::from("/tmp/sockets/10.0.0.1-22")
        );
    }
}
