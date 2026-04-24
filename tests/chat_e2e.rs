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
}

#[test]
fn chat_api_returns_seed_message() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /api/messages HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("application/json; charset=utf-8")
    );
    assert!(response.body_text().contains("Welcome to tiny_server chat"));
}

#[test]
fn chat_api_creates_and_lists_messages() {
    let server = TestServer::start();

    let mut post_stream = connect(&server.addr);
    let create_response = send_request(
        &mut post_stream,
        "POST /api/messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 29\r\nConnection: close\r\n\r\nuser=alice&message=hello+world",
    );
    assert_eq!(create_response.status_code(), 201);
    assert!(create_response.body_text().contains("\"user\":\"alice\""));
    assert!(
        create_response
            .body_text()
            .contains("\"text\":\"hello world\"")
    );

    let mut list_stream = connect(&server.addr);
    let list_response = send_request(
        &mut list_stream,
        "GET /api/messages HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(list_response.status_code(), 200);
    assert!(list_response.body_text().contains("\"user\":\"alice\""));
    assert!(
        list_response
            .body_text()
            .contains("\"text\":\"hello world\"")
    );
}

#[test]
fn chat_api_rejects_empty_message() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "POST /api/messages HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 17\r\nConnection: close\r\n\r\nuser=alice&message=",
    );

    assert_eq!(response.status_code(), 400);
    assert_eq!(response.body_text(), "message is required");
}
