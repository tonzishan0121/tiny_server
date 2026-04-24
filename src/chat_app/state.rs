use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ChatState {
    inner: Arc<Mutex<ChatStore>>,
}

#[derive(Clone)]
pub struct ChatMessage {
    pub id: usize,
    pub user: String,
    pub text: String,
}

struct ChatStore {
    next_id: usize,
    messages: Vec<ChatMessage>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ChatStore {
                next_id: 1,
                messages: vec![ChatMessage {
                    id: 0,
                    user: "system".to_string(),
                    text: "Welcome to tiny_server chat".to_string(),
                }],
            })),
        }
    }

    pub fn list_messages(&self) -> Vec<ChatMessage> {
        self.inner
            .lock()
            .expect("chat state lock should work")
            .messages
            .clone()
    }

    pub fn add_message(&self, user: String, text: String) -> ChatMessage {
        let mut store = self.inner.lock().expect("chat state lock should work");
        let message = ChatMessage {
            id: store.next_id,
            user,
            text,
        };
        store.next_id += 1;
        store.messages.push(message.clone());
        message
    }
}
