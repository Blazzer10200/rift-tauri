<script lang="ts">
  import { tick, untrack } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import {
    Search, Home, MessageSquare, Activity,
    Settings as SettingsIcon, Plus, Palette, Accessibility as A11yIcon,
    Sparkles, Mic, Info, History,
  } from "lucide-svelte";
  import { commandPalette, type SettingsSection } from "../../state/command-palette.svelte";
  import { workspace, type WorkspaceId } from "../../state/workspace.svelte";
  import { assistant } from "../../state/assistant.svelte";

  // lucide-svelte 1.x ships icons as legacy components — `typeof Search`
  // matches what the workspaces registry uses (see workspaces/index.ts).
  type Icon = typeof Search;

  type Item = {
    id: string;
    label: string;
    sub?: string;
    group: "Go to" | "Recent chats" | "Actions";
    icon: Icon;
    tone?: "accent" | "info" | "warn" | "danger" | "ok" | "neutral";
    keywords?: string;
    run: () => void | Promise<void>;
  };

  let input: HTMLInputElement | undefined = $state();
  let query = $state("");
  let activeIdx = $state(0);
  let listEl: HTMLDivElement | undefined = $state();

  // ── Items, computed reactively so palette stays fresh while open ────
  const items = $derived.by<Item[]>(() => {
    const out: Item[] = [];

    // Workspace navigation
    const navs: { id: WorkspaceId; label: string; icon: Icon; sub: string }[] = [
      { id: "home",     label: "Home",     icon: Home,          sub: "Ctrl+1" },
      { id: "chat",     label: "Chat",     icon: MessageSquare, sub: "Ctrl+2" },
      { id: "harness",  label: "Harness",  icon: Activity,      sub: "Ctrl+3" },
      { id: "settings", label: "Settings", icon: SettingsIcon,  sub: "Ctrl+4" },
    ];
    for (const n of navs) {
      out.push({
        id: `nav:${n.id}`,
        label: `Go to ${n.label}`,
        sub: n.sub,
        group: "Go to",
        icon: n.icon,
        keywords: `workspace pane ${n.id}`,
        run: () => workspace.setActive(n.id),
      });
    }

    // Settings sections (deep-link)
    const sects: { id: SettingsSection; label: string; icon: Icon }[] = [
      { id: "appearance",    label: "Appearance",    icon: Palette },
      { id: "accessibility", label: "Accessibility", icon: A11yIcon },
      { id: "assistant",     label: "Assistant",     icon: Sparkles },
      { id: "speech",        label: "Speech",        icon: Mic },
      { id: "about",         label: "About",         icon: Info },
    ];
    for (const s of sects) {
      out.push({
        id: `settings:${s.id}`,
        label: `Settings — ${s.label}`,
        sub: `Jump to ${s.label.toLowerCase()} settings`,
        group: "Go to",
        icon: s.icon,
        keywords: `preferences ${s.id} options`,
        run: () => {
          commandPalette.requestSettingsSection(s.id);
          workspace.setActive("settings");
        },
      });
    }

    // Recent chats — last 8 by updatedAt
    const recents = [...assistant.conversations]
      .sort((a, b) => b.updatedAt - a.updatedAt)
      .slice(0, 8);
    for (const c of recents) {
      out.push({
        id: `chat:${c.id}`,
        label: c.title || "Untitled chat",
        sub: `${c.model} · ${c.messageCount} msg`,
        group: "Recent chats",
        icon: History,
        keywords: c.model,
        run: () => {
          workspace.setActive("chat");
          void assistant.openTab(c.id);
        },
      });
    }

    // Actions
    out.push({
      id: "act:new-chat",
      label: "New chat tab",
      sub: "Ctrl+T",
      group: "Actions",
      icon: Plus,
      tone: "accent",
      keywords: "create open",
      run: () => {
        workspace.setActive("chat");
        void assistant.newTab();
      },
    });
    return out;
  });

  // ── Fuzzy filter ────────────────────────────────────────────────────
  // Simple subsequence match — every query char must appear in order,
  // case-insensitive. Hits early matches over scattered ones.
  function score(item: Item, q: string): number {
    if (!q) return 1;
    const hay = `${item.label} ${item.sub ?? ""} ${item.keywords ?? ""} ${item.group}`.toLowerCase();
    const needle = q.toLowerCase();
    if (hay.includes(needle)) return 1000 - hay.indexOf(needle);
    // Subsequence
    let hi = 0;
    let prevMatch = -1;
    let runScore = 0;
    for (let ni = 0; ni < needle.length; ni++) {
      const c = needle[ni];
      const found = hay.indexOf(c, hi);
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
  <div class="cp-scrim" role="presentation" onclick={backdropClick} transition:fade={{ duration: 120 }}>
    <div
      class="cp-panel"
      role="dialog"
      aria-label="Command palette"
      transition:fly={{ y: -8, duration: 160, easing: quintOut }}
    >
      <div class="cp-search">
        <Search size={14} />
        <input
          bind:this={input}
          bind:value={query}
          type="text"
          placeholder="Search or run a command…"
          autocomplete="off"
          spellcheck="false"
        />
        <kbd class="cp-hint">Esc</kbd>
      </div>

      <div class="cp-list" bind:this={listEl}>
        {#if flat.length === 0}
          <div class="cp-empty">No matches</div>
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
                <span class="cp-item-icon"><Icon size={13} /></span>
                <span class="cp-item-l">
                  <span class="cp-item-label">{it.label}</span>
                  {#if it.sub}<span class="cp-item-sub">{it.sub}</span>{/if}
                </span>
                {#if idx === activeIdx}
                  <span class="cp-item-r"><kbd class="cp-hint">↵</kbd></span>
                {/if}
              </button>
            {/each}
          {/each}
        {/if}
      </div>

      <footer class="cp-foot">
        <span><kbd class="cp-hint">↑↓</kbd> navigate</span>
        <span><kbd class="cp-hint">↵</kbd> run</span>
        <span><kbd class="cp-hint">Esc</kbd> close</span>
      </footer>
    </div>
  </div>
{/if}

<style>
  .cp-scrim {
    position: fixed; inset: 0;
    z-index: 200;
    background: color-mix(in oklch, var(--bg) 60%, transparent);
    backdrop-filter: blur(6px) saturate(140%);
    -webkit-backdrop-filter: blur(6px) saturate(140%);
    display: flex; justify-content: center; align-items: flex-start;
    padding-top: 14vh;
  }
  .cp-panel {
    width: min(640px, calc(100vw - 32px));
    max-height: 70vh;
    display: flex; flex-direction: column;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg);
    box-shadow:
      0 16px 48px -12px rgba(0, 0, 0, 0.55),
      0 2px 8px -4px rgba(0, 0, 0, 0.4),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    overflow: hidden;
  }

  .cp-search {
    display: flex; align-items: center; gap: 10px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(to bottom,
      color-mix(in oklab, var(--accent) 4%, transparent),
      transparent);
  }
  .cp-search :global(svg) { color: var(--fg-muted); flex-shrink: 0; }
  .cp-search input {
    flex: 1;
    background: transparent; border: 0; outline: 0;
    color: var(--fg);
    font: inherit; font-size: var(--fs-md);
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
    padding: 4px;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }

  .cp-group-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-subtle);
    padding: 12px 12px 5px;
  }
  .cp-group-label:first-child { padding-top: 6px; }

  .cp-item {
    display: flex; align-items: center; gap: 10px;
    width: 100%;
    padding: 7px 10px;
    background: transparent; border: 0;
    color: var(--fg);
    font: inherit; font-size: var(--fs-sm);
    text-align: left;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 80ms;
  }
  .cp-item[data-active="true"] {
    background: color-mix(in oklab, var(--accent) 16%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--accent) 26%, transparent);
  }
  .cp-item:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }

  .cp-item-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 22px; height: 22px;
    color: var(--fg-muted);
    background: var(--bg-elev-3);
    border-radius: 6px;
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

  .cp-item-l {
    flex: 1; min-width: 0;
    display: flex; flex-direction: column; gap: 1px;
  }
  .cp-item-label {
    color: var(--fg);
    font-weight: 500;
    letter-spacing: -0.005em;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .cp-item-sub {
    color: var(--fg-subtle);
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .cp-item-r { flex-shrink: 0; }

  .cp-empty {
    padding: 22px;
    text-align: center;
    color: var(--fg-faint);
    font-size: var(--fs-sm);
  }

  .cp-foot {
    display: flex; align-items: center; gap: 14px;
    padding: 7px 14px;
    border-top: 1px solid var(--border);
    background: var(--bg);
    font-size: var(--fs-xs);
    color: var(--fg-faint);
  }
  .cp-foot span { display: inline-flex; align-items: center; gap: 6px; }
</style>
