use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::app_state::AppState;
use crate::http::{HttpRequest, HttpResponse};

pub type HandlerFn = fn(&HttpRequest, &AppState) -> HttpResponse;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RouteMatchType {
    #[default]
    Exact,
    Prefix,
}

#[derive(Clone, Copy)]
pub struct Route {
    method: &'static str,
    path: &'static str,
    match_type: RouteMatchType,
    handler: HandlerFn,
}

impl Route {
    pub fn exact(method: &'static str, path: &'static str, handler: HandlerFn) -> Self {
        Self {
            method,
            path,
            match_type: RouteMatchType::Exact,
            handler,
        }
    }

    pub fn prefix(method: &'static str, path: &'static str, handler: HandlerFn) -> Self {
        Self {
            method,
            path,
            match_type: RouteMatchType::Prefix,
            handler,
        }
    }
}

#[macro_export]
macro_rules! routes {
    ($($method:ident $path:literal => $handler:path $([$match_type:ident])? ),+ $(,)?) => {{
        vec![
            $(
                $crate::route_entry!(
                    $method,
                    $path,
                    $handler
                    $(,$match_type)?
                )
            ),+
        ]
    }};
}

#[macro_export]
macro_rules! route_entry {
    ($method:ident, $path:literal, $handler:path, prefix) => {
        $crate::router::Route::prefix(stringify!($method), $path, $handler)
    };
    ($method:ident, $path:literal, $handler:path) => {
        $crate::router::Route::exact(stringify!($method), $path, $handler)
    };
}

/// Finds the best matching route and calls its handler.
pub fn route_request(request: &HttpRequest, routes: &[Route], state: &AppState) -> HttpResponse {
    let matched = routes.iter().find(|route| {
        route.method == request.method && path_matches(route.match_type, route.path, &request.path)
    });

    if let Some(route) = matched {
        return (route.handler)(request, state);
    }

    if routes
        .iter()
        .any(|route| path_matches(route.match_type, route.path, &request.path))
    {
        return HttpResponse::new(
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            "Method Not Allowed",
        );
    }

    HttpResponse::new("404 Not Found", "text/plain; charset=utf-8", "Not Found")
}

fn path_matches(match_type: RouteMatchType, route_path: &str, request_path: &str) -> bool {
    let request_path = request_path.split('?').next().unwrap_or(request_path);

    match match_type {
        RouteMatchType::Exact => route_path == request_path,
        RouteMatchType::Prefix => request_path.starts_with(route_path),
    }
}

pub fn home_handler(_: &HttpRequest, _: &AppState) -> HttpResponse {
    HttpResponse::new(
        "200 OK",
        "text/plain; charset=utf-8",
        "tiny_server is running",
    )
}

pub fn health_handler(_: &HttpRequest, _: &AppState) -> HttpResponse {
    HttpResponse::new("200 OK", "text/plain; charset=utf-8", "ok")
}

pub fn static_index_handler(_: &HttpRequest, _: &AppState) -> HttpResponse {
    static_file_response(Path::new("static/index.html"))
}

pub fn static_handler(request: &HttpRequest, _: &AppState) -> HttpResponse {
    let Some(relative_path) = request.path.strip_prefix("/static/") else {
        return HttpResponse::new("404 Not Found", "text/plain; charset=utf-8", "Not Found");
    };

    let Some(file_path) = safe_static_path(relative_path) else {
        return HttpResponse::new("403 Forbidden", "text/plain; charset=utf-8", "Forbidden");
    };

    static_file_response(&file_path)
}

pub fn echo_handler(request: &HttpRequest, _: &AppState) -> HttpResponse {
    HttpResponse::new("200 OK", "text/plain; charset=utf-8", request.body.clone())
}

pub fn debug_error_handler(_: &HttpRequest, _: &AppState) -> HttpResponse {
    HttpResponse::new(
        "500 Internal Server Error",
        "text/plain; charset=utf-8",
        "Debug Internal Error",
    )
}

fn static_file_response(path: &Path) -> HttpResponse {
    match fs::read(path) {
        Ok(content) => HttpResponse::bytes("200 OK", content_type(path), content)
            .with_header("Cache-Control", "public, max-age=60"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::new("404 Not Found", "text/plain; charset=utf-8", "Not Found")
        }
        Err(_) => HttpResponse::new(
            "500 Internal Server Error",
            "text/plain; charset=utf-8",
            "failed to read static file",
        ),
    }
}

fn safe_static_path(relative_path: &str) -> Option<PathBuf> {
    let mut path = PathBuf::from("static");
    let relative_path = relative_path.split('?').next().unwrap_or(relative_path);

    if relative_path.is_empty() {
        return None;
    }

    for component in Path::new(relative_path).components() {
        match component {
            Component::Normal(part) => path.push(part),
            _ => return None,
        }
    }

    Some(path)
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
