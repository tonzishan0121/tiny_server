/// Parsed HTTP request data used by the router and handlers.
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// Minimal HTTP response builder for status, headers, and body bytes.
pub struct HttpResponse {
    status: &'static str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl HttpResponse {
    /// Builds a text response with a `Content-Type` header.
    pub fn new(status: &'static str, content_type: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), content_type.to_string())],
            body: body.into().into_bytes(),
        }
    }

    /// Builds a response for raw bytes such as HTML files or images.
    pub fn bytes(status: &'static str, content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), content_type.to_string())],
            body,
        }
    }

    /// Adds one response header.
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// Serializes the response into HTTP bytes and controls the connection header.
    pub fn to_http_bytes(&self, keep_alive: bool) -> Vec<u8> {
        let connection = if keep_alive { "keep-alive" } else { "close" };
        let mut head = format!("HTTP/1.1 {}\r\n", self.status);

        for (name, value) in &self.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }

        head.push_str(&format!(
            "Content-Length: {}\r\nConnection: {connection}\r\n\r\n",
            self.body.len()
        ));

        let mut response = head.into_bytes();
        response.extend_from_slice(&self.body);
        response
    }

    pub fn status(&self) -> &str {
        self.status
    }
}

/// Parses a raw HTTP request into the minimal fields tiny_server uses.
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

impl HttpRequest {
    /// Finds a request header by name without caring about case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Decides whether this request asks to keep the TCP connection open.
    pub fn wants_keep_alive(&self) -> bool {
        let connection = self.header("Connection").unwrap_or("");
        if connection.eq_ignore_ascii_case("close") {
            return false;
        }

        self.version == "HTTP/1.1" || connection.eq_ignore_ascii_case("keep-alive")
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
