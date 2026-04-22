use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use crate::http::{HttpResponse, parse_http_request};
use crate::router::{RouteRule, route_request};

pub fn run(addr: &str, routes: &[RouteRule]) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    println!("tiny_server listening on http://{addr}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_client(stream, routes) {
                    eprintln!("request handling error: {err}");
                }
            }
            Err(err) => eprintln!("connection failed: {err}"),
        }
    }

    Ok(())
}

fn handle_client(mut stream: TcpStream, routes: &[RouteRule]) -> std::io::Result<()> {
    let mut buffer = [0_u8; 2048];
    let bytes_read = stream.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let response = match parse_http_request(&request) {
        Ok(parsed_request) => route_request(&parsed_request, routes),
        Err(_) => HttpResponse::new(
            "400 Bad Request",
            "text/plain; charset=utf-8",
            "Bad Request",
        ),
    };

    let response_text = response.to_http_string();
    stream.write_all(response_text.as_bytes())?;
    stream.flush()?;
    Ok(())
}
