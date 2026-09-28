import { fetchArticle, fetchTopic, openLink } from "./api";
import type { ArticleContent } from "./api";
import {
  TOPICS,
  type Article,
  type DisplayMode,
  type TopicResult,
} from "./types";
import "./styles.css";

const LS_MODE = "fralsare-daily.mode";
const LS_INTERVAL = "fralsare-daily.interval-min";
const LS_TOPIC = "fralsare-daily.topic";

const INTERVAL_OPTIONS = [1, 5, 15, 30];

interface State {
  topicId: string;
  mode: DisplayMode;
  intervalMin: number;
  result: TopicResult | null;
  loading: boolean;
  error: string | null;
  timer: ReturnType<typeof setTimeout> | null;
  ticking: ReturnType<typeof setInterval> | null;
  /** Articles opened in-app, most recent last. Empty = feed view. */
  articleStack: Article[];
  articleContent: ArticleContent | null;
  articleLoading: boolean;
  articleError: string | null;
}

const state: State = {
  topicId: localStorage.getItem(LS_TOPIC) ?? TOPICS[0].id,
  mode: (localStorage.getItem(LS_MODE) as DisplayMode) ?? "images",
  intervalMin: Number(localStorage.getItem(LS_INTERVAL)) || 5,
  result: null,
  loading: false,
  error: null,
  timer: null,
  ticking: null,
  articleStack: [],
  articleContent: null,
  articleLoading: false,
  articleError: null,
};

if (!TOPICS.some((t) => t.id === state.topicId)) {
  state.topicId = TOPICS[0].id;
}

// ---------------------------------------------------------------- DOM refs

const app = document.getElementById("app")!;

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  cls?: string,
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
}

// ------------------------------------------------------------- time helpers

function relativeTime(iso: string | null): string {
  if (!iso) return "";
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return "";
  const sec = Math.max(0, (Date.now() - then) / 1000);
  if (sec < 60) return "just now";
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  if (day < 7) return `${day}d ago`;
  return new Date(then).toLocaleDateString();
}

// ------------------------------------------------------------------- render

function render(): void {
  app.replaceChildren();
  app.classList.toggle("text-mode", state.mode === "text");

  const layout = el("div", "layout");
  const main =
    state.articleStack.length > 0 ? renderArticleView() : renderMain();
  layout.append(renderSidebar(), main);
  app.append(layout);
}

function renderSidebar(): HTMLElement {
  const aside = el("aside", "sidebar");

  const brand = el("div", "brand");
  brand.append(el("span", "brand-icon", "📰"), el("span", "brand-name", "fralsare-daily"));
  aside.append(brand);

  const nav = el("nav", "topics");
  for (const t of TOPICS) {
    const btn = el("button", `topic${t.id === state.topicId ? " active" : ""}`);
    btn.append(el("span", "topic-icon", t.icon), el("span", "topic-label", t.label));
    btn.addEventListener("click", () => {
      if (state.topicId === t.id) return;
      state.topicId = t.id;
      localStorage.setItem(LS_TOPIC, t.id);
      render();
      loadTopic();
    });
    nav.append(btn);
  }
  aside.append(nav);

  // Settings footer
  const settings = el("div", "settings");

  const modeRow = el("div", "setting-row");
  modeRow.append(el("span", "setting-label", "Display"));
  const modeSeg = el("div", "segmented");
  for (const [value, label] of [
    ["images", "With images"],
    ["text", "Text only"],
  ] as const) {
    const b = el("button", `seg${state.mode === value ? " active" : ""}`, label);
    b.addEventListener("click", () => {
      state.mode = value;
      localStorage.setItem(LS_MODE, value);
      render();
    });
    modeSeg.append(b);
  }
  modeRow.append(modeSeg);
  settings.append(modeRow);

  const refreshRow = el("div", "setting-row");
  refreshRow.append(el("span", "setting-label", "Auto-refresh"));
  const select = el("select", "interval-select");
  for (const min of INTERVAL_OPTIONS) {
    const opt = el("option", "", `${min} min`);
    opt.value = String(min);
    if (min === state.intervalMin) opt.selected = true;
    select.append(opt);
  }
  select.addEventListener("change", () => {
    state.intervalMin = Number(select.value);
    localStorage.setItem(LS_INTERVAL, select.value);
    scheduleAutoRefresh();
  });
  refreshRow.append(select);
  settings.append(refreshRow);

  aside.append(settings);
  return aside;
}

function renderMain(): HTMLElement {
  const main = el("main", "main");
  const topic = TOPICS.find((t) => t.id === state.topicId)!;

  const header = el("header", "main-header");
  const title = el("h1", "main-title");
  title.append(el("span", "", topic.icon), el("span", "", ` ${topic.label}`));
  const meta = el("div", "main-meta");
  updateMeta(meta);
  const refreshBtn = el("button", "refresh-btn", "↻ Refresh");
  refreshBtn.addEventListener("click", () => loadTopic(true));
  header.append(title, meta, refreshBtn);
  main.append(header);

  const content = el("div", "content");
  main.append(content);

  if (state.loading && !state.result) {
    content.append(el("div", "state-box", "Loading stories…"));
    return main;
  }
  if (state.error && !state.result) {
    content.append(
      el("div", "state-box error", `Couldn't load news: ${state.error}`),
    );
    return main;
  }
  if (!state.result) return main;

  if (state.result.errors.length > 0) {
    const banner = el("div", "error-banner");
    banner.append(
      el(
        "span",
        "error-banner-text",
        `Some feeds failed: ${state.result.errors
          .map((e) => e.feed)
          .join(", ")}`,
      ),
    );
    main.append(banner);
  }

  if (state.result.articles.length === 0) {
    content.append(el("div", "state-box", "No stories right now — try refreshing."));
    return main;
  }

  if (state.mode === "images") {
    const grid = el("div", "grid");
    for (const a of state.result.articles) grid.append(renderCard(a));
    content.append(grid);
  } else {
    const list = el("div", "rows");
    for (const a of state.result.articles) list.append(renderRow(a));
    content.append(list);
  }
  return main;
}

/* ------------------------------------------------------------ article */

const BLOCKED_TAGS = new Set([
  "SCRIPT",
  "STYLE",
  "NOSCRIPT",
  "IFRAME",
  "FRAME",
  "OBJECT",
  "EMBED",
  "FORM",
  "SVG",
  "CANVAS",
  "TEMPLATE",
  "DIALOG",
]);

/**
 * Second line of defense for article HTML (Rust already sanitizes).
 * Parses the HTML into a detached document, strips executable elements,
 * inline event handlers, and javascript: URLs, and returns the cleaned
 * body — appended as a live DOM node, never re-serialized.
 */
function sanitizeDocument(html: string): HTMLElement {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const walk = (root: Element): void => {
    for (const child of Array.from(root.children)) {
      if (BLOCKED_TAGS.has(child.tagName)) {
        child.remove();
        continue;
      }
      for (const attr of Array.from(child.attributes)) {
        const name = attr.name.toLowerCase();
        if (name.startsWith("on")) child.removeAttribute(attr.name);
        if (
          (name === "href" || name === "src") &&
          /^\s*javascript:/i.test(attr.value)
        ) {
          child.removeAttribute(attr.name);
        }
      }
      walk(child);
    }
  };
  walk(doc.documentElement);
  return doc.body;
}

function openArticle(article: Article): void {
  state.articleStack.push(article);
  state.articleContent = null;
  state.articleError = null;
  state.articleLoading = true;
  render();
  void loadArticle(article);
}

async function loadArticle(article: Article): Promise<void> {
  const isStillCurrent = () => {
    const top = state.articleStack[state.articleStack.length - 1];
    return top !== undefined && top.link === article.link;
  };
  try {
    const content = await fetchArticle(article.link);
    if (!isStillCurrent()) return;
    state.articleContent = content;
    if (content.title) {
      state.articleStack[state.articleStack.length - 1].title = content.title;
    }
  } catch (e) {
    if (!isStillCurrent()) return;
    state.articleError = String(e);
  } finally {
    if (isStillCurrent()) {
      state.articleLoading = false;
      render();
    }
  }
}

function backFromArticle(): void {
  state.articleStack.pop();
  state.articleContent = null;
  state.articleLoading = false;
  state.articleError = null;
  render();
}

function renderArticleView(): HTMLElement {
  const article = state.articleStack[state.articleStack.length - 1];
  const main = el("section", "main");

  const header = el("header", "article-header");
  const back = el("button", "back-btn");
  back.textContent = "← Back";
  back.addEventListener("click", backFromArticle);
  const heading = el("h1", "article-heading", article.title);
  const meta = el("div", "article-meta");
  meta.textContent = `${article.source} · ${relativeTime(article.pub_date)}`;
  const browser = el("button", "browser-btn");
  browser.textContent = "Open in browser";
  browser.addEventListener("click", () =>
    openLink(article.link).catch(() => undefined),
  );
  header.append(back, heading, meta, browser);
  main.append(header);

  const content = el("div", "article-content");
  if (state.articleLoading) {
    content.append(el("div", "state-box", "Loading article…"));
  } else if (state.articleError) {
    content.append(
      el(
        "div",
        "state-box error",
        `Couldn't load this story in-app. Use “Open in browser” above to read it.`,
      ),
    );
  } else if (state.articleContent) {
    const reader = el("article", "reader");
    // Transfers the sanitized nodes into the live document; no re-serialization.
    reader.append(sanitizeDocument(state.articleContent.html));
    // Links inside the article stay in-app.
    reader.addEventListener("click", (ev) => {
      const anchor = (ev.target as HTMLElement).closest("a");
      if (!anchor) return;
      const href = anchor.getAttribute("href");
      if (!href) return;
      ev.preventDefault();
      let abs: URL;
      try {
        abs = new URL(href, location.href);
      } catch {
        return;
      }
      if (abs.protocol !== "http:" && abs.protocol !== "https:") return;
      if (abs.href === article.link) return;
      openArticle({
        id: abs.href,
        title: anchor.textContent?.trim() || "Story",
        link: abs.href,
        source: article.source,
        image: null,
        summary: null,
        pub_date: null,
      });
    });
    content.append(reader);
  }
  main.append(content);
  return main;
}

function renderCard(a: Article): HTMLElement {
  const card = el("article", "card");
  if (a.image) {
    const fig = el("div", "card-img");
    const img = el("img", "card-image");
    img.src = a.image;
    img.alt = "";
    img.loading = "lazy";
    img.addEventListener("error", () => fig.remove());
    fig.append(img);
    card.append(fig);
  }
  const body = el("div", "card-body");
  const title = el("h2", "card-title", a.title);
  body.append(title);
  if (a.summary) body.append(el("p", "card-summary", a.summary));
  const foot = el("div", "card-foot");
  foot.append(
    el("span", "card-source", a.source),
    el("span", "card-time", relativeTime(a.pub_date)),
  );
  body.append(foot);
  card.append(body);
  card.addEventListener("click", () => openArticle(a));
  return card;
}

function renderRow(a: Article): HTMLElement {
  const row = el("article", "row");
  const mainCol = el("div", "row-main");
  mainCol.append(el("h2", "row-title", a.title));
  if (a.summary) mainCol.append(el("p", "row-summary", a.summary));
  const foot = el("div", "card-foot");
  foot.append(
    el("span", "card-source", a.source),
    el("span", "card-time", relativeTime(a.pub_date)),
  );
  mainCol.append(foot);
  row.append(mainCol);
  row.addEventListener("click", () => openArticle(a));
  return row;
}

function updateMeta(meta: HTMLElement): void {
  meta.replaceChildren();
  const parts: string[] = [];
  if (state.result) {
    parts.push(`${state.result.articles.length} stories`);
    const t = Date.parse(state.result.fetched_at);
    if (!Number.isNaN(t)) parts.push(`updated ${relativeTime(state.result.fetched_at)}`);
  }
  if (state.loading) parts.push("refreshing…");
  meta.append(el("span", "", parts.join(" · ")));
}

// ------------------------------------------------------------------- data

let requestSeq = 0;

async function loadTopic(force = false): Promise<void> {
  const seq = ++requestSeq;
  state.loading = true;
  state.error = null;
  render();

  try {
    const result = await fetchTopic(state.topicId);
    if (seq !== requestSeq) return; // a newer request superseded this one
    state.result = result;
  } catch (e) {
    if (seq !== requestSeq) return;
    state.error = String(e);
  } finally {
    if (seq === requestSeq) {
      state.loading = false;
      if (!force) scheduleAutoRefresh();
      render();
    }
  }
}

function scheduleAutoRefresh(): void {
  if (state.timer) clearTimeout(state.timer);
  state.timer = setTimeout(() => loadTopic(), state.intervalMin * 60_000);
}

// Keep relative timestamps fresh.
state.ticking = setInterval(() => {
  if (state.result) {
    const meta = document.querySelector<HTMLElement>(".main-meta");
    if (meta) updateMeta(meta);
  }
}, 30_000);

render();
loadTopic();
