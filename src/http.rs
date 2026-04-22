pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub struct HttpResponse {
    status: &'static str,
    content_type: &'static str,
    body: String,
}

impl HttpResponse {
    pub fn new(status: &'static str, content_type: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type,
            body: body.into(),
        }
    }

    pub fn to_http_string(&self) -> String {
        format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            self.status,
            self.content_type,
            self.body.len(),
            self.body
        )
    }

    pub fn status(&self) -> &str {
        self.status
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

pub fn parse_http_request(raw: &str) -> Result<HttpRequest, &'static str> {
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((raw, ""));
    let mut lines = head.lines();
    let request_line = lines.next().ok_or("missing request line")?;
    let (method, path, version) = parse_request_line(request_line)?;
    let headers = parse_headers(lines)?;

    Ok(HttpRequest {
        method,
        path,
        version,
        headers,
        body: body.to_string(),
    })
}

fn parse_request_line(line: &str) -> Result<(String, String, String), &'static str> {
    let mut parts = line.trim_end_matches('\r').split_whitespace();

    let method = parts.next().ok_or("missing method")?;
    let path = parts.next().ok_or("missing path")?;
    let version = parts.next().ok_or("missing version")?;

    if parts.next().is_some() {
        return Err("too many fields");
    }

    if !version.starts_with("HTTP/") {
        return Err("invalid version");
    }

    Ok((method.to_string(), path.to_string(), version.to_string()))
}

#[cfg(test)]
mod tests {
    use super::parse_http_request;

    #[test]
    fn parse_get_request() {
        let raw = "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_http_request(raw).expect("request should parse");

        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/health");
        assert_eq!(req.version, "HTTP/1.1");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.body, "");
    }

    #[test]
    fn parse_post_request_with_body() {
        let raw = "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 11\r\n\r\nhello world";
        let req = parse_http_request(raw).expect("request should parse");

        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/echo");
        assert_eq!(req.body, "hello world");
    }

    #[test]
    fn reject_invalid_request_line() {
        let raw = "BROKEN\r\nHost: localhost\r\n\r\n";
        assert!(parse_http_request(raw).is_err());
    }
}

fn parse_headers<'a>(
    lines: impl Iterator<Item = &'a str>,
) -> Result<Vec<(String, String)>, &'static str> {
    let mut headers = Vec::new();

    for line in lines {
        let trimmed = line.trim_end_matches('\r');
        if trimmed.is_empty() {
            break;
        }

        let (name, value) = trimmed.split_once(':').ok_or("invalid header")?;
        headers.push((name.trim().to_string(), value.trim().to_string()));
    }

    Ok(headers)
}
