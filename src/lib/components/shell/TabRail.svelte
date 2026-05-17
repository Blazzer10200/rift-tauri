<script lang="ts">
  import { FolderOpen, Activity, TriangleAlert, Cog, Download, RefreshCcw, Sparkles, ChevronRight, Pin, PinOff, ListChecks, History, Bot, TerminalSquare, Paperclip } from "lucide-svelte";
  import { connection } from "../../state/connection.svelte";
  import { updates } from "../../state/updates.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";
  import type { PanelId } from "../../state/panel-types";

  type Tab = "browse" | "activity" | "sync" | "assistant" | "conflicts" | "settings" | "diagnostics";
  type RailMode = "tabs" | "panels";

  let { mode = "tabs", active, onChange, onPanelToggle, onOpenSettings }: {
    mode?: RailMode;
    active: Tab;
    onChange: (t: Tab) => void;
    // Phase C Part 2: shift-click bypasses accordion mode (lets user keep
    // multiple panels open at once). Plumbed via the second arg.
    onPanelToggle?: (id: PanelId, opts?: { allowMulti?: boolean }) => void;
    onOpenSettings?: () => void;
  } = $props();

  type RailTone = "accent" | "info" | "danger" | "neutral";
  // `id` is intentionally `string` so the same shape holds tab IDs and panel IDs.
  type ItemDef = { id: string; label: string; icon: typeof FolderOpen; kbd: string; tone: RailTone; count?: () => number; countCls?: string; beta?: boolean };
  type ItemGroup = { id: string; items: ItemDef[] };

  // v0.2 tab groups — workspace tools / AI / status / admin.
  const tabGroups: ItemGroup[] = [
    {
      id: "workspace",
      items: [
        { id: "browse", label: "Files", icon: FolderOpen, kbd: "1", tone: "accent" },
        { id: "sync",   label: "Sync",  icon: RefreshCcw, kbd: "2", tone: "accent" },
      ],
    },
    {
      id: "ai",
      items: [
        { id: "assistant", label: "Assistant", icon: Sparkles, kbd: "3", tone: "info", beta: true },
      ],
    },
    {
      id: "status",
      items: [
        { id: "conflicts", label: "Conflicts", icon: TriangleAlert, kbd: "4", tone: "danger", count: () => connection.conflictCount, countCls: "danger" },
        { id: "activity",  label: "Activity",  icon: Activity,      kbd: "5", tone: "info",   count: () => connection.activityFeed.length, countCls: "" },
      ],
    },
  ];
  const tabFooter: ItemDef[] = [
    { id: "settings", label: "Settings", icon: Cog, kbd: "6", tone: "neutral" },
  ];

  // v0.3 panel groups — chat is permanent center; rail toggles dock panels.
  const panelGroups: ItemGroup[] = [
    {
      id: "workspace",
      items: [
        { id: "files", label: "Files", icon: FolderOpen, kbd: "3", tone: "accent" },
        { id: "sync",  label: "Sync",  icon: RefreshCcw, kbd: "2", tone: "accent" },
      ],
    },
    {
      id: "work",
      items: [
        { id: "tasks",   label: "Tasks",   icon: ListChecks, kbd: "1", tone: "accent" },
        { id: "agents",  label: "Agents",  icon: Bot,        kbd: "5", tone: "info", beta: true },
        { id: "history", label: "History", icon: History,    kbd: "4", tone: "neutral" },
      ],
    },
    {
      id: "tools",
      items: [
        { id: "terminal",    label: "Terminal",    icon: TerminalSquare, kbd: "6", tone: "neutral" },
        { id: "attachments", label: "Attachments", icon: Paperclip,      kbd: "7", tone: "neutral" },
      ],
    },
    {
      id: "status",
      items: [
        { id: "activity", label: "Activity", icon: Activity, kbd: "8", tone: "info", count: () => connection.activityFeed.length, countCls: "" },
      ],
    },
  ];
  const panelFooter: ItemDef[] = [
    { id: "settings", label: "Settings", icon: Cog, kbd: ",", tone: "neutral" },
  ];

  const groups = $derived(mode === "panels" ? panelGroups : tabGroups);
  const footer = $derived(mode === "panels" ? panelFooter : tabFooter);

  function isActive(itemId: string): boolean {
    if (mode === "panels") {
      if (itemId === "settings") return false;
      return uiPrefs.panels[itemId as PanelId]?.open ?? false;
    }
    return active === itemId;
  }

  function handleClick(itemId: string, ev?: MouseEvent | KeyboardEvent) {
    if (mode === "panels") {
      if (itemId === "settings") { onOpenSettings?.(); return; }
      onPanelToggle?.(itemId as PanelId, { allowMulti: !!ev?.shiftKey });
      return;
    }
    onChange(itemId as Tab);
  }

  const activeFooterIdx = $derived(footer.findIndex((t) => isActive(t.id)));
</script>

<aside class="rail" class:pinned={uiPrefs.railPinned} aria-label="Primary navigation">
  <div class="rail-panel">
    <div class="rail-brand" aria-hidden="true">
      <img class="brand-mark" src="/favicon.png" alt="" draggable="false"/>
      <span class="brand-word">RIFT</span>
    </div>
    <button
      class="rail-pin"
      type="button"
      onclick={(e) => { uiPrefs.toggleRailPinned(); (e.currentTarget as HTMLButtonElement).blur(); }}
      title={uiPrefs.railPinned ? "Unpin rail (collapse on leave)" : "Pin rail open"}
      aria-label={uiPrefs.railPinned ? "Unpin rail (collapse on leave)" : "Pin rail open"}
      aria-pressed={uiPrefs.railPinned}
    >
      {#if uiPrefs.railPinned}
        <PinOff size={12}/>
      {:else}
        <ChevronRight size={12}/>
      {/if}
    </button>
    {#each groups as g, gi (g.id)}
      {@const activeIdxInGroup = g.items.findIndex((t) => isActive(t.id))}
      {#if gi > 0}
        <div class="group-divider" aria-hidden="true"></div>
      {/if}
      <div class="group" style="--active-y: {Math.max(0, activeIdxInGroup) * 31}px">
        {#if activeIdxInGroup >= 0}
          <div class="rail-indicator" aria-hidden="true" data-tone={g.items[activeIdxInGroup].tone}></div>
        {/if}
        {#each g.items as t (t.id)}
          {@const Icon = t.icon}
          {@const c = t.count ? t.count() : 0}
          <button
            class="rail-btn"
            data-active={isActive(t.id)}
            data-tone={t.tone}
            data-panel-id={mode === "panels" ? t.id : null}
            onclick={(e) => { handleClick(t.id, e); (e.currentTarget as HTMLButtonElement).blur(); }}
            title="{t.label} (Ctrl+{t.kbd})"
            type="button"
          >
            <span class="rail-icon"><Icon size={16}/></span>
            <span class="label">{t.label}</span>
            {#if t.beta}<span class="beta-pill" title="Beta — use at your own risk">beta</span>{/if}
            <span class="kbd-hint mono">{t.kbd}</span>
            {#if c > 0}
              <span class="count-pip {t.countCls ?? ''}">{c}</span>
            {/if}
          </button>
        {/each}
      </div>
    {/each}

    <div class="bottom">
      <div class="divider" aria-hidden="true"></div>
      <div class="group footer" style="--active-y: {Math.max(0, activeFooterIdx) * 31}px">
        {#if activeFooterIdx >= 0}
          <div class="rail-indicator" aria-hidden="true" data-tone={footer[activeFooterIdx].tone}></div>
        {/if}
        {#each footer as t (t.id)}
          {@const Icon = t.icon}
          <button
            class="rail-btn"
            data-active={isActive(t.id)}
            data-tone={t.tone}
            onclick={(e) => { handleClick(t.id, e); (e.currentTarget as HTMLButtonElement).blur(); }}
            title="{t.label} ({mode === 'panels' ? 'Ctrl+,' : `Ctrl+${t.kbd}`})"
            type="button"
          >
            <span class="rail-icon"><Icon size={16}/></span>
            <span class="label">{t.label}</span>
            {#if t.beta}<span class="beta-pill" title="Beta — use at your own risk">beta</span>{/if}
            <span class="kbd-hint mono">{t.kbd}</span>
          </button>
        {/each}
      </div>
      {#if updates.state === "available" && updates.info}
        <button
          class="update-pill"
          type="button"
          onclick={(e) => { updates.open(); (e.currentTarget as HTMLButtonElement).blur(); }}
          title="Update {updates.info.version} available — click for details"
        >
          <span class="up-dot"></span>
          <Download size={12}/>
          <span class="up-text">
            <span class="up-l">Update available</span>
            <span class="up-v mono">{updates.info.version}</span>
          </span>
        </button>
      {/if}

    </div>
  </div>
</aside>

<style>
  .rail {
    position: relative;
    width: 48px;
    height: 100%;
  }
  .rail-panel {
    position: absolute;
    left: 0; top: 0; bottom: 0;
    width: 48px;
    background: var(--bg);
    border-right: 1px solid var(--border);
    padding: 8px 6px;
    display: flex; flex-direction: column;
    min-height: 0;
    overflow: hidden;
    container-type: inline-size;
    z-index: 20;
    transition: width 220ms cubic-bezier(0.4, 0, 0.2, 1),
                box-shadow 220ms ease,
                border-right-color 220ms ease,
                padding 220ms ease;
  }
  .rail:hover .rail-panel,
  .rail:focus-within .rail-panel,
  .rail.pinned .rail-panel {
    width: 220px;
    padding: 8px;
    box-shadow: 6px 0 24px rgba(0, 0, 0, 0.28);
    border-right-color: var(--border-strong, var(--border));
  }
  /* When pinned the rail behaves like a permanent sidebar: take real
     layout width so main content reflows next to it, not under it. */
  .rail.pinned { width: 220px; }
  .rail.pinned .rail-panel { box-shadow: none; }

  /* Rift wordmark at the top of the rail. Gives the rail a brand anchor and
     balances visual weight (was top-heavy w/ just icons). In collapsed
     state shows only the favicon mark; in expanded state the "RIFT"
     wordmark fades in alongside. */
  .rail-brand {
    display: flex; align-items: center; gap: 8px;
    height: 24px;
    padding: 0 6px;
    margin-bottom: 8px;
    color: var(--fg);
  }
  .brand-mark {
    width: 18px; height: 18px;
    border-radius: 4px;
    flex-shrink: 0;
  }
  .brand-word {
    font-weight: 700;
    font-size: var(--fs-sm);
    letter-spacing: 0.12em;
    color: var(--fg);
    opacity: 1;
    transition: opacity 140ms ease;
  }

  /* Top-of-rail pin/chevron button. Collapsed-state: ">" hint (rotates to
     "<" on hover as expand cue). Pinned-state: filled PinOff icon — click
     to release back to hover-expand. */
  .rail-pin {
    display: flex; align-items: center; justify-content: center;
    width: 24px; height: 20px;
    margin: 0 auto 6px; padding: 0;
    background: transparent; border: 0;
    color: var(--fg-faint);
    opacity: 0.65;
    cursor: pointer;
    border-radius: var(--radius-xs);
    transition: opacity 180ms ease, color 180ms ease, background 140ms ease;
  }
  .rail-pin :global(svg) {
    transition: transform 220ms cubic-bezier(0.4, 0, 0.2, 1);
  }
  .rail:not(.pinned):hover .rail-pin :global(svg),
  .rail:not(.pinned):focus-within .rail-pin :global(svg) {
    transform: rotate(180deg);
  }
  .rail:hover .rail-pin,
  .rail:focus-within .rail-pin {
    opacity: 1;
    color: var(--fg-muted);
  }
  .rail-pin:hover { background: var(--surface-hover); color: var(--fg); }
  .rail.pinned .rail-pin {
    color: var(--accent);
    opacity: 1;
  }

  .group { display: flex; flex-direction: column; gap: 1px; position: relative; }
  /* Inter-group divider — visually separates workspace / AI / status
     clusters. Hairline + reduced opacity so it reads as a subtle break,
     not a hard rule. */
  .group-divider {
    height: 1px;
    margin: 6px 8px;
    background: var(--border);
    opacity: 0.55;
  }
  .rail-indicator {
    position: absolute;
    left: -6px; top: 0;
    width: 2px; height: 30px;
    background: var(--accent);
    border-radius: 2px;
    transform: translateY(var(--active-y, 0px));
    transition: transform 220ms cubic-bezier(0.4, 0, 0.2, 1), left 220ms ease, background 180ms ease;
    pointer-events: none;
    z-index: 1;
  }
  .rail-indicator[data-tone="info"]    { background: var(--info); }
  .rail-indicator[data-tone="danger"]  { background: var(--danger); }
  .rail-indicator[data-tone="neutral"] { background: var(--fg-muted); }
  .rail:hover .rail-indicator,
  .rail:focus-within .rail-indicator { left: -8px; }

  .rail-btn {
    --tone: var(--accent);
    display: flex; align-items: center; gap: 10px;
    width: 100%; height: 30px;
    padding: 0 8px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    text-align: left; font: inherit; font-size: var(--fs-sm);
    border-radius: var(--radius-sm);
    cursor: pointer;
    position: relative;
    transition: background 140ms ease, color 140ms ease;
    white-space: nowrap;
    overflow: hidden;
  }
  .rail-btn[data-tone="info"]    { --tone: var(--info); }
  .rail-btn[data-tone="danger"]  { --tone: var(--danger); }
  .rail-btn[data-tone="neutral"] { --tone: var(--fg-muted); }
  .rail-icon {
    flex-shrink: 0;
    display: inline-flex; align-items: center; justify-content: center;
    width: 20px;
    color: var(--tone);
    opacity: 0.75;
    transition: transform 180ms cubic-bezier(0.34, 1.56, 0.64, 1), opacity 140ms ease;
  }
  .rail-btn:hover {
    background: color-mix(in oklch, var(--tone) 10%, var(--surface-hover));
    color: var(--fg);
  }
  .rail-btn:hover .rail-icon { transform: scale(1.18); opacity: 1; }
  .rail-btn[data-active="true"] {
    background: color-mix(in oklch, var(--tone) 14%, var(--surface));
    color: var(--fg);
  }
  /* Active icon takes the tab's tone. Big "where am I" signal — especially
     in collapsed 48px state where labels are hidden. */
  .rail-btn[data-active="true"] .rail-icon {
    opacity: 1;
    color: var(--tone);
    filter: drop-shadow(0 0 6px color-mix(in oklch, var(--tone) 35%, transparent));
  }
  .rail-btn:active { transform: translateY(0.5px); }
  .label { flex: 1; opacity: 1; transition: opacity 140ms ease; }
  /* Keyboard shortcut hint — slim digit. Tooltip on the button shows the
     full "Ctrl+N" form so this stays as a quiet visual numeral, not a
     bulky chip. Only visible when rail is expanded. */
  .kbd-hint {
    flex-shrink: 0;
    min-width: 14px;
    text-align: center;
    font-size: 10px;
    color: var(--fg-faint);
    opacity: 0.55;
    letter-spacing: 0.02em;
  }
  .rail-btn[data-active="true"] .kbd-hint { opacity: 0.9; color: var(--fg-muted); }
  .beta-pill {
    flex-shrink: 0;
    padding: 1px 5px;
    font-size: 9px;
    font-weight: 700;
    background: var(--warn-soft);
    color: var(--warn);
    border-radius: 4px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .count-pip {
    flex-shrink: 0;
    min-width: 18px; height: 16px;
    padding: 0 5px;
    font-size: 10px; font-weight: 600;
    background: var(--surface-hover);
    color: var(--fg-muted);
    border-radius: 8px;
    display: inline-flex; align-items: center; justify-content: center;
    transition: opacity 140ms ease;
  }
  .count-pip.danger {
    background: color-mix(in oklch, var(--danger) 22%, transparent);
    color: var(--danger);
  }

  .bottom {
    margin-top: auto;
    display: flex; flex-direction: column;
  }
  .divider {
    height: 1px;
    margin: 8px 4px;
    background: var(--border);
    opacity: 0.6;
  }
  .update-pill {
    margin-bottom: 8px;
    display: flex; align-items: center; gap: 8px;
    padding: 8px 10px;
    background: color-mix(in oklch, var(--accent) 10%, transparent);
    border: 1px solid color-mix(in oklch, var(--accent) 35%, transparent);
    color: var(--fg);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font: inherit; font-size: var(--fs-xs);
    text-align: left;
    overflow: hidden;
    transition: background 120ms, border-color 120ms;
  }
  .update-pill:hover {
    background: color-mix(in oklch, var(--accent) 18%, transparent);
    border-color: color-mix(in oklch, var(--accent) 55%, transparent);
  }
  .up-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--accent) 30%, transparent);
    flex-shrink: 0;
    animation: up-pulse 2s ease-in-out infinite;
  }
  @keyframes up-pulse {
    0%, 100% { box-shadow: 0 0 0 3px color-mix(in oklch, var(--accent) 30%, transparent); }
    50%      { box-shadow: 0 0 0 5px color-mix(in oklch, var(--accent) 14%, transparent); }
  }
  .up-text { display: flex; flex-direction: column; min-width: 0; line-height: 1.2; }
  .up-l { color: var(--fg); }
  .up-v { color: var(--fg-muted); font-size: var(--fs-xs); }

  /* Collapsed state: hide labels, kbd hints, button text. */
  @container (max-width: 130px) {
    .label, .up-text, .kbd-hint, .brand-word, .beta-pill { display: none; }
    .rail-brand { justify-content: center; padding: 0; gap: 0; }
    .rail-btn { justify-content: center; padding: 0; gap: 0; }
    .update-pill { justify-content: center; padding: 8px; }
    .count-pip {
      position: absolute; top: 2px; right: 2px;
      min-width: 14px; height: 14px;
      padding: 0 4px;
      font-size: 9px;
      pointer-events: none;
    }
  }
</style>
