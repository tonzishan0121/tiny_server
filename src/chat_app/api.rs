use crate::http::HttpResponse;

pub const CODE_OK: i32 = 0;

pub const CODE_BAD_REQUEST: i32 = 1000;

pub const CODE_ROOM_INVALID: i32 = 1101;
pub const CODE_ROOM_ALREADY_EXISTS: i32 = 1102;
pub const CODE_ROOM_NOT_FOUND: i32 = 1103;
pub const CODE_ROOM_PROTECTED: i32 = 1104;

pub const CODE_USER_INVALID: i32 = 1201;
pub const CODE_MESSAGE_INVALID: i32 = 1301;

pub const CODE_INTERNAL_ERROR: i32 = 9000;

pub fn success_response(status: &'static str, data_json: String) -> HttpResponse {
    let body = format!(
        "{{\"ok\":true,\"code\":{},\"message\":\"ok\",\"data\":{}}}",
        CODE_OK, data_json
    );
    HttpResponse::new(status, "application/json; charset=utf-8", body)
}

pub fn error_response(
    status: &'static str,
    code: i32,
    message: &str,
    details_json: Option<String>,
) -> HttpResponse {
    let details_json = details_json.unwrap_or_else(|| "null".to_string());
    let body = format!(
        "{{\"ok\":false,\"code\":{},\"message\":\"{}\",\"error\":{{\"code\":{},\"message\":\"{}\",\"details\":{}}},\"data\":null}}",
        code,
        escape_json(message),
        code,
        escape_json(message),
        details_json
    );
    HttpResponse::new(status, "application/json; charset=utf-8", body)
}

pub fn escape_json(value: &str) -> String {
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
