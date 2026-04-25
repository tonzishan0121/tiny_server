use chat_app::state::ChatState;

use crate::chat_app;
use crate::database;

#[derive(Clone)]
pub struct AppState {
    pub chat: ChatState,
}

impl AppState {
    pub fn new(database_config_path: &str) -> Result<Self, String> {
        let database = database::connect_from_config(database_config_path)?;
        Ok(Self {
            chat: ChatState::from_database(database)?,
        })
    }
}
