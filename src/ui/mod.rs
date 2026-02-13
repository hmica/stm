pub mod add_modal;
pub mod host_list;
pub mod status_bar;
pub mod theme;
pub mod tunnel_list;

use ratatui::{
    layout::{Constraint, Layout},
    Frame,
};

use crate::app::{App, Panel};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Terminal too small check
    if area.width < 60 || area.height < 10 {
        use ratatui::{style::Style, text::Line, widgets::Paragraph};
        let msg = Paragraph::new(Line::from("Terminal too small (min 60x10)"))
            .style(Style::default().fg(theme::ERROR_COLOR))
            .centered();
        frame.render_widget(msg, area);
        return;
    }

    let [main_area, status_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);

    let [host_area, tunnel_area] =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
            .areas(main_area);

    host_list::render(frame, host_area, app);
    tunnel_list::render(
        frame,
        tunnel_area,
        app.active_panel == Panel::Tunnels,
        &app.tunnels,
        &mut app.tunnel_list_state,
    );
    status_bar::render(frame, status_area, app);

    // Overlays
    if let Some(ref modal) = app.add_modal {
        add_modal::render(frame, modal);
    } else if app.show_help {
        render_help_overlay(frame);
    }
}

fn render_help_overlay(frame: &mut Frame) {
    use ratatui::{
        layout::{Constraint, Flex, Layout},
        style::{Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph},
    };

    let area = frame.area();

    let [modal_area] = Layout::horizontal([Constraint::Percentage(60)])
        .flex(Flex::Center)
        .areas(area);
    let [modal_area] = Layout::vertical([Constraint::Percentage(70)])
        .flex(Flex::Center)
        .areas(modal_area);

    frame.render_widget(Clear, modal_area);

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(theme::TEXT_DIM);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  j/k, ↑/↓    ", bold),
            Span::styled("Navigate list", dim),
        ]),
        Line::from(vec![
            Span::styled("  Tab         ", bold),
            Span::styled("Switch panel (hosts/tunnels)", dim),
        ]),
        Line::from(vec![
            Span::styled("  Enter       ", bold),
            Span::styled("Connect to selected host", dim),
        ]),
        Line::from(vec![
            Span::styled("  x           ", bold),
            Span::styled("Disconnect from host", dim),
        ]),
        Line::from(vec![
            Span::styled("  /           ", bold),
            Span::styled("Search hosts", dim),
        ]),
        Line::from(vec![
            Span::styled("  a           ", bold),
            Span::styled("Add tunnel", dim),
        ]),
        Line::from(vec![
            Span::styled("  Space       ", bold),
            Span::styled("Toggle tunnel on/off", dim),
        ]),
        Line::from(vec![
            Span::styled("  d           ", bold),
            Span::styled("Delete tunnel", dim),
        ]),
        Line::from(vec![
            Span::styled("  ?           ", bold),
            Span::styled("Toggle this help", dim),
        ]),
        Line::from(vec![
            Span::styled("  q, Esc      ", bold),
            Span::styled("Quit", dim),
        ]),
        Line::from(""),
    ];

    let help = Paragraph::new(lines).block(
        Block::default()
            .title(" Keyboard Shortcuts ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER_FOCUSED)),
    );

    frame.render_widget(help, modal_area);
}
