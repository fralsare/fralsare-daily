//! Topic -> RSS feed mapping. All feeds are public, keyless, and free.
//!
//! To add, remove, or reorder a feed, edit the table below and commit.
//! This is the file that contributors will most likely touch.

pub struct FeedSpec {
    pub name: &'static str,
    pub url: &'static str,
}

pub const TOPICS: &[(&str, &[FeedSpec])] = &[
    (
        "geopolitics",
        &[
            FeedSpec {
                name: "BBC — World",
                url: "https://feeds.bbci.co.uk/news/world/rss.xml",
            },
            FeedSpec {
                name: "Al Jazeera",
                url: "https://www.aljazeera.com/xml/rss/all.xml",
            },
            FeedSpec {
                name: "The Guardian — World",
                url: "https://www.theguardian.com/world/rss",
            },
        ],
    ),
    (
        "politics",
        &[
            FeedSpec {
                name: "NPR — Politics",
                url: "https://feeds.npr.org/1014/rss.xml",
            },
            FeedSpec {
                name: "BBC — Politics",
                url: "https://feeds.bbci.co.uk/news/politics/rss.xml",
            },
            FeedSpec {
                name: "The Guardian — Politics",
                url: "https://www.theguardian.com/politics/rss",
            },
        ],
    ),
    (
        "sports",
        &[
            FeedSpec {
                name: "BBC — Sport",
                url: "https://feeds.bbci.co.uk/sport/rss.xml",
            },
            FeedSpec {
                name: "ESPN — News",
                url: "https://www.espn.com/espn/rss/news",
            },
            FeedSpec {
                name: "The Guardian — Sport",
                url: "https://www.theguardian.com/sport/rss",
            },
        ],
    ),
    (
        "tech_business",
        &[
            FeedSpec {
                name: "TechCrunch",
                url: "https://techcrunch.com/feed/",
            },
            FeedSpec {
                name: "The Verge",
                url: "https://www.theverge.com/rss/index.xml",
            },
            FeedSpec {
                name: "Ars Technica",
                url: "https://feeds.arstechnica.com/arstechnica/index",
            },
        ],
    ),
    (
        "viral",
        &[
            FeedSpec {
                name: "Reddit — r/popular",
                url: "https://www.reddit.com/r/popular/.rss",
            },
            FeedSpec {
                name: "Reddit — r/worldnews",
                url: "https://www.reddit.com/r/worldnews/.rss",
            },
            FeedSpec {
                name: "Hacker News — Front page",
                url: "https://hnrss.org/frontpage",
            },
        ],
    ),
];

pub fn feeds_for(topic: &str) -> Option<&'static [FeedSpec]> {
    TOPICS.iter().find(|(id, _)| *id == topic).map(|(_, feeds)| *feeds)
}
