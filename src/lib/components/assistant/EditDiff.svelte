<script lang="ts">
  // Unified line-diff of an Edit tool block's old_string / new_string (del lines
  // then add lines in one column — NOT side-by-side; see the render at ~L219).
  // Used both inline in chat bubbles (default) and inside the dock op-card
  // (`compact`). When `input` lacks old_string / new_string, renders nothing
  // and the parent can fall back to raw JSON.

  import { untrack, onDestroy } from "svelte";
  import { slide } from "svelte/transition";
  import { diffArrays } from "diff";
  import DOMPurify from "dompurify";
  import { FileText, ChevronRight, CornerDownLeft, Copy, Check } from "lucide-svelte";
  import { highlightSync, whenReady } from "../../state/highlighter.svelte";
  import FilePathMenu from "./FilePathMenu.svelte";

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  import { tooltip } from "$lib/actions/tooltip";
  let {
    input,
    compact = false,
    defaultExpanded = false,
    hideHead = false,
    maxLines = null,
  }: {
    input: Record<string, unknown>;
    compact?: boolean;
    defaultExpanded?: boolean;
    // Suppress the breadcrumb header + chrome and force the body open (e.g.
    // EnhanceBar's inline before/after preview).
    hideHead?: boolean;
    // #34: cap rendered diff rows; the rest hides behind a "Show N more lines"
    // button. null = unlimited (chat-bubble default).
    maxLines?: number | null;
  } = $props();

  type DiffPair =
    | { kind: "ctx";  left: string; right: string }
    | { kind: "del";  left: string; right: null }
    | { kind: "add";  left: null;   right: string }
    | { kind: "mod";  left: string; right: string }
    | { kind: "meta"; text: string }
    | { kind: "gap";  lines: number };

  const pairs = $derived.by<DiffPair[] | null>(() => {
    // Write (new-file creation) has `content`, not old/new_string — render it
    // as an all-additions diff so the file body is actually shown, not hidden
    // behind a bare "N chars" chip. Only treat as write when content exists
    // AND new_string is absent.
    const isWrite = typeof input.content === "string" && !input.new_string;
    const oldStr = isWrite ? "" : (typeof input.old_string === "string" ? input.old_string : null);
    const newStr = isWrite ? (input.content as string) : (typeof input.new_string === "string" ? input.new_string : null);
    if (oldStr === null || newStr === null) return null;
    const out: DiffPair[] = [];
    if (input.replace_all === true) out.push({ kind: "meta", text: "replace_all: true" });

    // A write (new-file creation) has no prior content. `"".split("\n")` yields
    // `[""]` — one empty line — which diffs as a phantom removed-blank row (a
    // lone `−` gutter with no line number) before the real additions. A created
    // file has nothing to remove, so emit every line as a pure addition and skip
    // the diff entirely. (replace_all can't apply to a write, so the meta above
    // is a no-op here.)
    if (isWrite) {
      for (const line of newStr.split("\n")) out.push({ kind: "add", left: null, right: line });
      return out;
    }

    const oldLines = oldStr.split("\n");
    const newLines = newStr.split("\n");
    const chunks = diffArrays(oldLines, newLines);

    for (let i = 0; i < chunks.length; i++) {
      const c = chunks[i];
      if (!c.added && !c.removed) {
        for (const line of c.value) out.push({ kind: "ctx", left: line, right: line });
        continue;
      }
      if (c.removed) {
        const next = chunks[i + 1];
        if (next?.added) {
          const dels = c.value;
          const adds = next.value;
          const n = Math.max(dels.length, adds.length);
          for (let k = 0; k < n; k++) {
            const l = k < dels.length ? dels[k] : null;
            const r = k < adds.length ? adds[k] : null;
            if (l !== null && r !== null) out.push({ kind: "mod", left: l, right: r });
            else if (l !== null) out.push({ kind: "del", left: l, right: null });
            else if (r !== null) out.push({ kind: "add", left: null, right: r });
          }
          i++;
        } else {
          for (const line of c.value) out.push({ kind: "del", left: line, right: null });
        }
      } else if (c.added) {
        for (const line of c.value) out.push({ kind: "add", left: null, right: line });
      }
    }
    return out;
  });

  // Collapse runs of blank context lines so non-adjacent edits don't inflate
  // the chip with literal source whitespace. A single blank ctx renders as a
  // thin spacer row; 2+ consecutive blanks render as a `···` elision marker
  // showing the count. del/add/mod blanks are preserved (they're meaningful).
  const compactPairs = $derived.by<DiffPair[]>(() => {
    if (!pairs) return [];
    const out: DiffPair[] = [];
    let blankRun = 0;
    const flush = () => {
      if (blankRun > 0) out.push({ kind: "gap", lines: blankRun });
      blankRun = 0;
    };
    for (const p of pairs) {
      if (p.kind === "ctx" && p.left === "" && p.right === "") {
        blankRun++;
        continue;
      }
      flush();
      out.push(p);
    }
    flush();
    return out;
  });

  const filePath = $derived(typeof input.file_path === "string" ? input.file_path : null);

  // Breadcrumb header — dir/name split + language badge. Long dirs collapse to
  // their last two segments so the filename stays visible; full path goes in
  // the tooltip.
  const baseName = $derived.by(() => {
    if (!filePath) return "";
    const norm = filePath.replace(/\\/g, "/").replace(/\/$/, "");
    return norm.split("/").pop() ?? norm;
  });
  const dirLabel = $derived.by(() => {
    if (!filePath) return "";
    const norm = filePath.replace(/\\/g, "/");
    const idx = norm.lastIndexOf("/");
    if (idx < 0) return "";
    const segs = norm.slice(0, idx).split("/").filter(Boolean);
    if (segs.length <= 2) return segs.length ? segs.join("/") + "/" : "";
    return "…/" + segs.slice(-2).join("/") + "/";
  });
  const lang = $derived.by(() => {
    const dot = baseName.lastIndexOf(".");
    if (dot < 0) return "";
    const ext = baseName.slice(dot + 1).toUpperCase();
    return ext.length > 0 && ext.length <= 8 ? ext : "";
  });

  // Per-line syntax highlighting — reuse the shared shiki core (same themes as
  // fenced code blocks) so diff code is colorized instead of monochrome. Falls
  // back to plain text for unsupported langs or until shiki finishes loading.
  let shikiReady = $state(false);
  whenReady().then(() => { shikiReady = true; }).catch((e) => console.error("shiki init failed (diff stays monochrome):", e));
  const EXT_LANG: Record<string, string> = {
    rs: "rust", ts: "typescript", tsx: "typescript", mts: "typescript", cts: "typescript",
    js: "javascript", jsx: "javascript", mjs: "javascript", cjs: "javascript",
    svelte: "svelte", sh: "bash", bash: "bash", zsh: "bash",
    json: "json", jsonc: "json", toml: "toml", lua: "lua", py: "python", pyi: "python",
  };
  const langId = $derived(EXT_LANG[lang.toLowerCase()] ?? null);
  // Cache keyed on `${langId}:${text}` — stable across shikiReady flips since
  // langId and code lines are immutable once the diff is built.
  const hlCache = new Map<string, string | null>();
  function hl(text: string): string | null {
    if (!shikiReady || !langId || text.length === 0) return null;
    const key = `${langId}:${text}`;
    if (hlCache.has(key)) return hlCache.get(key)!;
    const html = highlightSync(text, langId);
    if (!html) { hlCache.set(key, null); return null; }
    // Extract just the inner tokens of shiki's single `.line` span.
    const m = html.match(/<span class="line">([\s\S]*?)<\/span><\/code>/);
    const result = m ? DOMPurify.sanitize(m[1], { ALLOWED_TAGS: ["span"], ALLOWED_ATTR: ["style", "class"] }) : null;
    hlCache.set(key, result);
    return result;
  }

  const counts = $derived.by(() => {
    if (!pairs) return { adds: 0, dels: 0 };
    let adds = 0; let dels = 0;
    for (const p of pairs) {
      if (p.kind === "add") adds++;
      else if (p.kind === "del") dels++;
      else if (p.kind === "mod") { adds++; dels++; }
    }
    return { adds, dels };
  });

  // Small edits auto-expand so the change is visible at a glance — hiding a
  // 1-line edit behind a chevron is worse than just showing it. Large diffs
  // (>SMALL_DIFF changed lines) stay collapsed so they don't eat the column;
  // the header's +N -M already summarizes them. Compact (dock) + explicit
  // defaultExpanded always open.
  // Initial-seed only: prop reads are intentionally non-reactive — once the
  // user clicks the chevron their choice is sticky and shouldn't snap back
  // when a parent re-renders the same block w/ identical props.
  const SMALL_DIFF = 12;
  let expanded = $state<boolean>(
    untrack(() => {
      if (compact || defaultExpanded || hideHead) return true;
      // Compute changed-line count directly from the raw strings (don't read
      // the `counts` $derived here — reading a derived inside a $state
      // initializer is order-fragile). Cheap line-set diff is enough to
      // decide auto-expand; the precise +N/-M still comes from `counts`.
      const isWrite = typeof input.content === "string" && !input.new_string;
      const oldStr = isWrite ? "" : (typeof input.old_string === "string" ? input.old_string : "");
      const newStr = isWrite ? (input.content as string) : (typeof input.new_string === "string" ? input.new_string : "");
      // Skip diffing huge blocks — default collapsed to avoid mount-time cost.
      if (oldStr.length + newStr.length > 200_000) return false;
      const _chunks = diffArrays(oldStr.split("\n"), newStr.split("\n"));
      const changed = _chunks.reduce((n, c) => n + (c.added || c.removed ? c.value.length : 0), 0);
      return changed <= SMALL_DIFF;
    }),
  );

  // Unified diff — flatten the side-by-side pairs into one chronological line
  // list. A `mod` pair expands to its del line then its add line. Line numbers
  // track the NEW file (relative, 1-based — the Edit tool gives no absolute
  // offset); del lines carry no number. Blank-context runs (`gap`) still count
  // toward the line tally so subsequent numbers stay consistent.
  type UnifiedLine =
    | { kind: "ctx" | "add"; num: number; text: string; html: string | null }
    | { kind: "del"; num: null; text: string; html: string | null }
    | { kind: "meta"; text: string }
    | { kind: "gap"; lines: number };
  // D3: build the line STRUCTURE without highlighting (depends only on
  // compactPairs), then layer html in a separate derived. A shikiReady flip
  // then re-runs only the cheap html map, not this O(N) structural walk.
  const unifiedStruct = $derived.by<UnifiedLine[]>(() => {
    const out: UnifiedLine[] = [];
    let ln = 0;
    for (const p of compactPairs) {
      if (p.kind === "meta") { out.push({ kind: "meta", text: p.text }); continue; }
      if (p.kind === "gap") { out.push({ kind: "gap", lines: p.lines }); ln += p.lines; continue; }
      if (p.kind === "ctx") { ln++; out.push({ kind: "ctx", num: ln, text: p.left, html: null }); continue; }
      if (p.kind === "del") { out.push({ kind: "del", num: null, text: p.left, html: null }); continue; }
      if (p.kind === "add") { ln++; out.push({ kind: "add", num: ln, text: p.right, html: null }); continue; }
      // mod — del then add.
      out.push({ kind: "del", num: null, text: p.left, html: null });
      ln++; out.push({ kind: "add", num: ln, text: p.right, html: null });
    }
    return out;
  });
  const unifiedLines = $derived.by<UnifiedLine[]>(() =>
    unifiedStruct.map((l) =>
      l.kind === "ctx" || l.kind === "add" || l.kind === "del"
        ? { ...l, html: hl(l.text) }
        : l,
    ),
  );

  // #34: line cap — render only the first `maxLines` rows until the user asks
  // for the rest. +24 hysteresis so a diff barely over the cap isn't truncated
  // for a handful of lines.
  let showAllLines = $state(false);
  const lineCapped = $derived(
    maxLines !== null && !showAllLines && unifiedLines.length > maxLines + 24,
  );
  const visibleLines = $derived(lineCapped ? unifiedLines.slice(0, maxLines!) : unifiedLines);
  const hiddenLineCount = $derived(unifiedLines.length - visibleLines.length);

  function toggleExpanded(e: MouseEvent) {
    if (compact) return;
    e.stopPropagation();
    expanded = !expanded;
  }

  // Copy the resulting (new) content to the clipboard — the most useful payload
  // when reusing an edit. Briefly flips to a "Copied" confirmation.
  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  function copyDiff(e: MouseEvent) {
    e.stopPropagation();
    const text = typeof input.new_string === "string" ? input.new_string
      : typeof input.content === "string" ? input.content : "";
    navigator.clipboard?.writeText(text).catch(() => {});
    copied = true;
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => { copied = false; }, 1400);
  }
  onDestroy(() => { if (copyTimer) clearTimeout(copyTimer); });

  // Open the file-actions menu (open in VS Code / default app, reveal, copy)
  // anchored to the crumb button.
  let menuPos = $state<{ x: number; y: number } | null>(null);
  function openFile(e: MouseEvent) {
    e.stopPropagation();
    if (!filePath) return;
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menuPos = { x: r.left, y: r.bottom + 4 };
  }
</script>

{#if pairs}
  <div class="edit-diff" class:compact class:collapsed={!expanded} class:embedded={hideHead}>
    {#if !compact && !hideHead && filePath}
      <div class="edit-head">
        <button
          type="button"
          class="edit-chev"
          class:open={expanded}
          onclick={toggleExpanded}
          use:tooltip={expanded ? "Collapse diff" : "Show diff"}
          aria-label={expanded ? "Collapse diff" : "Show diff"}
        >
          <ChevronRight size={12} />
        </button>
        <button type="button" class="edit-crumb mono" aria-label={`File actions for ${baseName} — open, reveal, copy path`} onclick={openFile} use:tooltip={{ text: "File actions — open, reveal, copy path", placement: "bottom" }}>
          <FileText size={12} class="crumb-file" />
          {#if dirLabel}<span class="dir">{dirLabel}</span>{/if}<span class="name">{baseName}</span>
          <CornerDownLeft size={11} class="edit-open" />
        </button>
        {#if lang}<span class="edit-lang mono">{lang}</span>{/if}
        <span class="edit-head-right">
          <span class="edit-counts mono">
            <span class="ct-add">+{counts.adds}</span>
            {#if counts.dels}<span class="ct-del">−{counts.dels}</span>{/if}
          </span>
          {#if copied}
            <span class="edit-copied mono"><Check size={12} />Copied</span>
          {:else}
            <button type="button" class="edit-copy" onclick={copyDiff} use:tooltip={"Copy new content"} aria-label="Copy new content"><Copy size={12} /></button>
          {/if}
        </span>
      </div>
    {/if}
    {#if expanded}
      <div class="diff-body" transition:slide={{ duration: reducedMotion ? 0 : 200 }}>
        {#each visibleLines as l, li (li)}
          {#if l.kind === "meta"}
            <div class="diff-meta" style="--ri: {Math.min(li, 14)}">{l.text}</div>
          {:else if l.kind === "gap"}
            <div class="diff-gap" style="--ri: {Math.min(li, 14)}" data-multi={l.lines > 1} use:tooltip={l.lines === 1 ? "1 blank line" : `${l.lines} blank lines`}>
              {#if l.lines > 1}<span class="gap-dots">···</span><span class="gap-count">{l.lines} blank lines</span>{/if}
            </div>
          {:else}
            <div class="diff-line" data-kind={l.kind} style="--ri: {Math.min(li, 14)}">
              <span class="diff-num mono">{l.num ?? ""}</span>
              <span class="diff-gutter mono">{l.kind === "add" ? "+" : l.kind === "del" ? "−" : ""}</span>
              {#if l.html !== null}<span class="diff-code mono">{@html l.html}</span>{:else}<span class="diff-code mono">{l.text}</span>{/if}
            </div>
          {/if}
        {/each}
        {#if lineCapped}
          <button type="button" class="diff-more mono" onclick={(e) => { e.stopPropagation(); showAllLines = true; }}>
            Show {hiddenLineCount} more line{hiddenLineCount === 1 ? "" : "s"}
          </button>
        {/if}
      </div>
    {/if}
  </div>
{/if}

{#if menuPos && filePath}
  <FilePathMenu path={filePath} x={menuPos.x} y={menuPos.y} onClose={() => (menuPos = null)} />
{/if}

<style>
  .edit-diff {
    --diff-fs: 12px;
    margin: 8px 0;
    border: 1px solid var(--border);
    border-radius: 10px;
    overflow: hidden;
    background: var(--bg-inset);
  }
  .edit-diff.compact {
    --diff-fs: 10px;
    margin: 0;
    border-radius: 8px;
  }
  /* Header-suppressed (hideHead) — no card chrome, the host owns it. */
  .edit-diff.embedded {
    margin: 0;
    border: 0;
    border-radius: 0;
    background: transparent;
  }
  .edit-diff.embedded .diff-body { max-height: none; }

  /* ── Breadcrumb header ─────────────────────────────────────────────────── */
  .edit-head {
    display: flex; align-items: center; gap: 8px;
    width: 100%;
    padding: 8px 11px;
    background: color-mix(in oklch, var(--bg-elev-1) 70%, transparent);
    border: 0;
    border-bottom: 1px solid var(--border);
    font: inherit;
    text-align: left;
  }
  .edit-diff.collapsed .edit-head { border-bottom-color: transparent; }
  .edit-chev {
    display: inline-flex;
    padding: 0; border: 0; background: transparent;
    color: var(--fg-faint);
    flex-shrink: 0;
    cursor: pointer;
    transition: transform 140ms ease, color 140ms ease;
  }
  .edit-chev:hover { color: var(--fg-muted); }
  .edit-chev.open { transform: rotate(90deg); }
  .edit-chev:focus-visible {
    outline: 2px solid color-mix(in oklab, var(--accent) 60%, transparent);
    outline-offset: 2px; border-radius: 4px;
  }
  @media (prefers-reduced-motion: reduce) { .edit-chev { transition: color 140ms ease; } }

  .edit-crumb {
    display: inline-flex; align-items: center; gap: 6px;
    flex: 1; min-width: 0;
    padding: 3px 7px; margin-left: -2px;
    border: 0; background: transparent;
    border-radius: var(--radius-sm);
    font: inherit;
    font-size: 11.5px;
    text-align: left;
    cursor: pointer;
    transition: background 140ms ease;
  }
  .edit-crumb:hover { background: var(--surface-hover); }
  .edit-crumb:focus-visible {
    outline: 2px solid color-mix(in oklab, var(--accent) 60%, transparent);
    outline-offset: -2px;
  }
  :global(.edit-crumb .crumb-file) { color: var(--fg-faint); flex-shrink: 0; }
  .edit-crumb .dir {
    color: var(--fg-subtle);
    min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .edit-crumb .name { color: var(--fg); font-weight: 600; flex-shrink: 0; }
  .edit-crumb:hover .name { color: var(--accent); }
  /* Open-file affordance — corner-down-left icon, hinted at rest. */
  :global(.edit-crumb .edit-open) {
    color: var(--fg-faint); flex-shrink: 0;
    opacity: 0.3; transition: opacity 140ms ease;
  }
  .edit-crumb:hover :global(.edit-open) { opacity: 0.75; }

  .edit-head-right { display: inline-flex; align-items: center; gap: 9px; flex-shrink: 0; }
  .edit-copy {
    display: inline-flex; padding: 3px;
    border: 0; background: transparent;
    color: var(--fg-faint);
    border-radius: 6px; cursor: pointer;
    transition: background 140ms ease, color 140ms ease;
  }
  .edit-copy:hover { background: var(--surface-hover); color: var(--fg-2); }
  .edit-copy:focus-visible {
    outline: 2px solid color-mix(in oklab, var(--accent) 60%, transparent);
    outline-offset: -1px;
  }
  .edit-copied {
    display: inline-flex; align-items: center; gap: 4px;
    font-size: 10px; color: var(--ok);
  }

  .edit-lang {
    flex-shrink: 0;
    font-size: 9px; font-weight: 700; letter-spacing: 0.08em;
    color: var(--fg-subtle);
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    padding: 2px 6px;
    border-radius: 5px;
  }
  .edit-counts {
    display: inline-flex; gap: 8px;
    flex-shrink: 0;
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
  }
  /* Mockup `.ct-diff-stat`: counts as soft pills, not bare text. */
  .ct-add, .ct-del { padding: 1px 6px; border-radius: 5px; font-weight: 600; }
  .ct-add { color: var(--ok); background: color-mix(in oklch, var(--ok) 15%, transparent); }
  .ct-del { color: var(--danger); background: color-mix(in oklab, var(--danger) 15%, transparent); }

  /* ── Unified body ──────────────────────────────────────────────────────── */
  .diff-body {
    padding: 6px 0;
    font-family: var(--font-mono, monospace);
    font-size: var(--diff-fs);
    line-height: 1.65;
    max-height: 480px;
    overflow: auto;
  }
  .edit-diff.compact .diff-body { max-height: 280px; }

  .diff-line {
    display: grid;
    grid-template-columns: 34px 14px 1fr;
    align-items: baseline;
  }
  /* Staggered reveal — each row slides in keyed off its --ri index (capped at
     14 so big diffs settle fast). NB: the row only translates; it never drops
     to opacity:0. A re-derive (e.g. shiki finishing warm-up flips `shikiReady`
     and re-runs the inline hl()) replays this animation, and an opacity:0 frame
     would make the green change-bar visibly flash out — the "dividing green
     sometimes shows, sometimes doesn't" glitch. Translate-only keeps the bar
     solid through any replay. */
  .diff-line, .diff-meta, .diff-gap {
    animation: diff-row-in 220ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(var(--ri, 0) * 16ms);
  }
  @keyframes diff-row-in {
    from { transform: translateX(-4px); }
    to   { transform: translateX(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .diff-line, .diff-meta, .diff-gap { animation: none; }
  }
  .diff-num {
    text-align: right;
    padding-right: 10px;
    font-size: 10px;
    color: var(--fg-faint);
    opacity: 0.5;
    user-select: none;
  }
  .diff-gutter {
    text-align: center;
    font-size: 10.5px;
    color: var(--fg-faint);
    user-select: none;
  }
  .diff-code {
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--fg-muted);
    padding-right: 10px;
    min-height: 1.5em;
  }
  /* Mockup `.ct-diff-line`: softer fill + a left change-bar so add/del read at a
     glance without the heavier full-row tint. */
  .diff-line[data-kind="add"] {
    background: color-mix(in oklch, var(--ok) 6%, transparent);
    box-shadow: inset 2.5px 0 0 color-mix(in oklch, var(--ok) 85%, transparent);
  }
  .diff-line[data-kind="add"] .diff-num { color: var(--ok); opacity: 0.7; }
  .diff-line[data-kind="add"] .diff-gutter { color: var(--ok); }
  .diff-line[data-kind="add"] .diff-code { color: var(--fg); }
  .diff-line[data-kind="del"] {
    background: color-mix(in oklab, var(--danger) 6%, transparent);
    box-shadow: inset 2.5px 0 0 color-mix(in oklab, var(--danger) 85%, transparent);
  }
  .diff-line[data-kind="del"] .diff-gutter,
  .diff-line[data-kind="del"] .diff-code { color: oklch(0.86 0.06 22); }

  .diff-meta {
    color: var(--fg-muted);
    font-style: italic;
    padding: 2px 12px 3px;
    background: var(--bg-elev-2);
    font-size: var(--fs-xs);
  }
  /* Blank-context elision. Single blank ctx → thin spacer; runs → labeled
     `··· N blank lines` strip. */
  .diff-gap {
    background: var(--bg-inset);
    border-top: 1px dashed color-mix(in oklch, var(--border) 60%, transparent);
    border-bottom: 1px dashed color-mix(in oklch, var(--border) 60%, transparent);
    height: 6px;
  }
  .diff-gap[data-multi="true"] {
    height: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 3px 10px;
    color: var(--fg-faint);
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .gap-dots {
    font-family: var(--font-mono, monospace);
    color: var(--fg-muted);
    letter-spacing: 0.18em;
    font-size: 11px;
  }
  .gap-count {
    font-variant-numeric: tabular-nums;
  }
  /* #34: line-cap reveal button — full-width strip at the truncation point. */
  .diff-more {
    display: block;
    width: 100%;
    padding: 5px 12px;
    border: 0;
    border-top: 1px dashed color-mix(in oklch, var(--border) 70%, transparent);
    background: var(--bg-elev-2);
    color: var(--fg-muted);
    font-size: 10.5px;
    letter-spacing: 0.03em;
    text-align: center;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .diff-more:hover { background: var(--surface-hover); color: var(--fg); }
  .diff-more:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .mono { font-family: var(--font-mono, monospace); }
</style>
