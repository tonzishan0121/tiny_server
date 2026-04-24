use chat_app::state::ChatState;

use crate::chat_app;

#[derive(Clone)]
pub struct AppState {
    pub chat: ChatState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            chat: ChatState::new(),
        }
    }
}
