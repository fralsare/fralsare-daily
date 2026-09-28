import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { TopicResult } from "./types";

export function fetchTopic(topic: string): Promise<TopicResult> {
  return invoke<TopicResult>("fetch_topic", { topic });
}

export interface ArticleContent {
  url: string;
  title: string;
  html: string;
  content_type: string;
}

/** Fetch an article page and return sanitized, in-app-renderable HTML. */
export function fetchArticle(url: string): Promise<ArticleContent> {
  return invoke<ArticleContent>("fetch_article", { url });
}

/** Open a link in the user's default browser (opt-in only). */
export function openLink(url: string): Promise<void> {
  return openUrl(url);
}
