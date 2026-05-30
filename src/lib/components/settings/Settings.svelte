<script lang="ts">
  import { untrack } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ServerProfile } from "../../state/connection.svelte";
  import { Cog, Server, Key, Info, Plus, Pencil, Trash2, RefreshCw, Sparkles, Palette, ChevronDown, FolderOpen, Copy, Check, Eye, EyeOff, X, Mic, Accessibility as A11yIcon } from "lucide-svelte";
  import { appConfigDir, appLogDir } from "@tauri-apps/api/path";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { updates } from "../../state/updates.svelte";
  import { assistant as assistantStore } from "../../state/assistant.svelte";
  import { stt } from "../../state/stt.svelte";
  import { accessibility } from "../../state/accessibility.svelte";
  import { commandPalette } from "../../state/command-palette.svelte";
  import { scrubUser } from "$lib/util/redact";

  import { tooltip } from "$lib/actions/tooltip";
  // SSH keys merged into Network section as a header action — one fewer nav row,
  // since the keys panel was just a single button anyway.
  type Section = "appearance" | "accessibility" | "assistant" | "speech" | "network" | "about";

  let { initialSection = "appearance", onAddServer, onEditServer, onDeleteServer, onLaunchKeygen }: {
    initialSection?: Section;
    onAddServer: () => void;
    onEditServer: (s: ServerProfile) => void;
    onDeleteServer: (s: ServerProfile) => void;
    onLaunchKeygen: () => void;
  } = $props();

  let section = $state<Section>(untrack(() => initialSection));

  // Command-palette deep-link: when CP requests a section, pull to it and
  // clear the request so it's a one-shot.
  $effect(() => {
    const req = commandPalette.targetSettingsSection;
    if (req) {
      section = req;
      commandPalette.clearSettingsSection();
    }
  });
  let appVersion = $state("?");
  let configDir = $state<string>("");
  let logDir = $state<string>("");
  let sshKeyConfigured = $state<boolean | null>(null);
  let sshKeyPath = $state<string | null>(null);

  async function refreshSshKeyStatus() {
    try {
      const exists = await invoke<boolean>("default_ssh_key_exists");
      sshKeyConfigured = exists;
      sshKeyPath = exists ? await invoke<string | null>("default_ssh_key_path") : null;
    } catch (e) {
      console.debug("ssh key status probe failed:", e);
      sshKeyConfigured = null;
    }
  }
  let diagCopied = $state(false);
  let diagCopiedTimer: ReturnType<typeof setTimeout> | null = null;

  async function loadAboutPaths() {
    try { configDir = await appConfigDir(); } catch (e) { console.warn("appConfigDir failed", e); }
    try { logDir = await appLogDir(); } catch (e) { console.warn("appLogDir failed", e); }
  }
  async function openDir(p: string) {
    if (!p) return;
    try { await openPath(p); } catch (e) { console.error("openPath failed", e); }
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
      if (diagCopiedTimer) clearTimeout(diagCopiedTimer);
      diagCopiedTimer = setTimeout(() => {
        diagCopied = false;
        diagCopiedTimer = null;
      }, 1400);
    } catch (e) { console.error("clipboard failed", e); }
  }

  const sections: { id: Section; label: string; icon: typeof Cog; subtitle: string }[] = [
    { id: "appearance",    label: "Appearance",    icon: Palette,         subtitle: "Layout, shortcuts, and interface preferences." },
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
  // Save-button dirty gates (P6) — disabled until the field actually differs
  // from the stored value, so "no pending edit" is visually distinct.
  const asstMaxBudgetDirty = $derived(asstMaxBudgetDraft !== assistantStore.maxBudgetUsd);
  const asstApiKeyDirty = $derived(asstApiKeyDraft.trim().length > 0);
  // CLI-session probe freshness (P5). Tick every 30s while the section is open
  // so "Checked Xm ago" stays current without a render-on-every-frame cost.
  let asstNowTick = $state(Date.now());
  $effect(() => {
    if (section !== "assistant") return;
    asstNowTick = Date.now();
    const iv = setInterval(() => { asstNowTick = Date.now(); }, 30_000);
    return () => clearInterval(iv);
  });
  function fmtAgo(ts: number, now: number): string {
    const s = Math.max(0, Math.round((now - ts) / 1000));
    if (s < 10) return "just now";
    if (s < 60) return `${s}s ago`;
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    return `${Math.round(m / 60)}h ago`;
  }
  $effect(() => {
    if (section !== "assistant") return;
    untrack(() => {
      void assistantStore.init().then(() => {
        // Phase 6 (#37): never prefill the draft with a stored value — the
        // renderer no longer receives it. Draft holds only what the user just typed.
        asstApiKeyDraft = "";
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
      asstMaxBudgetMsg = assistantStore.maxBudgetUsd != null ? `Saved: ${assistantStore.maxBudgetUsd.toFixed(2)} cap.` : "Cleared (no cap).";
    } catch (e) {
      asstMaxBudgetMsg = `Failed: ${e}`;
    } finally {
      asstMaxBudgetSaving = false;
    }
  }

  $effect(() => {
    void (async () => {
      try { appVersion = await invoke<string>("app_version"); } catch {}
      // Servers list is shared connection state; skip the refetch if another
      // mount already populated it. Settings opens often enough for this to matter.
      if (untrack(() => connection.servers.length) === 0) {
        await connection.loadServers();
      }
      await refreshSshKeyStatus();
    })();
  });

  // Lazy-load About paths only when the About section is visible.
  let aboutPathsLoaded = false;
  $effect(() => {
    if (section === "about" && !aboutPathsLoaded) {
      aboutPathsLoaded = true;
      void loadAboutPaths();
    }
  });

  $effect(() => {
    return () => {
      if (diagCopiedTimer) { clearTimeout(diagCopiedTimer); diagCopiedTimer = null; }
    };
  });

  async function pickServer(s: ServerProfile) {
    await connection.select(s.key);
  }

  let rotatedToken = $state<{ key: string; token: string } | null>(null);
  let rotatedTokenCopied = $state(false);

  async function onUnpinFingerprint(s: ServerProfile) {
    if (!s.fingerprint) return;
    const ok = confirm(
      `Clear pinned fingerprint for "${s.name}"?\n\nNext connect will re-prompt to trust the host key. Only do this after a deliberate server re-key.`
    );
    if (!ok) return;
    try {
      await invoke("clear_server_fingerprint", { serverKey: s.key });
      await connection.loadServers();
    } catch (e) {
      alert(`Failed to clear fingerprint: ${e}`);
    }
  }

  async function onRotateBridgeToken(s: ServerProfile) {
    const ok = confirm(
      `Rotate bridge token for "${s.name}"?\n\nThe new value will be shown ONCE — copy it into your remote bridge config (RIFT_BRIDGE_TOKEN env var) before closing. The old token stops working immediately.`
    );
    if (!ok) return;
    try {
      const token = await invoke<string>("rotate_bridge_token", { serverKey: s.key });
      rotatedToken = { key: s.key, token };
      rotatedTokenCopied = false;
      await connection.loadServers();
    } catch (e) {
      alert(`Failed to rotate token: ${e}`);
    }
  }

  async function onCopyRotatedToken() {
    if (!rotatedToken) return;
    try {
      await navigator.clipboard.writeText(rotatedToken.token);
      rotatedTokenCopied = true;
      setTimeout(() => { rotatedTokenCopied = false; }, 1800);
    } catch (e) {
      console.error("clipboard failed", e);
    }
  }

  const SHORTCUTS: { combo: string[]; label: string }[] = [
    { combo: ["Ctrl", "P"],         label: "Command palette" },
    { combo: ["Ctrl", "K"],         label: "Command palette (alias)" },
    { combo: ["Ctrl", "T"],         label: "New chat tab" },
    { combo: ["Ctrl", "W"],         label: "Close active tab" },
    { combo: ["Ctrl", "Tab"],       label: "Cycle tabs (Shift to reverse)" },
    { combo: ["Alt", "1…9"],        label: "Jump to chat tab N" },
    { combo: ["Ctrl", "1…5"],       label: "Switch workspace" },
    { combo: ["Ctrl", "0"],         label: "Switch to Chat workspace" },
    { combo: ["Ctrl", ","],         label: "Open Settings" },
    { combo: ["Ctrl", "\\"],        label: "Split chat pane" },
  ];
</script>

<div class="settings">
  <nav class="settings-nav">
    {#each sections as s (s.id)}
      {@const Icon = s.icon}
      <button
        data-active={section === s.id}
        aria-current={section === s.id ? "page" : undefined}
        onclick={() => (section = s.id)}
        type="button"
      >
        <Icon size={14} strokeWidth={1.75}/><span>{s.label}</span>
      </button>
    {/each}
  </nav>

  <div class="settings-body">
    {#key section}
      <div
        class="section-shell"
        in:fly={{ y: 6, duration: 180, delay: 80, easing: quintOut }}
        out:fade={{ duration: 0 }}
      >
        <header class="section-head">
          <div class="section-head-l">
            <h2>{currentSection.label}</h2>
            <p class="section-sub">{currentSection.subtitle}</p>
          </div>
          {#if section === "network"}
            <div class="section-head-r">
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
                <span class="set-hint">Click an icon on the left-edge activity bar to swap the main pane. Drag icons to reorder — the Ctrl+1…5 shortcuts follow the bar's order.</span>
              </div>
              <div class="set-row-r">
                <button class="btn danger sm" type="button" onclick={() => void assistantStore.closeAllTabs()} use:tooltip={"Close every chat tab in this session"}>
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

          <section class="set-group theme-slim">
            <header class="set-group-head">
              <span>Theme</span>
              <span class="theme-pill pill muted"><span class="dot"></span>Dark · Linear-precise</span>
            </header>
            <div class="set-row-note">
              <span class="set-note">Rift is dark-only — density, accent tint, and light-mode tokens land in a future build.</span>
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
                {#if assistantStore.authLastProbed && !assistantStore.authChecking}
                  <span class="set-stamp mono" use:tooltip={"Time since the last CLI session probe"}>Checked {fmtAgo(assistantStore.authLastProbed, asstNowTick)}</span>
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
                <span class="set-hint">Admits MCP tools from your global Claude config (<code>~/.claude/.mcp.json</code>). Disable to restrict tool surface to Rift's built-ins only.</span>
              </div>
              <div class="set-row-r">
                <button
                  type="button"
                  class="switch"
                  role="switch"
                  aria-label="Use full Claude Code config"
                  aria-checked={assistantStore.useFullConfig && !assistantStore.hasApiKey}
                  data-on={assistantStore.useFullConfig && !assistantStore.hasApiKey}
                  disabled={assistantStore.hasApiKey}
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
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Git tools</span>
                <span class="set-hint">Local <code>git</code> tools for the model. Read-only = status, diff, log. Standard adds commit, pull, and push (auth via your system git/SSH). Enabling “Allow remote shell” grants Standard automatically.</span>
              </div>
              <div class="set-row-r">
                <div class="seg" role="radiogroup" aria-label="Git tools trust level">
                  <button
                    type="button"
                    role="radio"
                    aria-checked={assistantStore.trustLevel === "readonly"}
                    data-active={assistantStore.trustLevel === "readonly"}
                    onclick={() => void assistantStore.setTrustLevel("readonly")}
                  >Read-only</button>
                  <button
                    type="button"
                    role="radio"
                    aria-checked={assistantStore.trustLevel !== "readonly"}
                    data-active={assistantStore.trustLevel !== "readonly"}
                    onclick={() => void assistantStore.setTrustLevel("standard")}
                  >Standard</button>
                </div>
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
                <button class="btn primary sm" type="button" onclick={saveAsstMaxBudget} disabled={asstMaxBudgetSaving || !asstMaxBudgetDirty}>
                  {asstMaxBudgetSaving ? "Saving…" : "Save"}
                </button>
                {#if assistantStore.maxBudgetUsd != null}
                  <button
                    class="btn ghost sm"
                    type="button"
                    disabled={asstMaxBudgetSaving}
                    onclick={() => { asstMaxBudgetDraft = null; void saveAsstMaxBudget(); }}
                  >
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
                <span class="set-hint">Pay-per-token via console.anthropic.com. When set, overrides the CLI session (forces <code>--bare</code>). Stored in the OS keychain (Windows Credential Manager).</span>
              </div>
              <div class="set-row-r">
                {#if assistantStore.hasApiKey}
                  <span class="pill ok"><span class="dot"></span>Configured</span>
                  <button
                    class="btn ghost sm"
                    type="button"
                    disabled={asstApiKeySaving}
                    onclick={() => { asstApiKeyDraft = ""; void saveAsstApiKey(); }}
                  >
                    Clear
                  </button>
                {:else}
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
                      use:tooltip={asstApiKeyVisible ? "Hide API key" : "Show API key"}
                    >
                      {#if asstApiKeyVisible}<EyeOff size={13}/>{:else}<Eye size={13}/>{/if}
                    </button>
                  </div>
                  <button class="btn primary sm" type="button" onclick={saveAsstApiKey} disabled={asstApiKeySaving || !asstApiKeyDirty}>
                    {asstApiKeySaving ? "Saving…" : "Save"}
                  </button>
                {/if}
              </div>
            </div>
            {#if asstApiKeyMsg}
              <div class="set-row set-row-note"><span class="set-note">{asstApiKeyMsg}</span></div>
            {/if}
            {#if assistantStore.auth?.envApiKeyPresent && !assistantStore.auth?.apiKeyConfigured}
              <div class="set-row set-row-note">
                <span class="set-note">⚠ A system <code>ANTHROPIC_API_KEY</code> environment variable is set, but Rift ignores env keys so it can't silently override your login. To use that key, paste it into the field above; otherwise remove it from your environment to avoid confusion.</span>
              </div>
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
                  disabled={assistantStore.activeTab?.compactingNow}
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
                <div class="set-pick-grid" role="radiogroup" aria-label="Speech recognition language">
                  {#each STT_LANGS as l (l.id)}
                    <button
                      type="button"
                      role="radio"
                      aria-checked={stt.config.language === l.id}
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
                        <button type="button" class="btn ghost sm" onclick={() => void stt.deleteModel(m.id)} use:tooltip={"Delete model"} aria-label="Delete">
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
                  <button type="button" class="btn ghost sm" onclick={() => void stt.refreshInputDevices()} use:tooltip={"Refresh device list"} aria-label="Refresh">
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
                  <div class="srv-card" data-active={connection.selectedKey === s.key}>
                    <button
                      type="button"
                      class="srv-select"
                      onclick={() => pickServer(s)}
                      aria-label={`Select server ${s.name}`}
                      aria-pressed={connection.selectedKey === s.key}
                    >
                      <div class="srv-l">
                        <span class="srv-dot"></span>
                        <div class="srv-meta-l">
                          <div class="mono srv-name">{s.name}</div>
                          <div class="mono dim srv-host">{s.user}@{s.host}{s.port !== 22 ? `:${s.port}` : ""}</div>
                        </div>
                      </div>
                      <div class="srv-meta mono dim" use:tooltip={s.fingerprint ?? "no fingerprint pinned"}>
                        {s.fingerprint ? `ed25519 · ${s.fingerprint.slice(0, 18)}…` : "no fingerprint pinned"}
                      </div>
                    </button>
                    <div class="srv-r">
                      {#if s.fingerprint}
                        <button class="btn ghost sm" onclick={() => onUnpinFingerprint(s)} type="button" use:tooltip={`Clear pinned fingerprint for ${s.name}`} aria-label={`Unpin fingerprint for ${s.name}`}>
                          <X size={11}/>
                        </button>
                      {/if}
                      {#if s.hasBridgeToken}
                        <button class="btn ghost sm" onclick={() => onRotateBridgeToken(s)} type="button" use:tooltip={`Rotate bridge token for ${s.name}`} aria-label={`Rotate bridge token for ${s.name}`}>
                          <RefreshCw size={11}/>
                        </button>
                      {/if}
                      <button class="btn ghost sm" onclick={() => onEditServer(s)} type="button" use:tooltip={`Edit ${s.name}`} aria-label={`Edit server ${s.name}`}>
                        <Pencil size={11}/>
                      </button>
                      <button class="btn ghost sm" onclick={() => onDeleteServer(s)} type="button" use:tooltip={`Delete ${s.name}`} aria-label={`Delete server ${s.name}`}>
                        <Trash2 size={11}/>
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </section>

          <section class="set-group">
            <header class="set-group-head">SSH private key</header>
            <div class="set-row">
              <div class="set-row-l">
                <span class="set-label">Default keypair</span>
                {#if sshKeyConfigured === true && sshKeyPath}
                  <span class="set-hint">
                    <Check size={11} style="color: var(--success, oklch(0.78 0.16 145)); vertical-align: -1px;"/>
                    Configured · <code class="mono">{sshKeyPath}</code>
                  </span>
                {:else if sshKeyConfigured === false}
                  <span class="set-hint">
                    Not configured. Generate or import a private key, then copy the public half into the server's <code>authorized_keys</code>.
                  </span>
                {:else}
                  <span class="set-hint">ed25519 keypair stored at <code>%APPDATA%/Rift/keys/</code>.</span>
                {/if}
              </div>
              <div class="set-row-r">
                <button class="btn ghost sm" onclick={() => { onLaunchKeygen(); void refreshSshKeyStatus(); }} type="button">
                  <Key size={11}/> {sshKeyConfigured ? "Manage key" : "Set up key"}
                </button>
              </div>
            </div>
          </section>

          {#if rotatedToken}
            <section class="set-group">
              <header class="set-group-head">New bridge token — copy now</header>
              <div class="set-row">
                <div class="set-row-l">
                  <span class="set-label">{connection.servers.find(s => s.key === rotatedToken!.key)?.name ?? rotatedToken.key}</span>
                  <span class="set-hint">Paste this into the remote bridge's <code>RIFT_BRIDGE_TOKEN</code> env var. Once you close this panel the token is no longer shown.</span>
                </div>
                <div class="set-row-r" style="display:flex; gap:6px; align-items:center;">
                  <code class="mono" style="font-size:11px; padding:4px 6px; background:var(--bg-elev-2); border-radius:var(--radius-xs); user-select:all;">{rotatedToken.token}</code>
                  <button class="btn ghost sm" onclick={onCopyRotatedToken} type="button" use:tooltip={"Copy token"}>
                    {#if rotatedTokenCopied}<Check size={11}/>{:else}<Copy size={11}/>{/if}
                  </button>
                  <button class="btn ghost sm" onclick={() => (rotatedToken = null)} type="button" use:tooltip={"Dismiss"}>
                    <X size={11}/>
                  </button>
                </div>
              </div>
            </section>
          {/if}

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
                <span class="set-hint mono path-val" use:tooltip={configDir}>{configDir || "—"}</span>
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
                <span class="set-hint mono path-val" use:tooltip={logDir}>{logDir || "—"}</span>
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
    box-shadow: 0 0 8px color-mix(in oklch, var(--accent) 50%, transparent);
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
  .set-stamp {
    font-size: 10.5px;
    color: var(--fg-faint);
    white-space: nowrap;
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
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 16px;
    padding: 0;
    background: transparent;
    border: 0;
    border-bottom: 1px solid var(--border);
    color: var(--fg);
    transition: background 100ms;
  }
  .srv-card:last-child { border-bottom: none; }
  .srv-card:hover { background: var(--surface-hover); }
  .srv-card[data-active="true"] {
    background: color-mix(in oklch, var(--accent) 10%, transparent);
    box-shadow: inset 2px 0 var(--accent);
  }
  .srv-select {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 16px;
    padding: 12px 14px;
    background: transparent;
    border: 0;
    cursor: pointer;
    color: inherit;
    font: inherit;
    text-align: left;
    width: 100%;
    min-width: 0;
  }
  .srv-select:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--ring);
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
    /* Two columns on wide settings panes, single column when narrower. Halves
       vertical scroll for the shortcut reference while keeping each row
       readable. */
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    column-gap: 0;
    padding: 4px 0;
  }
  .kbd-row {
    display: grid;
    grid-template-columns: 140px 1fr;
    align-items: center;
    gap: 12px;
    padding: 7px 14px;
    border-bottom: 1px solid var(--border);
  }
  .kbd-row:last-child { border-bottom: none; }
  /* When 2 columns are active, the second-to-last row also needs to lose its
     bottom border since the visual "last" depends on layout. Cheap heuristic:
     only the genuinely-last DOM row keeps the border-bottom; for an even row
     count + 2-col layout, the visual pairing handles itself. */
  @media (min-width: 720px) {
    .kbd-row:nth-last-child(2):nth-child(even) { border-bottom: none; }
  }
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

  /* Theme card — collapsed into a single-row header w/ the dark-mode pill
     inline. The hint moves into a set-row-note below the head. Saves ~50px
     vertical for a card that's purely informational until light mode lands. */
  .theme-slim .set-group-head {
    display: flex; align-items: center; justify-content: space-between;
  }
  .theme-slim .theme-pill {
    text-transform: none;
    letter-spacing: 0;
    font-weight: 500;
  }
</style>
