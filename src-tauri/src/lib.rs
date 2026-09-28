mod feeds;
mod rss;
mod sanitize;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use futures::future::join_all;
use rss::{parse_feed, Article};
use serde::Serialize;
use tauri::{Manager, State};

/// How long a fetched topic stays fresh in the in-memory cache.
const CACHE_TTL: Duration = Duration::from_secs(300);
/// Max articles kept per topic.
const MAX_ARTICLES: usize = 60;
/// Per-feed request timeout.
const FEED_TIMEOUT: Duration = Duration::from_secs(15);
/// Article page fetch timeout.
const ARTICLE_TIMEOUT: Duration = Duration::from_secs(20);
/// Cap on article bytes we are willing to parse.
const MAX_ARTICLE_BYTES: usize = 4_000_000;

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct FeedError {
    pub feed: String,
    pub error: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ArticleContent {
    pub url: String,
    pub title: String,
    pub html: String,
    pub content_type: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TopicResult {
    topic: String,
    articles: Vec<Article>,
    errors: Vec<FeedError>,
    fetched_at: DateTime<Utc>,
}

pub struct AppState {
    client: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, TopicResult)>>,
}

async fn fetch_one_feed(
    client: &reqwest::Client,
    feed: &feeds::FeedSpec,
) -> Result<Vec<Article>, String> {
    let resp = client
        .get(feed.url)
        .timeout(FEED_TIMEOUT)
        .header(
            "User-Agent",
            "fralsare-daily/0.1 (+https://github.com/fralsare/fralsare-daily)",
        )
        .header(
            "Accept",
            "application/rss+xml, application/xml, application/atom+xml, text/xml, */*",
        )
        .send()
        .await
        .map_err(|e| format!("{}: {e}", feed.name))?;

    if !resp.status().is_success() {
        return Err(format!("{}: HTTP {}", feed.name, resp.status()));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("{}: {e}", feed.name))?;
    let xml = String::from_utf8_lossy(&bytes);
    let articles = parse_feed(&xml, feed.name);

    if articles.is_empty() {
        Err(format!(
            "{}: no articles parsed (feed format may have changed)",
            feed.name
        ))
    } else {
        Ok(articles)
    }
}

#[tauri::command]
async fn fetch_topic(state: State<'_, AppState>, topic: String) -> Result<TopicResult, String> {
    let feeds = feeds::feeds_for(&topic)
        .ok_or_else(|| format!("unknown topic: {topic}"))?;

    // Serve from cache when fresh.
    {
        let cache = state.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, cached)) = cache.get(&topic) {
            if at.elapsed() < CACHE_TTL {
                return Ok(cached.clone());
            }
        }
    }

    let client = &state.client;
    let results = join_all(feeds.iter().map(|f| fetch_one_feed(client, f))).await;

    let mut articles: Vec<Article> = Vec::new();
    let mut errors: Vec<FeedError> = Vec::new();
    for (feed, res) in feeds.iter().zip(results) {
        match res {
            Ok(items) => articles.extend(items),
            Err(msg) => errors.push(FeedError {
                feed: feed.name.to_string(),
                error: msg,
            }),
        }
    }

    // Newest first; undated stories sink to the bottom. De-dupe by id.
    articles.sort_by(|a, b| b.pub_date.cmp(&a.pub_date));
    articles.dedup_by(|a, b| a.id == b.id);
    articles.truncate(MAX_ARTICLES);

    let result = TopicResult {
        topic: topic.clone(),
        articles,
        errors,
        fetched_at: Utc::now(),
    };

    let mut cache = state.cache.lock().unwrap_or_else(|e| e.into_inner());
    cache.insert(topic, (Instant::now(), result.clone()));
    Ok(result)
}

fn validate_public_url(raw: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(raw)
        .map_err(|e| format!("invalid URL: {e}"))?;
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(format!("unsupported URL scheme: {scheme}"));
    }
    Ok(url.to_string())
}

#[tauri::command]
async fn fetch_article(
    state: State<'_, AppState>,
    url: String,
) -> Result<ArticleContent, String> {
    let url = validate_public_url(&url)?;

    let resp = state
        .client
        .get(&url)
        .timeout(ARTICLE_TIMEOUT)
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36 fralsare-daily/0.1",
        )
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await
        .map_err(|e| format!("fetch failed: {e}"))?;

    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if !status.is_success() {
        return Err(format!("fetch failed: HTTP {status}"));
    }

    let bytes = resp.bytes().await.map_err(|e| format!("fetch failed: {e}"))?;
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_ARTICLE_BYTES)]);
    let ctype = content_type.to_ascii_lowercase();

    if ctype.contains("html") || ctype.is_empty() {
        let title = sanitize::extract_title(&text);
        let sanitized = sanitize::sanitize_html(&text);
        // Keep the rendered article reasonably sized for the webview.
        let cut = sanitized.floor_char_boundary(1_500_000.min(sanitized.len()));
        Ok(ArticleContent {
            url: url.to_string(),
            title,
            html: sanitized[..cut].to_string(),
            content_type,
        })
    } else if ctype.contains("xml") {
        // Some article URLs point at feeds; render the latest entry.
        let articles = parse_feed(&text, &url.to_string());
        match articles.first() {
            Some(first) => {
                let body = format!(
                    "<p class=\"reader-note\">This source publishes as a feed; showing its latest entry.</p><h2>{}</h2>{}",
                    sanitize::escape_html(&first.title),
                    first
                        .summary
                        .as_deref()
                        .map(|s| format!("<p>{}</p>", sanitize::escape_html(s)))
                        .unwrap_or_default()
                );
                Ok(ArticleContent {
                    url: url.to_string(),
                    title: first.title.clone(),
                    html: body,
                    content_type,
                })
            }
            None => Err(format!("feed at {url} contained no entries")),
        }
    } else if ctype.starts_with("text/") {
        Ok(ArticleContent {
            url: url.to_string(),
            title: sanitize::extract_title(&text),
            html: format!("<pre>{}</pre>", sanitize::escape_html(&text)),
            content_type,
        })
    } else {
        Err(format!(
            "unsupported content type: {content_type} — use “Open in browser” instead"
        ))
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            client: reqwest::Client::builder()
                .user_agent("fralsare-daily/0.1")
                .build()
                .expect("failed to build HTTP client"),
            cache: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![fetch_topic, fetch_article])
        .setup(|app| {
            let _window = app
                .get_webview_window("main")
                .expect("main window not found");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
