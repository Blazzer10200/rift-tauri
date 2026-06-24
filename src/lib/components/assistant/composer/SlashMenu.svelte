<script lang="ts">
  // C6 (per docs/design/composer-split.md) — the `/command` popover. Render +
  // click only: the keyboard nav index and open/filter state stay owned by the
  // parent's onKey. Restyled 2026-06-10 (ui-audit #2) to share the Ctrl+K
  // palette's design language: compact left-anchored panel, boxed icons,
  // grouped rows, bold matched prefix, kbd hints.
  import {
    Plus, Eraser, Cpu, RotateCcw, Copy, StopCircle,
    Wrench, Coins, Gauge, BarChart3, Terminal, ClipboardCopy, HelpCircle,
    Palette, LogIn,
  } from "lucide-svelte";

  type Icon = typeof Plus;

  let {
    commands,
    activeIdx,
    query = "",
    onPick,
  }: {
    commands: { name: string; desc: string }[];
    activeIdx: number;
    query?: string;
    onPick: (c: { name: string; desc: string }) => void;
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
    cost:      { icon: Coins,         group: "Info" },
    usage:     { icon: Gauge,         group: "Info" },
    stats:     { icon: BarChart3,     group: "Info" },
    openincli: { icon: Terminal,      group: "Info" },
    diag:      { icon: ClipboardCopy, group: "Info" },
    help:      { icon: HelpCircle,    group: "Info" },
    "design-sync":  { icon: Palette, group: "Design" },
    "design-login": { icon: LogIn,   group: "Design" },
  };
  const groupOf = (name: string) => META[name]?.group ?? "Commands";

  let listEl = $state<HTMLDivElement | undefined>();
  $effect(() => {
    const el = listEl?.querySelector<HTMLElement>(`[data-idx="${activeIdx}"]`);
    el?.scrollIntoView({ block: "nearest" });
  });
</script>

<div class="rift-menu slash-menu" role="menu" aria-label="Slash commands">
  <div class="sm-list" bind:this={listEl}>
    {#each commands as c, i (c.name)}
      {@const Icon = META[c.name]?.icon ?? Terminal}
      {#if i === 0 || groupOf(c.name) !== groupOf(commands[i - 1].name)}
        <div class="sm-group" role="separator">{groupOf(c.name)}</div>
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
        <span class="sm-icon"><Icon size={13} /></span>
        <span class="sm-body">
          <span class="sm-cmd mono">/{#if query}<b>{c.name.slice(0, query.length)}</b>{c.name.slice(query.length)}{:else}{c.name}{/if}</span>
          <span class="sm-desc">{c.desc}</span>
        </span>
        {#if i === activeIdx}<kbd class="sm-kbd">↵</kbd>{/if}
      </button>
    {/each}
  </div>
  <footer class="sm-foot">
    <span><kbd class="sm-kbd">↑↓</kbd> select</span>
    <span><kbd class="sm-kbd">⇥/↵</kbd> pick</span>
    <span><kbd class="sm-kbd">Esc</kbd> cancel</span>
  </footer>
</div>

<style>
  /* Shares the global .rift-menu chrome; sizing matches the Ctrl+K palette
     grammar instead of spanning the full composer width. */
  .slash-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: min(420px, 100%);
    display: flex; flex-direction: column;
    overflow: hidden;
    z-index: 10;
    animation: slash-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .sm-list { max-height: 300px; overflow-y: auto; padding: 4px; scrollbar-width: thin; }

  .sm-group {
    font-size: 10px; font-weight: 700;
    text-transform: uppercase; letter-spacing: 0.06em;
    color: var(--fg-subtle);
    padding: 10px 10px 4px;
  }
  .sm-group:first-child { padding-top: 5px; }

  .sm-item {
    display: flex; align-items: center; gap: 10px;
    width: 100%; padding: 6px 9px;
    background: transparent; border: 0;
    color: var(--fg); font: inherit; font-size: var(--fs-sm);
    text-align: left; border-radius: var(--radius-sm); cursor: pointer;
    transition: background 80ms;
    animation: slash-item-in 280ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(var(--idx, 0) * 16ms);
  }
  @keyframes slash-item-in {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .sm-item[data-active="true"] {
    background: color-mix(in oklab, var(--accent) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 26%, transparent);
  }
  .sm-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 22px; height: 22px; flex-shrink: 0;
    color: var(--fg-muted); background: var(--bg-elev-3); border-radius: 6px;
    transition: color 100ms, background 100ms;
  }
  .sm-item[data-active="true"] .sm-icon {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 18%, transparent);
  }
  .sm-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .sm-cmd { color: var(--accent); font-weight: 500; }
  .sm-cmd b { font-weight: 750; }
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
  .sm-foot {
    display: flex; align-items: center; gap: 12px;
    padding: 6px 12px;
    border-top: 1px solid var(--border);
    font-size: 10px; color: var(--fg-faint);
  }
  .sm-foot span { display: inline-flex; align-items: center; gap: 5px; }
  .mono { font-family: var(--font-mono, ui-monospace, monospace); }
  @media (prefers-reduced-motion: reduce) {
    .slash-menu, .sm-item { animation: none; }
  }
</style>
