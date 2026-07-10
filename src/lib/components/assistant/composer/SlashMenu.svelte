<script lang="ts">
  // C6 (per docs/design/composer-split.md) — the `/command` popover. Render +
  // click only: the keyboard nav index and open/filter state stay owned by the
  // parent's onKey. Redesigned 2026-07-09: glass panel + header strip + sticky
  // group labels + styled scrollbar; custom entries (user's Claude Code
  // skills/commands) carry source badges, argument hints, fuzzy highlight.
  import {
    Plus, Eraser, Cpu, RotateCcw, Copy, StopCircle,
    Wrench, Coins, Gauge, BarChart3, Terminal, ClipboardCopy, HelpCircle,
    Palette, LogIn, Sparkles, FileCode2, SearchX, SquareSlash, Plug,
  } from "lucide-svelte";
  import { slashMatchSegments } from "./helpers";

  type Icon = typeof Plus;
  type SlashCmd = {
    name: string;
    desc: string;
    custom?: { source: "user" | "project"; kind: "skill" | "command"; hint?: string };
  };

  let {
    commands,
    activeIdx,
    query = "",
    onPick,
  }: {
    commands: SlashCmd[];
    activeIdx: number;
    query?: string;
    onPick: (c: SlashCmd) => void;
  } = $props();

  // Display metadata only — the command list (and its order) is the parent's.
  const META: Record<string, { icon: Icon; group: string }> = {
    new:       { icon: Plus,          group: "Conversation" },
    clear:     { icon: Eraser,        group: "Conversation" },
    model:     { icon: Cpu,           group: "Compose" },
    retry:     { icon: RotateCcw,     group: "Compose" },
    copy:      { icon: Copy,          group: "Compose" },
    stop:      { icon: StopCircle,    group: "Compose" },
    tools:     { icon: Wrench,        group: "Info" },
    mcp:       { icon: Plug,          group: "Info" },
    cost:      { icon: Coins,        group: "Info" },
    usage:     { icon: Gauge,         group: "Info" },
    stats:     { icon: BarChart3,     group: "Info" },
    openincli: { icon: Terminal,      group: "Info" },
    diag:      { icon: ClipboardCopy, group: "Info" },
    help:      { icon: HelpCircle,    group: "Info" },
    "design-sync":  { icon: Palette, group: "Design" },
    "design-login": { icon: LogIn,   group: "Design" },
  };
  const groupOf = (c: SlashCmd) =>
    c.custom
      ? c.custom.source === "project" ? "Project skills" : "Your skills"
      : META[c.name]?.group ?? "Commands";
  const iconOf = (c: SlashCmd): Icon =>
    c.custom
      ? c.custom.kind === "skill" ? Sparkles : FileCode2
      : META[c.name]?.icon ?? Terminal;
  // Filtering re-ranks across groups, so headers would interleave — flat list
  // with per-row source badges instead (standard palette behavior).
  const grouped = $derived(query.length === 0);

  let listEl = $state<HTMLDivElement | undefined>();
  $effect(() => {
    const el = listEl?.querySelector<HTMLElement>(`[data-idx="${activeIdx}"]`);
    el?.scrollIntoView({ block: "nearest" });
  });
</script>

<div class="rift-menu slash-menu" role="menu" aria-label="Slash commands">
  <header class="sm-head">
    <span class="sm-head-icon"><SquareSlash size={12} /></span>
    <span class="sm-head-title">Commands</span>
    {#if query}<span class="sm-head-query mono">“{query}”</span>{/if}
    <span class="sm-head-count">{commands.length}</span>
  </header>
  {#if commands.length > 0}
    <div class="sm-list" bind:this={listEl}>
      {#each commands as c, i (c.name)}
        {@const Icon = iconOf(c)}
        {#if grouped && (i === 0 || groupOf(c) !== groupOf(commands[i - 1]))}
          <div class="sm-group" role="separator"><span>{groupOf(c)}</span><i></i></div>
        {/if}
        <button
          type="button"
          role="menuitem"
          class="sm-item"
          data-active={i === activeIdx}
          data-idx={i}
          style="--idx: {i}"
          onmousedown={(e) => { e.preventDefault(); onPick(c); }}
        >
          <span class="sm-bar"></span>
          <span class="sm-icon" data-skill={c.custom?.kind === "skill"}><Icon size={13} /></span>
          <span class="sm-body">
            <span class="sm-line">
              <span class="sm-cmd mono">/{#each slashMatchSegments(c.name, query) as seg, si (si)}{#if seg.hit}<b>{seg.text}</b>{:else}{seg.text}{/if}{/each}</span>
              {#if c.custom?.hint}<span class="sm-hint mono">{c.custom.hint}</span>{/if}
              {#if !grouped && c.custom}<span class="sm-badge" data-src={c.custom.source}>{c.custom.source === "project" ? "project" : "yours"}</span>{/if}
            </span>
            <span class="sm-desc">{c.desc}</span>
          </span>
          {#if i === activeIdx}<kbd class="sm-kbd">↵</kbd>{/if}
        </button>
      {/each}
    </div>
  {:else}
    <div class="sm-empty">
      <span class="sm-empty-icon"><SearchX size={16} /></span>
      <span class="sm-empty-title mono">/{query}</span>
      <span class="sm-empty-sub">No matching command — <kbd class="sm-kbd">Esc</kbd> to dismiss, or add a space to send as chat</span>
    </div>
  {/if}
  <footer class="sm-foot">
    <span><kbd class="sm-kbd">↑↓</kbd> select</span>
    <span><kbd class="sm-kbd">↵</kbd> run</span>
    <span><kbd class="sm-kbd">⇥</kbd> insert</span>
    <span><kbd class="sm-kbd">Esc</kbd> cancel</span>
  </footer>
</div>

<style>
  /* Overrides the shared .rift-menu chrome into a glass command-palette:
     translucent blurred surface, gradient hairline, layered depth shadow. */
  .slash-menu {
    position: absolute;
    bottom: calc(100% + 10px);
    left: 0;
    width: min(480px, 100%);
    display: flex; flex-direction: column;
    overflow: hidden;
    z-index: 10;
    padding: 0;
    background: color-mix(in oklab, var(--surface) 86%, transparent);
    backdrop-filter: blur(18px) saturate(1.25);
    border: 1px solid color-mix(in oklab, var(--accent) 14%, var(--border));
    border-radius: 16px;
    box-shadow:
      0 24px 56px -12px oklch(0 0 0 / 0.65),
      0 0 0 1px color-mix(in oklab, var(--accent) 7%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 7%, transparent);
    animation: slash-in 180ms var(--ease-page, cubic-bezier(0.22, 1, 0.36, 1));
  }
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(6px) scale(0.985); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }

  /* Header strip — palette identity + live filter echo + count. */
  .sm-head {
    display: flex; align-items: center; gap: 7px;
    padding: 9px 12px 8px;
    border-bottom: 1px solid color-mix(in oklab, var(--border) 70%, transparent);
    background: linear-gradient(
      to bottom,
      color-mix(in oklab, var(--accent) 6%, transparent),
      transparent
    );
  }
  .sm-head-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 18px; height: 18px; border-radius: 5px;
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 22%, transparent);
  }
  .sm-head-title {
    font-size: 10px; font-weight: 700;
    text-transform: uppercase; letter-spacing: 0.09em;
    color: var(--fg-muted);
  }
  .sm-head-query {
    font-size: var(--fs-xs); color: var(--accent);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .sm-head-count {
    margin-left: auto;
    font-size: 10px; font-weight: 700;
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--accent);
    padding: 1px 7px; border-radius: 999px;
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 20%, transparent);
  }

  /* List + scrollbar — thin accent thumb on a transparent track, with soft
     fade masks so rows scroll "under" the header/footer instead of clipping. */
  .sm-list {
    max-height: 336px; overflow-y: auto;
    padding: 4px 6px 8px;
    mask-image: linear-gradient(to bottom, transparent 0, black 8px, black calc(100% - 10px), transparent 100%);
  }
  .sm-list::-webkit-scrollbar { width: 9px; }
  .sm-list::-webkit-scrollbar-track { background: transparent; }
  .sm-list::-webkit-scrollbar-thumb {
    background: color-mix(in oklab, var(--accent) 26%, var(--bg-elev-3));
    border-radius: 999px;
    border: 3px solid transparent;
    background-clip: padding-box;
  }
  .sm-list:hover::-webkit-scrollbar-thumb {
    background: color-mix(in oklab, var(--accent) 45%, var(--bg-elev-3));
    background-clip: padding-box;
  }

  /* Group labels — sticky, with a hairline rule bleeding right. */
  .sm-group {
    position: sticky; top: 0; z-index: 1;
    display: flex; align-items: center; gap: 8px;
    font-size: 9.5px; font-weight: 700;
    text-transform: uppercase; letter-spacing: 0.09em;
    color: var(--fg-subtle);
    padding: 9px 8px 5px;
    background: linear-gradient(
      to bottom,
      color-mix(in oklab, var(--surface) 96%, transparent) 70%,
      transparent
    );
  }
  .sm-group i {
    flex: 1; height: 1px;
    background: linear-gradient(to right, color-mix(in oklab, var(--border-strong) 80%, transparent), transparent);
  }

  .sm-item {
    position: relative;
    display: flex; align-items: center; gap: 10px;
    width: 100%; padding: 7px 10px 7px 12px;
    background: transparent; border: 0;
    color: var(--fg); font: inherit; font-size: var(--fs-sm);
    text-align: left; border-radius: var(--radius); cursor: pointer;
    transition: background 90ms;
    animation: slash-item-in 260ms var(--ease-page, cubic-bezier(0.22, 1, 0.36, 1)) both;
    /* Cap the stagger — a long skills catalog shouldn't take a second to fade in. */
    animation-delay: calc(min(var(--idx, 0), 12) * 13ms);
  }
  @keyframes slash-item-in {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .sm-item:hover { background: color-mix(in oklab, var(--fg) 5%, transparent); }
  .sm-item[data-active="true"] {
    background: linear-gradient(
      100deg,
      color-mix(in oklab, var(--accent) 20%, transparent),
      color-mix(in oklab, var(--accent) 9%, transparent) 60%
    );
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 24%, transparent);
  }
  /* Active-row accent bar — slides in with the selection. */
  .sm-bar {
    position: absolute; left: 4px; top: 50%;
    width: 3px; height: 0; border-radius: 999px;
    background: var(--accent);
    transform: translateY(-50%);
    transition: height 140ms var(--ease-page, ease-out);
  }
  .sm-item[data-active="true"] .sm-bar {
    height: 60%;
    box-shadow: 0 0 8px color-mix(in oklab, var(--accent) 60%, transparent);
  }

  .sm-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 24px; height: 24px; flex-shrink: 0;
    color: var(--fg-muted); border-radius: 7px;
    background: linear-gradient(to bottom, var(--bg-elev-3), color-mix(in oklab, var(--bg-elev-2) 80%, transparent));
    box-shadow:
      inset 0 0 0 1px color-mix(in oklab, var(--fg) 8%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
    transition: color 110ms, box-shadow 110ms;
  }
  .sm-icon[data-skill="true"] { color: color-mix(in oklab, var(--accent) 72%, var(--fg-muted)); }
  .sm-item[data-active="true"] .sm-icon {
    color: var(--accent);
    box-shadow:
      inset 0 0 0 1px color-mix(in oklab, var(--accent) 36%, transparent),
      0 0 10px color-mix(in oklab, var(--accent) 22%, transparent);
  }
  .sm-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .sm-line { display: flex; align-items: baseline; gap: 7px; min-width: 0; }
  .sm-cmd { color: var(--accent); font-weight: 500; white-space: nowrap; }
  .sm-cmd b {
    font-weight: 800;
    color: var(--accent-hover, var(--accent));
    text-shadow: 0 0 10px color-mix(in oklab, var(--accent) 35%, transparent);
  }
  .sm-hint {
    color: var(--fg-faint); font-size: var(--fs-xs); font-style: italic;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .sm-badge {
    flex-shrink: 0; margin-left: auto;
    font-size: 9px; font-weight: 700; letter-spacing: 0.05em; text-transform: uppercase;
    padding: 1px 6px; border-radius: 999px;
    color: var(--fg-subtle); background: var(--bg-elev-3);
    border: 1px solid var(--border);
  }
  .sm-badge[data-src="project"] {
    color: color-mix(in oklab, var(--accent) 75%, var(--fg));
    border-color: color-mix(in oklab, var(--accent) 30%, transparent);
    background: color-mix(in oklab, var(--accent) 10%, transparent);
  }
  .sm-desc {
    color: var(--fg-subtle); font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .sm-kbd {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 18px; height: 18px; padding: 0 5px; flex-shrink: 0;
    font: inherit; font-size: 10px;
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--fg-muted);
    background: var(--bg-elev-3);
    border: 1px solid var(--border); border-radius: 4px;
    box-shadow: inset 0 -1px 0 var(--border);
  }
  .sm-item[data-active="true"] .sm-kbd {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 35%, var(--border));
  }
  .sm-empty {
    display: flex; flex-direction: column; align-items: center; gap: 4px;
    padding: 26px 16px;
    text-align: center;
  }
  .sm-empty-icon { color: var(--fg-faint); margin-bottom: 2px; }
  .sm-empty-title { color: var(--fg-muted); font-size: var(--fs-sm); font-weight: 600; }
  .sm-empty-sub { color: var(--fg-subtle); font-size: var(--fs-xs); }
  .sm-foot {
    display: flex; align-items: center; gap: 12px;
    padding: 7px 12px;
    border-top: 1px solid color-mix(in oklab, var(--border) 70%, transparent);
    font-size: 10px; color: var(--fg-faint);
    background: color-mix(in oklab, var(--bg-elev-1) 55%, transparent);
  }
  .sm-foot span { display: inline-flex; align-items: center; gap: 5px; }
  .mono { font-family: var(--font-mono, ui-monospace, monospace); }
  @media (prefers-reduced-motion: reduce) {
    .slash-menu, .sm-item { animation: none; }
    .sm-bar { transition: none; }
  }
</style>
