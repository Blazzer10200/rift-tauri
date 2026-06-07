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
  import { scrubUser } from "$lib/utils/redact";
  import { tooltip } from "$lib/actions/tooltip";
  import Select from "../Select.svelte";

  const DENSITIES = ["compact", "regular", "comfy"] as const;
  const PRESENCES = ["calm", "bold"] as const;

  type Section = "appearance" | "accessibility" | "assistant" | "speech" | "about";
  const ST_SECTIONS: { id: Section; label: string; icon: typeof Cog; sub: string; dot?: "ok" | "warn" }[] = [
    { id: "appearance",    label: "Appearance",    icon: Palette,  sub: "Theme color, density, code preview, and keyboard shortcuts — applied instantly across Rift." },
    { id: "accessibility", label: "Accessibility", icon: A11yIcon, sub: "Reading-comfort options for the Assistant chat." },
    { id: "assistant",     label: "Assistant",     icon: Sparkles, sub: "Your Claude session, per-turn cost guard, and conversation compaction." },
    { id: "speech",        label: "Speech",        icon: Mic,      sub: "Voice-to-text input. Web Speech (online) or Whisper (local, accent-tuned)." },
    { id: "about",         label: "About",         icon: Info,     sub: "Build info, file paths, first-run, and support diagnostics." },
  ];

  let activeSec = $state<Section>("appearance");
  let scrollEl = $state<HTMLDivElement>();
  // Per-section anchor elements for scroll-spy + jump().
  let secEls = $state<Partial<Record<Section, HTMLElement>>>({});

  function onScroll() {
    const sc = scrollEl;
    if (!sc) return;
    // Bottom-of-scroll: the last (often short) section can't reach the 140px
    // threshold before the container bottoms out, so spy it explicitly.
    if (sc.scrollTop + sc.clientHeight >= sc.scrollHeight - 2) {
      activeSec = ST_SECTIONS[ST_SECTIONS.length - 1].id;
      return;
    }
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
      activeSec = req as Section;
      // wait a frame so anchors are mounted before scrolling
      requestAnimationFrame(() => jump(req as Section));
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

  let asstBaseUrlDraft = $state("");
  let asstProviderModelDraft = $state("");
  let asstProviderSaving = $state(false);
  let asstProviderMsg = $state<string | null>(null);
  const asstProviderDirty = $derived(
    asstBaseUrlDraft.trim() !== (assistantStore.baseUrl ?? "") ||
    asstProviderModelDraft.trim() !== (assistantStore.providerModel ?? ""),
  );
  let asstProviderSeeded = false;
  $effect(() => {
    if (assistantStore.providerConfigLoaded && !asstProviderSeeded) {
      asstProviderSeeded = true;
      asstBaseUrlDraft = assistantStore.baseUrl ?? "";
      asstProviderModelDraft = assistantStore.providerModel ?? "";
    }
  });

  let asstNowTick = $state(Date.now());
  // Claude Code CLI version state — `isNewer` (not `available`) so Settings
  // always shows the true status even after the toolbar badge was dismissed.
  const cliInstalled = $derived(assistantStore.auth?.cliVersion ?? null);
  const cliInstalls = $derived(assistantStore.auth?.installs ?? []);
  const cliNewer = $derived(cliUpdate.isAnyStale(assistantStore.auth?.installs, cliInstalled));
  const cliIsNative = $derived((assistantStore.auth?.installMethod ?? null) === "native");
  $effect(() => { cliUpdate.setMethod(assistantStore.auth?.installMethod ?? null); });
  async function runCliUpdate() {
    const ok = await cliUpdate.runUpdate();
    if (ok) await assistantStore.refreshAuth();
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
  async function saveAsstProvider() {
    asstProviderSaving = true;
    asstProviderMsg = null;
    try {
      await assistantStore.setBaseUrl(asstBaseUrlDraft);
      await assistantStore.setProviderModel(asstProviderModelDraft);
      asstProviderMsg = assistantStore.baseUrl ? "Saved." : "Cleared.";
    } catch (e) {
      asstProviderMsg = `Failed: ${e}`;
    } finally {
      asstProviderSaving = false;
    }
  }
  function clearAsstProvider() {
    asstBaseUrlDraft = "";
    asstProviderModelDraft = "";
    void saveAsstProvider();
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
          Workspace · <span class="mono">local</span>
        </div>
      </div>
      <div class="st-index-list">
        {#each ST_SECTIONS as s (s.id)}
          {@const Icon = s.icon}
          {@const dot = s.id === "assistant" ? assistantDot : s.dot}
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

        <!-- ── APPEARANCE ── -->
        <section class="st-sec" bind:this={secEls.appearance}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><Palette size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">Appearance</div><div class="st-sec-sub">{ST_SECTIONS[0].sub}</div></div>
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
        </section>

        <!-- ── ACCESSIBILITY ── -->
        <section class="st-sec" bind:this={secEls.accessibility}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><A11yIcon size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">Accessibility</div><div class="st-sec-sub">{ST_SECTIONS[1].sub}</div></div>
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
            <div><div class="st-sec-tt">Assistant</div><div class="st-sec-sub">{ST_SECTIONS[2].sub}</div></div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Claude session</div>
            <div class="st-card">
              <div class="st-row st-srow">
                <div class="st-srow-top">
                  <div class="st-row-label">Status</div>
                  <div class="st-srow-ctl">
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
                <div class="st-row-desc">Rift uses your local <code>claude</code> CLI session by default. Not signed in? Run <code>claude login</code> in a terminal, then re-probe.</div>
              </div>
              <div class="st-row st-srow">
                <div class="st-srow-top">
                  <div class="st-row-label">Claude Code CLI</div>
                  <div class="st-srow-ctl">
                    {#if cliUpdate.status === "checking"}
                      <span class="st-pill"><span class="dot"></span>Checking…</span>
                    {:else if cliNewer}
                      <span class="st-pill accent"><span class="dot"></span>Update → {cliUpdate.latest}</span>
                    {:else if cliUpdate.status === "error"}
                      <span class="st-pill warn" use:tooltip={cliUpdate.error ?? "Check failed"}><span class="dot"></span>Check failed</span>
                    {:else if cliUpdate.latest}
                      <span class="st-pill ok"><span class="dot"></span>Up to date</span>
                    {:else}
                      <span class="st-pill"><span class="dot"></span>{cliInstalled ?? "—"}</span>
                    {/if}
                    {#if cliUpdate.checkedAt && cliUpdate.status !== "checking"}
                      <span class="st-stamp" use:tooltip={"Time since the last npm registry check"}>checked {fmtAgo(cliUpdate.checkedAt, asstNowTick)}</span>
                    {/if}
                    <button class="st-btn" type="button" onclick={() => void cliUpdate.maybeCheck(true)} disabled={cliUpdate.status === "checking"}><RefreshCw size={14} /> Check</button>
                  </div>
                </div>
                <div class="st-row-desc">
                  Rift spawns your local <code>claude</code> install{#if cliInstalled} (currently <code>{cliInstalled}</code>){/if}{#if cliIsNative} via the native installer{:else if cliUpdate.method === "npm"} via npm{/if}.
                  {#if cliIsNative}It auto-updates in the background — Rift can also apply updates on demand.{:else}Rift checks npm for newer releases and can update it for you.{/if}
                </div>
                {#if cliInstalls.length > 1}
                  <div class="st-cli-installs" use:tooltip={"Multiple Claude CLIs found — Rift runs the newest and updates them all so their versions can't drift apart."}>
                    {#each cliInstalls as inst (inst.path)}
                      {@const stale = cliUpdate.isAnyStale([inst], null)}
                      {@const cmd = cliUpdate.commandFor(inst.method)}
                      <div class="st-cli-inst" class:active={inst.active}>
                        <span class="st-cli-inst-method">{inst.method}</span>
                        <code>{inst.version ?? "?"}</code>
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
            <div class="st-block-label">Custom provider (advanced)</div>
            <div class="st-card">
              <div class="st-row">
                <div class="st-row-body">
                  <label class="st-row-label" for="asst-baseurl">API base URL</label>
                  <div class="st-row-desc">Route turns to an Anthropic-compatible endpoint (e.g. DeepSeek <code>https://api.deepseek.com/anthropic</code>) using the API key above. Blank = Anthropic. Lets headless turns draw on a cheaper provider instead of the metered subscription credits (June 15 change).</div>
                </div>
                <div class="st-row-ctl">
                  <input id="asst-baseurl" class="st-input mono" type="text" placeholder="https://api.deepseek.com/anthropic" style="width:248px;" bind:value={asstBaseUrlDraft} autocomplete="off" spellcheck="false" />
                </div>
              </div>
              <div class="st-row">
                <div class="st-row-body">
                  <label class="st-row-label" for="asst-provider-model">Provider model</label>
                  <div class="st-row-desc">Model id passed to <code>--model</code> (e.g. <code>deepseek-chat</code>). Required for direct providers; gateways may map Rift's tiers.</div>
                </div>
                <div class="st-row-ctl">
                  <input id="asst-provider-model" class="st-input mono" type="text" placeholder="deepseek-chat" style="width:188px;" bind:value={asstProviderModelDraft} autocomplete="off" spellcheck="false" />
                  <button class="st-btn primary" type="button" onclick={saveAsstProvider} disabled={asstProviderSaving || !asstProviderDirty}>{asstProviderSaving ? "Saving…" : "Save"}</button>
                  {#if assistantStore.baseUrl}
                    <button class="st-btn" type="button" disabled={asstProviderSaving} onclick={clearAsstProvider}>Clear</button>
                  {/if}
                </div>
              </div>
              {#if asstProviderMsg}<div class="st-note">{asstProviderMsg}</div>{/if}
              {#if assistantStore.baseUrl}
                <div class="st-note">Active — turns route to <code>{assistantStore.baseUrl}</code>{assistantStore.providerModel ? ` · ${assistantStore.providerModel}` : ""}.</div>
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
            <div><div class="st-sec-tt">Speech</div><div class="st-sec-sub">{ST_SECTIONS[3].sub}</div></div>
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

        <!-- ── ABOUT ── -->
        <section class="st-sec" bind:this={secEls.about}>
          <div class="st-sec-head">
            <div class="st-sec-ic"><Info size={19} strokeWidth={1.75} /></div>
            <div><div class="st-sec-tt">About</div><div class="st-sec-sub">{ST_SECTIONS[4].sub}</div></div>
          </div>

          <div class="st-block">
            <div class="st-block-label">Build</div>
            <div class="st-card">
              {#each [["Rift", `${appVersion} · Tauri 2`], ["Engine", "SvelteKit · Svelte 5 (runes)"], ["Style", "Graphite Ink · Tailwind v4 · OKLCH"], ["License", "MIT · github.com/Blazzer10200/rift"]] as kv (kv[0])}
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
              <button class="st-about-row" type="button" onclick={() => { onboarding.reset(); betaNotice.reset(); }}>
                <span class="st-about-ic"><RotateCcw size={15} /></span>
                <span class="st-about-body"><span class="st-about-t">Replay first-run walkthrough</span><span class="st-about-s">Shows the welcome walkthrough — including the beta &amp; AI-disclaimer notice — again on next launch</span></span>
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
  .st-index-row.on::before { content: ""; position: absolute; left: 0; top: 8px; bottom: 8px; width: 3px; border-radius: 2px; background: var(--accent); box-shadow: 0 0 8px color-mix(in oklab, var(--accent) 45%, transparent); }
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
  .st-doc { max-width: 760px; margin: 0; padding: 32px 56px 96px; display: flex; flex-direction: column; gap: 40px; }
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
  .st-row { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 16px; padding: 14px 17px; }
  .st-row + .st-row, .st-card .st-note + .st-row, .st-row + .st-note { border-top: 1px solid var(--border); }
  .st-row-stack { flex-direction: column; align-items: stretch; gap: 10px; }
  .st-row-stack > .st-row-body { flex: 0 0 auto; }
  .st-row-body { flex: 1 1 300px; min-width: 0; }
  .st-row-label { font-size: var(--fs-md); font-weight: 600; color: var(--fg); display: block; }
  .st-row-desc { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 3px; line-height: 1.5; }
  .st-row-desc .mono, .st-row-desc code { font-family: var(--font-mono); color: var(--code-fg); background: var(--code-bg); border: 1px solid var(--code-border); padding: 1px 5px; border-radius: 4px; font-size: 0.92em; }
  .st-row-ctl { flex: 0 1 auto; margin-left: auto; display: flex; align-items: center; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }
  .st-row[data-disabled="true"] { opacity: 0.55; }
  /* Status rows: label + status-cluster on the top line, description full-width below. */
  .st-srow { flex-direction: column; align-items: stretch; gap: 10px; }
  .st-srow-top { display: flex; align-items: center; flex-wrap: wrap; gap: 8px 12px; }
  .st-srow-top .st-row-label { flex: 1 1 auto; }
  .st-srow-ctl { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; margin-left: auto; justify-content: flex-end; }
  .st-srow .st-row-desc { margin-top: 0; }
  .st-srow .st-cli-cmd { margin-top: 0; }
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
  .st-cli-err { margin-top: 7px; font-size: var(--fs-xs); color: var(--danger, #e66); white-space: pre-wrap; }
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
