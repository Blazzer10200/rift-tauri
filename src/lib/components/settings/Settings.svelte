<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ServerProfile } from "../../state/connection.svelte";
  import { Cog, Server, Key, Info, Plus, Pencil, Trash2, RefreshCw, Sparkles, Palette, TerminalSquare, RotateCcw, ChevronDown, FolderOpen, Copy, Check, Eye, EyeOff, X, Mic, Accessibility as A11yIcon } from "lucide-svelte";
  import { appConfigDir, appLogDir } from "@tauri-apps/api/path";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { updates } from "../../state/updates.svelte";
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
  import { assistant as assistantStore } from "../../state/assistant.svelte";
  import { stt } from "../../state/stt.svelte";
  import { accessibility } from "../../state/accessibility.svelte";

  // SSH keys merged into Network section as a header action — one fewer nav row,
  // since the keys panel was just a single button anyway.
  type Section = "appearance" | "terminal" | "accessibility" | "assistant" | "speech" | "network" | "about";

  type ShellInfo = { id: string; label: string; program: string; args: string[]; available: boolean };
  let shells = $state<ShellInfo[]>([]);

  // Native <select> renders OS-themed dropdowns that break the dark theme.
  // We mirror the Titlebar server-picker pattern w/ custom buttons + menu.
  let shellDdOpen = $state(false);
  let shellDdRef = $state<HTMLDivElement | undefined>();
  let fontDdOpen = $state(false);
  let fontDdRef = $state<HTMLDivElement | undefined>();
  // Two-step confirm for the Terminal Reset button — top-right placement is
  // muscle-memory "Save" territory, so a single click was a footgun.
  let confirmResetTerm = $state(false);
  let confirmResetTimer: ReturnType<typeof setTimeout> | null = null;
  function clickResetTerm() {
    if (!confirmResetTerm) {
      confirmResetTerm = true;
      if (confirmResetTimer) clearTimeout(confirmResetTimer);
      confirmResetTimer = setTimeout(() => { confirmResetTerm = false; }, 3000);
      return;
    }
    if (confirmResetTimer) { clearTimeout(confirmResetTimer); confirmResetTimer = null; }
    confirmResetTerm = false;
    terminal.resetAppearance();
  }

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

  let { initialSection = "appearance", onAddServer, onEditServer, onDeleteServer, onLaunchKeygen }: {
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

  const sections: { id: Section; label: string; icon: typeof Cog; subtitle: string }[] = [
    { id: "appearance",    label: "Appearance",    icon: Palette,         subtitle: "Layout, shortcuts, and interface preferences." },
    { id: "terminal",      label: "Terminal",      icon: TerminalSquare,  subtitle: "Font, cursor, palette, and shell defaults for the embedded terminal." },
    { id: "accessibility", label: "Accessibility", icon: A11yIcon,        subtitle: "Reading comfort options for the Assistant chat." },
    { id: "assistant",     label: "Assistant",     icon: Sparkles,        subtitle: "Claude CLI session, cost guard, and conversation compaction." },
    { id: "speech",        label: "Speech",        icon: Mic,             subtitle: "Voice-to-text input. Web Speech (online) or Whisper (local, accent-tuned)." },
    { id: "network",       label: "Network",       icon: Server,          subtitle: "SSH server profiles, key management, and fingerprint pinning." },
    { id: "about",         label: "About",         icon: Info,            subtitle: "Build info, paths, updates, and support diagnostics." },
  ];
  const currentSection = $derived(sections.find(s => s.id === section)!);

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
  $effect(() => {
    if (section === "speech") {
      void stt.init();
    }
  });
  const STT_ENGINES: { id: "web_speech" | "whisper"; label: string; sub: string }[] = [
    { id: "web_speech", label: "Web Speech",    sub: "Edge · Azure when online" },
    { id: "whisper",    label: "Whisper",       sub: "On-device · accent-tuned" },
  ];
  function fmtMB(b: number): string { return (b / 1_000_000).toFixed(0) + " MB"; }
  function fmtPct(d: number, t: number): string { return t > 0 ? Math.round((d / t) * 100) + "%" : "0%"; }
  let asstApiKeyDraft = $state("");
  let asstApiKeySaving = $state(false);
  let asstApiKeyMsg = $state<string | null>(null);
  let asstApiKeyVisible = $state(false);
  let asstMaxBudgetDraft = $state<number | null>(null);
  let asstMaxBudgetSaving = $state(false);
  let asstMaxBudgetMsg = $state<string | null>(null);
  $effect(() => {
    if (section !== "assistant") return;
    untrack(() => {
      void assistantStore.init().then(() => {
        asstApiKeyDraft = assistantStore.apiKey ?? "";
        asstMaxBudgetDraft = assistantStore.maxBudgetUsd;
      });
    });
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

  const SHORTCUTS: { combo: string[]; label: string }[] = [
    { combo: ["Ctrl", "T"],         label: "New chat tab" },
    { combo: ["Ctrl", "W"],         label: "Close active tab" },
    { combo: ["Ctrl", "Tab"],       label: "Cycle tabs (Shift to reverse)" },
    { combo: ["Alt", "1…9"],        label: "Jump to chat tab N" },
    { combo: ["Ctrl", "1…7"],       label: "Switch workspace" },
    { combo: ["Ctrl", "0"],         label: "Switch to Chat workspace" },
    { combo: ["Ctrl", ","],         label: "Open Settings" },
    { combo: ["Ctrl", "\\"],        label: "Split chat pane" },
  ];
</script>

<div class="settings">
  <nav class="settings-nav">
    {#each sections as s (s.id)}
      {@const Icon = s.icon}
      <button data-active={section === s.id} onclick={() => (section = s.id)} type="button">
        <Icon size={14} strokeWidth={1.75}/><span>{s.label}</span>
      </button>
    {/each}
  </nav>

  <div class="settings-body">
    {#key section}
      <div
        class="section-shell"
        in:fly={{ y: 6, duration: 180, delay: 80, easing: quintOut }}
        out:fade={{ duration: 80 }}
      >
        <header class="section-head">
          <div class="section-head-l">
            <h2>{currentSection.label}</h2>
            <p class="section-sub">{currentSection.subtitle}</p>
          </div>
          {#if section === "terminal"}
            <div class="section-head-r">
              <button
                class="btn sm"
                class:ghost={!confirmResetTerm}
                class:danger={confirmResetTerm}
                onclick={clickResetTerm}
                type="button"
                title={confirmResetTerm ? "Click again to confirm — wipes terminal appearance back to defaults" : "Restore terminal defaults"}
              >
                <RotateCcw size={11}/>
                {confirmResetTerm ? "Confirm reset" : "Reset"}
              </button>
            </div>
          {:else if section === "network"}
            <div class="section-head-r">
              <button class="btn ghost sm" onclick={onLaunchKeygen} type="button">
                <Key size={11}/> SSH key setup
              </button>
              <button class="btn primary sm" onclick={onAddServer} type="button">
                <Plus size={11}/> Add server
              </button>
            </div>
          {:else if section === "about"}
            <div class="section-head-r">
              <button class="btn ghost sm" type="button" onclick={() => updates.open()}>
                <RefreshCw size={11}/> Check for updates
              </button>
            </div>
          {/if}
        </header>

        {#if section === "appearance"}
          <section class="set-group">
            <header class="set-group-head">Layout</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Activity bar &amp; tabs</span>
                <span class="set-hint">Click an icon on the right-edge activity bar to swap the main pane. Drag icons to reorder — the Ctrl+1…7 shortcuts follow the bar's order.</span>
              </div>
              <div class="set-row-r">
                <button class="btn ghost sm" type="button" onclick={() => void assistantStore.closeAllTabs()}>
                  <X size={11}/> Close all chat tabs
                </button>
              </div>
            </div>
          </section>

          <section class="set-group">
            <header class="set-group-head">Keyboard shortcuts</header>
            <div class="kbd-grid">
              {#each SHORTCUTS as sc (sc.label)}
                <div class="kbd-row">
                  <span class="kbd-combo">
                    {#each sc.combo as k, i}
                      {#if i > 0}<span class="kbd-plus">+</span>{/if}
                      <kbd class="kbd">{k}</kbd>
                    {/each}
                  </span>
                  <span class="kbd-label">{sc.label}</span>
                </div>
              {/each}
            </div>
          </section>

          <section class="set-group">
            <header class="set-group-head">Theme</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Color scheme</span>
                <span class="set-hint">Rift is dark-only — density, accent tint, and light-mode tokens land in a future build.</span>
              </div>
              <div class="set-row-r">
                <span class="pill muted"><span class="dot"></span>Dark · Linear-precise</span>
              </div>
            </div>
          </section>

        {:else if section === "terminal"}
          <section class="set-group">
            <header class="set-group-head">Shell</header>
            <div class="set-row">
              <div class="set-row-l">
                <label class="set-label" for="ts-default-shell">Default shell</label>
                <span class="set-hint">Used when opening a new tab without picking a preset.</span>
              </div>
              <div class="set-row-r">
                <div class="set-dd" bind:this={shellDdRef} data-open={shellDdOpen}>
                  <button
                    type="button"
                    id="ts-default-shell"
                    class="set-dd-btn"
                    aria-haspopup="listbox"
                    aria-expanded={shellDdOpen}
                    onclick={() => (shellDdOpen = !shellDdOpen)}
                  >
                    <span class="set-dd-value">
                      {shells.find((s) => s.id === terminal.defaultShellId)?.label ?? (shells.length === 0 ? "Detecting…" : "Pick a shell")}
                    </span>
                    <ChevronDown size={12}/>
                  </button>
                  {#if shellDdOpen}
                    <div class="set-dd-menu" role="listbox">
                      {#each shells as s (s.id)}
                        <button
                          type="button"
                          class="set-dd-item"
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
                          <span class="set-dd-item-l">
                            {#if terminal.defaultShellId === s.id}
                              <span class="set-dd-dot" aria-hidden="true"></span>
                            {:else}
                              <span class="set-dd-dot-spacer" aria-hidden="true"></span>
                            {/if}
                            <span>{s.label}</span>
                          </span>
                          {#if !s.available}
                            <span class="set-dd-dim mono">missing</span>
                          {/if}
                        </button>
                      {/each}
                      {#if shells.length === 0}
                        <div class="set-dd-empty">Detecting shells…</div>
                      {/if}
                    </div>
                  {/if}
                </div>
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <label class="set-label" for="ts-auto-launch">Auto-launch command</label>
                <span class="set-hint">Runs in every new tab after the prompt appears. Leave blank to skip.</span>
              </div>
              <div class="set-row-r">
                <input
                  id="ts-auto-launch"
                  class="input mono set-input"
                  type="text"
                  placeholder="e.g. claude"
                  value={terminal.autoLaunchCommand}
                  oninput={(e) => terminal.setAutoLaunchCommand((e.currentTarget as HTMLInputElement).value)}
                />
              </div>
            </div>
          </section>

          <section class="set-group">
            <header class="set-group-head">Typography</header>
            <div class="set-row">
              <div class="set-row-l">
                <label class="set-label" for="ts-font-size">Font size</label>
                <span class="set-hint">Between {TERM_FONT_SIZE_MIN} and {TERM_FONT_SIZE_MAX}px.</span>
              </div>
              <div class="set-row-r">
                <div class="set-slider">
                  <input
                    id="ts-font-size"
                    type="range"
                    min={TERM_FONT_SIZE_MIN}
                    max={TERM_FONT_SIZE_MAX}
                    step="1"
                    value={terminal.fontSize}
                    oninput={(e) => terminal.setFontSize(parseInt((e.currentTarget as HTMLInputElement).value, 10))}
                  />
                  <span class="set-slider-val mono">{terminal.fontSize}px</span>
                </div>
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <label class="set-label" for="ts-font-family">Font family</label>
                <span class="set-hint">Monospace stacks fall back if the first family is missing.</span>
              </div>
              <div class="set-row-r">
                <div class="set-dd" bind:this={fontDdRef} data-open={fontDdOpen}>
                  <button
                    type="button"
                    id="ts-font-family"
                    class="set-dd-btn"
                    aria-haspopup="listbox"
                    aria-expanded={fontDdOpen}
                    onclick={() => (fontDdOpen = !fontDdOpen)}
                  >
                    <span class="set-dd-value">
                      {FONT_FAMILY_OPTS.find((f) => f.id === terminal.fontFamilyPreset)?.label ?? "Default"}
                    </span>
                    <ChevronDown size={12}/>
                  </button>
                  {#if fontDdOpen}
                    <div class="set-dd-menu" role="listbox">
                      {#each FONT_FAMILY_OPTS as f (f.id)}
                        <button
                          type="button"
                          class="set-dd-item"
                          role="option"
                          aria-selected={terminal.fontFamilyPreset === f.id}
                          data-active={terminal.fontFamilyPreset === f.id}
                          onclick={() => {
                            terminal.setFontFamilyPreset(f.id);
                            fontDdOpen = false;
                          }}
                        >
                          <span class="set-dd-item-l">
                            {#if terminal.fontFamilyPreset === f.id}
                              <span class="set-dd-dot" aria-hidden="true"></span>
                            {:else}
                              <span class="set-dd-dot-spacer" aria-hidden="true"></span>
                            {/if}
                            <span>{f.label}</span>
                          </span>
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
              </div>
            </div>
            {#if terminal.fontFamilyPreset === "custom"}
              <div class="set-row">
                <div class="set-row-l">
                  <label class="set-label" for="ts-font-custom">Custom font stack</label>
                  <span class="set-hint">CSS font-family value. Must be monospace.</span>
                </div>
                <div class="set-row-r">
                  <input
                    id="ts-font-custom"
                    class="input mono set-input"
                    type="text"
                    placeholder={'"Fira Code", monospace'}
                    value={terminal.fontFamilyCustom}
                    oninput={(e) => terminal.setFontFamilyCustom((e.currentTarget as HTMLInputElement).value)}
                  />
                </div>
              </div>
            {/if}
          </section>

          <section class="set-group">
            <header class="set-group-head">Cursor</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Style</span>
                <span class="set-hint">Shape used when the terminal has focus.</span>
              </div>
              <div class="set-row-r">
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
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Blink</span>
                <span class="set-hint">Animate the cursor while focused.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="switch"
                  role="switch"
                  aria-label="Blink cursor"
                  aria-checked={terminal.cursorBlink}
                  data-on={terminal.cursorBlink}
                  onclick={() => terminal.setCursorBlink(!terminal.cursorBlink)}
                ><span class="switch-knob"></span></button>
              </div>
            </div>
          </section>

          <section class="set-group">
            <header class="set-group-head">Behavior</header>
            <div class="set-row">
              <div class="set-row-l">
                <label class="set-label" for="ts-scrollback">Scrollback</label>
                <span class="set-hint">Lines retained in history per tab ({TERM_SCROLLBACK_MIN.toLocaleString()}–{TERM_SCROLLBACK_MAX.toLocaleString()}).</span>
              </div>
              <div class="set-row-r">
                <input
                  id="ts-scrollback"
                  class="input mono set-input set-input-narrow"
                  type="number"
                  min={TERM_SCROLLBACK_MIN}
                  max={TERM_SCROLLBACK_MAX}
                  step="500"
                  value={terminal.scrollback}
                  onchange={(e) => terminal.setScrollback(parseInt((e.currentTarget as HTMLInputElement).value, 10))}
                />
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Bell</span>
                <span class="set-hint">What happens when a program rings the bell.</span>
              </div>
              <div class="set-row-r">
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
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Copy on select</span>
                <span class="set-hint">Selection auto-copies to clipboard.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="switch"
                  role="switch"
                  aria-label="Copy on select"
                  aria-checked={terminal.copyOnSelect}
                  data-on={terminal.copyOnSelect}
                  onclick={() => terminal.setCopyOnSelect(!terminal.copyOnSelect)}
                ><span class="switch-knob"></span></button>
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Right-click paste</span>
                <span class="set-hint">Right-click in the terminal pastes clipboard contents.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="switch"
                  role="switch"
                  aria-label="Right-click paste"
                  aria-checked={terminal.rightClickPaste}
                  data-on={terminal.rightClickPaste}
                  onclick={() => terminal.setRightClickPaste(!terminal.rightClickPaste)}
                ><span class="switch-knob"></span></button>
              </div>
            </div>
          </section>

          <section class="set-group">
            <header class="set-group-head">Theme</header>
            <div class="set-theme-grid">
              {#each THEME_PRESETS as t (t.id)}
                <button
                  type="button"
                  class="set-theme-card"
                  data-active={terminal.themePreset === t.id}
                  onclick={() => terminal.setThemePreset(t.id as ThemePresetId)}
                >
                  <span class="set-theme-swatch" data-preset={t.id}></span>
                  <span class="set-theme-text">
                    <span class="set-theme-label">{t.label}</span>
                    <span class="set-theme-sub">{t.subtitle}</span>
                  </span>
                </button>
              {/each}
            </div>
          </section>

        {:else if section === "accessibility"}
          <section class="set-group">
            <header class="set-group-head">Reading comfort</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Dyslexia-friendly mode</span>
                <span class="set-hint">Lexend font + wider line spacing, and tells Claude to interpret phonetic typos / voice-to-text artifacts charitably.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="switch"
                  role="switch"
                  aria-label="Dyslexia-friendly mode"
                  aria-checked={accessibility.dyslexiaMode}
                  data-on={accessibility.dyslexiaMode}
                  onclick={() => accessibility.setDyslexiaMode(!accessibility.dyslexiaMode)}
                ><span class="switch-knob"></span></button>
              </div>
            </div>
            <div class="set-row" data-disabled={!accessibility.dyslexiaMode}>
              <div class="set-row-l">
                <span class="set-label">UI font</span>
                <span class="set-hint">Lexend has the strongest research backing for reading-rate improvement on dyslexic readers.</span>
              </div>
              <div class="set-row-r">
                <div class="seg" role="radiogroup" aria-label="UI font">
                  <button
                    type="button"
                    role="radio"
                    aria-checked={accessibility.font === "system"}
                    data-active={accessibility.font === "system"}
                    disabled={!accessibility.dyslexiaMode}
                    onclick={() => accessibility.setFont("system")}
                  >Inter</button>
                  <button
                    type="button"
                    role="radio"
                    aria-checked={accessibility.font === "lexend"}
                    data-active={accessibility.font === "lexend"}
                    disabled={!accessibility.dyslexiaMode}
                    onclick={() => accessibility.setFont("lexend")}
                  >Lexend</button>
                </div>
              </div>
            </div>
            <div class="set-row" data-disabled={!accessibility.dyslexiaMode}>
              <div class="set-row-l">
                <span class="set-label">Wider line + letter spacing</span>
                <span class="set-hint">Bumps line-height to 1.85 inside Assistant bubbles and the composer.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="switch"
                  role="switch"
                  aria-label="Increased line and letter spacing"
                  aria-checked={accessibility.lineHeightBoost}
                  data-on={accessibility.lineHeightBoost}
                  disabled={!accessibility.dyslexiaMode}
                  onclick={() => accessibility.setLineHeightBoost(!accessibility.lineHeightBoost)}
                ><span class="switch-knob"></span></button>
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Warm reading tint</span>
                <span class="set-hint">Sepia overlay on Assistant message bubbles — softens bright-white-on-dark glare. UI chrome keeps the dark theme.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="switch"
                  role="switch"
                  aria-label="Warm reading tint"
                  aria-checked={accessibility.warmTint}
                  data-on={accessibility.warmTint}
                  onclick={() => accessibility.setWarmTint(!accessibility.warmTint)}
                ><span class="switch-knob"></span></button>
              </div>
            </div>
          </section>

        {:else if section === "assistant"}
          <section class="set-group">
            <header class="set-group-head">CLI session</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Status</span>
                <span class="set-hint">Rift piggybacks on your local <code>claude</code> CLI session by default. Not signed in? Run <code>claude login</code> in a terminal, then re-probe.</span>
              </div>
              <div class="set-row-r">
                {#if assistantStore.auth}
                  <span class="pill" data-tone={assistantStore.auth.pill === "green" ? "ok" : assistantStore.auth.pill === "yellow" ? "warn" : assistantStore.auth.pill === "red" ? "danger" : "muted"}>
                    <span class="dot"></span>{assistantStore.auth.summary}
                  </span>
                {:else if assistantStore.authChecking}
                  <span class="pill muted"><span class="dot"></span>Checking…</span>
                {:else}
                  <span class="pill muted"><span class="dot"></span>Unknown</span>
                {/if}
                <button class="btn ghost sm" type="button" onclick={() => assistantStore.refreshAuth()} disabled={assistantStore.authChecking}>
                  <RefreshCw size={11}/> Re-probe
                </button>
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Use my full Claude Code config</span>
                <span class="set-hint">Layers <code>~/.claude/CLAUDE.md</code>, slash commands, skills, and MCP servers into every turn alongside Rift's own MCP tools. Off = sandboxed (Rift MCP only).</span>
              </div>
              <div class="set-row-r">
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
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Allow remote shell</span>
                <span class="set-hint">Exposes <code>mcp__rift__remote_bash</code> to the model — runs commands on the connected SSH server. Workspace-scoped advisory lock serializes calls.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="switch"
                  role="switch"
                  aria-label="Allow remote shell"
                  aria-checked={assistantStore.allowRemoteShell}
                  data-on={assistantStore.allowRemoteShell}
                  onclick={() => void assistantStore.setAllowRemoteShell(!assistantStore.allowRemoteShell)}
                ><span class="switch-knob"></span></button>
              </div>
            </div>
          </section>

          <section class="set-group">
            <header class="set-group-head">Budget &amp; billing</header>
            <div class="set-row">
              <div class="set-row-l">
                <label class="set-label" for="asst-budget">Per-turn cost cap</label>
                <span class="set-hint">Passes <code>--max-budget-usd</code> to the CLI. If a turn would exceed this cap, the CLI exits with an error. Leave blank for no cap.</span>
              </div>
              <div class="set-row-r">
                <input
                  id="asst-budget"
                  class="input mono set-input set-input-narrow"
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
            </div>
            {#if asstMaxBudgetMsg}
              <div class="set-row set-row-note"><span class="set-note">{asstMaxBudgetMsg}</span></div>
            {/if}
            <div class="set-row">
              <div class="set-row-l">
                <label class="set-label" for="asst-apikey">API-key fallback</label>
                <span class="set-hint">Pay-per-token via console.anthropic.com. When set, overrides the CLI session (forces <code>--bare</code>). Stored plaintext in <code>~/.rift/assistant/config.json</code>.</span>
              </div>
              <div class="set-row-r">
                <div class="set-secret-wrap">
                  <input
                    id="asst-apikey"
                    class="input mono set-input"
                    type={asstApiKeyVisible ? "text" : "password"}
                    placeholder="sk-ant-api03-…"
                    bind:value={asstApiKeyDraft}
                    autocomplete="off"
                    spellcheck="false"
                  />
                  <button
                    class="set-eye"
                    type="button"
                    onclick={() => (asstApiKeyVisible = !asstApiKeyVisible)}
                    aria-label={asstApiKeyVisible ? "Hide API key" : "Show API key"}
                    title={asstApiKeyVisible ? "Hide API key" : "Show API key"}
                  >
                    {#if asstApiKeyVisible}<EyeOff size={13}/>{:else}<Eye size={13}/>{/if}
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
            </div>
            {#if asstApiKeyMsg}
              <div class="set-row set-row-note"><span class="set-note">{asstApiKeyMsg}</span></div>
            {/if}
          </section>

          <section class="set-group">
            <header class="set-group-head">Conversation compaction</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Auto-compact threshold</span>
                <span class="set-hint">When ctx (including cache-read) fills past the threshold, Rift summarizes via <code>claude -p</code> and seeds the next turn with the summary. Cache-read tokens count — they sit in the model's window every turn. 5min cooldown between fires.</span>
              </div>
              <div class="set-row-r">
                <select
                  id="auto-compact-threshold"
                  class="input mono set-input set-input-narrow"
                  value={assistantStore.autoCompactThreshold ?? 0}
                  onchange={(e) => {
                    const raw = Number((e.currentTarget as HTMLSelectElement).value);
                    void assistantStore.setAutoCompactThreshold(raw > 0 ? raw : null);
                  }}
                >
                  <option value={0}>Off</option>
                  <option value={0.70}>70%</option>
                  <option value={0.80}>80% (recommended)</option>
                  <option value={0.85}>85%</option>
                  <option value={0.90}>90%</option>
                </select>
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Compact model</span>
                <span class="set-hint">Haiku is sufficient for prose summarization. Sonnet only if Haiku misses details on your workflow.</span>
              </div>
              <div class="set-row-r">
                <div class="seg" role="radiogroup" aria-label="Compact model">
                  <button
                    type="button"
                    role="radio"
                    aria-checked={assistantStore.compactModel === "haiku"}
                    data-active={assistantStore.compactModel === "haiku"}
                    onclick={() => void assistantStore.setCompactModel("haiku")}
                  >Haiku</button>
                  <button
                    type="button"
                    role="radio"
                    aria-checked={assistantStore.compactModel === "sonnet"}
                    data-active={assistantStore.compactModel === "sonnet"}
                    onclick={() => void assistantStore.setCompactModel("sonnet")}
                  >Sonnet</button>
                </div>
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Compact now</span>
                <span class="set-hint">Fires <code>compactConversation()</code> on the active chat regardless of threshold. Needs ≥4 messages and no in-flight stream.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="btn primary sm"
                  onclick={() => void assistantStore.compactConversation()}
                >Compact now</button>
              </div>
            </div>
          </section>

        {:else if section === "speech"}
          <section class="set-group">
            <header class="set-group-head">Engine</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Speech-to-text</span>
                <span class="set-hint">Master switch. When off, the mic button in the composer is hidden.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="switch"
                  role="switch"
                  aria-label="Enable speech-to-text"
                  aria-checked={stt.config.enabled}
                  data-on={stt.config.enabled}
                  onclick={() => void stt.setConfig({ enabled: !stt.config.enabled })}
                ><span class="switch-knob"></span></button>
              </div>
            </div>
            <div class="set-row set-row-stack">
              <div class="set-row-l set-row-l-full">
                <span class="set-label">Recognition engine</span>
                <span class="set-hint">Web Speech is zero-install via Edge / Azure. Whisper runs on-device with stronger accent tolerance and vocabulary priming.</span>
              </div>
              <div class="set-pick-grid set-pick-grid-2">
                {#each STT_ENGINES as eng (eng.id)}
                  <button
                    type="button"
                    class="set-pick"
                    data-active={stt.config.engine === eng.id}
                    disabled={!stt.config.enabled || (eng.id === "whisper" && !stt.backendAvailable)}
                    onclick={() => void stt.setConfig({ engine: eng.id })}
                  >
                    <span class="set-pick-label">{eng.label}</span>
                    <span class="set-pick-sub mono">{eng.sub}</span>
                  </button>
                {/each}
              </div>
            </div>
            {#if !stt.backendAvailable}
              <div class="set-row set-row-note">
                <span class="set-warn-banner">
                  Whisper backend not built into this Rift release. To enable: install LLVM
                  (<code>winget install LLVM.LLVM</code>, admin required), optionally install the NVIDIA CUDA Toolkit
                  for GPU acceleration, then rebuild with <code>cargo build --release --features whisper-rs</code>.
                </span>
              </div>
            {/if}
          </section>

          {#if stt.config.engine === "web_speech"}
            <section class="set-group">
              <header class="set-group-head">Web Speech</header>
              <div class="set-row set-row-stack">
                <div class="set-row-l set-row-l-full">
                  <span class="set-label">Language</span>
                  <span class="set-hint">BCP-47 tag passed to the recogniser. Pick another language if you speak something other than English.</span>
                </div>
                <div class="set-pick-grid">
                  {#each STT_LANGS as l (l.id)}
                    <button
                      type="button"
                      class="set-pick"
                      data-active={stt.config.language === l.id}
                      onclick={() => void stt.setConfig({ language: l.id })}
                    >
                      <span class="set-pick-label">{l.label}</span>
                      <span class="set-pick-sub mono">{l.id}</span>
                    </button>
                  {/each}
                </div>
              </div>
              <div class="set-row">
                <div class="set-row-l">
                  <span class="set-label">Continuous mode</span>
                  <span class="set-hint">Keep listening across pauses until you click stop.</span>
                </div>
                <div class="set-row-r">
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
                </div>
              </div>
            </section>
          {/if}

          {#if stt.config.engine === "whisper"}
            <section class="set-group">
              <header class="set-group-head">Whisper model</header>
              <div class="set-model-list">
                {#each stt.models as m (m.id)}
                  {@const prog = stt.modelDownloads[m.id]}
                  {@const isActive = stt.config.whisper_model === m.id}
                  {@const isDownloading = prog && (prog.phase === "start" || prog.phase === "progress")}
                  <div class="set-model-row" data-active={isActive}>
                    <div class="set-model-meta">
                      <div class="set-model-name">{m.display_name}</div>
                      <div class="set-model-sub mono">
                        {m.filename}
                        {#if m.downloaded && m.on_disk_bytes !== null} · {fmtMB(m.on_disk_bytes)} on disk{:else} · ~{fmtMB(m.approx_size_bytes)}{/if}
                      </div>
                      {#if isDownloading && prog}
                        <div class="set-progress" role="progressbar" aria-valuemin="0" aria-valuemax={prog.total} aria-valuenow={prog.downloaded}>
                          <div class="set-progress-fill" style="width: {fmtPct(prog.downloaded, prog.total)}"></div>
                        </div>
                        <div class="set-progress-label muted mono">{fmtMB(prog.downloaded)} / {fmtMB(prog.total)} · {fmtPct(prog.downloaded, prog.total)}</div>
                      {/if}
                    </div>
                    <div class="set-model-actions">
                      {#if isDownloading}
                        <button type="button" class="btn sm" onclick={() => void stt.cancelDownload()}>Cancel</button>
                      {:else if m.downloaded}
                        {#if !isActive}
                          <button type="button" class="btn sm" onclick={() => void stt.setConfig({ whisper_model: m.id })}>Use</button>
                        {:else}
                          <span class="pill ok"><span class="dot"></span>Active</span>
                        {/if}
                        <button type="button" class="btn ghost sm" onclick={() => void stt.deleteModel(m.id)} title="Delete model" aria-label="Delete">
                          <Trash2 size={11}/>
                        </button>
                      {:else}
                        <button type="button" class="btn primary sm" disabled={!stt.config.enabled} onclick={() => void stt.downloadModel(m.id)}>Download</button>
                      {/if}
                    </div>
                  </div>
                {/each}
                {#if stt.models.length === 0}
                  <div class="set-row-note"><span class="set-note">Loading model catalogue…</span></div>
                {/if}
              </div>
            </section>

            <section class="set-group">
              <header class="set-group-head">Capture &amp; cleanup</header>
              <div class="set-row">
                <div class="set-row-l">
                  <span class="set-label">Input device</span>
                  <span class="set-hint">Microphone used by Whisper capture. System default is usually correct.</span>
                </div>
                <div class="set-row-r set-mic-r">
                  <select
                    class="input set-input set-mic-select"
                    disabled={!stt.config.enabled}
                    value={stt.config.input_device ?? ""}
                    onchange={(e) => {
                      const v = (e.currentTarget as HTMLSelectElement).value;
                      void stt.setConfig({ input_device: v === "" ? null : v });
                    }}
                  >
                    <option value="">System default</option>
                    {#each stt.inputDevices as d (d)}
                      <option value={d}>{d}</option>
                    {/each}
                  </select>
                  <button type="button" class="btn ghost sm" onclick={() => void stt.refreshInputDevices()} title="Refresh device list" aria-label="Refresh">
                    <RefreshCw size={11}/>
                  </button>
                </div>
              </div>
              <div class="set-row">
                <div class="set-row-l">
                  <span class="set-label">Clean with Claude Haiku</span>
                  <span class="set-hint">Polishes the final transcript via Haiku — fixes punctuation, capitalises proper nouns. ~200-400ms tail, ~$0.0001/utterance.</span>
                </div>
                <div class="set-row-r">
                  <button
                    type="button"
                    class="switch"
                    role="switch"
                    aria-label="Clean transcripts with Claude Haiku"
                    aria-checked={stt.config.cleanup_enabled}
                    data-on={stt.config.cleanup_enabled}
                    disabled={!stt.config.enabled}
                    onclick={() => void stt.setConfig({ cleanup_enabled: !stt.config.cleanup_enabled })}
                  ><span class="switch-knob"></span></button>
                </div>
              </div>
            </section>

            <section class="set-group">
              <header class="set-group-head">Vocabulary priming</header>
              <div class="set-row set-row-stack">
                <div class="set-row-l set-row-l-full">
                  <span class="set-label">Style prompt</span>
                  <span class="set-hint">Whisper's <code>initial_prompt</code> biases the decoder toward your speaking style.</span>
                </div>
                <textarea
                  class="input mono set-textarea"
                  rows="3"
                  disabled={!stt.config.enabled}
                  value={stt.config.initial_prompt}
                  oninput={(e) => void stt.setConfig({ initial_prompt: (e.currentTarget as HTMLTextAreaElement).value })}
                ></textarea>
              </div>
              <div class="set-row set-row-stack">
                <div class="set-row-l set-row-l-full">
                  <span class="set-label">Vocabulary</span>
                  <span class="set-hint">Comma- or newline-separated. Add project names, in-jokes, server names. Budget ~800 chars (Whisper's 224-token prompt limit). Vocab tail truncates first.</span>
                </div>
                <textarea
                  class="input mono set-textarea"
                  rows="4"
                  placeholder="FiveM, Qbox, RedM, rift_bridge, fxmanifest, ..."
                  disabled={!stt.config.enabled}
                  value={stt.config.vocab_text}
                  oninput={(e) => void stt.setConfig({ vocab_text: (e.currentTarget as HTMLTextAreaElement).value })}
                ></textarea>
              </div>
            </section>
          {/if}

          <section class="set-group">
            <header class="set-group-head">Composer integration</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Live partial transcripts</span>
                <span class="set-hint">Words appear in the composer as you speak. Off = wait for each sentence to commit.</span>
              </div>
              <div class="set-row-r">
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
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Insertion mode</span>
                <span class="set-hint">Append preserves what's typed; off = transcript replaces composer contents (mic-first workflow).</span>
              </div>
              <div class="set-row-r">
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
              </div>
            </div>
          </section>

          {#if stt.config.engine === "web_speech" && !stt.supported}
            <div class="set-row-note">
              <span class="set-warn-banner">
                Your WebView does not expose <code>SpeechRecognition</code>; Web Speech is unavailable — switch to Whisper or install LLVM and rebuild.
              </span>
            </div>
          {/if}
          {#if stt.lastError}
            <div class="set-row-note">
              <span class="set-warn-banner">{stt.lastError}</span>
            </div>
          {/if}

        {:else if section === "network"}
          <section class="set-group">
            <header class="set-group-head">
              SSH servers
              <span class="set-group-count">{connection.servers.length}</span>
            </header>
            {#if connection.servers.length === 0}
              <div class="set-empty">
                <span class="set-empty-title">No servers yet</span>
                <span class="set-empty-hint">Click <strong>Add server</strong> above to create your first profile.</span>
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
                      <span class="srv-dot"></span>
                      <div class="srv-meta-l">
                        <div class="mono srv-name">{s.name}</div>
                        <div class="mono dim srv-host">{s.user}@{s.host}{s.port !== 22 ? `:${s.port}` : ""}</div>
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
          </section>

          <section class="set-group">
            <header class="set-group-head">SSH keypair</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Local key</span>
                <span class="set-hint">ed25519 keypair stored at <code>%APPDATA%/Rift/keys/</code>. Generate or copy your public key from the setup dialog.</span>
              </div>
              <div class="set-row-r">
                <button class="btn ghost sm" onclick={onLaunchKeygen} type="button">
                  <Key size={11}/> Open key setup
                </button>
              </div>
            </div>
          </section>

        {:else if section === "about"}
          <section class="set-group">
            <header class="set-group-head">Build</header>
            <div class="set-row">
              <div class="set-row-l"><span class="set-label">Rift</span></div>
              <div class="set-row-r"><span class="mono dim">{appVersion} · Tauri 2</span></div>
            </div>
            <div class="set-row">
              <div class="set-row-l"><span class="set-label">Engine</span></div>
              <div class="set-row-r"><span class="mono dim">SvelteKit + Svelte 5 (runes)</span></div>
            </div>
            <div class="set-row">
              <div class="set-row-l"><span class="set-label">Style</span></div>
              <div class="set-row-r"><span class="mono dim">Tailwind v4 · OKLCH tokens</span></div>
            </div>
            <div class="set-row">
              <div class="set-row-l"><span class="set-label">SSH</span></div>
              <div class="set-row-r"><span class="mono dim">russh + russh-sftp · ring backend</span></div>
            </div>
            <div class="set-row">
              <div class="set-row-l"><span class="set-label">License</span></div>
              <div class="set-row-r"><span class="mono dim">MIT · github.com/Blazzer10200/rift</span></div>
            </div>
          </section>

          <section class="set-group">
            <header class="set-group-head">Paths</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Config</span>
                <span class="set-hint mono path-val" title={configDir}>{configDir || "—"}</span>
              </div>
              <div class="set-row-r">
                <button class="btn ghost sm" type="button" disabled={!configDir} onclick={() => openDir(configDir)}>
                  <FolderOpen size={11}/> Open
                </button>
              </div>
            </div>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Logs</span>
                <span class="set-hint mono path-val" title={logDir}>{logDir || "—"}</span>
              </div>
              <div class="set-row-r">
                <button class="btn ghost sm" type="button" disabled={!logDir} onclick={() => openDir(logDir)}>
                  <FolderOpen size={11}/> Open
                </button>
              </div>
            </div>
          </section>

          <section class="set-group">
            <header class="set-group-head">Diagnostics</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Support info</span>
                <span class="set-hint">Copies an OS-username-scrubbed bundle (version, platform, paths, server state) to your clipboard.</span>
              </div>
              <div class="set-row-r">
                <button class="btn ghost sm" type="button" onclick={copyDiagnostic}>
                  {#if diagCopied}<Check size={11}/> Copied{:else}<Copy size={11}/> Copy diagnostic{/if}
                </button>
              </div>
            </div>
          </section>
        {/if}
      </div>
    {/key}
  </div>
</div>

<style>
  /* ─── Layout shell ─── */
  .settings {
    display: grid;
    grid-template-columns: 220px 1fr;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  /* ─── Nav rail ─── */
  .settings-nav {
    display: flex; flex-direction: column; gap: 1px;
    padding: 14px 10px;
    background: var(--bg);
    border-right: 1px solid var(--border);
    overflow: auto;
  }
  .settings-nav button {
    position: relative;
    display: flex; align-items: center; gap: 10px;
    width: 100%; height: 30px; padding: 0 10px;
    background: transparent; border: 0;
    color: var(--fg-muted);
    text-align: left; font: inherit; font-size: var(--fs-sm);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }
  .settings-nav button :global(svg) { color: var(--fg-subtle); transition: color 100ms; flex-shrink: 0; }
  .settings-nav button:hover { background: var(--surface-hover); color: var(--fg); }
  .settings-nav button:hover :global(svg) { color: var(--fg-muted); }
  .settings-nav button[data-active="true"] {
    background: color-mix(in oklch, var(--accent) 14%, var(--surface));
    color: var(--fg);
    font-weight: 500;
  }
  .settings-nav button[data-active="true"] :global(svg) { color: var(--accent); }
  .settings-nav button[data-active="true"]::before {
    content: "";
    position: absolute;
    left: -10px; top: 6px; bottom: 6px;
    width: 2px;
    background: var(--accent);
    border-radius: 0 2px 2px 0;
  }

  /* ─── Body ─── */
  .settings-body {
    position: relative;
    min-width: 0;
    overflow: hidden;
  }
  .section-shell {
    position: absolute;
    inset: 0;
    padding: 22px 28px 32px;
    overflow: auto;
    display: flex; flex-direction: column;
    gap: 16px;
  }
  .section-shell::-webkit-scrollbar { width: 10px; }

  .section-head {
    display: flex; align-items: flex-start; justify-content: space-between;
    gap: 16px;
    padding-bottom: 4px;
    margin-bottom: 2px;
  }
  .section-head-l { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .section-head-r { display: flex; align-items: center; gap: 6px; flex-shrink: 0; padding-top: 4px; }
  .section-head h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--fg);
    letter-spacing: -0.015em;
  }
  .section-sub {
    margin: 0;
    font-size: var(--fs-xs);
    color: var(--fg-muted);
    max-width: 640px;
    line-height: 1.5;
  }

  /* ─── Unified group + row pattern ─── */
  .set-group {
    display: flex; flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    /* No overflow:hidden — dropdown menus need to escape the card. */
  }
  .set-group-head {
    display: flex; align-items: center; justify-content: space-between;
    padding: 9px 14px;
    background: var(--bg-elev-1);
    border-bottom: 1px solid var(--border);
    border-radius: calc(var(--radius) - 1px) calc(var(--radius) - 1px) 0 0;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-subtle);
  }
  .set-group-count {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 18px; height: 16px; padding: 0 5px;
    border-radius: 999px;
    background: var(--bg-elev-3);
    color: var(--fg-2);
    font-size: 9.5px;
    letter-spacing: 0.02em;
    font-weight: 600;
  }

  .set-row {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 16px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
  }
  .set-row:last-child { border-bottom: none; }
  .set-row[data-disabled="true"] { opacity: 0.55; }
  .set-row[data-disabled="true"] .set-label,
  .set-row[data-disabled="true"] .set-hint { color: var(--fg-faint); }

  .set-row-l { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .set-row-l-full { width: 100%; }
  .set-row-r { display: flex; align-items: center; gap: 8px; justify-content: flex-end; flex-wrap: wrap; }
  .set-row-stack { grid-template-columns: 1fr; gap: 8px; align-items: stretch; }
  .set-row-note {
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elev-1);
  }
  .set-row-note:last-child { border-bottom: none; }
  .set-note {
    font-size: var(--fs-xs);
    color: var(--fg-muted);
  }
  .set-warn-banner {
    display: block;
    font-size: var(--fs-xs);
    color: var(--warn);
    line-height: 1.5;
    padding: 8px 11px;
    background: color-mix(in oklch, var(--warn) 10%, var(--bg-elev-1));
    border: 1px solid color-mix(in oklch, var(--warn) 30%, var(--border));
    border-radius: var(--radius-sm);
  }
  .set-warn-banner code {
    background: var(--bg-elev-2);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--fg-2);
  }

  .set-label { font-size: var(--fs-sm); font-weight: 500; color: var(--fg); }
  .set-hint  { font-size: var(--fs-xs); color: var(--fg-subtle); line-height: 1.5; max-width: 540px; }
  .set-hint code {
    background: var(--bg-elev-2);
    padding: 1px 5px;
    border-radius: 3px;
    color: var(--fg-2);
    font-family: var(--font-mono);
    font-size: 0.93em;
  }
  .path-val {
    background: var(--bg-elev-2);
    padding: 3px 8px;
    border-radius: var(--radius-xs);
    color: var(--fg-muted);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    display: inline-block;
    max-width: 480px;
    margin-top: 2px;
  }

  /* ─── Inputs ─── */
  .set-input { width: 260px; max-width: 100%; }
  .set-input-narrow { width: 140px; text-align: right; }
  .set-input-narrow::-webkit-outer-spin-button,
  .set-input-narrow::-webkit-inner-spin-button { -webkit-appearance: none; margin: 0; }
  .set-input-narrow { appearance: textfield; -moz-appearance: textfield; }
  .set-secret-wrap { position: relative; display: inline-flex; align-items: center; }
  .set-secret-wrap .set-input { padding-right: 32px; }
  .set-eye {
    position: absolute; right: 6px;
    display: inline-flex; align-items: center; justify-content: center;
    width: 22px; height: 22px; padding: 0;
    background: transparent; border: 0; border-radius: var(--radius-xs);
    color: var(--fg-muted); cursor: pointer;
    transition: color 120ms, background 120ms;
  }
  .set-eye:hover { color: var(--fg); background: var(--surface-hover); }
  .set-eye:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }

  .set-textarea {
    width: 100%;
    padding: 8px 10px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--fg);
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    line-height: 1.5;
    resize: vertical;
    min-height: 60px;
  }
  .set-textarea:focus {
    outline: none;
    border-color: var(--border-focus);
    box-shadow: 0 0 0 3px var(--ring);
  }
  .set-textarea:disabled { opacity: 0.55; cursor: not-allowed; }

  /* ─── Custom dropdown (shared w/ Titlebar pattern) ─── */
  .set-dd { position: relative; width: 240px; }
  .set-dd-btn {
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
  .set-dd-btn:hover {
    background: var(--surface-hover);
    border-color: color-mix(in oklch, var(--accent) 30%, var(--border-strong));
  }
  .set-dd-btn:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .set-dd[data-open="true"] .set-dd-btn {
    border-color: var(--border-focus);
    box-shadow: 0 0 0 2px var(--ring);
  }
  .set-dd-btn :global(svg) { color: var(--fg-muted); flex-shrink: 0; }
  .set-dd-value { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .set-dd-menu {
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
  .set-dd-item {
    display: flex; align-items: center; justify-content: space-between;
    gap: 12px;
    background: transparent; border: 0;
    padding: 6px 10px;
    color: var(--fg);
    font: inherit; font-size: var(--fs-sm);
    text-align: left;
    border-radius: var(--radius-xs);
    cursor: pointer;
    transition: background 100ms;
  }
  .set-dd-item:hover:not(:disabled) { background: color-mix(in oklch, var(--accent) 12%, transparent); }
  .set-dd-item:disabled { color: var(--fg-faint); cursor: not-allowed; opacity: 0.6; }
  .set-dd-item[data-active="true"] { background: color-mix(in oklch, var(--accent) 14%, transparent); }
  .set-dd-item-l { display: inline-flex; align-items: center; gap: 8px; min-width: 0; }
  .set-dd-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 22%, transparent);
    flex-shrink: 0;
  }
  .set-dd-dot-spacer { width: 6px; height: 6px; flex-shrink: 0; }
  .set-dd-dim { color: var(--fg-faint); font-size: 10px; }
  .set-dd-empty { padding: 8px 10px; color: var(--fg-subtle); font-size: var(--fs-xs); }

  /* ─── Slider ─── */
  .set-slider { display: inline-flex; align-items: center; gap: 10px; }
  .set-slider input[type="range"] {
    -webkit-appearance: none; appearance: none;
    width: 180px; height: 18px;
    background: transparent; cursor: pointer;
  }
  .set-slider input[type="range"]::-webkit-slider-runnable-track {
    height: 4px; background: var(--bg-elev-3); border-radius: 999px;
  }
  .set-slider input[type="range"]::-moz-range-track {
    height: 4px; background: var(--bg-elev-3); border-radius: 999px;
  }
  .set-slider input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none; appearance: none;
    width: 14px; height: 14px; margin-top: -5px;
    border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--bg);
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 60%, transparent);
    cursor: grab;
    transition: transform 80ms;
  }
  .set-slider input[type="range"]::-moz-range-thumb {
    width: 14px; height: 14px; border-radius: 50%;
    background: var(--accent);
    border: 2px solid var(--bg);
    box-shadow: 0 0 0 1px color-mix(in oklch, var(--accent) 60%, transparent);
    cursor: grab;
  }
  .set-slider input[type="range"]:active::-webkit-slider-thumb { transform: scale(1.1); cursor: grabbing; }
  .set-slider input[type="range"]:focus-visible { outline: none; }
  .set-slider input[type="range"]:focus-visible::-webkit-slider-thumb {
    box-shadow: 0 0 0 1px var(--accent), 0 0 0 4px var(--ring);
  }
  .set-slider-val {
    display: inline-block;
    min-width: 44px;
    text-align: right;
    color: var(--fg-2);
    font-size: var(--fs-xs);
  }

  /* ─── Segmented ─── */
  .seg {
    display: inline-flex;
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    padding: 2px;
    gap: 2px;
  }
  .seg button {
    background: transparent; border: 0;
    color: var(--fg-muted);
    font: inherit; font-size: var(--fs-xs);
    padding: 4px 12px;
    border-radius: 3px;
    cursor: pointer;
    transition: background 100ms, color 100ms;
  }
  .seg button:hover:not(:disabled) { color: var(--fg); }
  .seg button:disabled { color: var(--fg-faint); cursor: not-allowed; }
  .seg button[data-active="true"] {
    background: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }

  /* ─── Switch ─── */
  .switch {
    position: relative;
    width: 34px; height: 18px;
    background: var(--bg-elev-3);
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    cursor: pointer; padding: 0;
    transition: background 120ms, border-color 120ms;
    flex-shrink: 0;
  }
  .switch[data-on="true"] { background: var(--accent); border-color: var(--accent); }
  .switch:disabled { opacity: 0.5; cursor: not-allowed; }
  .switch-knob {
    position: absolute; top: 1px; left: 1px;
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

  /* ─── Terminal theme grid ─── */
  .set-theme-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 8px;
    padding: 12px 14px;
  }
  .set-theme-card {
    display: flex; align-items: center; gap: 12px;
    padding: 10px 12px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--fg);
    font: inherit; text-align: left;
    cursor: pointer;
    transition: background 100ms, border-color 100ms, box-shadow 100ms;
  }
  .set-theme-card:hover { background: var(--surface-hover); }
  .set-theme-card[data-active="true"] {
    border-color: var(--accent);
    box-shadow: inset 2px 0 var(--accent), 0 0 0 1px color-mix(in oklch, var(--accent) 40%, transparent);
  }
  .set-theme-swatch {
    flex-shrink: 0;
    width: 36px; height: 28px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-strong);
    background-image: linear-gradient(135deg, var(--sw-bg) 0% 50%, var(--sw-accent) 50% 100%);
  }
  .set-theme-swatch[data-preset="rift"]           { --sw-bg: #0f1117; --sw-accent: #a78bfa; }
  .set-theme-swatch[data-preset="dracula"]        { --sw-bg: #282a36; --sw-accent: #ff79c6; }
  .set-theme-swatch[data-preset="solarized-dark"] { --sw-bg: #002b36; --sw-accent: #268bd2; }
  .set-theme-swatch[data-preset="monokai"]        { --sw-bg: #272822; --sw-accent: #f92672; }
  .set-theme-swatch[data-preset="github-dark"]    { --sw-bg: #0d1117; --sw-accent: #58a6ff; }
  .set-theme-text { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .set-theme-label { font-size: var(--fs-sm); font-weight: 500; }
  .set-theme-sub { font-size: 10px; color: var(--fg-subtle); }

  /* ─── Pick grid (lang + engine tiles) ─── */
  .set-pick-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 6px;
    width: 100%;
  }
  .set-pick-grid-2 { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .set-pick {
    display: flex; align-items: center; justify-content: space-between;
    gap: 8px;
    padding: 8px 11px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--fg);
    font: inherit; font-size: var(--fs-sm);
    cursor: pointer;
    text-align: left;
    transition: background 100ms, border-color 100ms;
  }
  .set-pick:hover { background: var(--surface-hover); }
  .set-pick[data-active="true"] {
    border-color: color-mix(in oklch, var(--accent) 45%, var(--border));
    background: color-mix(in oklch, var(--accent-soft) 60%, var(--bg-elev-1));
    color: var(--accent);
  }
  .set-pick-label { font-weight: 500; }
  .set-pick-sub { font-size: 10px; color: var(--fg-muted); }
  .set-pick[data-active="true"] .set-pick-sub { color: color-mix(in oklch, var(--accent) 80%, var(--fg-muted)); }
  .set-pick:disabled:not([data-active="true"]) { opacity: 0.5; cursor: not-allowed; }

  /* ─── Whisper models ─── */
  .set-model-list { display: flex; flex-direction: column; gap: 6px; padding: 12px 14px; }
  .set-model-row {
    display: flex; align-items: center; gap: 12px;
    padding: 10px 12px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .set-model-row[data-active="true"] {
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
    background: color-mix(in oklch, var(--accent-soft) 40%, var(--bg-elev-1));
  }
  .set-model-meta { flex: 1 1 auto; min-width: 0; }
  .set-model-name { font-size: var(--fs-sm); color: var(--fg); font-weight: 500; }
  .set-model-sub { font-size: 10px; color: var(--fg-muted); margin-top: 2px; }
  .set-model-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .set-progress {
    margin-top: 8px;
    height: 4px;
    background: var(--bg-elev-2);
    border-radius: 2px;
    overflow: hidden;
  }
  .set-progress-fill {
    height: 100%;
    background: var(--accent);
    transition: width 120ms linear;
  }
  .set-progress-label { font-size: 10px; margin-top: 3px; }

  .set-mic-r { width: 100%; max-width: 360px; }
  .set-mic-select { flex: 1 1 auto; }

  /* ─── Network: server list (lives inside set-group) ─── */
  .srv-list { display: flex; flex-direction: column; }
  .srv-card {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: 16px;
    padding: 12px 14px;
    background: transparent;
    border: 0;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    color: var(--fg);
    font: inherit;
    text-align: left;
    transition: background 100ms;
  }
  .srv-card:last-child { border-bottom: none; }
  .srv-card:hover { background: var(--surface-hover); }
  .srv-card[data-active="true"] {
    background: color-mix(in oklch, var(--accent) 10%, transparent);
    box-shadow: inset 2px 0 var(--accent);
  }
  .srv-card[data-active="true"]:hover {
    background: color-mix(in oklch, var(--accent) 14%, transparent);
  }
  .srv-l { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .srv-meta-l { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .srv-dot {
    width: 9px; height: 9px; border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 18%, transparent);
    flex-shrink: 0;
  }
  .srv-name { font-weight: 600; font-size: var(--fs-sm); }
  .srv-host { font-size: var(--fs-xs); }
  .srv-meta {
    font-size: var(--fs-xs);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    max-width: 240px;
  }
  .srv-r { display: flex; gap: 4px; }

  /* ─── Empty state inside a group ─── */
  .set-empty {
    padding: 28px 16px;
    display: flex; flex-direction: column; gap: 4px;
    align-items: center; text-align: center;
  }
  .set-empty-title { color: var(--fg); font-size: var(--fs-sm); font-weight: 600; }
  .set-empty-hint { color: var(--fg-muted); font-size: var(--fs-xs); max-width: 280px; line-height: 1.45; }

  /* ─── Keyboard shortcut table (Appearance) ─── */
  .kbd-grid {
    display: grid;
    grid-template-columns: 1fr;
    padding: 4px 0;
  }
  .kbd-row {
    display: grid;
    grid-template-columns: 160px 1fr;
    align-items: center;
    gap: 14px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
  }
  .kbd-row:last-child { border-bottom: none; }
  .kbd-combo {
    display: inline-flex; align-items: center; gap: 4px;
    flex-wrap: wrap;
  }
  .kbd-plus { color: var(--fg-faint); font-size: 10px; }
  .kbd-label { font-size: var(--fs-sm); color: var(--fg-2); }

  /* ─── Pill primitive shadowed locally for status tone variants ─── */
  /* Inherits .pill / .dot from app.css; only need scoped tweaks for the dim
     muted variant inside set-rows so the dot doesn't pulse. */
  .set-row :global(.pill) {
    height: 22px;
    padding: 0 10px;
  }
</style>
