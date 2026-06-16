<script lang="ts">
  import { ChevronRight, ExternalLink, Loader2 } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import DOMPurify from "dompurify";
  import { highlightSync, whenReady } from "../../state/highlighter.svelte";
  import { git, type GitFile } from "../../state/git.svelte";
  import { parseUnifiedDiff, foldContext } from "./parseDiff";

  let { fileStat, root }: { fileStat: GitFile; root: string | null } = $props();

  let expanded = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let diffText = $state<string | null>(null);
  let revealed = $state<Set<number>>(new Set());

  const slash = $derived(fileStat.path.lastIndexOf("/"));
  const dir = $derived(slash >= 0 ? fileStat.path.slice(0, slash + 1) : "");
  const base = $derived(slash >= 0 ? fileStat.path.slice(slash + 1) : fileStat.path);

  // Staged-only files (X set, Y clear) need the cached diff; otherwise show the
  // working-tree diff.
  const cached = $derived(
    fileStat.unstaged === " " && fileStat.staged !== " " && fileStat.staged !== "?",
  );

  const status = $derived.by(() => {
    const x = fileStat.staged;
    const y = fileStat.unstaged;
    if (x === "?" || y === "?") return { ch: "U", tone: "new", title: "Untracked" };
    const c = y !== " " ? y : x;
    if (c === "A") return { ch: "A", tone: "add", title: "Added" };
    if (c === "D") return { ch: "D", tone: "del", title: "Deleted" };
    if (c === "R") return { ch: "R", tone: "ren", title: "Renamed" };
    return { ch: c.trim() || "M", tone: "mod", title: "Modified" };
  });

  const parsed = $derived(diffText != null ? parseUnifiedDiff(diffText) : null);
  const segments = $derived(parsed ? foldContext(parsed.lines) : []);

  async function toggle() {
    expanded = !expanded;
    if (expanded && diffText == null && !loading) {
      loading = true;
      error = null;
      try {
        diffText = await git.fileDiff(root, fileStat.path, cached);
      } catch (e) {
        error = String(e);
      } finally {
        loading = false;
      }
    }
  }

  function openExternal(e: MouseEvent) {
    e.stopPropagation();
    const abs = root ? `${root}/${fileStat.path}` : fileStat.path;
    void invoke("open_in_vscode", { path: abs }).catch(() => {});
  }

  function reveal(idx: number) {
    const next = new Set(revealed);
    next.add(idx);
    revealed = next;
  }

  // ── Shiki per-line highlight (mirrors EditDiff.svelte) ──────────────────────
  let shikiReady = $state(false);
  whenReady().then(() => { shikiReady = true; }).catch(() => {});
  const EXT_LANG: Record<string, string> = {
    rs: "rust", ts: "typescript", tsx: "typescript", mts: "typescript", cts: "typescript",
    js: "javascript", jsx: "javascript", mjs: "javascript", cjs: "javascript",
    svelte: "svelte", sh: "bash", bash: "bash", zsh: "bash",
    json: "json", jsonc: "json", toml: "toml", lua: "lua", py: "python", pyi: "python",
    md: "markdown", css: "css", html: "html", yml: "yaml", yaml: "yaml",
  };
  const langId = $derived.by(() => {
    const dot = base.lastIndexOf(".");
    const ext = dot >= 0 ? base.slice(dot + 1).toLowerCase() : "";
    return EXT_LANG[ext] ?? null;
  });
  function hl(text: string): string | null {
    if (!shikiReady || !langId || text.length === 0) return null;
    const html = highlightSync(text, langId);
    if (!html) return null;
    const m = html.match(/<span class="line">([\s\S]*?)<\/span><\/code>/);
    if (!m) return null;
    return DOMPurify.sanitize(m[1], { ALLOWED_TAGS: ["span"], ALLOWED_ATTR: ["style", "class"] });
  }
</script>

<div class="card" class:open={expanded}>
  <div class="head">
    <button class="head-main" type="button" onclick={toggle} aria-expanded={expanded}>
      <ChevronRight size={13} class="chev" />
      <span class="badge" data-tone={status.tone} title={status.title}>{status.ch}</span>
      <span class="path"><span class="dir">{dir}</span><span class="base">{base}</span></span>
      <span class="stat">
        {#if fileStat.adds > 0}<span class="adds">+{fileStat.adds}</span>{/if}
        {#if fileStat.dels > 0}<span class="dels">−{fileStat.dels}</span>{/if}
      </span>
    </button>
    <button class="ext" type="button" aria-label="Open in editor" title="Open in editor" onclick={openExternal}><ExternalLink size={12} /></button>
  </div>

  {#if expanded}
    <div class="body">
      {#if loading}
        <div class="msg"><Loader2 size={13} class="spin" /> Loading diff…</div>
      {:else if error}
        <div class="msg err">{error}</div>
      {:else if parsed && parsed.binary}
        <div class="msg">Binary file — no textual diff.</div>
      {:else if parsed && parsed.lines.length === 0}
        <div class="msg">No textual changes to show.</div>
      {:else if parsed}
        <div class="diff">
          {#each segments as seg, i}
            {#if seg.fold && !revealed.has(i)}
              <button class="fold" type="button" onclick={() => reveal(i)}>
                ⋯ {seg.count} unmodified line{seg.count === 1 ? "" : "s"}
              </button>
            {:else if seg.fold}
              {#each seg.lines as ln}
                <div class="row ctx">
                  <span class="gut">{ln.oldNo ?? ""}</span><span class="gut">{ln.newNo ?? ""}</span>
                  <span class="code">{#if hl(ln.text)}{@html hl(ln.text)}{:else}{ln.text || " "}{/if}</span>
                </div>
              {/each}
            {:else if seg.line.kind === "hunk"}
              <div class="row hunk"><span class="gut"></span><span class="gut"></span><span class="code">{seg.line.text}</span></div>
            {:else}
              <div class="row {seg.line.kind}">
                <span class="gut">{seg.line.oldNo ?? ""}</span><span class="gut">{seg.line.newNo ?? ""}</span>
                <span class="sign">{seg.line.kind === "add" ? "+" : seg.line.kind === "del" ? "−" : ""}</span>
                <span class="code">{#if hl(seg.line.text)}{@html hl(seg.line.text)}{:else}{seg.line.text || " "}{/if}</span>
              </div>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .card { border: 1px solid var(--border); border-radius: var(--radius); background: var(--surface); overflow: hidden; }
  .card.open { border-color: color-mix(in oklab, var(--accent) 28%, var(--border)); }
  .head { display: flex; align-items: center; }
  .head:hover { background: color-mix(in oklab, var(--accent) 6%, transparent); }
  .head-main {
    display: flex; align-items: center; gap: 8px; flex: 1 1 auto; min-width: 0;
    padding: 7px 4px 7px 9px; background: none; border: 0; cursor: pointer;
    color: var(--fg); font: inherit; text-align: left;
  }
  .head :global(.chev) { color: var(--fg-3); transition: transform 140ms ease; flex: none; }
  .card.open .head :global(.chev) { transform: rotate(90deg); }
  .badge {
    flex: none; width: 16px; height: 16px; display: grid; place-items: center;
    border-radius: 4px; font-size: 10px; font-weight: 700; line-height: 1;
  }
  .badge[data-tone="add"], .badge[data-tone="new"] { background: color-mix(in oklab, var(--ok) 20%, transparent); color: var(--ok); }
  .badge[data-tone="del"] { background: color-mix(in oklab, var(--danger) 20%, transparent); color: var(--danger); }
  .badge[data-tone="mod"], .badge[data-tone="ren"] { background: color-mix(in oklab, var(--warn) 22%, transparent); color: var(--warn); }
  .path { flex: 1 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--fs-xs); }
  .dir { color: var(--fg-3); }
  .base { color: var(--fg); font-weight: 550; }
  .stat { flex: none; display: inline-flex; gap: 6px; font-size: var(--fs-xs); font-variant-numeric: tabular-nums; }
  .adds { color: var(--ok); }
  .dels { color: var(--danger); }
  .ext { flex: none; display: inline-flex; align-items: center; padding: 7px 9px; border: 0; background: none; border-radius: 5px; color: var(--fg-3); cursor: pointer; }
  .ext:hover { color: var(--fg); background: color-mix(in oklab, var(--accent) 12%, transparent); }
  .body { border-top: 1px solid var(--border); }
  .msg { display: flex; align-items: center; gap: 6px; padding: 9px 11px; color: var(--fg-2); font-size: var(--fs-xs); }
  .msg.err { color: var(--danger); white-space: pre-wrap; }
  .msg :global(.spin) { animation: env-spin 1s linear infinite; }
  @keyframes env-spin { to { transform: rotate(360deg); } }
  .diff {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 11px; line-height: 1.5; overflow-x: auto;
    padding: 4px 0;
  }
  .row { display: flex; align-items: flex-start; white-space: pre; }
  .gut {
    flex: none; width: 27px; padding-right: 4px; text-align: right;
    color: var(--fg-4, var(--fg-3)); user-select: none; opacity: 0.55;
  }
  .gut:first-child { padding-left: 6px; }
  .sign { flex: none; width: 11px; text-align: center; user-select: none; }
  .code { flex: 1 1 auto; padding-right: 12px; }
  .row.add { background: color-mix(in oklab, var(--ok) 11%, transparent); box-shadow: inset 2px 0 0 var(--ok); }
  .row.add .sign { color: var(--ok); }
  .row.del { background: color-mix(in oklab, var(--danger) 11%, transparent); box-shadow: inset 2px 0 0 var(--danger); }
  .row.del .sign { color: var(--danger); }
  .row.hunk { color: var(--accent); background: color-mix(in oklab, var(--accent) 7%, transparent); }
  .row.hunk .code { padding-left: 4px; }
  .fold {
    display: block; width: 100%; text-align: left; padding: 3px 0 3px 50px;
    background: color-mix(in oklab, var(--accent) 4%, transparent);
    border: 0; border-top: 1px dashed var(--border); border-bottom: 1px dashed var(--border);
    color: var(--fg-3); font: inherit; font-size: 11px; cursor: pointer;
  }
  .fold:hover { color: var(--accent); background: color-mix(in oklab, var(--accent) 9%, transparent); }
</style>
