use ratatui::widgets::ListState;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use crate::action::Action;
use crate::ssh::config::SshHost;
use crate::ssh::connection::ConnectionManager;
use crate::ssh::tunnel::Tunnel;
use crate::state::history::History;
use crate::state::persistence::AppConfig;
use crate::ui::add_modal::AddModalState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Hosts,
    Tunnels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected(String),
    Error(String),
}

pub struct App {
    pub running: bool,
    pub hosts: Vec<SshHost>,
    pub host_list_state: ListState,
    pub active_panel: Panel,
    pub search_query: String,
    pub search_mode: bool,
    pub filtered_host_indices: Vec<usize>,
    pub show_help: bool,
    pub connection: Option<ConnectionManager>,
    pub connection_status: ConnectionStatus,
    pub action_tx: mpsc::UnboundedSender<Action>,
    pub socket_dir: PathBuf,
    pub tick_count: u32,

    // Tunnel state
    pub tunnels: Vec<Tunnel>,
    pub tunnel_list_state: ListState,
    pub add_modal: Option<AddModalState>,

    // Persistence
    pub config: AppConfig,
    pub history: History,

    // Notifications
    pub notification: Option<Notification>,
    pub notification_ticks: u32,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub level: NotificationLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum NotificationLevel {
    Success,
    Error,
    Info,
}

impl App {
    pub fn new(action_tx: mpsc::UnboundedSender<Action>) -> Self {
        let config = AppConfig::load();
        let history = History::load();
        let socket_dir = config.general.socket_dir.clone();

        Self {
            running: true,
            hosts: Vec::new(),
            host_list_state: ListState::default(),
            active_panel: Panel::Hosts,
            search_query: String::new(),
            search_mode: false,
            filtered_host_indices: Vec::new(),
            show_help: false,
            connection: None,
            connection_status: ConnectionStatus::Disconnected,
            action_tx,
            socket_dir,
            tick_count: 0,
            tunnels: Vec::new(),
            tunnel_list_state: ListState::default(),
            add_modal: None,
            config,
            history,
            notification: None,
            notification_ticks: 0,
        }
    }

    pub fn load_hosts(&mut self, ssh_config_path: &Path) {
        match crate::ssh::config::parse_ssh_config(ssh_config_path) {
            Ok(hosts) => {
                self.hosts = hosts;
                self.rebuild_filtered_indices();
                if !self.filtered_host_indices.is_empty() {
                    self.host_list_state.select(Some(0));
                }
            }
            Err(_) => {
                self.hosts = Vec::new();
                self.filtered_host_indices = Vec::new();
            }
        }
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Quit => {
                if self.add_modal.is_some() {
                    self.add_modal = None;
                } else if self.search_mode {
                    self.search_mode = false;
                    self.search_query.clear();
                    self.rebuild_filtered_indices();
                } else if self.show_help {
                    self.show_help = false;
                } else {
                    self.running = false;
                }
            }
            Action::Tick => {
                self.tick_count += 1;
                // Auto-dismiss notifications after ~4 seconds (16 ticks)
                if self.notification.is_some() {
                    self.notification_ticks += 1;
                    if self.notification_ticks >= 16 {
                        self.notification = None;
                    }
                }
                if self.tick_count.is_multiple_of(40) {
                    if let ConnectionStatus::Connected(_) = &self.connection_status {
                        let tx = self.action_tx.clone();
                        if let Some(ref conn) = self.connection {
                            let socket = conn.socket_path().clone();
                            let target = conn.host().display_target();
                            tokio::spawn(async move {
                                let check_result = tokio::process::Command::new("ssh")
                                    .args(["-S", &socket.to_string_lossy(), "-O", "check", &target])
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .output()
                                    .await;

                                match check_result {
                                    Ok(output) if !output.status.success() => {
                                        let _ = tx.send(Action::ConnectionFailed(
                                            "Connection lost".to_string(),
                                        ));
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Action::ConnectionFailed(e.to_string()));
                                    }
                                    _ => {}
                                }
                            });
                        }
                    }
                }
            }
            Action::Render => {}
            Action::NavigateUp => self.navigate(-1),
            Action::NavigateDown => self.navigate(1),
            Action::Select => {
                if self.active_panel == Panel::Hosts {
                    if let Some(selected) = self.host_list_state.selected() {
                        if let Some(&real_idx) = self.filtered_host_indices.get(selected) {
                            let _ = self.action_tx.send(Action::Connect(real_idx));
                        }
                    }
                }
            }
            Action::SwitchPanel => {
                self.active_panel = match self.active_panel {
                    Panel::Hosts => Panel::Tunnels,
                    Panel::Tunnels => Panel::Hosts,
                };
            }
            Action::StartSearch => {
                if self.active_panel == Panel::Hosts {
                    self.search_mode = true;
                    self.search_query.clear();
                }
            }
            Action::SearchInput(c) => {
                if self.search_mode {
                    self.search_query.push(c);
                    self.rebuild_filtered_indices();
                    if !self.filtered_host_indices.is_empty() {
                        self.host_list_state.select(Some(0));
                    } else {
                        self.host_list_state.select(None);
                    }
                }
            }
            Action::SearchBackspace => {
                if self.search_mode {
                    self.search_query.pop();
                    self.rebuild_filtered_indices();
                    if !self.filtered_host_indices.is_empty() {
                        self.host_list_state.select(Some(0));
                    } else {
                        self.host_list_state.select(None);
                    }
                }
            }
            Action::EndSearch => {
                self.search_mode = false;
            }
            Action::ShowHelp => {
                self.show_help = !self.show_help;
            }

            // Connection actions
            Action::Connect(idx) => {
                if let Some(host) = self.hosts.get(idx).cloned() {
                    if let Some(mut conn) = self.connection.take() {
                        tokio::spawn(async move {
                            let _ = conn.disconnect().await;
                        });
                    }

                    // Clear tunnels from previous connection
                    self.tunnels.clear();
                    self.tunnel_list_state.select(None);
                    self.connection_status = ConnectionStatus::Connecting;

                    let socket_dir = self.socket_dir.clone();
                    let tx = self.action_tx.clone();

                    tokio::spawn(async move {
                        let mut mgr = ConnectionManager::new(host, &socket_dir);
                        match mgr.connect().await {
                            Ok(()) => {
                                let _ = tx.send(Action::ConnectionEstablished);
                            }
                            Err(e) => {
                                let _ = tx.send(Action::ConnectionFailed(e.to_string()));
                            }
                        }
                        drop(mgr);
                    });

                    // Pre-create the manager in app state for socket path / host info access
                    if let Some(host) = self.hosts.get(idx).cloned() {
                        self.connection = Some(ConnectionManager::new(host, &self.socket_dir));
                    }
                }
            }
            Action::ConnectionEstablished => {
                if let Some(ref conn) = self.connection {
                    let name = conn.host().name.clone();
                    self.connection_status = ConnectionStatus::Connected(name.clone());
                    self.history.record_connection(&name);
                    let _ = self.history.save();

                    // Load previously saved tunnels (disabled by default)
                    let saved = self.history.get_saved_tunnels(&name);
                    for st in saved {
                        let tunnel = Tunnel::new(st.local_port, st.remote_host, st.remote_port);
                        self.tunnels.push(tunnel);
                    }
                    if !self.tunnels.is_empty() {
                        self.tunnel_list_state.select(Some(0));
                    }

                    self.notify(format!("Connected to {name}"), NotificationLevel::Success);
                }
            }
            Action::ConnectionFailed(msg) => {
                self.notify(
                    format!("Connection failed: {msg}"),
                    NotificationLevel::Error,
                );
                self.connection_status = ConnectionStatus::Error(msg);
                self.connection = None;
                self.tunnels.clear();
            }
            Action::Disconnect => {
                // Save tunnels before disconnecting
                if let Some(ref conn) = self.connection {
                    let name = conn.host().name.clone();
                    self.history.save_tunnels(&name, &self.tunnels);
                    let _ = self.history.save();
                }
                if let Some(mut conn) = self.connection.take() {
                    let tx = self.action_tx.clone();
                    tokio::spawn(async move {
                        let _ = conn.disconnect().await;
                        let _ = tx.send(Action::Disconnected);
                    });
                    self.connection_status = ConnectionStatus::Disconnected;
                    self.tunnels.clear();
                    self.tunnel_list_state.select(None);
                }
            }
            Action::Disconnected => {
                self.connection = None;
                self.connection_status = ConnectionStatus::Disconnected;
                self.tunnels.clear();
                self.tunnel_list_state.select(None);
            }

            // Modal actions
            Action::ShowAddTunnelModal => {
                if matches!(self.connection_status, ConnectionStatus::Connected(_)) {
                    self.add_modal = Some(AddModalState::new());
                } else {
                    self.notify("Connect to a host first (Enter)", NotificationLevel::Info);
                }
            }
            Action::ModalInput(c) => {
                if let Some(ref mut modal) = self.add_modal {
                    modal.input(c);
                }
            }
            Action::ModalBackspace => {
                if let Some(ref mut modal) = self.add_modal {
                    modal.backspace();
                }
            }
            Action::ModalNextField => {
                if let Some(ref mut modal) = self.add_modal {
                    modal.next_field();
                }
            }
            Action::ModalSubmit => {
                if let Some(ref mut modal) = self.add_modal {
                    if let Some((local_port, remote_host, remote_port)) = modal.validate() {
                        let tunnel = Tunnel::new(local_port, remote_host, remote_port);
                        self.tunnels.push(tunnel);
                        let tunnel_idx = self.tunnels.len() - 1;
                        self.add_modal = None;

                        // Auto-enable the tunnel
                        let _ = self.action_tx.send(Action::ToggleTunnel(tunnel_idx));

                        // Select the new tunnel
                        self.tunnel_list_state.select(Some(tunnel_idx));
                        self.active_panel = Panel::Tunnels;
                    }
                }
            }
            // Tunnel actions
            Action::TunnelFailed(msg) => {
                self.notify(format!("Tunnel error: {msg}"), NotificationLevel::Error);
            }
            Action::ToggleTunnel(idx) => {
                if let (Some(tunnel), Some(ref conn)) =
                    (self.tunnels.get(idx).cloned(), &self.connection)
                {
                    let socket_path = conn.socket_path().clone();
                    let ssh_target = conn.host().display_target();
                    let tx = self.action_tx.clone();
                    let tunnel_id = tunnel.id;
                    let currently_enabled = tunnel.enabled;

                    tokio::spawn(async move {
                        let result = if currently_enabled {
                            crate::ssh::tunnel::remove_tunnel(&socket_path, &ssh_target, &tunnel)
                                .await
                        } else {
                            crate::ssh::tunnel::add_tunnel(&socket_path, &ssh_target, &tunnel).await
                        };

                        match result {
                            Ok(()) => {
                                let _ =
                                    tx.send(Action::TunnelToggled(tunnel_id, !currently_enabled));
                            }
                            Err(e) => {
                                let _ = tx.send(Action::TunnelFailed(e.to_string()));
                            }
                        }
                    });
                }
            }
            Action::TunnelToggled(id, enabled) => {
                if let Some(tunnel) = self.tunnels.iter_mut().find(|t| t.id == id) {
                    tunnel.enabled = enabled;
                }
            }
            Action::DeleteTunnel(idx) => {
                if let Some(tunnel) = self.tunnels.get(idx).cloned() {
                    if tunnel.enabled {
                        // Cancel the tunnel first, then remove
                        if let Some(ref conn) = self.connection {
                            let socket_path = conn.socket_path().clone();
                            let ssh_target = conn.host().display_target();
                            let tx = self.action_tx.clone();
                            let tunnel_id = tunnel.id;

                            tokio::spawn(async move {
                                let _ = crate::ssh::tunnel::remove_tunnel(
                                    &socket_path,
                                    &ssh_target,
                                    &tunnel,
                                )
                                .await;
                                let _ = tx.send(Action::TunnelDeleted(tunnel_id));
                            });
                        }
                    } else {
                        self.tunnels.retain(|t| t.id != tunnel.id);
                        self.fix_tunnel_selection();
                    }
                }
            }
            Action::TunnelDeleted(id) => {
                self.tunnels.retain(|t| t.id != id);
                self.fix_tunnel_selection();
            }

            // Persistence
            Action::RestoreTunnels => {
                if let ConnectionStatus::Connected(ref name) = self.connection_status {
                    let saved = self.history.get_saved_tunnels(name);
                    for st in saved {
                        let tunnel = Tunnel::new(st.local_port, st.remote_host, st.remote_port);
                        self.tunnels.push(tunnel);
                        let idx = self.tunnels.len() - 1;
                        let _ = self.action_tx.send(Action::ToggleTunnel(idx));
                    }
                    if !self.tunnels.is_empty() {
                        self.tunnel_list_state.select(Some(0));
                        self.active_panel = Panel::Tunnels;
                    }
                }
            }
        }
    }

    fn navigate(&mut self, delta: i32) {
        match self.active_panel {
            Panel::Hosts => {
                let max = self.filtered_host_indices.len();
                if max == 0 {
                    return;
                }
                let current = self.host_list_state.selected().unwrap_or(0);
                let next = if delta > 0 {
                    (current + 1).min(max - 1)
                } else {
                    current.saturating_sub(1)
                };
                self.host_list_state.select(Some(next));
            }
            Panel::Tunnels => {
                let max = self.tunnels.len();
                if max == 0 {
                    return;
                }
                let current = self.tunnel_list_state.selected().unwrap_or(0);
                let next = if delta > 0 {
                    (current + 1).min(max - 1)
                } else {
                    current.saturating_sub(1)
                };
                self.tunnel_list_state.select(Some(next));
            }
        }
    }

    fn rebuild_filtered_indices(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_host_indices = (0..self.hosts.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_host_indices = self
                .hosts
                .iter()
                .enumerate()
                .filter(|(_, host)| {
                    host.name.to_lowercase().contains(&query)
                        || host
                            .hostname
                            .as_ref()
                            .is_some_and(|h| h.to_lowercase().contains(&query))
                })
                .map(|(i, _)| i)
                .collect();
        }
    }

    fn fix_tunnel_selection(&mut self) {
        if self.tunnels.is_empty() {
            self.tunnel_list_state.select(None);
        } else if let Some(selected) = self.tunnel_list_state.selected() {
            if selected >= self.tunnels.len() {
                self.tunnel_list_state.select(Some(self.tunnels.len() - 1));
            }
        }
    }

    #[allow(dead_code)]
    pub fn selected_host(&self) -> Option<&SshHost> {
        let selected = self.host_list_state.selected()?;
        let real_index = *self.filtered_host_indices.get(selected)?;
        self.hosts.get(real_index)
    }

    pub fn connected_host_name(&self) -> Option<&str> {
        match &self.connection_status {
            ConnectionStatus::Connected(name) => Some(name),
            _ => None,
        }
    }

    fn notify(&mut self, message: impl Into<String>, level: NotificationLevel) {
        self.notification = Some(Notification {
            message: message.into(),
            level,
        });
        self.notification_ticks = 0;
    }

    /// Sort hosts so recently used ones appear first.
    pub fn sort_hosts_by_history(&mut self) {
        let history = &self.history;
        self.hosts.sort_by(|a, b| {
            let a_history = history.hosts.get(&a.name);
            let b_history = history.hosts.get(&b.name);
            match (a_history, b_history) {
                (Some(ah), Some(bh)) => bh.last_used.cmp(&ah.last_used),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.name.cmp(&b.name),
            }
        });
        self.rebuild_filtered_indices();
        if !self.filtered_host_indices.is_empty() {
            self.host_list_state.select(Some(0));
        }
    }
}
