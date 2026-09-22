<script lang="ts">
  // Read-result body — syntax-highlighted (same Shiki singleton as prose code
  // blocks) with a real line-number gutter. Handles BOTH backend shapes: the
  // CLI Read tool's `cat -n`-style gutter (sniffed + stripped into structured
  // numbers) and rift's MCP read_file raw content (numbered from `offset`).
  import { parseReadOutput, OUTPUT_CHAR_CAP } from "./streamModel";
  import { highlightSync, normalizeLang, whenReady } from "$lib/state/highlighter.svelte";
  import { leafName } from "$lib/utils/path";

  let {
    text,
    path = null,
    offset = null,
  }: { text: string; path?: string | null; offset?: number | null } = $props();

  // Shiki cost guard — a 500KB read shouldn't lock the frame on highlight.
  const capped = $derived(text.length > OUTPUT_CHAR_CAP);
  const raw = $derived((capped ? text.slice(0, OUTPUT_CHAR_CAP) : text).replace(/\s+$/, ""));

  const parsed = $derived(parseReadOutput(raw));
  const code = $derived(parsed ? parsed.code : raw);
  const start = $derived(parsed ? parsed.start : (offset ?? 1));
  const lineCount = $derived(code.length === 0 ? 0 : code.split("\n").length);

  const ext = $derived.by(() => {
    if (!path) return null;
    const base = leafName(path);
    const dot = base.lastIndexOf(".");
    return dot > 0 ? base.slice(dot + 1) : null;
  });
  const lang = $derived(normalizeLang(ext ?? undefined));

  let hlReady = $state(false);
  void whenReady().then(() => { hlReady = true; });
  const html = $derived.by(() => {
    if (!hlReady || !lang || code.length === 0) return null;
    return highlightSync(code, lang);
  });
</script>

<div class="rres">
  <div class="rres-head">
    {#if path}<span class="rres-name" title={path}>{leafName(path)}</span><span class="rres-sep">·</span>{/if}
    <span class="tile-label rres-lines">{lineCount} line{lineCount === 1 ? "" : "s"}</span>
    {#if lang}<span class="rres-sep">·</span><span class="tile-label">{lang}</span>{/if}
    {#if capped}<span class="rres-cap">truncated</span>{/if}
  </div>
  <div class="rres-code" style="counter-reset: rrline {start - 1}">
    {#if html}
      {@html html}
    {:else}
      <pre class="rres-plain">{#each code.split("\n") as line, i (i)}<span class="line">{line}
</span>{/each}</pre>
    {/if}
  </div>
</div>

<style>
  .rres { display: flex; flex-direction: column; min-width: 0; }
  /* Section label ("N lines" / lang) is the shared `.tile-label` (app.css) —
     this row is just its flex layout + the filename/truncated-note siblings,
     which keep their own (non-label) weight so the row doesn't read as ALL caps. */
  .rres-head { display: flex; align-items: center; gap: 6px; padding: 5px 11px 2px;
    font-family: var(--font-mono); color: var(--fg-faint); }
  .rres-name { color: var(--fg-subtle); font-weight: 700; letter-spacing: 0.02em; font-size: var(--tile-meta-fs); }
  .rres-sep { opacity: 0.5; font-size: var(--tile-meta-fs); }
  .rres-cap { margin-left: auto; font-weight: 700; font-style: italic; letter-spacing: 0.02em; font-size: var(--tile-meta-fs); }

  /* Shiki emits <pre class="shiki"><code><span class="line">…</span></code></pre>.
     The plain fallback mirrors that shape, so ONE gutter ruleset (CSS counters,
     seeded from the real starting line via inline counter-reset) numbers both.
     The code area is a nested well INSIDE a tile body (never a second card —
     DESIGN: no card-in-a-card) — tile-well fill + the inner (not outer) tile
     radius. */
  .rres-code { max-height: 340px; overflow: auto;
    background: var(--tile-well); border-radius: var(--tile-radius-inner); }
  .rres-code :global(pre) { margin: 0; padding: 7px 0 8px;
    background: transparent !important;
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.55; }
  .rres-code :global(code) { display: block; min-width: 0; }
  .rres-code :global(.line) { display: block; padding-right: 11px; white-space: pre-wrap; word-break: break-word; }
  .rres-code :global(.line)::before {
    counter-increment: rrline; content: counter(rrline);
    display: inline-block; width: 4ch; margin-right: 11px; padding-left: 8px;
    text-align: right; color: var(--fg-faint); opacity: 0.7;
    user-select: none; flex: none;
  }
  .rres-plain { color: var(--fg-2); }
</style>
