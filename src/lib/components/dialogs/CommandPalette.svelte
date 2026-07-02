<script lang="ts">
  import { tick, untrack } from "svelte";
  import { fade, scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import {
    Search, Home, MessageSquare,
    Settings as SettingsIcon, Plus, Palette,
    Sparkles, Mic, Info, History,
    SplitSquareHorizontal, AppWindow,
  } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { commandPalette, type SettingsSection } from "../../state/command-palette.svelte";
  import { workspace, type WorkspaceId } from "../../state/workspace.svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { projects } from "../../state/projects.svelte";
  import { relTime } from "../workspace/hubHelpers";
  import { leafName } from "$lib/utils/path";

  // lucide-svelte 1.x ships icons as legacy components — `typeof Search`
  // matches what the workspaces registry uses (see workspaces/index.ts).
  type Icon = typeof Search;

  type Item = {
    id: string;
    label: string;
    /** Muted metadata rendered right-aligned (e.g. "exfil-v2 · 2h ago"). */
    sub?: string;
    /** Keyboard shortcut rendered as a key chip (e.g. "Ctrl+1"). */
    kbd?: string;
    group: "Go to" | "Recent chats" | "Actions";
    icon: Icon;
    tone?: "accent" | "info" | "warn" | "danger" | "ok" | "neutral";
    keywords?: string;
    run: () => void | Promise<void>;
  };

  let input: HTMLInputElement | undefined = $state();
  let panelEl: HTMLDivElement | undefined = $state();
  let query = $state("");
  let activeIdx = $state(0);
  let listEl: HTMLDivElement | undefined = $state();
  // Stamped on each open — relTime anchor for the recent-chat rows.
  let nowTs = $state(Date.now());

  // ── Items, computed reactively so palette stays fresh while open ────
  const items = $derived.by<Item[]>(() => {
    const out: Item[] = [];

    // Workspace navigation
    // Labels match the sidebar nav + workspaces registry — "Workspace" is the
    // home/dashboard surface (NOT "Home", which collided with the empty-chat
    // label). Keeps "Go to …" consistent across palette, sidebar, and titlebar.
    const navs: { id: WorkspaceId; label: string; icon: Icon; kbd: string }[] = [
      { id: "home",     label: "Workspace", icon: Home,          kbd: "Ctrl+1" },
      { id: "chat",     label: "Chat",      icon: MessageSquare, kbd: "Ctrl+2" },
      { id: "settings", label: "Settings",  icon: SettingsIcon,  kbd: "Ctrl+3" },
    ];
    for (const n of navs) {
      out.push({
        id: `nav:${n.id}`,
        label: `Go to ${n.label}`,
        kbd: n.kbd,
        group: "Go to",
        icon: n.icon,
        keywords: `workspace pane home ${n.id}`,
        run: () => workspace.setActive(n.id),
      });
    }

    // Settings sections (deep-link)
    const sects: { id: SettingsSection; label: string; icon: Icon; kw?: string }[] = [
      { id: "appearance", label: "Appearance", icon: Palette },
      { id: "chat",       label: "Claude",     icon: Sparkles, kw: "chat assistant accessibility reading comfort session keys cost plan" },
      { id: "speech",     label: "Speech",     icon: Mic },
      { id: "about",      label: "About",      icon: Info, kw: "shortcuts keyboard tools help diagnostics" },
    ];
    for (const s of sects) {
      out.push({
        id: `settings:${s.id}`,
        label: `Settings — ${s.label}`,
        group: "Go to",
        icon: s.icon,
        keywords: `preferences ${s.id} options ${s.kw ?? ""}`,
        run: () => {
          commandPalette.requestSettingsSection(s.id);
          workspace.setActive("settings");
        },
      });
    }

    // Recent chats — last 8 by real activity, tagged with their project so a
    // row reads like the hub cards ("exfil-v2 · 2h ago"), not a model id dump.
    const recents = [...assistant.conversations]
      .sort((a, b) => (b.lastActivityAt ?? b.createdAt) - (a.lastActivityAt ?? a.createdAt))
      .slice(0, 8);
    for (const c of recents) {
      const proj = projects.byRoot(c.workspaceRoot)?.name
        ?? (c.workspaceRoot ? leafName(c.workspaceRoot) : null);
      out.push({
        id: `chat:${c.id}`,
        label: c.title || "Untitled chat",
        sub: `${proj ? `${proj} · ` : ""}${relTime(c.lastActivityAt ?? c.createdAt, nowTs)}`,
        group: "Recent chats",
        icon: History,
        keywords: `${c.model} ${proj ?? ""}`,
        run: () => {
          workspace.setActive("chat");
          void assistant.openTab(c.id);
        },
      });
    }

    // Actions — includes the window utilities (split / new window) so they stay
    // keyboard-reachable after the old topbar dropdown was dissolved.
    out.push({
      id: "act:new-chat",
      label: "New chat tab",
      kbd: "Ctrl+T",
      group: "Actions",
      icon: Plus,
      tone: "accent",
      keywords: "create open",
      run: () => {
        workspace.setActive("chat");
        void assistant.newTab();
      },
    });
    if (workspace.activeId === "chat" && assistant.canAddPane) {
      out.push({
        id: "act:split",
        label: "Split editor",
        kbd: "Ctrl+\\",
        group: "Actions",
        icon: SplitSquareHorizontal,
        keywords: "pane second side by side",
        run: () => assistant.addPane(),
      });
    }
    out.push({
      id: "act:new-window",
      label: "New window",
      sub: "Separate Rift window",
      group: "Actions",
      icon: AppWindow,
      keywords: "open separate",
      run: () => { void invoke("open_new_window").catch(console.error); },
    });
    return out;
  });

  // ── Fuzzy filter ────────────────────────────────────────────────────
  // Substring matches rank across every field; the subsequence fallback only
  // runs against the LABEL — letter-scatter across keywords/group text matched
  // practically everything ("speech" hit "Go to Chat" via its keyword soup).
  function score(item: Item, q: string): number {
    if (!q) return 1;
    const hay = `${item.label} ${item.sub ?? ""} ${item.keywords ?? ""} ${item.group}`.toLowerCase();
    const needle = q.toLowerCase();
    if (hay.includes(needle)) return 1000 - hay.indexOf(needle);
    // Subsequence over the label only
    const label = item.label.toLowerCase();
    let hi = 0;
    let prevMatch = -1;
    let runScore = 0;
    for (let ni = 0; ni < needle.length; ni++) {
      const c = needle[ni];
      const found = label.indexOf(c, hi);
      if (found < 0) return 0;
      // contiguous-run bonus
      if (found === prevMatch + 1) runScore += 5;
      prevMatch = found;
      hi = found + 1;
    }
    return 100 + runScore - (prevMatch / 4);
  }

  const filtered = $derived.by(() => {
    const q = query.trim();
    const scored = items
      .map((it) => ({ it, s: score(it, q) }))
      .filter((x) => x.s > 0);
    scored.sort((a, b) => b.s - a.s);
    return scored.map((x) => x.it);
  });

  const grouped = $derived.by(() => {
    const order: Item["group"][] = ["Go to", "Recent chats", "Actions"];
    const map = new Map<string, Item[]>();
    for (const it of filtered) {
      const arr = map.get(it.group) ?? [];
      arr.push(it);
      map.set(it.group, arr);
    }
    return order
      .filter((g) => map.has(g))
      .map((g) => ({ group: g, items: map.get(g)! }));
  });

  // Flat list used for keyboard cursor — must mirror the rendered order.
  const flat = $derived(grouped.flatMap((g) => g.items));
  const flatIdx = $derived(new Map(flat.map((it, i) => [it.id, i])));

  // ── Open / focus / keyboard ─────────────────────────────────────────
  $effect(() => {
    if (commandPalette.open) {
      query = "";
      activeIdx = 0;
      nowTs = Date.now();
      void (async () => {
        await tick();
        input?.focus();
      })();
    }
  });

  // Refresh conversation list each open
  $effect(() => {
    void commandPalette.openTick;
    if (commandPalette.open) {
      void assistant.refreshConversations();
    }
  });

  // Clamp cursor when filter shrinks the list
  $effect(() => {
    const len = flat.length;
    untrack(() => { if (activeIdx >= len) activeIdx = Math.max(0, len - 1); });
  });

  function onKeydown(e: KeyboardEvent) {
    if (!commandPalette.open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      commandPalette.hide();
      return;
    }
    if (e.key === "Tab") {
      // Cycle focus within the panel: input + enabled buttons
      if (!panelEl) return;
      e.preventDefault();
      const focusable = Array.from(
        panelEl.querySelectorAll<HTMLElement>("input, button:not([disabled])")
      );
      if (!focusable.length) return;
      const cur = document.activeElement as HTMLElement;
      const idx = focusable.indexOf(cur);
      if (e.shiftKey) {
        const prev = idx <= 0 ? focusable[focusable.length - 1] : focusable[idx - 1];
        prev.focus();
      } else {
        const next = idx < 0 || idx >= focusable.length - 1 ? focusable[0] : focusable[idx + 1];
        next.focus();
      }
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (flat.length) activeIdx = (activeIdx + 1) % flat.length;
      scrollActiveIntoView();
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (flat.length) activeIdx = (activeIdx - 1 + flat.length) % flat.length;
      scrollActiveIntoView();
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const item = flat[activeIdx];
      if (item) {
        void item.run();
        commandPalette.hide();
      }
      return;
    }
  }

  async function scrollActiveIntoView() {
    await tick();
    if (!listEl) return;
    const el = listEl.querySelector<HTMLElement>(`[data-idx="${activeIdx}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }

  function pick(item: Item) {
    void item.run();
    commandPalette.hide();
  }

  function backdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) commandPalette.hide();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if commandPalette.open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="cp-scrim" role="presentation" onclick={backdropClick} transition:fade={{ duration: 130 }}>
    <div
      class="cp-panel"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      bind:this={panelEl}
      transition:scale={{ start: 0.97, duration: 170, easing: quintOut }}
    >
      <div class="cp-search">
        <Search size={15} />
        <input
          bind:this={input}
          bind:value={query}
          type="text"
          placeholder="Search chats, pages, and commands…"
          aria-label="Search commands and chats"
          autocomplete="off"
          spellcheck="false"
        />
        <kbd class="cp-hint">Esc</kbd>
      </div>

      <div class="cp-list" role="list" aria-label="Results" bind:this={listEl}>
        {#if flat.length === 0}
          <div class="cp-empty">
            <span class="cp-empty-t">No matches</span>
            <span class="cp-empty-s">Nothing found for “{query.trim()}”</span>
          </div>
        {:else}
          {#each grouped as g (g.group)}
            <div class="cp-group-label">{g.group}</div>
            {#each g.items as it (it.id)}
              {@const idx = flatIdx.get(it.id) ?? 0}
              {@const Icon = it.icon}
              <!-- svelte-ignore a11y_mouse_events_have_key_events -->
              <button
                type="button"
                class="cp-item"
                data-active={idx === activeIdx}
                data-tone={it.tone ?? "neutral"}
                data-idx={idx}
                onmouseenter={() => (activeIdx = idx)}
                onclick={() => pick(it)}
              >
                <span class="cp-item-icon"><Icon size={14} /></span>
                <span class="cp-item-label">{it.label}</span>
                {#if it.sub}<span class="cp-item-sub">{it.sub}</span>{/if}
                {#if it.kbd}<kbd class="cp-hint cp-item-kbd">{it.kbd}</kbd>{/if}
                <span class="cp-item-go" class:show={idx === activeIdx} aria-hidden="true">
                  <kbd class="cp-hint">↵</kbd>
                </span>
              </button>
            {/each}
          {/each}
        {/if}
      </div>

      <footer class="cp-foot">
        <span><kbd class="cp-hint">↑↓</kbd> navigate</span>
        <span><kbd class="cp-hint">↵</kbd> open</span>
        <span class="cp-foot-r"><kbd class="cp-hint">Esc</kbd> close</span>
      </footer>
    </div>
  </div>
{/if}

<style>
  .cp-scrim {
    position: fixed; inset: 0;
    z-index: 200;
    background: color-mix(in oklch, var(--bg) 55%, transparent);
    backdrop-filter: blur(8px) saturate(135%);
    -webkit-backdrop-filter: blur(8px) saturate(135%);
    display: flex; justify-content: center; align-items: flex-start;
    padding-top: 13vh;
  }
  .cp-panel {
    width: min(620px, calc(100vw - 32px));
    max-height: 68vh;
    display: flex; flex-direction: column;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-2xl);
    box-shadow:
      0 24px 64px -16px rgba(0, 0, 0, 0.6),
      0 4px 16px -8px rgba(0, 0, 0, 0.45),
      0 0 80px -32px color-mix(in oklab, var(--accent) 30%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
    overflow: hidden;
  }

  .cp-search {
    display: flex; align-items: center; gap: 11px;
    height: 48px; flex: none;
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
  }
  .cp-search :global(svg) { color: var(--accent); opacity: 0.85; flex-shrink: 0; }
  .cp-search input {
    flex: 1; min-width: 0;
    background: transparent; border: 0; outline: 0;
    color: var(--fg);
    caret-color: var(--accent);
    font: inherit; font-size: 14px;
    letter-spacing: -0.01em;
  }
  .cp-search input::placeholder { color: var(--fg-faint); }

  .cp-hint {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 18px; height: 18px;
    padding: 0 5px;
    font: inherit; font-size: 10px;
    font-family: var(--font-mono, ui-monospace, monospace);
    color: var(--fg-muted);
    background: var(--bg-elev-3);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: inset 0 -1px 0 var(--border);
  }

  .cp-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }

  .cp-group-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--fg-faint);
    padding: 13px 12px 5px;
  }
  .cp-group-label:first-child { padding-top: 7px; }

  /* Single-line rows: icon chip · label · (metadata | kbd) · ↵-on-active.
     One height, one baseline — the old stacked two-line rows read as clutter. */
  .cp-item {
    display: flex; align-items: center; gap: 10px;
    width: 100%;
    height: 36px;
    padding: 0 10px;
    background: transparent; border: 0;
    color: var(--fg);
    font: inherit; font-size: var(--fs-sm);
    text-align: left;
    border-radius: 10px;
    cursor: pointer;
    transition: background 80ms;
  }
  .cp-item[data-active="true"] {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 26%, transparent);
  }
  .cp-item[data-active="true"] .cp-item-label { color: var(--accent); }
  .cp-item:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }

  .cp-item-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 25px; height: 25px;
    color: var(--fg-muted);
    background: var(--bg-elev-3);
    border-radius: 7px;
    flex-shrink: 0;
    transition: color 100ms, background 100ms;
  }
  .cp-item[data-active="true"] .cp-item-icon {
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 18%, transparent);
  }
  .cp-item[data-tone="ok"]     .cp-item-icon { color: var(--ok); }
  .cp-item[data-tone="warn"]   .cp-item-icon { color: var(--warn); }
  .cp-item[data-tone="danger"] .cp-item-icon { color: var(--danger); }
  .cp-item[data-tone="info"]   .cp-item-icon { color: var(--info); }
  .cp-item[data-tone="accent"] .cp-item-icon { color: var(--accent); }

  .cp-item-label {
    flex: 1; min-width: 0;
    color: var(--fg);
    font-weight: 520;
    letter-spacing: -0.005em;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .cp-item-sub {
    flex: none; max-width: 40%;
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .cp-item-kbd { flex: none; }
  /* Reserved slot so rows don't shift width when the cursor moves. */
  .cp-item-go { flex: none; width: 20px; display: inline-flex; justify-content: flex-end; visibility: hidden; }
  .cp-item-go.show { visibility: visible; }

  .cp-empty {
    display: flex; flex-direction: column; align-items: center; gap: 4px;
    padding: 28px 22px;
    text-align: center;
  }
  .cp-empty-t { color: var(--fg-2); font-size: var(--fs-md); font-weight: 600; }
  .cp-empty-s { color: var(--fg-faint); font-size: var(--fs-xs); overflow-wrap: anywhere; }

  .cp-foot {
    display: flex; align-items: center; gap: 14px;
    padding: 8px 14px; flex: none;
    border-top: 1px solid var(--border);
    background: color-mix(in oklab, var(--fg) 2%, transparent);
    font-size: var(--fs-xs);
    color: var(--fg-faint);
  }
  .cp-foot span { display: inline-flex; align-items: center; gap: 6px; }
  .cp-foot-r { margin-left: auto; }
</style>
