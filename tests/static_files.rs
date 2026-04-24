mod common;

use common::{TestServer, connect, send_request};

#[test]
fn static_file_returns_html_with_cache_headers() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /static/index.html HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(response.header("Cache-Control"), Some("public, max-age=60"));
    assert!(response.body_text().contains("tiny_server"));
}

#[test]
fn missing_static_file_returns_404() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /static/missing.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 404);
    assert_eq!(response.body_text(), "Not Found");
}

#[test]
fn static_path_traversal_returns_403() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET /static/../Cargo.toml HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 403);
    assert_eq!(response.body_text(), "Forbidden");
}
