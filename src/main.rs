mod action;
mod app;
mod error;
mod event;
mod ssh;
mod state;
mod tui;
mod ui;

use clap::Parser;
use crossterm::event::{KeyCode, KeyModifiers};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

use action::Action;
use app::{App, Panel};
use event::{Event, EventHandler};

#[derive(Parser)]
#[command(name = "stm", about = "SSH Tunnel Manager", version)]
struct Cli {
    /// Path to SSH config file
    #[arg(long)]
    ssh_config: Option<PathBuf>,

    /// Auto-connect to a host on startup
    #[arg(long)]
    connect: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tui::install_panic_hook();
    let _ = state::persistence::ensure_config_dir();

    let mut terminal = tui::init()?;
    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();
    let mut app = App::new(action_tx);
    let mut events = EventHandler::new(Duration::from_millis(250));

    // Load SSH hosts from config path (CLI override or config file setting)
    let ssh_config_path = cli
        .ssh_config
        .unwrap_or_else(|| app.config.general.ssh_config_path.clone());
    if ssh_config_path.exists() {
        app.load_hosts(&ssh_config_path);
    }

    // Sort hosts: recently used first
    app.sort_hosts_by_history();

    // Auto-connect if requested
    if let Some(ref host_name) = cli.connect {
        if let Some(idx) = app.hosts.iter().position(|h| h.name == *host_name) {
            let _ = app.action_tx.send(Action::Connect(idx));
        }
    }

    // Initial render
    terminal.draw(|frame| ui::render(frame, &mut app))?;

    loop {
        if !app.running {
            break;
        }

        tokio::select! {
            Some(event) = events.next() => {
                let action = match event {
                    Event::Tick => Some(Action::Tick),
                    Event::Resize => Some(Action::Render),
                    Event::Key(key) => map_key_to_action(&app, key.modifiers, key.code),
                };

                if let Some(action) = action {
                    app.update(action);
                    terminal.draw(|frame| ui::render(frame, &mut app))?;
                }
            }
            Some(action) = action_rx.recv() => {
                app.update(action);
                terminal.draw(|frame| ui::render(frame, &mut app))?;
            }
        }
    }

    // Graceful cleanup: save tunnels and disconnect
    if let Some(ref conn) = app.connection {
        let name = conn.host().name.clone();
        app.history.save_tunnels(&name, &app.tunnels);
        let _ = app.history.save();
    }
    if let Some(mut conn) = app.connection.take() {
        let _ = conn.disconnect().await;
    }

    tui::restore()?;
    Ok(())
}

fn map_key_to_action(app: &App, modifiers: KeyModifiers, code: KeyCode) -> Option<Action> {
    if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
        return Some(Action::Quit);
    }

    if app.add_modal.is_some() {
        return match code {
            KeyCode::Esc => Some(Action::Quit),
            KeyCode::Enter => Some(Action::ModalSubmit),
            KeyCode::Tab => Some(Action::ModalNextField),
            KeyCode::Backspace => Some(Action::ModalBackspace),
            KeyCode::Char(c) => Some(Action::ModalInput(c)),
            _ => None,
        };
    }

    if app.search_mode {
        return match code {
            KeyCode::Esc => Some(Action::Quit),
            KeyCode::Enter => Some(Action::EndSearch),
            KeyCode::Backspace => Some(Action::SearchBackspace),
            KeyCode::Char(c) => Some(Action::SearchInput(c)),
            _ => None,
        };
    }

    if app.show_help {
        return match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => Some(Action::ShowHelp),
            _ => None,
        };
    }

    match code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::NavigateUp),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::NavigateDown),
        KeyCode::Enter => Some(Action::Select),
        KeyCode::Tab | KeyCode::BackTab => Some(Action::SwitchPanel),
        KeyCode::Char('/') => Some(Action::StartSearch),
        KeyCode::Char('?') => Some(Action::ShowHelp),
        KeyCode::Char('x') => Some(Action::Disconnect),
        KeyCode::Char('a') => Some(Action::ShowAddTunnelModal),
        KeyCode::Char('r') => Some(Action::RestoreTunnels),
        KeyCode::Char(' ') => {
            if app.active_panel == Panel::Tunnels {
                app.tunnel_list_state.selected().map(Action::ToggleTunnel)
            } else {
                None
            }
        }
        KeyCode::Char('d') => {
            if app.active_panel == Panel::Tunnels {
                app.tunnel_list_state.selected().map(Action::DeleteTunnel)
            } else {
                None
            }
        }
        _ => None,
    }
}
