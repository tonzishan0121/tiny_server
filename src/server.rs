use std::borrow::Cow;
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Maximum number of bytes accepted for a single HTTP request (headers + body).
const MAX_REQUEST_BYTES: usize = 1024 * 1024; // 1 MiB

use crate::app_state::AppState;
use crate::concurrency::ThreadPool;
use crate::config::ServerConfig;
use crate::http::{HttpResponse, parse_http_request};
use crate::router::{Route, route_request};

/// Starts the TCP listener and dispatches accepted connections to the worker pool.
pub fn run(
    addr: &str,
    config: ServerConfig,
    routes: Vec<Route>,
    state: AppState,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    let pool = ThreadPool::new(config.worker_threads);
    let routes = Arc::new(routes);
    let state = Arc::new(state);
    let running = Arc::new(AtomicBool::new(true));
    let read_timeout = Duration::from_secs(config.read_timeout_secs);
    let keep_alive_requests = config.keep_alive_requests.max(1);

    install_ctrlc_handler(&running)?;

    println!(
        "tiny_server listening on http://{addr} with {} workers",
        config.worker_threads
    );

    while running.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let routes = Arc::clone(&routes);
                let state = Arc::clone(&state);
                if let Err(err) = pool.execute(move || {
                    if let Err(err) =
                        handle_client(stream, &routes, &state, read_timeout, keep_alive_requests)
                    {
                        eprintln!("request handling error: {err}");
                    }
                }) {
                    eprintln!("failed to queue connection: {err}");
                };
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => eprintln!("connection failed: {err}"),
        }
    }

    println!("tiny_server shutting down");

    Ok(())
}

fn install_ctrlc_handler(running: &Arc<AtomicBool>) -> std::io::Result<()> {
    let running = Arc::clone(running);
    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);
    })
    .map_err(|err| std::io::Error::other(format!("failed to install ctrl-c handler: {err}")))
}

fn handle_client(
    mut stream: TcpStream,
    routes: &[Route],
    state: &AppState,
    read_timeout: Duration,
    keep_alive_requests: usize,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(read_timeout))?;

    for request_idx in 0..keep_alive_requests {
        let raw_request = match read_http_request(&mut stream) {
            Ok(Some(raw)) => raw,
            Ok(None) => return Ok(()),
            Err(err) if err.kind() == ErrorKind::InvalidData => {
                let response = HttpResponse::new(
                    "413 Content Too Large",
                    "text/plain; charset=utf-8",
                    "Request Too Large",
                );
                stream.write_all(&response.to_http_bytes(false))?;
                stream.flush()?;
                return Ok(());
            }
            Err(err) => return Err(err),
        };

        let parsed_request = match parse_http_request(&raw_request) {
            Ok(request) => request,
            Err(_) => {
                let response = HttpResponse::new(
                    "400 Bad Request",
                    "text/plain; charset=utf-8",
                    "Bad Request",
                );
                stream.write_all(&response.to_http_bytes(false))?;
                stream.flush()?;
                return Ok(());
            }
        };

        let keep_alive = parsed_request.wants_keep_alive() && request_idx + 1 < keep_alive_requests;
        let response = route_request(&parsed_request, routes, state);
        let status = response.status().to_string();

        println!(
            "{} {} -> {status}",
            sanitize_log(&parsed_request.method),
            sanitize_log(&parsed_request.path)
        );
        stream.write_all(&response.to_http_bytes(keep_alive))?;
        stream.flush()?;

        if !keep_alive {
            break;
        }
    }

    Ok(())
}

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<Option<String>> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];

    loop {
        match stream.read(&mut chunk) {
            Ok(0) if buffer.is_empty() => return Ok(None),
            Ok(0) => break,
            Ok(bytes_read) => {
                buffer.extend_from_slice(&chunk[..bytes_read]);
                if buffer.len() > MAX_REQUEST_BYTES {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidData,
                        "request too large",
                    ));
                }
                if request_is_complete(&buffer) {
                    break;
                }
            }
            Err(err)
                if matches!(
                    err.kind(),
                    ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                ) && buffer.is_empty() =>
            {
                if err.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Ok(None);
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }

    Ok(Some(String::from_utf8_lossy(&buffer).into_owned()))
}

fn request_is_complete(buffer: &[u8]) -> bool {
    let Some(header_end) = find_header_end(buffer) else {
        return false;
    };

    let content_length = request_content_length(&buffer[..header_end]).unwrap_or(0);
    buffer.len() >= header_end + 4 + content_length
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn request_content_length(header_bytes: &[u8]) -> Option<usize> {
    let headers = std::str::from_utf8(header_bytes).ok()?;

    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("Content-Length") {
            return value.trim().parse().ok();
        }
        None
    })
}

/// Replaces ASCII control characters with spaces to prevent log injection.
fn sanitize_log(s: &str) -> Cow<'_, str> {
    if s.chars().any(|c| (c as u32) < 0x20) {
        Cow::Owned(s.chars().map(|c| if (c as u32) < 0x20 { ' ' } else { c }).collect())
    } else {
        Cow::Borrowed(s)
    }
}
