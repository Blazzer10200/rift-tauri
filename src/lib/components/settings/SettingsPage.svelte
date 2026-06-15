<script lang="ts">
  import { untrack, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    Cog, Info, RefreshCw, Sparkles, Palette,
    FolderOpen, Copy, Check, Eye, EyeOff, Mic, Accessibility as A11yIcon,
    CircleCheck, RotateCcw, Trash2, ArrowUpCircle, Loader2,
  } from "lucide-svelte";
  import { appConfigDir, appLogDir } from "@tauri-apps/api/path";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { updates } from "../../state/updates.svelte";
  import { cliUpdate } from "../../state/cliUpdate.svelte";
  import { assistant as assistantStore } from "../../state/assistant.svelte";
  import { stt } from "../../state/stt.svelte";
  import { accessibility } from "../../state/accessibility.svelte";
  import { commandPalette } from "../../state/command-palette.svelte";
  import { uiPrefs, ACCENTS } from "../../state/ui-prefs.svelte";
  import { onboarding } from "../../state/onboarding.svelte";
  import { betaNotice } from "../../state/betaNotice.svelte";
  import { environment } from "../../state/environment.svelte";
  import { scrubUser } from "$lib/utils/redact";
  import { tooltip } from "$lib/actions/tooltip";
  import Select from "../Select.svelte";
  import PageHero from "../shared/PageHero.svelte";

  const DENSITIES = ["compact", "regular", "comfy"] as const;

  type Section = "appearance" | "accessibility" | "assistant" | "speech" | "about";
  const ST_SECTIONS: { id: Section; label: string; icon: typeof Cog; sub: string; dot?: "ok" | "warn" }[] = [
    { id: "appearance",    label: "Appearance",    icon: Palette,  sub: "Theme color, density, code preview, and keyboard shortcuts — applied instantly across Rift." },
    { id: "accessibility", label: "Accessibility", icon: A11yIcon, sub: "Reading-comfort options for the Assistant chat." },
    { id: "assistant",     label: "Assistant",     icon: Sparkles, sub: "Your Claude session and per-turn cost guard." },
    { id: "speech",        label: "Speech",        icon: Mic,      sub: "Voice-to-text input. Web Speech (online) or Whisper (local, accent-tuned)." },
    { id: "about",         label: "About",         icon: Info,     sub: "Build info, file paths, first-run, and support diagnostics." },
  ];

  let activeSec = $state<Section>("appearance");
  let scrollEl = $state<HTMLDivElement>();
  const activeMeta = $derived(ST_SECTIONS.find((s) => s.id === activeSec) ?? ST_SECTIONS[0]);
  const HeroIcon = $derived(activeMeta.icon);

  // Bento layout shows one section per tab — switching resets the scroll.
  function selectSec(id: Section) {
    activeSec = id;
    scrollEl?.scrollTo({ top: 0 });
  }

  // Command-palette deep-link: open the requested tab, then clear (one-shot).
  $effect(() => {
    const req = commandPalette.targetSettingsSection;
    if (req) {
      activeSec = req as Section;
      requestAnimationFrame(() => scrollEl?.scrollTo({ top: 0 }));
      untrack(() => commandPalette.clearSettingsSection());
    }
  });

  let appVersion = $state("?");
  let configDir = $state<string>("");
  let logDir = $state<string>("");

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
  // Optional host tools surfaced in About → Local tools. `use` = what breaks
  // without it; `hint` = how to get it (shown only when missing).
  const LOCAL_TOOLS: { key: "git" | "node" | "npm" | "cargo" | "code"; label: string; use: string; hint: string }[] = [
    { key: "git",   label: "Git",     use: "Version control — powers the assistant's git tools.", hint: "install Git for Windows (git-scm.com)" },
    { key: "node",  label: "Node.js", use: "JavaScript runtime for project tooling.", hint: "install from nodejs.org" },
    { key: "npm",   label: "npm",     use: "Runs frontend project tooling like npm run check.", hint: "ships with Node.js" },
    { key: "cargo", label: "Cargo",   use: "Runs Rust project tooling like cargo check.", hint: "install via rustup.rs" },
    { key: "code",  label: "VS Code", use: "Enables “Open in VS Code” on file paths.", hint: "install VS Code and enable the ‘code’ command in PATH" },
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
  $effect(() => { if (!asstApiKeyDraft) asstApiKeyVisible = false; });

  let asstNowTick = $state(Date.now());
  // Claude Code CLI version state — `isNewer` (not `available`) so Settings
  // always shows the true status even after the toolbar badge was dismissed.
  const fmtCliVer = (v: string | null | undefined) => v?.replace(/\s*\(claude code\)\s*$/i, "") ?? null;
  const cliInstalled = $derived(fmtCliVer(assistantStore.auth?.cliVersion));
  const cliInstalls = $derived(assistantStore.auth?.installs ?? []);
  const cliNewer = $derived(cliUpdate.isAnyStale(assistantStore.auth?.installs, cliInstalled));
  const cliIsNative = $derived((assistantStore.auth?.installMethod ?? null) === "native");
  $effect(() => { cliUpdate.setMethod(assistantStore.auth?.installMethod ?? null); });
  async function runCliUpdate() {
    const ok = await cliUpdate.runUpdate();
    if (ok) await assistantStore.refreshAuth();
  }
  function reprobeAll() {
    void assistantStore.refreshAuth();
    void cliUpdate.maybeCheck(true);
  }
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
      asstApiKeyDraft = "";
      asstApiKeyVisible = false;
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

  const assistantDot = $derived<"ok" | "warn" | undefined>(
    assistantStore.auth?.pill === "green" ? "ok"
      : assistantStore.auth?.pill === "yellow" || assistantStore.auth?.pill === "red" ? "warn" : undefined
  );

  const SHORTCUTS: { combo: string[]; alt?: string[]; label: string }[] = [
    { combo: ["Ctrl", "P"],   alt: ["Ctrl", "K"], label: "Command palette" },
    { combo: ["Ctrl", "T"],   label: "New chat tab" },
    { combo: ["Ctrl", "W"],   label: "Close active tab" },
    { combo: ["Ctrl", "Tab"], label: "Cycle tabs (Shift to reverse)" },
    { combo: ["Alt", "1…9"],  label: "Jump to chat tab N" },
    { combo: ["Ctrl", "1…6"], label: "Switch workspace" },
    { combo: ["Ctrl", "0"],   label: "Switch to Chat workspace" },
    { combo: ["Ctrl", ","],   label: "Open Settings" },
    { combo: ["Ctrl", "\\"],  label: "Split chat pane" },
  ];

  // ── one-time loads on mount (single-scroll: every section is live) ──
  onMount(() => {
    void (async () => {
      try { appVersion = await invoke<string>("app_version"); } catch (e) { console.warn('app_version invoke failed', e); }
    })();
    void assistantStore.init().then(() => {
      asstApiKeyDraft = "";
      asstMaxBudgetDraft = assistantStore.maxBudgetUsd;
    }).catch((e) => console.warn("assistantStore.init failed", e)); // F160: no unhandled rejection
    void stt.init();
    void loadAboutPaths();
    void cliUpdate.maybeCheck();
    void environment.refresh(); // fresh probe each time Settings opens — tools may have just been installed

    asstNowTick = Date.now();
    const iv = setInterval(() => { asstNowTick = Date.now(); }, 30_000);
    return () => {
      clearInterval(iv);
      if (diagCopiedTimer) { clearTimeout(diagCopiedTimer); diagCopiedTimer = null; }
    };
  });
</script>

<div class="sb-main">
  <!-- ── Hero + sticky tab bar ── -->
  <PageHero eyebrow="Settings" title={activeMeta.label} desc={activeMeta.sub} padBottom={false}>
    {#snippet icon()}<HeroIcon size={22} strokeWidth={1.75} />{/snippet}
    {#snippet chip()}
      <span class="sb-chip"><span class="mono">local workspace</span></span>
      <button
        class="sb-chip {updates.summary.kind}"
        type="button"
        onclick={() => updates.open()}
        use:tooltip={"Check for updates"}
      >
        {#if updates.summary.kind === "warn"}
          <ArrowUpCircle size={14} />
        {:else if updates.summary.kind === "busy"}
          <Loader2 size={14} class="spin" />
        {:else}
          <CircleCheck size={14} />
        {/if}
        {appVersion}{updates.summary.label ? ` · ${updates.summary.label}` : ""}
      </button>
    {/snippet}
    {#snippet children()}
      <div class="sb-tabs" role="tablist">
        {#each ST_SECTIONS as s (s.id)}
          {@const Icon = s.icon}
          {@const dot = s.id === "assistant" ? assistantDot : s.dot}
          <button class="sb-tab" class:on={activeSec === s.id} role="tab" aria-selected={activeSec === s.id} onclick={() => selectSec(s.id)} type="button">
            <Icon size={16} strokeWidth={1.75} />
            <span>{s.label}</span>
            {#if dot}<span class="sb-tab-dot {dot}"></span>{/if}
          </button>
        {/each}
      </div>
    {/snippet}
  </PageHero>

  <!-- ── Scrolling bento canvas ── -->
  <div class="sb-scroll" bind:this={scrollEl}>
    <div class="sb-wrap">

      {#if activeSec === "appearance"}
        <div class="sb-bento">

          <div class="st-block sb-s7">
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

          <div class="st-block sb-s5">
            <div class="st-block-label">Interface</div>
            <div class="st-card">
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

          <div class="st-block sb-s5">
            <div class="st-block-label">Code preview</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Font size</div>
                  <div class="st-row-desc">Size of code blocks in Claude's chat replies.</div>
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

          <div class="st-block sb-s7">
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
                      {#if sc.alt}
                        <span class="kbd-or">or</span>
                        {#each sc.alt as k, i}
                          {#if i > 0}<span class="kbd-plus">+</span>{/if}
                          <kbd class="kbd">{k}</kbd>
                        {/each}
                      {/if}
                    </span>
                    <span class="kbd-label">{sc.label}</span>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}

      {#if activeSec === "accessibility"}
        <div class="sb-bento">
          <div class="st-block sb-s8">
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
        </div>
      {/if}

      {#if activeSec === "assistant"}
        <!-- session status promoted to a hero banner — auth + CLI version share one surface -->
        <div class="sb-status {assistantDot ?? 'ok'}">
          <div class="sb-status-l">
            <div class="sb-status-ic">
              {#if assistantStore.auth}<CircleCheck size={18} />{:else}<Loader2 size={18} class="st-spin" />{/if}
            </div>
            <div class="sb-status-main">
              <b>{assistantStore.auth ? assistantStore.auth.summary : assistantStore.authChecking ? "Checking session…" : "Session unknown"}</b>
              <div class="sub">Rift runs your local <code>claude</code> install{#if cliInstalled}{' — '}<code>{cliInstalled}</code>{/if}. Not signed in? Run <code>claude login</code> in a terminal, then re-probe.</div>
            </div>
          </div>
          <div class="sb-status-r">
            {#if cliUpdate.status === "checking"}
              <span class="st-pill"><span class="dot"></span>Checking…</span>
            {:else if cliNewer}
              <span class="st-pill accent"><span class="dot"></span>Update → {cliUpdate.latest}</span>
            {:else if cliUpdate.status === "error"}
              <span class="st-pill warn" use:tooltip={cliUpdate.error ?? "Check failed"}><span class="dot"></span>Check failed</span>
            {:else if cliUpdate.latest}
              <span class="st-pill ok" use:tooltip={cliIsNative ? "Native install — auto-updates in the background; Rift can also apply updates on demand" : "Rift checks npm for newer releases and can update it for you"}><span class="dot"></span>CLI up to date</span>
            {/if}
            {#if assistantStore.authLastProbed && !assistantStore.authChecking}
              <span class="st-stamp" use:tooltip={"Time since the last CLI session probe"}>checked {fmtAgo(assistantStore.authLastProbed, asstNowTick)}</span>
            {/if}
            <button class="st-btn" type="button" onclick={reprobeAll} disabled={assistantStore.authChecking}><RefreshCw size={14} /> Re-probe</button>
          </div>
          {#if cliInstalls.length > 1}
            <div class="st-cli-installs" use:tooltip={"Multiple Claude CLIs found — Rift runs the newest and updates them all so their versions can't drift apart."}>
              {#each cliInstalls as inst (inst.path)}
                {@const stale = cliUpdate.isAnyStale([inst], null)}
                {@const cmd = cliUpdate.commandFor(inst.method)}
                <div class="st-cli-inst" class:active={inst.active}>
                  <span class="st-cli-inst-method">{inst.method}</span>
                  <code>{fmtCliVer(inst.version) ?? "?"}</code>
                  {#if inst.active}<span class="st-cli-inst-tag">active</span>{/if}
                  {#if stale}<span class="st-cli-inst-tag stale">behind</span>{/if}
                  <span class="st-cli-inst-path" use:tooltip={inst.path}>{inst.path}</span>
                  {#if stale}
                    <button class="st-cli-copy sm" class:done={cliUpdate.copiedCmd === cmd} type="button" onclick={() => void cliUpdate.copyValue(cmd)} use:tooltip={"Copy: " + cmd} aria-label="Copy this install's update command">
                      {#if cliUpdate.copiedCmd === cmd}<Check size={12} />{:else}<Copy size={12} />{/if}
                    </button>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
          {#if cliNewer}
            <div class="st-cli-act">
              <button class="st-btn primary" type="button" disabled={cliUpdate.updating} onclick={runCliUpdate}>
                {#if cliUpdate.updating}<Loader2 size={14} class="st-spin" /> Updating…{:else}<ArrowUpCircle size={14} /> Update now{/if}
              </button>
              <div class="st-cli-cmd">
                <code>{cliUpdate.updateCommand}</code>
                <button class="st-cli-copy" class:done={cliUpdate.copied} type="button" onclick={() => void cliUpdate.copyCommand()} use:tooltip={"Copy update command"} aria-label="Copy update command">
                  {#if cliUpdate.copied}<Check size={13} />{:else}<Copy size={13} />{/if}
                </button>
              </div>
            </div>
            {#if cliUpdate.updateError}
              <div class="st-cli-err">{cliUpdate.updateError}</div>
            {:else if cliUpdate.updateOutput}
              <div class="st-cli-ok">{cliUpdate.updateOutput}</div>
            {/if}
            {#if cliUpdate.updateStuck}
              <div class="st-cli-warn">Update ran, but a copy is still behind. A native install sometimes reports success without bumping — copy its command above and run it in a terminal, or reinstall it.</div>
            {/if}
          {/if}
        </div>

        <div class="sb-bento">
          <div class="st-block sb-s7">
            <div class="st-block-label">Claude session</div>
            <div class="st-card">
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
                  <div class="st-row-label">Git tools</div>
                  <div class="st-row-desc">Local <code>git</code> tools for the model. Read-only = status, diff, log. Standard adds commit, pull, and push.</div>
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

          <div class="st-block sb-s5">
            <div class="st-block-label">Cost guard</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <label class="st-row-label" for="asst-budget">Per-turn cost cap</label>
                  <div class="st-row-desc">Stops a turn before it spends more than this dollar amount. Leave blank for no cap.</div>
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
            </div>
          </div>

          <div class="st-block sb-s12">
            <div class="st-block-label">Model &amp; routing</div>
            <div class="st-card">
              <div class="st-row-desc" style="padding:14px 17px 0;">
                By default, turns run on your Claude session above. Setting an <strong>API key</strong> overrides that and bills pay-per-token instead. The key is stored in your OS keychain.
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <label class="st-row-label" for="asst-apikey">API-key fallback</label>
                  <div class="st-row-desc">Bill pay-per-token through the Anthropic Console instead of your Claude session. Overrides the session whenever a key is set. Key turns run the CLI bare — your personal <code>~/.claude</code> config, MCP servers, and CLAUDE.md won't load.</div>
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

        </div>
      {/if}

      {#if activeSec === "speech"}
        <div class="sb-bento">
          <div class="st-block sb-s7">
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
            <div class="st-block sb-s5">
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
            <div class="st-block sb-s12">
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
                          <div class="set-progress" role="progressbar" aria-label={`Downloading ${m.display_name}`} aria-valuemin="0" aria-valuemax={prog.total > 0 ? prog.total : undefined} aria-valuenow={prog.downloaded}>
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
                <div class="st-row">
                  <div class="st-row-body">
                    <div class="st-row-label">Beam search</div>
                    <div class="st-row-desc">Higher-accuracy decode (beam width 5) instead of greedy — sharper on technical terms, ~2-4× slower. GPU recommended.</div>
                  </div>
                  <div class="st-row-ctl">
                    <button class="st-switch" class:on={(stt.config.beam_size ?? 1) > 1} role="switch" aria-checked={(stt.config.beam_size ?? 1) > 1} aria-label="Use beam search decoding" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ beam_size: (stt.config.beam_size ?? 1) > 1 ? null : 5 })}></button>
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
                  <textarea class="set-textarea" rows="4" placeholder="Project names, libraries, jargon — e.g. Tauri, SvelteKit, oklch, my_project" disabled={!stt.config.enabled} value={stt.config.vocab_text} oninput={(e) => void stt.setConfig({ vocab_text: (e.currentTarget as HTMLTextAreaElement).value })}></textarea>
                </div>
              </div>
            </div>
          {/if}

          <div class="st-block sb-s12">
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
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Voice commands</div>
                  <div class="st-row-desc">"send it" fires the message, "new line" / "new paragraph" insert breaks, "scratch that" deletes the last phrase.</div>
                </div>
                <div class="st-row-ctl">
                  <button class="st-switch" class:on={stt.config.voice_commands} role="switch" aria-checked={stt.config.voice_commands} aria-label="Voice commands" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ voice_commands: !stt.config.voice_commands })}></button>
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <div class="st-row-label">Auto-stop on silence</div>
                  <div class="st-row-desc">Ends the recording by itself after a pause — hands-free dictation. Needs live partials on the Web Speech engine.</div>
                </div>
                <div class="st-row-ctl">
                  {#each [{ v: 0, label: "Off" }, { v: 3, label: "3s" }, { v: 5, label: "5s" }, { v: 10, label: "10s" }] as opt (opt.v)}
                    <button type="button" class="set-pick" data-active={stt.config.auto_stop_secs === opt.v} disabled={!stt.config.enabled} onclick={() => void stt.setConfig({ auto_stop_secs: opt.v })}>{opt.label}</button>
                  {/each}
                </div>
              </div>
            </div>
          </div>

          {#if stt.config.engine === "web_speech" && !stt.supported}
            <div class="st-warn sb-s12">Your WebView does not expose <code>SpeechRecognition</code>; Web Speech is unavailable — switch to Whisper or install LLVM and rebuild.</div>
          {/if}
          {#if stt.lastError}<div class="st-warn sb-s12">{stt.lastError}</div>{/if}
        </div>
      {/if}

      {#if activeSec === "about"}
        <div class="sb-bento">
          <div class="st-block sb-s4">
            <div class="st-block-label">Build</div>
            <div class="st-card">
              {#each [["Rift", `${appVersion} · Tauri 2`], ["Engine", "SvelteKit · Svelte 5 (runes)"], ["Style", "Graphite Ink · Tailwind v4 · OKLCH"], ["License", "MIT · github.com/Blazzer10200/rift-releases"]] as kv (kv[0])}
                <div class="st-kv"><span class="st-kv-k">{kv[0]}</span><span class="st-kv-v">{kv[1]}</span></div>
              {/each}
            </div>
          </div>

          <div class="st-block sb-s8">
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

          <div class="st-block sb-s12">
            <div class="st-block-label">Local tools</div>
            <div class="st-card">
              {#each LOCAL_TOOLS as t (t.key)}
                {@const present = environment[t.key]}
                <div class="st-row">
                  <div class="st-row-body">
                    <div class="st-row-label">{t.label}</div>
                    <div class="st-row-desc">{t.use}{present ? "" : ` · ${t.hint}`}</div>
                  </div>
                  <div class="st-row-ctl">
                    <span class="env-stat" class:ok={present} class:warn={!present}>
                      <span class="env-dot"></span>{present ? "Installed" : "Not found"}
                    </span>
                  </div>
                </div>
              {/each}
            </div>
          </div>

          <div class="st-block sb-s12">
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
              <button class="st-about-row" type="button" onclick={() => { onboarding.reset(); betaNotice.reset(); }}>
                <span class="st-about-ic"><RotateCcw size={15} /></span>
                <span class="st-about-body"><span class="st-about-t">Replay first-run walkthrough</span><span class="st-about-s">Shows the welcome walkthrough — including the beta &amp; AI-disclaimer notice — again on next launch</span></span>
              </button>
            </div>
          </div>
        </div>
      {/if}

    </div>
  </div>
</div>

<style>
  .sb-main { position: relative; overflow: hidden; display: flex; flex-direction: column; flex: 1; min-height: 0; min-width: 0; background: var(--bg); color: var(--fg); }

  /* ── Hero tab bar (hero chrome via PageHero component) ── */
  .sb-chip { display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 12px; border-radius: 999px; background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); font: inherit; font-size: var(--fs-xs); font-weight: 600; cursor: default; }
  button.sb-chip { cursor: pointer; transition: background 120ms, border-color 120ms; }
  button.sb-chip:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .sb-chip.ok { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); color: var(--ok); }
  .sb-chip.ok :global(svg) { color: var(--ok); }
  .sb-chip.warn { background: var(--warn-soft); border-color: color-mix(in oklch, var(--warn) 28%, transparent); color: var(--warn); }
  .sb-chip.warn :global(svg) { color: var(--warn); }
  .sb-chip.danger { background: var(--danger-soft); border-color: color-mix(in oklch, var(--danger) 28%, transparent); color: var(--danger); }
  .sb-chip.danger :global(svg) { color: var(--danger); }
  .sb-chip .mono { font-family: var(--font-mono); }

  .sb-tabs { display: flex; gap: 4px; max-width: 820px; margin: 22px auto 0; }
  .sb-tab { display: inline-flex; align-items: center; gap: 9px; height: 42px; padding: 0 15px; border: 0; background: none; color: var(--fg-muted); font: inherit; font-size: var(--fs-md); font-weight: 600; cursor: pointer; position: relative; border-radius: 8px 8px 0 0; transition: color 120ms; }
  .sb-tab:hover { color: var(--fg-2); }
  .sb-tab.on { color: var(--fg); }
  .sb-tab.on::after { content: ""; position: absolute; left: 9px; right: 9px; bottom: -1px; height: 2px; border-radius: 2px; background: var(--accent); box-shadow: 0 0 10px color-mix(in oklab, var(--accent) 55%, transparent); }
  .sb-tab :global(svg) { flex: none; color: var(--fg-subtle); transition: color 120ms; }
  .sb-tab.on :global(svg) { color: var(--accent); }
  .sb-tab-dot { width: 6px; height: 6px; border-radius: 999px; flex: none; }
  .sb-tab-dot.ok { background: var(--ok); box-shadow: 0 0 0 3px var(--ok-soft); }
  .sb-tab-dot.warn { background: var(--warn); box-shadow: 0 0 0 3px var(--warn-soft); }

  /* ── Scrolling bento canvas ── */
  .sb-scroll { flex: 1; min-width: 0; min-height: 0; overflow-y: auto; overflow-x: hidden; scroll-behavior: smooth; }
  .sb-wrap { max-width: 820px; margin: 0 auto; padding: 26px 40px 90px; }
  /* Single readable column — cards stack vertically; no ragged bento bottoms. */
  .sb-bento { display: flex; flex-direction: column; gap: 16px; }
  .sb-bento > .st-block { min-width: 0; }

  /* ── Session status banner ── */
  .sb-status { display: flex; align-items: center; gap: 16px; flex-wrap: wrap; padding: 16px 18px; margin-bottom: 18px; border-radius: var(--r-card); background: linear-gradient(180deg, color-mix(in oklab, var(--ok) 6%, var(--surface)), var(--surface)); border: 1px solid color-mix(in oklab, var(--ok) 20%, var(--border)); }
  .sb-status.warn { background: linear-gradient(180deg, color-mix(in oklab, var(--warn) 7%, var(--surface)), var(--surface)); border-color: color-mix(in oklab, var(--warn) 24%, var(--border)); }
  .sb-status-l { display: flex; align-items: center; gap: 14px; min-width: 0; flex: 1 1 360px; }
  .sb-status-ic { width: 38px; height: 38px; border-radius: 10px; flex: none; display: grid; place-items: center; background: var(--ok-soft); color: var(--ok); }
  .sb-status.warn .sb-status-ic { background: var(--warn-soft); color: var(--warn); }
  .sb-status-main { min-width: 0; }
  .sb-status-main b { font-size: var(--fs-md); font-weight: 680; }
  .sb-status-main .sub { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; line-height: 1.5; }
  .sb-status-r { display: flex; align-items: center; gap: 10px; margin-left: auto; }

  /* Titled-card container system: the section title is a header band inside the card,
     not a label floating above a slab. .st-block IS the card; .st-card is its body. */
  .st-block { display: flex; flex-direction: column; background: var(--surface); border: 1px solid var(--border); border-radius: var(--r-card); box-shadow: inset 0 1px 0 color-mix(in oklab, white 4%, transparent), var(--shadow-sm); }
  .st-block-label { font-size: var(--fs-sm); font-weight: 650; letter-spacing: 0; text-transform: none; color: var(--fg); padding: 13px 17px; border-bottom: 1px solid var(--border); }
  .st-card { background: transparent; border: 0; border-radius: 0; }
  /* Loose (non-row) content sits flush to the body edge — give it the row/header inset. */
  .st-card > .st-row-desc { padding: 14px 17px 0; }
  .st-card > .st-warn { margin: 4px 17px 14px; }
  /* In-card sub-section heading (e.g. "Custom providers" within Model & routing). */

  /* ── Rows ── */
  .st-row { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 16px; padding: 14px 17px; }
  .st-row + .st-row, .st-row + .st-note { border-top: 1px solid var(--border); }
  .st-row-stack { flex-direction: column; align-items: stretch; gap: 10px; }
  .st-row-stack > .st-row-body { flex: 0 0 auto; }
  .st-row-body { flex: 1 1 300px; min-width: 0; }
  .st-row-label { font-size: var(--fs-md); font-weight: 600; color: var(--fg); display: block; }
  .st-row-desc { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; line-height: 1.5; max-width: 60ch; }
  /* Calm inline code: monospace + faint wash, no boxed border — keeps descriptions readable, not noisy. */
  .st-row-desc .mono, .st-row-desc code { font-family: var(--font-mono); color: var(--fg-2); background: color-mix(in oklab, var(--fg) 6%, transparent); padding: 0 4px; border-radius: 4px; font-size: 0.9em; }
  .st-row-ctl { flex: 0 1 auto; margin-left: auto; display: flex; align-items: center; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }
  .st-row[data-disabled="true"] { opacity: 0.55; }
  /* Tool-presence pill (About → Local tools). Dot + label, tinted by status. */
  .env-stat { display: inline-flex; align-items: center; gap: 6px; height: 26px; padding: 0 11px; border-radius: 999px; font-size: var(--fs-xs); font-weight: 600; white-space: nowrap; border: 1px solid transparent; }
  .env-stat .env-dot { width: 7px; height: 7px; border-radius: 999px; flex: none; }
  .env-stat.ok { color: var(--ok); background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); }
  .env-stat.ok .env-dot { background: var(--ok); box-shadow: 0 0 0 3px var(--ok-soft); }
  .env-stat.warn { color: var(--warn); background: var(--warn-soft); border-color: color-mix(in oklch, var(--warn) 28%, transparent); }
  .env-stat.warn .env-dot { background: var(--warn); box-shadow: 0 0 0 3px var(--warn-soft); }
  /* Status rows: label + status-cluster on the top line, description full-width below. */
  /* CLI install list + update CTA live inside the hero banner — span its full width. */
  .sb-status > .st-cli-installs, .sb-status > .st-cli-act, .sb-status > .st-cli-err, .sb-status > .st-cli-ok, .sb-status > .st-cli-warn { flex: 1 1 100%; margin-top: 0; }
  .st-note { padding: 10px 17px; font-size: var(--fs-xs); color: var(--fg-muted); border-top: 1px solid var(--border); }
  .st-note code { font-family: var(--font-mono); background: var(--code-bg); border: 1px solid var(--code-border); padding: 1px 5px; border-radius: 4px; color: var(--code-fg); }
  .st-warn { display: block; font-size: var(--fs-xs); color: var(--warn); line-height: 1.5; padding: 10px 13px; background: var(--warn-soft); border: 1px solid color-mix(in oklab, var(--warn) 32%, transparent); border-radius: var(--r-card); }
  .st-warn code { background: color-mix(in oklab, var(--warn) 16%, transparent); border: 1px solid color-mix(in oklab, var(--warn) 30%, transparent); padding: 1px 5px; border-radius: 4px; color: var(--warn); font-family: var(--font-mono); }

  /* ── Toggle switch ── */
  .st-switch { position: relative; width: 40px; height: 23px; border-radius: 999px; border: 0; padding: 0; background: var(--bg-elev-3); cursor: pointer; transition: background 160ms var(--ease-soft); flex: none; }
  .st-switch::after { content: ""; position: absolute; top: 3px; left: 3px; width: 17px; height: 17px; border-radius: 999px; background: var(--fg-muted); transition: transform 180ms var(--ease-page), background 160ms; }
  .st-switch.on { background: var(--accent); }
  .st-switch.on::after { transform: translateX(17px); background: var(--accent-fg); }
  .st-switch:disabled { opacity: 0.5; cursor: not-allowed; }
  .st-switch:focus-visible { outline: 0; box-shadow: 0 0 0 3px var(--ring); }

  /* ── Segmented ── */
  .st-seg { display: inline-flex; background: var(--track); border: 1px solid var(--border); border-radius: var(--radius); padding: 3px; gap: 2px; }
  .st-seg-btn { height: 26px; padding: 0 12px; border: 0; border-radius: 6px; background: none; color: var(--fg-muted); font: inherit; font-size: var(--fs-xs); font-weight: 600; cursor: pointer; white-space: nowrap; transition: color 120ms, background 120ms; }
  .st-seg-btn:hover:not(:disabled) { color: var(--fg); }
  .st-seg-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .st-seg-btn.on { background: var(--surface-hover); color: var(--fg); box-shadow: var(--shadow-sm); }

  /* ── Text input ── */
  .st-input { height: 32px; padding: 0 12px; border-radius: var(--radius); background: var(--field); border: 1px solid var(--field-border); color: var(--fg); font: inherit; font-size: var(--fs-sm); }
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
  .st-btn.danger-btn { color: var(--danger); border-color: color-mix(in oklab, var(--danger) 30%, var(--border)); }
  .st-btn.danger-btn:hover:not(:disabled) { background: color-mix(in oklab, var(--danger) 12%, var(--surface)); border-color: var(--danger); color: var(--danger); }
  .st-btn :global(svg) { color: currentColor; }
  .st-stamp { font-family: var(--font-mono); font-size: 10.5px; color: var(--fg-faint); white-space: nowrap; }

  /* ── Status pills ── */
  .st-pill { display: inline-flex; align-items: center; gap: 7px; height: 24px; padding: 0 10px; border-radius: 999px; font-size: var(--fs-xs); font-weight: 650; border: 1px solid var(--border); background: var(--surface); color: var(--fg-muted); }
  .st-pill .dot { width: 7px; height: 7px; border-radius: 999px; background: var(--fg-faint); }
  .st-pill.ok { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); color: var(--ok); }
  .st-pill.ok .dot { background: var(--ok); }
  .st-pill.warn { background: var(--warn-soft); border-color: color-mix(in oklab, var(--warn) 28%, transparent); color: var(--warn); }
  .st-pill.warn .dot { background: var(--warn); }
  .st-pill.accent { background: color-mix(in oklab, var(--accent) 12%, transparent); border-color: color-mix(in oklab, var(--accent) 38%, var(--border)); color: var(--accent); font-variant-numeric: tabular-nums; }
  .st-pill.accent .dot { background: var(--accent); }

  /* ── Custom-provider list (2a) ── */

  /* CLI update command line — copyable npm install command. */
  .st-cli-cmd { display: flex; align-items: center; gap: 8px; margin-top: 8px; max-width: 360px; background: color-mix(in oklch, white 9%, var(--surface)); border: 1px solid var(--border-strong); border-radius: 8px; padding: 6px 7px 6px 10px; }
  .st-cli-cmd code { flex: 1; min-width: 0; font-family: var(--font-mono); font-size: 11px; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .st-cli-copy { flex-shrink: 0; display: inline-flex; align-items: center; justify-content: center; width: 26px; height: 24px; border-radius: 6px; border: 1px solid var(--border); background: var(--surface); color: var(--fg-muted); cursor: pointer; transition: color 120ms, border-color 120ms, background 120ms; }
  .st-cli-copy:hover { color: var(--fg); border-color: var(--border-strong); }
  .st-cli-copy.done { color: var(--accent); border-color: color-mix(in oklab, var(--accent) 50%, var(--border)); }
  .st-cli-copy.sm { flex-shrink: 0; width: 22px; height: 20px; border-radius: 5px; }
  .st-cli-installs { display: flex; flex-direction: column; gap: 4px; margin-top: 8px; }
  .st-cli-inst { display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--fg-muted); padding: 4px 8px; border-radius: 6px; background: var(--bg-inset); border: 1px solid var(--border); }
  .st-cli-inst.active { border-color: color-mix(in oklab, var(--accent) 45%, var(--border)); }
  .st-cli-inst-method { text-transform: uppercase; font-size: 9px; letter-spacing: 0.05em; font-weight: 600; color: var(--fg); }
  .st-cli-inst code { font-family: var(--font-mono); color: var(--fg); }
  .st-cli-inst-tag { font-size: 9px; text-transform: uppercase; letter-spacing: 0.04em; padding: 1px 5px; border-radius: 4px; background: color-mix(in oklab, var(--accent) 22%, transparent); color: var(--accent); }
  .st-cli-inst-tag.stale { background: color-mix(in oklab, var(--warn) 22%, transparent); color: var(--warn); }
  .st-cli-inst-path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: right; opacity: 0.6; font-family: var(--font-mono); font-size: 10px; }
  .st-cli-act { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-top: 8px; }
  .st-cli-act .st-cli-cmd { margin-top: 0; }
  .st-btn :global(.st-spin) { animation: st-spin 0.8s linear infinite; }
  @keyframes st-spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .st-btn :global(.st-spin) { animation: none; } }
  .st-cli-err { margin-top: 7px; font-size: var(--fs-xs); color: var(--danger); white-space: pre-wrap; }
  .st-cli-ok { margin-top: 7px; font-size: var(--fs-xs); color: var(--fg-muted); white-space: pre-wrap; }
  .st-cli-warn { margin-top: 7px; font-size: var(--fs-xs); color: var(--warn); line-height: 1.4; }

  /* ── About: kv + resource rows ── */
  .st-kv { display: flex; align-items: center; gap: 16px; padding: 11px 17px; }
  .st-kv + .st-kv { border-top: 1px solid var(--border); }
  .st-kv-k { font-size: var(--fs-md); font-weight: 600; color: var(--fg); flex: none; width: 84px; }
  .st-kv-v { font-family: var(--font-mono); font-size: var(--fs-xs); color: var(--fg-muted); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .st-about-row { display: flex; align-items: center; gap: 12px; padding: 13px 17px; width: 100%; border: 0; background: none; text-align: left; font: inherit; cursor: pointer; }
  .st-about-row + .st-about-row { border-top: 1px solid var(--border); }
  .st-about-ic { width: 32px; height: 32px; border-radius: 9px; display: grid; place-items: center; background: var(--field); border: 1px solid var(--field-border); color: var(--fg-muted); flex: none; }
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
  .kbd-grid { display: flex; flex-direction: column; padding: 4px 0; }
  .kbd-row { display: grid; grid-template-columns: 184px 1fr; align-items: center; gap: 12px; padding: 9px 17px; border-bottom: 1px solid var(--border); }
  .kbd-row:last-child { border-bottom: none; }
  .kbd-combo { display: inline-flex; align-items: center; gap: 4px; flex-wrap: wrap; }
  .kbd-plus { color: var(--fg-faint); font-size: 10px; }
  .kbd-or { color: var(--fg-faint); font-size: 10px; margin: 0 4px; }
  .kbd-label { font-size: var(--fs-sm); color: var(--fg-2); }

  /* ── Speech: pick grid / models / textarea (lifted from legacy) ── */
  .set-pick-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 6px; width: 100%; }
  .set-pick-grid-2 { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .set-pick { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 8px 11px; background: var(--field); border: 1px solid var(--field-border); border-radius: var(--radius); color: var(--fg); font: inherit; font-size: var(--fs-sm); cursor: pointer; text-align: left; transition: background 100ms, border-color 100ms; }
  .set-pick:hover { background: var(--surface-hover); }
  .set-pick[data-active="true"] { border-color: color-mix(in oklab, var(--accent) 45%, var(--border)); background: var(--accent-soft); color: var(--accent); }
  .set-pick-label { font-weight: 550; }
  .set-pick-sub { font-size: 10px; color: var(--fg-muted); }
  .set-pick[data-active="true"] .set-pick-sub { color: color-mix(in oklab, var(--accent) 80%, var(--fg-muted)); }
  .set-pick:disabled:not([data-active="true"]) { opacity: 0.5; cursor: not-allowed; }
  .set-model-list { display: flex; flex-direction: column; gap: 6px; padding: 14px 17px; }
  .set-model-row { display: flex; align-items: center; gap: 12px; padding: 10px 12px; background: var(--field); border: 1px solid var(--field-border); border-radius: var(--radius); }
  .set-model-row[data-active="true"] { border-color: color-mix(in oklab, var(--accent) 40%, var(--border)); background: var(--accent-soft); }
  .set-model-meta { flex: 1 1 auto; min-width: 0; }
  .set-model-name { font-size: var(--fs-sm); color: var(--fg); font-weight: 550; }
  .set-model-sub { font-size: 10px; color: var(--fg-muted); margin-top: 2px; }
  .set-model-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .set-progress { margin-top: 8px; height: 4px; background: var(--bg-elev-2); border-radius: 2px; overflow: hidden; }
  .set-progress-fill { height: 100%; background: var(--accent); transition: width 120ms linear; }
  .set-progress-label { font-size: 10px; margin-top: 3px; color: var(--fg-muted); }
  .set-mic-r { width: 100%; max-width: 360px; }
  .set-mic-select { flex: 1 1 auto; }
  .set-textarea { width: 100%; padding: 8px 10px; background: var(--field); border: 1px solid var(--field-border); border-radius: var(--radius); color: var(--fg); font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5; resize: vertical; min-height: 60px; }
  .set-textarea:focus { outline: none; border-color: var(--border-focus); box-shadow: 0 0 0 3px var(--ring); }
  .set-textarea:disabled { opacity: 0.55; cursor: not-allowed; }
</style>
