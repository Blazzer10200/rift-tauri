<script lang="ts">
  // Structured grep/glob result — `path:line: text` rows become navigable:
  // click a row → resolve against the workspace root → FilePathMenu (open in
  // editor / reveal / copy), same flow Markdown's clickable code-span paths
  // use. Match substrings highlight when the search pattern compiles.
  import { invoke } from "@tauri-apps/api/core";
  import { parseGrepLine, type GrepRow } from "./streamModel";
  import { assistant } from "$lib/state/assistant.svelte";
  import { notify } from "$lib/state/toast.svelte";
  import { leafName } from "$lib/utils/path";
  import FilePathMenu from "../FilePathMenu.svelte";

  let {
    text,
    pattern = null,
    bare = false,
  }: {
    text: string;
    pattern?: string | null;
    bare?: boolean; // Glob / files-with-matches — every line IS a path
  } = $props();

  type Row =
    | { t: "match"; row: GrepRow }
    | { t: "path"; path: string }
    | { t: "plain"; line: string };

  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    for (const line of text.replace(/\s+$/, "").split("\n")) {
      if (!line.trim()) continue;
      if (bare) {
        // Glob output is one path per line — headers/"No files found" stay plain.
        if (/^No files found/i.test(line) || /^\(/.test(line)) out.push({ t: "plain", line });
        else out.push({ t: "path", path: line.trim() });
        continue;
      }
      const m = parseGrepLine(line);
      if (m) out.push({ t: "match", row: m });
      else if (/^(Found \d+|No matches|No files)/i.test(line) || !line.includes(":")) out.push({ t: "plain", line });
      else out.push({ t: "path", path: line.trim() }); // files_with_matches mode
    }
    return out;
  });

  // Two-tier reveal, mirroring OutputBlock's collapsed cap.
  const CAP = 12;
  let showAll = $state(false);
  const shown = $derived(showAll ? rows : rows.slice(0, CAP));
  const hidden = $derived(Math.max(0, rows.length - CAP));

  // Highlight the match inside a content cell. The Grep pattern is a regex
  // string — try it as-is; an invalid/exotic pattern just skips highlighting.
  const matcher = $derived.by<RegExp | null>(() => {
    if (!pattern) return null;
    try { return new RegExp(pattern, "gi"); } catch { return null; }
  });
  function splitHits(content: string): { text: string; hit: boolean }[] {
    if (!matcher) return [{ text: content, hit: false }];
    const out: { text: string; hit: boolean }[] = [];
    let last = 0;
    matcher.lastIndex = 0;
    let m: RegExpExecArray | null;
    let guard = 0;
    while ((m = matcher.exec(content)) && guard++ < 50) {
      if (m[0].length === 0) break; // zero-width match — bail, don't loop
      if (m.index > last) out.push({ text: content.slice(last, m.index), hit: false });
      out.push({ text: m[0], hit: true });
      last = m.index + m[0].length;
    }
    if (last < content.length) out.push({ text: content.slice(last), hit: false });
    return out.length ? out : [{ text: content, hit: false }];
  }

  let pathMenu = $state<{ path: string; line: number | null; x: number; y: number } | null>(null);
  async function openRow(e: MouseEvent, path: string, line: number | null) {
    try {
      const resolved = await invoke<string>("resolve_workspace_path", { root: assistant.activeRoot, path });
      pathMenu = { path: resolved, line, x: e.clientX, y: e.clientY + 6 };
    } catch (err) {
      notify.warn("Couldn't locate path", { detail: String(err) });
    }
  }
</script>

<div class="gres">
  {#each shown as r, i (i)}
    {#if r.t === "match"}
      <button class="gres-row" type="button" title={r.row.path} onclick={(e) => openRow(e, r.row.path, r.row.line)}>
        <span class="gres-path">{leafName(r.row.path)}<span class="gres-line">:{r.row.line}</span></span>
        <span class="gres-text">{#each splitHits(r.row.text) as seg, si (si)}{#if seg.hit}<mark class="gres-hit">{seg.text}</mark>{:else}{seg.text}{/if}{/each}</span>
      </button>
    {:else if r.t === "path"}
      <button class="gres-row" type="button" title={r.path} onclick={(e) => openRow(e, r.path, null)}>
        <span class="gres-path wide">{r.path}</span>
      </button>
    {:else}
      <div class="gres-plain">{r.line}</div>
    {/if}
  {/each}
  {#if hidden > 0 && !showAll}
    <button class="gres-more" type="button" onclick={() => (showAll = true)}>Show all {rows.length}</button>
  {/if}
</div>

<style>
  .gres { display: flex; flex-direction: column; padding: 4px 0;
    font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5; }
  .gres-row { display: flex; align-items: baseline; gap: 8px; width: 100%; min-width: 0;
    padding: 1px 10px; border: 0; background: none; cursor: pointer;
    font: inherit; color: var(--fg-2); text-align: left;
    transition: background var(--dur-fast), color var(--dur-fast); }
  .gres-row:hover { background: var(--surface-hover); color: var(--fg); }
  /* Neutral at rest (one chromatic voice per view — DESIGN §2); the hover
     underline + accent carry the link affordance. */
  .gres-path { flex: none; color: var(--fg-2); font-weight: 550; }
  .gres-row:hover .gres-path { color: var(--accent); text-decoration: underline; text-underline-offset: 2px; }
  .gres-path.wide { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gres-line { color: var(--fg-faint); font-weight: 400; }
  .gres-text { flex: 1; min-width: 0; white-space: pre-wrap; word-break: break-word; color: var(--fg-2); }
  .gres-hit { background: color-mix(in oklab, var(--accent) 26%, transparent);
    color: var(--fg); border-radius: 2px; padding: 0 1px; }
  .gres-plain { padding: 1px 10px; color: var(--fg-faint); white-space: pre-wrap; word-break: break-word; }
  .gres-more { align-self: flex-start; margin: 3px 6px 1px; padding: 2px 8px;
    border: 0; border-radius: 6px; background: none; cursor: pointer;
    font: inherit; font-size: 10.5px; font-weight: 550; color: var(--fg-subtle);
    transition: background var(--dur-fast), color var(--dur-fast); }
  .gres-more:hover { background: var(--surface-hover); color: var(--fg-2); }
</style>

{#if pathMenu}
  <FilePathMenu path={pathMenu.path} line={pathMenu.line} x={pathMenu.x} y={pathMenu.y} onClose={() => (pathMenu = null)} />
{/if}
