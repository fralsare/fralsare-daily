//! Minimal RSS 2.0 / Atom parser built on quick-xml streaming.
//!
//! Handles the common cases we need: <item> (RSS) and <entry> (Atom),
//! titles, links, pub dates, descriptions/content, and images from
//! media:content, media:thumbnail, enclosure, or the first <img> in the
//! HTML payload.

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
#[derive(Debug)]
pub struct Article {
    pub id: String,
    pub title: String,
    pub link: String,
    pub source: String,
    pub image: Option<String>,
    pub summary: Option<String>,
    pub pub_date: Option<DateTime<Utc>>,
}

#[derive(Debug)]
enum Capture {
    Title,
    Summary,
    Content,
    Date,
    Link,
}

fn new_article(source: &str) -> Article {
    Article {
        id: String::new(),
        title: String::new(),
        link: String::new(),
        source: source.to_string(),
        image: None,
        summary: None,
        pub_date: None,
    }
}

fn finish(article: &mut Article) {
    if article.title.is_empty() {
        article.title = article.summary.take().unwrap_or_default();
    }
    if article.title.contains('<') {
        article.title = strip_html(&article.title);
    }
    if article.link.is_empty() {
        article.id = article.title.clone();
    } else {
        article.id = article.link.clone();
    }
}

pub fn parse_feed(xml: &str, source: &str) -> Vec<Article> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut articles: Vec<Article> = Vec::new();
    let mut item: Option<Article> = None;
    let mut capturing: Option<Capture> = None;
    let mut text = String::new();

    loop {
        match reader.read_event() {
            // Self-closing tags (<link .../>, <media:content .../>) arrive
            // as Event::Empty, which also carries a BytesStart.
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_ascii_lowercase();

                if name == "item" || name == "entry" {
                    item = Some(new_article(source));
                    continue;
                }
                // Nested markup inside a non-CDATA content:encoded arrives
                // as real start/end events; fold it back into the raw text.
                if name != "content:encoded" && name != "content"
                    && matches!(capturing, Some(Capture::Content))
                    && item.is_some()
                    && !KNOWN_TAGS.contains(&name.as_str())
                {
                    let mut tag = format!("<{name}");
                    for result in e.attributes() {
                        if let Ok(a) = result {
                            let key =
                                std::str::from_utf8(a.key.as_ref()).unwrap_or("?");
                            if let Ok(value) = a.unescape_value() {
                                tag.push_str(&format!(" {key}=\"{value}\""));
                            }
                        }
                    }
                    text.push_str(&tag);
                    text.push('>');
                    continue;
                }

                let Some(article) = item.as_mut() else {
                    continue;
                };

                match name.as_str() {
                    "title" => {
                        if article.title.is_empty() {
                            capturing = Some(Capture::Title);
                            text.clear();
                        }
                    }
                    "description" | "summary" => {
                        if article.summary.is_none() {
                            capturing = Some(Capture::Summary);
                            text.clear();
                        }
                    }
                    "content:encoded" | "content" => {
                        capturing = Some(Capture::Content);
                        text.clear();
                    }
                    "media:content" | "media:thumbnail" => {
                        if let Some(url) = attr(e, "url").or_else(|| attr(e, "href")) {
                            if article.image.is_none() && is_image_url(&url) {
                                article.image = Some(url);
                            }
                        }
                    }
                    "pubdate" | "published" | "updated" | "date" => {
                        capturing = Some(Capture::Date);
                        text.clear();
                    }
                    "link" => {
                        // Atom: <link href="..." rel="alternate"/>
                        if let Some(href) = attr(e, "href") {
                            let rel = attr(e, "rel").unwrap_or_default();
                            if (rel.is_empty() || rel == "alternate") && article.link.is_empty() {
                                article.link = href;
                            }
                        } else {
                            // RSS: <link>http://...</link>
                            if article.link.is_empty() {
                                capturing = Some(Capture::Link);
                                text.clear();
                            }
                        }
                    }
                    "enclosure" => {
                        if let Some(url) = attr(e, "url").or_else(|| attr(e, "href")) {
                            if article.image.is_none() && is_image_url(&url) {
                                article.image = Some(url);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => push_unescaped(e.unescape(), &mut text),
            Ok(Event::CData(ref e)) => {
                // CData content is raw text (no entity escaping).
                if let Some(raw) = std::str::from_utf8(e.as_ref()).ok() {
                    if capturing.is_some() {
                        text.push_str(raw);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_ascii_lowercase();

                if name == "item" || name == "entry" {
                    capturing = None;
                    if let Some(mut article) = item.take() {
                        finish(&mut article);
                        if !article.id.is_empty() {
                            articles.push(article);
                        }
                    }
                    continue;
                }

                // Closing tag of nested markup inside content:encoded.
                if matches!(capturing, Some(Capture::Content))
                    && item.is_some()
                    && !KNOWN_TAGS.contains(&name.as_str())
                {
                    text.push_str(&format!("</{name}>"));
                    continue;
                }

                let Some(field) = capturing.take() else {
                    continue;
                };
                let value = text.trim().to_string();
                text.clear();
                let Some(article) = item.as_mut() else {
                    continue;
                };

                match field {
                    Capture::Title => {
                        if article.title.is_empty() && !value.is_empty() {
                            article.title = value;
                        }
                    }
                    Capture::Summary => {
                        if article.summary.is_none() && !value.is_empty() {
                            article.summary = Some(strip_html(&value));
                        }
                    }
                    Capture::Content => {
                        if article.image.is_none() {
                            article.image = first_image_url(&value);
                        }
                        if article.summary.is_none() && !value.is_empty() {
                            article.summary = Some(strip_html(&value));
                        }
                    }
                    Capture::Date => {
                        article.pub_date = parse_date(&value);
                    }
                    Capture::Link => {
                        if article.link.is_empty() && !value.is_empty() {
                            article.link = value;
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("feed parse error ({source}): {e}");
                break;
            }
        }
    }

    if let Some(mut article) = item.take() {
        finish(&mut article);
        if !article.id.is_empty() {
            articles.push(article);
        }
    }
    articles
}

fn push_unescaped(res: Result<std::borrow::Cow<'_, str>, quick_xml::Error>, text: &mut String) {
    if let Ok(unescaped) = res {
        text.push_str(&unescaped);
    }
}

/// Element names that mark the start/end of an item-level field.
const KNOWN_TAGS: &[&str] = &[
    "item",
    "entry",
    "title",
    "description",
    "summary",
    "content",
    "content:encoded",
    "pubdate",
    "published",
    "updated",
    "date",
    "link",
    "enclosure",
    "media:content",
    "media:thumbnail",
];

fn attr(e: &quick_xml::events::BytesStart, name: &str) -> Option<String> {
    for result in e.attributes() {
        let Ok(a) = result else {
            continue;
        };
        if a.key.as_ref() == name.as_bytes() {
            return a.unescape_value().ok().map(|v| v.to_string());
        }
    }
    None
}

fn is_image_url(url: &str) -> bool {
    const EXTS: &[&str] = &[".jpg", ".jpeg", ".png", ".webp", ".gif", ".avif"];
    let lower = url
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    EXTS.iter().any(|ext| lower.ends_with(ext))
}

/// Find the first <img src="..."> URL in an HTML fragment.
fn first_image_url(html: &str) -> Option<String> {
    let mut idx = 0;
    while let Some(pos) = html[idx..].find("<img") {
        let start = idx + pos;
        let end = html[start..].find('>')?;
        let tag = &html[start..start + end + 1];
        let lower = tag.to_ascii_lowercase();
        if let Some(src_pos) = lower.find("src=") {
            let rest = tag[src_pos + 4..].trim_start();
            let q = rest.chars().next()?;
            if q == '"' || q == '\'' {
                // rest[0] is the opening quote; find the closing one.
                let closing = rest[1..].find(q).unwrap_or(0);
                let url = rest[1..1 + closing].trim().to_string();
                if url.starts_with("http://") || url.starts_with("https://") {
                    return Some(url);
                }
            }
        }
        idx = start + end + 1;
    }
    None
}

/// Strip HTML tags, decode a handful of common entities, collapse
/// whitespace, and truncate to ~300 chars at a word boundary.
fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for c in input.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&rsquo;", "'")
        .replace("&lsquo;", "'")
        .replace("&rdquo;", "\"")
        .replace("&ldquo;", "\"")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…")
        .replace("&nbsp;", " ")
        .replace('\u{a0}', " ");
    let out: String = out
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > 300 {
        let mut end = 300;
        while end > 0 && !out.is_char_boundary(end) {
            end -= 1;
        }
        if let Some(sp) = out[..end].rfind(' ') {
            end = sp;
        }
        format!("{}…", &out[..end])
    } else {
        out
    }
}

#[cfg(test)]
mod tests {


    use super::*;


    const RSS: &str = r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test Channel</title>
    <link>https://example.com</link>
    <item>
      <title>First story</title>
      <link>https://example.com/1</link>
      <description><![CDATA[<p>Body <b>text</b> &amp; more.</p> <img src="https://example.com/a.jpg">]]></description>
      <pubDate>Mon, 01 Sep 2025 10:00:00 GMT</pubDate>
      <media:content url="https://example.com/m.webp"/>
    </item>
    <item>
      <title>Second story</title>
      <link>https://example.com/2</link>
      <description>&lt;p&gt;Escaped description&lt;/p&gt;</description>
    </item>
    <item>
      <title>No image here</title>
      <link>https://example.com/3</link>
      <description>Fallback img test</description>
      <content:encoded><figure><img alt="x" src="https://example.com/b.png" /></figure></content:encoded>
    </item>
  </channel>
</rss>"#;

    const ATOM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Atom Test</title>
  <entry>
    <title type="html">Atom story</title>
    <link rel="alternate" href="https://example.org/atom-1"/>
    <published>2025-09-01T10:00:00Z</published>
    <summary>Short summary.</summary>
  </entry>
</feed>"#;

    #[test]
    fn parses_rss_items() {
        let items = parse_feed(RSS, "Test Feed");
        assert_eq!(items.len(), 3);

        let first = &items[0];
        assert_eq!(first.title, "First story");
        assert_eq!(first.link, "https://example.com/1");
        assert_eq!(first.source, "Test Feed");
        assert!(first.pub_date.is_some());
        // media:content wins over the <img> in the body
        assert_eq!(first.image.as_deref(), Some("https://example.com/m.webp"));
        assert!(first.summary.as_deref().unwrap().contains("Body text & more."));

        // description with entity-escaped HTML becomes plain text
        assert!(items[1].summary.as_deref().unwrap().contains("Escaped description"));

        // content:encoded fallback finds the <img>
        assert_eq!(items[2].image.as_deref(), Some("https://example.com/b.png"));
    }

    #[test]
    fn parses_atom_entries() {
        let items = parse_feed(ATOM, "Atom Feed");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Atom story");
        assert_eq!(items[0].link, "https://example.org/atom-1");
        assert_eq!(items[0].id, "https://example.org/atom-1");
        assert!(items[0].pub_date.is_some());
    }

    #[test]
    fn strips_html_and_truncates() {
        let out = strip_html("<p>Hello&nbsp;world</p>");
        assert_eq!(out, "Hello world");

        let long = format!("<p>{}</p>", "word ".repeat(100));
        let out = strip_html(&long);
        assert!(out.len() <= 303, "len was {}", out.len());
        assert!(out.ends_with('…'));
    }

    #[test]
    fn finds_first_image_url() {
        assert_eq!(
            first_image_url("<div><img src='https://a.co/x.jpg'></div>"),
            Some("https://a.co/x.jpg".to_string())
        );
        // relative / protocol-relative URLs are rejected
        assert_eq!(first_image_url(r#"<img src="/local.png">"#), None);
        assert_eq!(first_image_url(r#"<img src="//cdn.x/y.png">"#), None);
        // first <img> wins
        assert_eq!(
            first_image_url(r#"<img src="https://a/1.jpg"><img src="https://a/2.jpg">"#),
            Some("https://a/1.jpg".to_string())
        );
    }

    #[test]
    fn parses_known_date_formats() {
        assert!(parse_date("2025-09-01T10:00:00Z").is_some());
        assert!(parse_date("Mon, 01 Sep 2025 10:00:00 GMT").is_some());
        assert!(parse_date("2025-09-01 10:00:00").is_some());
        assert!(parse_date("not a date").is_none());
    }

    #[test]
    fn garbage_input_yields_no_articles() {
        assert!(parse_feed("this is not xml at all", "X").is_empty());
        assert!(parse_feed("", "X").is_empty());
    }
}

fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();

    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    if let Ok(d) = DateTime::parse_from_rfc2822(s) {
        return Some(d.with_timezone(&Utc));
    }
    let naive = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .ok()?;
    let ts = naive.and_utc().timestamp();
    DateTime::from_timestamp(ts, 0).map(|d| d.with_timezone(&Utc))
}
