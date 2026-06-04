<script lang="ts">
  // Session Diff — a full-surface review of every code edit Claude made this
  // conversation, grouped by file (Write rendered as all-additions, MultiEdit
  // expanded per-edit). Reuses the chat's EditDiff renderer (`hideHead`) so the
  // diff vocabulary is identical to the inline chips. Opens over the chat pane;
  // closes on X / backdrop / Escape. Reads the shared preview singleton's mock
  // edits when preview mode is on, so it demos without a real turn.
  import { onMount } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { diffArrays } from "diff";
  import { X, GitCompare, ChevronRight, FileText, FilePlus2, FoldVertical, UnfoldVertical } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import type { Block, ChatMessage } from "../../state/assistant.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import EditDiff from "./EditDiff.svelte";

  let { tabId = null, onClose }: { tabId?: string | null; onClose: () => void } = $props();

  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

  const tab = $derived(tabId == null ? assistant.activeTab : assistant.tabFor(tabId));
  const messages = $derived<ChatMessage[]>(tab?.messages ?? []);
  const target = $derived(assistant.ui.diffTarget);

  const EDIT_TOOLS = new Set(["Write", "Edit", "MultiEdit", "NotebookEdit"]);
  const s = (v: unknown): string => (typeof v === "string" ? v : "");

  type Edit = { id: string; file: string; input: Record<string, unknown>; ts: number };
  const edits = $derived.by<Edit[]>(() => {
    const out: Edit[] = [];
    for (const m of messages) {
      for (const b of m.blocks as Block[]) {
        if (b.type !== "tool" || !EDIT_TOOLS.has(b.name)) continue;
        if (b.isError || b.status === "error" || b.status === "pending") continue;
        const inp = (b.input ?? {}) as Record<string, unknown>;
        const ts = b.startedAt ?? 0;
        if (b.name === "Write") {
          const fp = s(inp.file_path);
          if (fp) out.push({ id: b.id, file: fp, ts, input: { file_path: fp, old_string: "", new_string: s(inp.content) } });
        } else if (b.name === "MultiEdit") {
          const fp = s(inp.file_path);
          const arr = Array.isArray(inp.edits) ? inp.edits : [];
          arr.forEach((raw, k) => {
            const o = (raw ?? {}) as Record<string, unknown>;
            out.push({ id: `${b.id}-${k}`, file: fp, ts, input: { file_path: fp, old_string: s(o.old_string), new_string: s(o.new_string), replace_all: o.replace_all === true } });
          });
        } else {
          const fp = s(inp.file_path) || s(inp.notebook_path);
          out.push({ id: b.id, file: fp, ts, input: { ...inp, file_path: fp } });
        }
      }
    }
    return out;
  });

  function countDiff(oldStr: string, newStr: string): { adds: number; dels: number } {
    if (oldStr === "") return { adds: newStr === "" ? 0 : newStr.split("\n").length, dels: 0 };
    let adds = 0; let dels = 0;
    for (const c of diffArrays(oldStr.split("\n"), newStr.split("\n"))) {
      if (c.added) adds += c.value.length;
      else if (c.removed) dels += c.value.length;
    }
    return { adds, dels };
  }
  function baseName(p: string): string {
    const norm = p.replace(/\\/g, "/").replace(/\/$/, "");
    return norm.split("/").pop() ?? norm;
  }
  function dirLabel(p: string): string {
    const norm = p.replace(/\\/g, "/");
    const idx = norm.lastIndexOf("/");
    if (idx < 0) return "";
    const segs = norm.slice(0, idx).split("/").filter(Boolean);
    if (segs.length <= 2) return segs.length ? segs.join("/") + "/" : "";
    return "…/" + segs.slice(-2).join("/") + "/";
  }
  function langOf(p: string): string {
    const b = baseName(p);
    const dot = b.lastIndexOf(".");
    if (dot < 0) return "";
    const ext = b.slice(dot + 1).toUpperCase();
    return ext.length > 0 && ext.length <= 8 ? ext : "";
  }

  type Group = { key: string; file: string; base: string; dir: string; lang: string; edits: Edit[]; adds: number; dels: number; lastTs: number };
  const groups = $derived.by<Group[]>(() => {
    const map = new Map<string, Group>();
    for (const e of edits) {
      const key = e.file.replace(/\\/g, "/");
      let g = map.get(key);
      if (!g) { g = { key, file: e.file, base: baseName(e.file), dir: dirLabel(e.file), lang: langOf(e.file), edits: [], adds: 0, dels: 0, lastTs: 0 }; map.set(key, g); }
      g.edits.push(e);
      const c = countDiff(s(e.input.old_string), s(e.input.new_string));
      g.adds += c.adds; g.dels += c.dels;
      if (e.ts > g.lastTs) g.lastTs = e.ts;
    }
    return [...map.values()].sort((a, b) => b.lastTs - a.lastTs);
  });
  const totals = $derived(groups.reduce((a, g) => ({ files: a.files + 1, adds: a.adds + g.adds, dels: a.dels + g.dels }), { files: 0, adds: 0, dels: 0 }));

  // Per-file collapse — default all open. Set holds the COLLAPSED keys.
  let collapsed = $state<Set<string>>(new Set());
  function toggleGroup(key: string) {
    const next = new Set(collapsed);
    if (next.has(key)) next.delete(key); else next.add(key);
    collapsed = next;
  }
  const allCollapsed = $derived(groups.length > 0 && groups.every((g) => collapsed.has(g.key)));
  function toggleAll() {
    collapsed = allCollapsed ? new Set() : new Set(groups.map((g) => g.key));
  }

  function close() {
    assistant.ui.diffTarget = null;
    onClose();
  }

  let rootEl = $state<HTMLDivElement | undefined>();
  onMount(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") { e.preventDefault(); close(); } };
    window.addEventListener("keydown", onKey);
    // Deep-link from an Outputs row — scroll to the matching file (by basename).
    if (target) {
      requestAnimationFrame(() => {
        const el = rootEl?.querySelector<HTMLElement>(`[data-base="${CSS.escape(target)}"]`);
        el?.scrollIntoView({ behavior: "smooth", block: "start" });
      });
    }
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="diff-overlay"
  bind:this={rootEl}
  transition:fade={{ duration: reducedMotion ? 0 : 140 }}
  onclick={(e) => { if (e.target === e.currentTarget) close(); }}
>
  <div class="diff-sheet" transition:fly={{ y: reducedMotion ? 0 : 14, duration: reducedMotion ? 0 : 220, easing: cubicOut }}>
    <header class="dh">
      <span class="dh-ic"><GitCompare size={15} /></span>
      <span class="dh-title">Changes</span>
      {#if totals.files > 0}
        <span class="dh-sum mono">
          {totals.files} {totals.files === 1 ? "file" : "files"}
          <span class="dh-add">+{totals.adds}</span>{#if totals.dels}<span class="dh-del">−{totals.dels}</span>{/if}
        </span>
      {/if}
      <span class="dh-right">
        {#if groups.length > 0}
          <button type="button" class="dh-btn" onclick={toggleAll} use:tooltip={allCollapsed ? "Expand all files" : "Collapse all files"}>
            {#if allCollapsed}<UnfoldVertical size={14} />{:else}<FoldVertical size={14} />{/if}
          </button>
        {/if}
        <button type="button" class="dh-x" onclick={close} use:tooltip={"Close (Esc)"} aria-label="Close diff"><X size={15} /></button>
      </span>
    </header>

    <div class="dh-scroll">
      {#if groups.length === 0}
        <div class="dh-empty">
          <FilePlus2 size={20} />
          <p>No code edits in this conversation yet.</p>
          <span>Files Claude writes or edits will collect here as a reviewable diff.</span>
        </div>
      {:else}
        {#each groups as g, gi (g.key)}
          {@const isCol = collapsed.has(g.key)}
          <section class="dg" data-base={g.base} in:fly={{ y: reducedMotion ? 0 : 8, duration: reducedMotion ? 0 : 220, delay: Math.min(gi, 6) * 30, easing: cubicOut }}>
            <button type="button" class="dg-head" class:collapsed={isCol} onclick={() => toggleGroup(g.key)}>
              <ChevronRight size={13} class="dg-chev" />
              <FileText size={13} class="dg-file" />
              {#if g.dir}<span class="dg-dir mono">{g.dir}</span>{/if}
              <span class="dg-name mono">{g.base}</span>
              {#if g.lang}<span class="dg-lang mono">{g.lang}</span>{/if}
              <span class="dg-meta">
                {#if g.edits.length > 1}<span class="dg-edits">{g.edits.length} edits</span>{/if}
                <span class="dg-stat mono"><span class="dg-add">+{g.adds}</span>{#if g.dels}<span class="dg-del">−{g.dels}</span>{/if}</span>
              </span>
            </button>
            {#if !isCol}
              <div class="dg-body">
                {#each g.edits as e, ei (e.id)}
                  {#if ei > 0}<div class="dg-rule" aria-hidden="true"></div>{/if}
                  <EditDiff input={e.input} hideHead />
                {/each}
              </div>
            {/if}
          </section>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .diff-overlay {
    position: absolute;
    inset: 0;
    z-index: 8;
    display: flex;
    flex-direction: column;
    background: color-mix(in oklch, var(--bg) 80%, transparent);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
  }
  .diff-sheet {
    flex: 1; min-height: 0;
    display: flex; flex-direction: column;
    margin: 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg, 14px);
    background: var(--bg);
    box-shadow: 0 24px 60px -16px oklch(0 0 0 / 0.55), 0 0 0 1px color-mix(in oklab, var(--accent) 6%, transparent);
    overflow: hidden;
  }

  /* Header */
  .dh {
    display: flex; align-items: center; gap: 10px;
    height: 48px; padding: 0 12px 0 14px; flex-shrink: 0;
    border-bottom: 1px solid var(--border);
    background: color-mix(in oklch, var(--bg-elev-1) 60%, var(--bg));
  }
  .dh-ic { display: inline-flex; color: var(--accent); }
  .dh-title { font-size: var(--fs-md); font-weight: 650; color: var(--fg); }
  .dh-sum {
    margin-left: 4px;
    display: inline-flex; align-items: center; gap: 8px;
    font-size: 11.5px; color: var(--fg-muted); font-variant-numeric: tabular-nums;
  }
  .dh-add { color: var(--ok); font-weight: 600; }
  .dh-del { color: var(--danger); font-weight: 600; }
  .dh-right { margin-left: auto; display: inline-flex; align-items: center; gap: 4px; }
  .dh-btn, .dh-x {
    width: 30px; height: 30px; display: grid; place-items: center;
    border: 0; background: transparent; color: var(--fg-faint);
    border-radius: 8px; cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .dh-btn:hover { background: var(--surface-hover); color: var(--fg); }
  .dh-x:hover { background: color-mix(in oklab, var(--danger) 12%, transparent); color: var(--danger); }
  .dh-btn:focus-visible, .dh-x:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }

  /* Scroll body */
  .dh-scroll {
    flex: 1; min-height: 0; overflow-y: auto;
    padding: 12px;
    display: flex; flex-direction: column; gap: 12px;
    scrollbar-width: thin;
  }
  .dh-scroll::-webkit-scrollbar { width: 9px; }
  .dh-scroll::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 5px; }
  .dh-scroll::-webkit-scrollbar-thumb:hover { background: var(--fg-faint); }

  .dh-empty {
    margin: auto; max-width: 320px;
    display: flex; flex-direction: column; align-items: center; gap: 8px;
    text-align: center; color: var(--fg-faint); padding: 40px 24px;
  }
  .dh-empty :global(svg) { color: var(--fg-subtle); opacity: 0.6; }
  .dh-empty p { margin: 0; font-size: var(--fs-sm); color: var(--fg-2); font-weight: 600; }
  .dh-empty span { font-size: 11px; line-height: 1.5; }

  /* File group */
  .dg {
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
    background: var(--bg-inset);
  }
  .dg-head {
    display: flex; align-items: center; gap: 8px;
    width: 100%; padding: 9px 12px;
    border: 0; background: color-mix(in oklch, var(--bg-elev-1) 55%, transparent);
    border-bottom: 1px solid var(--border);
    font: inherit; text-align: left; cursor: pointer;
    transition: background 130ms ease;
  }
  .dg-head:hover { background: var(--surface-hover); }
  .dg-head.collapsed { border-bottom-color: transparent; }
  .dg-head:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  :global(.dg-head .dg-chev) { color: var(--fg-faint); flex-shrink: 0; transition: transform 140ms ease; }
  .dg-head:not(.collapsed) :global(.dg-chev) { transform: rotate(90deg); }
  :global(.dg-head .dg-file) { color: var(--fg-faint); flex-shrink: 0; }
  .dg-dir { color: var(--fg-subtle); font-size: 11.5px; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .dg-name { color: var(--fg); font-weight: 600; font-size: 11.5px; flex-shrink: 0; }
  .dg-lang {
    flex-shrink: 0; font-size: 9px; font-weight: 700; letter-spacing: 0.08em;
    color: var(--fg-subtle); background: var(--bg-elev-2);
    border: 1px solid var(--border); padding: 2px 6px; border-radius: 5px;
  }
  .dg-meta { margin-left: auto; display: inline-flex; align-items: center; gap: 10px; flex-shrink: 0; }
  .dg-edits { font-size: 10px; color: var(--fg-faint); font-variant-numeric: tabular-nums; }
  .dg-stat { display: inline-flex; gap: 6px; font-size: 11px; font-variant-numeric: tabular-nums; }
  .dg-add { color: var(--ok); padding: 1px 6px; border-radius: 5px; font-weight: 600; background: color-mix(in oklch, var(--ok) 14%, transparent); }
  .dg-del { color: var(--danger); padding: 1px 6px; border-radius: 5px; font-weight: 600; background: color-mix(in oklab, var(--danger) 14%, transparent); }

  .dg-body { display: flex; flex-direction: column; }
  /* Divider between successive edits to the same file. */
  .dg-rule {
    height: 0;
    border-top: 1px dashed color-mix(in oklch, var(--border) 70%, transparent);
    margin: 2px 0;
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.dg-head .dg-chev) { transition: none; }
  }
</style>
