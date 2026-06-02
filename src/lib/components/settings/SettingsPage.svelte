<script lang="ts">
  import { untrack, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { connection, type ServerProfile } from "../../state/connection.svelte";
  import {
    Cog, Server, Key, Info, Plus, Pencil, Trash2, RefreshCw, Sparkles, Palette,
    FolderOpen, Copy, Check, Eye, EyeOff, X, Mic, Accessibility as A11yIcon,
    CircleCheck, IdCard, Terminal, RotateCcw,
  } from "lucide-svelte";
  import { appConfigDir, appLogDir } from "@tauri-apps/api/path";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { updates } from "../../state/updates.svelte";
  import { assistant as assistantStore } from "../../state/assistant.svelte";
  import { stt } from "../../state/stt.svelte";
  import { accessibility } from "../../state/accessibility.svelte";
  import { commandPalette } from "../../state/command-palette.svelte";
  import { uiPrefs, ACCENTS } from "../../state/ui-prefs.svelte";
  import { dialogs } from "../../state/dialogs.svelte";
  import { onboarding } from "../../state/onboarding.svelte";
  import { syncPage } from "../../state/sync-page.svelte";
  import { scrubUser } from "$lib/utils/redact";
  import { tooltip } from "$lib/actions/tooltip";
  import Select from "../Select.svelte";

  const DENSITIES = ["compact", "regular", "comfy"] as const;
  const PRESENCES = ["calm", "bold"] as const;

  // Scroll-spy section ids. `network` kept (not "server") so the command-palette
  // deep-link channel (requestSettingsSection) keeps working untouched.
  type Section = "general" | "appearance" | "accessibility" | "assistant" | "speech" | "network" | "about";
  const ST_SECTIONS: { id: Section; label: string; icon: typeof Cog; sub: string; dot?: "ok" | "warn" }[] = [
    { id: "general",       label: "General",       icon: IdCard,   sub: "Workspace identity, on-machine behavior, and first-run." },
    { id: "appearance",    label: "Appearance",    icon: Palette,  sub: "Theme color, density, code preview, and keyboard shortcuts — applied instantly across Rift." },
    { id: "accessibility", label: "Accessibility", icon: A11yIcon, sub: "Reading-comfort options for the Assistant chat." },
    { id: "assistant",     label: "Assistant",     icon: Sparkles, sub: "Your Claude session, per-turn cost guard, and conversation compaction." },
    { id: "speech",        label: "Speech",        icon: Mic,      sub: "Voice-to-text input. Web Speech (online) or Whisper (local, accent-tuned)." },
    { id: "network",       label: "Server",        icon: Server,   sub: "SSH server profiles, key management, and fingerprint / bridge-token pinning." },
    { id: "about",         label: "About",         icon: Info,     sub: "Build info, file paths, and support diagnostics." },
  ];

  let activeSec = $state<Section>("general");
  let scrollEl = $state<HTMLDivElement>();
  // Per-section anchor elements for scroll-spy + jump().
  let secEls = $state<Partial<Record<Section, HTMLElement>>>({});

  function onScroll() {
    const sc = scrollEl;
    if (!sc) return;
    const scTop = sc.getBoundingClientRect().top;
    let cur: Section = ST_SECTIONS[0].id;
    for (const s of ST_SECTIONS) {
      const el = secEls[s.id];
      if (el && el.getBoundingClientRect().top - scTop <= 140) cur = s.id;
    }
    activeSec = cur;
  }
  function jump(id: Section) {
    const el = secEls[id];
    const sc = scrollEl;
    if (!el || !sc) return;
    const delta = el.getBoundingClientRect().top - sc.getBoundingClientRect().top;
    sc.scrollTo({ top: sc.scrollTop + delta - 24, behavior: "smooth" });
  }

  // Command-palette deep-link: pull to the requested section, then clear (one-shot).
  $effect(() => {
    const req = commandPalette.targetSettingsSection;
    if (req) {
      activeSec = req;
      // wait a frame so anchors are mounted before scrolling
      requestAnimationFrame(() => jump(req));
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
      diagCopiedTimer = setTimeout(() => { diagCopied = false; diagCopiedTimer = null; }, 1400);
    } catch (e) { console.error("clipboard failed", e); }
  }

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
  const STT_ENGINES: { id: "web_speech" | "whisper"; label: string; sub: string }[] = [
    { id: "web_speech", label: "Web Speech", sub: "Edge · Azure when online" },
    { id: "whisper",    label: "Whisper",    sub: "On-device · accent-tuned" },
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
  const asstMaxBudgetDirty = $derived(asstMaxBudgetDraft !== assistantStore.maxBudgetUsd);
  const asstApiKeyDirty = $derived(asstApiKeyDraft.trim().length > 0);

  let asstNowTick = $state(Date.now());
  function fmtAgo(ts: number, now: number): string {
    const s = Math.max(0, Math.round((now - ts) / 1000));
    if (s < 10) return "just now";
    if (s < 60) return `${s}s ago`;
    const m = Math.round(s / 60);
    if (m < 60) return `${m}m ago`;
    return `${Math.round(m / 60)}h ago`;
  }

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

  async function pickServer(s: ServerProfile) {
    await connection.select(s.key);
  }

  let rotatedToken = $state<{ key: string; token: string } | null>(null);
  let rotatedTokenCopied = $state(false);

  // ── RCON console state ──
  type RconKind = "cmd" | "ok" | "warn" | "sys";
  type RconLine = { time: string; msg: string; kind: RconKind };
  let rconConfigured = $state(false);
  let rconStatus = $state<"idle" | "checking" | "online" | "offline">("idle");
  let rconPassword = $state("");
  let rconShowPw = $state(false);
  let rconLog = $state<RconLine[]>([]);
  let rconInput = $state("");
  let rconBusy = $state(false);
  const rconServerKey = $derived(connection.selected?.key ?? null);

  function rconTime(): string {
    return new Date().toLocaleTimeString([], { hour12: true });
  }
  function rconPushLine(msg: string, kind: RconKind) {
    rconLog = [...rconLog, { time: rconTime(), msg, kind }].slice(-200);
  }

  async function rconSavePassword() {
    if (!rconServerKey) return;
    try {
      await invoke("rcon_set_password", { serverKey: rconServerKey, password: rconPassword });
      rconConfigured = true;
      rconPushLine("Password saved.", "sys");
    } catch (e) {
      rconPushLine(`Save failed: ${String(e)}`, "warn");
    }
  }
  async function rconClearPassword() {
    if (!rconServerKey) return;
    try {
      await invoke("rcon_set_password", { serverKey: rconServerKey, password: "" });
      rconConfigured = false;
      rconPassword = "";
      rconPushLine("Password cleared.", "sys");
    } catch (e) {
      rconPushLine(`Clear failed: ${String(e)}`, "warn");
    }
  }

  async function runRcon(cmd: string) {
    if (!rconServerKey || !cmd.trim()) return;
    rconPushLine("❯ " + cmd, "cmd");
    rconBusy = true;
    rconStatus = "checking";
    try {
      const out = await invoke<string>("rcon_send", { serverKey: rconServerKey, command: cmd });
      rconPushLine(out || "(no output)", "ok");
      rconStatus = "online";
    } catch (e) {
      rconPushLine(String(e), "warn");
      rconStatus = "offline";
    } finally {
      rconBusy = false;
    }
    rconInput = "";
  }

  // $effect: probe rcon_has_password when selected server changes
  $effect(() => {
    const key = rconServerKey;
    if (!key) { rconConfigured = false; rconStatus = "idle"; return; }
    invoke<boolean>("rcon_has_password", { serverKey: key }).then((has) => {
      rconConfigured = has;
    }).catch(() => { rconConfigured = false; });
  });

  // $effect: auto-reconnect ping (15s interval, leak-free)
  $effect(() => {
    if (!uiPrefs.rconAutoReconnect || !rconConfigured || !rconServerKey) return;
    const iv = setInterval(() => {
      if (!rconBusy) void invoke<string>("rcon_send", { serverKey: rconServerKey!, command: "version" }).then(() => { rconStatus = "online"; }).catch(() => { rconStatus = "offline"; });
    }, 15000);
    return () => clearInterval(iv);
  });

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
    { combo: ["Ctrl", "P"],   label: "Command palette" },
    { combo: ["Ctrl", "K"],   label: "Command palette (alias)" },
    { combo: ["Ctrl", "T"],   label: "New chat tab" },
    { combo: ["Ctrl", "W"],   label: "Close active tab" },
    { combo: ["Ctrl", "Tab"], label: "Cycle tabs (Shift to reverse)" },
    { combo: ["Alt", "1…9"],  label: "Jump to chat tab N" },
    { combo: ["Ctrl", "1…6"], label: "Switch workspace" },
    { combo: ["Ctrl", "0"],   label: "Switch to Chat workspace" },
    { combo: ["Ctrl", ","],   label: "Open Settings" },
    { combo: ["Ctrl", "\\"],  label: "Split chat pane" },
  ];

  // The connection status feeds the index dot for the Server section.
  // "connected" = the sync watcher is live (watching / idle / syncing).
  const watcherOn = $derived(
    connection.status?.state === "watching" || connection.status?.state === "idle" || connection.status?.state === "syncing"
  );
  const serverDot = $derived<"ok" | "warn" | undefined>(
    watcherOn ? "ok"
      : connection.status?.state === "error" ? "warn"
      : undefined
  );
  const assistantDot = $derived<"ok" | "warn" | undefined>(
    assistantStore.auth?.pill === "green" ? "ok"
      : assistantStore.auth?.pill === "yellow" || assistantStore.auth?.pill === "red" ? "warn" : undefined
  );

  // ── one-time loads on mount (single-scroll: every section is live) ──
  onMount(() => {
    void (async () => {
      try { appVersion = await invoke<string>("app_version"); } catch {}
      if (untrack(() => connection.servers.length) === 0) await connection.loadServers();
      await refreshSshKeyStatus();
    })();
    void assistantStore.init().then(() => {
      asstApiKeyDraft = "";
      asstMaxBudgetDraft = assistantStore.maxBudgetUsd;
    });
    void stt.init();
    void loadAboutPaths();

    asstNowTick = Date.now();
    const iv = setInterval(() => { asstNowTick = Date.now(); }, 30_000);
    return () => {
      clearInterval(iv);
      if (diagCopiedTimer) { clearTimeout(diagCopiedTimer); diagCopiedTimer = null; }
    };
  });
</script>

<div class="st-main">
  <div class="st-inner">
    <!-- ── Sticky section index ── -->
    <nav class="st-index">
      <div class="st-index-head">
        <div class="st-index-title">Settings</div>
        <div class="st-index-sub">
          Workspace · <span class="mono">{connection.selected?.name ?? "no server"}</span>
        </div>
      </div>
      <div class="st-index-list">
        {#each ST_SECTIONS as s (s.id)}
          {@const Icon = s.icon}
          {@const dot = s.id === "network" ? serverDot : s.id === "assistant" ? assistantDot : s.dot}
          <button class="st-index-row" class:on={activeSec === s.id} onclick={() => jump(s.id)} type="button">
            <span class="st-index-ic"><Icon size={16} strokeWidth={1.75} /></span>
            <span class="lbl">{s.label}</span>
            {#if dot}<span class="st-index-dot {dot}"></span>{/if}
          </button>
        {/each}
      </div>
      <div class="st-index-foot">
        <button class="st-version" type="button" onclick={() => updates.open()} use:tooltip={"Check for updates"}>
          <CircleCheck size={16} />
          <div style="flex:1; min-width:0; text-align:left;">
            <div class="st-version-t">Rift</div>
            <div class="st-version-s">{appVersion} · up to date</div>
          </div>
          <RefreshCw size={13} />
        </button>
      </div>
    </nav>

    <!-- ── Scroll document ── -->
    <div class="st-scroll" bind:this={scrollEl} onscroll={onScroll}>
      <div class="st-doc">

        <!-- ── GENERAL ── -->
        <section class="st-sec" bind:this={secEls.general}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><IdCard size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">General</div><div class="st-sec-sub">{ST_SECTIONS[0].sub}</div></div>
          </div>

          <!-- Workspace identity -->
          <div class="st-block">
            <div class="st-block-label">Workspace</div>
            <div class="st-card">
              <div class="st-account">
                <div class="st-avatar">{(connection.selected?.name?.[0] ?? "R").toUpperCase()}</div>
                <div class="st-account-body">
                  <div class="st-account-name">{connection.selected?.name ?? "No workspace"}</div>
                  <div class="st-account-mail">{connection.selected?.remoteRoot ?? "—"}</div>
                </div>
                <div style="display:flex; align-items:center; gap:9px; flex:none;">
                  {#if watcherOn}
                    <span class="st-conn-pill up"><span class="dot"></span>Connected</span>
                  {:else}
                    <span class="st-conn-pill down"><span class="dot"></span>Offline</span>
                  {/if}
                  <button class="st-btn" type="button" onclick={() => jump("network")}><Server size={14} /> Switch</button>
                </div>
              </div>
            </div>
          </div>

          <!-- On-machine behavior toggles -->
          <div class="st-block">
            <div class="st-block-label">On-machine behavior</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Launch at login</div>
                  <div class="st-row-desc">Records your intent to launch Rift at login. No OS-level startup entry is created — enable your OS's startup mechanism separately.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={uiPrefs.launchAtLogin} role="switch" aria-checked={uiPrefs.launchAtLogin} aria-label="Launch at login" type="button" onclick={() => uiPrefs.toggleLaunchAtLogin()}></button>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Restore last session</div>
                  <div class="st-row-desc">Stores your preference to reopen the last-used workspace on next launch. Honoured when the stored server profile is still present.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={uiPrefs.restoreSession} role="switch" aria-checked={uiPrefs.restoreSession} aria-label="Restore last session" type="button" onclick={() => uiPrefs.toggleRestoreSession()}></button>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Confirm before quitting</div>
                  <div class="st-row-desc">Stores your preference for a quit-confirmation prompt. The actual prompt requires Tauri window-close handling wired to this flag.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={uiPrefs.confirmOnQuit} role="switch" aria-checked={uiPrefs.confirmOnQuit} aria-label="Confirm before quitting" type="button" onclick={() => uiPrefs.toggleConfirmOnQuit()}></button>
                </div>
              </div>
            </div>
          </div>

          <!-- First-run replay -->
          <div class="st-block">
            <div class="st-block-label">First run</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Replay first-run walkthrough</div>
                  <div class="st-row-desc">Re-arms the onboarding gate. The walkthrough re-shows on next launch if its other conditions are met (no connected server, no SSH key).</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-btn" type="button" onclick={() => onboarding.reset()}><RotateCcw size={14} /> Replay first-run</button>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- ── APPEARANCE ── -->
        <section class="st-sec" bind:this={secEls.appearance}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><Palette size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">Appearance</div><div class="st-sec-sub">{ST_SECTIONS[1].sub}</div></div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Accent color</div>
            <div class="st-card">
              <div class="st-swatch-grid">
                {#each ACCENTS as a (a.id)}
                  <button
                    class="st-swatch" class:on={uiPrefs.accentHue === a.hue}
                    style="--sw: oklch(0.78 0.15 {a.hue})" type="button"
                    onclick={() => uiPrefs.setAccentHue(a.hue)}
                    aria-pressed={uiPrefs.accentHue === a.hue} use:tooltip={a.label}
                  >
                    <span class="st-swatch-chip">{#if uiPrefs.accentHue === a.hue}<Check size={15} strokeWidth={3} />{/if}</span>
                    <span class="st-swatch-label">{a.label}</span>
                  </button>
                {/each}
              </div>
            </div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Interface</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Accent presence</div>
                  <div class="st-row-desc">How strongly the accent tints panels, highlights, and selected rows.</div>
                </div>
                <div class="st-row-ctl">
                  <div class="st-seg">
                    {#each PRESENCES as p (p)}
                      <button class="st-seg-btn" class:on={uiPrefs.presence === p} type="button" onclick={() => uiPrefs.setPresence(p)}>{p === "calm" ? "Calm" : "Bold"}</button>
                    {/each}
                  </div>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Density</div>
                  <div class="st-row-desc">Spacing of rows and cards across the app.</div>
                </div>
                <div class="st-row-ctl">
                  <div class="st-seg">
                    {#each DENSITIES as d (d)}
                      <button class="st-seg-btn" class:on={uiPrefs.density === d} type="button" onclick={() => uiPrefs.setDensity(d)}>{d[0].toUpperCase() + d.slice(1)}</button>
                    {/each}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Code preview</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Font size</div>
                  <div class="st-row-desc">Size of code in diffs, previews, and the file browser.</div>
                </div>
                <div class="st-row-ctl" style="min-width:150px;">
                  <Select
                    value={String(uiPrefs.code.fontSize)}
                    options={[11,12,13,14].map((n) => ({ value: String(n), label: `${n}px` }))}
                    onChange={(v) => uiPrefs.setCode({ fontSize: Number(v) })}
                    ariaLabel="Code font size"
                  />
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Tab width</div>
                  <div class="st-row-desc">Spaces per indentation level.</div>
                </div>
                <div class="st-row-ctl">
                  <div class="st-seg">
                    {#each [2, 4] as w (w)}
                      <button class="st-seg-btn" class:on={uiPrefs.code.tabWidth === w} type="button" onclick={() => uiPrefs.setCode({ tabWidth: w })}>{w}</button>
                    {/each}
                  </div>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Font ligatures</div>
                  <div class="st-row-desc">Render <span class="mono">→ ≠ &gt;=</span> as joined glyphs in JetBrains Mono.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={uiPrefs.code.ligatures} role="switch" aria-checked={uiPrefs.code.ligatures} aria-label="Font ligatures" type="button" onclick={() => uiPrefs.setCode({ ligatures: !uiPrefs.code.ligatures })}></button>
                </div>
              </div>
            </div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Layout</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Activity bar &amp; tabs</div>
                  <div class="st-row-desc">Click an icon on the left rail to swap the main pane. Drag icons to reorder — the Ctrl+1…6 shortcuts follow the bar's order.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-btn danger-btn" type="button" onclick={() => void assistantStore.closeAllTabs()} use:tooltip={"Close every chat tab in this session"}>
                    <X size={14} /> Close all chat tabs
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Keyboard shortcuts</div>
            <div class="st-card">
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
            </div>
          </div>
        </section>

        <!-- ── ACCESSIBILITY ── -->
        <section class="st-sec" bind:this={secEls.accessibility}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><A11yIcon size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">Accessibility</div><div class="st-sec-sub">{ST_SECTIONS[2].sub}</div></div>
          </div>
          <div class="st-block">
            <div class="st-block-label">Reading comfort</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Dyslexia-friendly mode</div>
                  <div class="st-row-desc">Lexend font + wider line spacing, and tells Claude to interpret phonetic typos / voice-to-text artifacts charitably.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={accessibility.dyslexiaMode} role="switch" aria-checked={accessibility.dyslexiaMode} aria-label="Dyslexia-friendly mode" type="button" onclick={() => accessibility.setDyslexiaMode(!accessibility.dyslexiaMode)}></button>
                </div>
              </div>
              <div class="st-row" data-disabled={!accessibility.dyslexiaMode}>
                <div class="st-row-body">
                  <div class="st-row-label">UI font</div>
                  <div class="st-row-desc">Lexend has the strongest research backing for reading-rate improvement on dyslexic readers.</div>
                </div>
                <div class="st-row-ctl">
                  <div class="st-seg" role="radiogroup" aria-label="UI font">
                    <button class="st-seg-btn" class:on={accessibility.font === "system"} role="radio" aria-checked={accessibility.font === "system"} disabled={!accessibility.dyslexiaMode} type="button" onclick={() => accessibility.setFont("system")}>Inter</button>
                    <button class="st-seg-btn" class:on={accessibility.font === "lexend"} role="radio" aria-checked={accessibility.font === "lexend"} disabled={!accessibility.dyslexiaMode} type="button" onclick={() => accessibility.setFont("lexend")}>Lexend</button>
                  </div>
                </div>
              </div>
              <div class="st-row" data-disabled={!accessibility.dyslexiaMode}>
                <div class="st-row-body">
                  <div class="st-row-label">Wider line + letter spacing</div>
                  <div class="st-row-desc">Bumps line-height to 1.85 inside Assistant bubbles and the composer.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={accessibility.lineHeightBoost} role="switch" aria-checked={accessibility.lineHeightBoost} aria-label="Increased line and letter spacing" disabled={!accessibility.dyslexiaMode} type="button" onclick={() => accessibility.setLineHeightBoost(!accessibility.lineHeightBoost)}></button>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Warm reading tint</div>
                  <div class="st-row-desc">Sepia overlay on Assistant message bubbles — softens bright-white-on-dark glare. UI chrome keeps the dark theme.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={accessibility.warmTint} role="switch" aria-checked={accessibility.warmTint} aria-label="Warm reading tint" type="button" onclick={() => accessibility.setWarmTint(!accessibility.warmTint)}></button>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- ── ASSISTANT ── -->
        <section class="st-sec" bind:this={secEls.assistant}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><Sparkles size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">Assistant</div><div class="st-sec-sub">{ST_SECTIONS[3].sub}</div></div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Claude session</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Status</div>
                  <div class="st-row-desc">Rift uses your local <code>claude</code> CLI session by default. Not signed in? Run <code>claude login</code> in a terminal, then re-probe.</div>
                </div>
                <div class="st-row-ctl">
                  {#if assistantStore.auth}
                    <span class="st-pill" class:ok={assistantStore.auth.pill === "green"} class:warn={assistantStore.auth.pill === "yellow" || assistantStore.auth.pill === "red"}><span class="dot"></span>{assistantStore.auth.summary}</span>
                  {:else if assistantStore.authChecking}
                    <span class="st-pill"><span class="dot"></span>Checking…</span>
                  {:else}
                    <span class="st-pill"><span class="dot"></span>Unknown</span>
                  {/if}
                  {#if assistantStore.authLastProbed && !assistantStore.authChecking}
                    <span class="st-stamp" use:tooltip={"Time since the last CLI session probe"}>checked {fmtAgo(assistantStore.authLastProbed, asstNowTick)}</span>
                  {/if}
                  <button class="st-btn" type="button" onclick={() => assistantStore.refreshAuth()} disabled={assistantStore.authChecking}><RefreshCw size={14} /> Re-probe</button>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Use my full Claude Code config</div>
                  <div class="st-row-desc">Layers <code>~/.claude/CLAUDE.md</code>, slash commands, skills, and MCP servers into every turn alongside Rift's own MCP tools. Off = sandboxed (Rift MCP only).</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={assistantStore.useFullConfig && !assistantStore.hasApiKey} role="switch" aria-checked={assistantStore.useFullConfig && !assistantStore.hasApiKey} aria-label="Use full Claude Code config" disabled={assistantStore.hasApiKey} type="button" onclick={() => void assistantStore.setUseFullConfig(!assistantStore.useFullConfig)}></button>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Allow remote shell</div>
                  <div class="st-row-desc">Exposes <code>mcp__rift__remote_bash</code> to the model — runs commands on the connected SSH server. Workspace-scoped advisory lock serializes calls.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={assistantStore.allowRemoteShell} role="switch" aria-checked={assistantStore.allowRemoteShell} aria-label="Allow remote shell" type="button" onclick={() => void assistantStore.setAllowRemoteShell(!assistantStore.allowRemoteShell)}></button>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Git tools</div>
                  <div class="st-row-desc">Local <code>git</code> tools for the model. Read-only = status, diff, log. Standard adds commit, pull, and push. Enabling "Allow remote shell" grants Standard automatically.</div>
                </div>
                <div class="st-row-ctl">
                  <div class="st-seg" role="radiogroup" aria-label="Git tools trust level">
                    <button class="st-seg-btn" class:on={assistantStore.trustLevel === "readonly"} role="radio" aria-checked={assistantStore.trustLevel === "readonly"} type="button" onclick={() => void assistantStore.setTrustLevel("readonly")}>Read-only</button>
                    <button class="st-seg-btn" class:on={assistantStore.trustLevel !== "readonly"} role="radio" aria-checked={assistantStore.trustLevel !== "readonly"} type="button" onclick={() => void assistantStore.setTrustLevel("standard")}>Standard</button>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Budget &amp; billing</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <label class="st-row-label" for="asst-budget">Per-turn cost cap</label>
                  <div class="st-row-desc">Passes <code>--max-budget-usd</code> to the CLI. If a turn would exceed this cap, the CLI exits with an error. Leave blank for no cap.</div>
                </div>
                <div class="st-row-ctl">
                  <input id="asst-budget" class="st-input mono" type="number" min="0" step="0.01" placeholder="5.00" style="width:88px; text-align:right;" bind:value={asstMaxBudgetDraft} />
                  <button class="st-btn primary" type="button" onclick={saveAsstMaxBudget} disabled={asstMaxBudgetSaving || !asstMaxBudgetDirty}>{asstMaxBudgetSaving ? "Saving…" : "Save"}</button>
                  {#if assistantStore.maxBudgetUsd != null}
                    <button class="st-btn" type="button" disabled={asstMaxBudgetSaving} onclick={() => { asstMaxBudgetDraft = null; void saveAsstMaxBudget(); }}>Clear</button>
                  {/if}
                </div>
              </div>
              {#if asstMaxBudgetMsg}<div class="st-note">{asstMaxBudgetMsg}</div>{/if}
              <div class="st-row">
                <div class="st-row-body">
                  <label class="st-row-label" for="asst-apikey">API-key fallback</label>
                  <div class="st-row-desc">Pay-per-token via console.anthropic.com. When set, overrides the CLI session (forces <code>--bare</code>). Stored in the OS keychain.</div>
                </div>
                <div class="st-row-ctl">
                  {#if assistantStore.hasApiKey}
                    <span class="st-pill ok"><span class="dot"></span>Configured</span>
                    <button class="st-btn" type="button" disabled={asstApiKeySaving} onclick={() => { asstApiKeyDraft = ""; void saveAsstApiKey(); }}>Clear</button>
                  {:else}
                    <span class="st-secret">
                      <input id="asst-apikey" class="st-input mono" type={asstApiKeyVisible ? "text" : "password"} placeholder="sk-ant-api03-…" style="width:188px;" bind:value={asstApiKeyDraft} autocomplete="off" spellcheck="false" />
                      <button class="st-eye" type="button" onclick={() => (asstApiKeyVisible = !asstApiKeyVisible)} aria-label={asstApiKeyVisible ? "Hide API key" : "Show API key"}>{#if asstApiKeyVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</button>
                    </span>
                    <button class="st-btn primary" type="button" onclick={saveAsstApiKey} disabled={asstApiKeySaving || !asstApiKeyDirty}>{asstApiKeySaving ? "Saving…" : "Save"}</button>
                  {/if}
                </div>
              </div>
              {#if asstApiKeyMsg}<div class="st-note">{asstApiKeyMsg}</div>{/if}
              {#if assistantStore.auth?.envApiKeyPresent && !assistantStore.auth?.apiKeyConfigured}
                <div class="st-note">⚠ A system <code>ANTHROPIC_API_KEY</code> environment variable is set, but Rift ignores env keys so it can't silently override your login. To use that key, paste it above; otherwise remove it from your environment.</div>
              {/if}
            </div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Conversation compaction</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Auto-compact threshold</div>
                  <div class="st-row-desc">When context (incl. cache-read) fills past the threshold, Rift summarizes via <code>claude -p</code> and seeds the next turn with the summary. 5min cooldown between fires.</div>
                </div>
                <div class="st-row-ctl" style="min-width:170px;">
                  <Select
                    value={String(assistantStore.autoCompactThreshold ?? 0)}
                    options={[{ value: "0", label: "Off" }, { value: "0.7", label: "70%" }, { value: "0.8", label: "80% — recommended" }, { value: "0.85", label: "85%" }, { value: "0.9", label: "90%" }]}
                    onChange={(v) => { const raw = Number(v); void assistantStore.setAutoCompactThreshold(raw > 0 ? raw : null); }}
                    ariaLabel="Auto-compact threshold"
                  />
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Compact model</div>
                  <div class="st-row-desc">Haiku is sufficient for prose summarization. Sonnet only if Haiku misses details on your workflow.</div>
                </div>
                <div class="st-row-ctl">
                  <div class="st-seg" role="radiogroup" aria-label="Compact model">
                    <button class="st-seg-btn" class:on={assistantStore.compactModel === "haiku"} role="radio" aria-checked={assistantStore.compactModel === "haiku"} type="button" onclick={() => void assistantStore.setCompactModel("haiku")}>Haiku</button>
                    <button class="st-seg-btn" class:on={assistantStore.compactModel === "sonnet"} role="radio" aria-checked={assistantStore.compactModel === "sonnet"} type="button" onclick={() => void assistantStore.setCompactModel("sonnet")}>Sonnet</button>
                  </div>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Compact now</div>
                  <div class="st-row-desc">Summarize the active chat regardless of threshold. Needs ≥4 messages and no in-flight stream.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-btn primary" type="button" disabled={assistantStore.activeTab?.compactingNow} onclick={() => void assistantStore.compactConversation()}>Compact now</button>
                </div>
              </div>
            </div>
          </div>
        </section>

        <!-- ── SPEECH ── -->
        <section class="st-sec" bind:this={secEls.speech}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><Mic size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">Speech</div><div class="st-sec-sub">{ST_SECTIONS[4].sub}</div></div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Engine</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Speech-to-text</div>
                  <div class="st-row-desc">Master switch. When off, the mic button in the composer is hidden.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={stt.config.enabled} role="switch" aria-checked={stt.config.enabled} aria-label="Enable speech-to-text" type="button" onclick={() => void stt.setConfig({ enabled: !stt.config.enabled })}></button>
                </div>
              </div>
              <div class="st-row st-row-stack">
                <div class="st-row-body">
                  <div class="st-row-label">Recognition engine</div>
                  <div class="st-row-desc">Web Speech is zero-install via Edge / Azure. Whisper runs on-device with stronger accent tolerance and vocabulary priming.</div>
                </div>
                <div class="set-pick-grid set-pick-grid-2">
                  {#each STT_ENGINES as eng (eng.id)}
                    <button type="button" class="set-pick" data-active={stt.config.engine === eng.id} disabled={!stt.config.enabled || (eng.id === "whisper" && !stt.backendAvailable)} onclick={() => void stt.setConfig({ engine: eng.id })}>
                      <span class="set-pick-label">{eng.label}</span>
                      <span class="set-pick-sub mono">{eng.sub}</span>
                    </button>
                  {/each}
                </div>
              </div>
              {#if !stt.backendAvailable}
                <div class="st-warn">Whisper backend not built into this Rift release. To enable: install LLVM (<code>winget install LLVM.LLVM</code>, admin required), optionally the NVIDIA CUDA Toolkit for GPU, then rebuild with <code>cargo build --release --features whisper-rs</code>.</div>
              {/if}
            </div>
          </div>

          {#if stt.config.engine === "web_speech"}
            <div class="st-block">
              <div class="st-block-label">Web Speech</div>
              <div class="st-card">
                <div class="st-row st-row-stack">
                  <div class="st-row-body">
                    <div class="st-row-label">Language</div>
                    <div class="st-row-desc">BCP-47 tag passed to the recogniser. Pick another language if you speak something other than English.</div>
                  </div>
                  <div class="set-pick-grid" role="radiogroup" aria-label="Speech recognition language">
                    {#each STT_LANGS as l (l.id)}
                      <button type="button" role="radio" aria-checked={stt.config.language === l.id} class="set-pick" data-active={stt.config.language === l.id} onclick={() => void stt.setConfig({ language: l.id })}>
                        <span class="set-pick-label">{l.label}</span>
                        <span class="set-pick-sub mono">{l.id}</span>
                      </button>
                    {/each}
                  </div>
                </div>
                <div class="st-row">
                  <div class="st-row-body">
                    <div class="st-row-label">Continuous mode</div>
                    <div class="st-row-desc">Keep listening across pauses until you click stop.</div>
                  </div>
                  <div class="st-row-ctl">
                    <button class="st-switch" class:on={stt.config.continuous} role="switch" aria-checked={stt.config.continuous} aria-label="Continuous mode" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ continuous: !stt.config.continuous })}></button>
                  </div>
                </div>
              </div>
            </div>
          {/if}

          {#if stt.config.engine === "whisper"}
            <div class="st-block">
              <div class="st-block-label">Whisper model</div>
              <div class="st-card">
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
                          <div class="set-progress-label mono">{fmtMB(prog.downloaded)} / {fmtMB(prog.total)} · {fmtPct(prog.downloaded, prog.total)}</div>
                        {/if}
                      </div>
                      <div class="set-model-actions">
                        {#if isDownloading}
                          <button type="button" class="st-btn" onclick={() => void stt.cancelDownload()}>Cancel</button>
                        {:else if m.downloaded}
                          {#if !isActive}
                            <button type="button" class="st-btn" onclick={() => void stt.setConfig({ whisper_model: m.id })}>Use</button>
                          {:else}
                            <span class="st-pill ok"><span class="dot"></span>Active</span>
                          {/if}
                          <button type="button" class="st-btn" onclick={() => void stt.deleteModel(m.id)} use:tooltip={"Delete model"} aria-label="Delete"><Trash2 size={14} /></button>
                        {:else}
                          <button type="button" class="st-btn primary" disabled={!stt.config.enabled} onclick={() => void stt.downloadModel(m.id)}>Download</button>
                        {/if}
                      </div>
                    </div>
                  {/each}
                  {#if stt.models.length === 0}<div class="st-note">Loading model catalogue…</div>{/if}
                </div>
              </div>
            </div>

            <div class="st-block">
              <div class="st-block-label">Capture &amp; cleanup</div>
              <div class="st-card">
                <div class="st-row">
                  <div class="st-row-body">
                    <div class="st-row-label">Input device</div>
                    <div class="st-row-desc">Microphone used by Whisper capture. System default is usually correct.</div>
                  </div>
                  <div class="st-row-ctl set-mic-r">
                    <div class="set-mic-select">
                      <Select
                        value={stt.config.input_device ?? ""}
                        options={[{ value: "", label: "System default" }, ...stt.inputDevices.map((d) => ({ value: d, label: d }))]}
                        onChange={(v) => void stt.setConfig({ input_device: v === "" ? null : v })}
                        disabled={!stt.config.enabled}
                        ariaLabel="Whisper input device"
                      />
                    </div>
                    <button type="button" class="st-btn" onclick={() => void stt.refreshInputDevices()} use:tooltip={"Refresh device list"} aria-label="Refresh"><RefreshCw size={14} /></button>
                  </div>
                </div>
                <div class="st-row">
                  <div class="st-row-body">
                    <div class="st-row-label">Clean with Claude Haiku</div>
                    <div class="st-row-desc">Polishes the final transcript via Haiku — fixes punctuation, capitalises proper nouns. ~200-400ms tail, ~$0.0001/utterance.</div>
                  </div>
                  <div class="st-row-ctl">
                    <button class="st-switch" class:on={stt.config.cleanup_enabled} role="switch" aria-checked={stt.config.cleanup_enabled} aria-label="Clean transcripts with Claude Haiku" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ cleanup_enabled: !stt.config.cleanup_enabled })}></button>
                  </div>
                </div>
              </div>
            </div>

            <div class="st-block">
              <div class="st-block-label">Vocabulary priming</div>
              <div class="st-card">
                <div class="st-row st-row-stack">
                  <div class="st-row-body">
                    <div class="st-row-label">Style prompt</div>
                    <div class="st-row-desc">Whisper's <code>initial_prompt</code> biases the decoder toward your speaking style.</div>
                  </div>
                  <textarea class="set-textarea" rows="3" disabled={!stt.config.enabled} value={stt.config.initial_prompt} oninput={(e) => void stt.setConfig({ initial_prompt: (e.currentTarget as HTMLTextAreaElement).value })}></textarea>
                </div>
                <div class="st-row st-row-stack">
                  <div class="st-row-body">
                    <div class="st-row-label">Vocabulary</div>
                    <div class="st-row-desc">Comma- or newline-separated. Add project names, server names. Budget ~800 chars (Whisper's 224-token prompt limit).</div>
                  </div>
                  <textarea class="set-textarea" rows="4" placeholder="FiveM, Qbox, RedM, rift_bridge, fxmanifest, ..." disabled={!stt.config.enabled} value={stt.config.vocab_text} oninput={(e) => void stt.setConfig({ vocab_text: (e.currentTarget as HTMLTextAreaElement).value })}></textarea>
                </div>
              </div>
            </div>
          {/if}

          <div class="st-block">
            <div class="st-block-label">Composer integration</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Live partial transcripts</div>
                  <div class="st-row-desc">Words appear in the composer as you speak. Off = wait for each sentence to commit.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={stt.config.show_interim} role="switch" aria-checked={stt.config.show_interim} aria-label="Live partial transcripts" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ show_interim: !stt.config.show_interim })}></button>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Insertion mode</div>
                  <div class="st-row-desc">Append preserves what's typed; off = transcript replaces composer contents (mic-first workflow).</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={stt.config.append_to_draft} role="switch" aria-checked={stt.config.append_to_draft} aria-label="Append transcript to existing draft" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ append_to_draft: !stt.config.append_to_draft })}></button>
                </div>
              </div>
            </div>
          </div>

          {#if stt.config.engine === "web_speech" && !stt.supported}
            <div class="st-warn">Your WebView does not expose <code>SpeechRecognition</code>; Web Speech is unavailable — switch to Whisper or install LLVM and rebuild.</div>
          {/if}
          {#if stt.lastError}<div class="st-warn">{stt.lastError}</div>{/if}
        </section>

        <!-- ── SERVER (network) ── -->
        <section class="st-sec" bind:this={secEls.network}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><Server size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">Server</div><div class="st-sec-sub">{ST_SECTIONS[5].sub}</div></div>
          </div>

          {#if connection.selected}
            <div class="st-block">
              <div class="st-card">
                <div class="st-hero">
                  <div class="st-hero-ic"><Server size={24} strokeWidth={1.75} /></div>
                  <div class="st-hero-meta">
                    <div class="st-hero-name">{connection.selected.name}</div>
                    <div class="st-hero-host">{connection.selected.user}@{connection.selected.host}{connection.selected.port !== 22 ? `:${connection.selected.port}` : ""} · SFTP mirror</div>
                  </div>
                  <div class="st-hero-act">
                    {#if watcherOn}
                      <span class="st-conn-pill up"><span class="dot"></span>Connected</span>
                    {:else}
                      <span class="st-conn-pill down"><span class="dot"></span>{connection.status?.state ?? "Offline"}</span>
                    {/if}
                  </div>
                </div>
              </div>
            </div>
          {/if}

          <div class="st-block">
            <div class="st-block-label" style="display:flex; align-items:center; justify-content:space-between;">
              <span>SSH servers · {connection.servers.length}</span>
              <button class="st-btn primary" type="button" onclick={() => dialogs.onAddServer()}><Plus size={14} /> Add server</button>
            </div>
            <div class="st-card">
              {#if connection.servers.length === 0}
                <div class="set-empty">
                  <span class="set-empty-title">No servers yet</span>
                  <span class="set-empty-hint">Click <strong>Add server</strong> above to create your first profile.</span>
                </div>
              {:else}
                <div class="srv-list">
                  {#each connection.servers as s (s.key)}
                    <div class="srv-card" data-active={connection.selectedKey === s.key}>
                      <button type="button" class="srv-select" onclick={() => pickServer(s)} aria-label={`Select server ${s.name}`} aria-pressed={connection.selectedKey === s.key}>
                        <div class="srv-l">
                          <span class="srv-dot"></span>
                          <div class="srv-meta-l">
                            <div class="mono srv-name">{s.name}</div>
                            <div class="mono dim srv-host">{s.user}@{s.host}{s.port !== 22 ? `:${s.port}` : ""}</div>
                          </div>
                        </div>
                        <div class="srv-meta mono dim" use:tooltip={s.fingerprint ?? "no fingerprint pinned"}>{s.fingerprint ? `ed25519 · ${s.fingerprint.slice(0, 18)}…` : "no fingerprint pinned"}</div>
                      </button>
                      <div class="srv-r">
                        {#if s.fingerprint}<button class="st-btn icon" onclick={() => onUnpinFingerprint(s)} type="button" use:tooltip={`Clear pinned fingerprint for ${s.name}`} aria-label={`Unpin fingerprint for ${s.name}`}><X size={14} /></button>{/if}
                        {#if s.hasBridgeToken}<button class="st-btn icon" onclick={() => onRotateBridgeToken(s)} type="button" use:tooltip={`Rotate bridge token for ${s.name}`} aria-label={`Rotate bridge token for ${s.name}`}><RefreshCw size={14} /></button>{/if}
                        <button class="st-btn icon" onclick={() => dialogs.onEditServer(s)} type="button" use:tooltip={`Edit ${s.name}`} aria-label={`Edit server ${s.name}`}><Pencil size={14} /></button>
                        <button class="st-btn icon" onclick={() => dialogs.onDeleteServer(s)} type="button" use:tooltip={`Delete ${s.name}`} aria-label={`Delete server ${s.name}`}><Trash2 size={14} /></button>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </div>

          <div class="st-block">
            <div class="st-block-label">SSH private key</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Default keypair</div>
                  {#if sshKeyConfigured === true && sshKeyPath}
                    <div class="st-row-desc"><span class="st-keynote"><CircleCheck size={13} /> Configured · <span class="mono">{sshKeyPath}</span></span></div>
                  {:else if sshKeyConfigured === false}
                    <div class="st-row-desc">Not configured. Generate or import a private key, then copy the public half into the server's <code>authorized_keys</code>.</div>
                  {:else}
                    <div class="st-row-desc">ed25519 keypair stored at <code>%APPDATA%/Rift/keys/</code>.</div>
                  {/if}
                </div>
                <div class="st-row-ctl">
                  <button class="st-btn" onclick={() => { dialogs.onLaunchKeygen(); void refreshSshKeyStatus(); }} type="button"><Key size={14} /> {sshKeyConfigured ? "Manage key" : "Set up key"}</button>
                </div>
              </div>
            </div>
          </div>

          {#if rotatedToken}
            <div class="st-block">
              <div class="st-block-label">New bridge token — copy now</div>
              <div class="st-card">
                <div class="st-row">
                  <div class="st-row-body">
                    <div class="st-row-label">{connection.servers.find(s => s.key === rotatedToken!.key)?.name ?? rotatedToken.key}</div>
                    <div class="st-row-desc">Paste this into the remote bridge's <code>RIFT_BRIDGE_TOKEN</code> env var. Once you close this panel the token is no longer shown.</div>
                  </div>
                  <div class="st-row-ctl">
                    <code class="mono" style="font-size:11px; padding:4px 6px; background:var(--bg-inset); border-radius:4px; user-select:all;">{rotatedToken.token}</code>
                    <button class="st-btn icon" onclick={onCopyRotatedToken} type="button" use:tooltip={"Copy token"}>{#if rotatedTokenCopied}<Check size={14} />{:else}<Copy size={14} />{/if}</button>
                    <button class="st-btn icon" onclick={() => (rotatedToken = null)} type="button" use:tooltip={"Dismiss"}><X size={14} /></button>
                  </div>
                </div>
              </div>
            </div>
          {/if}

          <!-- ── Drift detection ── -->
          <div class="st-block">
            <div class="st-block-label">Drift detection</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Periodic rescan</div>
                  <div class="st-row-desc">Catch remote-side drift (teammates pushing, out-of-band edits) that the local watcher can't see.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={syncPage.autoRescanEnabled} role="switch" aria-checked={syncPage.autoRescanEnabled} aria-label="Periodic rescan" type="button" onclick={() => { syncPage.autoRescanEnabled = !syncPage.autoRescanEnabled; if (typeof localStorage !== "undefined") localStorage.setItem("rift.sync.autoRescan.enabled", syncPage.autoRescanEnabled ? "1" : "0"); }}></button>
                </div>
              </div>
              <div class="st-row" data-disabled={!syncPage.autoRescanEnabled}>
                <div class="st-row-body">
                  <div class="st-row-label">Rescan interval</div>
                  <div class="st-row-desc">How often Rift re-checks for remote drift while the watcher is running.</div>
                </div>
                <div class="st-row-ctl" style="min-width:160px;">
                  <Select
                    value={String(syncPage.autoRescanIntervalSec)}
                    options={[30,60,120,300,600].map(s => ({ value: String(s), label: s < 60 ? `${s}s` : `${Math.round(s/60)}m` }))}
                    onChange={(v) => { syncPage.autoRescanIntervalSec = Number(v); if (typeof localStorage !== "undefined") localStorage.setItem("rift.sync.autoRescan.intervalSec", v); }}
                    disabled={!syncPage.autoRescanEnabled}
                    ariaLabel="Rescan interval"
                  />
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Mirror mode</div>
                  <div class="st-row-desc">Buckets local-missing + remote-has files as remote-delete instead of pull. Destructive — use with care.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={syncPage.mirrorEnabled} role="switch" aria-checked={syncPage.mirrorEnabled} aria-label="Mirror mode" type="button" onclick={() => void syncPage.toggleMirror(!syncPage.mirrorEnabled)}></button>
                </div>
              </div>
            </div>
          </div>

          <!-- ── Danger zone ── -->
          {#if connection.selected}
            <div class="st-block">
              <div class="st-block-label">Danger zone</div>
              <div class="st-card danger st-danger">
                <div class="st-row">
                  <div class="st-row-body">
                    <div class="st-row-label">Forget server</div>
                    <div class="st-row-desc">Permanently removes this server profile. Local files are not deleted.</div>
                  </div>
                  <div class="st-row-ctl">
                    <button class="st-btn danger-btn" type="button" onclick={() => dialogs.onDeleteServer(connection.selected!)}><Trash2 size={14} /> Forget server</button>
                  </div>
                </div>
              </div>
            </div>
          {/if}

          <!-- ── RCON live console ── -->
          <div class="st-block">
            <div class="st-block-label">RCON console</div>
            <!-- Password config row -->
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">RCON password</div>
                  <div class="st-row-desc">Set the RCON password for the selected server. Leave blank to clear.</div>
                </div>
                <div class="st-row-ctl">
                  <span class="st-secret">
                    <input class="st-input mono" type={rconShowPw ? "text" : "password"} placeholder="password…" style="width:160px;" bind:value={rconPassword} autocomplete="off" spellcheck="false" disabled={!connection.selected} />
                    <button class="st-eye" type="button" onclick={() => (rconShowPw = !rconShowPw)} aria-label={rconShowPw ? "Hide password" : "Show password"}>{#if rconShowPw}<EyeOff size={14} />{:else}<Eye size={14} />{/if}</button>
                  </span>
                  <button class="st-btn primary" type="button" onclick={rconSavePassword} disabled={!connection.selected || !rconPassword}><Check size={14} /> Save</button>
                  {#if rconConfigured}
                    <button class="st-btn" type="button" onclick={rconClearPassword} disabled={!connection.selected}><X size={14} /> Clear</button>
                  {/if}
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Auto-reconnect ping</div>
                  <div class="st-row-desc">Send a <code>version</code> command every 15s to keep the RCON status fresh.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={uiPrefs.rconAutoReconnect} role="switch" aria-checked={uiPrefs.rconAutoReconnect} aria-label="RCON auto-reconnect ping" type="button" onclick={() => uiPrefs.toggleRconAutoReconnect()}></button>
                </div>
              </div>
            </div>
            <!-- Console terminal -->
            <div class="st-console">
              {#if !connection.selected || !rconConfigured}
                <div class="st-console-offline">
                  <Terminal size={24} strokeWidth={1.5} />
                  <span class="t">RCON not configured</span>
                  <span class="s">Set a password above, then save it to connect.</span>
                </div>
              {:else}
                <div class="st-console-bar">
                  <Terminal size={14} strokeWidth={1.5} />
                  <span class="st-console-title">RCON · {connection.selected.name}</span>
                  <div class="st-console-quick">
                    <button class="st-quick" type="button" disabled={rconBusy} onclick={() => runRcon("status")}>status</button>
                    <button class="st-quick" type="button" disabled={rconBusy} onclick={() => runRcon("version")}>version</button>
                    <button class="st-quick" type="button" disabled={rconBusy} onclick={() => runRcon("resources")}>resources</button>
                  </div>
                  <span class="st-pill" class:ok={rconStatus === "online"} class:warn={rconStatus === "offline" || rconStatus === "checking"}>
                    <span class="dot"></span>{rconStatus}
                  </span>
                </div>
                <div class="st-console-log">
                  {#each rconLog as line (line.time + line.msg)}
                    <div class="st-log-line">
                      <span class="st-log-time">{line.time}</span>
                      <span class="st-log-msg {line.kind}">{line.msg}</span>
                    </div>
                  {/each}
                  {#if rconLog.length === 0}
                    <span style="color:var(--fg-faint); font-family:var(--font-mono); font-size:11px;">No output yet — run a command.</span>
                  {/if}
                </div>
                <div class="st-console-input">
                  <span class="prompt">❯</span>
                  <input bind:value={rconInput} disabled={rconBusy} placeholder="enter command…" onkeydown={(e) => { if (e.key === "Enter" && rconInput.trim()) { e.preventDefault(); void runRcon(rconInput.trim()); } }} />
                  <button class="st-console-send" type="button" disabled={rconBusy || !rconInput.trim()} onclick={() => runRcon(rconInput.trim())}><Terminal size={13} /></button>
                </div>
              {/if}
            </div>
          </div>
        </section>

        <!-- ── ABOUT ── -->
        <section class="st-sec" bind:this={secEls.about}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><Info size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">About</div><div class="st-sec-sub">{ST_SECTIONS[6].sub}</div></div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Build</div>
            <div class="st-card">
              {#each [["Rift", `${appVersion} · Tauri 2`], ["Engine", "SvelteKit · Svelte 5 (runes)"], ["Style", "Graphite Ink · Tailwind v4 · OKLCH"], ["SSH", "russh + russh-sftp · ring"], ["License", "MIT · github.com/Blazzer10200/rift"]] as kv (kv[0])}
                <div class="st-kv"><span class="st-kv-k">{kv[0]}</span><span class="st-kv-v">{kv[1]}</span></div>
              {/each}
            </div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Paths</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Config</div>
                  <div class="st-row-desc"><span class="mono" use:tooltip={configDir}>{configDir || "—"}</span></div>
                </div>
                <div class="st-row-ctl"><button class="st-btn" type="button" disabled={!configDir} onclick={() => openDir(configDir)}><FolderOpen size={14} /> Open</button></div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Logs</div>
                  <div class="st-row-desc"><span class="mono" use:tooltip={logDir}>{logDir || "—"}</span></div>
                </div>
                <div class="st-row-ctl"><button class="st-btn" type="button" disabled={!logDir} onclick={() => openDir(logDir)}><FolderOpen size={14} /> Open</button></div>
              </div>
            </div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Help &amp; diagnostics</div>
            <div class="st-card">
              <button class="st-about-row" type="button" onclick={() => updates.open()}>
                <span class="st-about-ic"><RefreshCw size={15} /></span>
                <span class="st-about-body"><span class="st-about-t">Check for updates</span><span class="st-about-s">Compare against the latest GitHub release</span></span>
              </button>
              <button class="st-about-row" type="button" onclick={copyDiagnostic}>
                <span class="st-about-ic">{#if diagCopied}<Check size={15} />{:else}<Copy size={15} />{/if}</span>
                <span class="st-about-body"><span class="st-about-t">{diagCopied ? "Copied to clipboard" : "Copy diagnostic"}</span><span class="st-about-s">Version, platform, and paths — username-scrubbed</span></span>
              </button>
            </div>
          </div>
        </section>

      </div>
    </div>
  </div>
</div>

<style>
  .st-main { position: relative; overflow: hidden; display: flex; flex: 1; min-height: 0; min-width: 0; background: var(--bg); color: var(--fg); }
  .st-inner { flex: 1; min-width: 0; min-height: 0; display: flex; }

  /* ── Sticky section index ── */
  .st-index { width: 232px; flex: none; min-height: 0; border-right: 1px solid var(--border); display: flex; flex-direction: column; background: var(--bg); padding: 20px 14px 14px; }
  .st-index-head { padding: 0 8px 16px; }
  .st-index-title { font-size: 19px; font-weight: 700; letter-spacing: -0.02em; }
  .st-index-sub { font-size: var(--fs-xs); color: var(--fg-subtle); margin-top: 3px; display: flex; align-items: center; gap: 6px; }
  .st-index-sub .mono { font-family: var(--font-mono); color: var(--fg-muted); }
  .st-index-list { display: flex; flex-direction: column; gap: 1px; }
  .st-index-row { display: flex; align-items: center; gap: 11px; position: relative; height: 36px; padding: 0 11px; border: 0; border-radius: var(--radius); background: none; color: var(--fg-muted); font: inherit; font-size: var(--fs-md); font-weight: 550; cursor: pointer; text-align: left; width: 100%; transition: background 120ms var(--ease-soft), color 120ms var(--ease-soft); }
  .st-index-row:hover { background: var(--surface); color: var(--fg-2); }
  .st-index-row.on { background: var(--accent-soft); color: var(--fg); font-weight: 600; }
  .st-index-row.on::before { content: ""; position: absolute; left: 0; top: 8px; bottom: 8px; width: 2px; border-radius: 2px; background: var(--accent); }
  .st-index-row.on .st-index-ic :global(svg) { color: var(--accent); }
  .st-index-ic { color: var(--fg-subtle); display: inline-flex; flex: none; }
  .st-index-ic :global(svg) { transition: color 120ms; }
  .st-index-row .lbl { flex: 1; }
  .st-index-dot { width: 6px; height: 6px; border-radius: 999px; flex: none; }
  .st-index-dot.ok { background: var(--ok); box-shadow: 0 0 0 3px var(--ok-soft); }
  .st-index-dot.warn { background: var(--warn); box-shadow: 0 0 0 3px var(--warn-soft); }
  .st-index-foot { margin-top: auto; padding: 12px 4px 0; }
  .st-version { display: flex; align-items: center; gap: 9px; width: 100%; padding: 9px 11px; border-radius: var(--radius); background: var(--surface); border: 1px solid var(--border); cursor: pointer; font: inherit; color: var(--fg-2); transition: background 120ms, border-color 120ms; }
  .st-version:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .st-version :global(svg:first-child) { color: var(--accent); flex: none; }
  .st-version :global(svg:last-child) { color: var(--fg-subtle); flex: none; }
  .st-version-t { font-size: var(--fs-xs); color: var(--fg-2); font-weight: 600; }
  .st-version-s { font-family: var(--font-mono); font-size: 10px; color: var(--fg-faint); }

  /* ── Scroll document ── */
  .st-scroll { flex: 1; min-width: 0; min-height: 0; overflow-y: auto; overflow-x: hidden; scroll-behavior: smooth; }
  .st-doc { max-width: 680px; margin: 0 auto; padding: 32px 44px 96px; display: flex; flex-direction: column; gap: 40px; }
  .st-sec { display: flex; flex-direction: column; gap: 16px; scroll-margin-top: 24px; }
  .st-sec-head { display: flex; align-items: center; gap: 13px; }
  .st-sec-ic { width: 36px; height: 36px; border-radius: 10px; flex: none; display: grid; place-items: center; background: var(--accent-soft); color: var(--accent); box-shadow: inset 0 0 0 1px var(--ghost-border); }
  .st-sec-ic :global(svg) { color: var(--accent); }
  .st-sec-tt { font-size: 20px; font-weight: 700; letter-spacing: -0.02em; line-height: 1.1; }
  .st-sec-sub { font-size: var(--fs-sm); color: var(--fg-muted); margin-top: 3px; line-height: 1.45; }

  .st-block { display: flex; flex-direction: column; gap: 9px; }
  .st-block-label { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.05em; text-transform: uppercase; color: var(--fg-subtle); padding: 0 2px; }
  .st-card { background: var(--surface); border: 1px solid var(--border); border-radius: var(--r-card); }

  /* ── Rows ── */
  .st-row { display: flex; align-items: center; gap: 16px; padding: 14px 17px; }
  .st-row + .st-row, .st-card .st-note + .st-row, .st-row + .st-note { border-top: 1px solid var(--border); }
  .st-row-stack { flex-direction: column; align-items: stretch; gap: 10px; }
  .st-row-body { flex: 1; min-width: 0; }
  .st-row-label { font-size: var(--fs-md); font-weight: 600; color: var(--fg); display: block; }
  .st-row-desc { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; line-height: 1.5; }
  .st-row-desc .mono, .st-row-desc code { font-family: var(--font-mono); color: var(--fg-2); background: var(--bg-inset); padding: 1px 5px; border-radius: 4px; font-size: 0.92em; }
  .st-row-ctl { flex: none; display: flex; align-items: center; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }
  .st-row[data-disabled="true"] { opacity: 0.55; }
  .st-note { padding: 10px 17px; font-size: var(--fs-xs); color: var(--fg-muted); border-top: 1px solid var(--border); }
  .st-note code { font-family: var(--font-mono); background: var(--bg-inset); padding: 1px 5px; border-radius: 4px; color: var(--fg-2); }
  .st-warn { display: block; font-size: var(--fs-xs); color: var(--warn); line-height: 1.5; padding: 10px 13px; background: color-mix(in oklch, var(--warn) 10%, var(--surface)); border: 1px solid color-mix(in oklch, var(--warn) 30%, var(--border)); border-radius: var(--r-card); }
  .st-warn code { background: var(--bg-inset); padding: 1px 5px; border-radius: 3px; color: var(--fg-2); font-family: var(--font-mono); }

  /* ── Toggle switch ── */
  .st-switch { position: relative; width: 40px; height: 23px; border-radius: 999px; border: 0; padding: 0; background: var(--bg-elev-3); cursor: pointer; transition: background 160ms var(--ease-soft); flex: none; }
  .st-switch::after { content: ""; position: absolute; top: 3px; left: 3px; width: 17px; height: 17px; border-radius: 999px; background: var(--fg-muted); transition: transform 180ms var(--ease-page), background 160ms; }
  .st-switch.on { background: var(--accent); }
  .st-switch.on::after { transform: translateX(17px); background: var(--accent-fg); }
  .st-switch:disabled { opacity: 0.5; cursor: not-allowed; }
  .st-switch:focus-visible { outline: 0; box-shadow: 0 0 0 3px var(--ring); }

  /* ── Segmented ── */
  .st-seg { display: inline-flex; background: var(--bg-inset); border: 1px solid var(--border); border-radius: var(--radius); padding: 3px; gap: 2px; }
  .st-seg-btn { height: 26px; padding: 0 12px; border: 0; border-radius: 6px; background: none; color: var(--fg-muted); font: inherit; font-size: var(--fs-xs); font-weight: 600; cursor: pointer; white-space: nowrap; transition: color 120ms, background 120ms; }
  .st-seg-btn:hover:not(:disabled) { color: var(--fg); }
  .st-seg-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .st-seg-btn.on { background: var(--surface-hover); color: var(--fg); box-shadow: var(--shadow-sm); }

  /* ── Text input ── */
  .st-input { height: 32px; padding: 0 12px; border-radius: var(--radius); background: var(--bg-inset); border: 1px solid var(--border); color: var(--fg); font: inherit; font-size: var(--fs-sm); }
  .st-input:focus { outline: 0; border-color: var(--border-focus); box-shadow: 0 0 0 3px var(--ring); }
  .st-input.mono { font-family: var(--font-mono); }
  .st-secret { position: relative; display: inline-flex; align-items: center; }
  .st-secret .st-input { padding-right: 34px; }
  .st-eye { position: absolute; right: 5px; display: grid; place-items: center; width: 24px; height: 24px; border: 0; background: none; color: var(--fg-subtle); cursor: pointer; border-radius: 6px; }
  .st-eye:hover { color: var(--fg); background: var(--surface-hover); }

  /* ── Buttons ── */
  .st-btn { display: inline-flex; align-items: center; gap: 7px; height: 32px; padding: 0 13px; border-radius: var(--radius); font: inherit; font-size: var(--fs-sm); font-weight: 600; cursor: pointer; border: 1px solid var(--border); background: var(--surface); color: var(--fg-2); transition: background 120ms, border-color 120ms, color 120ms; white-space: nowrap; }
  .st-btn:hover:not(:disabled) { background: var(--surface-hover); border-color: var(--border-strong); color: var(--fg); }
  .st-btn:disabled { opacity: 0.45; cursor: not-allowed; }
  .st-btn.primary { background: var(--accent); border-color: transparent; color: var(--accent-fg); box-shadow: 0 4px 14px -5px color-mix(in srgb, var(--accent) 60%, transparent); }
  .st-btn.primary:hover:not(:disabled) { background: var(--accent-hover); }
  .st-btn.icon { padding: 0; width: 32px; justify-content: center; }
  .st-btn.danger-btn { color: var(--danger); border-color: color-mix(in oklch, var(--danger) 30%, var(--border)); }
  .st-btn.danger-btn:hover:not(:disabled) { background: color-mix(in oklch, var(--danger) 12%, var(--surface)); border-color: var(--danger); color: var(--danger); }
  .st-btn :global(svg) { color: currentColor; }
  .st-stamp { font-family: var(--font-mono); font-size: 10.5px; color: var(--fg-faint); white-space: nowrap; }

  /* ── Status pills ── */
  .st-pill { display: inline-flex; align-items: center; gap: 7px; height: 24px; padding: 0 10px; border-radius: 999px; font-size: var(--fs-xs); font-weight: 650; border: 1px solid var(--border); background: var(--surface); color: var(--fg-muted); }
  .st-pill .dot { width: 7px; height: 7px; border-radius: 999px; background: var(--fg-faint); }
  .st-pill.ok { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); color: var(--ok); }
  .st-pill.ok .dot { background: var(--ok); }
  .st-pill.warn { background: var(--warn-soft); border-color: color-mix(in oklch, var(--warn) 28%, transparent); color: var(--warn); }
  .st-pill.warn .dot { background: var(--warn); }

  /* ── Server: hero + connection pill ── */
  .st-hero { display: flex; align-items: center; gap: 15px; padding: 18px; background: linear-gradient(160deg, var(--surface), color-mix(in srgb, var(--accent) 5%, var(--surface))); border-radius: var(--r-card); }
  .st-hero-ic { width: 48px; height: 48px; border-radius: 13px; flex: none; display: grid; place-items: center; background: var(--bg-inset); border: 1px solid var(--border); color: var(--accent); }
  .st-hero-ic :global(svg) { color: var(--accent); }
  .st-hero-meta { flex: 1; min-width: 0; }
  .st-hero-name { font-size: var(--fs-lg); font-weight: 650; }
  .st-hero-host { font-family: var(--font-mono); font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; }
  .st-hero-act { display: flex; align-items: center; gap: 9px; flex: none; }
  .st-conn-pill { display: inline-flex; align-items: center; gap: 7px; height: 26px; padding: 0 11px; border-radius: 999px; font-size: var(--fs-xs); font-weight: 650; border: 1px solid transparent; flex: none; text-transform: capitalize; }
  .st-conn-pill .dot { width: 7px; height: 7px; border-radius: 999px; }
  .st-conn-pill.up { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 30%, transparent); color: var(--ok); }
  .st-conn-pill.up .dot { background: var(--ok); box-shadow: 0 0 0 3px color-mix(in srgb, var(--ok) 22%, transparent); }
  .st-conn-pill.down { background: var(--surface); border-color: var(--border); color: var(--fg-muted); }
  .st-conn-pill.down .dot { background: var(--fg-faint); }
  .st-keynote { display: inline-flex; align-items: center; gap: 6px; }
  .st-keynote :global(svg) { color: var(--ok); }
  .st-keynote .mono { font-family: var(--font-mono); color: var(--fg-2); background: var(--bg-inset); padding: 1px 5px; border-radius: 4px; }

  /* ── About: kv + resource rows ── */
  .st-kv { display: flex; align-items: center; gap: 16px; padding: 11px 17px; }
  .st-kv + .st-kv { border-top: 1px solid var(--border); }
  .st-kv-k { font-size: var(--fs-md); font-weight: 600; color: var(--fg); flex: none; width: 84px; }
  .st-kv-v { font-family: var(--font-mono); font-size: var(--fs-xs); color: var(--fg-muted); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .st-about-row { display: flex; align-items: center; gap: 12px; padding: 13px 17px; width: 100%; border: 0; background: none; text-align: left; font: inherit; cursor: pointer; }
  .st-about-row + .st-about-row { border-top: 1px solid var(--border); }
  .st-about-ic { width: 32px; height: 32px; border-radius: 9px; display: grid; place-items: center; background: var(--bg-inset); border: 1px solid var(--border); color: var(--fg-muted); flex: none; }
  .st-about-body { flex: 1; min-width: 0; }
  .st-about-t { font-size: var(--fs-sm); font-weight: 600; display: block; color: var(--fg); }
  .st-about-s { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 2px; display: block; }
  .st-about-row:hover { background: var(--surface-hover); }

  /* ── Appearance: accent swatch picker ── */
  .st-swatch-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; padding: 16px; }
  .st-swatch { display: flex; flex-direction: column; align-items: center; gap: 9px; padding: 8px 4px 6px; background: transparent; border: 0; border-radius: 10px; cursor: pointer; font: inherit; transition: background .13s ease; }
  .st-swatch:hover { background: var(--surface-hover); }
  .st-swatch-chip { width: 100%; height: 42px; border-radius: 9px; display: grid; place-items: center; background: var(--sw); color: rgba(0,0,0,0.82); box-shadow: inset 0 0 0 1px rgba(255,255,255,0.14); transition: box-shadow .15s ease, transform .12s ease; }
  .st-swatch:hover .st-swatch-chip { transform: translateY(-1px); }
  .st-swatch.on .st-swatch-chip { box-shadow: inset 0 0 0 1px rgba(255,255,255,0.22), 0 0 0 2px var(--surface), 0 0 0 4px color-mix(in srgb, var(--sw) 80%, transparent); }
  .st-swatch-label { font-size: var(--fs-xs); font-weight: 550; color: var(--fg-subtle); transition: color .13s ease; }
  .st-swatch.on .st-swatch-label { color: var(--fg); }

  /* ── Keyboard shortcut table ── */
  .kbd-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); padding: 4px 0; }
  .kbd-row { display: grid; grid-template-columns: 140px 1fr; align-items: center; gap: 12px; padding: 8px 17px; border-bottom: 1px solid var(--border); }
  .kbd-row:last-child { border-bottom: none; }
  @media (min-width: 720px) { .kbd-row:nth-last-child(2):nth-child(even) { border-bottom: none; } }
  .kbd-combo { display: inline-flex; align-items: center; gap: 4px; flex-wrap: wrap; }
  .kbd-plus { color: var(--fg-faint); font-size: 10px; }
  .kbd-label { font-size: var(--fs-sm); color: var(--fg-2); }

  /* ── Speech: pick grid / models / textarea (lifted from legacy) ── */
  .set-pick-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 6px; width: 100%; }
  .set-pick-grid-2 { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .set-pick { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 8px 11px; background: var(--bg-inset); border: 1px solid var(--border); border-radius: var(--radius); color: var(--fg); font: inherit; font-size: var(--fs-sm); cursor: pointer; text-align: left; transition: background 100ms, border-color 100ms; }
  .set-pick:hover { background: var(--surface-hover); }
  .set-pick[data-active="true"] { border-color: color-mix(in oklch, var(--accent) 45%, var(--border)); background: var(--accent-soft); color: var(--accent); }
  .set-pick-label { font-weight: 550; }
  .set-pick-sub { font-size: 10px; color: var(--fg-muted); }
  .set-pick[data-active="true"] .set-pick-sub { color: color-mix(in oklch, var(--accent) 80%, var(--fg-muted)); }
  .set-pick:disabled:not([data-active="true"]) { opacity: 0.5; cursor: not-allowed; }
  .set-model-list { display: flex; flex-direction: column; gap: 6px; padding: 14px 17px; }
  .set-model-row { display: flex; align-items: center; gap: 12px; padding: 10px 12px; background: var(--bg-inset); border: 1px solid var(--border); border-radius: var(--radius); }
  .set-model-row[data-active="true"] { border-color: color-mix(in oklch, var(--accent) 40%, var(--border)); background: var(--accent-soft); }
  .set-model-meta { flex: 1 1 auto; min-width: 0; }
  .set-model-name { font-size: var(--fs-sm); color: var(--fg); font-weight: 550; }
  .set-model-sub { font-size: 10px; color: var(--fg-muted); margin-top: 2px; }
  .set-model-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .set-progress { margin-top: 8px; height: 4px; background: var(--bg-elev-2); border-radius: 2px; overflow: hidden; }
  .set-progress-fill { height: 100%; background: var(--accent); transition: width 120ms linear; }
  .set-progress-label { font-size: 10px; margin-top: 3px; color: var(--fg-muted); }
  .set-mic-r { width: 100%; max-width: 360px; }
  .set-mic-select { flex: 1 1 auto; }
  .set-textarea { width: 100%; padding: 8px 10px; background: var(--bg-inset); border: 1px solid var(--border); border-radius: var(--radius); color: var(--fg); font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5; resize: vertical; min-height: 60px; }
  .set-textarea:focus { outline: none; border-color: var(--border-focus); box-shadow: 0 0 0 3px var(--ring); }
  .set-textarea:disabled { opacity: 0.55; cursor: not-allowed; }

  /* ── Server list ── */
  .srv-list { display: flex; flex-direction: column; }
  .srv-card { display: grid; grid-template-columns: 1fr auto; align-items: center; gap: 16px; background: transparent; border: 0; border-bottom: 1px solid var(--border); color: var(--fg); transition: background 100ms; }
  .srv-card:last-child { border-bottom: none; }
  .srv-card:hover { background: var(--surface-hover); }
  .srv-card[data-active="true"] { background: var(--accent-soft); box-shadow: inset 2px 0 var(--accent); }
  .srv-select { display: grid; grid-template-columns: 1fr auto; align-items: center; gap: 16px; padding: 13px 17px; background: transparent; border: 0; cursor: pointer; color: inherit; font: inherit; text-align: left; width: 100%; min-width: 0; }
  .srv-select:focus-visible { outline: none; box-shadow: inset 0 0 0 2px var(--ring); }
  .srv-l { display: flex; align-items: center; gap: 12px; min-width: 0; }
  .srv-meta-l { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .srv-dot { width: 9px; height: 9px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 0 2px color-mix(in oklch, var(--accent) 18%, transparent); flex-shrink: 0; }
  .srv-name { font-weight: 600; font-size: var(--fs-sm); }
  .srv-host { font-size: var(--fs-xs); color: var(--fg-muted); }
  .srv-meta { font-size: var(--fs-xs); color: var(--fg-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 220px; }
  .srv-r { display: flex; gap: 4px; padding-right: 13px; }

  /* ── Empty state ── */
  .set-empty { padding: 28px 16px; display: flex; flex-direction: column; gap: 4px; align-items: center; text-align: center; }
  .set-empty-title { color: var(--fg); font-size: var(--fs-sm); font-weight: 600; }
  .set-empty-hint { color: var(--fg-muted); font-size: var(--fs-xs); max-width: 280px; line-height: 1.45; }

  /* ── General: workspace identity card ── */
  .st-account { display: flex; align-items: center; gap: 14px; padding: 17px; }
  .st-avatar { width: 50px; height: 50px; border-radius: 13px; display: grid; place-items: center; background: linear-gradient(150deg, var(--accent), color-mix(in oklch, var(--accent) 70%, oklch(0.3 0.05 0))); color: var(--accent-fg); font-weight: 700; font-size: 21px; flex: none; box-shadow: 0 4px 16px -4px color-mix(in oklch, var(--accent) 50%, transparent); }
  .st-account-body { flex: 1; min-width: 0; }
  .st-account-name { font-size: var(--fs-lg); font-weight: 650; }
  .st-account-mail { font-family: var(--font-mono); font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  /* ── Danger zone ── */
  .st-card.danger { border-color: color-mix(in oklch, var(--danger) 30%, var(--border)); }
  .st-danger { background: color-mix(in oklch, var(--danger) 5%, var(--surface)); }
  .st-danger .st-row-label { color: var(--danger); }

  /* ── RCON live console ── */
  .st-console { background: var(--bg-inset); border: 1px solid var(--border); border-radius: var(--r-card); overflow: hidden; }
  .st-console-bar { display: flex; align-items: center; gap: 8px; height: 34px; padding: 0 12px; border-bottom: 1px solid var(--border); background: var(--surface); }
  .st-console-bar :global(svg) { color: var(--fg-subtle); }
  .st-console-title { font-size: var(--fs-xs); font-weight: 650; color: var(--fg-2); flex: 1; }
  .st-console-quick { display: flex; gap: 6px; }
  .st-quick { display: inline-flex; align-items: center; gap: 5px; height: 23px; padding: 0 9px; border-radius: 6px; border: 1px solid var(--border); background: var(--bg); color: var(--fg-muted); font: inherit; font-family: var(--font-mono); font-size: 10px; font-weight: 600; cursor: pointer; }
  .st-quick:hover:not(:disabled) { color: var(--fg); border-color: var(--border-strong); }
  .st-quick:disabled { opacity: 0.4; cursor: not-allowed; }
  .st-console-log { height: 172px; overflow-y: auto; padding: 10px 12px; display: flex; flex-direction: column; gap: 3px; }
  .st-log-line { display: flex; gap: 9px; font-family: var(--font-mono); font-size: 11px; line-height: 1.55; }
  .st-log-time { color: var(--fg-faint); flex: none; }
  .st-log-msg { color: var(--fg-2); }
  .st-log-msg.cmd { color: var(--accent); }
  .st-log-msg.ok { color: var(--ok); }
  .st-log-msg.warn { color: var(--warn); }
  .st-log-msg.sys { color: var(--fg-subtle); }
  .st-console-input { display: flex; align-items: center; gap: 9px; padding: 9px 12px; border-top: 1px solid var(--border); background: var(--surface); }
  .st-console-input .prompt { color: var(--accent); font-family: var(--font-mono); font-weight: 700; }
  .st-console-input input { flex: 1; min-width: 0; background: none; border: 0; outline: 0; color: var(--fg); font: inherit; font-family: var(--font-mono); font-size: 12px; }
  .st-console-input input::placeholder { color: var(--fg-faint); }
  .st-console-input input:disabled { color: var(--fg-faint); }
  .st-console-send { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 7px; border: 0; background: var(--accent-soft); color: var(--accent); cursor: pointer; }
  .st-console-send:hover:not(:disabled) { background: color-mix(in oklch, var(--accent) 20%, var(--surface)); }
  .st-console-send:disabled { opacity: 0.35; cursor: not-allowed; }
  .st-console-offline { height: 172px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; color: var(--fg-muted); text-align: center; padding: 24px; }
  .st-console-offline :global(svg) { color: var(--fg-subtle); }
  .st-console-offline .t { font-size: var(--fs-sm); font-weight: 600; color: var(--fg-2); }
  .st-console-offline .s { font-size: var(--fs-xs); max-width: 260px; line-height: 1.5; }
</style>
