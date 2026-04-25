mod common;

use common::{
    TestServer, TestServerConfig, connect, reserve_port, send_request, send_then_shutdown,
};

#[test]
fn home_route_returns_200() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(response.body_text(), "tiny_server is running");
}

#[test]
fn health_route_returns_ok() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(response.body_text(), "ok");
}

#[test]
fn echo_route_returns_request_body() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 11\r\nConnection: close\r\n\r\nhello world",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(response.body_text(), "hello world");
}

#[test]
fn index_html_route_returns_static_page() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /index.html HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    assert!(response.body_text().contains("static file serving works"));
}

#[test]
fn malformed_request_returns_400() {
    let server = TestServer::start();
    let response = send_then_shutdown(&server.addr, "BROKEN\r\nHost: localhost\r\n\r\n");

    assert_eq!(response.status_code(), 400);
    assert_eq!(response.body_text(), "Bad Request");
}

#[test]
fn missing_route_returns_404() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /missing HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 404);
    assert_eq!(response.body_text(), "Not Found");
}

#[test]
fn wrong_method_returns_405() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "POST /health HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 405);
    assert_eq!(response.body_text(), "Method Not Allowed");
}

#[test]
fn unknown_handler_returns_500() {
    let port = reserve_port();
    let server = TestServer::start_with_config(TestServerConfig {
        server_yaml: format!(
            "host: 127.0.0.1\nport: {port}\nworker_threads: 2\nread_timeout_secs: 1\nkeep_alive_requests: 2\n"
        ),
        static_index_html: "<h1>unused</h1>\n",
        chat_index_html: include_str!("../static/chat/index.html"),
        chat_style_css: include_str!("../static/chat/style.css"),
    });
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /debug/500 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 500);
    assert_eq!(response.body_text(), "Debug Internal Error");
}

#[test]
fn oversized_request_returns_413() {
    use std::io::Write;

    let server = TestServer::start();
    let mut stream = connect(&server.addr);

    // Declare a 2 MiB body and stream enough bytes to exceed the 1 MiB server limit.
    let body_len = 2 * 1024 * 1024_usize;
    let header = format!(
        "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(header.as_bytes())
        .expect("should write header");

    // Fill enough body bytes to trigger the limit; write errors are ignored because
    // the server may close the connection before we finish.
    let chunk = vec![b'x'; 8 * 1024];
    let mut sent = 0;
    while sent < body_len {
        if stream.write_all(&chunk).is_err() {
            break;
        }
        sent += chunk.len();
    }
    let _ = stream.flush();

    let response = common::read_response_from(&mut stream);
    assert_eq!(response.status_code(), 413);
}
