<script lang="ts">
  import "$lib/styles/settings-controls.css";
  import { untrack, onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    Cog, Info, RefreshCw, Sparkles, Palette,
    FolderOpen, Copy, Check, Eye, EyeOff, Mic,
    CircleCheck, RotateCcw, Trash2, ArrowUpCircle, Loader2,
    Download, ShieldCheck, ExternalLink, Bug, Terminal,
    Activity, MessageSquare, Search, CornerDownLeft, ChevronDown,
  } from "lucide-svelte";
  import { appConfigDir, appLogDir } from "@tauri-apps/api/path";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { openPath, openUrl } from "@tauri-apps/plugin-opener";
  import { updates } from "../../state/updates.svelte";
  import { cliUpdate, cmpSemver, CLI_RECOMMENDED_VERSION } from "../../state/cliUpdate.svelte";
  import { assistant as assistantStore } from "../../state/assistant.svelte";
  import { stt, isLocalEngine, type ModelInfo } from "../../state/stt.svelte";
  import { accessibility } from "../../state/accessibility.svelte";
  import { commandPalette } from "../../state/command-palette.svelte";
  import { uiPrefs, ACCENTS, TOOL_DETAILS, DENSITY_PRESETS, VIVIDNESS_MIN, VIVIDNESS_MAX } from "../../state/ui-prefs.svelte";
  import { onboarding } from "../../state/onboarding.svelte";
  import { betaNotice } from "../../state/betaNotice.svelte";
  import { environment } from "../../state/environment.svelte";
  import { elevation } from "../../state/elevation.svelte";
  import { scrubUser } from "$lib/utils/redact";
  import { tooltip } from "$lib/actions/tooltip";
  import { sliderBubble } from "$lib/actions/sliderBubble";
  import { diagnostics } from "../../state/diagnostics.svelte";
  import DiagnosticsConsole from "../diagnostics/DiagnosticsConsole.svelte";
  import Select from "../Select.svelte";
  import PageHero from "../shared/PageHero.svelte";

  const DENSITIES = ["compact", "regular", "comfy"] as const;
  const NARRATIONS = [
    { id: "focused", label: "Focused" },
    { id: "balanced", label: "Balanced" },
    { id: "chatty", label: "Chatty" },
  ] as const;

  const COMMAND_OUTPUTS = [
    { id: "minimal", label: "Minimal" },
    { id: "peek", label: "Peek" },
    { id: "full", label: "Full" },
  ] as const;

  type Section = "appearance" | "chat" | "claude" | "speech" | "about";
  const ST_SECTIONS: { id: Section; label: string; icon: typeof Cog; sub: string; dot?: "ok" | "warn" }[] = [
    { id: "appearance", label: "Appearance", icon: Palette,       sub: "Accent color, density, and code rendering — every change applies instantly." },
    { id: "chat",       label: "Chat",       icon: MessageSquare, sub: "How conversations read — stream layout, detail level, and reading comfort." },
    { id: "claude",     label: "Claude",     icon: Sparkles,      sub: "Your Claude session, plan, and API-key fallback." },
    { id: "speech",     label: "Speech",     icon: Mic,           sub: "Voice-to-text input — Web Speech (online), Parakeet (on-device, fast) or Whisper (on-device, multilingual)." },
    { id: "about",      label: "About",      icon: Info,          sub: "Build info, keyboard shortcuts, local tools, and support diagnostics." },
  ];

  let activeSec = $state<Section>("appearance");
  let scrollEl = $state<HTMLDivElement>();

  // ── Settings search — every control indexed, jump-and-flash on pick ──
  // `anchor` is the card's DOM id inside the scroll surface; control-level
  // entries point at their parent card. Engine-dependent Speech cards anchor
  // to the always-rendered Engine card so a jump never lands nowhere.
  type SearchEntry = { tab: Section; anchor: string; card: string; title: string; kw: string };
  const SEARCH_INDEX: SearchEntry[] = [
    { tab: "appearance", anchor: "card-accent",    card: "Accent color",      title: "Accent color",        kw: "theme hue swatch color highlight" },
    { tab: "appearance", anchor: "card-accent",    card: "Accent color",      title: "Vividness",           kw: "saturation intensity accent" },
    { tab: "appearance", anchor: "card-interface", card: "Interface & code",  title: "Interface density",   kw: "spacing compact comfy regular rows" },
    { tab: "appearance", anchor: "card-interface", card: "Interface & code",  title: "Code font size",      kw: "monospace px code blocks" },
    { tab: "appearance", anchor: "card-interface", card: "Interface & code",  title: "Tab width",           kw: "indentation spaces code" },
    { tab: "appearance", anchor: "card-interface", card: "Interface & code",  title: "Font ligatures",      kw: "glyphs jetbrains mono arrows" },
    { tab: "chat",       anchor: "card-rendering", card: "Chat rendering",    title: "Stream view",         kw: "boxless activity layout working header" },
    { tab: "chat",       anchor: "card-rendering", card: "Chat rendering",    title: "Density preset",      kw: "calm standard verbose detail" },
    { tab: "chat",       anchor: "card-rendering", card: "Chat rendering",    title: "Tool detail",         kw: "minimal balanced detailed collapse rows" },
    { tab: "chat",       anchor: "card-rendering", card: "Chat rendering",    title: "Narration",           kw: "commentary focused chatty prose" },
    { tab: "chat",       anchor: "card-rendering", card: "Chat rendering",    title: "Command output",      kw: "terminal shell peek full minimal" },
    { tab: "chat",       anchor: "card-comfort",   card: "Reading comfort",   title: "Dyslexia-friendly mode", kw: "lexend accessibility font reading" },
    { tab: "chat",       anchor: "card-comfort",   card: "Reading comfort",   title: "Warm reading tint",   kw: "sepia glare eyes overlay" },
    { tab: "chat",       anchor: "card-comfort",   card: "Reading comfort",   title: "Line and letter spacing", kw: "line height readability" },
    { tab: "claude",     anchor: "card-session",   card: "Claude session",    title: "Use my full Claude Code config", kw: "claude.md hooks mcp skills settings piggyback sandbox" },
    { tab: "claude",     anchor: "card-session",   card: "Claude session",    title: "Git tools",           kw: "read-only standard commit push trust" },
    { tab: "claude",     anchor: "card-session",   card: "Claude session",    title: "Plan",                kw: "free pro max context window 200k 1m gauge subscription" },
    { tab: "claude",     anchor: "card-admin",     card: "Administrator access", title: "Relaunch as administrator", kw: "admin elevated elevation uac privileges run as administrator sudo" },
    { tab: "claude",     anchor: "card-admin",     card: "Administrator access", title: "Always run as administrator", kw: "admin elevated elevation uac no prompt scheduled task startup" },
    { tab: "claude",     anchor: "card-api",       card: "API key & spending", title: "API-key fallback",   kw: "anthropic console token sk-ant billing keychain" },
    { tab: "claude",     anchor: "card-api",       card: "API key & spending", title: "Per-turn cost cap",  kw: "budget dollar limit spend guard" },
    { tab: "speech",     anchor: "card-engine",    card: "Engine",             title: "Speech-to-text",     kw: "voice mic dictation stt enable" },
    { tab: "speech",     anchor: "card-engine",    card: "Engine",             title: "Recognition engine", kw: "web speech whisper parakeet on-device azure" },
    { tab: "speech",     anchor: "card-engine",    card: "Web Speech",         title: "Language",           kw: "english spanish french german locale bcp-47" },
    { tab: "speech",     anchor: "card-engine",    card: "Parakeet",           title: "Parakeet model",     kw: "download on-device offline fast nvidia tdt local" },
    { tab: "speech",     anchor: "card-engine",    card: "Whisper",            title: "Whisper model",      kw: "download tiny base small medium accuracy" },
    { tab: "speech",     anchor: "card-engine",    card: "Whisper",            title: "Input device",       kw: "microphone capture audio" },
    { tab: "speech",     anchor: "card-engine",    card: "Whisper",            title: "Vocabulary priming", kw: "jargon names initial prompt style" },
    { tab: "speech",     anchor: "card-composer",  card: "Composer integration", title: "Voice commands",   kw: "send it new line scratch that" },
    { tab: "speech",     anchor: "card-composer",  card: "Composer integration", title: "Auto-stop on silence", kw: "pause hands-free recording" },
    { tab: "speech",     anchor: "card-composer",  card: "Composer integration", title: "Live partial transcripts", kw: "interim words appear speak" },
    { tab: "about",      anchor: "card-build",     card: "Build & install",    title: "Version & paths",    kw: "build config logs folders license update github source mit stack" },
    { tab: "about",      anchor: "card-shortcuts", card: "Keyboard shortcuts", title: "Keyboard shortcuts", kw: "hotkeys ctrl palette tabs split" },
    { tab: "about",      anchor: "card-tools",     card: "Local tools",        title: "Local tools",        kw: "git node npm cargo vs code install winget" },
    { tab: "about",      anchor: "card-help",      card: "Help & diagnostics", title: "Repair installation", kw: "reinstall corrupted fix" },
    { tab: "about",      anchor: "card-help",      card: "Help & diagnostics", title: "Diagnostics console", kw: "events logs export bug report copy" },
    { tab: "about",      anchor: "card-help",      card: "Help & diagnostics", title: "Replay first-run walkthrough", kw: "onboarding intro welcome tour" },
    { tab: "about",      anchor: "card-help",      card: "Help & diagnostics", title: "Report a bug", kw: "bug issue github feedback problem broken" },
  ];
  let searchQ = $state("");
  let searchIdx = $state(0);
  let searchEl = $state<HTMLInputElement>();
  // "/" or Ctrl+F anywhere on the page focuses the settings search.
  function onGlobalKey(ev: KeyboardEvent) {
    const t = ev.target as HTMLElement | null;
    if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable)) return;
    if ((ev.key === "/" && !ev.ctrlKey && !ev.altKey && !ev.metaKey) ||
        (ev.ctrlKey && !ev.shiftKey && !ev.altKey && ev.key.toLowerCase() === "f")) {
      ev.preventDefault();
      searchEl?.focus();
    }
  }
  const searchResults = $derived.by(() => {
    const q = searchQ.trim().toLowerCase();
    if (!q) return [];
    const scored: { e: SearchEntry; s: number }[] = [];
    for (const e of SEARCH_INDEX) {
      const title = e.title.toLowerCase();
      const hay = `${title} ${e.card.toLowerCase()} ${e.kw}`;
      let s: number | null = null;
      if (title.startsWith(q)) s = 3000 - title.length;
      else if (title.includes(q)) s = 2000 - title.indexOf(q);
      else if (hay.includes(q)) s = 1000 - hay.indexOf(q);
      if (s !== null) scored.push({ e, s });
    }
    scored.sort((a, b) => b.s - a.s);
    return scored.slice(0, 8).map((x) => x.e);
  });
  $effect(() => { void searchResults.length; searchIdx = 0; });
  function jumpTo(e: SearchEntry) {
    searchQ = "";
    activeSec = e.tab;
    // Two frames: one for the tab's cards to mount, one for layout to settle.
    requestAnimationFrame(() => requestAnimationFrame(() => {
      const el = scrollEl?.querySelector<HTMLElement>(`#${e.anchor}`);
      if (!el) return;
      el.scrollIntoView({ block: "start", behavior: "smooth" });
      el.classList.add("sflash");
      setTimeout(() => el.classList.remove("sflash"), 1700);
    }));
  }
  function onSearchKey(ev: KeyboardEvent) {
    if (searchResults.length === 0) {
      if (ev.key === "Escape") { searchQ = ""; (ev.currentTarget as HTMLInputElement).blur(); }
      return;
    }
    if (ev.key === "ArrowDown") { ev.preventDefault(); searchIdx = (searchIdx + 1) % searchResults.length; }
    else if (ev.key === "ArrowUp") { ev.preventDefault(); searchIdx = (searchIdx - 1 + searchResults.length) % searchResults.length; }
    else if (ev.key === "Enter") { ev.preventDefault(); jumpTo(searchResults[searchIdx]); }
    else if (ev.key === "Escape") { searchQ = ""; }
  }
  const activeMeta = $derived(ST_SECTIONS.find((s) => s.id === activeSec) ?? ST_SECTIONS[0]);

  // Switching tabs resets the scroll position.
  function selectSec(id: Section) {
    activeSec = id;
    scrollEl?.scrollTo({ top: 0 });
  }

  const vivPct = $derived(Math.round(((uiPrefs.vividness - VIVIDNESS_MIN) / (VIVIDNESS_MAX - VIVIDNESS_MIN)) * 100));

  // Command-palette deep-link: open the requested tab, then clear (one-shot).
  $effect(() => {
    const req = commandPalette.targetSettingsSection;
    if (req) {
      activeSec = req;
      requestAnimationFrame(() => scrollEl?.scrollTo({ top: 0 }));
      untrack(() => commandPalette.clearSettingsSection());
    }
  });

  let appVersion = $state("?");
  let configDir = $state<string>("");
  let logDir = $state<string>("");

  let diagCopied = $state(false);
  let diagCopiedTimer: ReturnType<typeof setTimeout> | null = null;
  let diagConsoleOpen = $state(false);

  async function loadAboutPaths() {
    try { configDir = await appConfigDir(); } catch (e) { console.warn("appConfigDir failed", e); }
    try { logDir = await appLogDir(); } catch (e) { console.warn("appLogDir failed", e); }
  }
  async function openDir(p: string) {
    if (!p) return;
    try { await openPath(p); } catch (e) { console.error("openPath failed", e); }
  }
  async function openClaudeCode() {
    try { await openUrl("https://claude.com/claude-code"); } catch (e) { console.error("openUrl failed", e); }
  }
  async function openRepo() {
    try { await openUrl("https://github.com/Blazzer10200/rift-tauri"); } catch (e) { console.error("openUrl failed", e); }
  }
  // Report-a-bug journey: diagnostic lands on the clipboard first, so the new
  // GitHub issue is one paste away from being useful.
  async function reportBug() {
    await copyDiagnostic();
    try { await openUrl("https://github.com/Blazzer10200/rift-tauri/issues/new"); } catch (e) { console.error("openUrl failed", e); }
  }
  // Resetting the flags remounts the walkthrough immediately — the takeover IS
  // the click feedback.
  function replayWalkthrough() {
    onboarding.reset();
    betaNotice.reset();
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

  // ── Per-tab factory reset. Defaults live in each store's reset method so
  // they can't drift from the store's own initial values. About has no reset —
  // it's info and actions, not preferences.
  const RESET_COPY: Partial<Record<Section, string>> = {
    appearance: "Accent, density, and code rendering go back to the stock emerald look.",
    chat: "Stream layout, detail dials, and reading comfort go back to their defaults.",
    claude: "Full config on, git tools read-only, plan Max. Your API key and spending cap are kept.",
    speech: "All voice-input settings return to factory defaults. Downloaded Whisper models stay on disk.",
  };
  // In-flight latch: stacked confirm() calls (double-click, programmatic) leave
  // orphaned native dialogs whose promises never settle — allow exactly one.
  let resetBusy = false;
  async function resetTab(sec: Section) {
    if (resetBusy) return;
    resetBusy = true;
    let ok = false;
    try {
      const label = ST_SECTIONS.find((s) => s.id === sec)?.label ?? sec;
      ok = await confirm(`Reset all ${label} settings to their defaults?\n\n${RESET_COPY[sec] ?? ""}`,
        { title: `Reset ${label}`, kind: "warning", okLabel: "Reset", cancelLabel: "Cancel" });
    } finally {
      resetBusy = false;
    }
    if (!ok) return;
    try {
      if (sec === "appearance") uiPrefs.resetAppearance();
      else if (sec === "chat") { uiPrefs.resetChatRendering(); accessibility.reset(); }
      else if (sec === "claude") await assistantStore.resetSessionDefaults();
      else if (sec === "speech") await stt.resetConfig();
    } catch (e) {
      console.error(`reset ${sec} failed`, e); // store already toasted the failure
    }
  }

  async function repairInstall() {
    const ok = await confirm(
      "Repair will re-download and reinstall the current version, then restart Rift. Continue?",
      { title: "Repair installation", kind: "warning", okLabel: "Repair", cancelLabel: "Cancel" },
    );
    if (!ok) return;
    await updates.repair();
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
    { key: "git",   label: "Git",     use: "Version control — powers the assistant's git tools.", hint: "Install Git for Windows (git-scm.com)." },
    { key: "node",  label: "Node.js", use: "JavaScript runtime for project tooling.", hint: "Install from nodejs.org." },
    { key: "npm",   label: "npm",     use: "Runs frontend project tooling like npm run check.", hint: "Ships with Node.js." },
    { key: "cargo", label: "Cargo",   use: "Runs Rust project tooling like cargo check.", hint: "Install via rustup.rs." },
    { key: "code",  label: "VS Code", use: "Enables “Open in VS Code” on file paths.", hint: "Install VS Code and enable the ‘code’ command in PATH." },
  ];
  const toolsInstalled = $derived(LOCAL_TOOLS.filter((t) => environment[t.key]));
  const toolsMissing = $derived(LOCAL_TOOLS.filter((t) => !environment[t.key]));

  const STT_ENGINES: { id: "web_speech" | "whisper" | "parakeet"; label: string; sub: string }[] = [
    { id: "web_speech", label: "Web Speech", sub: "Edge · Azure when online" },
    { id: "parakeet",   label: "Parakeet",   sub: "On-device · fast · any GPU" },
    { id: "whisper",    label: "Whisper",    sub: "On-device · multilingual" },
  ];
  const parakeetModels = $derived(stt.models.filter((m) => m.engine === "parakeet"));
  const whisperModels = $derived(stt.models.filter((m) => m.engine === "whisper"));
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
  // Active CLI present but its `--version` couldn't be read → backend gates it
  // as conservative-old (cli_caps). Surface that so a degraded session isn't a
  // silent mystery. Only when a session actually resolved (auth known).
  const cliVersionUnknown = $derived(!!assistantStore.auth && !assistantStore.auth.cliVersion && cliInstalls.length <= 1);
  // P2 (#45): readable version, at/above the hard floor (a turn can run) but
  // below the version where every spawn flag is available — one calm line, not
  // a warning (the gap is at most one internal flag; nothing is broken).
  const cliBelowFeatureFloor = $derived(
    !!assistantStore.auth?.cliVersion &&
      cmpSemver(assistantStore.auth.cliVersion, CLI_RECOMMENDED_VERSION) < 0,
  );
  const cliNewer = $derived(cliUpdate.isAnyStale(assistantStore.auth?.installs, cliInstalled));
  // Per-feed update target — what the pill/banner advertises. A native install
  // targets the native channel's latest, not npm's (which routinely runs ahead).
  const cliTarget = $derived(cliUpdate.targetFor(assistantStore.auth?.installs, cliInstalled));
  const cliSummary = $derived(cliUpdate.summary(assistantStore.auth?.installs));
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
      console.error("setApiKey failed", e);
      asstApiKeyMsg = "Couldn't save the API key. See logs for details.";
    } finally {
      asstApiKeySaving = false;
    }
  }
  async function saveAsstMaxBudget() {
    // B11: a typed 0/negative is invalid input, not a clear — the Clear button
    // is the explicit no-cap path (sets draft to null). Reject it loudly
    // instead of silently coercing to null + a misleading "Cleared" success.
    if (asstMaxBudgetDraft !== null && !(asstMaxBudgetDraft > 0)) {
      asstMaxBudgetMsg = "Enter an amount above 0, or use Clear to remove the cap.";
      return;
    }
    asstMaxBudgetSaving = true;
    asstMaxBudgetMsg = null;
    try {
      await assistantStore.setMaxBudgetUsd(asstMaxBudgetDraft);
      asstMaxBudgetMsg = assistantStore.maxBudgetUsd != null ? `Saved: ${assistantStore.maxBudgetUsd.toFixed(2)} cap.` : "Cleared (no cap).";
    } catch (e) {
      console.error("setMaxBudgetUsd failed", e);
      asstMaxBudgetMsg = "Couldn't save the budget cap. See logs for details.";
    } finally {
      asstMaxBudgetSaving = false;
    }
  }

  // D9: CLI stdout/stderr can carry ANSI color codes + run long — strip the
  // escapes and cap before rendering in the pre-wrap blocks below.
  function cleanCliText(s: string | null | undefined): string {
    if (!s) return "";
    const stripped = s.replace(/\x1b\[[0-9;]*m/g, "");
    return stripped.length > 500 ? stripped.slice(0, 500) + "…" : stripped;
  }
  const cliUpdateErrorClean = $derived(cleanCliText(cliUpdate.updateError));
  const cliUpdateOutputClean = $derived(cleanCliText(cliUpdate.updateOutput));

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

  // ── Progressive disclosure for long option lists ──
  // Collapsed views show one row's worth; if the active choice isn't in that
  // head slice it replaces the last slot so your current setting is always
  // visible without expanding. (Lives below the list consts it reads — TDZ.)
  function headWithSelected<T>(items: T[], count: number, isSel: (t: T) => boolean): T[] {
    const head = items.slice(0, count);
    if (head.some(isSel)) return head;
    const sel = items.find(isSel);
    return sel ? [...head.slice(0, count - 1), sel] : head;
  }
  let langsExpanded = $state(false);
  const langsShown = $derived(
    langsExpanded ? STT_LANGS : headWithSelected(STT_LANGS, 3, (l) => l.id === stt.config.language),
  );
  let shortcutsExpanded = $state(false);
  const shortcutsShown = $derived(shortcutsExpanded ? SHORTCUTS : SHORTCUTS.slice(0, 4));

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
    void diagnostics.init().catch((e) => console.warn("diagnostics.init failed", e));
    void cliUpdate.maybeCheck();
    void environment.refresh(); // fresh probe each time Settings opens — tools may have just been installed
    void elevation.refresh(); // reflect current admin state + always-elevated pref

    asstNowTick = Date.now();
    const iv = setInterval(() => { asstNowTick = Date.now(); }, 30_000);
    return () => {
      clearInterval(iv);
      if (diagCopiedTimer) { clearTimeout(diagCopiedTimer); diagCopiedTimer = null; }
    };
  });
</script>

<svelte:window onkeydown={onGlobalKey} />

<div class="sb-main">
  <!-- ── Hero + sticky tab bar ── -->
  <PageHero eyebrow="Settings" title={activeMeta.label} desc={activeMeta.sub} padBottom={false} maxWidth={820}>
    {#snippet chip()}
      <span class="sb-chip ghost"><span class="mono">local workspace</span></span>
      <button
        class="sb-chip {updates.summary.kind}"
        type="button"
        onclick={() => updates.open()}
        use:tooltip={updates.summary.kind === "dev" ? "Running an unpackaged dev build — auto-update is off" : "Check for updates"}
      >
        {#if updates.summary.kind === "warn"}
          <ArrowUpCircle size={14} />
        {:else if updates.summary.kind === "busy"}
          <Loader2 size={14} class="spin" />
        {:else if updates.summary.kind === "dev"}
          <Cog size={13} />
        {:else if updates.summary.kind === "danger"}
          <RefreshCw size={13} />
        {:else}
          <CircleCheck size={14} />
        {/if}
        <span class="sb-chip-ver mono">{appVersion}</span>
        {#if updates.summary.label}<span class="sb-chip-tag">{updates.summary.label}</span>{/if}
      </button>
    {/snippet}
    {#snippet children()}
      <div class="tabrow">
        <div class="tabnav" role="tablist">
          {#each ST_SECTIONS as s (s.id)}
            {@const Icon = s.icon}
            <!-- Dot is an ALERT, not a status lamp: a permanent green dot next
                 to "Claude" read as noise, so it only renders when the session
                 needs attention. -->
            {@const dot = s.id === "claude" ? (assistantDot === "warn" ? "warn" as const : undefined) : s.dot}
            <button class="snav" class:on={activeSec === s.id} role="tab" aria-selected={activeSec === s.id} onclick={() => selectSec(s.id)} type="button" title={dot === "warn" ? "Claude session needs attention" : dot === "ok" ? "Claude session connected" : undefined}>
              <Icon size={15} strokeWidth={1.75} />
              <span>{s.label}</span>
              {#if dot}<span class="snav-dot" class:warn={dot === "warn"}></span>{/if}
            </button>
          {/each}
        </div>
        <div class="sset-search" class:open={searchResults.length > 0}>
          <Search size={13} class="sset-search-ic" />
          <input
            class="sset-search-in"
            type="text"
            placeholder="Find a setting…"
            bind:value={searchQ}
            bind:this={searchEl}
            onkeydown={onSearchKey}
            aria-label="Search settings"
            spellcheck="false"
          />
          <span class="sset-search-kbd mono" aria-hidden="true">/</span>
          {#if searchResults.length > 0}
            <div class="sset-results" role="listbox" aria-label="Matching settings">
              {#each searchResults as r, i (r.tab + r.anchor + r.title)}
                <button type="button" class="sset-result" role="option" aria-selected={i === searchIdx} data-active={i === searchIdx} onmousedown={(ev) => { ev.preventDefault(); jumpTo(r); }} onpointerenter={() => (searchIdx = i)}>
                  <span class="sset-result-body">
                    <span class="sset-result-t">{r.title}</span>
                    <span class="sset-result-s">{r.card}</span>
                  </span>
                  <span class="sset-result-tab">{ST_SECTIONS.find((s) => s.id === r.tab)?.label}</span>
                  {#if i === searchIdx}<CornerDownLeft size={12} class="sset-result-kbd" />{/if}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/snippet}
  </PageHero>

  <div class="surface-body" bind:this={scrollEl}>

    {#if activeSec === "appearance"}
      <div class="set-surface"><div class="set-col">

          <div class="card" id="card-accent">
            <div class="card-tt">Accent color <span class="ap-dot" style="background: oklch(0.72 var(--accent-c) var(--accent-h));"></span>
              <button class="st-btn card-tt-act" type="button" onclick={() => uiPrefs.resetAccent()} use:tooltip={"Back to the stock emerald look"}><RotateCcw size={13} /> Reset</button>
            </div>
            <div class="card-sub">The highlight color used across Rift — buttons, toggles, and selection. Pick a swatch or dial in your own hue. Only the accent changes; your background stays put.</div>
              <div class="swatches">
                {#each ACCENTS as a (a.id)}
                  <button class="sw" class:sel={uiPrefs.accentHue === a.hue} type="button" style="background: oklch(0.72 0.16 {a.hue});" onclick={() => uiPrefs.setAccentHue(a.hue)} aria-pressed={uiPrefs.accentHue === a.hue} use:tooltip={a.label}>
                    {#if uiPrefs.accentHue === a.hue}<Check size={15} strokeWidth={3} color="rgba(0,0,0,0.82)" />{/if}
                  </button>
                {/each}
              </div>
              <div class="hue-row">
                <input class="hue-range" type="range" min="0" max="360" step="1" value={uiPrefs.accentHue} oninput={(e) => uiPrefs.setAccentHue(Number(e.currentTarget.value))} aria-label="Custom accent hue" use:sliderBubble={{ format: (v) => `${Math.round(v)}°` }} />
                <span class="range-val">{uiPrefs.accentHue}°</span>
              </div>
              <div class="ap-divider"></div>
              <div class="ctl-row tight">
                <div><div class="ctl-t">Vividness</div><div class="ctl-s">How saturated the accent reads across the app.</div></div>
                <div class="range-wrap grow">
                  <input class="set-range" type="range" min={VIVIDNESS_MIN} max={VIVIDNESS_MAX} step="0.005" value={uiPrefs.vividness} style="--fill: {vivPct}%" oninput={(e) => uiPrefs.setVividness(Number(e.currentTarget.value))} aria-label="Accent vividness" use:sliderBubble={{ format: (v) => `${Math.round(((v - VIVIDNESS_MIN) / (VIVIDNESS_MAX - VIVIDNESS_MIN)) * 100)}%` }} />
                  <span class="range-val">{vivPct}%</span>
                </div>
              </div>
              <!-- Live sample of every surface the accent drives — same show-don't-tell
                   chrome as the code preview below. Decorative only. -->
              <div class="code-preview" aria-hidden="true">
                <div class="code-preview-bar"><span></span><span></span><span></span><span class="code-preview-name mono">what your accent touches · updates live</span></div>
                <div class="ap-stage">
                  <span class="ap-demo-btn">Send</span>
                  <span class="rift-toggle on ap-demo-toggle"><span class="rift-toggle-knob"></span></span>
                  <span class="ap-demo-pill"><span class="ap-demo-dot"></span>Connected</span>
                  <span class="ap-demo-sel">selected text</span>
                  <span class="ap-demo-link">a link</span>
                  <span class="ap-demo-field">focus ring</span>
                </div>
              </div>
          </div>

          <div class="card" id="card-interface">
            <div class="card-tt">Interface &amp; code</div>
            <div class="card-sub">Spacing across the app, and how code renders in Claude's replies.</div>
            <div class="ctl-row tight">
              <div><div class="ctl-t">Interface density</div><div class="ctl-s">Compact fits more on screen; comfy breathes.</div></div>
              <div class="seg">
                {#each DENSITIES as d (d)}
                  <button class:on={uiPrefs.density === d} type="button" onclick={() => uiPrefs.setDensity(d)}>{d[0].toUpperCase() + d.slice(1)}</button>
                {/each}
              </div>
            </div>
              <div class="ctl-row tight">
                <div><div class="ctl-t">Font size</div><div class="ctl-s">Size of code in Claude's replies.</div></div>
                <div class="seg" role="radiogroup" aria-label="Code font size">
                  {#each [11, 12, 13, 14] as n (n)}<button class:on={uiPrefs.code.fontSize === n} role="radio" aria-checked={uiPrefs.code.fontSize === n} type="button" onclick={() => uiPrefs.setCode({ fontSize: n })}>{n}px</button>{/each}
                </div>
              </div>
              <div class="ctl-row tight">
                <div><div class="ctl-t">Tab width</div><div class="ctl-s">Spaces per indentation level.</div></div>
                <div class="seg">
                  {#each [2, 4] as w (w)}<button class:on={uiPrefs.code.tabWidth === w} type="button" onclick={() => uiPrefs.setCode({ tabWidth: w })}>{w}</button>{/each}
                </div>
              </div>
              <div class="ctl-row tight">
                <div><div class="ctl-t">Font ligatures</div><div class="ctl-s">Render <code>→ ≠ &gt;=</code> as joined glyphs in JetBrains Mono.</div></div>
                <button class="rift-toggle" class:on={uiPrefs.code.ligatures} role="switch" aria-checked={uiPrefs.code.ligatures} aria-label="Font ligatures" type="button" onclick={() => uiPrefs.setCode({ ligatures: !uiPrefs.code.ligatures })}><span class="rift-toggle-knob"></span></button>
              </div>
              <!-- Live preview — reads the same --code-* root vars the real chat
                   renderer uses (ui-prefs.svelte.ts applyToRoot), so every click
                   above repaints it exactly as replies will render. Tab-indented
                   + ligature-rich on purpose. -->
              <div class="code-preview" aria-hidden="true">
                <div class="code-preview-bar"><span></span><span></span><span></span><span class="code-preview-name mono">interface &amp; code · updates live</span></div>
                <!-- Density stage — rows sized by the REAL --row-h/--gap/--fs-md
                     vars the density attribute drives, so the seg above repaints
                     it exactly like the app chrome. -->
                <div class="iface-stage">
                  <div class="iface-row"><span class="iface-dot"></span>Fix composer bug<span class="iface-meta mono">2m</span></div>
                  <div class="iface-row on"><span class="iface-dot on"></span>Settings overhaul<span class="iface-meta mono">now</span></div>
                  <div class="iface-row"><span class="iface-dot"></span>Refactor turn engine<span class="iface-meta mono">1h</span></div>
                </div>
                <pre class="mono"><span class="cp-kw">const</span> ship = (v) <span class="cp-op">=&gt;</span> &#123;{"\n\t"}<span class="cp-kw">if</span> (v <span class="cp-op">!=</span> <span class="cp-kw">null</span> &amp;&amp; v <span class="cp-op">&gt;=</span> <span class="cp-num">1</span>) <span class="cp-kw">return</span> <span class="cp-str">"ready"</span>;{"\n\t"}<span class="cp-kw">return</span> <span class="cp-str">"hold"</span>; <span class="cp-cm">// ligatures: =&gt; != &gt;=</span>{"\n"}&#125;;</pre>
              </div>
          </div>

          <button class="set-expand set-reset" type="button" onclick={() => void resetTab("appearance")}><RotateCcw size={13} /> Reset Appearance to defaults</button>
      </div></div>
    {/if}

    {#if activeSec === "chat"}
      <div class="set-surface"><div class="set-col">
        <!-- moved from the old Claude tab (2026-07-09 restructure): these are
             display prefs, not session config — they live with Chat now. -->
        <div class="card" id="card-rendering">
          <div class="card-tt">Chat rendering</div>
          <div class="card-sub">How Claude's activity and replies are laid out.</div>
          <div class="ctl-row tight">
            <div><div class="ctl-t">Stream view</div><div class="ctl-s">A boxless, text-first activity stream instead of classic bubbles.</div></div>
            <button class="rift-toggle" class:on={uiPrefs.streamMode} role="switch" aria-checked={uiPrefs.streamMode} aria-label="Stream view" type="button" onclick={() => uiPrefs.toggleStreamMode()}><span class="rift-toggle-knob"></span></button>
          </div>
          {#if uiPrefs.streamMode}
            <div class="ctl-row tight">
              <div><div class="ctl-t">Density preset{#if uiPrefs.activePreset === null}<span class="preset-custom">Custom mix</span>{/if}</div><div class="ctl-s">Sets the three dials below together — fine-tune any of them after.</div></div>
              <div class="seg" role="radiogroup" aria-label="Density preset">
                {#each DENSITY_PRESETS as p (p.id)}
                  <button class:on={uiPrefs.activePreset === p.id} role="radio" aria-checked={uiPrefs.activePreset === p.id} type="button" onclick={() => uiPrefs.applyPreset(p.id)}>{p.label}</button>
                {/each}
              </div>
            </div>
            <div class="ctl-row tight">
              <div><div class="ctl-t">Tool detail</div><div class="ctl-s">How much each tool and file action shows.</div></div>
              <div class="seg" role="radiogroup" aria-label="Tool detail density">
                {#each TOOL_DETAILS as d (d.id)}
                  <button class:on={uiPrefs.toolDetail === d.id} role="radio" aria-checked={uiPrefs.toolDetail === d.id} type="button" onclick={() => uiPrefs.setToolDetail(d.id)}>{d.label}</button>
                {/each}
              </div>
            </div>
            <div class="ctl-row tight">
              <div><div class="ctl-t">Narration</div><div class="ctl-s">How much of Claude's between-step commentary shows.</div></div>
              <div class="seg" role="radiogroup" aria-label="Narration density">
                {#each NARRATIONS as n (n.id)}
                  <button class:on={uiPrefs.narration === n.id} role="radio" aria-checked={uiPrefs.narration === n.id} type="button" onclick={() => uiPrefs.setNarration(n.id)}>{n.label}</button>
                {/each}
              </div>
            </div>
            <div class="ctl-row tight" class:overridden={uiPrefs.toolDetail === "detailed"}>
              <div><div class="ctl-t">Command output {#if uiPrefs.toolDetail === "detailed"}<span class="ctl-note">· set by Detailed</span>{/if}</div><div class="ctl-s">How much of a shell command's terminal output shows.</div></div>
              <div class="seg" role="radiogroup" aria-label="Command output detail" aria-disabled={uiPrefs.toolDetail === "detailed"}>
                {#each COMMAND_OUTPUTS as c (c.id)}
                  <button class:on={uiPrefs.commandOutput === c.id} role="radio" aria-checked={uiPrefs.commandOutput === c.id} type="button" disabled={uiPrefs.toolDetail === "detailed"} onclick={() => uiPrefs.setCommandOutput(c.id)}>{c.label}</button>
                {/each}
              </div>
            </div>
            <!-- Mini activity stream driven by the real pref values — the dials
                 above explain themselves here instead of via paragraph glossaries. -->
            {@const effCmdOut = uiPrefs.toolDetail === "detailed" ? "full" : uiPrefs.commandOutput}
            <div class="code-preview" aria-hidden="true">
              <div class="code-preview-bar"><span></span><span></span><span></span><span class="code-preview-name mono">stream preview · updates live</span></div>
              <div class="sp-stage mono">
                <div class="sp-head">✦ Working for 12s</div>
                {#if uiPrefs.narration === "chatty"}
                  <div class="sp-prose">Now checking the composer wiring before I edit anything.</div>
                {:else if uiPrefs.narration === "balanced"}
                  <div class="sp-note">checking composer wiring</div>
                {/if}
                {#if uiPrefs.toolDetail === "minimal"}
                  <div class="sp-tool"><span class="sp-chev">▸</span> Edited 2 files</div>
                {:else if uiPrefs.toolDetail === "balanced"}
                  <div class="sp-tool"><span class="sp-chev">▸</span> Edit <span class="sp-file">Composer.svelte</span> <span class="sp-diff">+12 −4</span></div>
                {:else}
                  <div class="sp-tool open"><span class="sp-chev">▾</span> Edit <span class="sp-file">src/lib/components/assistant/Composer.svelte</span> <span class="sp-diff">+12 −4</span></div>
                  <div class="sp-tool-body"><span class="sp-add">+ const preview = buildStreamPreview();</span><br /><span class="sp-del">− // TODO: preview</span></div>
                {/if}
                <div class="sp-cmd">$ npm run check</div>
                {#if effCmdOut === "peek"}
                  <div class="sp-out dim">✓ 0 errors · exit 0 — click to expand</div>
                {:else if effCmdOut === "full"}
                  <div class="sp-out">svelte-check found 0 errors and 0 warnings</div>
                  <div class="sp-out">✓ done in 3.2s · exit 0</div>
                {/if}
              </div>
            </div>
          {:else}
            <!-- The toggle explains itself both ways — off shows what classic
                 bubbles look like instead of collapsing to an empty card. -->
            <div class="code-preview" aria-hidden="true">
              <div class="code-preview-bar"><span></span><span></span><span></span><span class="code-preview-name mono">classic bubbles · what off looks like</span></div>
              <div class="bp-stage">
                <div class="bp-row user"><div class="bp-bubble user">fix the composer bug</div></div>
                <div class="bp-row"><div class="bp-bubble">Found it — the mention popover was anchored to a stale offset. Patched and verified.</div></div>
                <div class="bp-row"><div class="bp-tool mono">▸ Edit Composer.svelte · +12 −4</div></div>
              </div>
            </div>
          {/if}
        </div>

        <div class="card" id="card-comfort">
          <div class="card-tt">Reading comfort</div>
          <div class="card-sub">Make Claude's replies easier to read. These affect the chat only — the rest of Rift keeps the dark theme.</div>
          <div class="ctl-row tight">
            <div><div class="ctl-t">Dyslexia-friendly mode</div><div class="ctl-s">Lexend font, wider spacing, and charitable reading of phonetic typos.</div></div>
            <button class="rift-toggle" class:on={accessibility.dyslexiaMode} role="switch" aria-checked={accessibility.dyslexiaMode} aria-label="Dyslexia-friendly mode" type="button" onclick={() => accessibility.setDyslexiaMode(!accessibility.dyslexiaMode)}><span class="rift-toggle-knob"></span></button>
          </div>
          <div class="ctl-row tight sub-row" data-disabled={!accessibility.dyslexiaMode}>
            <div><div class="ctl-t">Font</div><div class="ctl-s">Lexend has the strongest research backing for dyslexic reading speed.</div></div>
            <div class="seg" role="radiogroup" aria-label="UI font">
              <button class:on={accessibility.font === "system"} role="radio" aria-checked={accessibility.font === "system"} disabled={!accessibility.dyslexiaMode} type="button" onclick={() => accessibility.setFont("system")}>Inter</button>
              <button class:on={accessibility.font === "lexend"} role="radio" aria-checked={accessibility.font === "lexend"} disabled={!accessibility.dyslexiaMode} type="button" onclick={() => accessibility.setFont("lexend")}>Lexend</button>
            </div>
          </div>
          <div class="ctl-row tight sub-row" data-disabled={!accessibility.dyslexiaMode}>
            <div><div class="ctl-t">Wider line and letter spacing</div><div class="ctl-s">Raises line height to 1.85 in bubbles and the composer.</div></div>
            <button class="rift-toggle" class:on={accessibility.lineHeightBoost} role="switch" aria-checked={accessibility.lineHeightBoost} aria-label="Wider line and letter spacing" disabled={!accessibility.dyslexiaMode} type="button" onclick={() => accessibility.setLineHeightBoost(!accessibility.lineHeightBoost)}><span class="rift-toggle-knob"></span></button>
          </div>
          <div class="ctl-row tight">
            <div><div class="ctl-t">Warm reading tint</div><div class="ctl-s">A soft sepia overlay on chat bubbles to ease bright-white-on-dark glare.</div></div>
            <button class="rift-toggle" class:on={accessibility.warmTint} role="switch" aria-checked={accessibility.warmTint} aria-label="Warm reading tint" type="button" onclick={() => accessibility.setWarmTint(!accessibility.warmTint)}><span class="rift-toggle-knob"></span></button>
          </div>
          <!-- A mock reply styled by the REAL a11y pipeline: the global
               data-a11y-* selectors target .bubble/.markdown-body, so this
               inherits font/spacing/tint with zero duplicated logic. -->
          <div class="code-preview" aria-hidden="true">
            <div class="code-preview-bar"><span></span><span></span><span></span><span class="code-preview-name mono">reply preview · updates live</span></div>
            <div class="cf-stage">
              <div class="bubble markdown-body cf-bubble">
                <p>Here's the fix — the mention popover was anchored to a stale offset. I moved the reset into the tab-switch effect, so <code>pickMention()</code> can't corrupt the new draft anymore.</p>
              </div>
            </div>
          </div>
        </div>

        <button class="set-expand set-reset" type="button" onclick={() => void resetTab("chat")}><RotateCcw size={13} /> Reset Chat to defaults</button>
      </div></div>
    {/if}

    {#if activeSec === "claude"}
      <div class="set-surface"><div class="set-col">
        <!-- session status promoted to a hero banner — auth + CLI version share one surface -->
        <div class="sb-status {assistantDot ?? 'ok'}">
          <div class="sb-status-l">
            <div class="sb-status-ic">
              {#if assistantStore.auth}<CircleCheck size={18} />{:else}<Loader2 size={18} class="spin" />{/if}
            </div>
            <div class="sb-status-main">
              <b>{assistantStore.auth ? assistantStore.auth.summary : assistantStore.authChecking ? "Checking session…" : "Session unknown"}</b>
              <div class="sub">Every turn runs through your own Claude Code CLI.{#if !assistantStore.auth && !assistantStore.authChecking}{' '}Not signed in? Run <code>claude login</code> in a terminal, then re-probe.{/if}</div>
              {#if cliVersionUnknown}
                <div class="sub st-cli-warn" use:tooltip={"`claude --version` failed or timed out, so Rift can't tell how new this CLI is. To stay safe it treats it as an old version and turns newer features off. Re-probe after an update, or check the install is healthy."}>⚠ Couldn't read this CLI's version — newer features are off until it's readable.</div>
              {:else if cliBelowFeatureFloor && !cliNewer}
                <div class="sub st-cli-note" use:tooltip={`Your CLI can run every Rift turn, but a couple of spawn-time options only exist on Claude Code ≥ ${CLI_RECOMMENDED_VERSION}. Updating turns them on automatically — nothing's broken in the meantime.`}>Some features need Claude Code ≥ <code>{CLI_RECOMMENDED_VERSION}</code> — update to enable them all.</div>
              {/if}
            </div>
          </div>
          <div class="sb-status-r">
            {#if cliInstalled}
              <span class="st-cli-chip mono" use:tooltip={"The Claude Code CLI version Rift spawns for every turn"}>claude {cliInstalled}</span>
            {/if}
            {#if cliUpdate.status === "checking"}
              <span class="st-pill"><span class="dot"></span>Checking…</span>
            {:else if cliNewer}
              <span class="st-pill accent"><span class="dot"></span>Update → {cliTarget ?? cliUpdate.latest}</span>
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
                  {#if !inst.version}<span class="st-cli-inst-tag unknown" use:tooltip={"Rift couldn't read this install's version (`claude --version` failed or timed out). It's treated as an old CLI — newer features are turned off for safety until the version is readable."}>version?</span>{/if}
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
              <div class="st-cli-err">{cliUpdateErrorClean}</div>
            {:else if cliUpdate.updateOutput}
              <div class="st-cli-ok">{cliUpdateOutputClean}</div>
            {/if}
            {#if cliUpdate.updateStuck}
              <div class="st-cli-warn">{cliSummary.detail}</div>
            {/if}
          {/if}
        </div>

        <div class="card" id="card-session">
          <div class="card-tt">Claude session</div>
          <div class="card-sub">How each turn runs — config, git access, and plan.</div>
          <div class="ctl-row tight">
            <div><div class="ctl-t">Use my full Claude Code config</div><div class="ctl-s">Layers your global <code>~/.claude</code> setup — CLAUDE.md, hooks, skills, MCP servers — into every turn. Off = sandboxed with Rift's tools only.</div></div>
            <button class="rift-toggle" class:on={assistantStore.useFullConfig && !assistantStore.hasApiKey} role="switch" aria-checked={assistantStore.useFullConfig && !assistantStore.hasApiKey} aria-label="Use full Claude Code config" disabled={assistantStore.hasApiKey} type="button" onclick={() => void assistantStore.setUseFullConfig(!assistantStore.useFullConfig)}><span class="rift-toggle-knob"></span></button>
          </div>
          <div class="ctl-row tight">
            <div><div class="ctl-t">Git tools</div><div class="ctl-s">Read-only = status, diff, log. Standard adds commit, pull, and push.</div></div>
            <div class="seg" role="radiogroup" aria-label="Git tools trust level">
              <button class:on={assistantStore.trustLevel === "readonly"} role="radio" aria-checked={assistantStore.trustLevel === "readonly"} type="button" onclick={() => void assistantStore.setTrustLevel("readonly")}>Read-only</button>
              <button class:on={assistantStore.trustLevel !== "readonly"} role="radio" aria-checked={assistantStore.trustLevel !== "readonly"} type="button" onclick={() => void assistantStore.setTrustLevel("standard")}>Standard</button>
            </div>
          </div>
          <div class="ctl-row tight no-line">
            <div><div class="ctl-t">Plan</div><div class="ctl-s">Sets the context-window gauge — Anthropic doesn't expose your plan, so pick it here. Free caps at 200K; Pro and Max unlock 1M.</div></div>
            <div class="seg" role="radiogroup" aria-label="Subscription plan">
              <button class:on={assistantStore.plan === "free"} role="radio" aria-checked={assistantStore.plan === "free"} type="button" onclick={() => assistantStore.setPlan("free")}>Free</button>
              <button class:on={assistantStore.plan === "pro"} role="radio" aria-checked={assistantStore.plan === "pro"} type="button" onclick={() => assistantStore.setPlan("pro")}>Pro</button>
              <button class:on={assistantStore.plan === "max"} role="radio" aria-checked={assistantStore.plan === "max"} type="button" onclick={() => assistantStore.setPlan("max")}>Max</button>
            </div>
          </div>
          <!-- What the picker actually drives: the context gauge, live. -->
          <div class="plan-gauge" aria-hidden="true">
            <div class="plan-gauge-track">
              <span class="plan-gauge-tick"></span>
              <div class="plan-gauge-fill" style="width: {assistantStore.plan === 'free' ? 20 : 100}%"></div>
            </div>
            <span class="plan-gauge-cap mono">{assistantStore.plan === "free" ? "200K" : "1M"} context</span>
          </div>
          {#if assistantStore.plan === "pro"}
            <div class="st-note">Pro reaches 1M only once usage credits are enabled at <code>claude.ai/settings/usage</code> — otherwise it behaves like 200K.</div>
          {/if}
        </div>

        {#if elevation.supported}
          <div class="card" id="card-admin">
            <div class="card-tt">Administrator access</div>
            <div class="card-sub">Run Rift elevated so the assistant's tools inherit admin rights — no per-action UAC prompts, just like launching VS Code as administrator.</div>
            <div class="ctl-row tight">
              <div>
                <div class="ctl-t">Status</div>
                <div class="ctl-s">
                  {#if elevation.elevated}
                    Running as <strong>Administrator</strong>. Commands the assistant runs are elevated — no per-action prompts.
                  {:else}
                    Running as a <strong>standard user</strong>. Elevated actions each trigger a Windows UAC prompt.
                  {/if}
                </div>
              </div>
              {#if elevation.elevated}
                <span class="admin-live"><ShieldCheck size={14} /> Administrator</span>
              {:else}
                <button class="st-btn" type="button" disabled={elevation.busy} onclick={() => void elevation.relaunchAsAdmin()}>
                  {#if elevation.busy}<Loader2 size={14} class="st-spin" /> Relaunching…{:else}<ShieldCheck size={14} /> Relaunch as administrator{/if}
                </button>
              {/if}
            </div>
            <div class="ctl-row tight no-line">
              <div>
                <div class="ctl-t">Always run as administrator</div>
                <div class="ctl-s">Rift launches elevated every time with no UAC prompt (via a per-user scheduled task). Convenient — but every tool the assistant runs then has full admin rights. Turn it off anytime; the task is removed.</div>
              </div>
              <button class="rift-toggle" class:on={elevation.alwaysElevated} role="switch" aria-checked={elevation.alwaysElevated} aria-label="Always run as administrator" disabled={elevation.busy} type="button" onclick={() => void elevation.setAlwaysElevated(!elevation.alwaysElevated)}><span class="rift-toggle-knob"></span></button>
            </div>
            {#if elevation.error}
              <div class="st-note">{elevation.error}</div>
            {/if}
          </div>
        {/if}

          <div class="card" id="card-api">
            <div class="card-tt">API key &amp; spending</div>
            <div class="card-sub">Where your turns bill. Setting a key switches from your subscription to pay-per-token via the Anthropic Console.</div>
            <!-- Route strip: which billing path is live right now. -->
            <div class="route" aria-hidden="true">
              <span class="route-node" class:on={!assistantStore.hasApiKey}>Claude session</span>
              <span class="route-or">or</span>
              <span class="route-node" class:on={assistantStore.hasApiKey}>API key</span>
              <span class="route-arrow">→</span>
              <span class="route-note">{assistantStore.hasApiKey ? "turns bill pay-per-token via the Console" : "turns bill against your plan's usage windows"}</span>
            </div>
            <div class="ctl-row tight">
              <div><label class="ctl-t" for="asst-apikey">API-key fallback</label><div class="ctl-s">Stored in your OS keychain. Key turns run the CLI bare — your <code>~/.claude</code> config and MCP servers won't load.</div></div>
              <div class="ctl-actions">
                {#if assistantStore.hasApiKey}
                  <span class="st-pill ok"><span class="dot"></span>Configured</span>
                  <button class="st-btn danger-btn" type="button" disabled={asstApiKeySaving} onclick={() => { asstApiKeyDraft = ""; void saveAsstApiKey(); }}>Clear</button>
                {:else}
                  <span class="st-secret">
                    <input id="asst-apikey" class="st-input mono" type={asstApiKeyVisible ? "text" : "password"} placeholder="sk-ant-api03-…" style="width:100%; max-width:188px;" bind:value={asstApiKeyDraft} autocomplete="off" spellcheck="false" />
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
            <!-- Per-turn dollar cap only does anything in API-key mode (pay-per-token,
                 --max-budget-usd stops the turn). A subscription session bills against
                 plan usage-limit windows, not dollars, so the cap is inert there and
                 the row is hidden rather than shown as a no-op control. -->
            {#if assistantStore.hasApiKey}
              <div class="ctl-row tight">
                <div><label class="ctl-t" for="asst-budget">Per-turn cost cap</label><div class="ctl-s">Stops a turn before it spends more than this dollar amount of API credit. Leave blank for no cap.</div></div>
                <div class="ctl-actions">
                  <input id="asst-budget" class="st-input mono" type="number" min="0" step="0.01" placeholder="5.00" style="width:100%; max-width:88px; text-align:right;" bind:value={asstMaxBudgetDraft} />
                  <button class="st-btn primary" type="button" onclick={saveAsstMaxBudget} disabled={asstMaxBudgetSaving || !asstMaxBudgetDirty}>{asstMaxBudgetSaving ? "Saving…" : "Save"}</button>
                  {#if assistantStore.maxBudgetUsd != null}
                    <button class="st-btn" type="button" disabled={asstMaxBudgetSaving} onclick={() => { asstMaxBudgetDraft = null; void saveAsstMaxBudget(); }}>Clear</button>
                  {/if}
                </div>
              </div>
              {#if asstMaxBudgetMsg}<div class="st-note">{asstMaxBudgetMsg}</div>{/if}
            {/if}
          </div>

          <button class="set-expand set-reset" type="button" onclick={() => void resetTab("claude")}><RotateCcw size={13} /> Reset Claude session to defaults</button>
      </div></div>
    {/if}

    {#if activeSec === "speech"}
      <div class="set-surface"><div class="set-col">
          <div class="card" id="card-engine">
            <div class="card-tt">Engine</div>
            <div class="card-sub">Turn voice input on and pick what transcribes it.</div>
            <div class="ctl-row tight">
              <div><div class="ctl-t">Speech-to-text</div><div class="ctl-s">Master switch. When off, the mic button in the composer is hidden.</div></div>
              <button class="rift-toggle" class:on={stt.config.enabled} role="switch" aria-checked={stt.config.enabled} aria-label="Enable speech-to-text" type="button" onclick={() => void stt.setConfig({ enabled: !stt.config.enabled })}><span class="rift-toggle-knob"></span></button>
            </div>
            <div class="ctl-row stack">
              <div><div class="ctl-t">Recognition engine</div><div class="ctl-s">Web Speech is zero-install via Edge / Azure. Parakeet runs on-device — fast, private, works offline. Whisper is the on-device multilingual option with vocabulary priming.</div></div>
              <div class="set-pick-grid set-pick-grid-3">
                {#each STT_ENGINES as eng (eng.id)}
                  <button type="button" class="set-pick" data-active={stt.config.engine === eng.id} disabled={!stt.config.enabled || (eng.id === "whisper" && !stt.backends.whisper) || (eng.id === "parakeet" && !stt.backends.parakeet)} onclick={() => void stt.setConfig({ engine: eng.id })}>
                    <span class="set-pick-label">{eng.label}</span>
                    <span class="set-pick-sub mono">{eng.sub}</span>
                  </button>
                {/each}
              </div>
            </div>
            {#if !stt.backends.whisper}
              <div class="st-info">
                <div class="st-info-t">On-device Whisper isn't included in this build.</div>
                <div class="st-info-s">{#if stt.backends.parakeet}Parakeet covers on-device dictation without it — Whisper only adds broad multilingual support.{:else}Web Speech is selected for you and works right now — no setup, no download. It transcribes through your browser engine while you're online.{/if}</div>
                <details class="st-dev">
                  <summary>Build Whisper into Rift yourself</summary>
                  <div class="st-dev-body">For offline multilingual transcription you can compile Rift from source with Whisper enabled:
                    <ol>
                      <li>Install LLVM — <code>winget install LLVM.LLVM</code> (run as administrator)</li>
                      <li>Optional, for GPU acceleration — install the NVIDIA CUDA Toolkit</li>
                      <li>Rebuild — <code>cargo build --release --features whisper-rs</code></li>
                    </ol>
                  </div>
                </details>
              </div>
            {/if}
          </div>

          {#if stt.config.engine === "web_speech"}
            <div class="card">
              <div class="card-tt">Web Speech</div>
              <div class="card-sub">Language and listening behavior for the online recognizer.</div>
              <div class="ctl-row stack">
                <div><div class="ctl-t">Language</div><div class="ctl-s">BCP-47 tag passed to the recognizer. Pick another language if you speak something other than English.</div></div>
                <div class="set-pick-grid" role="radiogroup" aria-label="Speech recognition language">
                  {#each langsShown as l, ti (l.id)}
                    <button type="button" role="radio" aria-checked={stt.config.language === l.id} class="set-pick anim-reveal" style="--ti: {ti}" data-active={stt.config.language === l.id} onclick={() => void stt.setConfig({ language: l.id })}>
                      <span class="set-pick-label">{l.label}</span>
                      <span class="set-pick-sub mono">{l.id}</span>
                    </button>
                  {/each}
                </div>
                <button class="set-expand" type="button" aria-expanded={langsExpanded} onclick={() => (langsExpanded = !langsExpanded)}>
                  {langsExpanded ? "Show less" : `Show all ${STT_LANGS.length} languages`}
                  <ChevronDown size={13} class={langsExpanded ? "set-expand-ic flip" : "set-expand-ic"} />
                </button>
              </div>
              <div class="ctl-row tight">
                <div><div class="ctl-t">Continuous mode</div><div class="ctl-s">Keep listening across pauses until you click stop.</div></div>
                <button class="rift-toggle" class:on={stt.config.continuous} role="switch" aria-checked={stt.config.continuous} aria-label="Continuous mode" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ continuous: !stt.config.continuous })}><span class="rift-toggle-knob"></span></button>
              </div>
            </div>
          {/if}

          {#snippet modelRows(models: ModelInfo[], activeId: string, pick: (id: string) => void)}
            {#each models as m (m.id)}
              {@const prog = stt.modelDownloads[m.id]}
              {@const isActive = activeId === m.id}
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
                      <button type="button" class="st-btn" onclick={() => pick(m.id)}>Use</button>
                    {:else}
                      <span class="st-pill ok"><span class="dot"></span>Active</span>
                    {/if}
                    <button type="button" class="st-btn danger-btn" onclick={() => void stt.deleteModel(m.id)} use:tooltip={"Delete model"} aria-label="Delete"><Trash2 size={14} /></button>
                  {:else}
                    <button type="button" class="st-btn primary" disabled={!stt.config.enabled} onclick={() => void stt.downloadModel(m.id)}>Download</button>
                  {/if}
                </div>
              </div>
            {/each}
            {#if models.length === 0}<div class="st-note">Loading model catalog…</div>{/if}
          {/snippet}

          {#if stt.config.engine === "parakeet"}
            <div class="card">
              <div class="card-tt">Parakeet model</div>
              <div class="card-sub">One download, then dictation runs fully on this machine — offline, uncensored, GPU-accelerated on any card.</div>
              <div class="set-model-list">
                {@render modelRows(parakeetModels, stt.config.parakeet_model, (id) => void stt.setConfig({ parakeet_model: id }))}
              </div>
            </div>
          {/if}

          {#if stt.config.engine === "whisper"}
            <div class="card">
              <div class="card-tt">Whisper model</div>
              <div class="card-sub">Bigger models hear more accurately but download larger and transcribe slower.</div>
              <div class="set-model-list">
                {@render modelRows(whisperModels, stt.config.whisper_model, (id) => void stt.setConfig({ whisper_model: id }))}
              </div>
            </div>
          {/if}

          {#if isLocalEngine(stt.config.engine)}
            <div class="card">
              <div class="card-tt">Capture &amp; cleanup</div>
              <div class="card-sub">Which microphone the recorder hears, and how the transcript gets polished.</div>
              <div class="ctl-row tight">
                <div><div class="ctl-t">Input device</div><div class="ctl-s">Microphone used for capture. System default is usually correct.</div></div>
                <div class="ctl-actions set-mic-r">
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
              <div class="ctl-row tight">
                <div><div class="ctl-t">Clean up transcript</div><div class="ctl-s">Polishes the final transcript with Claude — fixes punctuation, capitalizes proper nouns. Adds a short tail after you stop.</div></div>
                <button class="rift-toggle" class:on={stt.config.cleanup_enabled} role="switch" aria-checked={stt.config.cleanup_enabled} aria-label="Clean up transcript with Claude" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ cleanup_enabled: !stt.config.cleanup_enabled })}><span class="rift-toggle-knob"></span></button>
              </div>
              {#if stt.config.engine === "whisper"}
                <div class="ctl-row tight">
                  <div><div class="ctl-t">Beam search</div><div class="ctl-s">Higher-accuracy decode (beam width 5) instead of greedy — sharper on technical terms, ~2-4× slower. GPU recommended.</div></div>
                  <button class="rift-toggle" class:on={(stt.config.beam_size ?? 1) > 1} role="switch" aria-checked={(stt.config.beam_size ?? 1) > 1} aria-label="Use beam search decoding" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ beam_size: (stt.config.beam_size ?? 1) > 1 ? null : 5 })}><span class="rift-toggle-knob"></span></button>
                </div>
              {/if}
            </div>
          {/if}

          {#if stt.config.engine === "whisper"}
            <div class="card">
              <div class="card-tt">Vocabulary priming</div>
              <div class="card-sub">Teach Whisper your project names and jargon so it stops guessing.</div>
              <div class="ctl-row stack">
                <div><div class="ctl-t">Style prompt</div><div class="ctl-s">Whisper's <code>initial_prompt</code> biases the decoder toward your speaking style.</div></div>
                <textarea class="set-textarea" rows="3" disabled={!stt.config.enabled} value={stt.config.initial_prompt} oninput={(e) => void stt.setConfig({ initial_prompt: (e.currentTarget as HTMLTextAreaElement).value })}></textarea>
              </div>
              <div class="ctl-row stack">
                <div><div class="ctl-t">Vocabulary</div><div class="ctl-s">Comma- or newline-separated. Add project names, server names. Budget ~800 chars (Whisper's 224-token prompt limit).</div></div>
                <textarea class="set-textarea" rows="4" placeholder="Project names, libraries, jargon — e.g. Tauri, SvelteKit, oklch, my_project" disabled={!stt.config.enabled} value={stt.config.vocab_text} oninput={(e) => void stt.setConfig({ vocab_text: (e.currentTarget as HTMLTextAreaElement).value })}></textarea>
              </div>
            </div>
          {/if}

          {#if stt.config.engine === "web_speech" && !stt.supported}
            <div class="st-warn">Your WebView doesn't expose <code>SpeechRecognition</code>, so Web Speech can't run here.{#if stt.backends.parakeet} Switch to the Parakeet engine above — it runs on-device and needs no browser support.{:else} No on-device engine is built into this binary either; see the note under <strong>Engine</strong>.{/if}</div>
          {/if}
          {#if stt.lastError}<div class="st-warn">{stt.lastError}</div>{/if}

          <div class="card" id="card-composer">
            <div class="card-tt">Composer integration</div>
            <div class="card-sub">How spoken words land in the message box.</div>
            <div class="ctl-row tight">
              <div><div class="ctl-t">Live partial transcripts</div><div class="ctl-s">Words appear in the composer as you speak. Off = wait for each sentence to commit.</div></div>
              <button class="rift-toggle" class:on={stt.config.show_interim} role="switch" aria-checked={stt.config.show_interim} aria-label="Live partial transcripts" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ show_interim: !stt.config.show_interim })}><span class="rift-toggle-knob"></span></button>
            </div>
            <div class="ctl-row tight">
              <div><div class="ctl-t">Insertion mode</div><div class="ctl-s">Append preserves what's typed; off = transcript replaces composer contents (mic-first workflow).</div></div>
              <button class="rift-toggle" class:on={stt.config.append_to_draft} role="switch" aria-checked={stt.config.append_to_draft} aria-label="Append transcript to existing draft" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ append_to_draft: !stt.config.append_to_draft })}><span class="rift-toggle-knob"></span></button>
            </div>
            <div class="ctl-row tight">
              <div><div class="ctl-t">Voice commands</div><div class="ctl-s">"send it" fires the message, "new line" / "new paragraph" insert breaks, "scratch that" deletes the last phrase.</div></div>
              <button class="rift-toggle" class:on={stt.config.voice_commands} role="switch" aria-checked={stt.config.voice_commands} aria-label="Voice commands" disabled={!stt.config.enabled} type="button" onclick={() => void stt.setConfig({ voice_commands: !stt.config.voice_commands })}><span class="rift-toggle-knob"></span></button>
            </div>
            <div class="ctl-row tight">
              <div><div class="ctl-t">Auto-stop on silence</div><div class="ctl-s">Ends the recording by itself after a pause — hands-free dictation. Needs live partials on the Web Speech engine.</div></div>
              <div class="set-pick-grid">
                {#each [{ v: 0, label: "Off" }, { v: 3, label: "3s" }, { v: 5, label: "5s" }, { v: 10, label: "10s" }] as opt (opt.v)}
                  <button type="button" class="set-pick" data-active={stt.config.auto_stop_secs === opt.v} disabled={!stt.config.enabled} onclick={() => void stt.setConfig({ auto_stop_secs: opt.v })}>{opt.label}</button>
                {/each}
              </div>
            </div>
          </div>

          <button class="set-expand set-reset" type="button" onclick={() => void resetTab("speech")}><RotateCcw size={13} /> Reset Speech to defaults</button>
      </div></div>
    {/if}

    {#if activeSec === "about"}
      <div class="set-surface"><div class="set-col">
          <div class="card" id="card-build">
            <div class="card-tt">Build &amp; install</div>
            <div class="card-sub">This copy of Rift, the stack it's built on, and where it keeps its files.</div>
            <div class="st-build">
              <div class="st-build-mark"><Sparkles size={18} strokeWidth={2} /></div>
              <div class="st-build-id">
                <div class="st-build-name">Rift <span class="st-build-ver">{appVersion}</span></div>
                <div class="st-build-chan">
                  {#if updates.summary.kind === "dev"}
                    <span class="st-build-tag dev">dev build</span> · not auto-updated
                  {:else if updates.summary.kind === "warn"}
                    <span class="st-build-tag warn">{updates.summary.label}</span>
                  {:else}
                    <span class="st-build-tag ok">{updates.summary.label || "installed"}</span>
                  {/if}
                </div>
              </div>
              <button class="st-btn st-build-check" type="button" onclick={() => updates.open()}><RefreshCw size={14} /> Check for updates</button>
            </div>
            <!-- Stack trivia compressed to a chip strip — Engine/Platform/Style rows
                 were four lines of developer trivia; the license row is now a real link. -->
            <div class="st-stack">
              {#each ["Tauri 2", "Rust", "SvelteKit", "Svelte 5", "Tailwind 4", "OKLCH"] as chip (chip)}
                <span class="st-stack-chip">{chip}</span>
              {/each}
            </div>
            <!-- Paths folded in from the old standalone card — same "this install" domain. -->
            <div class="ctl-row tight">
              <div><div class="ctl-t">Config</div><div class="ctl-s"><span class="mono" use:tooltip={configDir}>{configDir || "—"}</span></div></div>
              <button class="st-btn" type="button" disabled={!configDir} onclick={() => openDir(configDir)}><FolderOpen size={14} /> Open</button>
            </div>
            <div class="ctl-row tight">
              <div><div class="ctl-t">Logs</div><div class="ctl-s"><span class="mono" use:tooltip={logDir}>{logDir || "—"}</span></div></div>
              <button class="st-btn" type="button" disabled={!logDir} onclick={() => openDir(logDir)}><FolderOpen size={14} /> Open</button>
            </div>
            <button class="st-about-row" type="button" onclick={openRepo}>
              <span class="st-about-ic"><ExternalLink size={15} /></span>
              <span class="st-about-body"><span class="st-about-t">Source code &amp; license</span><span class="st-about-s">MIT-licensed and open on GitHub — github.com/Blazzer10200/rift-tauri</span></span>
            </button>
            <button class="st-powered" type="button" onclick={() => void openClaudeCode()} use:tooltip={"Rift runs every turn through Anthropic's Claude Code CLI — opens claude.com/claude-code. Rift is an independent project, not affiliated with Anthropic."}>
              <span class="st-powered-mark"><Terminal size={15} /></span>
              <span class="st-powered-t">Powered by <b>Claude Code</b></span>
            </button>
          </div>

          <div class="card" id="card-shortcuts">
            <div class="card-tt">Keyboard shortcuts</div>
            <div class="card-sub">Move around Rift without the mouse.</div>
            {#each shortcutsShown as sc, ti (sc.label)}
              <div class="kbd-row anim-reveal" style="--ti: {ti}">
                <span>{sc.label}</span>
                <span class="keys">
                  {#each sc.combo as k}<b>{k}</b>{/each}
                  {#if sc.alt}<span class="kbd-or">or</span>{#each sc.alt as k}<b>{k}</b>{/each}{/if}
                </span>
              </div>
            {/each}
            <button class="set-expand" type="button" aria-expanded={shortcutsExpanded} onclick={() => (shortcutsExpanded = !shortcutsExpanded)}>
              {shortcutsExpanded ? "Show less" : `Show all ${SHORTCUTS.length} shortcuts`}
              <ChevronDown size={13} class={shortcutsExpanded ? "set-expand-ic flip" : "set-expand-ic"} />
            </button>
          </div>

          <div class="card" id="card-tools">
            <div class="card-tt">Local tools</div>
            <div class="card-sub">{toolsMissing.length === 0
              ? "Optional programs that unlock extra assistant abilities. You have all of them — hover one to see what it does."
              : "Optional programs that unlock extra assistant abilities — install any missing one with a click."}</div>
            <!-- Healthy tools collapse to chips; a tool only earns a full row (what it
                 does + install CTA) while it's actually missing. -->
            {#if toolsInstalled.length > 0}
              <div class="tools-strip">
                {#each toolsInstalled as t (t.key)}
                  <span class="env-stat ok" use:tooltip={t.use}><span class="env-dot"></span>{t.label}</span>
                {/each}
                <button class="link-btn tools-recheck" type="button" onclick={() => void environment.refresh()} use:tooltip={"Probe the system again — e.g. after installing something yourself"}>Re-check</button>
              </div>
            {/if}
            {#each toolsMissing as t (t.key)}
              {@const installing = environment.installing[t.key]}
              <div class="ctl-row tight">
                <div><div class="ctl-t">{t.label}</div><div class="ctl-s">{t.use} {t.hint}</div></div>
                {#if installing}
                  <span class="env-stat warn"><Loader2 size={12} class="spin" /> Installing…</span>
                {:else}
                  <button class="st-btn primary" type="button" onclick={() => void environment.install(t.key)} use:tooltip={`Install ${t.label} via winget`}>
                    <Download size={13} /> Install
                  </button>
                {/if}
              </div>
            {/each}
            {#if environment.installError}
              <div class="st-note">⚠ {environment.installError}</div>
            {:else if Object.values(environment.installing).some(Boolean)}
              <div class="st-note">An install console opened — finish there; Rift re-checks every few seconds. Impatient? <button class="link-btn" type="button" onclick={() => void environment.refresh()}>Re-probe now</button>.</div>
            {/if}
          </div>

          <div class="card" id="card-help">
            <div class="card-tt">Help &amp; diagnostics</div>
            <div class="card-sub">Fix a wonky install, replay the intro, or file a bug.</div>
            <button class="st-about-row" type="button" onclick={() => (diagConsoleOpen = true)}>
              <span class="st-about-ic"><Activity size={15} /></span>
              <span class="st-about-body"><span class="st-about-t">Open diagnostics console</span><span class="st-about-s">Live event stream — filter, search, and export. Username-scrubbed.</span></span>
            </button>
            <button class="st-about-row" type="button" onclick={copyDiagnostic}>
              <span class="st-about-ic">{#if diagCopied}<Check size={15} />{:else}<Copy size={15} />{/if}</span>
              <span class="st-about-body"><span class="st-about-t">{diagCopied ? "Copied to clipboard" : "Copy diagnostic"}</span><span class="st-about-s">Version, platform, and paths — username-scrubbed</span></span>
            </button>
            <button class="st-about-row" type="button" onclick={reportBug}>
              <span class="st-about-ic"><Bug size={15} /></span>
              <span class="st-about-body"><span class="st-about-t">Report a bug</span><span class="st-about-s">Copies your diagnostic info, then opens a new GitHub issue to paste it into</span></span>
            </button>
            <button class="st-about-row" type="button" onclick={replayWalkthrough}>
              <span class="st-about-ic"><RotateCcw size={15} /></span>
              <span class="st-about-body"><span class="st-about-t">Replay first-run walkthrough</span><span class="st-about-s">Restarts the welcome setup right now — the beta &amp; AI-disclaimer notice will show again too</span></span>
            </button>
            <button class="st-about-row" type="button" onclick={repairInstall}>
              <span class="st-about-ic"><ShieldCheck size={15} /></span>
              <span class="st-about-body"><span class="st-about-t">Repair installation</span><span class="st-about-s">Re-download and reinstall the current version — fixes corrupted or missing program files. Rift will restart.</span></span>
            </button>
          </div>
      </div></div>
    {/if}

  </div>
</div>

{#if diagConsoleOpen}
  <DiagnosticsConsole onclose={() => (diagConsoleOpen = false)} />
{/if}

<style>
  /* Transparent — keeps the app dot-field continuous across surfaces
     (AssistantPage doctrine); opaque var(--bg) hid the user's texture here. */
  .sb-main { position: relative; overflow: hidden; display: flex; flex-direction: column; flex: 1; min-height: 0; min-width: 0; background: transparent; color: var(--fg); }

  /* ════════ Redesign RailShell (spec: rift-redesign.html) ════════ */
  @keyframes blockIn { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: none; } }

  /* hero tab bar + settings search share one row; the hairline lives on the row */
  .tabrow { display: flex; align-items: center; gap: 14px; border-bottom: 1px solid var(--border); }
  .tabnav { display: flex; gap: 4px; flex: 1; min-width: 0; }
  .snav { display: inline-flex; align-items: center; gap: 7px; height: 42px; padding: 0 4px; margin: 0 8px; background: none; border: 0; cursor: pointer; color: var(--fg-muted); font: inherit; font-size: 13px; font-weight: 500; border-bottom: 2px solid transparent; margin-bottom: -1px; transition: color var(--dur-fast); }
  .tabnav .snav:first-child { margin-left: 0; }
  .snav:hover { color: var(--fg-2); }
  .snav.on { color: var(--fg); border-bottom-color: var(--accent); }
  .snav :global(svg) { flex: none; color: var(--fg-subtle); transition: color var(--dur-fast); }
  .snav.on :global(svg) { color: var(--accent); }
  .snav-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); }
  .snav-dot.warn { background: var(--warn); }

  /* ── Settings search — find any control, jump to its card ── */
  .sset-search { position: relative; display: flex; align-items: center; gap: 7px; flex: none; width: 210px; height: 30px; margin-bottom: 6px; padding: 0 10px; border-radius: 999px; background: var(--field); border: 1px solid var(--field-border); transition: border-color var(--dur-fast), box-shadow var(--dur-fast), width 180ms var(--ease-page); }
  .sset-search:focus-within { width: 250px; border-color: color-mix(in oklab, var(--accent) 45%, var(--border)); box-shadow: 0 0 0 3px var(--ring); }
  .sset-search :global(.sset-search-ic) { flex: none; color: var(--fg-subtle); }
  .sset-search-in { flex: 1; min-width: 0; background: none; border: 0; outline: none; color: var(--fg); font: inherit; font-size: 12px; }
  .sset-search-in::placeholder { color: var(--fg-faint); }
  .sset-search-kbd { flex: none; font-size: 10px; line-height: 16px; padding: 0 5px; border: 1px solid var(--border); border-radius: 4px; color: var(--fg-faint); }
  .sset-search:focus-within .sset-search-kbd { display: none; }
  .sset-results { position: absolute; top: calc(100% + 8px); right: 0; width: 330px; max-height: 340px; overflow-y: auto; z-index: 30; padding: 5px; border-radius: 12px; background: color-mix(in oklab, var(--surface) 92%, transparent); backdrop-filter: blur(14px); border: 1px solid color-mix(in oklab, var(--accent) 14%, var(--border)); box-shadow: 0 18px 44px -8px oklch(0 0 0 / 0.6); scrollbar-width: thin; }
  .sset-result { display: flex; align-items: center; gap: 10px; width: 100%; padding: 7px 9px; border: 0; background: none; text-align: left; font: inherit; cursor: pointer; border-radius: 8px; }
  .sset-result[data-active="true"] { background: color-mix(in oklab, var(--accent) 14%, transparent); }
  .sset-result-body { flex: 1; min-width: 0; }
  .sset-result-t { display: block; font-size: 12.5px; font-weight: 550; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sset-result-s { display: block; font-size: 10.5px; color: var(--fg-subtle); margin-top: 1px; }
  .sset-result-tab { flex: none; font-size: 9px; font-weight: 700; letter-spacing: 0.05em; text-transform: uppercase; padding: 1.5px 7px; border-radius: 999px; color: var(--fg-muted); background: var(--bg-elev-3); border: 1px solid var(--border); }
  .sset-result[data-active="true"] .sset-result-tab { color: var(--accent); border-color: color-mix(in oklab, var(--accent) 32%, transparent); background: color-mix(in oklab, var(--accent) 10%, transparent); }
  .sset-result :global(.sset-result-kbd) { flex: none; color: var(--accent); }

  /* search-jump landing pulse — draws the eye to the card you asked for */
  :global(.card.sflash) { animation: sflash 1.6s var(--ease-page) both; }
  @keyframes sflash {
    0% { box-shadow: 0 0 0 2px color-mix(in oklab, var(--accent) 70%, transparent), 0 0 26px color-mix(in oklab, var(--accent) 30%, transparent); }
    60% { box-shadow: 0 0 0 2px color-mix(in oklab, var(--accent) 45%, transparent), 0 0 14px color-mix(in oklab, var(--accent) 14%, transparent); }
    100% { box-shadow: none; }
  }

  .surface-body { flex: 1; min-height: 0; overflow-y: auto; overflow-x: hidden; scroll-behavior: smooth; scrollbar-width: thin; scrollbar-color: var(--border-strong) transparent; }

  /* shared rail layout */
  .set-surface { max-width: 820px; margin: 0 auto; padding: 26px 40px 48px; }
  .set-col { min-width: 0; }
  .set-col > .card { margin-bottom: 16px; animation: blockIn var(--dur-base) var(--ease-page) both; }
  .set-col > .card:last-child { margin-bottom: 0; }
  /* Gentle top-down stagger so a section's cards assemble in order rather than
     flashing in together — same cadence as the AI Health dashboard. */
  .set-col > .card:nth-child(2) { animation-delay: 50ms; }
  .set-col > .card:nth-child(3) { animation-delay: 100ms; }
  .set-col > .card:nth-child(4) { animation-delay: 150ms; }
  .set-col > .card:nth-child(n+5) { animation-delay: 190ms; }
  @media (prefers-reduced-motion: reduce) { .set-col > .card { animation: none; } }

  /* card */
  /* island dialect: translucent tint + hairline (no shadow) so the canvas glow
     reads through settings the same way it does the stream + sidebar. */
  .card { background: var(--island-fill); border: 1px solid var(--island-border); border-radius: 12px; padding: 16px 18px; margin-bottom: 16px; scroll-margin-top: 12px; transition: border-color var(--dur-fast); }
  .card:hover { border-color: var(--border-strong); }

  /* Claude tab — CLI version chip + billing route strip */
  .st-cli-chip { display: inline-flex; align-items: center; height: 26px; padding: 0 11px; border-radius: 999px; font-size: 11px; font-weight: 600; color: var(--fg-2); background: var(--bg-inset); border: 1px solid var(--border-strong); white-space: nowrap; cursor: help; }
  .route { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; margin-bottom: 12px; padding: 9px 12px; border-radius: var(--radius); background: color-mix(in oklab, var(--fg) 3%, transparent); border: 1px dashed color-mix(in oklab, var(--border-strong) 80%, transparent); font-size: 11px; }
  .route-node { padding: 2px 9px; border-radius: 999px; font-weight: 600; color: var(--fg-subtle); background: var(--bg-elev-2); border: 1px solid var(--border); }
  .route-node.on { color: var(--accent); background: color-mix(in oklab, var(--accent) 12%, transparent); border-color: color-mix(in oklab, var(--accent) 34%, transparent); box-shadow: 0 0 8px color-mix(in oklab, var(--accent) 14%, transparent); }
  .route-or, .route-arrow { color: var(--fg-faint); }
  .route-note { color: var(--fg-muted); font-style: italic; }

  /* gated sub-rows — indent + rail so dependents read as children of their toggle */
  .ctl-row.sub-row { margin-left: 14px; padding-left: 14px; border-left: 2px solid color-mix(in oklab, var(--border-strong) 70%, transparent); }
  .ctl-row.sub-row:last-of-type { border-bottom: 1px solid color-mix(in oklch, var(--border) 55%, transparent); }

  /* mini activity-stream preview (Chat rendering) */
  .sp-stage { padding: 12px 14px; display: flex; flex-direction: column; gap: 6px; font-size: 11.5px; line-height: 1.5; }
  .sp-head { color: var(--fg-2); font-weight: 650; font-size: 12px; }
  .sp-prose { color: var(--fg-2); font-family: var(--font-sans, inherit); }
  .sp-note { color: var(--fg-subtle); font-style: italic; }
  .sp-tool { color: var(--fg-muted); }
  .sp-tool.open { color: var(--fg-2); }
  .sp-chev { color: var(--fg-subtle); margin-right: 2px; }
  .sp-file { color: var(--accent); }
  .sp-diff { color: var(--fg-subtle); margin-left: 6px; }
  .sp-tool-body { margin-left: 16px; padding: 6px 9px; border-left: 2px solid var(--border-strong); background: color-mix(in oklab, var(--fg) 3%, transparent); border-radius: 0 6px 6px 0; font-size: 11px; }
  .sp-add { color: var(--ok); }
  .sp-del { color: var(--danger); }
  .sp-cmd { color: var(--fg-2); margin-top: 2px; }
  .sp-out { color: var(--fg-muted); }
  .sp-out.dim { color: var(--fg-subtle); font-style: italic; }

  /* classic-bubbles preview (Stream view OFF) — schematic, not the live renderer */
  .bp-stage { padding: 12px 14px; display: flex; flex-direction: column; gap: 8px; }
  .bp-row { display: flex; }
  .bp-row.user { justify-content: flex-end; }
  .bp-bubble { max-width: 78%; padding: 8px 12px; border-radius: 12px; border-bottom-left-radius: 4px; border: 1px solid var(--border); background: var(--bg-elev-2); color: var(--fg-2); font-size: 12px; line-height: 1.5; }
  .bp-bubble.user { border-radius: 12px; border-bottom-right-radius: 4px; background: color-mix(in oklab, var(--accent) 14%, var(--bg-elev-2)); border-color: color-mix(in oklab, var(--accent) 30%, var(--border)); color: var(--fg); }
  .bp-tool { font-size: 11px; color: var(--fg-muted); padding-left: 2px; }

  /* plan → context-window mini gauge */
  .ctl-row.no-line { border-bottom: 0; }
  .plan-gauge { display: flex; align-items: center; gap: 12px; padding: 2px 0 10px; }
  .plan-gauge-track { position: relative; flex: 1; height: 6px; border-radius: 999px; background: var(--track); border: 1px solid var(--border); overflow: hidden; }
  .plan-gauge-fill { height: 100%; border-radius: 999px; background: linear-gradient(90deg, color-mix(in oklab, var(--accent) 55%, transparent), var(--accent)); transition: width 420ms var(--ease-page); }
  .plan-gauge-tick { position: absolute; left: 20%; top: 0; bottom: 0; width: 1px; background: var(--border-strong); }
  .plan-gauge-cap { flex: none; min-width: 86px; text-align: right; font-size: 10.5px; color: var(--fg-muted); }

  /* reading-comfort reply preview — bare .bubble/.markdown-body so the global
     data-a11y-* rules (app.css) style it exactly like a real reply */
  .cf-stage { padding: 13px 14px; }
  .cf-bubble { max-width: 100%; padding: 11px 14px; border-radius: 12px; border: 1px solid var(--border); font-size: 13px; }
  .cf-bubble p { margin: 0; }
  .cf-bubble code { font-family: var(--font-mono); border: 1px solid var(--code-border); padding: 1px 5px; border-radius: 4px; font-size: 0.92em; }
  /* Contested props sit in :where() (zero specificity) so the global
     data-a11y-* tint/line-height rules outrank them when active. */
  :where(.cf-bubble) { background: var(--bg-elev-2); color: var(--fg-2); }
  :where(.cf-bubble p) { line-height: 1.6; }
  :where(.cf-bubble code) { background: var(--code-bg); color: var(--code-fg); }

  /* accent "in action" stage — one of each accent-driven surface, live */
  .card-tt-act { margin-left: auto; }
  .ap-stage { display: flex; align-items: center; flex-wrap: wrap; gap: 16px; padding: 13px 14px; pointer-events: none; user-select: none; }
  .ap-demo-btn { display: inline-flex; align-items: center; height: 26px; padding: 0 14px; border-radius: 8px; font-size: 12px; font-weight: 650; background: var(--accent); color: var(--accent-fg); box-shadow: 0 2px 8px color-mix(in oklab, var(--accent) 30%, transparent); }
  .ap-demo-toggle { pointer-events: none; }
  .ap-demo-pill { display: inline-flex; align-items: center; gap: 6px; height: 24px; padding: 0 11px; border-radius: 999px; font-size: 11px; font-weight: 600; color: var(--accent); background: var(--accent-soft); border: 1px solid var(--ghost-border); }
  .ap-demo-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 6px color-mix(in oklab, var(--accent) 60%, transparent); }
  .ap-demo-sel { font-size: 12px; color: var(--fg); padding: 2px 4px; border-radius: 3px; background: color-mix(in oklab, var(--accent) 28%, transparent); }
  .ap-demo-link { font-size: 12px; color: var(--accent); text-decoration: underline; text-underline-offset: 3px; text-decoration-color: color-mix(in oklab, var(--accent) 55%, transparent); }
  .ap-demo-field { display: inline-flex; align-items: center; height: 26px; padding: 0 10px; border-radius: 7px; font-size: 11px; color: var(--fg-muted); background: var(--field); border: 1px solid var(--border-focus); box-shadow: 0 0 0 3px var(--ring); }

  /* density stage — list rows driven by the live --row-h/--gap/--fs-md vars */
  .iface-stage { display: flex; flex-direction: column; gap: var(--gap, 9px); padding: 12px 14px; border-bottom: 1px solid var(--border); }
  .iface-row { display: flex; align-items: center; gap: 9px; height: var(--row-h, 30px); padding: 0 11px; border-radius: 7px; font-size: var(--fs-md, 13px); color: var(--fg-2); background: color-mix(in oklab, var(--fg) 3.5%, transparent); border: 1px solid color-mix(in oklch, var(--border) 60%, transparent); transition: height var(--dur-base) var(--ease-page); }
  .iface-row.on { color: var(--fg); background: color-mix(in oklab, var(--accent) 10%, transparent); border-color: color-mix(in oklab, var(--accent) 28%, transparent); }
  .iface-dot { width: 7px; height: 7px; border-radius: 50%; flex: none; background: var(--fg-faint); }
  .iface-dot.on { background: var(--accent); box-shadow: 0 0 6px color-mix(in oklab, var(--accent) 60%, transparent); }
  .iface-meta { margin-left: auto; font-size: 10px; color: var(--fg-subtle); }

  /* live code preview — mirrors the chat renderer via the shared --code-* vars */
  .code-preview { margin-top: 14px; border: 1px solid var(--border); border-radius: 10px; background: var(--bg-inset); overflow: hidden; }
  .code-preview-bar { display: flex; align-items: center; gap: 5px; padding: 7px 11px; border-bottom: 1px solid var(--border); }
  .code-preview-bar > span:nth-child(-n+3) { width: 8px; height: 8px; border-radius: 50%; background: var(--bg-elev-3); border: 1px solid var(--border-strong); }
  .code-preview-name { margin-left: auto; font-size: 10px; color: var(--fg-faint); }
  .code-preview pre { margin: 0; padding: 12px 14px; overflow-x: auto; font-family: var(--font-mono); line-height: 1.65; color: var(--fg-2); font-size: var(--code-fs, 12px); tab-size: var(--code-tab, 2); font-variant-ligatures: var(--code-liga, none); }
  .cp-kw { color: var(--accent); }
  .cp-op { color: color-mix(in oklab, var(--accent) 65%, var(--fg-2)); }
  .cp-str { color: var(--ok); }
  .cp-num { color: var(--warn); }
  .cp-cm { color: var(--fg-subtle); font-style: italic; }

  /* progressive-disclosure expander — one quiet row under a collapsed list */
  .set-expand { display: flex; align-items: center; justify-content: center; gap: 6px; width: 100%; margin-top: 12px; padding: 7px 0; border: 1px dashed color-mix(in oklab, var(--border-strong) 80%, transparent); border-radius: var(--radius); background: none; cursor: pointer; font: inherit; font-size: 11.5px; font-weight: 550; color: var(--fg-subtle); transition: color var(--dur-fast), border-color var(--dur-fast), background var(--dur-fast); }
  .set-expand:hover { color: var(--fg-2); border-color: var(--border-strong); background: color-mix(in oklab, var(--fg) 4%, transparent); }
  .set-expand :global(.set-expand-ic) { transition: transform var(--dur-fast) var(--ease-page); }
  .set-expand :global(.set-expand-ic.flip) { transform: rotate(180deg); }
  /* per-tab factory reset — same quiet chrome, warn tint on hover so it reads
     as "careful" without shouting */
  .set-reset { margin-top: 0; }
  .set-reset:hover { color: var(--warn); border-color: color-mix(in oklab, var(--warn) 40%, var(--border)); background: color-mix(in oklab, var(--warn) 6%, transparent); }

  /* expanding a collapsed list cascades the new entries in (keyed each — the
     already-visible head keeps its DOM, so only revealed items animate) */
  .anim-reveal { animation: reveal-in var(--dur-base) var(--ease-page) both; animation-delay: calc(min(var(--ti, 0), 16) * 22ms); }
  @keyframes reveal-in {
    from { opacity: 0; transform: translateY(7px) scale(0.985); }
    to   { opacity: 1; transform: none; }
  }
  @media (prefers-reduced-motion: reduce) { .bg-opt, .anim-reveal { animation: none; } }
  .card-tt { display: flex; align-items: center; gap: 8px; font-size: 14px; font-weight: 650; margin-bottom: 3px; }
  .card-sub { font-size: 11.5px; color: var(--fg-subtle); margin-bottom: 14px; }
  .card-divider { height: 1px; background: var(--border); margin: 20px 0; }
  .ap-dot { width: 13px; height: 13px; border-radius: 50%; box-shadow: 0 0 0 1px var(--border-strong), 0 0 10px -1px oklch(0.72 var(--accent-c) var(--accent-h) / 0.6); flex: none; }
  .ap-divider { height: 1px; background: var(--border); margin: 16px 0 14px; }
  .card-sub code, .ctl-s code { font-family: var(--font-mono); background: var(--code-bg); border: 1px solid var(--code-border); padding: 1px 5px; border-radius: 4px; color: var(--code-fg); }

  /* control rows */
  .ctl-row { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 12px 0; border-bottom: 1px solid color-mix(in oklch, var(--border) 55%, transparent); }
  .ctl-row:last-child { border-bottom: 0; padding-bottom: 0; }
  .ctl-row.tight { padding: 9px 0; }
  .ctl-row.stack { flex-direction: column; align-items: stretch; gap: 10px; }
  .ctl-row[data-disabled="true"] { opacity: 0.5; }
  /* dim only the label side when a control is overridden by another pref (the
     seg buttons dim themselves via :disabled) */
  .ctl-row.overridden .ctl-t, .ctl-row.overridden .ctl-s { opacity: 0.5; }
  .ctl-note { font-weight: 400; font-size: 11px; color: var(--accent); opacity: 0.9; }
  .ctl-t { font-size: 13px; font-weight: 500; }
  /* "you've drifted off a preset" tag — the seg shows no active pill then */
  .preset-custom { margin-left: 8px; font-size: 10px; font-weight: 600; letter-spacing: 0.03em;
    padding: 1.5px 7px; border-radius: 999px; color: var(--fg-subtle); vertical-align: 1px;
    background: color-mix(in oklab, var(--fg) 7%, transparent);
    border: 1px solid color-mix(in oklab, var(--fg) 9%, transparent); }
  .ctl-s { font-size: 11.5px; color: var(--fg-subtle); margin-top: 2px; }
  .ctl-actions { display: flex; align-items: center; gap: 8px; flex: none; }

  /* segmented control */
  .seg { display: inline-flex; padding: 2px; background: var(--track); border: 1px solid var(--border); border-radius: 9px; gap: 2px; flex: none; }
  .seg button { height: 26px; padding: 0 14px; border: 0; background: none; cursor: pointer; color: var(--fg-muted); font: inherit; font-size: 12px; border-radius: 7px; transition: background var(--dur-fast), color var(--dur-fast); }
  .seg button:hover:not(:disabled) { color: var(--fg); }
  .seg button.on { background: var(--surface-active); color: var(--fg); box-shadow: var(--shadow-sm); }
  .seg button:disabled { opacity: 0.5; cursor: not-allowed; }

  /* toggle — canonical .rift-toggle from app.css */

  /* accent swatches + hue spectrum + vividness */
  .swatches { display: grid; grid-template-columns: repeat(8, 1fr); gap: 9px; max-width: 460px; }
  .sw { aspect-ratio: 1.4; border-radius: 9px; border: 1px solid var(--border); cursor: pointer; display: grid; place-items: center; transition: transform var(--dur-fast); }
  .sw:hover { transform: translateY(-2px); }
  .sw.sel { box-shadow: 0 0 0 2px var(--surface), 0 0 0 4px var(--accent); }
  .hue-row { display: flex; align-items: center; gap: 12px; margin-top: 12px; }
  .hue-range { -webkit-appearance: none; appearance: none; flex: 1; min-width: 0; height: 12px; border-radius: 999px; cursor: pointer; border: 1px solid var(--border);
    background: linear-gradient(90deg, oklch(0.72 0.17 20), oklch(0.78 0.16 70), oklch(0.80 0.16 110), oklch(0.78 0.15 160), oklch(0.74 0.13 200), oklch(0.70 0.15 250), oklch(0.68 0.18 300), oklch(0.72 0.18 340), oklch(0.72 0.17 380)); }
  .hue-range::-webkit-slider-thumb { -webkit-appearance: none; width: 20px; height: 20px; border-radius: 50%; background: oklch(0.82 var(--accent-c) var(--accent-h)); border: 3px solid var(--bg-inset); box-shadow: 0 0 0 1px var(--border-strong), var(--shadow-sm); cursor: pointer; transition: transform var(--dur-fast) var(--ease-page); }
  .hue-range::-webkit-slider-thumb:hover { transform: scale(1.12); }
  .hue-range:focus { outline: none; }
  .range-wrap { display: flex; align-items: center; gap: 12px; }
  /* full-width variant — matches the hue slider's span so the two dials read as one family */
  .range-wrap.grow { flex: 1 1 auto; max-width: 440px; }
  .range-wrap.grow .set-range { flex: 1; width: auto; min-width: 0; }
  .range-val { font-size: 11.5px; color: var(--fg-muted); font-variant-numeric: tabular-nums; min-width: 36px; text-align: right; }
  .set-range { -webkit-appearance: none; appearance: none; width: 150px; height: 6px; border-radius: 999px; border: 1px solid var(--border); background: linear-gradient(90deg, var(--accent) var(--fill, 0%), var(--track) var(--fill, 0%)); cursor: pointer; }
  .set-range::-webkit-slider-thumb { -webkit-appearance: none; width: 15px; height: 15px; border-radius: 50%; background: var(--accent); border: 2px solid var(--bg-inset); box-shadow: var(--shadow-sm); cursor: pointer; transition: transform var(--dur-fast) var(--ease-page); }
  .set-range::-webkit-slider-thumb:hover { transform: scale(1.14); }
  .set-range:focus { outline: none; }
  .set-range:focus-visible::-webkit-slider-thumb { box-shadow: 0 0 0 3px var(--ring); }

  /* keyboard shortcut rows (spec flex variant) */
  .keys { display: inline-flex; gap: 4px; }
  .keys b { font-family: var(--font-mono); font-size: 11px; color: var(--fg-muted); background: var(--bg-elev-2); border: 1px solid var(--border-strong); border-radius: 4px; padding: 2px 7px; }
  .kbd-or { color: var(--fg-faint); font-size: 10px; margin: 0 2px; }
  .mono { font-family: var(--font-mono); }

  /* ── Hero tab bar (hero chrome via PageHero component) ── */
  .sb-chip { display: inline-flex; align-items: center; gap: 7px; height: 30px; padding: 0 12px; border-radius: 999px; background: var(--surface); border: 1px solid var(--border); color: var(--fg-2); font: inherit; font-size: var(--fs-xs); font-weight: 600; cursor: default; }
  button.sb-chip { cursor: pointer; transition: background var(--dur-fast), border-color var(--dur-fast); }
  button.sb-chip:hover { background: var(--surface-hover); border-color: var(--border-strong); }
  .sb-chip.ok { background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); color: var(--ok); }
  .sb-chip.ok :global(svg) { color: var(--ok); }
  .sb-chip.warn { background: var(--warn-soft); border-color: color-mix(in oklch, var(--warn) 28%, transparent); color: var(--warn); }
  .sb-chip.warn :global(svg) { color: var(--warn); }
  .sb-chip.danger { background: var(--danger-soft); border-color: color-mix(in oklch, var(--danger) 28%, transparent); color: var(--danger); }
  .sb-chip.danger :global(svg) { color: var(--danger); }
  /* Dev build — calm neutral, not an alarm. A quiet mono tag, no colored fill. */
  .sb-chip.dev { color: var(--fg-2); }
  .sb-chip.dev :global(svg) { color: var(--fg-subtle); }
  /* The static "local workspace" chip carries no state — recede it below the
     interactive version pill so the eye lands on the actionable one. */
  .sb-chip.ghost { background: transparent; border-color: color-mix(in oklch, var(--border) 70%, transparent); color: var(--fg-subtle); }
  .sb-chip-ver { font-family: var(--font-mono); font-weight: 600; }
  /* Version → tag separator: a mid-dot in the chip's own muted ink. */
  .sb-chip-tag { position: relative; padding-left: 9px; margin-left: 2px; font-size: 11px; opacity: 0.9; }
  .sb-chip-tag::before { content: ""; position: absolute; left: 0; top: 50%; width: 3px; height: 3px; margin-top: -1.5px; border-radius: 50%; background: currentColor; opacity: 0.55; }
  .sb-chip .mono { font-family: var(--font-mono); }

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

  /* Tool-presence pill (About → Local tools). Dot + label, tinted by status. */
  .env-stat { display: inline-flex; align-items: center; gap: 6px; height: 26px; padding: 0 11px; border-radius: 999px; font-size: var(--fs-xs); font-weight: 600; white-space: nowrap; border: 1px solid transparent; }
  .env-stat .env-dot { width: 7px; height: 7px; border-radius: 999px; flex: none; }
  .env-stat.ok { color: var(--ok); background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 28%, transparent); }
  .env-stat.ok .env-dot { background: var(--ok); box-shadow: 0 0 0 3px var(--ok-soft); }
  .env-stat.warn { color: var(--warn); background: var(--warn-soft); border-color: color-mix(in oklch, var(--warn) 28%, transparent); }
  /* Status rows: label + status-cluster on the top line, description full-width below. */
  /* CLI install list + update CTA live inside the hero banner — span its full width. */
  .sb-status > .st-cli-installs, .sb-status > .st-cli-act, .sb-status > .st-cli-err, .sb-status > .st-cli-ok, .sb-status > .st-cli-warn { flex: 1 1 100%; margin-top: 0; }
  .st-note { padding: 10px 0 0; margin-top: 8px; font-size: var(--fs-xs); color: var(--fg-muted); border-top: 1px solid var(--border); }
  .st-note code { font-family: var(--font-mono); background: var(--code-bg); border: 1px solid var(--code-border); padding: 1px 5px; border-radius: 4px; color: var(--code-fg); }
  .st-warn { display: block; font-size: var(--fs-xs); color: var(--warn); line-height: 1.5; padding: 10px 13px; background: var(--warn-soft); border: 1px solid color-mix(in oklab, var(--warn) 32%, transparent); border-radius: var(--r-card); }
  .st-warn code { background: color-mix(in oklab, var(--warn) 16%, transparent); border: 1px solid color-mix(in oklab, var(--warn) 30%, transparent); padding: 1px 5px; border-radius: 4px; color: var(--warn); font-family: var(--font-mono); }
  .st-warn strong { font-weight: 600; color: var(--fg); }

  /* Neutral, friendly note for expected states (not an error) — used when a build feature simply isn't present. */
  .st-info { display: block; font-size: var(--fs-xs); line-height: 1.5; padding: 11px 13px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--r-card); }
  .st-info-t { font-weight: 600; color: var(--fg); }
  .st-info-s { color: var(--fg-muted); margin-top: 3px; }
  .st-dev { margin-top: 9px; }
  .st-dev > summary { cursor: pointer; color: var(--fg-muted); font-size: var(--fs-xs); list-style: none; user-select: none; display: inline-flex; align-items: center; gap: 5px; }
  .st-dev > summary::before { content: "›"; display: inline-block; transition: transform var(--dur-fast) ease-out; }
  .st-dev[open] > summary::before { transform: rotate(90deg); }
  .st-dev > summary:hover { color: var(--fg); }
  .st-dev-body { margin-top: 7px; color: var(--fg-muted); }
  .st-dev-body ol { margin: 6px 0 0; padding-left: 18px; display: flex; flex-direction: column; gap: 4px; }
  .st-dev-body code { font-family: var(--font-mono); background: var(--code-bg); border: 1px solid var(--code-border); padding: 1px 5px; border-radius: 4px; color: var(--code-fg); }

  /* Input · button · status-pill kit → $lib/styles/settings-controls.css */
  .st-stamp { font-family: var(--font-mono); font-size: 10.5px; color: var(--fg-faint); white-space: nowrap; }

  /* ── Custom-provider list (2a) ── */

  /* CLI update command line — copyable npm install command. */
  .st-cli-cmd { display: flex; align-items: center; gap: 8px; margin-top: 8px; max-width: 360px; background: color-mix(in oklch, white 9%, var(--surface)); border: 1px solid var(--border-strong); border-radius: 8px; padding: 6px 7px 6px 10px; }
  .st-cli-cmd code { flex: 1; min-width: 0; font-family: var(--font-mono); font-size: 11px; color: var(--fg); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .st-cli-copy { flex-shrink: 0; display: inline-flex; align-items: center; justify-content: center; width: 26px; height: 24px; border-radius: 6px; border: 1px solid var(--border); background: var(--surface); color: var(--fg-muted); cursor: pointer; transition: color var(--dur-fast), border-color var(--dur-fast), background var(--dur-fast); }
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
  .st-cli-inst-tag.unknown { background: color-mix(in oklab, var(--fg-muted) 20%, transparent); color: var(--fg-muted); cursor: help; }
  .st-cli-warn { color: var(--warn); cursor: help; margin-top: 2px; }
  .st-cli-inst-path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: right; opacity: 0.6; font-family: var(--font-mono); font-size: 10px; }
  .st-cli-act { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; margin-top: 8px; }
  .st-cli-act .st-cli-cmd { margin-top: 0; }
  .st-cli-err { margin-top: 7px; font-size: var(--fs-xs); color: var(--danger); white-space: pre-wrap; }
  .st-cli-ok { margin-top: 7px; font-size: var(--fs-xs); color: var(--fg-muted); white-space: pre-wrap; }
  .st-cli-warn { margin-top: 7px; font-size: var(--fs-xs); color: var(--warn); line-height: 1.4; }
  .st-cli-note { margin-top: 7px; font-size: var(--fs-xs); color: var(--fg-muted); line-height: 1.4; cursor: help; }

  /* ── About: Build identity header ── */
  .st-build { display: flex; align-items: center; gap: 14px; padding: 4px 0 14px; margin-bottom: 4px; border-bottom: 1px solid var(--border); }
  .st-build-mark { width: 40px; height: 40px; border-radius: 11px; flex: none; display: grid; place-items: center; color: var(--accent); background: var(--accent-soft); border: 1px solid color-mix(in oklab, var(--accent) 26%, transparent); }
  .st-build-id { min-width: 0; }
  .st-build-name { font-size: 16px; font-weight: 680; letter-spacing: -0.01em; color: var(--fg); }
  .st-build-ver { font-family: var(--font-mono); font-weight: 600; color: var(--fg-2); font-size: 14px; margin-left: 4px; }
  .st-build-chan { font-size: 11.5px; color: var(--fg-muted); margin-top: 3px; display: flex; align-items: center; gap: 6px; }
  .st-build-tag { font-size: 10px; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; padding: 1.5px 7px; border-radius: 999px; border: 1px solid transparent; }
  .st-build-tag.ok { color: var(--ok); background: var(--ok-soft); border-color: color-mix(in oklch, var(--ok) 26%, transparent); }
  .st-build-tag.warn { color: var(--warn); background: var(--warn-soft); border-color: color-mix(in oklch, var(--warn) 26%, transparent); }
  .st-build-tag.dev { color: var(--fg-2); background: color-mix(in oklab, var(--fg) 8%, transparent); border-color: color-mix(in oklab, var(--fg) 12%, transparent); }

  /* ── About: powered-by-Claude attribution ── */
  .st-powered { display: flex; align-items: center; gap: 9px; width: 100%; margin-top: 12px; padding: 11px 0 2px; border: 0; border-top: 1px solid var(--border); background: none; cursor: pointer; font: inherit; font-size: 12px; color: var(--fg-muted); text-align: left; transition: color var(--dur-fast); }
  .st-powered:hover { color: var(--fg-2); }
  .st-powered-mark { display: flex; flex: none; color: #d97757; }
  .st-powered-t b { font-weight: 640; color: var(--fg-2); transition: color var(--dur-fast); }
  .st-powered:hover .st-powered-t b { color: var(--fg); }

  /* ── About: kv + resource rows ── */
  .st-build-check { margin-left: auto; flex: none; }
  .st-stack { display: flex; flex-wrap: wrap; gap: 6px; padding: 11px 0 9px; }
  .st-stack-chip { font-family: var(--font-mono); font-size: 11px; line-height: 1; color: var(--fg-muted); background: var(--field); border: 1px solid var(--field-border); border-radius: 999px; padding: 5px 10px; }
  .tools-strip { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; padding: 8px 0 4px; }
  .tools-recheck { margin-left: auto; }
  .link-btn { border: 0; background: none; padding: 0; font: inherit; font-size: var(--fs-xs); color: var(--fg-muted); text-decoration: underline; text-underline-offset: 3px; cursor: pointer; }
  .link-btn:hover { color: var(--fg-2); }
  .st-about-row { display: flex; align-items: center; gap: 12px; padding: 13px 12px; margin: 0 -12px; width: calc(100% + 24px); border: 0; background: none; text-align: left; font: inherit; cursor: pointer; border-radius: 10px; }
  .st-about-row + .st-about-row { border-top: 1px solid var(--border); }
  .st-about-ic { width: 32px; height: 32px; border-radius: var(--radius); display: grid; place-items: center; background: var(--field); border: 1px solid var(--field-border); color: var(--fg-muted); flex: none; }
  .st-about-body { flex: 1; min-width: 0; }
  .st-about-t { font-size: var(--fs-sm); font-weight: 600; display: block; color: var(--fg); }
  .st-about-s { font-size: var(--fs-xs); color: var(--fg-muted); margin-top: 2px; display: block; }
  .st-about-row:hover { background: var(--surface-hover); }

  /* ── Keyboard shortcut rows ── */
  .kbd-row { display: flex; align-items: center; justify-content: space-between; padding: 10px 0; border-bottom: 1px solid color-mix(in oklch, var(--border) 55%, transparent); font-size: 13px; color: var(--fg-2); }
  .kbd-row:last-child { border-bottom: 0; padding-bottom: 0; }

  /* ── Speech: pick grid / models / textarea (lifted from legacy) ── */
  .set-pick-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 6px; width: 100%; }
  .set-pick-grid-2 { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .set-pick-grid-3 { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .set-pick { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 8px 11px; background: var(--field); border: 1px solid var(--field-border); border-radius: var(--radius); color: var(--fg); font: inherit; font-size: var(--fs-sm); cursor: pointer; text-align: left; transition: background var(--dur-fast), border-color var(--dur-fast); }
  .set-pick:hover { background: var(--surface-hover); }
  .set-pick[data-active="true"] { border-color: color-mix(in oklab, var(--accent) 45%, var(--border)); background: var(--accent-soft); color: var(--accent); }
  .set-pick-label { font-weight: 550; }
  .set-pick-sub { font-size: 10px; color: var(--fg-muted); }
  .set-pick[data-active="true"] .set-pick-sub { color: color-mix(in oklab, var(--accent) 80%, var(--fg-muted)); }
  .set-pick:disabled:not([data-active="true"]) { opacity: 0.5; cursor: not-allowed; }
  .set-model-list { display: flex; flex-direction: column; gap: 6px; }
  .set-model-row { display: flex; align-items: center; gap: 12px; padding: 10px 12px; background: var(--field); border: 1px solid var(--field-border); border-radius: var(--radius); }
  .set-model-row[data-active="true"] { border-color: color-mix(in oklab, var(--accent) 40%, var(--border)); background: var(--accent-soft); }
  .set-model-meta { flex: 1 1 auto; min-width: 0; }
  .set-model-name { font-size: var(--fs-sm); color: var(--fg); font-weight: 550; }
  .set-model-sub { font-size: 10px; color: var(--fg-muted); margin-top: 2px; }
  .set-model-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .set-progress { margin-top: 8px; height: 4px; background: var(--bg-elev-2); border-radius: 2px; overflow: hidden; }
  .set-progress-fill { height: 100%; background: var(--accent); transition: width var(--dur-fast) linear; }
  .set-progress-label { font-size: 10px; margin-top: 3px; color: var(--fg-muted); }
  .set-mic-r { width: 100%; max-width: 360px; }
  .set-mic-select { flex: 1 1 auto; }
  .set-textarea { width: 100%; padding: 8px 10px; background: var(--field); border: 1px solid var(--field-border); border-radius: var(--radius); color: var(--fg); font-family: var(--font-mono); font-size: var(--fs-xs); line-height: 1.5; resize: vertical; min-height: 60px; }
  .set-textarea:focus { outline: none; border-color: var(--border-focus); box-shadow: 0 0 0 3px var(--ring); }
  .set-textarea:disabled { opacity: 0.55; cursor: not-allowed; }
  /* Live "Administrator" indicator in the elevation card — accent-tinted, matches
     the app's positive/active color rather than the status-bar amber alert tone. */
  .admin-live { display: inline-flex; align-items: center; gap: 6px; font-size: var(--fs-sm); font-weight: 600;
    color: var(--accent); white-space: nowrap; }
  .admin-live :global(svg) { color: var(--accent); }
</style>
