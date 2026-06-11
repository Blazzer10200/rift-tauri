<script lang="ts">
  // In-assistant web browser dock. Renders an address bar + an empty "stage"
  // placeholder; the actual page is a NATIVE child webview (Tauri multiwebview)
  // positioned by the Rust `browser` module to overlap the stage. We report the
  // stage's rect and show/hide the native webview as the dock's visibility
  // changes. See src-tauri/src/browser/mod.rs.
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onDestroy, onMount } from "svelte";
  import { Globe, RotateCw, ChevronLeft, ChevronRight, MessageSquarePlus, Check, X, Copy, ExternalLink, MoreHorizontal } from "lucide-svelte";
  import { portal } from "$lib/actions/portal";
  import { workspace } from "../../state/workspace.svelte";
  import { browserDock } from "../../state/browserDock.svelte";
  import { assistant } from "../../state/assistant.svelte";

  let address = $state("");
  let addressEl = $state<HTMLInputElement | null>(null);

  // Overflow menu (Copy URL / Open external) — keeps the bar uncluttered.
  let menuOpen = $state(false);
  let menuPos = $state({ x: 0, y: 0 });
  function openMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menuPos = { x: Math.max(4, r.right - 200), y: r.bottom + 4 };
    menuOpen = true;
  }
  let opened = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  // "Add page to chat" — in-flight + brief success/failure flash on the button.
  // Feedback lives on the button (always visible) not the stage, which the
  // native webview floats over and would hide.
  let adding = $state(false);
  let added = $state(false);
  let failed = $state(false);
  let flashTimer: ReturnType<typeof setTimeout> | null = null;

  // Fallback: if a "finished" load event never lands (rare), don't leave the
  // spinner stuck forever.
  let loadWatchdog: ReturnType<typeof setTimeout> | null = null;
  function armLoadWatchdog() {
    if (loadWatchdog) clearTimeout(loadWatchdog);
    loadWatchdog = setTimeout(() => { loading = false; }, 20000);
  }

  function flash(kind: "ok" | "fail") {
    added = kind === "ok";
    failed = kind === "fail";
    if (flashTimer) clearTimeout(flashTimer);
    flashTimer = setTimeout(() => { added = false; failed = false; }, 1500);
  }

  type PageSnapshot = { title: string; url: string; text: string; truncated: boolean; full_len: number };
  let stageEl: HTMLDivElement | null = $state(null);
  // True while the address input is focused — don't overwrite what the user is
  // typing with the synced live URL.
  let inputFocused = $state(false);

  function normalizeUrl(raw: string): string {
    const t = raw.trim();
    if (!t) return t;
    // F199: don't forward `data:` (or other non-web schemes) to the native
    // webview — the backend `parse_url` allowlist rejects them anyway.
    if (/^https?:\/\//i.test(t)) return t;
    // Omnibox: a dotted, space-free token is a host → prefix https://;
    // anything else is a web search.
    if (!/\s/.test(t) && /^[^\s]+\.[^\s]{2,}$/.test(t)) return `https://${t}`;
    return `https://duckduckgo.com/?q=${encodeURIComponent(t)}`;
  }

  function rect(): { x: number; y: number; w: number; h: number } | null {
    if (!stageEl) return null;
    const r = stageEl.getBoundingClientRect();
    // CSS px == Tauri logical px; viewport origin == window content origin.
    return { x: r.left, y: r.top, w: r.width, h: r.height };
  }

  // rAF-coalesced bounds push so divider-drag resizes don't spam the IPC.
  let rafPending = false;
  function syncBounds() {
    if (!opened || rafPending) return;
    rafPending = true;
    requestAnimationFrame(async () => {
      rafPending = false;
      const b = rect();
      if (!b) return;
      try { await invoke("browser_set_bounds", b); } catch (e) { console.warn("browser_set_bounds:", e); }
    });
  }

  async function go() {
    const url = normalizeUrl(address);
    if (!url) return;
    address = url;
    const b = rect();
    if (!b) return;
    loading = true;
    error = null;
    try {
      await invoke("browser_open", { url, ...b });
      opened = true;
      // Don't clear `loading` here — `browser_open` resolves when navigation is
      // dispatched, not when the page finishes. The `browser://load` "finished"
      // event clears it (watchdog as a fallback if it never arrives).
      armLoadWatchdog();
    } catch (e) {
      error = String(e);
      opened = false;
      loading = false;
    }
  }

  async function reload() {
    if (!opened) return;
    // True in-place reload — preserves history (no duplicate Back entry) and
    // re-runs SPA state, unlike re-navigating to the current URL.
    try { await invoke("browser_reload"); } catch (e) { error = String(e); }
  }

  async function copyUrl() {
    menuOpen = false;
    try {
      const live = (await invoke<string>("browser_current_url")) || address;
      if (live) await navigator.clipboard.writeText(live);
    } catch (e) { console.warn("copyUrl:", e); }
  }

  async function openExternal() {
    menuOpen = false;
    try {
      const live = (await invoke<string>("browser_current_url")) || normalizeUrl(address);
      if (!live || !/^https?:\/\//i.test(live)) return;
      await openUrl(live);
    } catch (e) { console.warn("openExternal:", e); }
  }

  async function goBack() {
    if (!opened) return;
    try { await invoke("browser_back"); } catch (e) { error = String(e); }
    // Nudge the address bar to catch up faster than the 1.2s poll.
    setTimeout(syncAddress, 250);
  }

  async function goForward() {
    if (!opened) return;
    try { await invoke("browser_forward"); } catch (e) { error = String(e); }
    setTimeout(syncAddress, 250);
  }

  // The unlock: pull the page's rendered (post-JS, authenticated) text and drop
  // it into the composer as a labelled context block — something WebFetch can't
  // reach. Stays visible + editable so the user trims/sends on their terms.
  async function addToChat() {
    if (!opened || adding) return;
    adding = true;
    try {
      const p = await invoke<PageSnapshot>("browser_read_page");
      const body = (p.text || "").trim();
      if (!body) { flash("fail"); return; }
      // Sanitize title/url to prevent ] or newlines from breaking the delimiter.
      const safeTitle = (p.title || "untitled").replace(/[\]\r\n]/g, " ").slice(0, 200);
      const safeUrl = (p.url || "").replace(/[\]\r\n]/g, " ");
      const head = `[Page context: ${safeTitle} — ${safeUrl}]`;
      const tail = p.truncated ? `\n[…truncated — full page is ${p.full_len.toLocaleString()} chars]` : "";
      const block = `${head}\n${body}${tail}\n[End page context]`;
      const cur = assistant.composerDraft;
      assistant.composerDraft = cur ? `${cur}\n\n${block}` : block;
      flash("ok");
    } catch (e) {
      console.warn("browser_read_page:", e);
      flash("fail");
    } finally {
      adding = false;
    }
  }

  // Keep the address bar honest: the native webview navigates on its own (link
  // clicks, redirects) without notifying us, so poll its real URL while the
  // dock is actually visible. Cheap (a `wv.url()` read) and paused otherwise.
  async function syncAddress() {
    if (!opened || inputFocused) return;
    try {
      const live = await invoke<string>("browser_current_url");
      if (live && live !== address) address = live;
    } catch { /* noop */ }
  }

  // Visibility: the native webview floats above the main webview, so it MUST be
  // hidden whenever the chat workspace isn't the active one (the dock stays
  // mounted-but-hidden behind other workspaces otherwise).
  $effect(() => {
    const onChat = workspace.activeId === "chat";
    if (!opened) return;
    if (onChat) {
      void invoke("browser_show").then(syncBounds).catch((e) => { error = String(e); });
    } else {
      void invoke("browser_hide");
    }
  });

  // Assistant-driven navigation (mcp__rift__open_browser) or a localhost link
  // click in the chat: consume the queued URL once the stage exists. go()
  // reads the stage rect, so this must wait for mount — tracking stageEl
  // covers the dock-was-closed case where openUrl() also triggered the mount.
  $effect(() => {
    const url = browserDock.pendingUrl;
    if (!url || !stageEl) return;
    browserDock.pendingUrl = null;
    address = url;
    void go();
  });

  // Ctrl+L (via browserDock.focusAddress) bumps focusToken — focus + select-all
  // the address bar so the user can immediately type over the current URL.
  let lastFocusToken = 0;
  $effect(() => {
    const t = browserDock.focusToken;
    if (t === lastFocusToken) return;
    lastFocusToken = t;
    queueMicrotask(() => { addressEl?.focus(); addressEl?.select(); });
  });

  // Escape closes the overflow menu (the scrim already handles click-outside).
  $effect(() => {
    if (!menuOpen) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") menuOpen = false; };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  let ro: ResizeObserver | null = null;
  let urlPoll: ReturnType<typeof setInterval> | null = null;
  let unlistenLoad: UnlistenFn | null = null;
  let mounted = true;
  onDestroy(() => { mounted = false; });
  onMount(async () => {
    if (stageEl && "ResizeObserver" in window) {
      ro = new ResizeObserver(() => syncBounds());
      ro.observe(stageEl);
    }
    window.addEventListener("resize", syncBounds);
    urlPoll = setInterval(() => {
      if (opened && workspace.activeId === "chat" && !document.hidden) void syncAddress();
    }, 1200);
    // Native page-load phases → drive the spinner + keep the address honest on
    // link clicks / redirects / back-forward (faster than the 1.2s poll).
    const u = await listen<{ phase: string; url: string }>("browser://load", (e) => {
      const { phase, url } = e.payload;
      if (phase === "started") {
        loading = true;
        armLoadWatchdog();
      } else {
        loading = false;
        if (loadWatchdog) { clearTimeout(loadWatchdog); loadWatchdog = null; }
      }
      if (!inputFocused && url) address = url;
    });
    if (!mounted) { u(); return; }
    unlistenLoad = u;
  });

  onDestroy(() => {
    ro?.disconnect();
    if (urlPoll) clearInterval(urlPoll);
    if (flashTimer) clearTimeout(flashTimer);
    if (loadWatchdog) clearTimeout(loadWatchdog);
    unlistenLoad?.();
    window.removeEventListener("resize", syncBounds);
    // Closing the dock destroys the native webview (no lingering surface).
    void invoke("browser_close");
  });
</script>

<div class="wb-root">
  <div class="wb-bar">
    <button class="wb-btn" type="button" onclick={goBack} disabled={!opened} title="Back" aria-label="Back">
      <ChevronLeft size={16} />
    </button>
    <button class="wb-btn" type="button" onclick={goForward} disabled={!opened} title="Forward" aria-label="Forward">
      <ChevronRight size={16} />
    </button>
    <button class="wb-btn" type="button" onclick={reload} disabled={!opened} title="Reload" aria-label="Reload">
      <RotateCw size={15} />
    </button>

    <span class="wb-glyph" class:wb-glyph-busy={loading}><Globe size={15} /></span>
    <input
      bind:this={addressEl}
      class="wb-address"
      type="text"
      spellcheck="false"
      placeholder="Search or enter a URL…"
      bind:value={address}
      onkeydown={(e) => { if (e.key === "Enter") go(); }}
      onfocus={(e) => { inputFocused = true; e.currentTarget.select(); }}
      onblur={() => (inputFocused = false)}
    />
    <button
      class="wb-add"
      class:wb-add-done={added}
      class:wb-add-fail={failed}
      type="button"
      onclick={addToChat}
      disabled={!opened || adding}
      title="Add this page's text to the chat"
    >
      {#if added}<Check size={14} /><span>Added</span>
      {:else if failed}<X size={14} /><span>No text</span>
      {:else}<MessageSquarePlus size={14} /><span>Add to chat</span>{/if}
    </button>

    <button class="wb-btn" type="button" onclick={openMenu} disabled={!opened} title="More actions" aria-label="More actions">
      <MoreHorizontal size={16} />
    </button>

    <button class="wb-btn wb-btn-close" type="button" onclick={() => browserDock.toggle()} title="Close browser panel" aria-label="Close browser panel">
      <X size={15} />
    </button>
  </div>

  {#if menuOpen}
    <button type="button" class="wb-menu-scrim" use:portal aria-label="Close menu" onclick={() => (menuOpen = false)}></button>
    <div class="rift-menu wb-menu" use:portal role="menu" style="left: {menuPos.x}px; top: {menuPos.y}px;">
      <button type="button" class="rift-menu-row" role="menuitem" onclick={copyUrl}>
        <Copy size={14} class="rift-menu-row-ic" />
        <span class="rift-menu-row-t">Copy URL</span>
      </button>
      <button type="button" class="rift-menu-row" role="menuitem" onclick={openExternal}>
        <ExternalLink size={14} class="rift-menu-row-ic" />
        <span class="rift-menu-row-t">Open in system browser</span>
      </button>
    </div>
  {/if}

  <!-- The native child webview overlaps this stage. Keep it visually empty. -->
  <div class="wb-stage" bind:this={stageEl}>
    {#if error}
      <div class="wb-empty wb-error">{error}</div>
    {:else if !opened}
      <div class="wb-empty">
        <Globe size={40} />
        <p>Browse inside Rift — then <strong>Add to chat</strong> to let the assistant read the page you're on.</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .wb-root {
    display: flex; flex-direction: column;
    height: 100%; min-height: 0;
    background: var(--bg);
  }
  .wb-bar {
    display: flex; align-items: center; gap: 6px;
    padding: 8px 10px;
    border-bottom: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    background: color-mix(in oklch, var(--surface) 80%, transparent);
  }
  .wb-glyph { display: inline-flex; color: var(--fg-muted); margin-left: 2px; }
  .wb-glyph-busy { color: var(--accent); animation: wb-spin 0.9s linear infinite; }
  @keyframes wb-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .wb-glyph-busy { animation: none; } }
  .wb-address {
    flex: 1; min-width: 0;
    height: 30px; padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--fg);
    font: inherit; font-size: 13px;
  }
  .wb-address:focus { outline: none; border-color: var(--accent); }
  .wb-btn {
    display: inline-flex; align-items: center; justify-content: center;
    width: 30px; height: 30px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--fg-muted);
    cursor: pointer;
  }
  .wb-btn:hover:not(:disabled) { color: var(--fg); border-color: var(--accent); }
  .wb-btn:disabled { opacity: 0.4; cursor: default; }
  .wb-btn-close:hover:not(:disabled) { color: var(--danger); border-color: var(--danger); }

  /* Add-page-to-chat — the AI affordance, given accent weight to read as the
     primary action in the bar. */
  .wb-add {
    display: inline-flex; align-items: center; gap: 5px;
    height: 30px; padding: 0 10px;
    border: 1px solid color-mix(in oklab, var(--accent) 55%, var(--border));
    border-radius: 8px;
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    color: var(--fg);
    font: inherit; font-size: 12px; font-weight: 500; white-space: nowrap;
    cursor: pointer;
    transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
  }
  .wb-add:hover:not(:disabled) { background: color-mix(in oklab, var(--accent) 24%, transparent); border-color: var(--accent); }
  .wb-add:disabled { opacity: 0.4; cursor: default; }
  .wb-add-done {
    border-color: var(--ok);
    background: color-mix(in oklab, var(--ok) 16%, transparent);
    color: var(--ok);
  }
  .wb-add-fail {
    border-color: var(--danger);
    background: color-mix(in oklab, var(--danger) 16%, transparent);
    color: var(--danger);
  }
  /* Disabled state shouldn't wash out the success/fail flash mid-cooldown. */
  .wb-add-done:disabled, .wb-add-fail:disabled { opacity: 1; }

  /* Overflow menu (portalled to <body>). Chrome from .rift-menu (app.css). */
  .wb-menu-scrim {
    position: fixed; inset: 0; z-index: 999;
    background: transparent; border: 0; cursor: default;
  }
  .wb-menu {
    position: fixed; z-index: 1000;
    min-width: 200px;
    display: flex; flex-direction: column; gap: 1px;
  }
  .wb-menu :global(.rift-menu-row) { align-items: center; }

  .wb-stage {
    flex: 1; min-height: 0;
    display: flex; align-items: center; justify-content: center;
    overflow: hidden;
    background: var(--bg);
  }
  .wb-empty {
    display: flex; flex-direction: column; align-items: center; gap: 12px;
    color: var(--fg-muted);
    font-size: 13px;
    padding: 24px;
    text-align: center;
  }
  .wb-error { color: var(--danger); max-width: 480px; word-break: break-word; }
</style>
