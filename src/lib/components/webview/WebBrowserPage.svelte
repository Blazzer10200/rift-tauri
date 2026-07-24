<script lang="ts">
  // In-assistant web browser dock. Renders an address bar + an empty "stage"
  // placeholder; the actual page is a NATIVE child webview (Tauri multiwebview)
  // positioned by the Rust `browser` module to overlap the stage. We report the
  // stage's rect and show/hide the native webview as the dock's visibility
  // changes. See src-tauri/src/browser/mod.rs.
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { onDestroy, onMount, untrack } from "svelte";
  import { Globe, RotateCw, ChevronLeft, ChevronRight, MessageSquarePlus, Check, X, Copy, ExternalLink, MoreHorizontal, Sparkles, CircleAlert, Search, ChevronUp, ChevronDown, ZoomIn, ZoomOut } from "lucide-svelte";
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

  // Favicon of the current page as a data: URI (`browser://icon`) — the app
  // CSP blocks external image URLs, so the backend inlines the bytes.
  let favicon = $state<string | null>(null);
  // Console badge: error/warn tally polled alongside the URL sync.
  let consoleCounts = $state<{ errors: number; warns: number }>({ errors: 0, warns: 0 });
  let consoleBusy = $state(false);
  let consoleFlash = $state<"ok" | "fail" | null>(null);
  let consoleFlashTimer: ReturnType<typeof setTimeout> | null = null;
  // Find-in-page bar (Ctrl+F). `window.find` scrolls to + highlights matches
  // natively — no page cooperation needed.
  let findOpen = $state(false);
  let findQuery = $state("");
  let findEl = $state<HTMLInputElement | null>(null);

  // Page zoom for the dock webview — independent of the app-chrome UI zoom.
  // 1 = 100%. Re-applied after each navigation (a fresh document resets it).
  let zoom = $state(1);
  const ZOOM_MIN = 0.5;
  const ZOOM_MAX = 3;

  // "Read by assistant" chip — driven by browserDock.assistantRead (set from
  // the tool stream when a read_browser_* tool fires), auto-fades.
  let aiRead = $state<"page" | "console" | null>(null);
  let aiReadTimer: ReturnType<typeof setTimeout> | null = null;
  let lastAiReadToken = 0;
  $effect(() => {
    const r = browserDock.assistantRead;
    if (!r || r.token === lastAiReadToken) return;
    lastAiReadToken = r.token;
    aiRead = r.kind;
    if (aiReadTimer) clearTimeout(aiReadTimer);
    aiReadTimer = setTimeout(() => (aiRead = null), 2600);
  });

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

  function hostLabel(u: string | null): string {
    if (!u) return "";
    try { return new URL(u).host || u; } catch { return u; }
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

  // Find-in-page: window.find (native Chromium search) scrolls to the next/prev
  // match, wrapping at the ends. Debounced on input via the keydown handler.
  async function runFind(backwards = false) {
    if (!opened || !findQuery.trim()) return;
    try { await invoke("browser_find", { query: findQuery, backwards }); }
    catch (e) { console.warn("browser_find:", e); }
  }
  function closeFind() {
    findOpen = false;
    findQuery = "";
    void invoke("browser_clear_find").catch(() => { /* dock closed */ });
  }

  async function applyZoom(next: number) {
    zoom = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, Math.round(next * 100) / 100));
    try { await invoke("browser_set_zoom", { factor: zoom }); }
    catch (e) { console.warn("browser_set_zoom:", e); }
  }
  function zoomIn() { void applyZoom(zoom + 0.1); }
  function zoomOut() { void applyZoom(zoom - 0.1); }
  function zoomReset() { void applyZoom(1); }

  // Focus the address bar from the empty-state quick action (opens the omnibox).
  function focusAddressBar() { addressEl?.focus(); addressEl?.select(); }

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
      // Neutralize the closing sentinel inside the page body so a hostile page
      // can't emit a literal "[End page context]" to escape the delimited block
      // and inject instructions into the prompt (zero-width space breaks the
      // exact match while staying invisible).
      const body = (p.text || "").trim()
        .replace(/\[End page context\]/gi, "[End page context​]")
        .replace(/\[Page context:/gi, "[Page context​:");
      if (!body) { flash("fail"); return; }
      // Sanitize title/url to prevent ] or newlines from breaking the delimiter.
      const safeTitle = (p.title || "untitled").replace(/[[\]\r\n]/g, " ").slice(0, 200);
      const safeUrl = (p.url || "").replace(/[[\]\r\n]/g, " ").slice(0, 2048);
      const head = `[Page context: ${safeTitle} — ${safeUrl}]`;
      const tail = p.truncated ? `\n[…truncated — full page is ${p.full_len.toLocaleString()} chars]` : "";
      const block = `${head}\n${body}${tail}\n[End page context]`;
      const cur = assistant.composerDraft;
      const next = cur ? `${cur}\n\n${block}` : block;
      if (next.length > 200_000) { flash("fail"); return; }
      assistant.composerDraft = next;
      flash("ok");
    } catch (e) {
      console.warn("browser_read_page:", e);
      flash("fail");
    } finally {
      adding = false;
    }
  }

  async function syncConsoleCounts() {
    if (!opened) return;
    try {
      consoleCounts = await invoke<{ errors: number; warns: number }>("browser_console_counts");
    } catch { /* dock closed mid-poll */ }
  }

  function flashConsole(kind: "ok" | "fail") {
    consoleFlash = kind;
    if (consoleFlashTimer) clearTimeout(consoleFlashTimer);
    consoleFlashTimer = setTimeout(() => (consoleFlash = null), 1500);
  }

  // Console twin of addToChat: drop the current page's console buffer into the
  // composer as a labelled, sentinel-neutralized context block.
  async function addConsoleToChat() {
    if (!opened || consoleBusy) return;
    consoleBusy = true;
    try {
      const s = await invoke<{ url: string; entries: { level: string; text: string; ts: number }[]; dropped: number }>(
        "browser_read_console",
      );
      if (!s.entries.length) { flashConsole("fail"); return; }
      const lines = s.entries
        .map((e) => `[${e.level}] ${e.text}`)
        .join("\n")
        .replace(/\[End console output\]/gi, "[End console output​]")
        .replace(/\[Console output:/gi, "[Console output​:");
      const safeUrl = (s.url || "").replace(/[[\]\r\n]/g, " ").slice(0, 2048);
      const block = `[Console output: ${safeUrl}]\n${lines}\n[End console output]`;
      const cur = assistant.composerDraft;
      const next = cur ? `${cur}\n\n${block}` : block;
      if (next.length > 200_000) { flashConsole("fail"); return; }
      assistant.composerDraft = next;
      flashConsole("ok");
    } catch (e) {
      console.warn("browser_read_console:", e);
      flashConsole("fail");
    } finally {
      consoleBusy = false;
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
    untrack(() => {
      browserDock.pendingUrl = null;
      address = url;
      void go();
    });
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

  // Ctrl+F (via browserDock.requestFind) bumps findToken — open + focus the
  // find bar so the user can immediately type a query.
  let lastFindToken = 0;
  $effect(() => {
    const t = browserDock.findToken;
    if (t === lastFindToken) return;
    lastFindToken = t;
    findOpen = true;
    queueMicrotask(() => { findEl?.focus(); findEl?.select(); });
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
  let unlistenIcon: UnlistenFn | null = null;
  let mounted = true;
  onDestroy(() => { mounted = false; });
  onMount(async () => {
    if (stageEl && "ResizeObserver" in window) {
      ro = new ResizeObserver(() => syncBounds());
      ro.observe(stageEl);
    }
    window.addEventListener("resize", syncBounds);
    urlPoll = setInterval(() => {
      if (opened && workspace.activeId === "chat" && !document.hidden) {
        void syncAddress();
        void syncConsoleCounts();
      }
    }, 1200);
    // Native page-load phases → drive the spinner + keep the address honest on
    // link clicks / redirects / back-forward (faster than the 1.2s poll).
    const u = await listen<{ phase: string; url: string }>("browser://load", (e) => {
      const { phase, url } = e.payload;
      if (phase === "started") {
        loading = true;
        // New document — stale favicon + console tallies must not linger.
        favicon = null;
        consoleCounts = { errors: 0, warns: 0 };
        armLoadWatchdog();
      } else {
        loading = false;
        if (loadWatchdog) { clearTimeout(loadWatchdog); loadWatchdog = null; }
        if (url) browserDock.setLastUrl(url);
        // A fresh document resets the webview zoom — re-apply the user's factor.
        if (zoom !== 1) void invoke("browser_set_zoom", { factor: zoom }).catch(() => {});
      }
      if (!inputFocused && url) address = url;
    });
    const ui = await listen<{ page: string; icon: string }>("browser://icon", (e) => {
      const icon = e.payload?.icon ?? "";
      // Only data:image/* ever renders — anything else keeps the glyph.
      if (/^data:image\//.test(icon)) favicon = icon;
    });
    if (!mounted) { u(); ui(); return; }
    unlistenLoad = u;
    unlistenIcon = ui;
  });

  onDestroy(() => {
    ro?.disconnect();
    if (urlPoll) clearInterval(urlPoll);
    if (flashTimer) clearTimeout(flashTimer);
    if (loadWatchdog) clearTimeout(loadWatchdog);
    if (consoleFlashTimer) clearTimeout(consoleFlashTimer);
    if (aiReadTimer) clearTimeout(aiReadTimer);
    unlistenLoad?.();
    unlistenIcon?.();
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

    <span class="wb-glyph" class:wb-glyph-busy={loading}>
      {#if !loading && favicon}
        <img class="wb-fav" src={favicon} alt="" onerror={() => (favicon = null)} />
      {:else}
        <Globe size={15} />
      {/if}
    </span>
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
    {#if aiRead}
      <span
        class="wb-airead"
        role="status"
        title={aiRead === "console" ? "The assistant just read this page's console" : "The assistant just read this page"}
      >
        <Sparkles size={12} />
        <span>{aiRead === "console" ? "Console read" : "Page read"}</span>
      </span>
    {/if}
    {#if opened && (consoleCounts.errors > 0 || consoleCounts.warns > 0)}
      <button
        class="wb-console"
        class:wb-console-warnonly={consoleCounts.errors === 0}
        class:wb-console-ok={consoleFlash === "ok"}
        class:wb-console-fail={consoleFlash === "fail"}
        type="button"
        onclick={addConsoleToChat}
        disabled={consoleBusy}
        title="{consoleCounts.errors} console errors, {consoleCounts.warns} warnings — click to add the console output to the chat"
      >
        <CircleAlert size={13} />
        <span>{consoleCounts.errors > 0 ? consoleCounts.errors : consoleCounts.warns}</span>
      </button>
    {/if}
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

    {#if loading}<span class="wb-progress" aria-hidden="true"></span>{/if}
  </div>

  {#if findOpen}
    <div class="wb-find">
      <Search size={14} class="wb-find-ic" />
      <input
        bind:this={findEl}
        class="wb-find-input"
        type="text"
        spellcheck="false"
        placeholder="Find on page…"
        bind:value={findQuery}
        oninput={() => runFind(false)}
        onkeydown={(e) => {
          if (e.key === "Enter") { e.preventDefault(); runFind(e.shiftKey); }
          else if (e.key === "Escape") { e.preventDefault(); closeFind(); }
        }}
      />
      <button class="wb-find-btn" type="button" onclick={() => runFind(true)} title="Previous match (Shift+Enter)" aria-label="Previous match"><ChevronUp size={15} /></button>
      <button class="wb-find-btn" type="button" onclick={() => runFind(false)} title="Next match (Enter)" aria-label="Next match"><ChevronDown size={15} /></button>
      <button class="wb-find-btn" type="button" onclick={closeFind} title="Close find (Esc)" aria-label="Close find"><X size={14} /></button>
    </div>
  {/if}

  {#if menuOpen}
    <button type="button" class="wb-menu-scrim" use:portal aria-label="Close menu" onclick={() => (menuOpen = false)}></button>
    <div class="rift-menu wb-menu" use:portal role="menu" style="left: {menuPos.x}px; top: {menuPos.y}px;">
      <button type="button" class="rift-menu-row" role="menuitem" onclick={() => { menuOpen = false; browserDock.requestFind(); }}>
        <Search size={14} class="rift-menu-row-ic" />
        <span class="rift-menu-row-t">Find on page</span>
        <span class="wb-menu-kbd">Ctrl F</span>
      </button>
      <div class="wb-zoomrow" role="group" aria-label="Page zoom">
        <button type="button" class="wb-zoom-btn" onclick={zoomOut} disabled={zoom <= ZOOM_MIN} title="Zoom out" aria-label="Zoom out"><ZoomOut size={14} /></button>
        <button type="button" class="wb-zoom-pct" onclick={zoomReset} title="Reset to 100%">{Math.round(zoom * 100)}%</button>
        <button type="button" class="wb-zoom-btn" onclick={zoomIn} disabled={zoom >= ZOOM_MAX} title="Zoom in" aria-label="Zoom in"><ZoomIn size={14} /></button>
      </div>
      <div class="wb-menu-sep" aria-hidden="true"></div>
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
        <div class="wb-hero">
          <span class="wb-hero-ic"><Globe size={30} /></span>
          <h3>Browse inside Rift</h3>
          <p>A real browser beside the chat — and the assistant can see what's on it.</p>
        </div>

        <ul class="wb-caps">
          <li>
            <MessageSquarePlus size={15} />
            <span><strong>Add to chat</strong> shares this page's live text with the assistant — even behind a login, where a plain fetch can't reach.</span>
          </li>
          <li>
            <CircleAlert size={15} />
            <span>The <strong>console badge</strong> appears when a page logs errors — one click hands them to the chat.</span>
          </li>
          <li>
            <Sparkles size={15} />
            <span>Ask the assistant to <strong>open a page</strong> and it opens right here, ready to read.</span>
          </li>
        </ul>

        <div class="wb-quick">
          <button class="wb-quick-btn wb-quick-primary" type="button" onclick={focusAddressBar}>
            <Search size={14} />
            <span>Search or enter a URL</span>
          </button>
          {#if browserDock.lastUrl}
            <button
              class="wb-quick-btn"
              type="button"
              onclick={() => { address = browserDock.lastUrl ?? ""; void go(); }}
              title={browserDock.lastUrl}
            >
              <RotateCw size={13} />
              <span>Reopen {hostLabel(browserDock.lastUrl)}</span>
            </button>
          {/if}
        </div>

        <p class="wb-hint">
          <kbd>Ctrl</kbd><kbd>L</kbd> address · <kbd>Ctrl</kbd><kbd>F</kbd> find · <kbd>Shift</kbd><kbd>B</kbd> toggle panel
        </p>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Transparent chrome — the app dot-field stays continuous (the native child
     webview paints its own surface over .wb-stage when a page is open). */
  .wb-root {
    display: flex; flex-direction: column;
    height: 100%; min-height: 0;
    background: transparent;
    container-type: inline-size;
  }
  .wb-bar {
    position: relative;
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

  .wb-fav { width: 15px; height: 15px; border-radius: 3px; display: block; }

  /* Loading: thin accent comet sweeping the bar's bottom edge. */
  .wb-progress {
    position: absolute; left: 0; right: 0; bottom: -1px; height: 2px;
    overflow: hidden; pointer-events: none;
  }
  .wb-progress::before {
    content: ""; position: absolute; inset: 0; width: 40%;
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    animation: wb-sweep 1.1s ease-in-out infinite;
  }
  @keyframes wb-sweep {
    from { transform: translateX(-100%); }
    to { transform: translateX(350%); }
  }
  @media (prefers-reduced-motion: reduce) {
    .wb-progress::before { animation: none; width: 100%; opacity: 0.5; }
  }

  /* "Read by assistant" — accent shimmer pill, appears on each AI read so the
     user always sees when the model looked at the page. */
  .wb-airead {
    display: inline-flex; align-items: center; gap: 5px;
    height: 24px; padding: 0 9px; flex: none;
    border: 1px solid color-mix(in oklab, var(--accent) 55%, var(--border));
    border-radius: 999px;
    color: var(--accent);
    font-size: 11px; font-weight: 600; white-space: nowrap;
    background:
      linear-gradient(110deg,
        color-mix(in oklab, var(--accent) 10%, transparent) 30%,
        color-mix(in oklab, var(--accent) 26%, transparent) 50%,
        color-mix(in oklab, var(--accent) 10%, transparent) 70%);
    background-size: 200% 100%;
    animation: wb-shimmer 1.5s linear infinite, wb-airead-in var(--dur-fast) ease;
  }
  @keyframes wb-shimmer {
    from { background-position: 120% 0; }
    to { background-position: -80% 0; }
  }
  @keyframes wb-airead-in {
    from { opacity: 0; transform: translateY(2px); }
  }
  @media (prefers-reduced-motion: reduce) { .wb-airead { animation: none; } }

  /* Console badge — danger when errors, warn tint when warnings only. */
  .wb-console {
    display: inline-flex; align-items: center; gap: 4px;
    height: 30px; padding: 0 8px; flex: none;
    border: 1px solid color-mix(in oklab, var(--danger) 55%, var(--border));
    border-radius: 8px;
    background: color-mix(in oklab, var(--danger) 14%, transparent);
    color: var(--danger);
    font: inherit; font-size: 12px; font-weight: 600;
    cursor: pointer;
    transition: background var(--dur-fast) ease, border-color var(--dur-fast) ease, color var(--dur-fast) ease;
  }
  .wb-console:hover:not(:disabled) { background: color-mix(in oklab, var(--danger) 24%, transparent); }
  .wb-console:disabled { opacity: 0.6; cursor: default; }
  .wb-console-warnonly {
    border-color: color-mix(in oklab, var(--warn) 55%, var(--border));
    background: var(--warn-soft);
    color: var(--warn);
  }
  .wb-console-warnonly:hover:not(:disabled) { background: color-mix(in oklab, var(--warn) 24%, transparent); }
  .wb-console-ok, .wb-console-ok.wb-console-warnonly {
    border-color: var(--ok);
    background: color-mix(in oklab, var(--ok) 16%, transparent);
    color: var(--ok);
  }
  .wb-console-fail, .wb-console-fail.wb-console-warnonly {
    border-color: var(--danger);
    background: color-mix(in oklab, var(--danger) 16%, transparent);
    color: var(--danger);
  }

  /* Narrow dock: labels give way, icons stay. */
  @container (max-width: 470px) {
    .wb-add span, .wb-airead span { display: none; }
  }

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
    transition: background var(--dur-fast) ease, border-color var(--dur-fast) ease, color var(--dur-fast) ease;
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
  .wb-menu-kbd {
    margin-left: auto;
    font-size: 10px; font-weight: 600; letter-spacing: 0.02em;
    color: var(--fg-faint);
  }
  .wb-menu-sep {
    height: 1px; margin: 3px 6px;
    background: color-mix(in oklch, var(--border) 70%, transparent);
  }
  /* Zoom stepper row inside the overflow menu. */
  .wb-zoomrow {
    display: flex; align-items: center; gap: 4px;
    padding: 3px 6px;
  }
  .wb-zoom-btn {
    display: inline-flex; align-items: center; justify-content: center;
    width: 28px; height: 26px;
    border: 1px solid var(--border); border-radius: 7px;
    background: var(--bg); color: var(--fg-muted);
    cursor: pointer;
  }
  .wb-zoom-btn:hover:not(:disabled) { color: var(--fg); border-color: var(--accent); }
  .wb-zoom-btn:disabled { opacity: 0.4; cursor: default; }
  .wb-zoom-pct {
    flex: 1; height: 26px;
    border: 1px solid transparent; border-radius: 7px;
    background: transparent; color: var(--fg);
    font: inherit; font-size: 12px; font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .wb-zoom-pct:hover { background: var(--bg-elev-2); }

  /* Find-in-page bar — sits under the toolbar; native window.find drives it. */
  .wb-find {
    display: flex; align-items: center; gap: 4px;
    padding: 6px 10px;
    border-bottom: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    background: color-mix(in oklch, var(--surface) 80%, transparent);
  }
  .wb-find :global(.wb-find-ic) { color: var(--fg-muted); flex: none; margin-right: 2px; }
  .wb-find-input {
    flex: 1; min-width: 0;
    height: 28px; padding: 0 8px;
    border: 1px solid var(--border); border-radius: 7px;
    background: var(--bg); color: var(--fg);
    font: inherit; font-size: 13px;
  }
  .wb-find-input:focus { outline: none; border-color: var(--accent); }
  .wb-find-btn {
    display: inline-flex; align-items: center; justify-content: center;
    width: 28px; height: 28px; flex: none;
    border: 1px solid var(--border); border-radius: 7px;
    background: var(--bg); color: var(--fg-muted);
    cursor: pointer;
  }
  .wb-find-btn:hover { color: var(--fg); border-color: var(--accent); }

  .wb-stage {
    flex: 1; min-height: 0;
    display: flex; align-items: center; justify-content: center;
    overflow: hidden;
    background: transparent;
  }
  .wb-empty {
    display: flex; flex-direction: column; align-items: center; gap: 18px;
    color: var(--fg-muted);
    font-size: 13px;
    padding: 28px 24px;
    max-width: 440px;
    text-align: center;
  }
  .wb-error { color: var(--danger); max-width: 480px; word-break: break-word; }

  /* Hero — icon chip + title + one-line pitch. */
  .wb-hero { display: flex; flex-direction: column; align-items: center; gap: 8px; }
  .wb-hero-ic {
    display: inline-flex; align-items: center; justify-content: center;
    width: 56px; height: 56px; margin-bottom: 2px;
    border: 1px solid color-mix(in oklab, var(--accent) 35%, var(--border));
    border-radius: 16px;
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    color: var(--accent);
  }
  .wb-hero h3 { margin: 0; font-size: 15px; font-weight: 650; color: var(--fg); }
  .wb-hero p { margin: 0; font-size: 12.5px; line-height: 1.45; color: var(--fg-muted); }

  /* Capability list — what the browser (and the assistant) can do here. */
  .wb-caps {
    list-style: none; margin: 0; padding: 0;
    display: flex; flex-direction: column; gap: 10px;
    text-align: left; width: 100%;
  }
  .wb-caps li {
    display: flex; align-items: flex-start; gap: 9px;
    font-size: 12.5px; line-height: 1.4; color: var(--fg-muted);
  }
  .wb-caps li :global(svg) { flex: none; margin-top: 1px; color: var(--accent); }
  .wb-caps strong { color: var(--fg); font-weight: 600; }

  /* Quick actions — the two primary ways in. */
  .wb-quick { display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; }
  .wb-quick-btn {
    display: inline-flex; align-items: center; gap: 6px;
    height: 30px; padding: 0 14px;
    border: 1px solid var(--border); border-radius: 999px;
    background: var(--bg); color: var(--fg-muted);
    font: inherit; font-size: 12px; cursor: pointer;
    transition: border-color var(--dur-fast) ease, color var(--dur-fast) ease, background var(--dur-fast) ease;
  }
  .wb-quick-btn:hover { color: var(--fg); border-color: var(--accent); }
  .wb-quick-primary {
    border-color: color-mix(in oklab, var(--accent) 55%, var(--border));
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    color: var(--fg);
  }
  .wb-quick-primary:hover { background: color-mix(in oklab, var(--accent) 22%, transparent); }

  /* Keyboard hint strip. */
  .wb-hint { margin: 0; font-size: 11px; color: var(--fg-faint); line-height: 1.6; }
  .wb-hint kbd {
    display: inline-block; padding: 1px 5px; margin: 0 1px;
    border: 1px solid var(--border); border-bottom-width: 2px; border-radius: 4px;
    background: var(--bg-elev-2); color: var(--fg-muted);
    font-family: inherit; font-size: 10px; font-weight: 600;
  }
</style>
