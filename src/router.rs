use std::fs;

use serde::Deserialize;

use crate::http::{HttpRequest, HttpResponse};

#[derive(Deserialize)]
struct RouteConfig {
    routes: Vec<RouteRule>,
}

#[derive(Deserialize)]
pub struct RouteRule {
    method: String,
    path: String,
    handler: String,
}

pub fn load_routes(path: &str) -> Result<Vec<RouteRule>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: RouteConfig = serde_yaml::from_str(&content)?;
    Ok(config.routes)
}

pub fn route_request(request: &HttpRequest, routes: &[RouteRule]) -> HttpResponse {
    let _ = (&request.version, &request.headers);

    let Some(rule) = routes
        .iter()
        .find(|rule| rule.method == request.method && rule.path == request.path)
    else {
        if routes.iter().any(|rule| rule.path == request.path) {
            return HttpResponse::new(
                "405 Method Not Allowed",
                "text/plain; charset=utf-8",
                "Method Not Allowed",
            );
        }

        return HttpResponse::new("404 Not Found", "text/plain; charset=utf-8", "Not Found");
    };

    dispatch_handler(&rule.handler, request)
}

fn dispatch_handler(handler: &str, request: &HttpRequest) -> HttpResponse {
    match handler {
        "home" => HttpResponse::new(
            "200 OK",
            "text/plain; charset=utf-8",
            "tiny_server is running",
        ),
        "health" => HttpResponse::new("200 OK", "text/plain; charset=utf-8", "ok"),
        "static_index" => match fs::read_to_string("static/index.html") {
            Ok(content) => HttpResponse::new("200 OK", "text/html; charset=utf-8", content),
            Err(_) => HttpResponse::new(
                "500 Internal Server Error",
                "text/plain; charset=utf-8",
                "failed to read static/index.html",
            ),
        },
        "echo" => HttpResponse::new(
            "200 OK",
            "text/plain; charset=utf-8",
            handler_echo(request),
        ),
        _ => HttpResponse::new(
            "500 Internal Server Error",
            "text/plain; charset=utf-8",
            "Unknown handler",
        ),
    }
}

fn handler_echo(request: &HttpRequest) -> String {
    request.body.clone()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{load_routes, route_request};
    use crate::http::HttpRequest;

    #[test]
    fn route_post_echo_returns_body() {
        let routes = load_routes("config/routes.yaml").expect("should load routes");
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/echo".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: vec![],
            body: "ping".to_string(),
        };

        let response = route_request(&request, &routes);
        assert_eq!(response.status(), "200 OK");
        assert_eq!(response.body(), "ping");
    }

    #[test]
    fn route_method_not_allowed_when_path_exists() {
        let routes = load_routes("config/routes.yaml").expect("should load routes");
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/health".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: vec![],
            body: String::new(),
        };

        let response = route_request(&request, &routes);
        assert_eq!(response.status(), "405 Method Not Allowed");
    }

    #[test]
    fn load_routes_from_yaml_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("routes_{unique}.yaml"));
        let yaml = "routes:\n  - method: POST\n    path: /echo\n    handler: echo\n";
        fs::write(&path, yaml).expect("should write routes config");

        let routes = load_routes(path.to_str().expect("path should be utf-8"))
            .expect("should load routes config");
        let request = HttpRequest {
            method: "POST".to_string(),
            path: "/echo".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: vec![],
            body: "hello".to_string(),
        };
        let response = route_request(&request, &routes);
        assert_eq!(response.status(), "200 OK");
        assert_eq!(response.body(), "hello");

        fs::remove_file(path).expect("should remove temp file");
    }
}
