<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { TerminalSquare, ChevronDown, ChevronUp, Plus, X, Trash2, Search, Eraser } from "lucide-svelte";
  import Terminal, { type SearchApi } from "./Terminal.svelte";
  import TerminalFindBar from "./TerminalFindBar.svelte";
  import { terminal, TERM_HEIGHT_MIN } from "../../state/terminal.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";
  import { rightPane } from "../../state/right-pane.svelte";

  // v0.4.1 hosts the terminal inside RightPane (one full-page surface). Under
  // v0.2 the overlay chrome still drives visibility via `terminal.open`.
  const isV03 = $derived(uiPrefs.useV03Shell);
  const isVisible = $derived(isV03 ? rightPane.activeId === "terminal" : terminal.open);

  type ShellInfo = {
    id: string;
    label: string;
    program: string;
    args: string[];
    available: boolean;
  };

  // Built-in presets — one-click launches that bake in shell + command.
  // The two big agentic coding CLIs as of 2026; users supply their own auth.
  type Preset = {
    id: string;
    label: string;
    subtitle: string;
    shellId: string;     // resolved against shells at runtime
    autoLaunch: string;
  };
  const PRESETS: Preset[] = [
    { id: "claude-code", label: "Claude Code", subtitle: "Anthropic · claude",
      shellId: "git-bash", autoLaunch: "claude" },
    { id: "codex", label: "Codex CLI", subtitle: "OpenAI · codex",
      shellId: "git-bash", autoLaunch: "codex" },
  ];

  let shells = $state<ShellInfo[]>([]);
  let pickerOpen = $state(false);
  let pickerEl = $state<HTMLDivElement | undefined>();
  let panelEl = $state<HTMLElement | undefined>();
  let resizing = $state(false);
  let findOpen = $state(false);
  let editingTabId = $state<string | null>(null);
  let editingValue = $state("");
  let dropActive = $state(false);
  let unlistenDrop: UnlistenFn | null = null;
  // Per-tab search APIs — Terminal calls onSearchReady when its addon mounts.
  // Map keyed by tab id so FindBar can route to the active tab.
  const searchApis = new Map<string, SearchApi>();
  let activeApi = $state<SearchApi | null>(null);

  function refreshActiveApi() {
    const id = terminal.activeTabId;
    activeApi = id ? (searchApis.get(id) ?? null) : null;
  }
  $effect(() => {
    // Re-resolve when the active tab changes.
    const _id = terminal.activeTabId;
    void _id;
    refreshActiveApi();
  });

  function openFind() {
    if (!isVisible || terminal.tabs.length === 0) return;
    findOpen = true;
  }
  function closeFind() { findOpen = false; }

  function startRename(id: string, currentLabel: string) {
    editingTabId = id;
    editingValue = currentLabel;
  }
  function commitRename() {
    if (!editingTabId) return;
    terminal.renameTab(editingTabId, editingValue);
    editingTabId = null;
    editingValue = "";
  }
  function cancelRename() {
    editingTabId = null;
    editingValue = "";
  }
  function onRenameKey(e: KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); commitRename(); }
    else if (e.key === "Escape") { e.preventDefault(); cancelRename(); }
  }

  function clearActive() {
    const tab = terminal.tabs.find((t) => t.id === terminal.activeTabId);
    if (!tab?.sessionId) return;
    // \x0c (form feed) is the same byte Ctrl+L emits — every shell treats
    // this as "clear screen". Works for bash/zsh/PS/cmd uniformly.
    void invoke("term_write", { id: tab.sessionId, data: "\x0c" })
      .catch((e) => console.warn("clear failed", e));
  }

  function isPositionOverPanel(x: number, y: number): boolean {
    if (!panelEl) return false;
    const r = panelEl.getBoundingClientRect();
    return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
  }
  function quoteIfNeeded(p: string): string {
    return /[\s'"]/.test(p) ? `"${p.replace(/"/g, '\\"')}"` : p;
  }
  async function pasteDroppedPaths(paths: string[]) {
    const tab = terminal.tabs.find((t) => t.id === terminal.activeTabId);
    if (!tab?.sessionId) return;
    const joined = paths.map(quoteIfNeeded).join(" ");
    try {
      await invoke("term_write", { id: tab.sessionId, data: joined });
    } catch (e) { console.warn("drop write failed", e); }
  }

  // Picker is sorted: available shells first (default-marked rises to top),
  // then anything unavailable at the bottom, dimmed.
  const sortedShells = $derived.by(() => {
    const out = [...shells];
    out.sort((a, b) => {
      if (a.available !== b.available) return a.available ? -1 : 1;
      if (a.id === terminal.defaultShellId) return -1;
      if (b.id === terminal.defaultShellId) return 1;
      return 0;
    });
    return out;
  });

  // Collapsed strip subtitle: show the active tab's shell, OR the default
  // shell if nothing's running, OR fall back to "Terminal".
  const collapsedLabel = $derived.by(() => {
    const active = terminal.tabs.find((t) => t.id === terminal.activeTabId);
    if (active?.customLabel) return active.customLabel;
    if (active?.shellLabel && active.shellLabel !== "Terminal") return active.shellLabel;
    const def = shells.find((s) => s.id === terminal.defaultShellId);
    return def?.label ?? "Terminal";
  });

  function onGlobalKey(e: KeyboardEvent) {
    const meta = e.ctrlKey || e.metaKey;
    if (!meta || e.altKey) return;
    const k = e.key.toLowerCase();
    const ae = document.activeElement;
    const inOtherInput =
      ae && (ae.tagName === "INPUT" || ae.tagName === "TEXTAREA") &&
      !(ae as HTMLElement).closest(".term-panel");

    if (!e.shiftKey && k === "f" && isVisible) {
      if (inOtherInput) return;
      e.preventDefault();
      openFind();
      return;
    }
    if (e.shiftKey && isVisible) {
      // Ctrl+Shift+[ / ] cycle tabs; Ctrl+Shift+T = new tab. We check `e.code`
      // for the brackets b/c `e.key` reports "{" / "}" under Shift.
      if (e.code === "BracketLeft" || e.code === "BracketRight") {
        if (inOtherInput) return;
        e.preventDefault();
        terminal.cycleTab(e.code === "BracketRight" ? 1 : -1);
        return;
      }
      if (k === "t") {
        if (inOtherInput) return;
        e.preventDefault();
        terminal.addTab(terminal.defaultShellId);
        return;
      }
    }
  }
  $effect(() => {
    window.addEventListener("keydown", onGlobalKey);
    return () => window.removeEventListener("keydown", onGlobalKey);
  });

  onMount(async () => {
    terminal.init();
    try { shells = await invoke<ShellInfo[]>("term_list_shells"); }
    catch (e) { console.warn("term_list_shells failed", e); }

    // Tauri 2 file-drop. Routes to the active tab only — drop on the terminal
    // panel area, the absolute paths get typed into the active shell.
    try {
      unlistenDrop = await getCurrentWebview().onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") {
          const pos = (p as { position?: { x: number; y: number } }).position;
          dropActive = !!pos && isPositionOverPanel(pos.x, pos.y) && isVisible;
        } else if (p.type === "drop") {
          const pos = (p as { position?: { x: number; y: number } }).position;
          const paths = (p as { paths?: string[] }).paths ?? [];
          if (pos && isPositionOverPanel(pos.x, pos.y) && isVisible && paths.length) {
            void pasteDroppedPaths(paths);
          }
          dropActive = false;
        } else {
          dropActive = false;
        }
      });
    } catch (e) { console.warn("onDragDropEvent failed", e); }
    if (!terminal.defaultShellId) {
      const first = shells.find((s) => s.available);
      if (first) terminal.setDefaultShell(first.id);
    }
    // Rehydrate saved tab structure from last session (shells don't spawn
    // until user activates a tab — lazy by design).
    terminal.consumePendingRestore();
    // Open-but-no-tabs fallback: panel was open last time but no saved tabs
    // (first run, or storage cleared) — spin up one with the default shell.
    // Under v0.3, mount itself means the dock panel is open, so always seed.
    if (isVisible && terminal.tabs.length === 0) {
      terminal.addTab(terminal.defaultShellId);
    }
  });

  function onPickerKey(e: KeyboardEvent) {
    if (e.key === "Escape") { pickerOpen = false; }
  }
  function onDocClick(e: MouseEvent) {
    if (!pickerOpen || !pickerEl) return;
    if (!pickerEl.contains(e.target as Node)) pickerOpen = false;
  }
  $effect(() => {
    if (!pickerOpen) return;
    document.addEventListener("click", onDocClick, true);
    document.addEventListener("keydown", onPickerKey);
    return () => {
      document.removeEventListener("click", onDocClick, true);
      document.removeEventListener("keydown", onPickerKey);
    };
  });

  function addTab(shellId: string | null) {
    pickerOpen = false;
    terminal.addTab(shellId ?? terminal.defaultShellId);
  }

  function addPresetTab(p: Preset) {
    pickerOpen = false;
    terminal.addTab(p.shellId, p.label, p.autoLaunch);
  }

  function onDividerPointerDown(e: PointerEvent) {
    e.preventDefault();
    resizing = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function onDividerPointerMove(e: PointerEvent) {
    if (!resizing) return;
    const wrap = panelEl?.parentElement;
    if (!wrap) return;
    const rect = wrap.getBoundingClientRect();
    const distFromBottom = rect.bottom - e.clientY;
    const maxH = Math.max(TERM_HEIGHT_MIN, rect.height - 200);
    let h = distFromBottom;
    if (h < TERM_HEIGHT_MIN) h = TERM_HEIGHT_MIN;
    if (h > maxH) h = maxH;
    terminal.height = h;
  }
  function onDividerPointerUp(e: PointerEvent) {
    if (!resizing) return;
    resizing = false;
    try { (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId); } catch { /* noop */ }
    terminal.setHeight(terminal.height);
  }
  function resetHeight() { terminal.resetHeight(); }

  onDestroy(() => {
    if (unlistenDrop) { unlistenDrop(); unlistenDrop = null; }
  });
</script>

{#snippet termInner(showHideButton: boolean)}
  <header class="term-head">
    <div class="term-tabs" role="tablist">
      {#each terminal.tabs as t (t.id)}
        {@const displayLabel = t.customLabel || t.shellLabel}
        <button
          type="button"
          class="term-tab"
          role="tab"
          data-active={t.id === terminal.activeTabId}
          data-status={t.status}
          onclick={() => terminal.setActive(t.id)}
          ondblclick={(e) => { e.preventDefault(); startRename(t.id, displayLabel); }}
          title="{displayLabel} — double-click to rename"
        >
          <TerminalSquare size={11}/>
          {#if editingTabId === t.id}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="term-tab-input mono"
              type="text"
              bind:value={editingValue}
              onkeydown={onRenameKey}
              onblur={commitRename}
              onclick={(e) => e.stopPropagation()}
              ondblclick={(e) => e.stopPropagation()}
              autofocus
              placeholder={t.shellLabel}
            />
          {:else}
            <span class="term-tab-label">{displayLabel}</span>
          {/if}
          {#if t.status === "exited"}
            <span class="term-tab-dim mono">· exited</span>
          {/if}
          {#if terminal.tabs.length > 1}
            <span
              class="term-tab-x"
              role="button"
              tabindex="-1"
              aria-label="Close tab"
              title="Close tab"
              onclick={(e) => { e.stopPropagation(); terminal.closeTab(t.id); }}
              onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.stopPropagation(); terminal.closeTab(t.id); } }}
            ><X size={9}/></span>
          {/if}
        </button>
      {/each}

      <div class="term-tab-add" bind:this={pickerEl} data-open={pickerOpen}>
        <button
          type="button"
          class="term-tab-new"
          onclick={() => addTab(null)}
          title="New {shells.find((s) => s.id === terminal.defaultShellId)?.label ?? 'shell'} tab"
          aria-label="New tab"
        ><Plus size={11}/></button>
        <span class="term-tab-sep" aria-hidden="true"></span>
        <button
          type="button"
          class="term-tab-pick"
          onclick={() => (pickerOpen = !pickerOpen)}
          title="Pick a shell"
          aria-label="Pick shell"
          aria-expanded={pickerOpen}
        ><ChevronDown size={10}/></button>

        {#if pickerOpen}
          <div class="term-picker" role="menu">
            <div class="term-picker-section">Presets</div>
            {#each PRESETS as p (p.id)}
              {@const shellAvail = shells.find((s) => s.id === p.shellId)?.available ?? false}
              <button
                type="button"
                class="term-picker-item"
                role="menuitem"
                disabled={!shellAvail}
                onclick={() => addPresetTab(p)}
                title={shellAvail ? `New tab → ${p.autoLaunch}` : "Underlying shell not installed"}
              >
                <span class="term-picker-left">
                  <span class="term-picker-dot-spacer" aria-hidden="true"></span>
                  <span class="term-picker-stack">
                    <span class="term-picker-label">{p.label}</span>
                    <span class="term-picker-sub mono">{p.subtitle}</span>
                  </span>
                </span>
              </button>
            {/each}

            <div class="term-picker-section">Shells</div>
            {#each sortedShells as s (s.id)}
              <button
                type="button"
                class="term-picker-item"
                role="menuitem"
                disabled={!s.available}
                data-default={s.id === terminal.defaultShellId}
                onclick={() => addTab(s.id)}
                title={s.available ? s.program : "Not installed"}
              >
                <span class="term-picker-left">
                  {#if s.id === terminal.defaultShellId}
                    <span class="term-picker-dot" aria-hidden="true"></span>
                  {:else}
                    <span class="term-picker-dot-spacer" aria-hidden="true"></span>
                  {/if}
                  <span class="term-picker-label">{s.label}</span>
                </span>
                {#if !s.available}
                  <span class="term-picker-dim mono">missing</span>
                {:else if s.id === terminal.defaultShellId}
                  <span class="term-picker-dim mono">default</span>
                {/if}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <div class="term-head-r">
      <button
        type="button"
        class="term-close"
        onclick={clearActive}
        title="Clear buffer (Ctrl+L)"
        aria-label="Clear terminal buffer"
      ><Eraser size={12}/></button>
      <button
        type="button"
        class="term-close"
        onclick={openFind}
        title="Find in terminal (Ctrl+F)"
        aria-label="Find in terminal"
      ><Search size={12}/></button>
      {#if terminal.tabs.length > 1}
        <button
          type="button"
          class="term-close-all"
          onclick={() => terminal.closeAllTabs()}
          title="Close all tabs"
          aria-label="Close all tabs"
        ><Trash2 size={12}/></button>
      {/if}
      {#if showHideButton}
        <button
          type="button"
          class="term-close"
          onclick={() => terminal.setOpen(false)}
          title="Hide terminal (Ctrl+`)"
          aria-label="Hide terminal"
        ><ChevronDown size={14}/></button>
      {/if}
    </div>
  </header>

  <div class="term-body">
    {#if findOpen}
      <TerminalFindBar api={activeApi} onClose={closeFind} />
    {/if}
    {#each terminal.tabs as t (t.id)}
      <div
        class="term-mount"
        style:display={t.id === terminal.activeTabId ? "flex" : "none"}
      >
        <Terminal
          visible={t.id === terminal.activeTabId && isVisible}
          shellId={t.shellId}
          fontSize={terminal.fontSize}
          autoLaunch={t.autoLaunch || terminal.autoLaunchCommand}
          onSessionStart={(info) => terminal.patchTab(t.id, {
            sessionId: info.id,
            // Preserve preset-supplied labels ("Claude Code", "Codex CLI") —
            // only overwrite if the tab is still on the default "Terminal".
            shellLabel: t.shellLabel === "Terminal" ? info.shell_label : t.shellLabel,
            status: "running",
          })}
          onStatusChange={(s) => terminal.patchTab(t.id, { status: s === "idle" ? t.status : s })}
          onSearchReady={(api) => { searchApis.set(t.id, api); refreshActiveApi(); }}
          onSearchTeardown={() => { searchApis.delete(t.id); refreshActiveApi(); }}
        />
      </div>
    {/each}
  </div>
{/snippet}

{#if isV03}
  <!-- v0.3 dock mode: PanelShell wraps this and owns visibility/height.
       No drag divider, no inline height, no collapsed strip, no hide-self
       button (PanelShell exposes Close via its panel menu). -->
  <section
    class="term-panel term-panel-v03"
    bind:this={panelEl}
    data-drop-active={dropActive}
  >
    {@render termInner(false)}
  </section>
{:else if terminal.open}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="term-divider"
    role="separator"
    aria-orientation="horizontal"
    aria-label="Resize terminal (drag, or double-click to reset)"
    tabindex="0"
    title="Drag to resize · double-click to reset"
    data-resizing={resizing}
    onpointerdown={onDividerPointerDown}
    onpointermove={onDividerPointerMove}
    onpointerup={onDividerPointerUp}
    onpointercancel={onDividerPointerUp}
    ondblclick={resetHeight}
  ><span class="term-divider-grip" aria-hidden="true"></span></div>

  <section
    class="term-panel"
    style="height: {terminal.height}px"
    bind:this={panelEl}
    data-resizing={resizing}
    data-drop-active={dropActive}
  >
    {@render termInner(true)}
  </section>
{:else}
  <button
    type="button"
    class="term-collapsed"
    onclick={() => terminal.setOpen(true)}
    title="Open terminal (Ctrl+`)"
    aria-label="Open terminal"
  >
    <span class="term-collapsed-left">
      <TerminalSquare size={12}/>
      <span class="term-collapsed-label">Terminal</span>
      {#if collapsedLabel !== "Terminal"}
        <span class="term-collapsed-sub mono">· {collapsedLabel}</span>
      {/if}
      {#if terminal.tabs.length > 1}
        <span class="term-collapsed-count mono">{terminal.tabs.length} tabs</span>
      {/if}
    </span>
    <span class="term-collapsed-right">
      <span class="term-collapsed-kbd mono">Ctrl+`</span>
      <ChevronUp size={12}/>
    </span>
  </button>
{/if}

<style>
  .term-divider {
    flex: 0 0 6px;
    background: var(--bg-elev-3);
    cursor: row-resize;
    display: flex; align-items: center; justify-content: center;
    transition: background 100ms ease;
    touch-action: none;
  }
  .term-divider:hover,
  .term-divider[data-resizing="true"] { background: var(--accent); }
  .term-divider:focus-visible {
    outline: none;
    background: var(--accent);
    box-shadow: 0 0 0 2px var(--ring);
  }
  .term-divider-grip {
    width: 28px; height: 2px;
    background: var(--fg-faint);
    border-radius: 1px;
    opacity: 0.6;
    transition: opacity 100ms ease, background 100ms ease;
  }
  .term-divider:hover .term-divider-grip,
  .term-divider[data-resizing="true"] .term-divider-grip {
    opacity: 1; background: oklch(0.99 0 0);
  }

  .term-panel {
    flex: 0 0 auto;
    display: flex; flex-direction: column;
    background: var(--bg);
    border-top: 1px solid var(--border);
    min-height: 0;
    min-width: 0;
    position: relative;
  }
  /* v0.3 dock mode: PanelShell owns the border + sizing. The panel fills the
     dock-body slot top-to-bottom; the .wrap min-height in TerminalDockPanel
     provides the floor (xterm has no natural height). */
  .term-panel.term-panel-v03 {
    flex: 1;
    height: 100%;
    border-top: 0;
  }
  .term-panel[data-resizing="true"] :global(*) {
    user-select: none !important;
    pointer-events: none;
  }
  .term-panel[data-drop-active="true"] {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .term-panel[data-drop-active="true"]::after {
    content: "Drop to paste path";
    position: absolute;
    top: 50%; left: 50%;
    transform: translate(-50%, -50%);
    padding: 6px 12px;
    background: var(--accent);
    color: var(--accent-fg);
    font-size: var(--fs-xs);
    font-weight: 600;
    border-radius: var(--radius-sm);
    pointer-events: none;
    z-index: 50;
  }
  .term-tab-input {
    flex: 1; min-width: 60px;
    height: 18px;
    padding: 0 4px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-xs);
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-xs);
  }
  .term-tab-input:focus {
    outline: none;
    border-color: var(--border-focus);
    box-shadow: 0 0 0 2px var(--ring);
  }

  .term-head {
    flex: 0 0 30px;
    display: flex; align-items: stretch; gap: 4px;
    padding: 0 6px 0 4px;
    background: var(--bg-elev-1);
    border-bottom: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: var(--fs-xs);
  }
  .term-tabs {
    flex: 1; min-width: 0;
    display: flex; align-items: center; gap: 2px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .term-tabs::-webkit-scrollbar { display: none; }

  .term-tab {
    flex: 0 0 auto;
    display: inline-flex; align-items: center; gap: 6px;
    height: 24px; padding: 0 4px 0 8px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--fg-muted);
    font: inherit;
    font-size: var(--fs-xs);
    letter-spacing: 0.02em;
    cursor: pointer;
    max-width: 200px;
    transition: background 100ms ease, color 100ms ease, border-color 100ms ease;
  }
  .term-tab :global(svg) { color: var(--accent); opacity: 0.85; flex-shrink: 0; }
  .term-tab:hover { background: var(--surface-hover); color: var(--fg); }
  .term-tab[data-active="true"] {
    background: var(--bg);
    color: var(--fg);
    border-color: var(--border);
    font-weight: 600;
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .term-tab[data-status="error"] { color: var(--danger); }
  .term-tab[data-status="exited"] { color: var(--fg-faint); }
  .term-tab-label {
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .term-tab-dim { color: var(--fg-faint); font-size: 10px; }
  .term-tab-x {
    display: inline-flex; align-items: center; justify-content: center;
    width: 16px; height: 16px;
    border-radius: var(--radius-xs);
    color: var(--fg-faint);
    cursor: pointer;
    flex-shrink: 0;
    transition: background 100ms ease, color 100ms ease;
  }
  .term-tab-x:hover { background: var(--danger); color: oklch(0.99 0 0); }

  .term-tab-add {
    position: relative;
    display: inline-flex; align-items: stretch;
    margin-left: 4px;
    height: 22px;
    border-radius: var(--radius-xs);
    background: transparent;
    transition: background 100ms ease;
  }
  .term-tab-add:hover,
  .term-tab-add[data-open="true"] { background: var(--surface-hover); }
  .term-tab-new,
  .term-tab-pick {
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    transition: color 100ms ease;
  }
  .term-tab-new { width: 22px; border-radius: var(--radius-xs) 0 0 var(--radius-xs); }
  .term-tab-pick { width: 16px; padding: 0; border-radius: 0 var(--radius-xs) var(--radius-xs) 0; }
  .term-tab-add:hover .term-tab-new,
  .term-tab-add:hover .term-tab-pick,
  .term-tab-add[data-open="true"] .term-tab-pick { color: var(--fg); }
  .term-tab-sep {
    width: 1px;
    margin: 4px 0;
    background: var(--border);
    align-self: stretch;
    opacity: 0;
    transition: opacity 100ms ease;
  }
  .term-tab-add:hover .term-tab-sep,
  .term-tab-add[data-open="true"] .term-tab-sep { opacity: 1; }

  .term-picker {
    position: absolute; top: calc(100% + 4px); left: 0;
    z-index: 60;
    min-width: 180px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    padding: 4px;
    display: flex; flex-direction: column; gap: 1px;
  }
  .term-picker-item {
    display: flex; align-items: center; justify-content: space-between;
    gap: 12px;
    background: transparent;
    border: 0;
    padding: 6px 10px;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    text-align: left;
    border-radius: var(--radius-xs);
    cursor: pointer;
    transition: background 100ms ease, color 100ms ease;
  }
  .term-picker-item:hover:not(:disabled) {
    background: color-mix(in oklch, var(--accent) 12%, transparent);
    color: var(--fg);
  }
  .term-picker-item:disabled {
    color: var(--fg-faint); cursor: not-allowed; opacity: 0.6;
  }
  .term-picker-item[data-default="true"] { color: var(--fg); }
  .term-picker-section {
    padding: 6px 10px 4px;
    color: var(--fg-faint);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .term-picker-section:not(:first-child) {
    margin-top: 4px;
    border-top: 1px solid var(--border);
    padding-top: 8px;
  }
  .term-picker-stack {
    display: inline-flex; flex-direction: column; gap: 1px;
    line-height: 1.25;
  }
  .term-picker-sub {
    color: var(--fg-faint);
    font-size: 10px;
    letter-spacing: 0;
  }
  .term-picker-left {
    display: inline-flex; align-items: center; gap: 8px;
  }
  .term-picker-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 22%, transparent);
    flex-shrink: 0;
  }
  .term-picker-dot-spacer {
    width: 6px; height: 6px; flex-shrink: 0;
  }
  .term-picker-label { font-weight: 500; }
  .term-picker-dim { color: var(--fg-faint); font-size: 10px; }

  .term-head-r {
    display: inline-flex; align-items: center; gap: 2px;
    margin: auto 0;
  }
  .term-close,
  .term-close-all {
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    width: 22px; height: 22px;
    border-radius: var(--radius-xs);
    cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
    transition: background 100ms ease, color 100ms ease;
  }
  .term-close:hover { background: var(--surface-hover); color: var(--fg); }
  .term-close-all:hover { background: var(--danger); color: oklch(0.99 0 0); }

  .term-body {
    flex: 1; min-height: 0;
    display: flex;
    position: relative;
  }
  .term-mount {
    flex: 1; min-height: 0; min-width: 0;
    flex-direction: column;
  }

  .term-collapsed {
    flex: 0 0 24px;
    display: flex; align-items: center; justify-content: space-between;
    width: 100%;
    padding: 0 12px;
    background: var(--bg-elev-1);
    border: 0;
    border-top: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    letter-spacing: 0.04em;
    cursor: pointer;
    text-align: left;
    transition: background 100ms ease, color 100ms ease;
  }
  .term-collapsed:hover {
    background: color-mix(in oklch, var(--accent) 6%, var(--bg-elev-1));
    color: var(--fg);
  }
  .term-collapsed-left,
  .term-collapsed-right {
    display: inline-flex; align-items: center; gap: 8px;
  }
  .term-collapsed-left :global(svg) { color: var(--accent); opacity: 0.9; }
  .term-collapsed-right :global(svg) { color: var(--fg-faint); }
  .term-collapsed:hover .term-collapsed-right :global(svg) { color: var(--fg-muted); }
  .term-collapsed-label { font-weight: 500; }
  .term-collapsed-sub {
    font-size: 10px;
    color: var(--fg-faint);
    letter-spacing: 0;
  }
  .term-collapsed:hover .term-collapsed-sub { color: var(--fg-muted); }
  .term-collapsed-count {
    font-size: 10px;
    color: var(--fg-faint);
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 5px;
    letter-spacing: 0;
  }
  .term-collapsed-kbd {
    font-size: 10px;
    color: var(--fg-faint);
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 5px;
  }
</style>
