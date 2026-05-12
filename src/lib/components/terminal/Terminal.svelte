<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Terminal as XTerm, type ITheme } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebLinksAddon } from "@xterm/addon-web-links";
  import "@xterm/xterm/css/xterm.css";
  import { connection } from "../../state/connection.svelte";

  type SessionStartInfo = { id: string; shell_id: string; shell_label: string };
  type Props = {
    visible?: boolean;
    shellId?: string | null;
    fontSize?: number;
    autoLaunch?: string;
    onSessionStart?: (info: SessionStartInfo) => void;
    onStatusChange?: (s: "idle" | "starting" | "running" | "exited" | "error", err?: string | null) => void;
  };
  let {
    visible = true,
    shellId = null,
    fontSize = 13,
    autoLaunch = "",
    onSessionStart,
    onStatusChange,
  }: Props = $props();

  type DataPayload = { id: string; chunk: string };
  type ExitPayload = { id: string };

  let host = $state<HTMLDivElement | undefined>();
  let term: XTerm | null = null;
  let fit: FitAddon | null = null;
  let sessionId: string | null = null;
  let unlistenData: UnlistenFn | null = null;
  let unlistenExit: UnlistenFn | null = null;
  let ro: ResizeObserver | null = null;
  let resizeTimer: ReturnType<typeof setTimeout> | null = null;
  let status = $state<"idle" | "starting" | "running" | "exited" | "error">("idle");
  let errorMsg = $state<string | null>(null);

  function readVar(name: string, fallback: string): string {
    if (typeof window === "undefined") return fallback;
    const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return v || fallback;
  }
  function buildTheme(): ITheme {
    return {
      background: readVar("--bg", "#0f1117"),
      foreground: readVar("--fg", "#f4f4f5"),
      cursor: readVar("--accent", "#a78bfa"),
      cursorAccent: readVar("--bg", "#0f1117"),
      selectionBackground: readVar("--accent-soft", "#a78bfa33"),
      black: "#1f2128",
      red: readVar("--danger", "#f87171"),
      green: "#86efac",
      yellow: readVar("--warn", "#fbbf24"),
      blue: readVar("--info", "#38bdf8"),
      magenta: readVar("--accent", "#a78bfa"),
      cyan: readVar("--info", "#38bdf8"),
      white: readVar("--fg", "#f4f4f5"),
      brightBlack: "#3f3f46",
      brightRed: readVar("--danger", "#f87171"),
      brightGreen: "#86efac",
      brightYellow: readVar("--warn", "#fbbf24"),
      brightBlue: readVar("--info", "#38bdf8"),
      brightMagenta: readVar("--accent", "#a78bfa"),
      brightCyan: readVar("--info", "#38bdf8"),
      brightWhite: "#ffffff",
    };
  }

  async function init() {
    if (!host || term) return;
    status = "starting";
    onStatusChange?.(status, null);
    errorMsg = null;
    term = new XTerm({
      fontFamily: 'JetBrains Mono Variable, "JetBrains Mono", Cascadia Code, Consolas, monospace',
      fontSize,
      lineHeight: 1.2,
      cursorBlink: true,
      cursorStyle: "bar",
      scrollback: 5000,
      allowProposedApi: true,
      theme: buildTheme(),
      convertEol: false,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.loadAddon(new WebLinksAddon());
    term.open(host);
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
    ro?.disconnect(); ro = null;
    if (unlistenData) { unlistenData(); unlistenData = null; }
    if (unlistenExit) { unlistenExit(); unlistenExit = null; }
    if (sessionId) {
      const id = sessionId;
      sessionId = null;
      try { await invoke("term_kill", { id }); } catch { /* already exited */ }
    }
    term?.dispose();
    term = null;
    fit = null;
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

  async function restart() {
    await teardown();
    if (visible) await init();
  }

  onMount(() => { if (visible) void init(); });
  onDestroy(() => { void teardown(); });
</script>

<div class="term-wrap" data-status={status}>
  <div class="term-host" bind:this={host}></div>
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
