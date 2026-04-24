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

/// Summary data used by the room list API.
#[derive(Clone)]
pub struct ChatRoomSummary {
    pub room: ChatRoom,
    pub message_count: usize,
    pub last_message: Option<ChatMessage>,
}
