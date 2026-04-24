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
    assert!(response.body_text().contains("tiny_server chat"));
    assert!(response.body_text().contains("Room"));
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
    assert!(response.body_text().contains("\"message\":\"message is required\""));
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
    assert!(response.body_text().contains("user must use letters, numbers, '-' or '_'"));
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

    let mut list_stream = connect(&server.addr);
    let list_response = send_request(
        &mut list_stream,
        "GET /api/rooms HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(list_response.status_code(), 200);
    assert!(list_response.body_text().contains("\"name\":\"design\""));
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
    assert!(response.body_text().contains("\"service\":\"tiny_server_api\""));
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
