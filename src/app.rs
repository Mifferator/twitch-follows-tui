use crate::models::Channel;
use ratatui::widgets::TableState;

pub enum Status {
    Idle,
    LoadingFollows,
    LoadingDetails,
    LoadingDates,
    LoadingMutuals,
    Loaded(Vec<Channel>),
    Error(anyhow::Error),
}

pub enum Page {
    EnterName,
    ListView,
}

pub struct App {
    pub page: Page,
    pub status: Status,
    pub input: String,
    pub table_state: TableState,
}

impl App {
    pub fn new() -> Self {
        Self {
            page: Page::EnterName,
            status: Status::Idle,
            input: String::new(),
            table_state: TableState::default(),
        }
    }

    pub fn submit(&mut self) {
        self.status = Status::LoadingFollows;
        self.page = Page::ListView;
    }

    pub fn set_channels(&mut self, channels: Vec<Channel>) {
        self.table_state.select(Some(0));
        self.status = Status::Loaded(channels);
    }

    pub fn next(&mut self) {
        self.table_state.select_next();
    }

    pub fn previous(&mut self) {
        self.table_state.select_previous();
    }
}
