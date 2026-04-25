use std::fs;
use std::path::Path;

use crate::app_state::AppState;
use crate::chat_app::api::{
    CODE_INTERNAL_ERROR, CODE_MESSAGE_INVALID, CODE_ROOM_ALREADY_EXISTS,
    CODE_ROOM_INVALID, CODE_ROOM_NOT_FOUND, CODE_ROOM_PROTECTED, CODE_USER_INVALID, error_response,
    escape_json, success_response,
};
use crate::chat_app::models::{ChatMessage, ChatRoomSummary};
use crate::chat_app::state::DEFAULT_ROOM;
use crate::http::{HttpRequest, HttpResponse};

const MAX_USER_LEN: usize = 24;
const MAX_ROOM_LEN: usize = 24;
const MAX_MESSAGE_LEN: usize = 400;

enum ValidationError {
    Room(&'static str),
    User(&'static str),
    Message(&'static str),
}

impl ValidationError {
    fn api_code(&self) -> i32 {
        match self {
            ValidationError::Room(_) => CODE_ROOM_INVALID,
            ValidationError::User(_) => CODE_USER_INVALID,
            ValidationError::Message(_) => CODE_MESSAGE_INVALID,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            ValidationError::Room(msg)
            | ValidationError::User(msg)
            | ValidationError::Message(msg) => msg,
        }
    }
}

pub fn chat_page(_: &HttpRequest, _: &AppState) -> HttpResponse {
    match fs::read(Path::new("static/chat/index.html")) {
        Ok(content) => HttpResponse::bytes("200 OK", "text/html; charset=utf-8", content)
            .with_header("Cache-Control", "no-store"),
        Err(_) => error_response(
            "500 Internal Server Error",
            CODE_INTERNAL_ERROR,
            "chat page missing",
            None,
        ),
    }
}

pub fn list_messages(request: &HttpRequest, state: &AppState) -> HttpResponse {
    let room = query_param(&request.path, "room")
        .map(|value| normalize_room(&value))
        .unwrap_or_else(|| DEFAULT_ROOM.to_string());

    if !state.chat.has_room(&room) {
        return error_response(
            "404 Not Found",
            CODE_ROOM_NOT_FOUND,
            "room does not exist",
            Some(format!("{{\"room\":\"{}\"}}", escape_json(&room))),
        );
    }

    let body = messages_to_json(&state.chat.list_messages(&room));
    success_response("200 OK", body)
}

pub fn create_message(request: &HttpRequest, state: &AppState) -> HttpResponse {
    let params = parse_form_body(&request.body);
    let room = params
        .get("room")
        .cloned()
        .unwrap_or_else(|| DEFAULT_ROOM.to_string());
    let user = params
        .get("user")
        .cloned()
        .unwrap_or_else(|| "guest".to_string());
    let text = params.get("message").cloned().unwrap_or_default();

    let room = match validate_room(&room) {
        Ok(room) => room,
        Err(message) => return business_error_for_validation(message),
    };
    let user = match validate_user(&user) {
        Ok(user) => user,
        Err(message) => return business_error_for_validation(message),
    };
    let text = match validate_message(&text) {
        Ok(text) => text,
        Err(message) => return business_error_for_validation(message),
    };

    if !state.chat.has_room(&room) {
        return error_response(
            "404 Not Found",
            CODE_ROOM_NOT_FOUND,
            "room does not exist",
            Some(format!("{{\"room\":\"{}\"}}", escape_json(&room))),
        );
    }

    let message = state.chat.add_message(room, user, text);
    success_response("201 Created", message_to_json(&message))
}

pub fn list_rooms(_: &HttpRequest, state: &AppState) -> HttpResponse {
    let body = rooms_to_json(&state.chat.list_room_summaries());
    success_response("200 OK", body)
}

pub fn create_room(request: &HttpRequest, state: &AppState) -> HttpResponse {
    let params = parse_form_body(&request.body);
    let room = params
        .get("room")
        .cloned()
        .unwrap_or_else(|| DEFAULT_ROOM.to_string());

    let room = match validate_room(&room) {
        Ok(room) => room,
        Err(message) => return business_error_for_validation(message),
    };

    let created = state.chat.create_room(room.clone());
    if !created {
        return error_response(
            "409 Conflict",
            CODE_ROOM_ALREADY_EXISTS,
            "room already exists",
            Some(format!("{{\"room\":\"{}\"}}", escape_json(&room))),
        );
    }

    let room = state
        .chat
        .list_room_summaries()
        .into_iter()
        .find(|item| item.room.name == room)
        .expect("room should exist after creation");

    success_response("201 Created", room_to_json(&room))
}

pub fn rename_room(request: &HttpRequest, state: &AppState) -> HttpResponse {
    let params = parse_form_body(&request.body);
    let old_room = params.get("room").cloned().unwrap_or_default();
    let new_room = params.get("new_room").cloned().unwrap_or_default();

    let old_room = match validate_room(&old_room) {
        Ok(room) => room,
        Err(message) => return business_error_for_validation(message),
    };
    let new_room = match validate_room(&new_room) {
        Ok(room) => room,
        Err(message) => return business_error_for_validation(message),
    };

    if old_room == DEFAULT_ROOM {
        return room_protected_response(&old_room);
    }
    if !state.chat.has_room(&old_room) {
        return room_not_found_response(&old_room);
    }
    if old_room != new_room && state.chat.has_room(&new_room) {
        return room_already_exists_response(&new_room);
    }
    if old_room != new_room {
        state.chat.rename_room(&old_room, new_room.clone());
    }

    let room = find_room_summary(state, &new_room).expect("room should exist after update");
    success_response("200 OK", room_to_json(&room))
}

pub fn delete_room(request: &HttpRequest, state: &AppState) -> HttpResponse {
    let params = parse_form_body(&request.body);
    let room = params.get("room").cloned().unwrap_or_default();

    let room = match validate_room(&room) {
        Ok(room) => room,
        Err(message) => return business_error_for_validation(message),
    };

    if room == DEFAULT_ROOM {
        return room_protected_response(&room);
    }
    if !state.chat.has_room(&room) {
        return room_not_found_response(&room);
    }

    state.chat.delete_room(&room);
    success_response(
        "200 OK",
        format!("{{\"name\":\"{}\",\"deleted\":true}}", escape_json(&room)),
    )
}

pub fn api_health(_: &HttpRequest, _: &AppState) -> HttpResponse {
    success_response(
        "200 OK",
        "{\"status\":\"ok\",\"service\":\"tiny_server_api\"}".to_string(),
    )
}

pub fn api_meta(_: &HttpRequest, _: &AppState) -> HttpResponse {
    success_response(
        "200 OK",
        format!(
            "{{\"service\":\"tiny_server_api\",\"version\":\"1\",\"default_room\":\"{}\",\"limits\":{{\"max_user_len\":{},\"max_room_len\":{},\"max_message_len\":{}}}}}",
            DEFAULT_ROOM, MAX_USER_LEN, MAX_ROOM_LEN, MAX_MESSAGE_LEN
        ),
    )
}

fn parse_form_body(body: &str) -> std::collections::HashMap<String, String> {
    let mut values = std::collections::HashMap::new();

    for pair in body.split('&') {
        if pair.is_empty() {
            continue;
        }

        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        values.insert(url_decode(name), url_decode(value));
    }

    values
}

fn url_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded: Vec<u8> = Vec::new();
    let mut idx = 0;

    while idx < bytes.len() {
        match bytes[idx] {
            b'+' => {
                decoded.push(b' ');
                idx += 1;
            }
            b'%' if idx + 2 < bytes.len() => {
                let hex = &value[idx + 1..idx + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    decoded.push(byte);
                    idx += 3;
                } else {
                    decoded.push(b'%');
                    idx += 1;
                }
            }
            byte => {
                decoded.push(byte);
                idx += 1;
            }
        }
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn messages_to_json(messages: &[ChatMessage]) -> String {
    let items = messages
        .iter()
        .map(message_to_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{items}]")
}

fn rooms_to_json(rooms: &[ChatRoomSummary]) -> String {
    let items = rooms.iter().map(room_to_json).collect::<Vec<_>>().join(",");
    format!("[{items}]")
}

fn message_to_json(message: &ChatMessage) -> String {
    format!(
        "{{\"id\":{},\"room\":\"{}\",\"user\":\"{}\",\"text\":\"{}\",\"created_at_secs\":{}}}",
        message.id,
        escape_json(&message.room),
        escape_json(&message.user),
        escape_json(&message.text),
        message.created_at_secs
    )
}

fn room_to_json(summary: &ChatRoomSummary) -> String {
    let last_message_json = summary
        .last_message
        .as_ref()
        .map(message_to_json)
        .unwrap_or_else(|| "null".to_string());

    format!(
        "{{\"name\":\"{}\",\"created_at_secs\":{},\"message_count\":{},\"last_message\":{}}}",
        escape_json(&summary.room.name),
        summary.room.created_at_secs,
        summary.message_count,
        last_message_json
    )
}

fn find_room_summary(state: &AppState, room: &str) -> Option<ChatRoomSummary> {
    state
        .chat
        .list_room_summaries()
        .into_iter()
        .find(|item| item.room.name == room)
}

fn room_not_found_response(room: &str) -> HttpResponse {
    error_response(
        "404 Not Found",
        CODE_ROOM_NOT_FOUND,
        "room does not exist",
        Some(format!("{{\"room\":\"{}\"}}", escape_json(room))),
    )
}

fn room_already_exists_response(room: &str) -> HttpResponse {
    error_response(
        "409 Conflict",
        CODE_ROOM_ALREADY_EXISTS,
        "room already exists",
        Some(format!("{{\"room\":\"{}\"}}", escape_json(room))),
    )
}

fn room_protected_response(room: &str) -> HttpResponse {
    error_response(
        "409 Conflict",
        CODE_ROOM_PROTECTED,
        "room is protected",
        Some(format!("{{\"room\":\"{}\"}}", escape_json(room))),
    )
}

fn business_error_for_validation(err: ValidationError) -> HttpResponse {
    error_response("400 Bad Request", err.api_code(), err.message(), None)
}

fn validate_room(room: &str) -> Result<String, ValidationError> {
    let room = normalize_room(room);
    if room.is_empty() {
        return Err(ValidationError::Room("room is required"));
    }
    if room.len() > MAX_ROOM_LEN {
        return Err(ValidationError::Room("room is too long"));
    }
    if !room
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_'))
    {
        return Err(ValidationError::Room(
            "room must use lowercase letters, numbers, '-' or '_'",
        ));
    }
    Ok(room)
}

fn validate_user(user: &str) -> Result<String, ValidationError> {
    let user = user.trim();
    if user.len() < 2 {
        return Err(ValidationError::User("user must be at least 2 characters"));
    }
    if user.len() > MAX_USER_LEN {
        return Err(ValidationError::User("user is too long"));
    }
    if !user
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err(ValidationError::User(
            "user must use letters, numbers, '-' or '_'",
        ));
    }
    Ok(user.to_string())
}

fn validate_message(text: &str) -> Result<String, ValidationError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(ValidationError::Message("message is required"));
    }
    if text.chars().count() > MAX_MESSAGE_LEN {
        return Err(ValidationError::Message("message is too long"));
    }
    Ok(text.to_string())
}

fn normalize_room(room: &str) -> String {
    room.trim().to_ascii_lowercase()
}

fn query_param(path: &str, name: &str) -> Option<String> {
    let (_, query) = path.split_once('?')?;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key == name {
            return Some(url_decode(value));
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::{url_decode, validate_message, MAX_MESSAGE_LEN};

    #[test]
    fn url_decode_handles_plus_as_space() {
        assert_eq!(url_decode("hello+world"), "hello world");
    }

    #[test]
    fn url_decode_handles_percent_encoded_ascii() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("%21"), "!");
    }

    #[test]
    fn url_decode_handles_multibyte_utf8() {
        // %C3%A9 is UTF-8 for 'é' (U+00E9).
        assert_eq!(url_decode("%C3%A9"), "é");
        // %F0%9F%98%80 is UTF-8 for 😀 (U+1F600).
        assert_eq!(url_decode("%F0%9F%98%80"), "😀");
    }

    #[test]
    fn url_decode_passes_through_invalid_percent_sequences() {
        assert_eq!(url_decode("%ZZ"), "%ZZ");
        assert_eq!(url_decode("%"), "%");
        assert_eq!(url_decode("%2"), "%2");
    }

    #[test]
    fn validate_message_counts_characters_not_bytes() {
        // Each emoji is 4 bytes; 400 of them exceed 400 bytes but equal MAX_MESSAGE_LEN chars.
        let emoji_message = "😀".repeat(MAX_MESSAGE_LEN);
        assert!(validate_message(&emoji_message).is_ok());

        // MAX_MESSAGE_LEN - 1 characters should be accepted.
        let under_limit = "😀".repeat(MAX_MESSAGE_LEN - 1);
        assert!(validate_message(&under_limit).is_ok());

        let too_long = "😀".repeat(MAX_MESSAGE_LEN + 1);
        assert!(validate_message(&too_long).is_err());
    }
}
