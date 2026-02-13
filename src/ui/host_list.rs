use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, ConnectionStatus, Panel};
use crate::ui::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.active_panel == Panel::Hosts;
    let border_color = if focused {
        theme::BORDER_FOCUSED
    } else {
        theme::BORDER_UNFOCUSED
    };

    let title = if app.search_mode {
        format!(" Hosts [/{}] ", app.search_query)
    } else {
        format!(" Hosts ({}) ", app.filtered_host_indices.len())
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if app.filtered_host_indices.is_empty() {
        let msg = if app.hosts.is_empty() {
            "No SSH hosts found in ~/.ssh/config"
        } else {
            "No matching hosts"
        };
        let text = Line::from(msg).style(Style::default().fg(theme::TEXT_DIM));
        let paragraph = Paragraph::new(text).block(block).centered();
        frame.render_widget(paragraph, area);
        return;
    }

    let connected_name = app.connected_host_name().map(|s| s.to_string());

    let items: Vec<ListItem> = app
        .filtered_host_indices
        .iter()
        .map(|&idx| {
            let host = &app.hosts[idx];
            let is_connected = connected_name.as_deref() == Some(&host.name);
            let is_connecting = matches!(&app.connection_status, ConnectionStatus::Connecting)
                && app
                    .connection
                    .as_ref()
                    .is_some_and(|c| c.host().name == host.name);

            let (dot, dot_color) = if is_connected {
                ("● ", theme::CONNECTED)
            } else if is_connecting {
                ("◌ ", theme::HIGHLIGHT_FG)
            } else {
                ("○ ", theme::DISCONNECTED)
            };

            let name_span = Span::styled(
                &host.name,
                Style::default()
                    .fg(theme::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            );
            let detail = format!("  {}", host.display_target());
            let detail_span = Span::styled(detail, Style::default().fg(theme::TEXT_DIM));

            ListItem::new(Line::from(vec![
                Span::styled(dot, Style::default().fg(dot_color)),
                name_span,
                detail_span,
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme::HIGHLIGHT_BG)
                .fg(theme::HIGHLIGHT_FG),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.host_list_state);
}
