//! Minimal HTML sanitizer for in-app article rendering.
//!
//! We strip anything that can execute or navigate the webview: scripts,
//! styles, embedded objects, inline event handlers, and javascript: URLs.
//! This is intentionally conservative — article text, images, and links
//! remain, everything else is dropped.

use regex::Regex;
use std::sync::OnceLock;

/// Tags removed together with their entire contents.
const BLOCK_TAGS: &[&str] = &[
    "script",
    "style",
    "noscript",
    "iframe",
    "frame",
    "object",
    "embed",
    "form",
    "svg",
    "canvas",
    "template",
    "dialog",
    // `<base>` hijacks relative URLs; `<meta http-equiv>` can redirect.
    "base",
    "meta",
];

fn block_re(tag: &str) -> &'static Regex {
    static RE: OnceLock<Vec<Regex>> = OnceLock::new();
    let all = RE.get_or_init(|| {
        BLOCK_TAGS
            .iter()
            .map(|t| {
                Regex::new(&format!(
                    r#"(?isx)<{t}\b[^>]*>.*?</{t}\s*>|<{t}\b[^>]*/?>"#
                ))
                .expect("valid regex")
            })
            .collect()
    });
    &all[BLOCK_TAGS.iter().position(|x| *x == tag).unwrap()]
}

fn on_attr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)\son[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#)
            .expect("valid regex")
    })
}

/// Inline style attributes can pin elements over the whole window
/// (position:fixed lightboxes that can no longer be dismissed because the
/// page scripts were stripped). Remove them wholesale.
fn style_attr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?i)\s+style\s*=\s*(\"[^\"]*\"|'[^']*'|[^\s>]+)"#)
            .expect("valid regex")
    })
}

fn js_url_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"(?i)(?:href|src)\s*=\s*(?:"[^"]*javascript:[^"]*"|'[^']*javascript:[^']*')"#,
        )
            .expect("valid regex")
    })
}

/// Strip executable and dangerous constructs from raw HTML.
pub fn sanitize_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    out.push_str(html);
    for tag in BLOCK_TAGS {
        out = block_re(tag).replace_all(&out, "").into_owned();
    }
    out = on_attr_re().replace_all(&out, "").into_owned();
    out = style_attr_re().replace_all(&out, "").into_owned();
    out = js_url_re().replace_all(&out, "").into_owned();
    out
}

/// Pull the document title out of raw HTML (best effort).
pub fn extract_title(html: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?is)<title[^>]*>(.*?)</title\s*>"#).expect("valid regex")
    });
    if let Some(m) = re.captures(html) {
        let text = m
            .get(1)
            .map(|x| x.as_str())
            .unwrap_or("")
            .replace(|c: char| c.is_whitespace(), " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !text.is_empty() {
            return truncate(&text, 220);
        }
    }
    String::new()
}

/// Escape a string for safe embedding in HTML.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_scripts_and_handlers() {
        let input = r#"<div onclick='alert(1)'><script>evil()</script><p onclick="x">Hi</p><iframe src="https://e.com"></iframe></div>"#;
        let out = sanitize_html(input);
        assert!(!out.contains("script"));
        assert!(!out.contains("iframe"));
        assert!(!out.contains("onclick"));
        assert!(out.contains("<p>Hi</p>"));
    }

    #[test]
    fn neutralizes_javascript_urls() {
        let input = r#"<a href="javascript:alert(1)">x</a><a href='javascript:void(0)'>y</a>"#;
        let out = sanitize_html(input);
        assert!(!out.contains("javascript:"));
        assert!(out.contains("<a"));
        assert_eq!(out.matches("<a").count(), 2);
    }

    #[test]
    fn keeps_normal_content() {
        let input = r#"<p>Hello <a href="https://a.co/b">link</a> <img src="https://a.co/i.png" alt="x"></p>"#;
        let out = sanitize_html(input);
        assert!(out.contains("<a href=\"https://a.co/b\">link</a>"));
        assert!(out.contains("<img"));
    }

    #[test]
    fn strips_inline_styles_base_and_meta() {
        let input = r#"<head><base href="https://evil.example/"><meta http-equiv="refresh" content="0; url=https://evil.example/"></head><body><div style="position:fixed;left:0;top:0;width:100vw;height:100vh;background:white">x</div></body>"#;
        let out = sanitize_html(input);
        assert!(!out.contains("<base"));
        assert!(!out.contains("<meta"));
        assert!(!out.contains("position"));
        assert!(out.contains("<div>x</div>"));
    }

    #[test]
    fn extracts_title() {
        assert_eq!(
            extract_title("<html><head><title>  My   Story </title></head></html>"),
            "My Story"
        );
    }
}
