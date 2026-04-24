use std::fs;
use std::path::Path;

use crate::app_state::AppState;
use crate::chat_app::api::{
    CODE_BAD_REQUEST, CODE_INTERNAL_ERROR, CODE_MESSAGE_INVALID, CODE_ROOM_ALREADY_EXISTS,
    CODE_ROOM_INVALID, CODE_ROOM_NOT_FOUND, CODE_ROOM_PROTECTED, CODE_USER_INVALID, error_response,
    escape_json, success_response,
};
use crate::chat_app::models::{ChatMessage, ChatRoomSummary};
use crate::chat_app::state::DEFAULT_ROOM;
use crate::http::{HttpRequest, HttpResponse};

const MAX_USER_LEN: usize = 24;
const MAX_ROOM_LEN: usize = 24;
const MAX_MESSAGE_LEN: usize = 400;

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
    let mut output = String::new();
    let mut idx = 0;

    while idx < bytes.len() {
        match bytes[idx] {
            b'+' => {
                output.push(' ');
                idx += 1;
            }
            b'%' if idx + 2 < bytes.len() => {
                let hex = &value[idx + 1..idx + 3];
                if let Ok(decoded) = u8::from_str_radix(hex, 16) {
                    output.push(decoded as char);
                    idx += 3;
                } else {
                    output.push('%');
                    idx += 1;
                }
            }
            byte => {
                output.push(byte as char);
                idx += 1;
            }
        }
    }

    output
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

fn business_error_for_validation(message: &str) -> HttpResponse {
    let code = if message.starts_with("room ") || message == "room is required" {
        if message == "room is required"
            || message == "room is too long"
            || message == "room must use lowercase letters, numbers, '-' or '_'"
        {
            CODE_ROOM_INVALID
        } else {
            CODE_BAD_REQUEST
        }
    } else if message.starts_with("user ") {
        CODE_USER_INVALID
    } else if message.starts_with("message ") {
        CODE_MESSAGE_INVALID
    } else {
        CODE_BAD_REQUEST
    };

    error_response("400 Bad Request", code, message, None)
}

fn validate_room(room: &str) -> Result<String, &'static str> {
    let room = normalize_room(room);
    if room.is_empty() {
        return Err("room is required");
    }
    if room.len() > MAX_ROOM_LEN {
        return Err("room is too long");
    }
    if !room
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || matches!(ch, '-' | '_'))
    {
        return Err("room must use lowercase letters, numbers, '-' or '_'");
    }
    Ok(room)
}

fn validate_user(user: &str) -> Result<String, &'static str> {
    let user = user.trim();
    if user.len() < 2 {
        return Err("user must be at least 2 characters");
    }
    if user.len() > MAX_USER_LEN {
        return Err("user is too long");
    }
    if !user
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err("user must use letters, numbers, '-' or '_'");
    }
    Ok(user.to_string())
}

fn validate_message(text: &str) -> Result<String, &'static str> {
    let text = text.trim();
    if text.is_empty() {
        return Err("message is required");
    }
    if text.len() > MAX_MESSAGE_LEN {
        return Err("message is too long");
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
