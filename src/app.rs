use crate::models::Channel;
use ratatui::widgets::ListState;

pub enum Status {
    Loading,
    Loaded(Vec<Channel>),
    Error(anyhow::Error),
}

pub enum Page {
    EnterName,
    ListView
}

pub struct App {
    pub page: Page,
    pub status: Status,
    pub input: String,
    pub list_state: ListState,
}

impl App {
    pub fn new() -> Self {
        Self {
            page: Page::EnterName,
            status: Status::Loading,
            input: String::new(),
            list_state: ListState::default(),
        }
    }

    pub fn submit(&mut self) {
        self.page = Page::ListView;
    }

    pub fn set_channels(&mut self, channels: Vec<Channel>) {
        self.list_state.select(Some(0));
        self.status = Status::Loaded(channels);
    }

    pub fn next(&mut self) {
        self.list_state.select_next();
    }

    pub fn previous(&mut self) {
        self.list_state.select_previous();
    }
}