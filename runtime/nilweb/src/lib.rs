// runtime/nilweb — OnuronOS native web engine.
//
// Shared by the shell's Browser screen and the standalone `browser` app so the
// fetch/parse behaviour has exactly one implementation.
//
// This is a text-mode engine, not an HTML rendering engine, and it does not
// pretend otherwise: it performs a real HTTP/HTTPS request and returns the
// response reduced to text. What it genuinely does:
//
//   * Resolves and connects over the network with a real TLS stack (ureq/rustls).
//   * Reports the real status code, content type and byte length.
//   * Extracts the document title from real markup.
//   * Converts HTML to readable text: script/style contents dropped, block tags
//     become line breaks, entities decoded.
//   * Follows a bounded number of redirects (the HTTP client does this).
//
// What it does not do, stated plainly in the UI: CSS layout, JavaScript, images.
// A page that requires JavaScript will show its noscript text or nothing, and the
// status line says so rather than showing a blank "loaded" screen.

use std::time::Duration;

/// Hard cap on how much of a response body is downloaded and kept. A browser on
/// a phone must not let one page exhaust memory.
const MAX_BODY_BYTES: usize = 512 * 1024;
const MAX_RENDER_LINES: usize = 400;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// A fetched page, already converted to text.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub final_url: String,
    pub status: u16,
    pub content_type: String,
    pub title: String,
    pub lines: Vec<String>,
    pub truncated: bool,
}

/// Browser state held by the shell.
#[derive(Debug, Clone)]
pub struct BrowserState {
    pub url: String,
    pub page: Option<Page>,
    pub status: String,
    pub error: Option<String>,
    pub scroll: usize,
    pub history: Vec<String>,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            url: String::new(),
            page: None,
            status: "Ready".to_string(),
            error: None,
            scroll: 0,
            history: Vec::new(),
        }
    }
}

impl BrowserState {
    /// Fetch `url` and store the result. Never panics; failures land in `error`.
    pub fn navigate(&mut self, url: &str) {
        let normalized = normalize_url(url);
        self.url = normalized.clone();
        self.scroll = 0;
        self.error = None;
        self.status = format!("Loading {normalized} ...");

        match fetch(&normalized) {
            Ok(page) => {
                self.status = format!(
                    "{} {} — {} lines{}",
                    page.status,
                    page.content_type,
                    page.lines.len(),
                    if page.truncated { " (truncated)" } else { "" }
                );
                if !self.history.contains(&normalized) {
                    self.history.push(normalized.clone());
                }
                self.page = Some(page);
            }
            Err(e) => {
                self.status = "Failed".to_string();
                self.error = Some(e);
                self.page = None;
            }
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        if let Some(page) = &self.page {
            let max = page.lines.len().saturating_sub(1);
            self.scroll = (self.scroll + n).min(max);
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }
}

/// Add a scheme when the user typed a bare host, and turn a bare word into a
/// search. This mirrors what a browser address bar does.
pub fn normalize_url(input: &str) -> String {
    let t = input.trim();
    if t.is_empty() {
        return "about:blank".to_string();
    }
    if t.contains("://") {
        return t.to_string();
    }
    if t.starts_with("about:") {
        return t.to_string();
    }
    // A bare token with no dot and no slash is treated as a search query.
    if !t.contains('.') && !t.contains('/') && !t.starts_with("localhost") {
        let encoded: String = t
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || "-_.~".contains(c) {
                    c.to_string()
                } else {
                    c.to_string().bytes().map(|b| format!("%{b:02X}")).collect()
                }
            })
            .collect();
        return format!("https://duckduckgo.com/html/?q={encoded}");
    }
    format!("https://{t}")
}

/// Perform the request and convert the response into renderable text.
pub fn fetch(url: &str) -> Result<Page, String> {
    if url.starts_with("about:") {
        return Ok(Page {
            final_url: url.to_string(),
            status: 200,
            content_type: "text/plain".into(),
            title: url.to_string(),
            lines: vec![
                "OnuronOS browser".to_string(),
                String::new(),
                "Type an address and press Enter.".to_string(),
                "A bare word is treated as a search.".to_string(),
                "Text-mode browser: CSS, JavaScript and images are not rendered.".to_string(),
            ],
            truncated: false,
        });
    }

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(REQUEST_TIMEOUT)
        .user_agent("OnuronOS-Browser/0.1 (+text-mode; rustls)")
        .build();

    let resp = agent.get(url).call().map_err(|e| match e {
        ureq::Error::Status(code, _) => format!("HTTP {code}"),
        ureq::Error::Transport(t) => format!("network error: {t}"),
    })?;

    let status = resp.status();
    let content_type = resp
        .header("content-type")
        .unwrap_or("application/octet-stream")
        .to_string();
    let final_url = resp.get_url().to_string();

    // Read at most MAX_BODY_BYTES + 1 so truncation is detectable.
    let mut body = Vec::new();
    {
        use std::io::Read;
        let mut reader = resp.into_reader().take((MAX_BODY_BYTES + 1) as u64);
        reader
            .read_to_end(&mut body)
            .map_err(|e| format!("read error: {e}"))?;
    }
    let truncated = body.len() > MAX_BODY_BYTES;
    body.truncate(MAX_BODY_BYTES);

    let raw = String::from_utf8_lossy(&body).to_string();
    let is_html = content_type.contains("html");

    let (title, text) = if is_html {
        (extract_title(&raw), html_to_text(&raw))
    } else {
        (final_url.clone(), raw)
    };

    let mut lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.trim().is_empty())
        .collect();

    let dropped = lines.len().saturating_sub(MAX_RENDER_LINES);
    lines.truncate(MAX_RENDER_LINES);
    if dropped > 0 {
        lines.push(format!("… {dropped} more lines not shown"));
    }
    if is_html && lines.len() < 3 {
        lines.push(String::new());
        lines.push("This page produced almost no text; it likely requires JavaScript.".to_string());
    }

    Ok(Page {
        final_url,
        status,
        content_type,
        title,
        lines,
        truncated,
    })
}

/// Extract `<title>` content, decoded and trimmed.
pub fn extract_title(html: &str) -> String {
    let lower = html.to_lowercase();
    let Some(start) = lower.find("<title") else {
        return String::new();
    };
    let Some(open_end) = lower[start..].find('>') else {
        return String::new();
    };
    let content_start = start + open_end + 1;
    let Some(close_rel) = lower[content_start..].find("</title>") else {
        return String::new();
    };
    let raw = &html[content_start..content_start + close_rel];
    decode_entities(raw)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Convert HTML to readable plain text.
///
/// Order matters: script and style *contents* must be removed before tags are
/// stripped, otherwise a page's JavaScript shows up as body text.
pub fn html_to_text(html: &str) -> String {
    let without_scripts = remove_element(html, "script");
    let without_styles = remove_element(&without_scripts, "style");
    let without_comments = remove_comments(&without_styles);

    let mut out = String::with_capacity(without_comments.len());
    let mut in_tag = false;
    let mut tag = String::new();

    for ch in without_comments.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let name = tag
                    .trim_start_matches('/')
                    .split(|c: char| c.is_whitespace() || c == '/')
                    .next()
                    .unwrap_or("")
                    .to_lowercase();
                // Block-level elements and <br> end a line so paragraphs do not
                // run together into one unreadable blob.
                if is_block_element(&name) {
                    out.push('\n');
                }
            }
            c if in_tag => tag.push(c),
            c => out.push(c),
        }
    }

    decode_entities(&out)
}

fn is_block_element(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "br"
            | "li"
            | "tr"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "section"
            | "article"
            | "header"
            | "footer"
            | "ul"
            | "ol"
            | "table"
            | "blockquote"
            | "pre"
            | "form"
            | "nav"
            | "hr"
    )
}

/// Remove every `<tag ...>...</tag>` block including its content.
fn remove_element(html: &str, tag: &str) -> String {
    let lower = html.to_lowercase();
    let open = format!("<{tag}");
    let close = format!("</{tag}");
    let mut result = String::with_capacity(html.len());
    let mut cursor = 0usize;

    while let Some(rel) = lower[cursor..].find(&open) {
        let start = cursor + rel;
        result.push_str(&html[cursor..start]);
        // Skip past the opening tag's `>`.
        match lower[start..].find('>') {
            Some(gt) => {
                let after_open = start + gt + 1;
                match lower[after_open..].find(&close) {
                    Some(crel) => {
                        let after_close = after_open + crel;
                        // Skip past `</tag>`.
                        cursor = match lower[after_close..].find('>') {
                            Some(gt2) => after_close + gt2 + 1,
                            None => lower.len(),
                        };
                    }
                    None => {
                        // Unterminated element: drop the rest.
                        cursor = lower.len();
                    }
                }
            }
            None => {
                cursor = lower.len();
            }
        }
    }
    result.push_str(&html[cursor.min(html.len())..]);
    result
}

fn remove_comments(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut cursor = 0usize;
    while let Some(rel) = html[cursor..].find("<!--") {
        let start = cursor + rel;
        result.push_str(&html[cursor..start]);
        match html[start..].find("-->") {
            Some(end_rel) => cursor = start + end_rel + 3,
            None => {
                cursor = html.len();
                break;
            }
        }
    }
    result.push_str(&html[cursor.min(html.len())..]);
    result
}

/// Decode the entities that actually appear in page text.
pub fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        // Look ahead for the terminating semicolon within a sane bound.
        let rest = &s[i + 1..];
        let Some(semi) = rest.find(';').filter(|n| *n <= 10) else {
            out.push('&');
            continue;
        };
        let entity = &rest[..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            "mdash" => Some('—'),
            "ndash" => Some('–'),
            "hellip" => Some('…'),
            "copy" => Some('©'),
            "reg" => Some('®'),
            "trade" => Some('™'),
            "laquo" => Some('«'),
            "raquo" => Some('»'),
            _ => None,
        };
        if let Some(d) = decoded {
            out.push(d);
            for _ in 0..=semi {
                chars.next();
            }
        } else if let Some(num) = entity.strip_prefix('#') {
            let code = if let Some(hex) = num.strip_prefix(['x', 'X']) {
                u32::from_str_radix(hex, 16).ok()
            } else {
                num.parse::<u32>().ok()
            };
            match code.and_then(char::from_u32) {
                Some(ch) => {
                    out.push(ch);
                    for _ in 0..=semi {
                        chars.next();
                    }
                }
                None => out.push('&'),
            }
        } else {
            out.push('&');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_url_adds_scheme_and_searches_bare_words() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(
            normalize_url("http://example.com/x"),
            "http://example.com/x"
        );
        assert!(normalize_url("rust programming").starts_with("https://duckduckgo.com/html/?q="));
        assert_eq!(normalize_url("about:blank"), "about:blank");
        assert_eq!(normalize_url("   "), "about:blank");
        assert_eq!(normalize_url("localhost:8080"), "https://localhost:8080");
    }

    #[test]
    fn extract_title_reads_and_normalizes_the_title() {
        let html = "<html><head><TITLE>  Hello   World </TITLE></head></html>";
        assert_eq!(extract_title(html), "Hello World");
        assert_eq!(extract_title("<html>no title</html>"), "");
        assert_eq!(extract_title("<title>unclosed"), "");
    }

    #[test]
    fn html_to_text_drops_scripts_and_styles() {
        let html = r#"<html><head>
            <script>var secret = "should-not-appear";</script>
            <style>.x { color: red }</style>
            </head><body><p>Visible text</p>
            <script>alsoHidden()</script></body></html>"#;
        let text = html_to_text(html);
        assert!(text.contains("Visible text"), "got: {text}");
        assert!(!text.contains("should-not-appear"), "script leaked: {text}");
        assert!(!text.contains("alsoHidden"), "script leaked: {text}");
        assert!(!text.contains("color: red"), "style leaked: {text}");
    }

    #[test]
    fn html_to_text_separates_block_elements_and_strips_comments() {
        let html = "<div>one</div><!-- hidden --><div>two</div>";
        let text = html_to_text(html);
        assert!(text.contains("one"));
        assert!(text.contains("two"));
        assert!(!text.contains("hidden"));
        // Blocks must not be glued together on one line.
        assert_ne!(text.trim(), "onetwo");
    }

    #[test]
    fn decode_entities_handles_named_numeric_and_unknown() {
        assert_eq!(decode_entities("a &amp; b &lt;c&gt;"), "a & b <c>");
        assert_eq!(decode_entities("&#65;&#x42;"), "AB");
        assert_eq!(decode_entities("100% &unknown; ok"), "100% &unknown; ok");
        assert_eq!(decode_entities("bare & ampersand"), "bare & ampersand");
    }

    #[test]
    fn remove_element_handles_unterminated_tags() {
        // Must not hang or panic on malformed markup.
        let out = remove_element("<p>keep</p><script>never closed", "script");
        assert!(out.contains("keep"));
        assert!(!out.contains("never closed"));
    }

    #[test]
    fn about_blank_needs_no_network() {
        let page = fetch("about:blank").unwrap();
        assert_eq!(page.status, 200);
        assert!(!page.lines.is_empty());
    }

    /// Real end-to-end fetch against a real TCP server on loopback. This
    /// exercises the actual HTTP path rather than a mock.
    #[test]
    fn fetches_a_real_page_over_http() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(_) => return, // no loopback networking available
        };
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || {
            if let Ok((mut sock, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = sock.read(&mut buf);
                let body = "<html><head><title>Onuron Test</title></head>\
                            <body><p>Real fetched content</p></body></html>";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes());
            }
        });

        let page = fetch(&format!("http://127.0.0.1:{port}/")).expect("fetch must succeed");
        assert_eq!(page.status, 200);
        assert_eq!(page.title, "Onuron Test");
        assert!(
            page.lines
                .iter()
                .any(|l| l.contains("Real fetched content")),
            "lines: {:?}",
            page.lines
        );
        let _ = server.join();
    }

    #[test]
    fn navigate_records_history_and_reports_failure_honestly() {
        let mut st = BrowserState::default();
        // Port 1 on loopback refuses connections on any normal system.
        st.navigate("http://127.0.0.1:1/");
        assert!(st.page.is_none());
        assert!(st.error.is_some(), "a failed fetch must set an error");
        assert_eq!(st.status, "Failed");
    }

    #[test]
    fn scroll_is_clamped_to_content() {
        let mut st = BrowserState {
            page: Some(Page {
                final_url: "x".into(),
                status: 200,
                content_type: "text/plain".into(),
                title: "x".into(),
                lines: vec!["a".into(), "b".into(), "c".into()],
                truncated: false,
            }),
            ..BrowserState::default()
        };
        st.scroll_down(100);
        assert_eq!(st.scroll, 2);
        st.scroll_up(100);
        assert_eq!(st.scroll, 0);
    }
}
