use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_ROOM: &str = "lobby";
const MAX_STORED_MESSAGES: usize = 200;

/// Shared in-memory chat state for the current process.
#[derive(Clone)]
pub struct ChatState {
    inner: Arc<Mutex<ChatStore>>,
}

/// One chat message returned by the API and rendered by the page.
#[derive(Clone)]
pub struct ChatMessage {
    pub id: usize,
    pub room: String,
    pub user: String,
    pub text: String,
    pub created_at_secs: u64,
}

/// One chat room returned by the API.
#[derive(Clone)]
pub struct ChatRoom {
    pub name: String,
    pub created_at_secs: u64,
}

struct ChatStore {
    next_id: usize,
    rooms: Vec<ChatRoom>,
    messages: Vec<ChatMessage>,
}

impl ChatState {
    /// Creates the chat store with one welcome message.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ChatStore {
                next_id: 1,
                rooms: default_rooms(),
                messages: vec![ChatMessage {
                    id: 0,
                    room: DEFAULT_ROOM.to_string(),
                    user: "system".to_string(),
                    text: "Welcome to tiny_server chat".to_string(),
                    created_at_secs: now_secs(),
                }],
            })),
        }
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

    /// Returns the currently known rooms.
    pub fn list_rooms(&self) -> Vec<ChatRoom> {
        self.inner
            .lock()
            .expect("chat state lock should work")
            .rooms
            .clone()
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

        store.rooms.push(ChatRoom {
            name: room,
            created_at_secs: now_secs(),
        });
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
        message
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

#[cfg(test)]
mod tests {
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

        let rooms = state.list_rooms();
        assert!(rooms.iter().any(|room| room.name == "design"));
    }

    #[test]
    fn has_room_checks_existing_rooms() {
        let state = ChatState::new();

        assert!(state.has_room(DEFAULT_ROOM));
        assert!(!state.has_room("unknown"));
    }
}
