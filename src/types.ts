export interface Article {
  id: string;
  title: string;
  link: string;
  source: string;
  image: string | null;
  summary: string | null;
  /** RFC3339 timestamp or null */
  pub_date: string | null;
}

export interface FeedError {
  feed: string;
  error: string;
}

export interface TopicResult {
  topic: string;
  articles: Article[];
  errors: FeedError[];
  /** RFC3339 timestamp of when the backend fetched */
  fetched_at: string;
}

export interface TopicDef {
  id: string;
  label: string;
  icon: string;
}

export const TOPICS: TopicDef[] = [
  { id: "ai", label: "AI", icon: "🤖" },
  { id: "geopolitics", label: "Geopolitics & Conflict", icon: "🌍" },
  { id: "politics", label: "Politics & Governance", icon: "🏛️" },
  { id: "sports", label: "Sports", icon: "⚽" },
  { id: "tech_business", label: "Technology & Business", icon: "💻" },
  { id: "viral", label: "Viral & Social Issues", icon: "🔥" },
];

export type DisplayMode = "images" | "text";
