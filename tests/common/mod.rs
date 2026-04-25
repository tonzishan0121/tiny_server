#![allow(dead_code)]

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct TestServer {
    child: Child,
    pub addr: String,
    root_dir: PathBuf,
}

pub struct TestServerConfig<'a> {
    pub server_yaml: String,
    pub static_index_html: &'a str,
    pub chat_index_html: &'a str,
    pub chat_style_css: &'a str,
}

impl TestServer {
    pub fn start() -> Self {
        let port = reserve_port();
        Self::start_with_config(default_config_for_port(port))
    }

    pub fn start_with_config(config: TestServerConfig<'_>) -> Self {
        let port = extract_port(&config.server_yaml);
        let root_dir = prepare_test_root(config);
        let addr = format!("127.0.0.1:{port}");
        let binary = env!("CARGO_BIN_EXE_tiny_server");

        let child = Command::new(binary)
            .current_dir(&root_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("should start tiny_server");

        wait_until_ready(&addr);

        Self {
            child,
            addr,
            root_dir,
        }
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.root_dir);
    }
}

pub fn connect(addr: &str) -> TcpStream {
    TcpStream::connect(addr).expect("should connect to test server")
}

pub fn send_request(stream: &mut TcpStream, request: &str) -> ParsedResponse {
    write_partial_request(stream, request);
    read_response(stream)
}

pub fn send_then_shutdown(addr: &str, request: &str) -> ParsedResponse {
    let mut stream = connect(addr);
    write_partial_request(&mut stream, request);
    stream
        .shutdown(Shutdown::Write)
        .expect("should shutdown write half");
    read_response(&mut stream)
}

pub fn write_partial_request(stream: &mut TcpStream, request: &str) {
    stream
        .write_all(request.as_bytes())
        .expect("should write request");
    stream.flush().expect("should flush request");
}

pub fn assert_connection_closed(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("should set timeout");

    let mut byte = [0_u8; 1];
    let bytes = stream.read(&mut byte).expect("connection should close");
    assert_eq!(bytes, 0, "expected server to close the connection");
}

pub fn assert_read_times_out(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_millis(200)))
        .expect("should set timeout");

    let mut byte = [0_u8; 1];
    let err = stream
        .read(&mut byte)
        .expect_err("read should time out before server responds");
    assert!(
        matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut),
        "expected timeout error, got {err}"
    );
}

pub struct ParsedResponse {
    pub status_line: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl ParsedResponse {
    pub fn status_code(&self) -> u16 {
        self.status_line
            .split_whitespace()
            .nth(1)
            .expect("status line should contain code")
            .parse()
            .expect("status code should parse")
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

fn read_response(stream: &mut TcpStream) -> ParsedResponse {
    read_response_from(stream)
}

pub fn read_response_from(stream: &mut TcpStream) -> ParsedResponse {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("should set timeout");

    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        let bytes_read = stream.read(&mut chunk).expect("should read response");
        assert!(bytes_read > 0, "server closed before response completed");
        buffer.extend_from_slice(&chunk[..bytes_read]);

        if let Some(response) = try_parse_response(&buffer) {
            return response;
        }
    }
}

fn try_parse_response(buffer: &[u8]) -> Option<ParsedResponse> {
    let header_end = buffer.windows(4).position(|window| window == b"\r\n\r\n")?;
    let head = std::str::from_utf8(&buffer[..header_end]).ok()?;
    let mut lines = head.lines();
    let status_line = lines.next()?.to_string();
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect::<Vec<_>>();
    let content_length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("Content-Length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    let body_start = header_end + 4;

    if buffer.len() < body_start + content_length {
        return None;
    }

    Some(ParsedResponse {
        status_line,
        headers,
        body: buffer[body_start..body_start + content_length].to_vec(),
    })
}

pub fn reserve_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("should bind random port")
        .local_addr()
        .expect("listener should have local addr")
        .port()
}

fn wait_until_ready(addr: &str) {
    for _ in 0..50 {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }

    panic!("test server did not become ready at {addr}");
}

fn default_config_for_port(port: u16) -> TestServerConfig<'static> {
    TestServerConfig {
        server_yaml: format!(
            "host: 127.0.0.1\nport: {port}\nworker_threads: 4\nread_timeout_secs: 1\nkeep_alive_requests: 2\n"
        ),
        static_index_html: include_str!("../../static/index.html"),
        chat_index_html: include_str!("../../static/chat/index.html"),
        chat_style_css: include_str!("../../static/chat/style.css"),
    }
}

fn prepare_test_root(config: TestServerConfig<'_>) -> PathBuf {
    let port = extract_port(&config.server_yaml);
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be valid")
        .as_nanos();
    let root_dir = std::env::temp_dir().join(format!("tiny_server_test_{port}_{unique}"));
    fs::create_dir_all(root_dir.join("config")).expect("should create config dir");
    fs::create_dir_all(root_dir.join("data")).expect("should create data dir");
    fs::create_dir_all(root_dir.join("static")).expect("should create static dir");
    fs::create_dir_all(root_dir.join("static/chat")).expect("should create chat static dir");

    fs::write(root_dir.join("config/server.yaml"), config.server_yaml)
        .expect("should write server config");
    fs::write(
        root_dir.join("config/database.yaml"),
        "driver: sqlite\npath: data/test.sqlite3\n",
    )
    .expect("should write database config");
    fs::write(root_dir.join("static/index.html"), config.static_index_html)
        .expect("should write static fixture");
    fs::write(
        root_dir.join("static/chat/index.html"),
        config.chat_index_html,
    )
    .expect("should write chat fixture");
    fs::write(
        root_dir.join("static/chat/style.css"),
        config.chat_style_css,
    )
    .expect("should write chat style fixture");

    root_dir
}

fn extract_port(server_yaml: &str) -> u16 {
    server_yaml
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim() == "port" {
                return value.trim().parse().ok();
            }
            None
        })
        .expect("server yaml should contain a valid port")
}
