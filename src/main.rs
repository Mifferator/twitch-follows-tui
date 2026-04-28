mod api;
mod app;
mod models;
mod ui;

use std::{sync::mpsc, time::Duration};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    let mut app = app::App::new();
    let (tx, rx) = mpsc::channel();
    let client = reqwest::Client::new();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(channels) => app.set_channels(channels),
                Err(e) => app.status = app::Status::Error(e),
            }
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.page {
                        app::Page::EnterName => match key.code {
                            KeyCode::Esc => break,
                            KeyCode::Backspace => { app.input.pop(); }
                            KeyCode::Enter => {
                                let username = app.input.clone();
                                let tx = tx.clone();
                                let client = client.clone();
                                tokio::spawn(async move {
                                    tx.send(api::fetch_follows(&client, &username).await).ok();
                                });
                                app.submit();
                            }
                            KeyCode::Char(c) => app.input.push(c),
                            _ => {}
                        },
                        app::Page::ListView => match key.code {
                            KeyCode::Esc => {
                                app.input.clear();
                                app.page = app::Page::EnterName;
                            }
                            KeyCode::Char('q') => break,
                            KeyCode::Down | KeyCode::Char('j') => app.next(),
                            KeyCode::Up | KeyCode::Char('k') => app.previous(),
                            KeyCode::Enter => {
                                if let app::Status::Loaded(channels) = &app.status {
                                    if let Some(i) = app.list_state.selected() {
                                        let url = format!("https://twitch.tv/{}", channels[i].login);
                                        open::that(url).ok();
                                    }
                                }
                            }
                            KeyCode::Char('c') => {
                                if let app::Status::Loaded(channels) = &app.status {
                                    if let Some(i) = app.list_state.selected() {
                                        let login = channels[i].login.clone();
                                        let next_input = login.clone();
                                        let tx = tx.clone();
                                        let client = client.clone();
                                        tokio::spawn(async move {
                                            tx.send(api::fetch_follows(&client, &login).await).ok();
                                        });
                                        app.input = next_input;
                                        app.submit();
                                    }
                                }
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    }

    Ok(())
}
