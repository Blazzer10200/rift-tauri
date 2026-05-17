<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ServerProfile } from "../../state/connection.svelte";
  import { Cog, Server, Key, Info, Plus, Pencil, Trash2, RefreshCw, Sparkles, TerminalSquare, RotateCcw, ChevronDown, FolderOpen, Copy, Check, Eye, EyeOff, X, Volume2 } from "lucide-svelte";
  import { appConfigDir, appLogDir } from "@tauri-apps/api/path";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { updates } from "../../state/updates.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";
  import { rightPane } from "../../state/right-pane.svelte";
  import {
    terminal,
    TERM_FONT_SIZE_MIN,
    TERM_FONT_SIZE_MAX,
    TERM_SCROLLBACK_MIN,
    TERM_SCROLLBACK_MAX,
    type CursorStyle,
    type BellStyle,
    type FontFamilyPreset,
    type ThemePresetId,
  } from "../../state/terminal.svelte";
  import { THEME_PRESETS } from "../terminal/themePresets";

  type Section = "appearance" | "terminal" | "assistant" | "voice" | "servers" | "keys" | "about";

  type ShellInfo = { id: string; label: string; program: string; args: string[]; available: boolean };
  let shells = $state<ShellInfo[]>([]);

  // Native <select> renders OS-themed dropdowns that break the dark theme.
  // We mirror the Titlebar server-picker pattern w/ custom buttons + menu.
  let shellDdOpen = $state(false);
  let shellDdRef = $state<HTMLDivElement | undefined>();
  let fontDdOpen = $state(false);
  let fontDdRef = $state<HTMLDivElement | undefined>();

  function onDdDocMouseDown(e: MouseEvent) {
    const t = e.target as Node;
    if (shellDdOpen && shellDdRef && !shellDdRef.contains(t)) shellDdOpen = false;
    if (fontDdOpen && fontDdRef && !fontDdRef.contains(t)) fontDdOpen = false;
  }
  $effect(() => {
    if (!shellDdOpen && !fontDdOpen) return;
    document.addEventListener("mousedown", onDdDocMouseDown);
    return () => document.removeEventListener("mousedown", onDdDocMouseDown);
  });

  const FONT_FAMILY_OPTS: { id: FontFamilyPreset; label: string }[] = [
    { id: "default",  label: "JetBrains Mono (default)" },
    { id: "cascadia", label: "Cascadia Code" },
    { id: "consolas", label: "Consolas" },
    { id: "menlo",    label: "Menlo / Monaco" },
    { id: "custom",   label: "Custom…" },
  ];
  const CURSOR_OPTS: { id: CursorStyle; label: string }[] = [
    { id: "bar",       label: "Bar" },
    { id: "block",     label: "Block" },
    { id: "underline", label: "Underline" },
  ];
  const BELL_OPTS: { id: BellStyle; label: string }[] = [
    { id: "none",   label: "Off" },
    { id: "visual", label: "Visual" },
    { id: "sound",  label: "Sound" },
  ];

  let { initialSection = "terminal", onAddServer, onEditServer, onDeleteServer, onLaunchKeygen }: {
    initialSection?: Section;
    onAddServer: () => void;
    onEditServer: (s: ServerProfile) => void;
    onDeleteServer: (s: ServerProfile) => void;
    onLaunchKeygen: () => void;
  } = $props();

  let section = $state<Section>(untrack(() => initialSection));
  let appVersion = $state("?");
  let configDir = $state<string>("");
  let logDir = $state<string>("");
  let diagCopied = $state(false);

  async function loadAboutPaths() {
    try { configDir = await appConfigDir(); } catch (e) { console.warn("appConfigDir failed", e); }
    try { logDir = await appLogDir(); } catch (e) { console.warn("appLogDir failed", e); }
  }
  async function openDir(p: string) {
    if (!p) return;
    try { await openPath(p); } catch (e) { console.error("openPath failed", e); }
  }
  // Replace OS username segments in paths with <user> so a pasted
  // diagnostic doesn't leak the user's Windows/macOS login name.
  // Matches C:\Users\<name>\..., /home/<name>/..., /Users/<name>/...
  function scrubUser(p: string): string {
    if (!p) return p;
    return p
      .replace(/([A-Za-z]:[\\/]Users[\\/])[^\\/]+/g, "$1<user>")
      .replace(/(\/home\/)[^/]+/g, "$1<user>")
      .replace(/(\/Users\/)[^/]+/g, "$1<user>");
  }
  async function copyDiagnostic() {
    const lines = [
      `Rift ${appVersion}`,
      `Platform: ${navigator.platform}`,
      `Config: ${scrubUser(configDir) || "(unknown)"}`,
      `Logs:   ${scrubUser(logDir) || "(unknown)"}`,
      `Servers: ${connection.servers.length}`,
      `Active server: ${connection.selected ? "(configured)" : "(none)"} · state: ${connection.status?.state ?? "offline"}`,
    ].join("\n");
    try {
      await navigator.clipboard.writeText(lines);
      diagCopied = true;
      setTimeout(() => (diagCopied = false), 1400);
    } catch (e) { console.error("clipboard failed", e); }
  }

  // Appearance carries the v0.3 shell toggle now (and will pick up density /
  // font / accent later). Surfaced as the first nav item per convention.
  const sections: { id: Section; label: string; icon: typeof Cog }[] = [
    { id: "appearance", label: "Appearance", icon: Sparkles },
    { id: "terminal",   label: "Terminal",   icon: TerminalSquare },
    { id: "assistant",  label: "Assistant",  icon: Sparkles },
    { id: "voice",      label: "Voice",      icon: Volume2 },
    { id: "servers",    label: "Servers",    icon: Server },
    { id: "keys",       label: "SSH keys",   icon: Key },
    { id: "about",      label: "About",      icon: Info },
  ];

  // Assistant API-key field — value mirrors store, save commits to disk.
  import { assistant as assistantStore } from "../../state/assistant.svelte";
  import { tts } from "../../state/tts.svelte";
  import { stt } from "../../state/stt.svelte";
  const STT_LANGS: { id: string; label: string }[] = [
    { id: "en-US", label: "English (US)" },
    { id: "en-GB", label: "English (UK)" },
    { id: "en-AU", label: "English (Australia)" },
    { id: "es-ES", label: "Spanish (Spain)" },
    { id: "es-MX", label: "Spanish (Mexico)" },
    { id: "fr-FR", label: "French" },
    { id: "de-DE", label: "German" },
    { id: "it-IT", label: "Italian" },
    { id: "pt-BR", label: "Portuguese (Brazil)" },
    { id: "ja-JP", label: "Japanese" },
    { id: "ko-KR", label: "Korean" },
    { id: "zh-CN", label: "Chinese (Mandarin)" },
  ];
  let voiceDdOpen = $state(false);
  let voiceDdRef = $state<HTMLDivElement | undefined>();
  let voiceTestText = $state("This is a test of the current voice.");
  $effect(() => {
    if (section === "voice") {
      void tts.init();
      void tts.loadVoices();
      void stt.init();
    }
  });
  $effect(() => {
    if (!voiceDdOpen) return;
    const onDoc = (e: MouseEvent) => {
      const t = e.target as Node;
      if (voiceDdRef && !voiceDdRef.contains(t)) voiceDdOpen = false;
    };
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  });
  const currentVoiceLabel = $derived.by(() => {
    const sel = tts.config.voice;
    if (!sel) return "Aria (en-US, default)";
    const hit = tts.voices.find((v) => v.name === sel);
    if (!hit) return sel;
    const gender = hit.gender ? ` · ${hit.gender}` : "";
    return `${hit.short_name} (${hit.locale}${gender})`;
  });
  let asstApiKeyDraft = $state("");
  let asstApiKeySaving = $state(false);
  let asstApiKeyMsg = $state<string | null>(null);
  let asstApiKeyVisible = $state(false);
  let asstMaxBudgetDraft = $state<number | null>(null);
  let asstMaxBudgetSaving = $state(false);
  let asstMaxBudgetMsg = $state<string | null>(null);
  $effect(() => {
    if (section === "assistant") {
      void assistantStore.init();
      asstApiKeyDraft = assistantStore.apiKey ?? "";
      asstMaxBudgetDraft = assistantStore.maxBudgetUsd;
    }
  });
  async function saveAsstApiKey() {
    asstApiKeySaving = true;
    asstApiKeyMsg = null;
    try {
      await assistantStore.setApiKey(asstApiKeyDraft);
      asstApiKeyMsg = asstApiKeyDraft.trim() ? "Saved." : "Cleared.";
    } catch (e) {
      asstApiKeyMsg = `Failed: ${e}`;
    } finally {
      asstApiKeySaving = false;
    }
  }
  async function saveAsstMaxBudget() {
    asstMaxBudgetSaving = true;
    asstMaxBudgetMsg = null;
    try {
      await assistantStore.setMaxBudgetUsd(asstMaxBudgetDraft);
      asstMaxBudgetMsg = assistantStore.maxBudgetUsd != null ? `Saved: $${assistantStore.maxBudgetUsd.toFixed(2)} cap.` : "Cleared (no cap).";
    } catch (e) {
      asstMaxBudgetMsg = `Failed: ${e}`;
    } finally {
      asstMaxBudgetSaving = false;
    }
  }

  onMount(async () => {
    try { appVersion = await invoke<string>("app_version"); } catch {}
    void loadAboutPaths();
    await connection.loadServers();
    try { shells = await invoke<ShellInfo[]>("term_list_shells"); }
    catch (e) { console.warn("term_list_shells failed", e); }
  });

  async function pickServer(s: ServerProfile) {
    await connection.select(s.key);
  }
</script>

<div class="settings">
  <nav class="nav">
    {#each sections as s (s.id)}
      {@const Icon = s.icon}
      <button data-active={section === s.id} onclick={() => (section = s.id)} type="button">
        <Icon size={13}/><span>{s.label}</span>
      </button>
    {/each}
  </nav>

  <div class="body">
    {#key section}
      <div
        class="sub-shell"
        in:fly={{ y: 6, duration: 180, delay: 90, easing: quintOut }}
        out:fade={{ duration: 90 }}
      >
    {#if section === "appearance"}
      <div>
        <h3>Appearance</h3>
        <p class="help">Theme, density, font, and shell-layout controls.</p>

        <div class="v03-toggle card">
          <div class="v03-toggle-l">
            <span class="v03-title">
              <Sparkles size={14} class="v03-ico"/>
              Experimental v0.3 shell layout
              <span class="v03-beta">beta</span>
            </span>
            <span class="v03-hint">
              Single-canvas chat-first layout with a customizable right-side dock.
              Restart Rift after toggling — some components only read the flag at mount.
            </span>
          </div>
          <label class="v03-switch" title={uiPrefs.useV03Shell ? "Disable v0.3 shell" : "Enable v0.3 shell"}>
            <input
              type="checkbox"
              checked={uiPrefs.useV03Shell}
              onchange={(e) => uiPrefs.setUseV03Shell((e.currentTarget as HTMLInputElement).checked)}
            />
            <span class="v03-switch-track"></span>
          </label>
        </div>

        {#if uiPrefs.useV03Shell}
          <!-- v0.4.1 Layout sub-section: reset the right pane, close all chat
               tabs, kbd cheat sheet. The dock-accordion + split-dock-reset
               controls were retired alongside the dock primitive. -->
          <div class="v03-toggle card v03-sub layout-card">
            <div class="v03-toggle-l">
              <span class="v03-title v03-sub-title">Layout</span>
              <span class="v03-hint">
                v0.4.1 — chat lives on the left; pick a tool from the activity bar on the right.
                Drag activity-bar icons to reorder them — your <kbd class="kbd">Ctrl</kbd>+<kbd class="kbd">1</kbd>…<kbd class="kbd">7</kbd>
                shortcuts follow the bar's order.
              </span>
              <div class="layout-actions">
                <button class="btn ghost sm" type="button" onclick={() => rightPane.reset()}>
                  <RotateCcw size={11}/> Reset right pane
                </button>
                <button class="btn ghost sm" type="button" onclick={() => void assistantStore.closeAllTabs()}>
                  <X size={11}/> Close all chat tabs
                </button>
              </div>
              <div class="kbd-grid">
                <div class="kbd-row"><kbd class="kbd">Ctrl</kbd>+<kbd class="kbd">T</kbd><span>New chat tab</span></div>
                <div class="kbd-row"><kbd class="kbd">Ctrl</kbd>+<kbd class="kbd">W</kbd><span>Close active tab</span></div>
                <div class="kbd-row"><kbd class="kbd">Ctrl</kbd>+<kbd class="kbd">Tab</kbd><span>Cycle tabs (Shift to reverse)</span></div>
                <div class="kbd-row"><kbd class="kbd">Alt</kbd>+<kbd class="kbd">1</kbd>…<kbd class="kbd">9</kbd><span>Jump to tab N</span></div>
                <div class="kbd-row"><kbd class="kbd">Ctrl</kbd>+<kbd class="kbd">1</kbd>…<kbd class="kbd">7</kbd><span>Toggle right-pane page</span></div>
                <div class="kbd-row"><kbd class="kbd">Ctrl</kbd>+<kbd class="kbd">0</kbd><span>Close right pane</span></div>
              </div>
            </div>
          </div>
        {/if}

        <div class="soon card">
          <Sparkles size={18} class="soon-ico"/>
          <span class="soon-title">More coming soon</span>
          <span class="soon-hint">Density, font sizing, and accent-tint controls are planned for a later build. Dark mode stays the default.</span>
        </div>
      </div>

    {:else if section === "terminal"}
      <div class="term-set">
        <div class="set-head">
          <div>
            <h3>Terminal</h3>
            <p class="help">Font, cursor, palette, and shell defaults for the embedded terminal.</p>
          </div>
          <button class="btn ghost sm" onclick={() => terminal.resetAppearance()} type="button" title="Restore defaults">
            <RotateCcw size={11}/> Reset
          </button>
        </div>

        <div class="ts-group">
          <div class="ts-group-title">Shell</div>

          <div class="ts-row">
            <div class="ts-row-l">
              <label class="ts-label" for="ts-default-shell">Default shell</label>
              <span class="ts-hint">Used when opening a new tab without picking a preset.</span>
            </div>
            <div class="ts-dd" bind:this={shellDdRef} data-open={shellDdOpen}>
              <button
                type="button"
                id="ts-default-shell"
                class="ts-dd-btn"
                aria-haspopup="listbox"
                aria-expanded={shellDdOpen}
                onclick={() => (shellDdOpen = !shellDdOpen)}
              >
                <span class="ts-dd-value">
                  {shells.find((s) => s.id === terminal.defaultShellId)?.label ?? (shells.length === 0 ? "Detecting…" : "Pick a shell")}
                </span>
                <ChevronDown size={12}/>
              </button>
              {#if shellDdOpen}
                <div class="ts-dd-menu" role="listbox">
                  {#each shells as s (s.id)}
                    <button
                      type="button"
                      class="ts-dd-item"
                      role="option"
                      aria-selected={terminal.defaultShellId === s.id}
                      data-active={terminal.defaultShellId === s.id}
                      disabled={!s.available}
                      onclick={() => {
                        if (!s.available) return;
                        terminal.setDefaultShell(s.id);
                        shellDdOpen = false;
                      }}
                    >
                      <span class="ts-dd-item-l">
                        {#if terminal.defaultShellId === s.id}
                          <span class="ts-dd-dot" aria-hidden="true"></span>
                        {:else}
                          <span class="ts-dd-dot-spacer" aria-hidden="true"></span>
                        {/if}
                        <span>{s.label}</span>
                      </span>
                      {#if !s.available}
                        <span class="ts-dd-dim mono">missing</span>
                      {/if}
                    </button>
                  {/each}
                  {#if shells.length === 0}
                    <div class="ts-dd-empty">Detecting shells…</div>
                  {/if}
                </div>
              {/if}
            </div>
          </div>

          <div class="ts-row">
            <div class="ts-row-l">
              <label class="ts-label" for="ts-auto-launch">Auto-launch command</label>
              <span class="ts-hint">Runs in every new tab after the prompt appears. Leave blank to skip.</span>
            </div>
            <input
              id="ts-auto-launch"
              class="input mono ts-input"
              type="text"
              placeholder="e.g. claude"
              value={terminal.autoLaunchCommand}
              oninput={(e) => terminal.setAutoLaunchCommand((e.currentTarget as HTMLInputElement).value)}
            />
          </div>
        </div>

        <div class="ts-group">
          <div class="ts-group-title">Typography</div>

          <div class="ts-row">
            <div class="ts-row-l">
              <label class="ts-label" for="ts-font-size">Font size</label>
              <span class="ts-hint">Between {TERM_FONT_SIZE_MIN} and {TERM_FONT_SIZE_MAX}px.</span>
            </div>
            <div class="ts-slider">
              <input
                id="ts-font-size"
                type="range"
                min={TERM_FONT_SIZE_MIN}
                max={TERM_FONT_SIZE_MAX}
                step="1"
                value={terminal.fontSize}
                oninput={(e) => terminal.setFontSize(parseInt((e.currentTarget as HTMLInputElement).value, 10))}
              />
              <span class="ts-slider-val mono">{terminal.fontSize}px</span>
            </div>
          </div>

          <div class="ts-row">
            <div class="ts-row-l">
              <label class="ts-label" for="ts-font-family">Font family</label>
              <span class="ts-hint">Monospace stacks fall back if the first family is missing.</span>
            </div>
            <div class="ts-dd" bind:this={fontDdRef} data-open={fontDdOpen}>
              <button
                type="button"
                id="ts-font-family"
                class="ts-dd-btn"
                aria-haspopup="listbox"
                aria-expanded={fontDdOpen}
                onclick={() => (fontDdOpen = !fontDdOpen)}
              >
                <span class="ts-dd-value">
                  {FONT_FAMILY_OPTS.find((f) => f.id === terminal.fontFamilyPreset)?.label ?? "Default"}
                </span>
                <ChevronDown size={12}/>
              </button>
              {#if fontDdOpen}
                <div class="ts-dd-menu" role="listbox">
                  {#each FONT_FAMILY_OPTS as f (f.id)}
                    <button
                      type="button"
                      class="ts-dd-item"
                      role="option"
                      aria-selected={terminal.fontFamilyPreset === f.id}
                      data-active={terminal.fontFamilyPreset === f.id}
                      onclick={() => {
                        terminal.setFontFamilyPreset(f.id);
                        fontDdOpen = false;
                      }}
                    >
                      <span class="ts-dd-item-l">
                        {#if terminal.fontFamilyPreset === f.id}
                          <span class="ts-dd-dot" aria-hidden="true"></span>
                        {:else}
                          <span class="ts-dd-dot-spacer" aria-hidden="true"></span>
                        {/if}
                        <span>{f.label}</span>
                      </span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          </div>

          {#if terminal.fontFamilyPreset === "custom"}
            <div class="ts-row">
              <div class="ts-row-l">
                <label class="ts-label" for="ts-font-custom">Custom font stack</label>
                <span class="ts-hint">CSS font-family value. Must be monospace.</span>
              </div>
              <input
                id="ts-font-custom"
                class="input mono ts-input"
                type="text"
                placeholder={'"Fira Code", monospace'}
                value={terminal.fontFamilyCustom}
                oninput={(e) => terminal.setFontFamilyCustom((e.currentTarget as HTMLInputElement).value)}
              />
            </div>
          {/if}
        </div>

        <div class="ts-group">
          <div class="ts-group-title">Cursor</div>

          <div class="ts-row">
            <div class="ts-row-l">
              <span class="ts-label">Style</span>
              <span class="ts-hint">Shape used when the terminal has focus.</span>
            </div>
            <div class="seg" role="radiogroup" aria-label="Cursor style">
              {#each CURSOR_OPTS as c (c.id)}
                <button
                  type="button"
                  role="radio"
                  aria-checked={terminal.cursorStyle === c.id}
                  data-active={terminal.cursorStyle === c.id}
                  onclick={() => terminal.setCursorStyle(c.id)}
                >{c.label}</button>
              {/each}
            </div>
          </div>

          <div class="ts-row">
            <div class="ts-row-l">
              <span class="ts-label">Blink</span>
              <span class="ts-hint">Animate the cursor while focused.</span>
            </div>
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Toggle"
              aria-checked={terminal.cursorBlink}
              data-on={terminal.cursorBlink}
              onclick={() => terminal.setCursorBlink(!terminal.cursorBlink)}
            ><span class="switch-knob"></span></button>
          </div>
        </div>

        <div class="ts-group">
          <div class="ts-group-title">Behavior</div>

          <div class="ts-row">
            <div class="ts-row-l">
              <label class="ts-label" for="ts-scrollback">Scrollback</label>
              <span class="ts-hint">Lines retained in history per tab ({TERM_SCROLLBACK_MIN.toLocaleString()}–{TERM_SCROLLBACK_MAX.toLocaleString()}).</span>
            </div>
            <input
              id="ts-scrollback"
              class="input mono ts-input ts-input-narrow"
              type="number"
              min={TERM_SCROLLBACK_MIN}
              max={TERM_SCROLLBACK_MAX}
              step="500"
              value={terminal.scrollback}
              onchange={(e) => terminal.setScrollback(parseInt((e.currentTarget as HTMLInputElement).value, 10))}
            />
          </div>

          <div class="ts-row">
            <div class="ts-row-l">
              <span class="ts-label">Bell</span>
              <span class="ts-hint">What happens when a program rings the bell.</span>
            </div>
            <div class="seg" role="radiogroup" aria-label="Bell style">
              {#each BELL_OPTS as b (b.id)}
                <button
                  type="button"
                  role="radio"
                  aria-checked={terminal.bellStyle === b.id}
                  data-active={terminal.bellStyle === b.id}
                  onclick={() => terminal.setBellStyle(b.id)}
                >{b.label}</button>
              {/each}
            </div>
          </div>

          <div class="ts-row">
            <div class="ts-row-l">
              <span class="ts-label">Copy on select</span>
              <span class="ts-hint">Selection auto-copies to clipboard.</span>
            </div>
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Toggle"
              aria-checked={terminal.copyOnSelect}
              data-on={terminal.copyOnSelect}
              onclick={() => terminal.setCopyOnSelect(!terminal.copyOnSelect)}
            ><span class="switch-knob"></span></button>
          </div>

          <div class="ts-row">
            <div class="ts-row-l">
              <span class="ts-label">Right-click paste</span>
              <span class="ts-hint">Right-click in the terminal pastes clipboard contents.</span>
            </div>
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Toggle"
              aria-checked={terminal.rightClickPaste}
              data-on={terminal.rightClickPaste}
              onclick={() => terminal.setRightClickPaste(!terminal.rightClickPaste)}
            ><span class="switch-knob"></span></button>
          </div>
        </div>

        <div class="ts-group">
          <div class="ts-group-title">Theme</div>

          <div class="ts-theme-grid">
            {#each THEME_PRESETS as t (t.id)}
              <button
                type="button"
                class="ts-theme-card"
                data-active={terminal.themePreset === t.id}
                onclick={() => terminal.setThemePreset(t.id as ThemePresetId)}
              >
                <span class="ts-theme-swatch" data-preset={t.id}></span>
                <span class="ts-theme-text">
                  <span class="ts-theme-label">{t.label}</span>
                  <span class="ts-theme-sub">{t.subtitle}</span>
                </span>
              </button>
            {/each}
          </div>
        </div>
      </div>

    {:else if section === "assistant"}
      <div>
        <h3>Assistant</h3>
        <p class="help">Authentication for the Assistant page. By default Rift piggybacks on your local <code>claude</code> CLI session — no key needed.</p>

        <div class="card asst-card">
          <div class="asst-row">
            <span class="asst-label">CLI status</span>
            {#if assistantStore.auth}
              <span class="asst-pill" data-tone={assistantStore.auth.pill}>
                <span class="asst-dot"></span>{assistantStore.auth.summary}
              </span>
            {:else if assistantStore.authChecking}
              <span class="asst-pill" data-tone="neutral"><span class="asst-dot"></span>Checking…</span>
            {:else}
              <span class="asst-pill" data-tone="neutral"><span class="asst-dot"></span>Unknown — open Assistant to probe</span>
            {/if}
            <button class="btn ghost sm" type="button" onclick={() => assistantStore.refreshAuth()} disabled={assistantStore.authChecking}>
              <RefreshCw size={11}/> Re-probe
            </button>
          </div>
          <p class="muted">Not signed in? In a terminal run <code>claude login</code>, then re-probe.</p>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Use my full Claude Code config</h4>
          <p class="muted">
            Layers your <code>~/.claude/CLAUDE.md</code>, slash commands, skills, and MCP servers
            into every Assistant turn alongside Rift's own MCP tools. Off = sandboxed mode (Rift
            MCP only, no user slash commands).
          </p>
          <div class="asst-row">
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Use full Claude Code config"
              aria-checked={assistantStore.useFullConfig && !assistantStore.apiKey}
              data-on={assistantStore.useFullConfig && !assistantStore.apiKey}
              disabled={!!assistantStore.apiKey}
              onclick={() => void assistantStore.setUseFullConfig(!assistantStore.useFullConfig)}
            ><span class="switch-knob"></span></button>
            <span class="muted">
              {#if assistantStore.apiKey}
                Force-off while API-key mode is active — <code>--bare</code> suppresses user config.
              {:else if assistantStore.useFullConfig}
                On — your CLAUDE.md, hooks, skills, slash commands, and MCPs are live.
              {:else}
                Off — sandboxed. Only Rift's MCP + the built-in tool set.
              {/if}
            </span>
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Per-turn cost cap</h4>
          <p class="muted">
            Passes <code>--max-budget-usd</code> to the CLI. If a turn would exceed this dollar
            amount, the CLI exits with an error and Rift surfaces it in the chat. Leave blank
            (or set 0) for no cap.
          </p>
          <div class="asst-row">
            <input
              class="input mono asst-input"
              type="number"
              min="0"
              step="0.01"
              placeholder="5.00"
              bind:value={asstMaxBudgetDraft}
            />
            <button class="btn primary sm" type="button" onclick={saveAsstMaxBudget} disabled={asstMaxBudgetSaving}>
              {asstMaxBudgetSaving ? "Saving…" : "Save"}
            </button>
            {#if assistantStore.maxBudgetUsd != null}
              <button class="btn ghost sm" type="button" onclick={() => { asstMaxBudgetDraft = null; void saveAsstMaxBudget(); }}>
                Clear
              </button>
            {/if}
          </div>
          {#if asstMaxBudgetMsg}<p class="muted">{asstMaxBudgetMsg}</p>{/if}
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Allow remote shell</h4>
          <p class="muted">
            Exposes <code>mcp__rift__remote_bash</code> to the model — runs commands on the
            connected SSH server via the auto-sync engine's russh session. A workspace-scoped
            advisory lock serializes calls between users. Off by default; flip on for ops
            work (pm2, server status, log inspection).
          </p>
          <div class="asst-row">
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Allow remote shell"
              aria-checked={assistantStore.allowRemoteShell}
              data-on={assistantStore.allowRemoteShell}
              onclick={() => void assistantStore.setAllowRemoteShell(!assistantStore.allowRemoteShell)}
            ><span class="switch-knob"></span></button>
            <span class="muted">
              {#if assistantStore.allowRemoteShell}
                On — Claude can run shell commands against the connected remote.
              {:else}
                Off — remote_bash tool is hidden from the model.
              {/if}
            </span>
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">API-key fallback</h4>
          <p class="muted">Pay-per-token via <code>console.anthropic.com</code>. Used when configured; overrides the CLI session.</p>
          <div class="asst-row">
            <div class="asst-input-wrap">
              <input
                class="input mono asst-input"
                type={asstApiKeyVisible ? "text" : "password"}
                placeholder="sk-ant-api03-…"
                bind:value={asstApiKeyDraft}
                autocomplete="off"
                spellcheck="false"
              />
              <button
                class="asst-eye"
                type="button"
                onclick={() => (asstApiKeyVisible = !asstApiKeyVisible)}
                aria-label={asstApiKeyVisible ? "Hide API key" : "Show API key"}
                title={asstApiKeyVisible ? "Hide API key" : "Show API key"}
              >
                {#if asstApiKeyVisible}<EyeOff size={14}/>{:else}<Eye size={14}/>{/if}
              </button>
            </div>
            <button class="btn primary sm" type="button" onclick={saveAsstApiKey} disabled={asstApiKeySaving}>
              {asstApiKeySaving ? "Saving…" : "Save"}
            </button>
            {#if assistantStore.apiKey}
              <button class="btn ghost sm" type="button" onclick={() => { asstApiKeyDraft = ""; void saveAsstApiKey(); }}>
                Clear
              </button>
            {/if}
          </div>
          {#if asstApiKeyMsg}<p class="muted">{asstApiKeyMsg}</p>{/if}
          <p class="muted asst-warn">Stored in <code>~/.rift/assistant/config.json</code> as plaintext. Keychain migration planned.</p>
        </div>
      </div>

    {:else if section === "voice"}
      <div>
        <h3>Voice</h3>
        <p class="help">
          Speak assistant replies aloud using Microsoft Edge's Azure Neural voices —
          free, no API key, no model download. Toggle <em>Auto-speak</em> in the
          Assistant header to enable streaming sentence-by-sentence playback.
        </p>

        <div class="card asst-card">
          <h4 class="asst-h4">Enable text-to-speech</h4>
          <p class="muted">Master switch. Off = the speaker button + per-message replay are inert.</p>
          <div class="asst-row">
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Enable text-to-speech"
              aria-checked={tts.config.enabled}
              data-on={tts.config.enabled}
              onclick={() => void tts.setConfig({ enabled: !tts.config.enabled })}
            ><span class="switch-knob"></span></button>
            <span class="muted">
              {#if tts.config.enabled}On — Rift can synthesize and play audio.{:else}Off — synthesis disabled.{/if}
            </span>
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Auto-speak streaming replies</h4>
          <p class="muted">When on, every completed sentence is spoken as the model streams it. Off = manual replay only.</p>
          <div class="asst-row">
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Auto-speak streaming replies"
              aria-checked={tts.config.auto_speak}
              data-on={tts.config.auto_speak}
              disabled={!tts.config.enabled}
              onclick={() => void tts.setConfig({ auto_speak: !tts.config.auto_speak })}
            ><span class="switch-knob"></span></button>
            <span class="muted">
              {#if !tts.config.enabled}Enable TTS first.{:else if tts.config.auto_speak}On.{:else}Off — use the speaker icon on each message to replay.{/if}
            </span>
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Voice</h4>
          <p class="muted">~500 Edge Neural voices — English locales listed first.</p>
          <div class="voice-dd-wrap" bind:this={voiceDdRef}>
            <button
              type="button"
              class="voice-dd-btn"
              onclick={() => (voiceDdOpen = !voiceDdOpen)}
              aria-haspopup="listbox"
              aria-expanded={voiceDdOpen}
            >
              <span class="voice-dd-label">{currentVoiceLabel}</span>
              <ChevronDown size={13}/>
            </button>
            {#if voiceDdOpen}
              <div class="voice-dd-menu" role="listbox">
                {#if tts.voicesLoading}
                  <div class="voice-dd-empty">Loading voices…</div>
                {:else if tts.voicesError}
                  <div class="voice-dd-empty err">Failed: {tts.voicesError}</div>
                {:else if tts.voices.length === 0}
                  <div class="voice-dd-empty">No voices available.</div>
                {:else}
                  <button
                    type="button"
                    class="voice-dd-item"
                    class:active={!tts.config.voice}
                    onclick={() => { void tts.setConfig({ voice: "" }); voiceDdOpen = false; }}
                  >
                    <span class="voice-item-name">Aria <span class="voice-item-meta">(en-US, default)</span></span>
                  </button>
                  {#each tts.voices as v (v.name)}
                    <button
                      type="button"
                      class="voice-dd-item"
                      class:active={tts.config.voice === v.name}
                      onclick={() => { void tts.setConfig({ voice: v.name }); voiceDdOpen = false; }}
                    >
                      <span class="voice-item-name">
                        {v.short_name}
                        <span class="voice-item-meta">
                          ({v.locale}{v.gender ? ` · ${v.gender}` : ""})
                        </span>
                      </span>
                    </button>
                  {/each}
                {/if}
              </div>
            {/if}
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Rate, pitch, volume</h4>
          <p class="muted">Range -100 to +100, each percent of the natural baseline. 0 = unchanged.</p>
          <div class="voice-slider-row">
            <label class="voice-slider-label" for="voice-rate">Rate</label>
            <input
              id="voice-rate"
              type="range" min="-50" max="50" step="5"
              value={tts.config.rate}
              oninput={(e) => void tts.setConfig({ rate: Number((e.target as HTMLInputElement).value) })}
              class="voice-slider"
            />
            <span class="voice-slider-val mono">{tts.config.rate}%</span>
          </div>
          <div class="voice-slider-row">
            <label class="voice-slider-label" for="voice-pitch">Pitch</label>
            <input
              id="voice-pitch"
              type="range" min="-50" max="50" step="5"
              value={tts.config.pitch}
              oninput={(e) => void tts.setConfig({ pitch: Number((e.target as HTMLInputElement).value) })}
              class="voice-slider"
            />
            <span class="voice-slider-val mono">{tts.config.pitch}%</span>
          </div>
          <div class="voice-slider-row">
            <label class="voice-slider-label" for="voice-volume">Volume</label>
            <input
              id="voice-volume"
              type="range" min="-50" max="50" step="5"
              value={tts.config.volume}
              oninput={(e) => void tts.setConfig({ volume: Number((e.target as HTMLInputElement).value) })}
              class="voice-slider"
            />
            <span class="voice-slider-val mono">{tts.config.volume}%</span>
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Test voice</h4>
          <p class="muted">Synthesises and plays the text below using the current settings.</p>
          <div class="asst-row">
            <input
              class="input asst-input"
              type="text"
              bind:value={voiceTestText}
              placeholder="Type something to speak…"
            />
            <button class="btn primary sm" type="button" onclick={() => void tts.testVoice(voiceTestText)}>
              <Volume2 size={11}/> Speak
            </button>
            {#if tts.playing}
              <button class="btn ghost sm" type="button" onclick={() => void tts.cancel()}>Stop</button>
            {/if}
          </div>
          {#if tts.lastError}<p class="muted asst-warn">{tts.lastError}</p>{/if}
        </div>

        <h3 class="set-subhead">Speech-to-text (dictation)</h3>
        <p class="help">
          Talk into the mic and your words stream into the composer. Uses the
          WebView's built-in speech recognition (Microsoft Azure when online) —
          nothing to install or download.
          {#if !stt.supported}
            <span class="asst-warn"> · Your WebView does not expose <code>SpeechRecognition</code>; STT is unavailable.</span>
          {/if}
        </p>

        <div class="card asst-card">
          <h4 class="asst-h4">Enable speech-to-text</h4>
          <p class="muted">Master switch. When off, the mic button in the composer is hidden / inert.</p>
          <div class="asst-row">
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Enable speech-to-text"
              aria-checked={stt.config.enabled}
              data-on={stt.config.enabled}
              disabled={!stt.supported}
              onclick={() => void stt.setConfig({ enabled: !stt.config.enabled })}
            ><span class="switch-knob"></span></button>
            <span class="muted">
              {#if stt.config.enabled}On — mic button in the composer is live.{:else}Off.{/if}
            </span>
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Language</h4>
          <p class="muted">BCP-47 tag passed to the recogniser. Use English (US) unless you're speaking another language.</p>
          <div class="stt-lang-grid">
            {#each STT_LANGS as l (l.id)}
              <button
                type="button"
                class="stt-lang-pick"
                data-active={stt.config.language === l.id}
                onclick={() => void stt.setConfig({ language: l.id })}
              >
                <span class="stt-lang-label">{l.label}</span>
                <span class="stt-lang-code mono">{l.id}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Live partial transcripts</h4>
          <p class="muted">Words appear in the composer as you speak. Off = wait for each sentence to commit before any text shows.</p>
          <div class="asst-row">
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Live partial transcripts"
              aria-checked={stt.config.show_interim}
              data-on={stt.config.show_interim}
              disabled={!stt.config.enabled}
              onclick={() => void stt.setConfig({ show_interim: !stt.config.show_interim })}
            ><span class="switch-knob"></span></button>
            <span class="muted">
              {#if stt.config.show_interim}On — interim words appear live.{:else}Off — only final committed text appears.{/if}
            </span>
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Continuous mode</h4>
          <p class="muted">Keep listening across pauses until you click stop. Off = recogniser stops after the first sentence.</p>
          <div class="asst-row">
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Continuous mode"
              aria-checked={stt.config.continuous}
              data-on={stt.config.continuous}
              disabled={!stt.config.enabled}
              onclick={() => void stt.setConfig({ continuous: !stt.config.continuous })}
            ><span class="switch-knob"></span></button>
            <span class="muted">
              {#if stt.config.continuous}On — recogniser keeps listening until you stop.{:else}Off — stops after one sentence.{/if}
            </span>
          </div>
        </div>

        <div class="card asst-card">
          <h4 class="asst-h4">Insertion mode</h4>
          <p class="muted">Transcript can append to the existing draft (preserves what's typed) or replace it (mic-first workflow).</p>
          <div class="asst-row">
            <button
              type="button"
              class="switch"
              role="switch"
              aria-label="Append transcript to existing draft"
              aria-checked={stt.config.append_to_draft}
              data-on={stt.config.append_to_draft}
              disabled={!stt.config.enabled}
              onclick={() => void stt.setConfig({ append_to_draft: !stt.config.append_to_draft })}
            ><span class="switch-knob"></span></button>
            <span class="muted">
              {#if stt.config.append_to_draft}Append — transcript adds to the composer.{:else}Replace — transcript overwrites the composer.{/if}
            </span>
          </div>
        </div>
        {#if stt.lastError}<p class="muted asst-warn">{stt.lastError}</p>{/if}
      </div>

    {:else if section === "servers"}
      <div>
        <div class="set-head">
          <div>
            <h3>Servers</h3>
            <p class="help">{connection.servers.length} configured · profiles, keys, and bridge ports.</p>
          </div>
          <button class="btn primary sm" onclick={onAddServer} type="button">
            <Plus size={11}/> Add server
          </button>
        </div>
        {#if connection.servers.length === 0}
          <div class="empty card">
            <span class="empty-title">No servers yet</span>
            <span class="empty-hint">Click <strong>Add server</strong> to create one.</span>
          </div>
        {:else}
          <div class="srv-list">
            {#each connection.servers as s (s.key)}
              <div
                class="srv-card"
                data-active={connection.selectedKey === s.key}
                role="button"
                tabindex="0"
                onclick={() => pickServer(s)}
                onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); pickServer(s); } }}
              >
                <div class="srv-l">
                  <span class="srv-dot lg"></span>
                  <div>
                    <div class="mono srv-name">{s.name}</div>
                    <div class="mono dim">{s.user}@{s.host}{s.port !== 22 ? `:${s.port}` : ""}</div>
                  </div>
                </div>
                <div class="srv-meta mono dim" title={s.fingerprint ?? "no fingerprint pinned"}>
                  {s.fingerprint ? `ed25519 · ${s.fingerprint.slice(0, 18)}…` : "no fingerprint pinned"}
                </div>
                <div class="srv-r">
                  <button class="btn ghost sm" onclick={(e) => { e.stopPropagation(); onEditServer(s); }} type="button" title="Edit" aria-label="Edit">
                    <Pencil size={11}/>
                  </button>
                  <button class="btn ghost sm" onclick={(e) => { e.stopPropagation(); onDeleteServer(s); }} type="button" title="Delete" aria-label="Delete">
                    <Trash2 size={11}/>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

    {:else if section === "keys"}
      <div>
        <div class="set-head">
          <div>
            <h3>SSH keys</h3>
            <p class="help">ed25519 keypair stored at <span class="mono">%APPDATA%/Rift/keys/</span>. Generate or copy your public key from the setup dialog.</p>
          </div>
          <button class="btn primary sm" onclick={onLaunchKeygen} type="button">
            <Key size={11}/> Open key setup
          </button>
        </div>
      </div>

    {:else if section === "about"}
      <div>
        <h3>About</h3>
        <div class="set-row"><span>Rift</span><span class="mono">{appVersion} · Tauri 2</span></div>
        <div class="set-row"><span>Engine</span><span class="mono">SvelteKit + Svelte 5 (runes)</span></div>
        <div class="set-row"><span>Style</span><span class="mono">Tailwind v4 · OKLCH tokens · Linear-precise</span></div>
        <div class="set-row"><span>SSH</span><span class="mono">russh + russh-sftp · ring backend</span></div>
        <div class="set-row"><span>License</span><span class="mono">MIT · github.com/Blazzer10200/rift</span></div>
        <div class="set-row">
          <span>Updates</span>
          <button class="btn ghost sm" type="button" onclick={() => updates.open()}>
            <RefreshCw size={11}/> Check for updates
          </button>
        </div>

        <h3 style="margin-top:24px">Paths</h3>
        <div class="set-row">
          <span>Config</span>
          <span class="path-cell">
            <code class="mono path-val" title={configDir}>{configDir || "—"}</code>
            <button class="btn ghost sm" type="button" disabled={!configDir} onclick={() => openDir(configDir)}>
              <FolderOpen size={11}/> Open
            </button>
          </span>
        </div>
        <div class="set-row">
          <span>Logs</span>
          <span class="path-cell">
            <code class="mono path-val" title={logDir}>{logDir || "—"}</code>
            <button class="btn ghost sm" type="button" disabled={!logDir} onclick={() => openDir(logDir)}>
              <FolderOpen size={11}/> Open
            </button>
          </span>
        </div>

        <h3 style="margin-top:24px">Diagnostics</h3>
        <div class="set-row">
          <span>Support info</span>
          <button class="btn ghost sm" type="button" onclick={copyDiagnostic}>
            {#if diagCopied}<Check size={11}/> Copied{:else}<Copy size={11}/> Copy diagnostic info{/if}
          </button>
        </div>
      </div>
    {/if}
      </div>
    {/key}
  </div>
</div>

<style>
  .settings {
    display: grid;
    grid-template-columns: 200px 1fr;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .nav {
    display: flex; flex-direction: column; gap: 1px;
    padding: 12px 8px;
    background: var(--bg);
    border-right: 1px solid var(--border);
    overflow: auto;
  }
  .nav button {
    display: flex; align-items: center; gap: 8px;
    width: 100%; height: 28px; padding: 0 10px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    text-align: left; font: inherit; font-size: var(--fs-sm);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }
  .nav button:hover { background: var(--surface-hover); color: var(--fg); }
  .nav button[data-active="true"] {
    background: var(--surface); color: var(--fg);
    box-shadow: inset 2px 0 var(--accent);
  }

  .body {
    position: relative;
    min-width: 0;
    overflow: hidden;
  }
  .sub-shell {
    position: absolute;
    inset: 0;
    padding: 22px;
    overflow: auto;
  }
  h3 { margin: 0 0 6px; font-size: var(--fs-lg); font-weight: 600; color: var(--fg); }

  .set-head { display: flex; align-items: flex-end; justify-content: space-between; margin-bottom: 14px; gap: 12px; }

  .soon {
    margin-top: 14px;
    padding: 32px 24px;
    display: flex; flex-direction: column; gap: 8px;
    align-items: center; text-align: center;
    background: color-mix(in oklch, var(--accent) 6%, var(--surface));
    border: 1px dashed color-mix(in oklch, var(--accent) 35%, var(--border));
    border-radius: var(--radius);
  }
  .soon :global(.soon-ico) { color: var(--accent); }
  .soon-title { color: var(--fg); font-size: var(--fs-sm); font-weight: 600; }
  .soon-hint { color: var(--fg-muted); font-size: var(--fs-xs); max-width: 360px; line-height: 1.5; }

  /* v0.3 experimental shell toggle — sits above the more-coming-soon card.
     Single switch row; restart required so we leave it visually deliberate
     (warn-toned border) but not alarming. */
  .v03-toggle {
    margin-top: 14px;
    padding: 14px 16px;
    display: flex; align-items: center; justify-content: space-between; gap: 16px;
    border: 1px solid color-mix(in oklch, var(--warn) 30%, var(--border));
    background: color-mix(in oklch, var(--warn) 4%, var(--surface));
    border-radius: var(--radius);
  }
  .v03-toggle-l { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .v03-title {
    display: inline-flex; align-items: center; gap: 8px;
    color: var(--fg); font-size: var(--fs-sm); font-weight: 600;
  }
  .v03-toggle :global(.v03-ico) { color: var(--warn); }
  .v03-beta {
    padding: 1px 6px;
    font-size: 9px; font-weight: 700;
    background: var(--warn-soft); color: var(--warn);
    border-radius: 4px;
    letter-spacing: 0.06em; text-transform: uppercase;
  }
  .v03-hint { color: var(--fg-muted); font-size: var(--fs-xs); line-height: 1.5; max-width: 540px; }
  .v03-switch { position: relative; display: inline-block; width: 38px; height: 22px; cursor: pointer; flex-shrink: 0; }
  .v03-switch input { position: absolute; opacity: 0; inset: 0; cursor: pointer; }
  .v03-switch-track {
    position: absolute; inset: 0;
    background: var(--bg-elev-3);
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .v03-switch-track::before {
    content: "";
    position: absolute; top: 2px; left: 2px;
    width: 16px; height: 16px;
    background: var(--fg-muted);
    border-radius: 50%;
    transition: transform 160ms cubic-bezier(0.22, 1, 0.36, 1), background 120ms ease;
  }
  .v03-switch input:checked + .v03-switch-track {
    background: color-mix(in oklch, var(--accent) 70%, transparent);
    border-color: var(--accent);
  }
  .v03-switch input:checked + .v03-switch-track::before {
    transform: translateX(16px);
    background: var(--accent-fg);
  }
  .v03-switch input:focus-visible + .v03-switch-track { box-shadow: 0 0 0 2px var(--ring); }

  /* Sub-toggle (accordion) — same shape as parent but neutral tone since
     it's only visible when v0.3 is on (not experimental in its own right). */
  .v03-toggle.v03-sub {
    margin-top: 8px;
    border-color: var(--border);
    background: var(--surface);
  }
  .v03-sub-title { font-weight: 500; }
  .v03-hint .kbd,
  .kbd-row .kbd {
    display: inline-block;
    margin: 0 1px;
    padding: 0 4px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 3px;
    font: inherit;
    font-size: 10px;
    color: var(--fg-muted);
  }

  .layout-card .layout-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 8px;
  }
  .layout-card .kbd-grid {
    margin-top: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
    display: grid;
    grid-template-columns: 1fr;
    gap: 5px;
    font-size: 11px;
    color: var(--fg-muted);
  }
  .layout-card .kbd-row {
    display: grid;
    grid-template-columns: 130px 1fr;
    align-items: center;
    gap: 8px;
  }
  .layout-card .kbd-row span { color: var(--fg-muted); }

  .set-row {
    display: flex; align-items: center; justify-content: space-between;
    padding: 8px 4px;
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-sm);
    gap: 12px;
  }
  .set-row:last-child { border-bottom: none; }
  .path-cell {
    display: inline-flex; align-items: center; gap: 8px;
    min-width: 0; max-width: 70%;
  }
  .path-val {
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    background: var(--bg-elev-1);
    padding: 2px 6px;
    border-radius: var(--radius-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    min-width: 0; max-width: 360px;
  }

  .empty {
    padding: 28px 16px;
    display: flex; flex-direction: column; gap: 4px;
    align-items: center; text-align: center;
  }
  .empty-title { color: var(--fg); font-size: var(--fs-sm); font-weight: 600; }
  .empty-hint { color: var(--fg-muted); font-size: var(--fs-xs); max-width: 280px; line-height: 1.45; }

  .srv-list { display: flex; flex-direction: column; gap: 6px; }
  .srv-card {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: 16px;
    padding: 12px 14px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    color: var(--fg);
    font: inherit;
    text-align: left;
    box-shadow: inset 2px 0 transparent;
    transition: background 100ms, border-color 100ms, box-shadow 100ms;
  }
  .srv-card:hover { background: var(--surface-hover); }
  .srv-card[data-active="true"] {
    background: color-mix(in oklch, var(--accent) 10%, var(--surface));
    border-color: color-mix(in oklch, var(--accent) 45%, var(--border));
    box-shadow: inset 2px 0 var(--accent);
  }
  .srv-card[data-active="true"]:hover {
    background: color-mix(in oklch, var(--accent) 14%, var(--surface));
  }
  .srv-l { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .srv-dot {
    width: 10px; height: 10px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 18%, transparent);
  }
  .srv-name { font-weight: 600; font-size: var(--fs-sm); }
  .srv-meta {
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    max-width: 220px;
  }
  .srv-r { display: flex; gap: 4px; }

  /* ─── Terminal section ─── */
  .term-set { display: flex; flex-direction: column; gap: 18px; }
  .ts-group {
    display: flex; flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    /* No overflow:hidden — dropdown menus inside need to escape the card. */
  }
  .ts-group-title {
    padding: 10px 14px 8px;
    background: var(--bg-elev-1);
    border-bottom: 1px solid var(--border);
    border-radius: calc(var(--radius) - 1px) calc(var(--radius) - 1px) 0 0;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-subtle);
  }
  .ts-row {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 16px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
  }
  .ts-row:last-child { border-bottom: none; }
  .ts-row-l { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .ts-label { font-size: var(--fs-sm); font-weight: 500; color: var(--fg); }
  .ts-hint  { font-size: var(--fs-xs); color: var(--fg-subtle); line-height: 1.4; }
  .ts-input { width: 260px; }
  .ts-input-narrow { width: 120px; text-align: right; }
  /* hide native number spinners — keep value-only display, integers via step */
  .ts-input-narrow::-webkit-outer-spin-button,
  .ts-input-narrow::-webkit-inner-spin-button { -webkit-appearance: none; margin: 0; }
  .ts-input-narrow { appearance: textfield; -moz-appearance: textfield; }

  /* custom dropdown — mirrors the Titlebar server picker */
  .ts-dd { position: relative; width: 240px; }
  .ts-dd-btn {
    display: inline-flex; align-items: center; justify-content: space-between;
    width: 100%; height: 28px; padding: 0 10px;
    gap: 8px;
    background: var(--bg-elev-1);
    color: var(--fg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    cursor: pointer;
    font: inherit; font-size: var(--fs-sm);
    text-align: left;
    transition: background 100ms, border-color 100ms;
  }
  .ts-dd-btn:hover {
    background: var(--surface-hover);
    border-color: color-mix(in oklch, var(--accent) 30%, var(--border-strong));
  }
  .ts-dd-btn:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .ts-dd[data-open="true"] .ts-dd-btn {
    border-color: var(--border-focus);
    box-shadow: 0 0 0 2px var(--ring);
  }
  .ts-dd-btn :global(svg) { color: var(--fg-muted); flex-shrink: 0; }
  .ts-dd-value {
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    min-width: 0;
  }
  .ts-dd-menu {
    position: absolute; top: calc(100% + 4px); right: 0;
    min-width: 100%;
    z-index: 60;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-lg);
    padding: 4px;
    display: flex; flex-direction: column; gap: 1px;
  }
  .ts-dd-item {
    display: flex; align-items: center; justify-content: space-between;
    gap: 12px;
    background: transparent;
    border: 0;
    padding: 6px 10px;
    color: var(--fg);
    font: inherit; font-size: var(--fs-sm);
    text-align: left;
    border-radius: var(--radius-xs);
    cursor: pointer;
    transition: background 100ms;
  }
  .ts-dd-item:hover:not(:disabled) {
    background: color-mix(in oklch, var(--accent) 12%, transparent);
  }
  .ts-dd-item:disabled { color: var(--fg-faint); cursor: not-allowed; opacity: 0.6; }
  .ts-dd-item[data-active="true"] { background: color-mix(in oklch, var(--accent) 14%, transparent); }
  .ts-dd-item-l { display: inline-flex; align-items: center; gap: 8px; min-width: 0; }
  .ts-dd-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 22%, transparent);
    flex-shrink: 0;
  }
  .ts-dd-dot-spacer { width: 6px; height: 6px; flex-shrink: 0; }
  .ts-dd-dim { color: var(--fg-faint); font-size: 10px; }
  .ts-dd-empty { padding: 8px 10px; color: var(--fg-subtle); font-size: var(--fs-xs); }

  /* range slider — fully Rift-themed (native accent-color is OS-tinted on Win) */
  .ts-slider { display: inline-flex; align-items: center; gap: 10px; }
  .ts-slider input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    width: 180px;
    height: 18px;
    background: transparent;
    cursor: pointer;
  }
  .ts-slider input[type="range"]::-webkit-slider-runnable-track {
    height: 4px;
    background: var(--bg-elev-3);
    border-radius: 999px;
  }
  .ts-slider input[type="range"]::-moz-range-track {
    height: 4px;
    background: var(--bg-elev-3);
    border-radius: 999px;
  }
  .ts-slider input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 14px; height: 14px;
    margin-top: -5px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--bg);
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 60%, transparent);
    cursor: grab;
    transition: transform 80ms;
  }
  .ts-slider input[type="range"]::-moz-range-thumb {
    width: 14px; height: 14px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--bg);
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 60%, transparent);
    cursor: grab;
  }
  .ts-slider input[type="range"]:active::-webkit-slider-thumb { transform: scale(1.1); cursor: grabbing; }
  .ts-slider input[type="range"]:focus-visible { outline: none; }
  .ts-slider input[type="range"]:focus-visible::-webkit-slider-thumb {
    box-shadow: 0 0 0 1px var(--accent), 0 0 0 4px var(--ring);
  }
  .ts-slider-val {
    display: inline-block;
    min-width: 44px;
    text-align: right;
    color: var(--fg-2);
    font-size: var(--fs-xs);
  }

  /* segmented control */
  .seg {
    display: inline-flex;
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 2px;
    gap: 2px;
  }
  .seg button {
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    font: inherit;
    font-size: var(--fs-xs);
    padding: 4px 10px;
    border-radius: 3px;
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }
  .seg button:hover { color: var(--fg); }
  .seg button[data-active="true"] {
    background: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }

  /* switch */
  .switch {
    position: relative;
    width: 34px; height: 18px;
    background: var(--bg-elev-3);
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    cursor: pointer;
    padding: 0;
    transition: background 120ms, border-color 120ms;
  }
  .switch[data-on="true"] {
    background: var(--accent);
    border-color: var(--accent);
  }
  .switch-knob {
    position: absolute;
    top: 1px; left: 1px;
    width: 14px; height: 14px;
    background: var(--fg);
    border-radius: 50%;
    transition: transform 140ms cubic-bezier(0.2,0.8,0.2,1);
  }
  .switch[data-on="true"] .switch-knob {
    transform: translateX(16px);
    background: var(--accent-fg);
  }
  .switch:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }

  /* theme grid */
  .ts-theme-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 8px;
    padding: 12px 14px;
  }
  .ts-theme-card {
    display: flex; align-items: center; gap: 12px;
    padding: 10px 12px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--fg);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background 100ms, border-color 100ms, box-shadow 100ms;
  }
  .ts-theme-card:hover { background: var(--surface-hover); }
  .ts-theme-card[data-active="true"] {
    border-color: var(--accent);
    box-shadow: inset 2px 0 var(--accent), 0 0 0 1px color-mix(in oklch, var(--accent) 40%, transparent);
  }
  .ts-theme-swatch {
    flex-shrink: 0;
    width: 36px; height: 28px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-strong);
    background-image: linear-gradient(135deg, var(--sw-bg) 0% 50%, var(--sw-accent) 50% 100%);
  }
  .ts-theme-swatch[data-preset="rift"]           { --sw-bg: #0f1117; --sw-accent: #a78bfa; }
  .ts-theme-swatch[data-preset="dracula"]        { --sw-bg: #282a36; --sw-accent: #ff79c6; }
  .ts-theme-swatch[data-preset="solarized-dark"] { --sw-bg: #002b36; --sw-accent: #268bd2; }
  .ts-theme-swatch[data-preset="monokai"]        { --sw-bg: #272822; --sw-accent: #f92672; }
  .ts-theme-swatch[data-preset="github-dark"]    { --sw-bg: #0d1117; --sw-accent: #58a6ff; }
  .ts-theme-text { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .ts-theme-label { font-size: var(--fs-sm); font-weight: 500; }
  .ts-theme-sub { font-size: 10px; color: var(--fg-subtle); }

  /* Assistant settings */
  .asst-card { padding: 14px 16px; margin-top: 12px; }
  .asst-card + .asst-card { margin-top: 10px; }
  .asst-h4 { margin: 0 0 4px; font-size: var(--fs-md); font-weight: 600; }
  .asst-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-top: 8px; }
  .asst-label { font-size: var(--fs-sm); color: var(--fg-muted); min-width: 80px; }
  .asst-input-wrap { position: relative; flex: 1; min-width: 280px; display: flex; align-items: center; }
  .asst-input-wrap .asst-input { width: 100%; padding-right: 34px; }
  .asst-input { flex: 1; min-width: 280px; padding: 7px 10px; }
  .asst-eye {
    position: absolute; right: 6px; top: 50%; transform: translateY(-50%);
    display: inline-flex; align-items: center; justify-content: center;
    width: 24px; height: 24px; padding: 0;
    background: transparent; border: 0; border-radius: var(--radius-xs);
    color: var(--fg-muted); cursor: pointer;
    transition: color 140ms ease, background 140ms ease;
  }
  .asst-eye:hover { color: var(--fg); background: var(--surface-hover); }
  .asst-eye:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  .asst-pill {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 3px 9px; border-radius: 999px;
    font-size: var(--fs-xs);
    background: var(--surface-hover);
    color: var(--fg);
  }
  .asst-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--fg-muted); }
  .asst-pill[data-tone="green"]   .asst-dot { background: var(--success, #4ade80); }
  .asst-pill[data-tone="yellow"]  .asst-dot { background: var(--warn,    #fbbf24); }
  .asst-pill[data-tone="red"]     .asst-dot { background: var(--danger,  #f87171); }
  .asst-warn { color: var(--warn, #fbbf24); }

  /* Voice settings */
  .voice-dd-wrap { position: relative; margin-top: 8px; }
  .voice-dd-btn {
    width: 100%;
    display: flex; align-items: center; justify-content: space-between;
    gap: 10px;
    padding: 8px 12px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    cursor: pointer;
    transition: background 120ms, border-color 120ms;
  }
  .voice-dd-btn:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .voice-dd-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .voice-dd-menu {
    position: absolute; top: calc(100% + 4px); left: 0; right: 0;
    z-index: 20;
    max-height: 320px;
    overflow-y: auto;
    background: var(--bg-elev-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 6px 18px rgba(0,0,0,0.35);
    padding: 4px;
  }
  .voice-dd-empty {
    padding: 10px 12px;
    color: var(--fg-muted);
    font-size: var(--fs-sm);
  }
  .voice-dd-empty.err { color: var(--danger); }
  .voice-dd-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 7px 10px;
    background: transparent;
    border: 0;
    border-radius: var(--radius-xs);
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    cursor: pointer;
  }
  .voice-dd-item:hover { background: var(--surface-hover); }
  .voice-dd-item.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .voice-item-name { display: inline-flex; gap: 6px; align-items: baseline; }
  .voice-item-meta { color: var(--fg-muted); font-size: 11px; }
  .voice-dd-item.active .voice-item-meta {
    color: color-mix(in oklch, var(--accent) 70%, var(--fg-muted));
  }

  .voice-slider-row {
    display: grid;
    grid-template-columns: 64px 1fr 56px;
    align-items: center;
    gap: 12px;
    margin-top: 10px;
  }
  .voice-slider-label { font-size: var(--fs-sm); color: var(--fg-muted); }
  .voice-slider {
    width: 100%;
    accent-color: var(--accent);
  }
  .voice-slider-val { font-size: var(--fs-xs); color: var(--fg); text-align: right; }

  .set-subhead {
    margin: 28px 0 4px;
    font-size: var(--fs-lg);
    font-weight: 600;
    color: var(--fg);
  }

  .stt-lang-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 6px;
    margin-top: 8px;
  }
  .stt-lang-pick {
    display: flex; align-items: center; justify-content: space-between;
    gap: 8px;
    padding: 7px 10px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-sm);
    cursor: pointer;
    text-align: left;
    transition: background 100ms, border-color 100ms;
  }
  .stt-lang-pick:hover { background: var(--surface-hover); }
  .stt-lang-pick[data-active="true"] {
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
    background: color-mix(in oklch, var(--accent-soft) 60%, var(--bg-elev-1));
    color: var(--accent);
  }
  .stt-lang-code { font-size: 10px; color: var(--fg-muted); }
  .stt-lang-pick[data-active="true"] .stt-lang-code { color: color-mix(in oklch, var(--accent) 80%, var(--fg-muted)); }
</style>
