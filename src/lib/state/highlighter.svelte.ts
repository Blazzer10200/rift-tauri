// Shiki singleton — one shared `HighlighterCore` instance across all
// Markdown renders. Lazy-init on first call; subsequent callers await the
// same in-flight promise. JS engine (no WASM) — saves ~231 KB vs Oniguruma,
// at the cost of ~10% grammar edge-case accuracy that's acceptable for
// display-only highlighting in chat.
//
// Language list mirrors the stack: Rust + TS/JS + Svelte + Bash + JSON +
// TOML + Lua + Python. Theme: github-dark-dimmed — matches the OKLCH dark
// palette better than One Dark / Dracula / Monokai.
//
// Usage from Markdown.svelte:
//   import { highlight } from "../state/highlighter.svelte";
//   const html = await highlight(code, lang);
//
// Returns an HTML string ready for {@html}. DOMPurify sanitizes downstream.

import { createHighlighterCore, type HighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "@shikijs/engine-javascript";

const LANGS = [
  "rust", "typescript", "javascript", "svelte",
  "bash", "json", "toml", "lua", "python",
] as const;

const LANG_ALIASES: Record<string, string> = {
  rs: "rust",
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  py: "python",
  sh: "bash",
  shell: "bash",
  zsh: "bash",
};

const SUPPORTED = new Set<string>(LANGS);

let hlPromise: Promise<HighlighterCore> | null = null;

function ensureHighlighter(): Promise<HighlighterCore> {
  if (hlPromise) return hlPromise;
  hlPromise = createHighlighterCore({
    themes: [import("@shikijs/themes/github-dark-dimmed")],
    langs: [
      import("@shikijs/langs/rust"),
      import("@shikijs/langs/typescript"),
      import("@shikijs/langs/javascript"),
      import("@shikijs/langs/svelte"),
      import("@shikijs/langs/bash"),
      import("@shikijs/langs/json"),
      import("@shikijs/langs/toml"),
      import("@shikijs/langs/lua"),
      import("@shikijs/langs/python"),
    ],
    engine: createJavaScriptRegexEngine(),
  });
  return hlPromise;
}

// Kick off background warmup on module load so the first code-block render
// doesn't pay the full ~200ms init cost.
if (typeof window !== "undefined") {
  void ensureHighlighter();
}

export function normalizeLang(lang: string | undefined): string | null {
  if (!lang) return null;
  const l = lang.toLowerCase().trim();
  if (SUPPORTED.has(l)) return l;
  const alias = LANG_ALIASES[l];
  return alias && SUPPORTED.has(alias) ? alias : null;
}

/** Synchronous highlight using the currently-loaded highlighter. Returns
 *  null if the highlighter hasn't initialized yet — caller should fall back
 *  to plain text rendering on null and re-render once `whenReady()` resolves. */
export function highlightSync(code: string, lang: string | undefined): string | null {
  const norm = normalizeLang(lang);
  if (!norm) return null;
  // Peek at the singleton's resolved state via a non-blocking check.
  // We can't `await` here (sync caller), so we rely on the warmup having
  // happened. If not ready yet, return null.
  const hl = _resolved;
  if (!hl) return null;
  try {
    return hl.codeToHtml(code, {
      lang: norm,
      theme: "github-dark-dimmed",
    });
  } catch {
    return null;
  }
}

/** Await highlighter readiness — for callers that can re-render after the
 *  warmup completes. */
export async function whenReady(): Promise<void> {
  await ensureHighlighter();
}

let _resolved: HighlighterCore | null = null;
void ensureHighlighter().then((hl) => {
  _resolved = hl;
});
