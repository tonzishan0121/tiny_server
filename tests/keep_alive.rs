mod common;

use common::{TestServer, assert_connection_closed, connect, send_request};

#[test]
fn keep_alive_supports_multiple_requests_on_one_connection() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);

    let first = send_request(
        &mut stream,
        "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert_eq!(first.status_code(), 200);
    assert_eq!(first.header("Connection"), Some("keep-alive"));

    let second = send_request(
        &mut stream,
        "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest",
    );
    assert_eq!(second.status_code(), 200);
    assert_eq!(second.body_text(), "test");
    assert_eq!(second.header("Connection"), Some("close"));
}

#[test]
fn connection_close_header_closes_stream_after_response() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);
    let response = send_request(
        &mut stream,
        "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );

    assert_eq!(response.status_code(), 200);
    assert_eq!(response.header("Connection"), Some("close"));
    assert_connection_closed(&mut stream);
}

#[test]
fn keep_alive_request_limit_closes_connection() {
    let server = TestServer::start();
    let mut stream = connect(&server.addr);

    let first = send_request(
        &mut stream,
        "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n",
    );
    assert_eq!(first.status_code(), 200);
    assert_eq!(first.header("Connection"), Some("keep-alive"));

    let second = send_request(&mut stream, "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert_eq!(second.status_code(), 200);
    assert_eq!(second.header("Connection"), Some("close"));

    assert_connection_closed(&mut stream);
}
