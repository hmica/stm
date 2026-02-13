use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, ConnectionStatus, NotificationLevel};
use crate::ui::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [status_area, hints_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area);

    // Left: notification or connection status
    let status_line = if let Some(ref notif) = app.notification {
        let color = match notif.level {
            NotificationLevel::Success => theme::SUCCESS,
            NotificationLevel::Error => theme::ERROR_COLOR,
            NotificationLevel::Info => theme::INFO,
        };
        Line::from(Span::styled(
            format!(" {}", notif.message),
            Style::default().fg(color),
        ))
    } else {
        match &app.connection_status {
            ConnectionStatus::Disconnected => Line::from(Span::styled(
                " Disconnected",
                Style::default().fg(theme::TEXT_DIM),
            )),
            ConnectionStatus::Connecting => Line::from(Span::styled(
                " Connecting...",
                Style::default().fg(theme::HIGHLIGHT_FG),
            )),
            ConnectionStatus::Connected(name) => Line::from(vec![
                Span::styled(" Connected to ", Style::default().fg(theme::CONNECTED)),
                Span::styled(
                    name,
                    Style::default()
                        .fg(theme::CONNECTED)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            ConnectionStatus::Error(msg) => {
                let display_msg = if msg.len() > 45 {
                    format!(" Error: {}...", &msg[..42])
                } else {
                    format!(" Error: {msg}")
                };
                Line::from(Span::styled(
                    display_msg,
                    Style::default().fg(theme::ERROR_COLOR),
                ))
            }
        }
    };

    frame.render_widget(Paragraph::new(status_line), status_area);

    // Right: keyboard hints
    let bold = Style::default()
        .fg(theme::HIGHLIGHT_FG)
        .add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme::TEXT_DIM);

    let hints = if app.search_mode {
        vec![
            Span::styled("Esc", bold),
            Span::styled(" Cancel  ", dim),
            Span::styled("Enter", bold),
            Span::styled(" Confirm", dim),
        ]
    } else {
        vec![
            Span::styled("j/k", bold),
            Span::styled(" Nav  ", dim),
            Span::styled("Enter", bold),
            Span::styled(" Connect  ", dim),
            Span::styled("x", bold),
            Span::styled(" Disconnect  ", dim),
            Span::styled("/", bold),
            Span::styled(" Search  ", dim),
            Span::styled("?", bold),
            Span::styled(" Help", dim),
        ]
    };

    let bar = Paragraph::new(Line::from(hints)).right_aligned();
    frame.render_widget(bar, hints_area);
}
