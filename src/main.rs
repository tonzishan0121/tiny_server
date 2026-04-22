use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    let addr = "127.0.0.1:7878";
    let listener = TcpListener::bind(addr).expect("failed to bind address");
    println!("tiny_server listening on http://{addr}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_client(stream) {
                    eprintln!("request handling error: {err}");
                }
            }
            Err(err) => eprintln!("connection failed: {err}"),
        }
    }
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0_u8; 2048];
    let bytes_read = stream.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let request_line = request.lines().next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();

    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();

    let response = route_request(method, path);
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn route_request(method: &str, path: &str) -> String {
    if method != "GET" {
        return build_response(
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            "Only GET is supported.",
        );
    }

    match path {
        "/" => build_response(
            "200 OK",
            "text/plain; charset=utf-8",
            "tiny_server is running",
        ),
        "/health" => build_response("200 OK", "text/plain; charset=utf-8", "ok"),
        "/index.html" => match fs::read_to_string("static/index.html") {
            Ok(content) => build_response("200 OK", "text/html; charset=utf-8", &content),
            Err(_) => build_response(
                "500 Internal Server Error",
                "text/plain; charset=utf-8",
                "failed to read static/index.html",
            ),
        },
        _ => build_response("404 Not Found", "text/plain; charset=utf-8", "Not Found"),
    }
}

fn build_response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
