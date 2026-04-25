use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::chat_app::models::{ChatMessage, ChatRoom, ChatRoomSummary};
use crate::database::{Database, DatabaseConnection};

pub const DEFAULT_ROOM: &str = "lobby";
const MAX_STORED_MESSAGES: usize = 200;

/// Shared in-memory chat state for the current process.
#[derive(Clone)]
pub struct ChatState {
    inner: Arc<Mutex<ChatStore>>,
    database: Option<Arc<Mutex<Database>>>,
}

struct ChatStore {
    next_id: usize,
    rooms: Vec<ChatRoom>,
    messages: Vec<ChatMessage>,
}

impl ChatState {
    /// Creates the chat store with one welcome message.
    #[cfg(test)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(default_store())),
            database: None,
        }
    }

    /// Creates the chat store from a database and keeps future writes in sync.
    pub fn from_database(database: Database) -> Result<Self, String> {
        initialize_database(&database)?;
        let mut store = load_store_from_database(&database)?;
        if store.rooms.is_empty() {
            store = default_store();
            persist_store_seed(&database, &store)?;
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(store)),
            database: Some(Arc::new(Mutex::new(database))),
        })
    }

    /// Returns a snapshot of all stored chat messages.
    pub fn list_messages(&self, room: &str) -> Vec<ChatMessage> {
        self.inner
            .lock()
            .expect("chat state lock should work")
            .messages
            .iter()
            .filter(|message| message.room == room)
            .cloned()
            .collect()
    }

    /// Returns room list data with message counts and the latest message.
    pub fn list_room_summaries(&self) -> Vec<ChatRoomSummary> {
        let store = self.inner.lock().expect("chat state lock should work");

        store
            .rooms
            .iter()
            .map(|room| {
                let room_messages = store
                    .messages
                    .iter()
                    .filter(|message| message.room == room.name);
                let message_count = room_messages.clone().count();
                let last_message = room_messages.last().cloned();

                ChatRoomSummary {
                    room: room.clone(),
                    message_count,
                    last_message,
                }
            })
            .collect()
    }

    /// Returns whether the room already exists.
    pub fn has_room(&self, room: &str) -> bool {
        self.inner
            .lock()
            .expect("chat state lock should work")
            .rooms
            .iter()
            .any(|item| item.name == room)
    }

    /// Creates a room when it is not already present.
    pub fn create_room(&self, room: String) -> bool {
        let mut store = self.inner.lock().expect("chat state lock should work");
        if store.rooms.iter().any(|item| item.name == room) {
            return false;
        }

        let chat_room = ChatRoom {
            name: room,
            created_at_secs: now_secs(),
        };
        store.rooms.push(chat_room.clone());
        self.with_database(|database| persist_room(database, &chat_room));
        true
    }

    /// Renames a room and moves all its messages to the new name.
    pub fn rename_room(&self, old_room: &str, new_room: String) -> bool {
        let mut store = self.inner.lock().expect("chat state lock should work");
        if store.rooms.iter().any(|item| item.name == new_room) {
            return false;
        }

        let Some(room) = store.rooms.iter_mut().find(|item| item.name == old_room) else {
            return false;
        };
        room.name = new_room.clone();

        for message in &mut store.messages {
            if message.room == old_room {
                message.room = new_room.clone();
            }
        }

        self.with_database(|database| persist_room_rename(database, old_room, &new_room));
        true
    }

    /// Deletes a room and drops all messages in it.
    pub fn delete_room(&self, room: &str) -> bool {
        let mut store = self.inner.lock().expect("chat state lock should work");
        let Some(room_index) = store.rooms.iter().position(|item| item.name == room) else {
            return false;
        };

        store.rooms.remove(room_index);
        store.messages.retain(|message| message.room != room);
        self.with_database(|database| persist_room_delete(database, room));
        true
    }

    /// Appends one message to the in-memory store.
    pub fn add_message(&self, room: String, user: String, text: String) -> ChatMessage {
        let mut store = self.inner.lock().expect("chat state lock should work");
        let message = ChatMessage {
            id: store.next_id,
            room,
            user,
            text,
            created_at_secs: now_secs(),
        };
        store.next_id += 1;
        store.messages.push(message.clone());
        drop_old_messages(&mut store.messages);
        self.with_database(|database| persist_message(database, &message));
        self.with_database(prune_persisted_messages);
        message
    }

    fn with_database(&self, action: impl FnOnce(&Database) -> Result<(), String>) {
        let Some(database) = &self.database else {
            return;
        };
        let database = database.lock().expect("database lock should work");
        if let Err(err) = action(&database) {
            eprintln!("chat database error: {err}");
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_secs()
}

fn drop_old_messages(messages: &mut Vec<ChatMessage>) {
    if messages.len() <= MAX_STORED_MESSAGES {
        return;
    }

    let excess = messages.len() - MAX_STORED_MESSAGES;
    messages.drain(0..excess);
}

fn default_store() -> ChatStore {
    ChatStore {
        next_id: 1,
        rooms: default_rooms(),
        messages: vec![ChatMessage {
            id: 0,
            room: DEFAULT_ROOM.to_string(),
            user: "system".to_string(),
            text: "Welcome to tiny_server chat".to_string(),
            created_at_secs: now_secs(),
        }],
    }
}

fn default_rooms() -> Vec<ChatRoom> {
    let created_at_secs = now_secs();
    vec![
        ChatRoom {
            name: DEFAULT_ROOM.to_string(),
            created_at_secs,
        },
        ChatRoom {
            name: "rust".to_string(),
            created_at_secs,
        },
        ChatRoom {
            name: "music".to_string(),
            created_at_secs,
        },
        ChatRoom {
            name: "gaming".to_string(),
            created_at_secs,
        },
    ]
}

fn initialize_database(database: &Database) -> Result<(), String> {
    database.execute(
        "CREATE TABLE IF NOT EXISTS chat_rooms (
            name TEXT PRIMARY KEY,
            created_at_secs INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY,
            room TEXT NOT NULL,
            user TEXT NOT NULL,
            text TEXT NOT NULL,
            created_at_secs INTEGER NOT NULL,
            FOREIGN KEY (room) REFERENCES chat_rooms(name)
                ON UPDATE CASCADE
                ON DELETE CASCADE
        );",
    )
}

fn load_store_from_database(database: &Database) -> Result<ChatStore, String> {
    let rooms = database
        .query("SELECT name, created_at_secs FROM chat_rooms ORDER BY created_at_secs, name;")?
        .into_iter()
        .filter_map(|row| {
            Some(ChatRoom {
                name: row.first()?.to_string(),
                created_at_secs: row.get(1)?.parse().ok()?,
            })
        })
        .collect::<Vec<_>>();
    let messages = database
        .query("SELECT id, room, user, text, created_at_secs FROM chat_messages ORDER BY id;")?
        .into_iter()
        .filter_map(|row| {
            Some(ChatMessage {
                id: row.first()?.parse().ok()?,
                room: row.get(1)?.to_string(),
                user: row.get(2)?.to_string(),
                text: row.get(3)?.to_string(),
                created_at_secs: row.get(4)?.parse().ok()?,
            })
        })
        .collect::<Vec<_>>();
    let next_id = messages
        .iter()
        .map(|message| message.id)
        .max()
        .map(|id| id + 1)
        .unwrap_or(1);

    Ok(ChatStore {
        next_id,
        rooms,
        messages,
    })
}

fn persist_store_seed(database: &Database, store: &ChatStore) -> Result<(), String> {
    for room in &store.rooms {
        persist_room(database, room)?;
    }
    for message in &store.messages {
        persist_message(database, message)?;
    }
    Ok(())
}

fn persist_room(database: &Database, room: &ChatRoom) -> Result<(), String> {
    database.execute_with_params(
        &format!(
            "INSERT OR IGNORE INTO chat_rooms (name, created_at_secs) VALUES (?, {});",
            room.created_at_secs
        ),
        &[&room.name],
    )
}

fn persist_room_rename(database: &Database, old_room: &str, new_room: &str) -> Result<(), String> {
    database.execute_with_params(
        "UPDATE chat_rooms SET name = ? WHERE name = ?;",
        &[new_room, old_room],
    )
}

fn persist_room_delete(database: &Database, room: &str) -> Result<(), String> {
    database.execute_with_params("DELETE FROM chat_rooms WHERE name = ?;", &[room])
}

fn persist_message(database: &Database, message: &ChatMessage) -> Result<(), String> {
    database.execute_with_params(
        &format!(
            "INSERT OR REPLACE INTO chat_messages (id, room, user, text, created_at_secs) \
             VALUES ({}, ?, ?, ?, {});",
            message.id, message.created_at_secs
        ),
        &[&message.room, &message.user, &message.text],
    )
}

fn prune_persisted_messages(database: &Database) -> Result<(), String> {
    database.execute(&format!(
        "DELETE FROM chat_messages \
         WHERE id NOT IN ( \
             SELECT id FROM chat_messages ORDER BY id DESC LIMIT {} \
         );",
        MAX_STORED_MESSAGES
    ))
}

#[cfg(test)]
mod tests {
    use crate::database::{DatabaseConfig, connect};

    use super::{ChatState, DEFAULT_ROOM, MAX_STORED_MESSAGES};

    #[test]
    fn list_messages_filters_by_room() {
        let state = ChatState::new();
        state.add_message("rust".to_string(), "alice".to_string(), "hello".to_string());
        state.add_message("music".to_string(), "bob".to_string(), "hi".to_string());

        let rust_messages = state.list_messages("rust");
        let music_messages = state.list_messages("music");
        let lobby_messages = state.list_messages(DEFAULT_ROOM);

        assert_eq!(rust_messages.len(), 1);
        assert_eq!(rust_messages[0].room, "rust");
        assert_eq!(music_messages.len(), 1);
        assert_eq!(music_messages[0].room, "music");
        assert_eq!(lobby_messages.len(), 1);
        assert_eq!(lobby_messages[0].user, "system");
    }

    #[test]
    fn message_store_keeps_recent_messages_only() {
        let state = ChatState::new();

        for index in 0..(MAX_STORED_MESSAGES + 10) {
            state.add_message(
                "load".to_string(),
                "robot".to_string(),
                format!("message {index}"),
            );
        }

        let messages = state.list_messages("load");
        assert_eq!(messages.len(), MAX_STORED_MESSAGES);
        assert_eq!(
            messages.first().expect("first message should exist").text,
            "message 10"
        );
        assert_eq!(
            messages.last().expect("last message should exist").text,
            format!("message {}", MAX_STORED_MESSAGES + 9)
        );
    }

    #[test]
    fn create_room_adds_new_room_once() {
        let state = ChatState::new();

        assert!(state.create_room("design".to_string()));
        assert!(!state.create_room("design".to_string()));

        let rooms = state.list_room_summaries();
        assert!(rooms.iter().any(|summary| summary.room.name == "design"));
    }

    #[test]
    fn has_room_checks_existing_rooms() {
        let state = ChatState::new();

        assert!(state.has_room(DEFAULT_ROOM));
        assert!(!state.has_room("unknown"));
    }

    #[test]
    fn database_backed_state_persists_rooms_and_messages() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tiny_server_chat_state_{unique}.db"));
        let config = DatabaseConfig {
            driver: "sqlite".to_string(),
            path: path.to_string_lossy().into_owned(),
        };

        {
            let database = connect(config.clone()).expect("should connect database");
            let state = ChatState::from_database(database).expect("should create database state");

            assert!(state.create_room("design".to_string()));
            state.add_message(
                "design".to_string(),
                "alice".to_string(),
                "hello db".to_string(),
            );
        }

        {
            let database = connect(config).expect("should reconnect database");
            let state = ChatState::from_database(database).expect("should reload database state");
            let messages = state.list_messages("design");

            assert!(state.has_room("design"));
            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].user, "alice");
            assert_eq!(messages[0].text, "hello db");
        }

        std::fs::remove_file(path).expect("should remove temp db");
    }

    #[test]
    fn database_foreign_keys_cascade_room_changes() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tiny_server_chat_state_cascade_{unique}.db"));
        let config = DatabaseConfig {
            driver: "sqlite".to_string(),
            path: path.to_string_lossy().into_owned(),
        };

        {
            let database = connect(config.clone()).expect("should connect database");
            let state = ChatState::from_database(database).expect("should create database state");

            assert!(state.create_room("design".to_string()));
            state.add_message(
                "design".to_string(),
                "alice".to_string(),
                "hello cascade".to_string(),
            );
            assert!(state.rename_room("design", "ideas".to_string()));
        }

        {
            let database = connect(config.clone()).expect("should reconnect database");
            let state = ChatState::from_database(database).expect("should reload database state");
            let messages = state.list_messages("ideas");

            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].room, "ideas");
            assert!(state.delete_room("ideas"));
        }

        {
            let database = connect(config).expect("should reconnect database");
            let state = ChatState::from_database(database).expect("should reload database state");

            assert!(!state.has_room("ideas"));
            assert!(state.list_messages("ideas").is_empty());
        }

        std::fs::remove_file(path).expect("should remove temp db");
    }
}
