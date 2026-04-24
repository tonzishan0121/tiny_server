mod common;

use std::thread;
use std::time::Duration;

use common::{
    TestServer, TestServerConfig, assert_connection_closed, assert_read_times_out, connect,
    reserve_port, send_request, write_partial_request,
};

#[test]
fn incomplete_request_times_out_and_connection_closes() {
    let port = reserve_port();
    let server = TestServer::start_with_config(TestServerConfig {
        server_yaml: format!(
            "host: 127.0.0.1\nport: {port}\nworker_threads: 2\nread_timeout_secs: 1\nkeep_alive_requests: 2\n"
        ),
        static_index_html: include_str!("../static/index.html"),
        chat_index_html: include_str!("../static/chat/index.html"),
        chat_style_css: include_str!("../static/chat/style.css"),
    });
    let mut stream = connect(&server.addr);

    write_partial_request(
        &mut stream,
        "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 11\r\n\r\nhello",
    );

    thread::sleep(Duration::from_millis(1200));
    assert_connection_closed(&mut stream);
}

#[test]
fn slow_connection_does_not_block_other_requests() {
    let port = reserve_port();
    let server = TestServer::start_with_config(TestServerConfig {
        server_yaml: format!(
            "host: 127.0.0.1\nport: {port}\nworker_threads: 2\nread_timeout_secs: 1\nkeep_alive_requests: 2\n"
        ),
        static_index_html: include_str!("../static/index.html"),
        chat_index_html: include_str!("../static/chat/index.html"),
        chat_style_css: include_str!("../static/chat/style.css"),
    });

    let mut slow_stream = connect(&server.addr);
    write_partial_request(
        &mut slow_stream,
        "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 20\r\n\r\npartial",
    );
    assert_read_times_out(&mut slow_stream);

    let addr = server.addr.clone();
    let workers = (0..4)
        .map(|_| {
            let addr = addr.clone();
            thread::spawn(move || {
                let mut stream = connect(&addr);
                let response = send_request(
                    &mut stream,
                    "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                );
                assert_eq!(response.status_code(), 200);
                assert_eq!(response.body_text(), "ok");
            })
        })
        .collect::<Vec<_>>();

    for worker in workers {
        worker.join().expect("request thread should finish");
    }

    thread::sleep(Duration::from_millis(1200));
    assert_connection_closed(&mut slow_stream);
}
