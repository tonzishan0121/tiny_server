mod common;

use common::{TestServer, connect, send_request};

#[test]
fn chat_page_returns_html() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /chat HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(response.header("Cache-Control"), Some("no-store"));
    assert!(response.body_text().contains("tiny_server chat"));
    assert!(response.body_text().contains("/static/chat/style.css"));
    assert!(response.body_text().contains("/api/rooms"));
    assert!(response.body_text().contains("Current room"));
    assert!(response.body_text().contains("chat:refresh"));
    assert!(!response.body_text().contains("setInterval"));
    assert!(!response.body_text().contains("setTimeout"));
}

#[test]
fn chat_api_returns_seed_message() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /api/messages?room=lobby HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("application/json; charset=utf-8")
    );
    assert!(response.body_text().contains("\"ok\":true"));
    assert!(response.body_text().contains("\"code\":0"));
    assert!(response.body_text().contains("Welcome to tiny_server chat"));
    assert!(response.body_text().contains("\"room\":\"lobby\""));
    assert!(response.body_text().contains("\"created_at_secs\":"));
}

#[test]
fn chat_api_creates_and_lists_messages() {
    let server = TestServer::start();

    let mut post_stream = connect(&server.addr);
    let create_response = send_request(
        &mut post_stream,
        "POST /api/messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 40\r\nConnection: close\r\n\r\nroom=lobby&user=alice&message=hello+world",
    );
    assert_eq!(create_response.status_code(), 201);
    assert!(create_response.body_text().contains("\"ok\":true"));
    assert!(create_response.body_text().contains("\"room\":\"lobby\""));
    assert!(create_response.body_text().contains("\"user\":\"alice\""));
    assert!(
        create_response
            .body_text()
            .contains("\"text\":\"hello world\"")
    );
    assert!(create_response.body_text().contains("\"created_at_secs\":"));

    let mut list_stream = connect(&server.addr);
    let list_response = send_request(
        &mut list_stream,
        "GET /api/messages?room=lobby HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(list_response.status_code(), 200);
    assert!(list_response.body_text().contains("\"ok\":true"));
    assert!(list_response.body_text().contains("\"room\":\"lobby\""));
    assert!(list_response.body_text().contains("\"user\":\"alice\""));
    assert!(
        list_response
            .body_text()
            .contains("\"text\":\"hello world\"")
    );
    assert!(list_response.body_text().contains("\"created_at_secs\":"));
}

#[test]
fn chat_api_rejects_empty_message() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "POST /api/messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 30\r\nConnection: close\r\n\r\nroom=lobby&user=alice&message=",
    );

    assert_eq!(response.status_code(), 400);
    assert!(response.body_text().contains("\"ok\":false"));
    assert!(response.body_text().contains("\"code\":1301"));
    assert!(
        response
            .body_text()
            .contains("\"message\":\"message is required\"")
    );
}

#[test]
fn chat_rooms_are_isolated() {
    let server = TestServer::start();

    let mut rust_stream = connect(&server.addr);
    let rust_response = send_request(
        &mut rust_stream,
        "POST /api/messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 38\r\nConnection: close\r\n\r\nroom=rust&user=alice&message=ownership",
    );
    assert_eq!(rust_response.status_code(), 201);

    let mut music_stream = connect(&server.addr);
    let music_response = send_request(
        &mut music_stream,
        "POST /api/messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 34\r\nConnection: close\r\n\r\nroom=music&user=bob&message=guitar",
    );
    assert_eq!(music_response.status_code(), 201);

    let mut rust_list = connect(&server.addr);
    let rust_messages = send_request(
        &mut rust_list,
        "GET /api/messages?room=rust HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(rust_messages.body_text().contains("\"room\":\"rust\""));
    assert!(rust_messages.body_text().contains("ownership"));
    assert!(!rust_messages.body_text().contains("guitar"));

    let mut music_list = connect(&server.addr);
    let music_messages = send_request(
        &mut music_list,
        "GET /api/messages?room=music HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert!(music_messages.body_text().contains("\"room\":\"music\""));
    assert!(music_messages.body_text().contains("guitar"));
    assert!(!music_messages.body_text().contains("ownership"));
}

#[test]
fn chat_api_rejects_invalid_user_name() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "POST /api/messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 38\r\nConnection: close\r\n\r\nroom=lobby&user=a!&message=hello+world",
    );

    assert_eq!(response.status_code(), 400);
    assert!(response.body_text().contains("\"ok\":false"));
    assert!(response.body_text().contains("\"code\":1201"));
    assert!(
        response
            .body_text()
            .contains("user must use letters, numbers, '-' or '_'")
    );
}

#[test]
fn rooms_api_lists_default_rooms() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /api/rooms HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("application/json; charset=utf-8")
    );
    assert!(response.body_text().contains("\"ok\":true"));
    assert!(response.body_text().contains("\"name\":\"lobby\""));
    assert!(response.body_text().contains("\"name\":\"rust\""));
    assert!(response.body_text().contains("\"message_count\":"));
    assert!(response.body_text().contains("\"last_message\":"));
    assert!(response.body_text().contains("Welcome to tiny_server chat"));
}

#[test]
fn rooms_api_creates_new_room() {
    let server = TestServer::start();
    let mut create_stream = connect(&server.addr);
    let create_response = send_request(
        &mut create_stream,
        "POST /api/rooms HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 11\r\nConnection: close\r\n\r\nroom=design",
    );

    assert_eq!(create_response.status_code(), 201);
    assert!(create_response.body_text().contains("\"ok\":true"));
    assert!(create_response.body_text().contains("\"name\":\"design\""));
    assert!(create_response.body_text().contains("\"message_count\":0"));
    assert!(
        create_response
            .body_text()
            .contains("\"last_message\":null")
    );

    let mut list_stream = connect(&server.addr);
    let list_response = send_request(
        &mut list_stream,
        "GET /api/rooms HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(list_response.status_code(), 200);
    assert!(list_response.body_text().contains("\"name\":\"design\""));
}

#[test]
fn rooms_api_includes_latest_message_preview() {
    let server = TestServer::start();
    let mut message_stream = connect(&server.addr);
    let create_response = send_request(
        &mut message_stream,
        "POST /api/messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 38\r\nConnection: close\r\n\r\nroom=rust&user=alice&message=ownership",
    );
    assert_eq!(create_response.status_code(), 201);

    let mut rooms_stream = connect(&server.addr);
    let rooms_response = send_request(
        &mut rooms_stream,
        "GET /api/rooms HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(rooms_response.status_code(), 200);
    assert!(rooms_response.body_text().contains("\"name\":\"rust\""));
    assert!(rooms_response.body_text().contains("\"message_count\":1"));
    assert!(rooms_response.body_text().contains("\"last_message\":"));
    assert!(rooms_response.body_text().contains("\"user\":\"alice\""));
    assert!(
        rooms_response
            .body_text()
            .contains("\"text\":\"ownership\"")
    );
}

#[test]
fn rooms_api_renames_room_and_moves_messages() {
    let server = TestServer::start();

    let mut create_stream = connect(&server.addr);
    let create_response = send_request(
        &mut create_stream,
        "POST /api/rooms HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 11\r\nConnection: close\r\n\r\nroom=design",
    );
    assert_eq!(create_response.status_code(), 201);

    let mut message_stream = connect(&server.addr);
    let message_response = send_request(
        &mut message_stream,
        "POST /api/messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 35\r\nConnection: close\r\n\r\nroom=design&user=alice&message=plan",
    );
    assert_eq!(message_response.status_code(), 201);

    let mut rename_stream = connect(&server.addr);
    let rename_response = send_request(
        &mut rename_stream,
        "POST /api/rooms/rename HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 26\r\nConnection: close\r\n\r\nroom=design&new_room=ideas",
    );
    assert_eq!(rename_response.status_code(), 200);
    assert!(rename_response.body_text().contains("\"name\":\"ideas\""));
    assert!(rename_response.body_text().contains("\"message_count\":1"));
    assert!(rename_response.body_text().contains("\"text\":\"plan\""));

    let mut old_room_stream = connect(&server.addr);
    let old_room_response = send_request(
        &mut old_room_stream,
        "GET /api/messages?room=design HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(old_room_response.status_code(), 404);

    let mut new_room_stream = connect(&server.addr);
    let new_room_response = send_request(
        &mut new_room_stream,
        "GET /api/messages?room=ideas HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(new_room_response.status_code(), 200);
    assert!(new_room_response.body_text().contains("\"room\":\"ideas\""));
    assert!(new_room_response.body_text().contains("\"text\":\"plan\""));
}

#[test]
fn rooms_api_deletes_room_and_messages() {
    let server = TestServer::start();

    let mut create_stream = connect(&server.addr);
    let create_response = send_request(
        &mut create_stream,
        "POST /api/rooms HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 11\r\nConnection: close\r\n\r\nroom=design",
    );
    assert_eq!(create_response.status_code(), 201);

    let mut delete_stream = connect(&server.addr);
    let delete_response = send_request(
        &mut delete_stream,
        "POST /api/rooms/delete HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 11\r\nConnection: close\r\n\r\nroom=design",
    );
    assert_eq!(delete_response.status_code(), 200);
    assert!(delete_response.body_text().contains("\"deleted\":true"));

    let mut messages_stream = connect(&server.addr);
    let messages_response = send_request(
        &mut messages_stream,
        "GET /api/messages?room=design HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(messages_response.status_code(), 404);
}

#[test]
fn rooms_api_protects_default_room_from_update_and_delete() {
    let server = TestServer::start();

    let mut rename_stream = connect(&server.addr);
    let rename_response = send_request(
        &mut rename_stream,
        "POST /api/rooms/rename HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 24\r\nConnection: close\r\n\r\nroom=lobby&new_room=main",
    );
    assert_eq!(rename_response.status_code(), 409);
    assert!(rename_response.body_text().contains("\"code\":1104"));

    let mut delete_stream = connect(&server.addr);
    let delete_response = send_request(
        &mut delete_stream,
        "POST /api/rooms/delete HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 10\r\nConnection: close\r\n\r\nroom=lobby",
    );
    assert_eq!(delete_response.status_code(), 409);
    assert!(delete_response.body_text().contains("\"code\":1104"));
}

#[test]
fn api_health_returns_json_status() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("application/json; charset=utf-8")
    );
    assert!(response.body_text().contains("\"ok\":true"));
    assert!(
        response
            .body_text()
            .contains("\"service\":\"tiny_server_api\"")
    );
}

#[test]
fn api_meta_returns_limits_and_default_room() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /api/meta HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert!(response.body_text().contains("\"ok\":true"));
    assert!(response.body_text().contains("\"default_room\":\"lobby\""));
    assert!(response.body_text().contains("\"max_message_len\":400"));
}

#[test]
fn messages_api_returns_room_not_found_business_code() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /api/messages?room=unknown HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 404);
    assert!(response.body_text().contains("\"ok\":false"));
    assert!(response.body_text().contains("\"code\":1103"));
    assert!(response.body_text().contains("room does not exist"));
}

#[test]
fn create_room_returns_conflict_for_duplicate_room() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "POST /api/rooms HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 10\r\nConnection: close\r\n\r\nroom=lobby",
    );

    assert_eq!(response.status_code(), 409);
    assert!(response.body_text().contains("\"ok\":false"));
    assert!(response.body_text().contains("\"code\":1102"));
    assert!(response.body_text().contains("room already exists"));
}
