<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Terminal as XTerm, type ITheme } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import { SearchAddon, type ISearchOptions, type ISearchDecorationOptions } from "@xterm/addon-search";
  import "@xterm/xterm/css/xterm.css";
  import { connection } from "../../state/connection.svelte";
  import { terminal as termStore, resolveFontFamily } from "../../state/terminal.svelte";
  import { buildPresetTheme } from "./themePresets";

  type SessionStartInfo = { id: string; shell_id: string; shell_label: string };
  export type SearchApi = {
    findNext: (term: string, opts?: ISearchOptions) => boolean;
    findPrevious: (term: string, opts?: ISearchOptions) => boolean;
    clearDecorations: () => void;
    onResults: (cb: (info: { resultIndex: number; resultCount: number }) => void) => () => void;
  };
  type Props = {
    visible?: boolean;
    shellId?: string | null;
    fontSize?: number;
    autoLaunch?: string;
    onSessionStart?: (info: SessionStartInfo) => void;
    onStatusChange?: (s: "idle" | "starting" | "running" | "exited" | "error", err?: string | null) => void;
    onSearchReady?: (api: SearchApi) => void;
    onSearchTeardown?: () => void;
  };
  let {
    visible = true,
    shellId = null,
    fontSize = 13,
    autoLaunch = "",
    onSessionStart,
    onStatusChange,
    onSearchReady,
    onSearchTeardown,
  }: Props = $props();

  type DataPayload = { id: string; chunk: string };
  type ExitPayload = { id: string };

  let host = $state<HTMLDivElement | undefined>();
  let term: XTerm | null = null;
  let fit: FitAddon | null = null;
  let search: SearchAddon | null = null;
  let sessionId: string | null = null;
  let unlistenData: UnlistenFn | null = null;
  let unlistenExit: UnlistenFn | null = null;
  let ro: ResizeObserver | null = null;
  let resizeTimer: ReturnType<typeof setTimeout> | null = null;
  let status = $state<"idle" | "starting" | "running" | "exited" | "error">("idle");
  let errorMsg = $state<string | null>(null);
  let bellFlash = $state(false);
  let bellTimer: ReturnType<typeof setTimeout> | null = null;
  let unlistenBell: { dispose: () => void } | null = null;

  function readVar(name: string, fallback: string): string {
    if (typeof window === "undefined") return fallback;
    const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return v || fallback;
  }
  function buildTheme(): ITheme {
    return buildPresetTheme(termStore.themePreset, readVar);
  }

  async function init() {
    if (!host || term) return;
    status = "starting";
    onStatusChange?.(status, null);
    errorMsg = null;
    term = new XTerm({
      fontFamily: resolveFontFamily(termStore.fontFamilyPreset, termStore.fontFamilyCustom),
      fontSize,
      lineHeight: 1.2,
      cursorBlink: termStore.cursorBlink,
      cursorStyle: termStore.cursorStyle,
      scrollback: termStore.scrollback,
      allowProposedApi: true,
      theme: buildTheme(),
      convertEol: false,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.loadAddon(new WebLinksAddon());
    search = new SearchAddon();
    term.loadAddon(search);
    term.open(host);

    // Pull theme colors so search-result decorations stay readable on every
    // preset. xterm.js needs hex/rgb here — CSS vars won't resolve.
    const accent = readVar("--accent", "#a78bfa");
    const accentSoft = readVar("--accent-soft", "rgba(167,139,250,0.32)");
    const warn = readVar("--warn", "#fbbf24");
    const decorations: ISearchDecorationOptions = {
      matchBackground: accentSoft,
      matchBorder: accent,
      matchOverviewRuler: accent,
      activeMatchBackground: accent,
      activeMatchBorder: warn,
      activeMatchColorOverviewRuler: warn,
    };

    const searchApi: SearchApi = {
      findNext: (q, opts) => search?.findNext(q, { decorations, ...opts }) ?? false,
      findPrevious: (q, opts) => search?.findPrevious(q, { decorations, ...opts }) ?? false,
      clearDecorations: () => { try { search?.clearDecorations(); } catch { /* noop */ } },
      onResults: (cb) => {
        const d = search?.onDidChangeResults(cb);
        return () => { try { d?.dispose(); } catch { /* noop */ } };
      },
    };
    onSearchReady?.(searchApi);
    try { fit.fit(); } catch { /* container not measured yet */ }

    const cols = term.cols;
    const rows = term.rows;
    const cwd = await invoke<string>("term_default_cwd", {
      profileLocalRoot: connection.selected?.localRoot ?? null,
    }).catch(() => "");

    try {
      const info = await invoke<SessionStartInfo>("term_spawn", {
        shellId,
        cwd,
        cols,
        rows,
      });
      // Teardown raced the spawn — kill the orphan and bail.
      if (!term) {
        try { await invoke("term_kill", { id: info.id }); } catch { /* noop */ }
        return;
      }
      sessionId = info.id;
      status = "running";
      onSessionStart?.(info);
      onStatusChange?.(status, null);
      // Per-user auto-launch preset: e.g. set to "claude" → every new tab
      // boots straight into Claude Code. Small delay so the shell's PS1 is
      // drawn first; otherwise the command lands before the prompt prints.
      if (autoLaunch && autoLaunch.trim()) {
        const cmd = autoLaunch.trim();
        const sid = sessionId;
        setTimeout(() => {
          if (sid && sessionId === sid) {
            void invoke("term_write", { id: sid, data: cmd + "\r" })
              .catch((e) => console.warn("autoLaunch write failed", e));
          }
        }, 250);
      }
    } catch (e) {
      errorMsg = String(e);
      status = "error";
      onStatusChange?.(status, errorMsg);
      return;
    }

    unlistenData = await listen<DataPayload>("term:data", (ev) => {
      if (ev.payload.id !== sessionId || !term) return;
      term.write(ev.payload.chunk);
    });
    unlistenExit = await listen<ExitPayload>("term:exit", (ev) => {
      if (ev.payload.id !== sessionId) return;
      status = "exited";
      sessionId = null;
      onStatusChange?.(status, null);
    });

    term.onData((data) => {
      if (!sessionId) return;
      void invoke("term_write", { id: sessionId, data }).catch((e) => console.warn("term_write", e));
    });
    // Bell handler — xterm.js v5 dropped the bellStyle option, so we react
    // to the bell event manually: visual = flash border; sound = short beep.
    unlistenBell = term.onBell(() => {
      const mode = termStore.bellStyle;
      if (mode === "visual") {
        bellFlash = true;
        if (bellTimer) clearTimeout(bellTimer);
        bellTimer = setTimeout(() => { bellFlash = false; }, 220);
      } else if (mode === "sound") {
        playBellTone();
      }
    });
    // Copy on select — write current selection to clipboard whenever the
    // selection changes (xterm fires on every drag tick; clipboard write is
    // cheap and only fires when content actually changes).
    term.onSelectionChange(() => {
      if (!termStore.copyOnSelect || !term) return;
      const sel = term.getSelection();
      if (!sel) return;
      void navigator.clipboard.writeText(sel).catch(() => { /* clipboard blocked */ });
    });
    term.onResize(({ cols, rows }) => {
      if (!sessionId) return;
      void invoke("term_resize", { id: sessionId, cols, rows }).catch((e) => console.warn("term_resize", e));
    });

    ro = new ResizeObserver(() => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        try { fit?.fit(); } catch { /* container collapsed */ }
      }, 60);
    });
    ro.observe(host);
  }

  async function teardown() {
    if (resizeTimer) { clearTimeout(resizeTimer); resizeTimer = null; }
    if (bellTimer) { clearTimeout(bellTimer); bellTimer = null; }
    ro?.disconnect(); ro = null;
    if (unlistenData) { unlistenData(); unlistenData = null; }
    if (unlistenExit) { unlistenExit(); unlistenExit = null; }
    if (unlistenBell) { unlistenBell.dispose(); unlistenBell = null; }
    if (sessionId) {
      const id = sessionId;
      sessionId = null;
      try { await invoke("term_kill", { id }); } catch { /* already exited */ }
    }
    onSearchTeardown?.();
    try { search?.dispose(); } catch { /* noop */ }
    try { fit?.dispose(); } catch { /* noop */ }
    term?.dispose();
    term = null;
    fit = null;
    search = null;
    status = "idle";
    onStatusChange?.(status, null);
  }

  $effect(() => {
    if (visible && !term) {
      void init();
    }
  });

  $effect(() => {
    if (visible && term && fit) {
      queueMicrotask(() => { try { fit!.fit(); } catch { /* noop */ } });
    }
  });

  // Live-apply settings changes from the Settings panel. Each effect reads
  // exactly one store field so updates re-fit only when needed.
  $effect(() => {
    const family = resolveFontFamily(termStore.fontFamilyPreset, termStore.fontFamilyCustom);
    if (!term) return;
    term.options.fontFamily = family;
    try { fit?.fit(); } catch { /* noop */ }
  });
  $effect(() => {
    const style = termStore.cursorStyle;
    if (term) term.options.cursorStyle = style;
  });
  $effect(() => {
    const blink = termStore.cursorBlink;
    if (term) term.options.cursorBlink = blink;
  });
  $effect(() => {
    const sb = termStore.scrollback;
    if (term) term.options.scrollback = sb;
  });
  // Audio bell — single shared AudioContext, lazily created on first ring.
  // Browsers gate AudioContext on user gesture, but the bell only fires from
  // shell output so the user has invariably typed already.
  let bellCtx: AudioContext | null = null;
  function playBellTone() {
    try {
      if (!bellCtx) bellCtx = new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)();
      const ctx = bellCtx;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = 880;
      gain.gain.setValueAtTime(0.0001, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.18, ctx.currentTime + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.15);
      osc.connect(gain).connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.16);
    } catch { /* audio blocked */ }
  }
  $effect(() => {
    // Re-touch theme preset to swap palettes live.
    const _id = termStore.themePreset;
    void _id;
    if (term) term.options.theme = buildTheme();
  });

  function onContextMenu(e: MouseEvent) {
    if (!termStore.rightClickPaste || !sessionId) return;
    e.preventDefault();
    navigator.clipboard.readText().then((txt) => {
      if (txt && sessionId) {
        void invoke("term_write", { id: sessionId, data: txt }).catch((err) => console.warn("paste", err));
      }
    }).catch(() => { /* clipboard blocked */ });
  }

  async function restart() {
    await teardown();
    if (visible) await init();
  }

  onMount(() => { if (visible) void init(); });
  onDestroy(() => { void teardown(); });
</script>

<div class="term-wrap" data-status={status} data-bell={bellFlash}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="term-host" bind:this={host} oncontextmenu={onContextMenu}></div>
  {#if status === "error" && errorMsg}
    <div class="term-overlay">
      <div class="term-error">
        <div class="term-error-title">Terminal failed to start</div>
        <div class="term-error-msg mono">{errorMsg}</div>
        <button type="button" class="btn" onclick={restart}>Retry</button>
      </div>
    </div>
  {:else if status === "exited"}
    <div class="term-overlay">
      <div class="term-error">
        <div class="term-error-title">Shell exited</div>
        <button type="button" class="btn primary" onclick={restart}>Restart</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .term-wrap {
    position: relative;
    flex: 1; min-height: 0;
    display: flex;
    background: var(--bg);
    overflow: hidden;
    transition: box-shadow 60ms ease-out;
  }
  .term-wrap[data-bell="true"] {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .term-host {
    flex: 1; min-height: 0; min-width: 0;
    padding: 6px 8px 4px;
  }
  .term-host :global(.xterm) { height: 100%; }
  .term-host :global(.xterm-viewport) {
    background: transparent !important;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }
  .term-host :global(.xterm-viewport::-webkit-scrollbar) {
    width: 10px;
    background: transparent;
  }
  .term-host :global(.xterm-viewport::-webkit-scrollbar-track) {
    background: transparent;
  }
  .term-host :global(.xterm-viewport::-webkit-scrollbar-thumb) {
    background: var(--border-strong);
    border: 2px solid transparent;
    background-clip: padding-box;
    border-radius: 6px;
    transition: background 120ms ease;
  }
  .term-host :global(.xterm-viewport::-webkit-scrollbar-thumb:hover) {
    background: var(--fg-faint);
    background-clip: padding-box;
  }
  .term-host :global(.xterm-viewport::-webkit-scrollbar-corner) {
    background: transparent;
  }
  .term-overlay {
    position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
    background: color-mix(in oklch, var(--bg) 88%, transparent);
    backdrop-filter: blur(2px);
  }
  .term-error {
    display: flex; flex-direction: column; gap: 10px;
    align-items: center;
    padding: 18px 22px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    max-width: 80%;
  }
  .term-error-title {
    color: var(--fg);
    font-size: var(--fs-sm);
    font-weight: 600;
  }
  .term-error-msg {
    color: var(--danger);
    font-size: var(--fs-xs);
    text-align: center;
    word-break: break-word;
  }
</style>
