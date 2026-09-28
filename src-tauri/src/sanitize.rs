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
    // Site chrome: headers, nav menus, footers, and sidebars are noise in
    // the reader (menus, related-stories lists, legal text).
    "nav",
    "header",
    "footer",
    "aside",
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

fn tag_blocks_re(tag: &str) -> &'static Regex {
    static RE: OnceLock<Vec<Regex>> = OnceLock::new();
    const TAGS: &[&str] = &["article", "main"];
    let all = RE.get_or_init(|| {
        TAGS
            .iter()
            .map(|t| {
                Regex::new(&format!(
                    r#"(?isx)<{t}\b[^>]*>(.*?)</{t}\s*>"#
                ))
                .expect("valid regex")
            })
            .collect()
    });
    &all[TAGS.iter().position(|x| *x == tag).unwrap()]
}

static TAG_RE: OnceLock<Regex> = OnceLock::new();

/// Approximate word count of an HTML fragment (tags removed).
fn word_count(html: &str) -> usize {
    let re = TAG_RE.get_or_init(|| Regex::new(r#"(?s)<[^>]*>"#).expect("valid regex"));
    re.replace_all(html, " ").split_whitespace().count()
}

/// Keep only the article body out of a full page (best effort), so the
/// in-app reader shows the story instead of the whole site. Prefers the
/// `<article>` block with the most text, then `<main>`, else the whole
/// document. Blocks shorter than a threshold are ignored so stub pages
/// (paywall teasers etc.) fall through to the broader container.
const MIN_ARTICLE_WORDS: usize = 100;

pub fn extract_main_content(html: &str) -> String {
    for (tag, re) in [("article", tag_blocks_re("article")), ("main", tag_blocks_re("main"))] {
        let best = re
            .captures_iter(html)
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str())
            .filter(|b| word_count(b) >= MIN_ARTICLE_WORDS)
            .max_by_key(|b| word_count(b));
        if let Some(block) = best {
            return format!("<{tag}>{block}</{tag}>");
        }
    }
    html.to_string()
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
    fn strips_site_chrome_tags() {
        let input =
            "<header><nav><a href=\"/\">Home</a></nav></header><p>body</p><aside>side</aside><footer>© 2026</footer>";
        let out = sanitize_html(input);
        assert!(!out.contains("<nav"));
        assert!(!out.contains("Home"));
        assert!(!out.contains("side"));
        assert!(!out.contains("© 2026"));
        assert!(out.contains("<p>body</p>"));
    }

    #[test]
    fn extract_main_content_prefers_article() {
        let body: String = vec!["word"; 150].join(" ");
        let input = format!(
            "<html><body>{}<main><article><h1>T</h1><p>{body}</p></article><aside>related</aside></main>{}<footer>© 2026</footer></body></html>",
            "<header><nav><a>Menu</a></nav></header>",
            "",
        );
        let out = extract_main_content(&input);
        assert!(out.starts_with("<article>"));
        assert!(out.contains(&body));
        assert!(!out.contains("Menu"));
        assert!(!out.contains("related"));
        assert!(!out.contains("© 2026"));
    }

    #[test]
    fn extract_main_content_falls_back_to_main() {
        let body: String = vec!["text"; 120].join(" ");
        let input = format!(
            "<html><body><header>nav</header><main><p>{body}</p></main><footer>f</footer></body></html>"
        );
        let out = extract_main_content(&input);
        assert!(out.starts_with("<main>"));
        assert!(!out.contains("<footer>"));
    }

    #[test]
    fn extract_main_content_ignores_short_article_and_keeps_document() {
        // Short <article> stub (e.g. paywall teaser) must not win over the
        // longer <main> content.
        let body: String = vec!["text"; 120].join(" ");
        let input = format!(
            "<html><body><article><p>short teaser</p></article><main><p>{body}</p></main></body></html>"
        );
        let out = extract_main_content(&input);
        assert!(out.starts_with("<main>"));
    }

    #[test]
    fn extract_main_content_passthrough_without_containers() {
        let body: String = vec!["plain"; 130].join(" ");
        let input = format!("<html><body><p>{body}</p></body></html>");
        let out = extract_main_content(&input);
        assert_eq!(out, input);
    }

    #[test]
    fn extracts_title() {
        assert_eq!(
            extract_title("<html><head><title>  My   Story </title></head></html>"),
            "My Story"
        );
    }
}
