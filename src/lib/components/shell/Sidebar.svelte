<script lang="ts">
  import { PanelLeftClose, Plus, Search } from "@lucide/svelte";
  import { workspace, type WorkspaceId } from "$lib/state/workspace.svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { shell } from "$lib/state/shell.svelte";
  import { commandPalette } from "$lib/state/command-palette.svelte";
  import { WORKSPACES } from "../workspaces";
  import RiftLogo from "./RiftLogo.svelte";
  import ConversationList from "./ConversationList.svelte";
  import ProjectSwitcher from "./ProjectSwitcher.svelte";
  import NotificationCenter from "./NotificationCenter.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { goHome } from "$lib/state/nav";
  import { rootKey } from "$lib/utils/path";

  // Footer navigation stays compact without becoming anonymous: the five daily
  // destinations share one labelled dock. Model + connection state live in the
  // composer/status bar, so the sidebar does not duplicate them.
  const footNav = $derived(
    workspace.order.filter(
      // Diagnostics is a support surface, not a daily destination. It remains
      // one keystroke away through the command palette and Settings → System,
      // while this dock stays legible as Workspace · Chat · AI Health.
      (id) => id !== "settings" && id !== "projects" && id !== "diagnostics" && !WORKSPACES[id].disabled,
    ),
  );

  function isNavActive(id: WorkspaceId): boolean {
    return workspace.activeId === id;
  }

  const NAV_LABEL: Record<WorkspaceId, string> = {
    home: "Home", chat: "Chat", projects: "Projects", settings: "Settings", "ai-health": "Health",
    diagnostics: "Support",
  };

  function goto(id: WorkspaceId) {
    workspace.setActive(id);
  }

  // Publish the rail's real layout footprint as --rail-w (registered <length>
  // in app.css so it tweens on collapse). Consumers that center on the MAIN
  // island, not the window — e.g. ToastHost — read it. Peek doesn't count:
  // the floating island overlays content without moving the layout.
  $effect(() => {
    document.documentElement.style.setProperty("--rail-w", `${shell.collapsed ? 0 : shell.width}px`);
  });

  // ── scope segment data ────────────────────────────────────────────────
  // Live count of chats in the current scope — mirrors ConversationList's own
  // filter so the number the segment shows matches the list below it.
  const scopeCount = $derived(
    shell.allProjects
      ? assistant.conversations.length
      : assistant.conversations.filter(
          (c) => !assistant.activeRoot || rootKey(c.workspaceRoot) === rootKey(assistant.activeRoot),
        ).length,
  );

  // Collapsed → the island only ever appears as the hover-peek, which renders
  // a MINI flavor: fixed compact width, content-sized height, no scope row —
  // a quick-switcher flyout, not the full furniture slab over the content.
  const mini = $derived(shell.collapsed);
  const MINI_W = 240;

  // ── rail resize ──────────────────────────────────────────────────────
  function startResize(e: PointerEvent) {
    e.preventDefault();
    shell.resizing = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function moveResize(e: PointerEvent) {
    if (!shell.resizing) return;
    // The island's right edge sits 12px inside the rail's right edge, so the
    // rail width that puts the island edge under the cursor is clientX + 12.
    shell.setWidth(e.clientX + 12);
  }
  function endResize(e: PointerEvent) {
    if (!shell.resizing) return;
    shell.resizing = false;
    shell.commitWidth();
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }
  function onResizeKey(e: KeyboardEvent) {
    const step = e.shiftKey ? 24 : 8;
    let next: number | null = null;
    if (e.key === "ArrowLeft") next = shell.width - step;
    else if (e.key === "ArrowRight") next = shell.width + step;
    else if (e.key === "Home") next = shell.minWidth;
    else if (e.key === "End") next = shell.maxWidth;
    if (next == null) return;
    e.preventDefault();
    shell.setWidth(next);
    shell.commitWidth();
  }
</script>

<!-- Floating island: the rail is only a width spacer in the flex row; the
     island card is absolutely positioned inside it, inset from the window
     edges. Pinned → rail holds layout width and the island reads as furniture
     (lift + hairline). Collapsed → rail width 0, island slides out; hovering
     the topbar trigger peeks it back OVER the content as a true floating
     layer (shadow), and a click pins it. -->
<div
  class="side-rail"
  role="presentation"
  class:collapsed={shell.collapsed}
  class:peeking={shell.collapsed && shell.peek}
  class:resizing={shell.resizing}
  style="width:{shell.collapsed ? 0 : shell.width}px"
  onmouseenter={() => { if (shell.collapsed && shell.peek) shell.cancelPeekClose(); }}
  onmouseleave={() => { if (shell.collapsed && shell.peek) shell.schedulePeekClose(); }}
>
  <aside
    class="sidebar"
    class:mini
    inert={shell.collapsed && !shell.peek}
    style="width:{mini ? MINI_W : shell.width - 24}px"
  >
    <!-- Head hides in the mini peek — the topbar cluster right above it already
         carries the brand context + pin trigger; repeating them wastes rows. -->
    {#if !mini}
      <div class="side-head" data-tauri-drag-region>
        <span class="brand">
          <RiftLogo size={22} class="brand-mk" />
          <span class="brand-name">Rift</span>
        </span>
        <button
          class="side-collapse"
          type="button"
          onclick={() => shell.toggleCollapsed()}
          use:tooltip={"Collapse sidebar"}
          aria-label="Collapse sidebar"
        >
          <PanelLeftClose size={16} />
        </button>
      </div>
    {/if}

    <ProjectSwitcher />

    <div class="action-row">
      <button class="new-chat" type="button" onclick={() => goHome()} use:tooltip={"New chat · Ctrl+N"}>
        <span class="nc-ic"><Plus size={16} strokeWidth={2.4} /></span>
        <span class="nc-lbl">New chat</span>
      </button>
      <button class="ic-btn" type="button" onclick={() => commandPalette.show()} use:tooltip={"Search chats · Ctrl+K"} aria-label="Search chats">
        <Search size={16} />
      </button>
    </div>

    <!-- ProjectSwitcher is the single scope owner. This header names the list
         without repeating a second This project / All control. -->
    {#if !mini}
      <div class="history-head">
        <span class="history-title">Conversations</span>
        <span class="count-note" aria-label={`${scopeCount} ${scopeCount === 1 ? "conversation" : "conversations"}`}>{scopeCount}</span>
      </div>
    {/if}

    <div class="side-sec">
      <ConversationList {mini} />
    </div>

    <nav class="foot-nav" aria-label="Destinations">
      {#each footNav as id (id)}
        {@const def = WORKSPACES[id]}
        <button
          class="fnav-item"
          class:on={isNavActive(id)}
          type="button"
          onclick={() => goto(id)}
          use:tooltip={def.title}
          aria-label={def.title}
          aria-current={isNavActive(id) ? "page" : undefined}
        >
          <def.icon class="snav-ic" size={16} />
          <span>{NAV_LABEL[id]}</span>
        </button>
      {/each}
      <div class="fnav-alert">
        <NotificationCenter />
        <span>Alerts</span>
      </div>
      <button
        class="fnav-item"
        class:on={isNavActive("settings")}
        type="button"
        onclick={() => goto("settings")}
        use:tooltip={"Settings"}
        aria-label="Settings"
        aria-current={isNavActive("settings") ? "page" : undefined}
      >
        <WORKSPACES.settings.icon class="snav-ic" size={16} />
        <span>Settings</span>
      </button>
    </nav>
  </aside>

  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="side-resize"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize sidebar"
    aria-valuemin={shell.minWidth}
    aria-valuemax={shell.maxWidth}
    aria-valuenow={shell.width}
    tabindex="0"
    onpointerdown={startResize}
    onpointermove={moveResize}
    onpointerup={endResize}
    onpointercancel={endResize}
    onkeydown={onResizeKey}
  ></div>
</div>

<style>
  /* Rail = layout spacer + positioning context. overflow stays visible so the
     absolutely-positioned island can float over the content while peeking. */
  .side-rail { position: relative; z-index: 30; flex: none; min-width: 0;
    /* Width is a layout prop — every frame reflows + rewraps the chat stream,
       so keep the animation SHORT (0.36s of full-app reflow read as lag). */
    transition: width var(--dur-base) var(--ease-soft); }
  .side-rail.resizing, .side-rail.resizing .sidebar { transition: none; }
  .side-rail.collapsed .sidebar { transform: translateX(-16px); opacity: 0; pointer-events: none; }
  /* Peek: same island, now a genuinely floating layer → it earns a shadow.
     Starts BELOW the topbar so the trigger cluster stays visible and hoverable
     (island covering the trigger would flicker the hover state). */
  .side-rail.peeking .sidebar { transform: none; opacity: 1; pointer-events: auto;
    top: 54px;
    box-shadow:
      var(--shadow-float),
      inset 0 1px 0 oklch(0.92 calc(var(--accent-c) * 0.25) var(--accent-h) / 0.07),
      inset 0 0 0 1px oklch(0 0 0 / 0.22); }
  .side-rail.collapsed .side-resize { display: none; }

  /* Launch handoff (data-intro on .app, intro.svelte.ts): the island slides in
     from the rail edge while the splash veil lifts — same vocabulary as the
     collapse state. Skipped when the rail boots collapsed. */
  :global(.app[data-intro="veil"]) .side-rail:not(.collapsed) .sidebar { transform: translateX(-16px); opacity: 0; }
  :global(.app[data-intro="handoff"]) .side-rail:not(.collapsed) .sidebar {
    transform: none;
    opacity: 1;
    transition: transform var(--dur-rise) var(--ease-page), opacity var(--dur-rise) var(--ease-page);
  }

  /* Handle rides the island's right edge (8px inside the rail). */
  .side-resize { position: absolute; top: 48px; right: 4px; width: 8px; height: calc(100% - 64px); z-index: 6; cursor: col-resize; -webkit-app-region: no-drag; }
  .side-resize::after { content: ""; position: absolute; top: 0; right: 3px; width: 2px; height: 100%; border-radius: 2px; background: transparent; transition: background var(--dur-fast); }
  .side-resize:hover::after { background: color-mix(in oklab, var(--fg) 22%, transparent); }
  .side-resize:focus-visible::after { background: var(--accent); box-shadow: 0 0 0 2px var(--accent-soft); }
  .side-rail.resizing .side-resize::after { background: color-mix(in oklab, var(--accent) 55%, transparent); }

  /* The island card — inset from the window edges, rounded, hairline-bordered.
     Pinned it reads as furniture (lift, no shadow); .peeking above adds the
     float shadow when it truly hovers over content. */
  .sidebar { position: absolute; top: 12px; bottom: 12px; left: 12px;
    display: flex; flex-direction: column; gap: 4px; min-height: 0;
    padding: 0 10px 10px;
    border-radius: var(--island-radius);
    border: 1px solid var(--island-border);
    /* Same onyx slab as AppShell .main — fill, grounded foot, machined bevel,
       grain (::after). The two docked islands are panels cut from one slab;
       keep every layer in lockstep. */
    background:
      linear-gradient(0deg, oklch(0 0 0 / 0.16) 0%, transparent 12%),
      linear-gradient(180deg, color-mix(in oklab, var(--fg) 6.5%, var(--bg)), color-mix(in oklab, var(--fg) 3.5%, var(--bg)) 280px);
    box-shadow:
      inset 0 1px 0 oklch(0.92 calc(var(--accent-c) * 0.25) var(--accent-h) / 0.07),
      inset 0 0 0 1px oklch(0 0 0 / 0.22),
      0 1px 2px oklch(0 0 0 / 0.35),
      0 0 14px oklch(0 0 0 / 0.4);
    overflow: hidden;
    box-sizing: border-box;
    isolation: isolate;
    transition: transform var(--dur-base) var(--ease-soft), top var(--dur-base) var(--ease-soft), opacity var(--dur-base) var(--ease-page), box-shadow var(--dur-base) var(--ease-page); }
  .sidebar::after {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='160' height='160'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='160' height='160' filter='url(%23n)'/%3E%3C/svg%3E");
    background-size: 160px 160px;
    opacity: 0.03;
  }
  /* Mini peek flavor — the flyout hugs its content instead of spanning the
     window: height comes from the capped list, bottom edge floats free.
     max-height pairs with the peek's top:54 (+8px bottom breathing room). */
  .sidebar.mini { bottom: auto; max-height: calc(100% - 62px); padding-top: 10px; }
  .sidebar button { -webkit-app-region: no-drag; }

  .side-head { display: flex; align-items: center; justify-content: space-between; height: 40px; margin-bottom: 3px; padding: 0 4px 0 6px; flex: none; -webkit-app-region: drag; }
  .brand { display: inline-flex; align-items: center; gap: 9px; }
  .brand :global(.brand-mk) { border-radius: 7px; display: block; }
  .brand-name { font-size: 14px; font-weight: 650; letter-spacing: -0.012em; color: var(--fg); }
  .side-collapse { width: 28px; height: 28px; display: grid; place-items: center; border-radius: 7px; flex: none;
    color: var(--fg-faint); transition: background var(--dur-fast), color var(--dur-fast); }
  .side-collapse:hover { background: var(--surface-hover); color: var(--fg-2); }

  /* Shared icon response stays quiet; navigation labels carry recognition. */
  :global(.snav-ic) { transition: transform var(--dur-fast) var(--ease-page); transform-origin: 50% 50%; }

  /* action row — New chat (primary, grows) + a search icon-button share one row
     to save vertical space (opens the command palette scoped to chats). */
  .action-row { display: flex; align-items: center; gap: 6px; flex: none; margin: 2px 0 5px; }
  .action-row .new-chat { flex: 1; margin: 0; }
  .ic-btn { width: 36px; height: 36px; flex: none; display: grid; place-items: center; border-radius: 9px;
    color: var(--fg-muted); border: 1px solid var(--border); background: var(--bg-inset);
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast); }
  .ic-btn:hover { background: var(--surface-hover); color: var(--fg); border-color: var(--border-strong); }

  .history-head { display: flex; align-items: center; gap: 8px; height: 28px; padding: 3px 5px 0 7px; flex: none;
    border-bottom: 1px solid color-mix(in oklab, var(--border) 48%, transparent); }
  .history-title { flex: 1; font-size: 10px; font-weight: 700; letter-spacing: 0.075em; text-transform: uppercase; color: var(--fg-subtle); }
  .count-note { min-width: 22px; height: 18px; padding: 0 6px; display: inline-grid; place-items: center; border-radius: 999px;
    font-size: 9.5px; font-weight: 650; color: var(--fg-faint); background: color-mix(in oklab, var(--fg) 6%, transparent); font-variant-numeric: tabular-nums; }

  /* One labelled destination dock. The former split icon groups looked like
     unrelated utilities; shared columns make the information architecture
     obvious without consuming conversation height. */
  .foot-nav { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); align-items: stretch; gap: 2px; flex: none;
    padding-top: 7px; margin-top: 3px; border-top: 1px solid color-mix(in oklab, var(--border) 56%, transparent); }
  .fnav-item, .fnav-alert { position: relative; min-width: 0; height: 42px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px;
    border-radius: 8px; color: var(--fg-faint); font-size: 8px; font-weight: 600; letter-spacing: -0.01em;
    transition: background var(--dur-fast), color var(--dur-fast), transform var(--dur-fast); }
  .fnav-item > span, .fnav-alert > span { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fnav-item:active { transform: scale(0.96); }
  .fnav-item:hover { background: var(--surface-hover); color: var(--fg-2); }
  .fnav-item:hover :global(.snav-ic) { transform: translateY(-1px); }
  .fnav-item.on { color: var(--accent); background: color-mix(in oklab, var(--accent) 9%, transparent); }
  .fnav-alert :global(.nc-bell) { width: 28px; height: 24px; border-radius: 6px; }
  .fnav-alert:focus-within, .fnav-alert:hover { color: var(--fg-2); background: var(--surface-hover); }

  /* primary action — New chat. Reads as primary via an accent-tinted surface +
     accent icon, not a saturated slab, so it stays in the rail's soft language. */
  .new-chat { display: flex; align-items: center; gap: 9px; height: 36px; margin: 0; padding: 0 10px; flex: none;
    border-radius: 9px; font-size: 12.5px; font-weight: 600; color: var(--fg);
    border: 1px solid color-mix(in oklab, var(--accent) 28%, var(--border));
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 12%, transparent), color-mix(in oklab, var(--accent) 5%, transparent));
    transition: background var(--dur-fast), border-color var(--dur-fast), transform var(--dur-fast); }
  .new-chat:hover { background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 18%, transparent), color-mix(in oklab, var(--accent) 9%, transparent));
    border-color: color-mix(in oklab, var(--accent) 44%, var(--border)); }
  .new-chat:active { transform: translateY(1px); }
  .new-chat .nc-ic { display: inline-flex; flex: none; }
  .new-chat .nc-lbl { flex: 1; text-align: left; }
  .new-chat :global(svg) { flex: none; color: var(--accent); }

  /* conversation-list section wrapper */
  .side-sec { display: flex; flex-direction: column; flex: 1; min-height: 0; }

  @media (prefers-reduced-motion: reduce) {
    :global(.snav-ic), .fnav-item { transition: none; }
  }
</style>
