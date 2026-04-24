use std::fs;
use std::path::Path;

use crate::app_state::AppState;
use crate::chat_app::state::ChatMessage;
use crate::server_core::http::{HttpRequest, HttpResponse};

pub fn chat_page(_: &HttpRequest, _: &AppState) -> HttpResponse {
    match fs::read(Path::new("static/chat/index.html")) {
        Ok(content) => HttpResponse::bytes("200 OK", "text/html; charset=utf-8", content),
        Err(_) => HttpResponse::new(
            "500 Internal Server Error",
            "text/plain; charset=utf-8",
            "chat page missing",
        ),
    }
}

pub fn list_messages(_: &HttpRequest, state: &AppState) -> HttpResponse {
    let body = messages_to_json(&state.chat.list_messages());
    HttpResponse::new("200 OK", "application/json; charset=utf-8", body)
}

pub fn create_message(request: &HttpRequest, state: &AppState) -> HttpResponse {
    let params = parse_form_body(&request.body);
    let user = params
        .get("user")
        .cloned()
        .unwrap_or_else(|| "anonymous".to_string());
    let text = params.get("message").cloned().unwrap_or_default();

    if text.trim().is_empty() {
        return HttpResponse::new(
            "400 Bad Request",
            "text/plain; charset=utf-8",
            "message is required",
        );
    }

    let message = state.chat.add_message(user, text);
    HttpResponse::new(
        "201 Created",
        "application/json; charset=utf-8",
        message_to_json(&message),
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

fn message_to_json(message: &ChatMessage) -> String {
    format!(
        "{{\"id\":{},\"user\":\"{}\",\"text\":\"{}\"}}",
        message.id,
        escape_json(&message.user),
        escape_json(&message.text)
    )
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
