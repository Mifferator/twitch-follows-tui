use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use crate::app::{App, Status};

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}


fn draw_enter_name(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 20, frame.area());

    let input = Paragraph::new(app.input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Twitch username"));

    frame.render_widget(input, area);

    // place the cursor after the last typed character
    frame.set_cursor_position((
        area.x + app.input.len() as u16 + 1,
        area.y + 1,
    ));
}


fn draw_list_view(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());
    
    match &app.status {
        Status::Loading => frame.render_widget(Paragraph::new(format!("Loading follows for '{}'", app.input)), chunks[0]),
        Status::Loaded(channels) => {
            let items: Vec<ListItem> = channels.iter()
                .map(|c| ListItem::new(c.display_name.as_str()))
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(format!("{}'s Following", app.input)))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");

            frame.render_stateful_widget(list, chunks[0], &mut app.list_state);
        }
        Status::Error(e) => frame.render_widget(Paragraph::new(format!("Error: {e}")), chunks[0]),
    }
    frame.render_widget(Paragraph::new("↑↓: navigate | enter: open | c: search this user | q: quit"), chunks[1]);
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.page {
        crate::app::Page::EnterName => draw_enter_name(frame, app),
        crate::app::Page::ListView => draw_list_view(frame, app),
    }
}