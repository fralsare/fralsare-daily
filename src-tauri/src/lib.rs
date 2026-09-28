mod feeds;
mod rss;

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

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct FeedError {
    pub feed: String,
    pub error: String,
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
        .invoke_handler(tauri::generate_handler![fetch_topic])
        .setup(|app| {
            let _window = app
                .get_webview_window("main")
                .expect("main window not found");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
