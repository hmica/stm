use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::ssh::tunnel::Tunnel;
use crate::ui::theme;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    focused: bool,
    tunnels: &[Tunnel],
    list_state: &mut ListState,
) {
    let border_color = if focused {
        theme::BORDER_FOCUSED
    } else {
        theme::BORDER_UNFOCUSED
    };

    let title = format!(" Tunnels ({}) ", tunnels.len());

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    if tunnels.is_empty() {
        let text =
            Line::from("No tunnels. Press 'a' to add.").style(Style::default().fg(theme::TEXT_DIM));
        let paragraph = Paragraph::new(text).block(block).centered();
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = tunnels
        .iter()
        .map(|tunnel| {
            let status = if tunnel.enabled {
                Span::styled(
                    "[ON] ",
                    Style::default()
                        .fg(theme::CONNECTED)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("[OFF]", Style::default().fg(theme::TEXT_DIM))
            };

            let spec = format!(
                " L  {} → {}:{}",
                tunnel.local_port, tunnel.remote_host, tunnel.remote_port
            );

            ListItem::new(Line::from(vec![
                status,
                Span::styled(spec, Style::default().fg(theme::TEXT_PRIMARY)),
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

    frame.render_stateful_widget(list, area, list_state);
}
