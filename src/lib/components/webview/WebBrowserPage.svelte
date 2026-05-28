<script lang="ts">
  // In-assistant web browser dock. Renders an address bar + an empty "stage"
  // placeholder; the actual page is a NATIVE child webview (Tauri multiwebview)
  // positioned by the Rust `browser` module to overlap the stage. We report the
  // stage's rect and show/hide the native webview as the dock's visibility
  // changes. See src-tauri/src/browser/mod.rs.
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy, onMount } from "svelte";
  import { Globe, RotateCw, ArrowRight, X } from "lucide-svelte";
  import { workspace } from "../../state/workspace.svelte";
  import { browserDock } from "../../state/browserDock.svelte";

  let address = $state("https://example.com");
  let opened = $state(false);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let stageEl: HTMLDivElement | null = $state(null);
  // True while the address input is focused — don't overwrite what the user is
  // typing with the synced live URL.
  let inputFocused = $state(false);

  function normalizeUrl(raw: string): string {
    const t = raw.trim();
    if (!t) return t;
    if (/^[a-z]+:\/\//i.test(t) || t.startsWith("data:")) return t;
    return `https://${t}`;
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
    } catch (e) {
      error = String(e);
      opened = false;
    } finally {
      loading = false;
    }
  }

  async function reload() {
    if (!opened) return;
    try {
      // Reload the page that's actually showing (it may have navigated via
      // in-page link clicks), not the stale address-bar value.
      const live = await invoke<string>("browser_current_url");
      await invoke("browser_navigate", { url: live || normalizeUrl(address) });
    } catch (e) { error = String(e); }
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
      void invoke("browser_show").then(syncBounds);
    } else {
      void invoke("browser_hide");
    }
  });

  let ro: ResizeObserver | null = null;
  let urlPoll: ReturnType<typeof setInterval> | null = null;
  onMount(() => {
    if (stageEl && "ResizeObserver" in window) {
      ro = new ResizeObserver(() => syncBounds());
      ro.observe(stageEl);
    }
    window.addEventListener("resize", syncBounds);
    urlPoll = setInterval(() => {
      if (opened && workspace.activeId === "chat" && !document.hidden) void syncAddress();
    }, 1200);
  });

  onDestroy(() => {
    ro?.disconnect();
    if (urlPoll) clearInterval(urlPoll);
    window.removeEventListener("resize", syncBounds);
    // Closing the dock destroys the native webview (no lingering surface).
    void invoke("browser_close");
  });
</script>

<div class="wb-root">
  <div class="wb-bar">
    <span class="wb-glyph"><Globe size={15} /></span>
    <input
      class="wb-address"
      type="text"
      spellcheck="false"
      placeholder="Enter a URL…"
      bind:value={address}
      onkeydown={(e) => { if (e.key === "Enter") go(); }}
      onfocus={() => (inputFocused = true)}
      onblur={() => (inputFocused = false)}
    />
    <button class="wb-btn" type="button" onclick={go} disabled={loading} title="Go">
      <ArrowRight size={15} />
    </button>
    <button class="wb-btn" type="button" onclick={reload} disabled={!opened} title="Reload">
      <RotateCw size={15} />
    </button>
    <button class="wb-btn" type="button" onclick={() => browserDock.toggle()} title="Close browser panel">
      <X size={15} />
    </button>
  </div>

  <!-- The native child webview overlaps this stage. Keep it visually empty. -->
  <div class="wb-stage" bind:this={stageEl}>
    {#if error}
      <div class="wb-empty wb-error">{error}</div>
    {:else if !opened}
      <div class="wb-empty">
        <Globe size={40} />
        <p>Enter a URL and press Go to browse inside Rift.</p>
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
  .wb-glyph { display: inline-flex; color: var(--fg-muted); }
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

  .wb-stage {
    flex: 1; min-height: 0;
    display: flex; align-items: center; justify-content: center;
    overflow: hidden;
    background: #0b0b0d;
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
