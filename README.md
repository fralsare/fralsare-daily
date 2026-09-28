# fralsare-daily

A fast, lightweight desktop news reader for Windows and Linux.

fralsare-daily aggregates headlines from public RSS feeds across six fixed
topics, so you get a clean, organized daily briefing without accounts, API
keys, or tracking.

![screenshot placeholder](docs/screenshot.png)

## Topics

| Topic | What you'll get |
| --- | --- |
| AI | Artificial intelligence news and research |
| Geopolitics & Conflict | World news and international affairs |
| Politics & Governance | Domestic politics and government |
| Sports | Match results, scores, and highlights |
| Technology & Business | Tech, startups, and markets |
| Viral & Social Issues | Trending stories and community highlights |

## Features

- **Standalone native window** — built with [Tauri 2](https://tauri.app),
  ~10–20 MB installer, no bundled Chromium
- **Images or text-only** — toggle the display mode; preference is remembered
- **Automatic refresh** — optional 5/15/30/60-minute intervals
- **Fast parallel loading** — 18 feeds fetched concurrently in Rust
- **Click to open** — articles open in your default browser
- **No accounts, no keys** — everything runs on public RSS feeds

## Building from source

### Prerequisites

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 20+
- Platform dependencies:
  - **Linux**: `libwebkit2gtk-4.1-dev build-essential libssl-dev libgtk-3-dev
    libayatana-appindicator3-dev librsvg2-dev`
  - **Windows**: Microsoft C++ Build Tools and WebView2 (preinstalled on
    Windows 10/11)

### Develop

```sh
npm install
npm run tauri dev
```

### Release binaries

```sh
npm install
npm run tauri build
```

Binaries land in `src-tauri/target/release/bundle/`.

## Architecture

```
┌─────────────────────────────────────────────┐
│  Frontend (TypeScript + Vite, no framework) │
│  sidebar · article cards · image/text toggle│
└──────────────────┬──────────────────────────┘
                   │ Tauri command: fetch_topic
┌──────────────────▼──────────────────────────┐
│  Backend (Rust)                             │
│  feeds.rs — 18 RSS sources, 6 topics        │
│  rss.rs   — streaming RSS 2.0/Atom parser   │
│  lib.rs   — parallel fetch, 5-min cache     │
└─────────────────────────────────────────────┘
```

Feeds are fetched in the Rust backend, which sidesteps browser CORS
restrictions. A per-topic in-memory cache (5-minute TTL) avoids hammering
sources during quick navigation.

## Adding or changing feeds

Edit `src-tauri/src/feeds.rs` — each topic maps to a list of
`(name, url)` feed pairs. RSS 2.0 and Atom are both supported.

## License

MIT — see [LICENSE](LICENSE).
