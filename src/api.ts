import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { TopicResult } from "./types";

export function fetchTopic(topic: string): Promise<TopicResult> {
  return invoke<TopicResult>("fetch_topic", { topic });
}

/** Open a link in the user's default browser. */
export function openLink(url: string): Promise<void> {
  return openUrl(url);
}
