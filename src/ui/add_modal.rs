use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalField {
    LocalPort,
    RemoteHost,
    RemotePort,
}

#[derive(Debug, Clone)]
pub struct AddModalState {
    pub local_port: String,
    pub remote_host: String,
    pub remote_port: String,
    pub active_field: ModalField,
    pub error_message: Option<String>,
}

impl AddModalState {
    pub fn new() -> Self {
        Self {
            local_port: String::new(),
            remote_host: "localhost".to_string(),
            remote_port: String::new(),
            active_field: ModalField::LocalPort,
            error_message: None,
        }
    }

    pub fn next_field(&mut self) {
        self.active_field = match self.active_field {
            ModalField::LocalPort => ModalField::RemoteHost,
            ModalField::RemoteHost => ModalField::RemotePort,
            ModalField::RemotePort => ModalField::LocalPort,
        };
    }

    pub fn input(&mut self, c: char) {
        match self.active_field {
            ModalField::LocalPort => {
                if c.is_ascii_digit() {
                    self.local_port.push(c);
                }
            }
            ModalField::RemoteHost => {
                self.remote_host.push(c);
            }
            ModalField::RemotePort => {
                if c.is_ascii_digit() {
                    self.remote_port.push(c);
                }
            }
        }
        self.error_message = None;
    }

    pub fn backspace(&mut self) {
        match self.active_field {
            ModalField::LocalPort => {
                self.local_port.pop();
            }
            ModalField::RemoteHost => {
                self.remote_host.pop();
            }
            ModalField::RemotePort => {
                self.remote_port.pop();
            }
        }
        self.error_message = None;
    }

    pub fn validate(&mut self) -> Option<(u16, String, u16)> {
        let local_port: u16 = match self.local_port.parse() {
            Ok(p) if p > 0 => p,
            _ => {
                self.error_message = Some("Invalid local port".to_string());
                return None;
            }
        };

        if self.remote_host.is_empty() {
            self.error_message = Some("Remote host cannot be empty".to_string());
            return None;
        }

        let remote_port: u16 = match self.remote_port.parse() {
            Ok(p) if p > 0 => p,
            _ => {
                self.error_message = Some("Invalid remote port".to_string());
                return None;
            }
        };

        if !crate::ssh::tunnel::is_port_available(local_port) {
            self.error_message = Some(format!("Port {local_port} is already in use"));
            return None;
        }

        Some((local_port, self.remote_host.clone(), remote_port))
    }
}

pub fn render(frame: &mut Frame, state: &AddModalState) {
    let area = frame.area();

    let [modal_area] = Layout::horizontal([Constraint::Percentage(50)])
        .flex(Flex::Center)
        .areas(area);
    let [modal_area] = Layout::vertical([Constraint::Length(12)])
        .flex(Flex::Center)
        .areas(modal_area);

    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" Add Tunnel (-L) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER_FOCUSED));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let [_, field1, _, field2, _, field3, _, error_area, _] = Layout::vertical([
        Constraint::Length(1), // padding
        Constraint::Length(1), // local port
        Constraint::Length(1), // spacing
        Constraint::Length(1), // remote host
        Constraint::Length(1), // spacing
        Constraint::Length(1), // remote port
        Constraint::Length(1), // spacing
        Constraint::Length(1), // error message
        Constraint::Min(0),    // remaining
    ])
    .areas(inner);

    render_field(
        frame,
        field1,
        "Local Port:",
        &state.local_port,
        state.active_field == ModalField::LocalPort,
    );
    render_field(
        frame,
        field2,
        "Remote Host:",
        &state.remote_host,
        state.active_field == ModalField::RemoteHost,
    );
    render_field(
        frame,
        field3,
        "Remote Port:",
        &state.remote_port,
        state.active_field == ModalField::RemotePort,
    );

    if let Some(ref error) = state.error_message {
        let err_line =
            Line::from(Span::styled(error, Style::default().fg(theme::ERROR_COLOR))).centered();
        frame.render_widget(Paragraph::new(err_line), error_area);
    }
}

fn render_field(frame: &mut Frame, area: Rect, label: &str, value: &str, active: bool) {
    let label_style = Style::default().fg(theme::TEXT_DIM);
    let value_style = if active {
        Style::default()
            .fg(theme::HIGHLIGHT_FG)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::TEXT_PRIMARY)
    };

    let cursor = if active { "█" } else { "" };

    let line = Line::from(vec![
        Span::styled(format!(" {label:<14}"), label_style),
        Span::styled(value, value_style),
        Span::styled(cursor, Style::default().fg(theme::BORDER_FOCUSED)),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}
