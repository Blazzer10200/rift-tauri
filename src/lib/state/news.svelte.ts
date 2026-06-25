// "What's new in AI" feed state (Workspace page). Mirrors cliUpdate.svelte.ts:
// fetch-on-launch, throttle to once / 6h, persist to localStorage so the page
// paints from cache instantly then refreshes in the background, bounded retry.
//
// Two tiers (see docs/design/workspace-activity-news.md + assistant/news.rs):
//  - Tier 1 (`items`): deterministic Claude Code release feed (free, no LLM).
//  - Tier 2 (`digest`): opt-in AI summary of recent Anthropic + Claude Code news.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// One Claude Code release, as the backend returns it.
export type NewsItem = {
  source: string;
  version: string;
  published_at: string | null; // ISO-8601 or null
  bullets: string[];
  maintenance: boolean;
  url: string;
};

// One AI-digest entry, AFTER frontend re-validation (normalizeNews).
export type DigestItem = {
  title: string;
  summary: string;
  date: string | null;
  url: string;
  tag: "model" | "claude-code" | "api" | "company";
};

type Persisted = {
  items: NewsItem[];
  checkedAt: number;
  // Tier-2 digest is cached too so it survives a reload, but it never auto-fetches.
  digest?: DigestItem[];
  digestAt?: number;
};

const LS_KEY = "rift.ai-news.v1";
const STALE_MS = 6 * 60 * 60 * 1000; // 6h — same cadence as the CLI-update check.
const RETRY_DELAYS_MS = [20_000, 60_000, 180_000];

type Status = "idle" | "checking" | "ok" | "error";
type DigestStatus = "idle" | "loading" | "ok" | "error";
type DigestStage = "" | "spawned" | "thinking" | "writing";

const ALLOWED_TAGS = new Set(["model", "claude-code", "api", "company"]);

/** Re-validate the AI digest JSON — never trust the model's raw output. Clamps
 *  summary length, allow-lists tag, drops items missing title/url or with a
 *  non-http(s) url. Mirrors AI Health's `normalizeApply` doctrine. */
export function normalizeNews(raw: string): DigestItem[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error("The summary came back in an unexpected format.");
  }
  const obj = parsed as { items?: unknown };
  const items = Array.isArray(obj?.items) ? obj.items : [];
  const out: DigestItem[] = [];
  const seen = new Set<string>();
  for (const it of items) {
    const o = it as Record<string, unknown>;
    const title = typeof o?.title === "string" ? o.title.trim() : "";
    const url = typeof o?.url === "string" ? o.url.trim() : "";
    if (!title || !/^https?:\/\//i.test(url)) continue;
    // Dedupe by url — the model sometimes repeats the same source; a duplicate
    // key would otherwise crash the keyed {#each} that renders these.
    if (seen.has(url)) continue;
    seen.add(url);
    const summaryRaw = typeof o?.summary === "string" ? o.summary.trim() : "";
    const tag = typeof o?.tag === "string" && ALLOWED_TAGS.has(o.tag)
      ? (o.tag as DigestItem["tag"])
      : "company";
    const date = typeof o?.date === "string" && o.date.trim() ? o.date.trim() : null;
    out.push({
      title: title.slice(0, 160),
      summary: summaryRaw.slice(0, 200),
      url,
      tag,
      date,
    });
    if (out.length >= 6) break;
  }
  return out;
}

class NewsStore {
  // ── Tier 1 ──
  items = $state<NewsItem[]>([]);
  checkedAt = $state<number | null>(null);
  status = $state<Status>("idle");
  error = $state<string | null>(null);

  // ── Tier 2 (opt-in) ──
  digest = $state<DigestItem[]>([]);
  digestAt = $state<number | null>(null);
  digestStatus = $state<DigestStatus>("idle");
  digestError = $state<string | null>(null);
  digestStage = $state<DigestStage>("");

  private _retries = 0;
  private _retryTimer: ReturnType<typeof setTimeout> | null = null;
  private _unlistenStage: (() => void) | null = null;

  constructor() {
    try {
      const raw = typeof localStorage !== "undefined" ? localStorage.getItem(LS_KEY) : null;
      if (raw) {
        const p = JSON.parse(raw) as Persisted;
        if (Array.isArray(p.items)) this.items = p.items;
        if (typeof p.checkedAt === "number") this.checkedAt = p.checkedAt;
        if (Array.isArray(p.digest)) this.digest = p.digest;
        if (typeof p.digestAt === "number") this.digestAt = p.digestAt;
      }
    } catch {
      /* corrupt cache — ignore, a fresh fetch rebuilds it */
    }
  }

  private persist() {
    try {
      const p: Persisted = {
        items: this.items,
        checkedAt: this.checkedAt ?? 0,
        digest: this.digest,
        digestAt: this.digestAt ?? 0,
      };
      localStorage.setItem(LS_KEY, JSON.stringify(p));
    } catch {
      /* private mode / quota — non-fatal */
    }
  }

  /** Tier 1 — fetch the release feed. Skips if checked within STALE_MS unless `force`. */
  async maybeFetch(force = false): Promise<void> {
    if (this.status === "checking") return;
    if (!force && this.checkedAt && Date.now() - this.checkedAt < STALE_MS) return;
    this.status = "checking";
    this.error = null;
    try {
      const items = await invoke<NewsItem[]>("assistant_fetch_ai_news");
      this.items = items;
      this.checkedAt = Date.now();
      this.status = "ok";
      this.persist();
      this._retries = 0;
      if (this._retryTimer) { clearTimeout(this._retryTimer); this._retryTimer = null; }
    } catch (e) {
      this.status = "error";
      this.error = e instanceof Error ? e.message : String(e);
      // Bounded auto-retry so a transient failure doesn't leave the feed dead
      // until the next app restart.
      const delay = RETRY_DELAYS_MS[this._retries];
      if (delay != null && this._retryTimer == null) {
        this._retries++;
        this._retryTimer = setTimeout(() => {
          this._retryTimer = null;
          void this.maybeFetch(true);
        }, delay);
      }
    }
  }

  /** Tier 2 — opt-in AI digest. User-triggered only; never auto-fires. */
  async summarize(): Promise<void> {
    if (this.digestStatus === "loading") return;
    this.digestStatus = "loading";
    this.digestError = null;
    this.digestStage = "";
    // Subscribe to backend progress stages for the animated loading card.
    if (!this._unlistenStage) {
      this._unlistenStage = await listen<{ stage: DigestStage }>(
        "assistant://news-progress",
        (e) => { this.digestStage = e.payload.stage; },
      );
    }
    try {
      const raw = await invoke<string>("assistant_summarize_ai_news");
      this.digest = normalizeNews(raw);
      this.digestAt = Date.now();
      this.digestStatus = "ok";
      this.persist();
    } catch (e) {
      this.digestStatus = "error";
      this.digestError = e instanceof Error ? e.message : String(e);
    } finally {
      this.digestStage = "";
    }
  }
}

export const news = new NewsStore();
