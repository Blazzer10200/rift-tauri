<script lang="ts">
  import { Send, Square, X, Mic, Loader2, HelpCircle, Wand2, Check, Paperclip,
    Hand, Code2, ClipboardList, Zap, Infinity as InfinityIcon, SlidersHorizontal,
    Bot, Terminal, ListPlus, Sparkles } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import type { ModelSel, PermissionMode } from "../../state/assistant/types";
  import { modelFamily, liveActivity } from "../../state/assistant/helpers";
  import { stt } from "../../state/stt.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { tick, onMount } from "svelte";

  // Mic-button visibility binds to stt.config.enabled, so load the backend
  // stt config eagerly — otherwise users with STT enabled wouldn't see the
  // mic until they opened Settings → Speech once.
  onMount(() => { void stt.init(); });

  let {
    onsubmit,
    tabId = null,
  }: {
    onsubmit: (text: string) => void;
    tabId?: string | null;
  } = $props();

  // Per-pane composer: bind to THIS tab's draft/attachments/queue/streaming
  // rather than the focused-pane shims, so two panes can compose & stream
  // concurrently. Tab can be null transiently (empty pane during drag/drop);
  // the parent (AssistantPane) gates Composer rendering on tab presence.
  const tab = $derived(assistant.tabFor(tabId));
  const draft = $derived(tab?.draft ?? "");
  const attachments = $derived(tab?.attachments ?? []);
  const queue = $derived(tab?.queue ?? []);
  const streaming = $derived(tab?.streaming ?? false);

  // Context gauge — feeds the composer divider's fill. Per-tab so each pane
  // shows its own conversation's window usage. Tone steps mirror the tab-bar
  // ctx-pill (yellow ≥70, red ≥90) so the two readouts never disagree.
  const ctxPct = $derived(tab ? assistant.ctxPctFor(tab) : 0);
  const ctxTokens = $derived(tab ? assistant.ctxTokensFor(tab) : 0);
  const ctxWindow = $derived(tab ? assistant.ctxWindowFor(tab) : 0);
  const ctxTone = $derived(ctxPct >= 90 ? "red" : ctxPct >= 70 ? "yellow" : "ok");
  const ctxTitle = $derived(
    ctxTokens > 0
      ? `Context: ${ctxTokens.toLocaleString()} / ${ctxWindow.toLocaleString()} tokens (${ctxPct.toFixed(1)}%) — fills as the conversation grows`
      : "Context window — fills as the conversation grows",
  );

  // ── Live activity pills ───────────────────────────────────────────────
  // Compact, additive readout of in-flight work — reuses the Activity panel's
  // `liveActivity` derivation (single source of truth) plus the telemetry
  // tok/s, so a busy turn surfaces ◍ agents · ▸ shells · elapsed + tok/s ·
  // queued without opening the panel. The idle bar renders none of this. The
  // 1s ticker only runs while streaming (drives elapsed + tok/s refresh).
  let now = $state(Date.now());
  $effect(() => {
    if (!streaming) return;
    const h = setInterval(() => { now = Date.now(); }, 1000);
    return () => clearInterval(h);
  });
  const liveItems = $derived(liveActivity(tab?.messages ?? [], tab?.agentSpawns ?? [], now));
  const agentCount = $derived(liveItems.filter((i) => i.kind === "agent").length);
  const shellCount = $derived(liveItems.filter((i) => i.kind === "shell").length);
  const turnStartedAt = $derived(tab?.activity.turnStartedAt ?? null);
  const turnElapsed = $derived(
    streaming && turnStartedAt != null ? fmtClock(now - turnStartedAt) : null,
  );
  // tok/s is session-global telemetry; recompute each tick by touching `now`.
  const tokPerSec = $derived.by(() => {
    void now;
    return streaming ? assistant.telemetry.snapshot().summary.outputTokensPerSec : null;
  });
  const showLivePills = $derived(streaming || agentCount > 0 || shellCount > 0 || queue.length > 0);
  function fmtClock(ms: number): string {
    const s = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  }
  // Toggle the Activity dock: if it's already open ON the activity tab, a
  // second click closes it; otherwise open + switch to activity (so clicking
  // from another panel tab focuses activity rather than closing).
  function openActivity() {
    if (assistant.ui.dockOpen && assistant.ui.panelTab === "activity") {
      assistant.ui.dockOpen = false;
      return;
    }
    assistant.ui.panelTab = "activity";
    assistant.ui.dockOpen = true;
  }

  function setDraft(v: string) { if (tab) tab.draft = v; }
  function setAttachments(
    v: { id: string; mime: string; dataBase64: string; previewUrl: string; sizeBytes: number }[],
  ) {
    if (tab) tab.attachments = v;
  }

  let ta = $state<HTMLTextAreaElement | undefined>();

  type SlashCmd = { name: string; desc: string };
  // Grouped: conversation lifecycle → model + composition → flow control → info.
  const SLASH_COMMANDS: SlashCmd[] = [
    { name: "new",       desc: "Start a new conversation (saves current)" },
    { name: "clear",     desc: "Clear this chat in place (saves current to History)" },
    { name: "compact",   desc: "Summarize + remint the CLI session (optional focus)" },
    { name: "history",   desc: "Open conversation history" },
    { name: "model",     desc: "Switch model — opens picker" },
    { name: "retry",     desc: "Re-fire the last prompt" },
    { name: "copy",      desc: "Copy last response to clipboard" },
    { name: "stop",      desc: "Halt the current turn" },
    { name: "tools",     desc: "List available workspace tools" },
    { name: "cost",      desc: "Show session cost" },
    { name: "stats",     desc: "Session telemetry summary (inline)" },
    { name: "summarize", desc: "Dry-run a compaction summary (no state change)" },
    { name: "openincli", desc: "Print the claude --resume command for this session" },
    { name: "diag",      desc: "Copy full telemetry JSON to clipboard" },
    { name: "help",      desc: "List slash commands" },
  ];

  // Model picker rows — version + tagline + context window. The CLI takes
  // the alias (`sonnet`/`opus`/`haiku`); version is display-only and pulled
  // from CLAUDE.md's source-of-truth section on the current model family.
  type ModelOpt = {
    id: ModelSel;
    label: string;
    version: string;
    tagline: string;
    ctx: string;
  };
  // Laid out 2×2 (row-major): Sonnet · Haiku on top, the two Opus versions
  // below. `opus` is the alias → newest Opus (4.8, 1M-ctx beta);
  // `claude-opus-4-7` pins the prior generation. Both render purple via the
  // shared opus family hue.
  const MODEL_OPTIONS: ModelOpt[] = [
    { id: "sonnet",          label: "Sonnet", version: "4.6", tagline: "Best speed + intelligence balance — the default", ctx: "1M ctx" },
    { id: "haiku",           label: "Haiku",  version: "4.5", tagline: "Fastest, near-frontier — quick edits & lookups", ctx: "200K ctx" },
    { id: "claude-opus-4-7", label: "Opus",   version: "4.7", tagline: "Previous-generation Opus — proven for complex reasoning", ctx: "1M ctx" },
    { id: "opus",            label: "Opus",   version: "4.8", tagline: "Newest + most capable — complex reasoning & agentic coding", ctx: "1M ctx" },
  ];

  // Session-rotated idle placeholders — cycle every ~6s while the composer is
  // unfocused + empty, so the user sees tips drift past without staring at a
  // single line. Pauses on focus/draft so it never moves under the cursor.
  const IDLE_PLACEHOLDERS = [
    "Ask Claude",
    "Ask Claude · @ to mention a file",
    "Ask Claude · / for commands",
    "Ask Claude · Shift+Enter for newline",
    "Ask Claude · paste an image to attach",
  ];
  let placeholderIdx = $state(Math.floor(Math.random() * IDLE_PLACEHOLDERS.length));
  let placeholderKey = $state(0); // bumps to retrigger fade animation
  let composerFocused = $state(false);
  const idlePlaceholder = $derived(IDLE_PLACEHOLDERS[placeholderIdx % IDLE_PLACEHOLDERS.length]);
  $effect(() => {
    if (composerFocused || draft.length > 0 || streaming || attachments.length > 0) return;
    const h = setInterval(() => {
      placeholderIdx = (placeholderIdx + 1) % IDLE_PLACEHOLDERS.length;
      placeholderKey++;
    }, 6000);
    return () => clearInterval(h);
  });

  function autosize() {
    if (!ta) return;
    ta.style.height = "auto";
    ta.style.height = Math.min(ta.scrollHeight, 220) + "px";
  }

  $effect(() => {
    const _v = draft;
    void _v;
    void tick().then(() => {
      autosize();
      if (draft && ta) ta.focus();
    });
  });

  // ── `@`-file mention picker ────────────────────────────────────────────
  // Triggers when the caret is just past an `@` token, e.g. `look at @sr|`
  // matches but `you@example.com` does not (preceding char must be ws/start).
  // Insert path as plain text — Claude resolves via Read tool from there.
  type MentionState = { start: number; query: string };
  function detectMention(): MentionState | null {
    if (!ta) return null;
    const d = draft;
    const caret = ta.selectionStart ?? d.length;
    if (ta.selectionStart !== ta.selectionEnd) return null;
    let i = caret - 1;
    while (i >= 0) {
      const ch = d[i];
      if (ch === "@") {
        const prev = i === 0 ? " " : d[i - 1];
        if (!/\s/.test(prev) && i !== 0) return null;
        return { start: i, query: d.slice(i + 1, caret) };
      }
      if (/\s/.test(ch)) return null;
      i--;
    }
    return null;
  }
  let mentionState = $state<MentionState | null>(null);
  let mentionIdx = $state(0);
  function refreshMention() {
    mentionState = detectMention();
    if (mentionState && assistant.workspaceFiles.length === 0) {
      void assistant.loadWorkspaceFiles();
    }
  }
  // Light fuzzy match with three tiers: literal substring in basename,
  // fuzzy match in basename, fuzzy match anywhere. `Comp` against
  // `lib/foo/Composer.svelte` should beat `compress.lua` because the match
  // starts at the basename.
  function fuzzyScore(path: string, query: string): number | null {
    if (query.length === 0) return 0;
    const p = path.toLowerCase();
    const q = query.toLowerCase();
    const basenameStart = p.lastIndexOf("/") + 1;
    const basename = p.slice(basenameStart);
    const subIdx = basename.indexOf(q);
    if (subIdx !== -1) return 1000 - subIdx;
    let pi = 0;
    let firstHit = -1;
    for (const ch of q) {
      const found = basename.indexOf(ch, pi);
      if (found === -1) { firstHit = -1; pi = -1; break; }
      if (firstHit === -1) firstHit = found;
      pi = found + 1;
    }
    if (pi !== -1 && firstHit >= 0) return 500 - firstHit;
    pi = 0; firstHit = -1;
    for (const ch of q) {
      const found = p.indexOf(ch, pi);
      if (found === -1) return null;
      if (firstHit === -1) firstHit = found;
      pi = found + 1;
    }
    return -firstHit;
  }
  const mentionResults = $derived.by(() => {
    if (!mentionState) return [] as string[];
    const q = mentionState.query;
    const files = assistant.workspaceFiles;
    if (q.length === 0) return files.slice(0, 12);
    const scored: { path: string; score: number }[] = [];
    for (const f of files) {
      const s = fuzzyScore(f, q);
      if (s !== null) scored.push({ path: f, score: s });
      if (scored.length >= 800) break;
    }
    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, 12).map((r) => r.path);
  });
  $effect(() => {
    const _l = mentionResults.length;
    void _l;
    mentionIdx = 0;
  });
  function pickMention(path: string) {
    if (!mentionState || !ta) return;
    const d = draft;
    const caret = ta.selectionStart ?? d.length;
    const before = d.slice(0, mentionState.start);
    const after = d.slice(caret);
    const insertion = `@${path} `;
    setDraft(before + insertion + after);
    const newCaret = before.length + insertion.length;
    mentionState = null;
    void tick().then(() => {
      ta?.focus();
      ta?.setSelectionRange(newCaret, newCaret);
      autosize();
    });
  }

  // Slash menu state. Triggers when the draft starts with `/` and the
  // textarea has focus. Filters by the text after the slash.
  // One unified settings popover folds model + thinking depth + permission
  // mode into a single toolbar control. `settingsOpen` flips it; `settingsIdx`
  // is the flat cursor across all three sections (see `settingsRows`).
  let settingsOpen = $state(false);
  let settingsIdx = $state(0);
  const slashOpen = $derived(
    !settingsOpen &&
      draft.startsWith("/") &&
      !draft.includes(" ") &&
      draft.length >= 1,
  );
  const slashFiltered = $derived.by(() => {
    const q = draft.slice(1).toLowerCase();
    return SLASH_COMMANDS.filter((c) => c.name.startsWith(q));
  });
  let slashIdx = $state(0);
  $effect(() => {
    const _v = slashFiltered.length;
    void _v;
    slashIdx = 0;
  });
  // Current model row — drives the composer's bottom-right pill label.
  const currentModel = $derived(MODEL_OPTIONS.find((m) => m.id === assistant.model));

  // Effort ladder. Haiku skips extended thinking server-side regardless, so
  // hide the pill on Haiku to avoid implying it does something. Cycle on click:
  // none → quick → deep → none. Names describe quality not speed — "Fast"
  // and "Quick" were ambiguous siblings; Instant/Smart/Deep is a real ladder.
  type EffortOpt = { id: "none" | "quick" | "deep" | "ultra"; label: string; hint: string; level: 1 | 2 | 3 | 4 };
  const EFFORT_OPTIONS: EffortOpt[] = [
    { id: "none",  label: "Instant", level: 1, hint: "Instant — straight to the answer, no thinking time" },
    { id: "quick", label: "Smart",   level: 2, hint: "Smart — thinks briefly before answering (~5s extra)" },
    { id: "deep",  label: "Deep",    level: 3, hint: "Deep — heavy reasoning (~15s extra) for hard problems" },
    { id: "ultra", label: "Ultracode", level: 4, hint: "Ultracode — max reasoning + autonomous multi-agent workflows. Claude orchestrates fleets of subagents for the most exhaustive answer." },
  ];
  const currentEffort = $derived(EFFORT_OPTIONS.find((e) => e.id === assistant.thinkingEffort) ?? EFFORT_OPTIONS[1]);
  function pickEffort(e: EffortOpt) {
    assistant.setThinkingEffort(e.id);
    settingsOpen = false;
    void tick().then(() => ta?.focus());
  }

  // Permission-mode picker — mirrors the effort/model pills. Order matches the
  // VS Code Claude Code menu: ask → auto-edit → plan → auto → bypass. Icons
  // echo that menu (hand / code / clipboard / zap / infinity).
  type ModeOpt = { id: PermissionMode; label: string; icon: typeof Hand; hint: string };
  const MODE_OPTIONS: ModeOpt[] = [
    { id: "default",           label: "Ask before edits", icon: Hand,          hint: "Ask before edits — approve each change before it's made" },
    { id: "acceptEdits",       label: "Edit automatically", icon: Code2,       hint: "Edit automatically — apply file edits without asking" },
    { id: "plan",              label: "Plan mode",        icon: ClipboardList, hint: "Plan mode — explore and present a plan before editing" },
    { id: "auto",              label: "Auto mode",        icon: Zap,           hint: "Auto mode — pick the best permission mode per task" },
    { id: "bypassPermissions", label: "Bypass permissions", icon: InfinityIcon, hint: "Bypass permissions — never ask before running anything" },
  ];
  const currentMode = $derived(MODE_OPTIONS.find((m) => m.id === assistant.permissionMode) ?? MODE_OPTIONS[4]);
  function pickMode(m: ModeOpt) {
    assistant.setPermissionMode(m.id);
    settingsOpen = false;
    void tick().then(() => ta?.focus());
  }

  // Flat, navigable row list spanning all three sections of the unified
  // settings panel. Effort is dropped on Haiku (ignored server-side), exactly
  // as the old standalone effort pill was hidden there. Drives ArrowUp/Down
  // + the active highlight; mouse clicks call the per-kind pick fns directly.
  type SettingsRow =
    | { kind: "model"; model: ModelOpt }
    | { kind: "effort"; effort: EffortOpt }
    | { kind: "mode"; mode: ModeOpt };
  const settingsRows = $derived.by<SettingsRow[]>(() => {
    const rows: SettingsRow[] = MODEL_OPTIONS.map((m) => ({ kind: "model" as const, model: m }));
    if (assistant.model !== "haiku") {
      rows.push(...EFFORT_OPTIONS.map((e) => ({ kind: "effort" as const, effort: e })));
    }
    rows.push(...MODE_OPTIONS.map((m) => ({ kind: "mode" as const, mode: m })));
    return rows;
  });
  // Re-seed the cursor to the current model row whenever the panel opens.
  $effect(() => {
    if (settingsOpen) {
      const i = settingsRows.findIndex((r) => r.kind === "model" && r.model.id === assistant.model);
      settingsIdx = i >= 0 ? i : 0;
    }
  });
  function pickRow(row: SettingsRow) {
    if (row.kind === "model") pickModel(row.model);
    else if (row.kind === "effort") pickEffort(row.effort);
    else pickMode(row.mode);
  }

  function pickSlash(c: SlashCmd) {
    if (c.name === "model") {
      // Open the unified settings panel instead of inserting `/model ` text.
      setDraft("");
      stt.consume();
      settingsOpen = true;
      void tick().then(() => ta?.focus());
      return;
    }
    // Direct-fire commands skip the textarea round-trip entirely.
    setDraft("");
    stt.consume();
    onsubmit(`/${c.name}`);
    void tick().then(autosize);
  }

  function pickModel(m: ModelOpt) {
    assistant.setModel(m.id);
    settingsOpen = false;
    void tick().then(() => ta?.focus());
  }

  // Bumps on every fire() — drives the send-button ripple keyed off `{#key}`.
  // A pure-CSS one-shot, mounted by the key flip and self-removed after its
  // animation ends.
  let fireKey = $state(0);

  function fire() {
    const text = draft.trim();
    // Allow attachments-only sends (paste-and-go); only block if both empty.
    if (!text && attachments.length === 0) return;
    // Auth gate — the button's `disabled={!canFire}` covers clicks, but Enter
    // routes straight here, so without this a fresh/logged-out user can fire a
    // turn that's doomed to "claude exited with 1". Guard BEFORE clearing the
    // draft so their text survives; re-probe (state may be stale) and surface
    // the actionable reason via the notice banner.
    if (!(assistant.auth?.pill === "green" || assistant.auth?.pill === "yellow")) {
      assistant.lastNotice =
        assistant.auth?.summary ??
        "Claude isn't set up on this machine — open Settings to sign in or add an API key.";
      void assistant.refreshAuth();
      return;
    }
    setDraft("");
    stt.consume();
    fireKey++;
    onsubmit(text);
    void tick().then(autosize);
  }

  // ── Prompt enhancer (wand) ───────────────────────────────────────────────
  // One-shot Haiku rewrite of the current draft into a clearer prompt. Result
  // shows as an editable preview above the composer — Accept drops it into the
  // textarea, Discard dismisses. Never auto-sends, never overwrites silently.
  let enhancing = $state(false);
  let enhancedPreview = $state<string | null>(null);
  let enhanceError = $state<string | null>(null);
  // Split preserving whitespace so the reveal can stagger word-by-word while
  // keeping spacing/newlines intact. Each chunk gets its own materialize delay.
  const enhancedWords = $derived(
    enhancedPreview === null ? [] : enhancedPreview.split(/(\s+)/),
  );
  async function runEnhance() {
    const text = draft.trim();
    if (!text || enhancing) return;
    enhancing = true;
    enhancedPreview = null;
    enhanceError = null;
    try {
      // Stream: deltas fill the preview live (first text in ~1-2s); the
      // resolved value is the authoritative final text.
      enhancedPreview = await assistant.enhancePrompt(text, (full) => {
        enhancedPreview = full;
      });
    } catch (e) {
      enhanceError = String(e);
    } finally {
      enhancing = false;
    }
  }
  function acceptEnhanced() {
    if (enhancedPreview === null) return;
    setDraft(enhancedPreview);
    enhancedPreview = null;
    enhanceError = null;
    void tick().then(() => { autosize(); ta?.focus(); });
  }
  function dismissEnhanced() {
    enhancedPreview = null;
    enhanceError = null;
    void tick().then(() => ta?.focus());
  }

  // S88: mic toggle. The stt store writes recognized text directly into the
  // focused tab's draft (via `assistant.composerDraft` setter shim → activeTab.draft)
  // as it arrives (interim + final), so we just start/stop and let the
  // autosizer catch up. Composer focus is restored on stop so the user can
  // hit Enter without an extra click.
  let micBusy = $state(false);
  async function toggleMic() {
    if (micBusy) return;
    micBusy = true;
    try {
      void stt.init();
      if (stt.recording) {
        await stt.stop();
        void tick().then(() => { autosize(); ta?.focus(); });
      } else {
        await stt.start();
        void tick().then(() => ta?.focus());
      }
    } finally {
      micBusy = false;
    }
  }
  // Live autosize while transcription is streaming in.
  $effect(() => {
    if (stt.recording || stt.transcribing) {
      const _v = draft;
      void _v;
      void tick().then(autosize);
    }
  });

  // ── Image paste ─────────────────────────────────────────────────────────
  // Captures any image item on the clipboard when pasted into the textarea.
  // Reads as ArrayBuffer → base64 → stages on the assistant store for the
  // next send. Mixed paste (image + text) keeps the text in the textarea
  // and stages the image separately. Caps mirror backend's 20 MiB guard so
  // we reject early rather than round-trip a doomed payload.
  function bytesToBase64(buf: ArrayBuffer): string {
    const bytes = new Uint8Array(buf);
    let bin = "";
    const CHUNK = 0x8000;
    for (let i = 0; i < bytes.length; i += CHUNK) {
      bin += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
    }
    return btoa(bin);
  }

  // Portal action — moves the node to <body> so it escapes the composer's
  // overflow:hidden + backdrop-filter containing block (any ancestor with
  // backdrop-filter traps `position: fixed` descendants inside it, which is
  // exactly what we need to avoid here).
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy() { node.remove(); } };
  }

  // Hint popover — keyboard shortcuts. Portals to <body> + position: fixed
  // w/ JS-computed coords so it escapes the composer's clip + backdrop-filter
  // containing block.
  let hintOpen = $state(false);
  let hintWrap = $state<HTMLDivElement | null>(null);
  let hintPop = $state<HTMLDivElement | null>(null);
  let hintPos = $state<{ top: number; left: number }>({ top: 0, left: 0 });
  function positionHint() {
    if (!hintWrap || !hintPop) return;
    const a = hintWrap.getBoundingClientRect();
    const ph = hintPop.offsetHeight || 160;
    const pw = hintPop.offsetWidth || 240;
    // Prefer above the trigger (matches the old visual); flip down if no room.
    let top = a.top - ph - 8;
    if (top < 8) top = a.bottom + 8;
    let left = a.left;
    const maxLeft = window.innerWidth - pw - 8;
    if (left > maxLeft) left = maxLeft;
    if (left < 8) left = 8;
    hintPos = { top, left };
  }
  function onDocHintMousedown(ev: MouseEvent) {
    if (!hintOpen) return;
    if (hintWrap && ev.target instanceof Node && hintWrap.contains(ev.target)) return;
    if (hintPop && ev.target instanceof Node && hintPop.contains(ev.target)) return;
    hintOpen = false;
  }
  $effect(() => {
    window.addEventListener("mousedown", onDocHintMousedown);
    return () => window.removeEventListener("mousedown", onDocHintMousedown);
  });
  $effect(() => {
    if (!hintOpen) return;
    void tick().then(positionHint);
    const onResize = () => positionHint();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  });

  let attachError = $state<string | null>(null);
  async function onPaste(e: ClipboardEvent) {
    const items = e.clipboardData?.items;
    if (!items) return;
    const imageItems = Array.from(items).filter(
      (it) => it.kind === "file" && it.type.startsWith("image/"),
    );
    if (imageItems.length === 0) return;
    e.preventDefault();
    attachError = null;
    for (const it of imageItems) {
      const file = it.getAsFile();
      if (!file) continue;
      if (file.size > 20 * 1024 * 1024) {
        attachError = `Image too large: ${(file.size / 1024 / 1024).toFixed(1)} MB > 20 MB cap`;
        continue;
      }
      try {
        const buf = await file.arrayBuffer();
        const dataBase64 = bytesToBase64(buf);
        const ok = assistant.addAttachment({
          mime: file.type || "image/png",
          dataBase64,
          previewUrl: `data:${file.type || "image/png"};base64,${dataBase64}`,
          sizeBytes: file.size,
        }, tabId);
        if (!ok) {
          attachError = "Attachment limit reached (20 MB total per turn).";
        }
      } catch (err) {
        attachError = `Failed to read pasted image: ${String(err)}`;
      }
    }
  }

  function fmtSize(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
    return `${(n / 1024 / 1024).toFixed(1)} MB`;
  }

  // Up-arrow recall offset (0 = newest). Reset whenever the user types or
  // switches focus so the next Up starts at the newest prompt again.
  let recallOffset = $state(-1);
  function resetRecall() { recallOffset = -1; }

  function onKey(e: KeyboardEvent) {
    // Enhance preview claims Escape first — dismiss it before menu handlers.
    if ((enhancedPreview !== null || enhanceError !== null) && e.key === "Escape") {
      e.preventDefault();
      dismissEnhanced();
      return;
    }
    // Mention picker keys take precedence — runs before history recall so
    // arrow keys navigate the list, not the prompt history.
    if (mentionState && mentionResults.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        mentionIdx = (mentionIdx + 1) % mentionResults.length;
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        mentionIdx = (mentionIdx - 1 + mentionResults.length) % mentionResults.length;
        return;
      }
      if (e.key === "Tab" || e.key === "Enter") {
        e.preventDefault();
        pickMention(mentionResults[mentionIdx]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        mentionState = null;
        return;
      }
    }
    // History recall — only when no menu is open and the textarea is
    // either empty, or cursor is at position 0 (so plain Up in a multiline
    // draft still moves the caret normally).
    const empty = draft.length === 0;
    const atStart = ta?.selectionStart === 0 && ta?.selectionEnd === 0;
    if (!settingsOpen && !slashOpen && !mentionState && (empty || atStart)) {
      if (e.key === "ArrowUp") {
        // Recall from THIS tab's promptHistory, not the focused tab's, so
        // each pane has its own prompt-history scroll.
        const hist = tab?.promptHistory ?? [];
        const idx = hist.length - 1 - (recallOffset + 1);
        const next = idx >= 0 ? hist[idx] : null;
        if (next !== null) {
          e.preventDefault();
          recallOffset += 1;
          setDraft(next);
          void tick().then(() => {
            autosize();
            if (ta) ta.setSelectionRange(next.length, next.length);
          });
          return;
        }
      } else if (e.key === "ArrowDown" && recallOffset >= 0) {
        e.preventDefault();
        recallOffset -= 1;
        const hist = tab?.promptHistory ?? [];
        const idx = recallOffset < 0 ? -1 : hist.length - 1 - recallOffset;
        const prev = idx >= 0 ? hist[idx] : "";
        setDraft(prev ?? "");
        void tick().then(() => {
          autosize();
          if (ta) ta.setSelectionRange((prev ?? "").length, (prev ?? "").length);
        });
        return;
      }
    }
    if (settingsOpen) {
      const n = settingsRows.length;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        settingsIdx = (settingsIdx + 1) % n;
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        settingsIdx = (settingsIdx - 1 + n) % n;
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        pickRow(settingsRows[settingsIdx]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        settingsOpen = false;
        return;
      }
      // Any other key cancels the panel so the user can type normally.
      if (e.key.length === 1) settingsOpen = false;
    }
    if (slashOpen && slashFiltered.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        slashIdx = (slashIdx + 1) % slashFiltered.length;
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        slashIdx = (slashIdx - 1 + slashFiltered.length) % slashFiltered.length;
        return;
      }
      if (e.key === "Tab") {
        e.preventDefault();
        pickSlash(slashFiltered[slashIdx]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        setDraft("");
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (slashOpen && slashFiltered.length >= 1) {
        pickSlash(slashFiltered[slashIdx]);
        return;
      }
      fire();
    }
  }

  function onBtnClick() {
    if (streaming && draft.trim().length === 0) {
      void assistant.stop(tabId);
      return;
    }
    fire();
  }

  // Three modes for the action button:
  //   idle + draft           → Send
  //   streaming + empty      → Stop (kill the running turn)
  //   streaming + draft      → Queue (append to message queue)
  const mode = $derived.by<"send" | "stop" | "queue">(() => {
    const hasDraft = draft.trim().length > 0;
    if (streaming && !hasDraft) return "stop";
    if (streaming && hasDraft) return "queue";
    return "send";
  });
  const canFire = $derived(
    mode === "stop" ||
      ((draft.trim().length > 0 || attachments.length > 0) &&
        (assistant.auth?.pill === "green" || assistant.auth?.pill === "yellow")),
  );

  // ── Drag-over highlight ────────────────────────────────────────────────
  // Files dragged anywhere over the composer shell flip a glow + dashed
  // border w/ "Drop image to attach" overlay. We only react to actual file
  // drags (dataTransfer.types contains "Files") — internal text/HTML drags
  // shouldn't trip the visual. Counter-based to survive enter/leave on
  // descendants without flicker.
  let dragDepth = $state(0);
  const dragOver = $derived(dragDepth > 0);
  function isFileDrag(e: DragEvent): boolean {
    const types = e.dataTransfer?.types;
    if (!types) return false;
    for (let i = 0; i < types.length; i++) if (types[i] === "Files") return true;
    return false;
  }
  function onDragEnter(e: DragEvent) {
    if (!isFileDrag(e)) return;
    e.preventDefault();
    dragDepth += 1;
  }
  function onDragLeave(e: DragEvent) {
    if (!isFileDrag(e)) return;
    e.preventDefault();
    dragDepth = Math.max(0, dragDepth - 1);
  }
  function onDragOverShell(e: DragEvent) {
    if (!isFileDrag(e)) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
  }
  async function onDrop(e: DragEvent) {
    if (!isFileDrag(e)) return;
    e.preventDefault();
    dragDepth = 0;
    const files = e.dataTransfer?.files;
    if (!files || files.length === 0) return;
    attachError = null;
    for (const file of Array.from(files)) {
      if (!file.type.startsWith("image/")) continue;
      if (file.size > 20 * 1024 * 1024) {
        attachError = `Image too large: ${(file.size / 1024 / 1024).toFixed(1)} MB > 20 MB cap`;
        continue;
      }
      try {
        const buf = await file.arrayBuffer();
        const dataBase64 = bytesToBase64(buf);
        const ok = assistant.addAttachment({
          mime: file.type || "image/png",
          dataBase64,
          previewUrl: `data:${file.type || "image/png"};base64,${dataBase64}`,
          sizeBytes: file.size,
        }, tabId);
        if (!ok) attachError = "Attachment limit reached (20 MB total per turn).";
      } catch (err) {
        attachError = `Failed to read dropped image: ${String(err)}`;
      }
    }
  }

  // ── Click-to-attach ───────────────────────────────────────────────────────
  // Paste + drag-drop already stage images; this adds the discoverable path
  // the placeholder has long advertised. Same staging + 20 MiB guard as onDrop.
  let fileInput = $state<HTMLInputElement | undefined>();
  function openFilePicker() { fileInput?.click(); }
  async function onFilePick(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = input.files;
    if (!files || files.length === 0) return;
    attachError = null;
    for (const file of Array.from(files)) {
      if (!file.type.startsWith("image/")) continue;
      if (file.size > 20 * 1024 * 1024) {
        attachError = `Image too large: ${(file.size / 1024 / 1024).toFixed(1)} MB > 20 MB cap`;
        continue;
      }
      try {
        const buf = await file.arrayBuffer();
        const dataBase64 = bytesToBase64(buf);
        const ok = assistant.addAttachment({
          mime: file.type || "image/png",
          dataBase64,
          previewUrl: `data:${file.type || "image/png"};base64,${dataBase64}`,
          sizeBytes: file.size,
        }, tabId);
        if (!ok) attachError = "Attachment limit reached (20 MB total per turn).";
      } catch (err) {
        attachError = `Failed to read image: ${String(err)}`;
      }
    }
    input.value = ""; // allow re-picking the same file
  }
</script>

<div class="composer-wrap" data-model={modelFamily(assistant.model)}>
  {#if queue.length > 0}
    <div class="queue">
      <span class="queue-label">Queued ({queue.length}):</span>
      {#each queue as q (q.id)}
        <span class="qpill" use:tooltip={q.text}>
          <span class="qtext">{q.text}</span>
          <button
            class="qx"
            type="button"
            onclick={() => { if (tab) tab.queue = tab.queue.filter((it) => it.id !== q.id); }}
            aria-label="Remove"
          >
            <X size={11} />
          </button>
        </span>
      {/each}
      {#if queue.length >= 2}
        <button
          class="qclear"
          type="button"
          onclick={() => { if (tab) tab.queue = []; }}
        >
          Clear all
        </button>
      {/if}
    </div>
  {/if}

  {#if attachments.length > 0 || attachError}
    <div class="attachments">
      {#each attachments as a (a.id)}
        <div class="attach-chip" use:tooltip={`${a.mime} · ${fmtSize(a.sizeBytes)}`}>
          <img class="attach-thumb" src={a.previewUrl} alt="pasted attachment" />
          <span class="attach-meta">
            <span class="attach-name">image</span>
            <span class="attach-size">{fmtSize(a.sizeBytes)}</span>
          </span>
          <button
            class="attach-x"
            type="button"
            onclick={() => assistant.removeAttachment(a.id, tabId)}
            aria-label="Remove attachment"
          >
            <X size={11} />
          </button>
        </div>
      {/each}
      {#if attachError}
        <div class="attach-error" role="alert">
          {attachError}
          <button class="attach-error-x" type="button" onclick={() => (attachError = null)} aria-label="Dismiss">
            <X size={10} />
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <div
    class="composer-shell"
    class:drag-over={dragOver}
    role="region"
    aria-label="Message composer"
    ondragenter={onDragEnter}
    ondragleave={onDragLeave}
    ondragover={onDragOverShell}
    ondrop={onDrop}
  >
    {#if dragOver}
      <div class="drop-overlay" aria-hidden="true">
        <div class="drop-pill">
          <span class="drop-dot" aria-hidden="true"></span>
          Drop image to attach
        </div>
      </div>
    {/if}
    {#if enhancedPreview !== null || enhanceError !== null}
      <div class="enhance-panel" role="region" aria-label="Enhanced prompt">
        {#if enhancedPreview !== null}
          <div class="enhance-head">
            <Wand2 size={13} />
            <span class="enhance-title">Enhanced prompt</span>
            <span class="enhance-sub">review before sending</span>
          </div>
          <div class="enhance-text">
            {#each enhancedWords as w, i (i)}<span class="ew" class:live={enhancing} style="--i:{i}">{w}</span>{/each}
          </div>
          <div class="enhance-actions">
            <button type="button" class="enhance-btn enhance-accept" onclick={acceptEnhanced}>
              <Check size={13} /> Use this
            </button>
            <button type="button" class="enhance-btn enhance-discard" onclick={dismissEnhanced}>
              Discard
            </button>
            <span class="enhance-kbd">Esc to dismiss</span>
          </div>
        {:else if enhanceError !== null}
          <div class="enhance-error" role="alert">
            <span class="enhance-error-msg">{enhanceError}</span>
            <button type="button" class="attach-error-x" onclick={dismissEnhanced} aria-label="Dismiss">
              <X size={11} />
            </button>
          </div>
        {/if}
      </div>
    {/if}

    {#if slashOpen && slashFiltered.length > 0}
      <div class="slash-menu" role="listbox">
        {#each slashFiltered as c, i (c.name)}
          <button
            type="button"
            class="slash-item"
            class:active={i === slashIdx}
            style="--idx: {i}"
            onmousedown={(e) => { e.preventDefault(); pickSlash(c); }}
          >
            <span class="slash-name">/{c.name}</span>
            <span class="slash-desc">{c.desc}</span>
          </button>
        {/each}
        <div class="slash-hint">↑↓ select · Tab/Enter pick · Esc cancel</div>
      </div>
    {/if}

    {#if mentionState && mentionResults.length > 0}
      <div class="slash-menu mention-menu" role="listbox">
        {#each mentionResults as path, i (path)}
          {@const slash = path.lastIndexOf("/")}
          {@const dir = slash > 0 ? path.slice(0, slash + 1) : ""}
          {@const base = slash >= 0 ? path.slice(slash + 1) : path}
          <button
            type="button"
            class="slash-item mention-item"
            class:active={i === mentionIdx}
            style="--idx: {i}"
            onmousedown={(e) => { e.preventDefault(); pickMention(path); }}
          >
            <span class="mention-base">{base}</span>
            <span class="mention-dir">{dir}</span>
          </button>
        {/each}
        <div class="slash-hint">
          {assistant.workspaceFiles.length > 0
            ? `${assistant.workspaceFiles.length} files · ↑↓ select · Tab/Enter pick · Esc cancel`
            : "loading workspace files…"}
        </div>
      </div>
    {/if}

    {#if settingsOpen}
      <div class="slash-menu model-menu settings-menu" role="listbox">
        <div class="settings-section">
          <div class="model-header"><span>Model</span></div>
          <div class="settings-grid">
            {#each MODEL_OPTIONS as m (m.id)}
              {@const idx = settingsRows.findIndex((r) => r.kind === "model" && r.model.id === m.id)}
              <button
                type="button"
                class="settings-item model-card"
                class:active={idx === settingsIdx}
                class:current={m.id === assistant.model}
                data-id={m.id}
                style="--idx: {idx}"
                use:tooltip={m.tagline}
                onmousedown={(e) => { e.preventDefault(); pickModel(m); }}
              >
                <span class="card-head">
                  <span class="model-dot" aria-hidden="true"></span>
                  <span class="model-label">{m.label}</span>
                  <span class="model-version">{m.version}</span>
                </span>
                <span class="model-ctx" class:wide={m.ctx === "1M ctx"}>{m.ctx}</span>
              </button>
            {/each}
          </div>
        </div>
        <div class="settings-cols">
          {#if assistant.model !== "haiku"}
            <div class="settings-section">
              <div class="model-header"><span>Thinking depth</span></div>
              <div class="settings-stack">
                {#each EFFORT_OPTIONS as e (e.id)}
                  {@const idx = settingsRows.findIndex((r) => r.kind === "effort" && r.effort.id === e.id)}
                  <button
                    type="button"
                    class="settings-item effort-row"
                    class:active={idx === settingsIdx}
                    class:current={e.id === assistant.thinkingEffort}
                    class:ultra={e.id === "ultra"}
                    data-level={e.level}
                    style="--idx: {idx}"
                    use:tooltip={e.hint}
                    onmousedown={(ev) => { ev.preventDefault(); pickEffort(e); }}
                  >
                    {#if e.id === "ultra"}
                      <span class="effort-ultra-icon" aria-hidden="true"><Sparkles size={13} /></span>
                    {:else}
                      <span class="effort-bars" aria-hidden="true" data-level={e.level}>
                        <span class="bar"></span>
                        <span class="bar"></span>
                        <span class="bar"></span>
                      </span>
                    {/if}
                    <span class="model-label">{e.label}</span>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
          <div class="settings-section">
            <div class="model-header"><span>Permission mode</span></div>
            <div class="settings-stack">
              {#each MODE_OPTIONS as m (m.id)}
                {@const idx = settingsRows.findIndex((r) => r.kind === "mode" && r.mode.id === m.id)}
                {@const Icon = m.icon}
                <button
                  type="button"
                  class="settings-item mode-row"
                  class:active={idx === settingsIdx}
                  class:current={m.id === assistant.permissionMode}
                  data-id={m.id}
                  style="--idx: {idx}"
                  use:tooltip={m.hint}
                  onmousedown={(ev) => { ev.preventDefault(); pickMode(m); }}
                >
                  <span class="mode-icon" aria-hidden="true"><Icon size={15} /></span>
                  <span class="model-label">{m.label}</span>
                </button>
              {/each}
            </div>
          </div>
        </div>
        <div class="slash-hint model-hint">
          <span><kbd>↑↓</kbd> navigate</span>
          <span><kbd>↵</kbd> pick</span>
          <span><kbd>Esc</kbd> close</span>
        </div>
      </div>
    {/if}

    <div class="composer" class:streaming={streaming} class:enchanting={enhancing} data-mode={mode}>
      <div class="textarea-wrap">
        <textarea
          bind:this={ta}
          value={draft}
          oninput={(e) => {
            setDraft((e.currentTarget as HTMLTextAreaElement).value);
            resetRecall(); autosize(); refreshMention();
          }}
          onkeyup={refreshMention}
          onclick={refreshMention}
          onfocus={() => { composerFocused = true; }}
          onblur={() => {
            composerFocused = false;
            if (!mentionState) return;
            requestAnimationFrame(() => { mentionState = null; });
          }}
          onkeydown={onKey}
          onpaste={onPaste}
          placeholder=""
          rows="1"
        ></textarea>
        {#if draft.length === 0 && !streaming && attachments.length === 0}
          {#key placeholderKey}
            <span class="placeholder-ghost" aria-hidden="true">{idlePlaceholder}</span>
          {/key}
        {:else if streaming && draft.length === 0}
          <span class="placeholder-ghost static" aria-hidden="true">Type to queue — Enter sends, /stop halts</span>
        {:else if attachments.length > 0 && draft.length === 0}
          <span class="placeholder-ghost static" aria-hidden="true">Ask about the image…</span>
        {/if}
        {#if enhancing}
          <span class="magic-aura" aria-hidden="true"></span>
          <div class="magic-text" aria-hidden="true">{draft}</div>
          <span class="magic-stars" aria-hidden="true">
            <i style="--sx:7%;  --sy:30%; --sz:9px; --sd:0s"></i>
            <i style="--sx:24%; --sy:60%; --sz:6px; --sd:.5s"></i>
            <i style="--sx:40%; --sy:32%; --sz:7px; --sd:.85s"></i>
            <i style="--sx:15%; --sy:74%; --sz:5px; --sd:.3s"></i>
          </span>
        {/if}
      </div>

      <!-- Divider + context gauge in one. Base hairline separates the input
           zone from the toolbar; the fill tracks context-window usage, and the
           trailing readout puts the live ctx% next to the bar (only once the
           conversation has tokens, so a fresh composer stays clean). -->
      <div class="composer-gauge">
        <div class="composer-divider" data-tone={ctxTone} use:tooltip={ctxTitle} role="img" aria-label={ctxTitle}>
          {#if ctxTokens > 0}
            <span class="composer-divider-fill" style="width: {Math.min(100, ctxPct)}%" aria-hidden="true"></span>
          {/if}
        </div>
        {#if ctxTokens > 0}
          <span class="ctx-readout" data-tone={ctxTone} aria-hidden="true">{ctxPct < 1 ? "<1" : Math.round(ctxPct)}%</span>
        {/if}
      </div>

      <div class="composer-toolbar">
        <div class="toolbar-cluster">
          <input
            bind:this={fileInput}
            type="file"
            accept="image/*"
            multiple
            class="file-input-hidden"
            onchange={onFilePick}
            tabindex="-1"
            aria-hidden="true"
          />
          <button
            type="button"
            class="iconbtn attachbtn"
            onclick={openFilePicker}
            use:tooltip={"Attach image — or paste / drag-drop"}
            aria-label="Attach image"
          >
            <Paperclip size={14} />
          </button>
          {#if stt.config.enabled && (
            (stt.config.engine === "web_speech" && stt.supported) ||
            (stt.config.engine === "whisper" && stt.backendAvailable)
          )}
          <button
            class="iconbtn micbtn"
            class:recording={stt.recording}
            class:transcribing={stt.transcribing}
            type="button"
            onclick={toggleMic}
            disabled={micBusy || stt.transcribing}
            use:tooltip={
              stt.recording ? "Stop recording" :
              stt.transcribing ? "Transcribing…" :
              stt.config.engine === "whisper" ? "Dictate (Whisper, local)" : "Dictate (Web Speech)"
            }
            aria-label={stt.recording ? "Stop recording" : "Start recording"}
          >
            {#if stt.transcribing}
              <Loader2 size={14} class="mic-spin" />
            {:else if stt.recording}
              <span class="mic-wave" aria-hidden="true">
                <span></span><span></span><span></span>
              </span>
            {:else}
              <Mic size={14} />
            {/if}
          </button>
          {/if}
          <div class="hint-wrap" bind:this={hintWrap}>
            <button
              type="button"
              class="iconbtn hintbtn"
              onclick={() => (hintOpen = !hintOpen)}
              aria-expanded={hintOpen}
              aria-label="Composer hints"
              aria-describedby={hintOpen ? "composer-hint-pop" : undefined}
              use:tooltip={"Keyboard shortcuts"}
            >
              <HelpCircle size={14} />
            </button>
            {#if hintOpen}
              <div
                id="composer-hint-pop"
                class="hint-pop"
                role="tooltip"
                bind:this={hintPop}
                use:portal
                style="top: {hintPos.top}px; left: {hintPos.left}px;"
              >
                <div class="hint-head">Keyboard shortcuts</div>
                <div class="hint-row"><span class="hint-keys"><kbd>Enter</kbd></span><span>Send message</span></div>
                <div class="hint-row"><span class="hint-keys"><kbd>Shift</kbd><kbd>Enter</kbd></span><span>New line</span></div>
                <div class="hint-row"><span class="hint-keys"><kbd>/</kbd></span><span>Slash command menu</span></div>
                <div class="hint-row"><span class="hint-keys"><kbd>@</kbd></span><span>Mention a file</span></div>
                <div class="hint-row"><span class="hint-keys"><kbd>↑</kbd></span><span>Recall previous prompt</span></div>
                <div class="hint-row"><span class="hint-keys"><kbd>Esc</kbd></span><span>Close any open menu</span></div>
              </div>
            {/if}
          </div>
          {#if draft.trim().length > 0 && !streaming}
            <button
              class="iconbtn wandbtn"
              class:enhancing
              type="button"
              onclick={runEnhance}
              disabled={enhancing}
              use:tooltip={enhancing ? "Enhancing…" : "Enhance prompt — clean up & clarify"}
              aria-label="Enhance prompt"
            >
              <Wand2 size={14} />
            </button>
          {/if}
          {#if draft.length > 500}
            <span
              class="char-count"
              class:warn={draft.length > 4000}
              use:tooltip={draft.length > 4000 ? `${draft.length.toLocaleString()} characters · long prompts may slow first reply` : `${draft.length.toLocaleString()} characters in this draft`}
            >
              {draft.length.toLocaleString()}
            </span>
          {/if}
        </div>

        {#if showLivePills}
          <div class="live-pills" role="group" aria-label="Live turn activity">
            {#if turnElapsed}
              <button
                type="button"
                class="live-pill turn"
                onclick={openActivity}
                use:tooltip={"Current turn — elapsed · output speed. Click to open Activity."}
              >
                <span class="lp-dot" aria-hidden="true"></span>
                <span class="mono">{turnElapsed}</span>
                {#if tokPerSec != null}
                  <span class="lp-sep" aria-hidden="true">·</span>
                  <span class="mono">{tokPerSec}<span class="lp-unit"> tok/s</span></span>
                {/if}
              </button>
            {/if}
            {#if agentCount > 0}
              <button
                type="button"
                class="live-pill"
                onclick={openActivity}
                use:tooltip={`${agentCount} sub-agent${agentCount === 1 ? "" : "s"} running. Click to open Activity.`}
              >
                <Bot size={12} />
                <span class="mono">{agentCount}</span>
              </button>
            {/if}
            {#if shellCount > 0}
              <button
                type="button"
                class="live-pill"
                onclick={openActivity}
                use:tooltip={`${shellCount} shell${shellCount === 1 ? "" : "s"} running. Click to open Activity.`}
              >
                <Terminal size={12} />
                <span class="mono">{shellCount}</span>
              </button>
            {/if}
            {#if queue.length > 0}
              <button
                type="button"
                class="live-pill queued"
                onclick={openActivity}
                use:tooltip={`${queue.length} message${queue.length === 1 ? "" : "s"} queued to send after this turn.`}
              >
                <ListPlus size={12} />
                <span class="mono">{queue.length}</span>
              </button>
            {/if}
          </div>
        {/if}

        <div class="toolbar-cluster toolbar-right">
          <button
            type="button"
            class="settings-pill"
            class:open={settingsOpen}
            class:ultra={assistant.thinkingEffort === "ultra"}
            data-mode={currentMode.id}
            onclick={() => { settingsOpen = !settingsOpen; void tick().then(() => ta?.focus()); }}
            aria-haspopup="listbox"
            aria-expanded={settingsOpen}
            aria-label="Model, thinking depth & permission mode"
            use:tooltip={`Model · thinking depth · permission mode\n${currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.model} · ${currentEffort.label} · ${currentMode.label}`}
          >
            <SlidersHorizontal size={15} />
            <span class="pill-label">{currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.model}</span>
            {#if assistant.thinkingEffort === "ultra"}
              <span class="pill-ultra" aria-hidden="true" use:tooltip={"Ultracode — max reasoning + autonomous workflows"}><Sparkles size={11} /></span>
            {/if}
            <span class="mode-dot" aria-hidden="true"></span>
            <span class="pill-caret" aria-hidden="true">▾</span>
          </button>
          <button
            class="sendbtn"
            class:stop={mode === "stop"}
            class:queue={mode === "queue"}
            type="button"
            onclick={onBtnClick}
            disabled={!canFire}
            use:tooltip={mode === "stop"
              ? { text: "Halt the current turn", kbd: "Esc" }
              : mode === "queue"
              ? { text: "Queue after current turn", kbd: "Enter" }
              : { text: "Send", kbd: "Enter" }}
          >
            <span class="icon-stack">
              <span class="icon-slot" class:active={mode === "send" || mode === "queue"}><Send size={14} /></span>
              <span class="icon-slot" class:active={mode === "stop"}><Square size={12} fill="currentColor" /></span>
            </span>
            {#key fireKey}
              {#if fireKey > 0}
                <span class="send-ripple" aria-hidden="true"></span>
                <span class="send-ripple send-ripple-2" aria-hidden="true"></span>
              {/if}
            {/key}
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .composer-wrap {
    position: relative;
    padding: 10px 18px 14px;
    max-width: var(--chat-col-max);
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }
  /* Aurora hue follows the active model — sonnet=blue, opus=purple,
     haiku=teal. Resolved here so every accent inside (border, ripple,
     streaming ring, model-pill pulse) reads from the same single source. */
  .composer-wrap[data-model="sonnet"] { --model-color: oklch(0.74 0.13 230); }
  .composer-wrap[data-model="opus"]   { --model-color: oklch(0.70 0.18 295); }
  .composer-wrap[data-model="haiku"]  { --model-color: oklch(0.78 0.14 180); }
  .composer-wrap                      { --model-color: var(--accent); }
  .composer-shell { position: relative; }
  .composer-shell.drag-over .composer {
    border-color: color-mix(in oklch, var(--model-color) 70%, transparent);
    border-style: dashed;
    box-shadow:
      0 0 0 4px color-mix(in oklch, var(--model-color) 22%, transparent),
      0 12px 36px -8px color-mix(in oklch, var(--model-color) 45%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
    transform: translateY(-1px);
  }
  .drop-overlay {
    position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
    z-index: 8;
    pointer-events: none;
    animation: drop-in 140ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .drop-pill {
    display: inline-flex; align-items: center; gap: 8px;
    padding: 8px 16px;
    background: color-mix(in oklch, var(--bg-elev-1) 92%, transparent);
    border: 1px solid color-mix(in oklch, var(--model-color) 50%, transparent);
    border-radius: 999px;
    color: var(--fg);
    font-size: var(--fs-sm);
    font-weight: 600;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    box-shadow: 0 8px 24px -6px color-mix(in oklch, var(--model-color) 40%, transparent);
  }
  .drop-dot {
    width: 8px; height: 8px;
    border-radius: 999px;
    background: var(--model-color);
    box-shadow: 0 0 8px color-mix(in oklch, var(--model-color) 70%, transparent);
    animation: drop-dot-pulse 1.2s ease-in-out infinite;
  }
  @keyframes drop-in {
    from { opacity: 0; transform: scale(0.94); }
    to   { opacity: 1; transform: scale(1); }
  }
  @keyframes drop-dot-pulse {
    0%, 100% { transform: scale(1); opacity: 1; }
    50%      { transform: scale(1.3); opacity: 0.7; }
  }

  /* ── Composer v3 ─────────────────────────────────────────────────────
     Two-row layout: textarea up top, toolbar below.  Glass-blur surface
     w/ soft accent focus ring + animated streaming edge.  All controls
     unified under .iconbtn (mic/help) + .settings-pill (model/effort/mode) +
     .sendbtn.  Replaces the v2 single-row design. */
  .composer {
    position: relative;
    display: flex; flex-direction: column;
    padding: 6px;
    background: color-mix(in oklch, var(--surface) 88%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid color-mix(in oklch, var(--border) 90%, transparent);
    border-radius: 18px;
    box-shadow:
      0 10px 28px -10px oklch(0 0 0 / 0.45),
      inset 0 1px 0 color-mix(in oklch, white 4%, transparent);
    transition: border-color 220ms cubic-bezier(0.22, 1, 0.36, 1),
                box-shadow 220ms cubic-bezier(0.22, 1, 0.36, 1),
                transform 140ms ease-out;
    overflow: hidden;
  }
  /* Calm focus ring — a tight 2px tint + a soft, low halo. The old glow
     (3px ring + 32% halo) stacked with the streaming treatment into a busy
     purple blob; this keeps focus legible without competing. */
  .composer:focus-within {
    border-color: color-mix(in oklch, var(--model-color) 45%, transparent);
    box-shadow:
      0 0 0 2px color-mix(in oklch, var(--model-color) 13%, transparent),
      0 8px 22px -12px color-mix(in oklch, var(--model-color) 20%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
  }

  /* Streaming = ONE coherent signal: a thin model-tinted border + the
     animated top-edge bar below (synced 2.6s with the model-pill breathe).
     No aurora swirl, no extra glow — calm and in-sync. */
  .composer.streaming {
    border-color: color-mix(in oklch, var(--model-color) 42%, var(--border));
  }
  /* Full-frame streaming ring — a model-tinted border that breathes around
     the entire composer (synced 2.6s with the model-pill breathe). Sits as a
     border-only overlay (transparent center, pointer-events none) so it traces
     the frame without crossing the input text. */
  .composer.streaming::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 18px;
    border: 1.5px solid color-mix(in oklch, var(--model-color) 65%, transparent);
    box-shadow:
      0 0 12px color-mix(in oklch, var(--model-color) 32%, transparent),
      inset 0 0 8px color-mix(in oklch, var(--model-color) 16%, transparent);
    pointer-events: none;
    animation: composer-stream 2.6s ease-in-out infinite;
    z-index: 2;
  }
  @keyframes composer-stream {
    0%, 100% { opacity: 0.35; }
    50%      { opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .composer.streaming::before { animation: none; opacity: 0.7; }
  }

  .textarea-wrap {
    position: relative;
    z-index: 1;
    display: flex;
  }
  textarea {
    position: relative;
    z-index: 1;
    flex: 1;
    resize: none;
    width: 100%;
    min-height: 28px; max-height: 220px;
    padding: 8px 10px 6px;
    background: transparent;
    border: 0; outline: none;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-md);
    line-height: 1.5;
    overflow-y: auto;
  }
  textarea::placeholder {
    color: transparent;
  }
  /* Custom ghost placeholder — sits over the textarea, fades+rotates so the
     idle composer feels alive but not noisy. {#key placeholderKey} retriggers
     `placeholder-fade` on each cycle. `.static` skips the animation for
     non-rotating contexts (streaming / attachment hints). */
  .placeholder-ghost {
    position: absolute;
    top: 8px; left: 10px; right: 10px;
    pointer-events: none;
    font-size: var(--fs-md);
    line-height: 1.5;
    color: var(--fg-subtle);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    animation: placeholder-fade 700ms cubic-bezier(0.22, 1, 0.36, 1) both;
    z-index: 0;
  }
  .placeholder-ghost.static { animation: none; }
  .composer:focus-within .placeholder-ghost { color: var(--fg-faint); }
  @keyframes placeholder-fade {
    from { opacity: 0; transform: translateY(8px); filter: blur(3px); }
    to   { opacity: 1; transform: translateY(0);   filter: blur(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .placeholder-ghost { animation: none; }
  }

  /* Divider between the input zone and the toolbar — doubles as a context
     gauge. The base hairline always shows (structure); the fill sweeps across
     proportional to context-window usage, tinted by the active model and
     stepping to warn/danger as the window fills. One element, two jobs. */
  /* Gauge row: the hairline track takes all remaining width, the ctx% readout
     sits flush to its right. Margin lives here now (was on the divider). */
  .composer-gauge {
    display: flex; align-items: center; gap: 8px;
    margin: 5px 8px 3px;
  }
  .composer-divider {
    position: relative;
    flex: 1;
    height: 1.5px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--border) 55%, transparent);
    overflow: hidden;
  }
  /* Trailing ctx% — tabular so it doesn't jitter as the number ticks; tone
     steps mirror the fill (yellow ≥70, red ≥90). */
  .ctx-readout {
    flex-shrink: 0;
    font-size: 9px; font-weight: 600; line-height: 1;
    letter-spacing: 0.02em;
    font-variant-numeric: tabular-nums;
    color: var(--fg-faint);
    transition: color 240ms ease-out;
  }
  .ctx-readout[data-tone="yellow"] { color: var(--warn); }
  .ctx-readout[data-tone="red"] { color: var(--danger); }
  .composer-divider-fill {
    position: absolute;
    inset: 0 auto 0 0;
    height: 100%;
    border-radius: 999px;
    background: color-mix(in oklch, var(--model-color) 75%, transparent);
    box-shadow: 0 0 6px color-mix(in oklch, var(--model-color) 45%, transparent);
    transition: width 360ms cubic-bezier(0.22, 1, 0.36, 1), background 240ms ease-out;
  }
  .composer-divider[data-tone="yellow"] .composer-divider-fill {
    background: var(--warn);
    box-shadow: 0 0 6px color-mix(in oklch, var(--warn) 50%, transparent);
  }
  .composer-divider[data-tone="red"] .composer-divider-fill {
    background: var(--danger);
    box-shadow: 0 0 8px color-mix(in oklch, var(--danger) 55%, transparent);
    animation: ctx-pulse 1.6s ease-in-out infinite;
  }
  @keyframes ctx-pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.6; }
  }
  @media (prefers-reduced-motion: reduce) {
    .composer-divider-fill { animation: none; transition: none; }
  }

  /* Hidden native file input — driven by the paperclip .attachbtn. */
  .file-input-hidden {
    position: absolute;
    width: 1px; height: 1px;
    padding: 0; margin: -1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
  }

  /* Toolbar row — left cluster (input affordances) + right cluster (action). */
  .composer-toolbar {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 4px 2px 4px;
  }
  .toolbar-cluster {
    display: flex; align-items: center; gap: 4px;
  }
  .toolbar-right { gap: 6px; }

  /* Unified icon button — base for mic + help + (future attach). */
  .iconbtn {
    width: 28px; height: 28px;
    display: inline-flex; align-items: center; justify-content: center;
    background: transparent;
    color: var(--fg-faint);
    border: 1px solid transparent;
    border-radius: 8px;
    cursor: pointer;
    flex-shrink: 0;
    opacity: 0.85;
    padding: 0;
    transition: color 140ms, background 140ms, border-color 140ms, opacity 140ms, transform 140ms;
  }
  .iconbtn:hover:not(:disabled) {
    color: var(--fg-2);
    background: color-mix(in oklch, var(--surface-hover) 80%, transparent);
    opacity: 1;
  }
  .iconbtn:active:not(:disabled) { transform: scale(0.94); }
  .iconbtn:disabled { opacity: 0.4; cursor: default; }
  .iconbtn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 2px var(--ring);
  }

  /* Mic — recording / transcribing states inherit .iconbtn base + override. */
  .micbtn.recording {
    background: var(--danger);
    color: oklch(0.98 0.01 22);
    border-color: var(--danger);
    opacity: 1;
    animation: mic-pulse 1.1s ease-in-out infinite;
  }
  .micbtn.transcribing {
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
    opacity: 1;
  }
  :global(.mic-spin) { animation: mic-spin 0.9s linear infinite; }
  @keyframes mic-spin { to { transform: rotate(360deg); } }
  @keyframes mic-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklch, var(--danger) 45%, transparent); }
    50%      { box-shadow: 0 0 0 6px transparent; }
  }
  /* Live recording waveform — 3 bars w/ staggered scaleY pulses. Pure CSS;
     no audio analyser needed for the visual cue. The .recording state on
     .micbtn paints the bg red and forces white bars via currentColor. */
  .mic-wave {
    display: inline-flex; align-items: center; gap: 2px;
    height: 12px;
  }
  .mic-wave span {
    width: 2.5px;
    height: 100%;
    background: currentColor;
    border-radius: 999px;
    transform-origin: center;
    animation: mic-bar 0.9s ease-in-out infinite;
  }
  .mic-wave span:nth-child(1) { animation-delay: 0s; }
  .mic-wave span:nth-child(2) { animation-delay: 0.15s; }
  .mic-wave span:nth-child(3) { animation-delay: 0.3s; }
  @keyframes mic-bar {
    0%, 100% { transform: scaleY(0.35); }
    50%      { transform: scaleY(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .mic-wave span { animation: none; transform: scaleY(0.7); }
  }

  /* Live character count — stays hidden until the draft passes 500 chars, so
     it reads as a "getting long" signal rather than constant clutter. Warns
     past 4000 chars (rough one-turn ceiling for short prompts). */
  .char-count {
    margin-left: 4px;
    padding: 2px 8px;
    font-size: 10px;
    font-weight: 600;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    color: var(--fg-faint);
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
    border-radius: 999px;
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    animation: enter 160ms ease-out;
  }
  .char-count.warn {
    color: var(--warn);
    border-color: color-mix(in oklch, var(--warn) 35%, var(--border));
    background: color-mix(in oklch, var(--warn) 10%, transparent);
  }

  /* Live turn pills — additive readout that only mounts while a turn is in
     flight (or work is queued). Sits centered between the input affordances
     and the action cluster; the idle bar shows none of this. All pills tint to
     the active model and open the Activity panel on click. */
  /* One neutral capsule (same surface as the settings pill) holding quiet
     ghost stats split by hairline dividers — reads as a single intentional
     "live" readout rather than three loud floating badges. Color is held back
     to --fg-muted; the model hue is reserved for the one pulsing live dot so
     the cluster blends into the toolbar instead of competing with it. */
  .live-pills {
    display: inline-flex; align-items: center;
    height: 26px;
    padding: 0 2px;
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 65%, transparent);
    border-radius: 999px;
    min-width: 0;
    animation: enter 180ms ease-out;
  }
  .live-pill {
    display: inline-flex; align-items: center; gap: 5px;
    height: 100%; padding: 0 9px;
    font: inherit; font-size: 11px; font-weight: 600; line-height: 1;
    color: var(--fg-muted);
    background: transparent;
    border: 0;
    border-radius: 999px;
    cursor: pointer;
    flex-shrink: 0;
    transition: color 140ms ease-out;
  }
  /* Hairline divider before every pill after the first. */
  .live-pill + .live-pill {
    box-shadow: inset 1px 0 0 color-mix(in oklch, var(--border) 55%, transparent);
  }
  .live-pill:hover { color: var(--fg); }
  .live-pill:active { transform: scale(0.97); }
  .live-pill:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .live-pill :global(svg) { color: var(--fg-faint); transition: color 140ms ease-out; }
  .live-pill:hover :global(svg) { color: var(--fg-muted); }
  .live-pill .mono { font-variant-numeric: tabular-nums; color: var(--fg-2); }
  .live-pill .lp-sep { color: var(--fg-faint); margin: 0 1px; }
  .live-pill .lp-unit { color: var(--fg-faint); font-weight: 500; margin-left: 2px; }
  /* The one accent — a pulsing model-tinted dot marking the live turn. */
  .lp-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--model-color);
    box-shadow: 0 0 6px color-mix(in oklch, var(--model-color) 65%, transparent);
    animation: lp-pulse 1.4s ease-in-out infinite;
  }
  @keyframes lp-pulse { 0%, 100% { opacity: 0.45; } 50% { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) { .lp-dot { animation: none; } }

  /* Send — primary CTA, accent surface w/ glow. Bigger than v2 (32px), more
     pronounced shadow, smoother mode-swap (send → stop → queue). */
  .sendbtn {
    position: relative;
    width: 32px; height: 32px;
    display: flex; align-items: center; justify-content: center;
    background: var(--accent);
    color: var(--accent-fg);
    border: 0; border-radius: 10px;
    cursor: pointer;
    flex-shrink: 0;
    overflow: hidden;
    transition: background 200ms ease-out, transform 140ms ease-out,
                box-shadow 220ms ease-out, opacity 140ms ease-out;
    box-shadow:
      inset 0 1px 0 color-mix(in oklch, white 18%, transparent),
      0 0 0 1px color-mix(in oklch, var(--accent) 40%, transparent),
      0 6px 18px -4px color-mix(in oklch, var(--accent) 60%, transparent);
  }
  .sendbtn:hover:not(:disabled) {
    background: var(--accent-hover);
    transform: translateY(-1px);
    box-shadow:
      inset 0 1px 0 color-mix(in oklch, white 22%, transparent),
      0 0 0 1px color-mix(in oklch, var(--accent) 55%, transparent),
      0 10px 28px -4px color-mix(in oklch, var(--accent) 75%, transparent);
  }
  .sendbtn:active:not(:disabled) { transform: translateY(0) scale(0.96); }
  .sendbtn:disabled {
    opacity: 0.35; cursor: default;
    box-shadow: inset 0 1px 0 color-mix(in oklch, white 10%, transparent);
  }
  .sendbtn:focus-visible {
    outline: none;
    box-shadow:
      inset 0 1px 0 color-mix(in oklch, white 22%, transparent),
      0 0 0 3px var(--ring),
      0 6px 18px -4px color-mix(in oklch, var(--accent) 60%, transparent);
  }
  .sendbtn.stop {
    background: var(--danger);
    color: oklch(0.98 0.01 22);
    box-shadow:
      inset 0 1px 0 color-mix(in oklch, white 22%, transparent),
      0 0 0 1px color-mix(in oklch, var(--danger) 50%, transparent),
      0 6px 18px -4px color-mix(in oklch, var(--danger) 60%, transparent);
  }
  .sendbtn.stop:hover { filter: brightness(1.08); transform: translateY(-1px); }
  .sendbtn.queue {
    background: color-mix(in oklch, var(--accent) 70%, var(--surface));
  }
  /* Launch ripple — two concentric rings expand outward on every fire().
     Mounted by {#key fireKey}; self-removed when the animation ends via
     the unmount on the next key flip. */
  .send-ripple {
    position: absolute;
    inset: -2px;
    border-radius: 14px;
    border: 1.5px solid color-mix(in oklch, var(--model-color) 70%, transparent);
    opacity: 0.85;
    pointer-events: none;
    animation: send-ripple 620ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .send-ripple-2 {
    animation-delay: 90ms;
    border-color: color-mix(in oklch, var(--model-color) 55%, transparent);
  }
  @keyframes send-ripple {
    from { transform: scale(0.6); opacity: 0.9; }
    to   { transform: scale(2.2); opacity: 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .send-ripple { animation: none; opacity: 0; }
  }
  .icon-stack {
    position: relative;
    display: inline-flex;
    width: 13px; height: 13px;
  }
  .icon-slot {
    position: absolute;
    inset: 0;
    display: inline-flex; align-items: center; justify-content: center;
    opacity: 0;
    transform: scale(0.6) rotate(-30deg);
    transition: opacity 180ms ease-out, transform 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .icon-slot.active {
    opacity: 1;
    transform: scale(1) rotate(0);
  }

  .attachments {
    display: flex; flex-wrap: wrap; align-items: center; gap: 8px;
    margin-bottom: 8px;
  }
  .attach-chip {
    display: inline-flex; align-items: center; gap: 8px;
    padding: 4px 8px 4px 4px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    transition: border-color 140ms ease-out, background 140ms ease-out;
  }
  .attach-chip:hover { border-color: var(--border-strong); }
  .attach-thumb {
    width: 40px; height: 40px;
    object-fit: cover;
    border-radius: 6px;
    background: var(--bg-elev-2);
  }
  .attach-meta {
    display: inline-flex; flex-direction: column; gap: 1px;
    line-height: 1.2;
  }
  .attach-name { font-size: var(--fs-xs); font-weight: 600; color: var(--fg); }
  .attach-size {
    font-size: 10px;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }
  .attach-x {
    display: inline-flex; align-items: center; justify-content: center;
    width: 20px; height: 20px;
    background: transparent;
    border: 0; border-radius: 50%;
    color: var(--fg-faint);
    cursor: pointer;
    padding: 0;
    margin-left: 2px;
  }
  .attach-x:hover { background: var(--bg-elev-2); color: var(--fg); }
  .attach-error {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 4px 8px 4px 10px;
    background: var(--danger-soft, color-mix(in oklch, var(--danger) 12%, transparent));
    border: 1px solid color-mix(in oklch, var(--danger) 35%, var(--border));
    border-radius: 8px;
    font-size: var(--fs-xs);
    color: var(--danger);
  }
  .attach-error-x {
    display: inline-flex; align-items: center; justify-content: center;
    width: 16px; height: 16px;
    background: transparent;
    border: 0; border-radius: 50%;
    color: var(--danger);
    cursor: pointer;
    opacity: 0.7;
    padding: 0;
  }
  .attach-error-x:hover { opacity: 1; background: color-mix(in oklch, var(--danger) 18%, transparent); }

  .queue {
    display: flex; flex-wrap: wrap; align-items: center; gap: 6px;
    margin-bottom: 8px;
    padding: 6px 10px;
    background: var(--bg-elev-1);
    border: 1px dashed var(--border-strong);
    border-radius: 10px;
    font-size: var(--fs-xs);
  }
  .queue-label {
    font-weight: 600;
    color: var(--fg-muted);
    margin-right: 4px;
  }
  .qpill {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 2px 4px 2px 8px;
    max-width: 220px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--fg);
  }
  .qtext {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .qx {
    display: inline-flex; align-items: center; justify-content: center;
    width: 16px; height: 16px;
    background: transparent;
    border: 0; border-radius: 50%;
    color: var(--fg-faint);
    cursor: pointer;
    padding: 0;
  }
  .qx:hover { background: var(--bg-elev-2); color: var(--fg); }
  .qclear {
    margin-left: auto;
    padding: 2px 9px;
    background: transparent;
    border: 1px solid color-mix(in oklch, var(--danger) 30%, var(--border));
    border-radius: 999px;
    color: color-mix(in oklch, var(--danger) 80%, var(--fg-muted));
    cursor: pointer;
    font: inherit;
    font-size: 10px;
    letter-spacing: 0.04em;
    transition: background 140ms ease-out, color 140ms ease-out;
  }
  .qclear:hover { background: var(--danger-soft); color: oklch(0.95 0.04 22); }

  .slash-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: 100%;
    max-height: 280px;
    overflow-y: auto;
    background: color-mix(in oklch, var(--surface) 86%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid color-mix(in oklch, var(--border) 80%, transparent);
    border-radius: 14px;
    box-shadow:
      0 18px 44px -8px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklch, var(--accent) 6%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    padding: 6px;
    z-index: 10;
    animation: slash-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .slash-item {
    display: flex; align-items: baseline; gap: 12px;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: 0; border-radius: 8px;
    color: var(--fg);
    text-align: left;
    cursor: pointer;
    font: inherit;
    transition: background 140ms ease-out;
    /* Stagger each item's entry — driven by inline style="--idx: {i}". */
    animation: slash-item-in 280ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(var(--idx, 0) * 22ms);
  }
  @keyframes slash-item-in {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .slash-item { animation: none; }
  }
  .slash-item:hover, .slash-item.active {
    background: color-mix(in oklch, var(--accent) 11%, transparent);
  }
  .slash-name {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--accent);
    min-width: 72px;
  }
  .slash-desc {
    font-size: var(--fs-xs);
    color: var(--fg-muted);
  }
  .slash-hint {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 12px 4px;
    margin-top: 4px;
    font-size: 10px;
    color: var(--fg-faint);
    border-top: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
  }

  /* ── Prompt enhancer preview ─────────────────────────────────────────
     Glass panel above the composer (mirrors .slash-menu positioning) holding
     the Haiku-rewritten draft. Accent reads from --model-color so it matches
     the active model's hue like the rest of the composer chrome. */
  .enhance-panel {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: 100%;
    box-sizing: border-box;
    background: color-mix(in oklch, var(--surface) 88%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid color-mix(in oklch, var(--model-color) 32%, var(--border));
    border-radius: 14px;
    box-shadow:
      0 18px 44px -8px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklch, var(--model-color) 10%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    padding: 12px;
    z-index: 10;
    animation: slash-in 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .enhance-head {
    display: flex; align-items: center; gap: 7px;
    margin-bottom: 8px;
    color: var(--model-color);
  }
  .enhance-title { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); }
  .enhance-sub {
    font-size: 10px; font-weight: 500;
    color: var(--fg-faint);
    margin-left: auto;
    letter-spacing: 0.02em;
  }
  .enhance-text {
    font-size: var(--fs-md);
    line-height: 1.55;
    color: var(--fg);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 220px;
    overflow-y: auto;
    padding: 2px 0;
    margin-bottom: 10px;
  }
  .enhance-actions { display: flex; align-items: center; gap: 8px; }
  .enhance-btn {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 5px 12px;
    border-radius: 8px;
    font: inherit; font-size: var(--fs-sm); font-weight: 600;
    cursor: pointer;
    transition: background 140ms ease-out, border-color 140ms ease-out, transform 120ms ease-out;
  }
  .enhance-btn:active { transform: scale(0.96); }
  .enhance-accept {
    background: var(--model-color);
    color: oklch(0.16 0.02 260);
    border: 1px solid transparent;
  }
  .enhance-accept:hover { background: color-mix(in oklch, var(--model-color) 88%, white 12%); }
  .enhance-discard {
    background: transparent;
    color: var(--fg-muted);
    border: 1px solid color-mix(in oklch, var(--border) 80%, transparent);
  }
  .enhance-discard:hover {
    background: color-mix(in oklch, var(--surface-hover) 80%, transparent);
    color: var(--fg);
  }
  .enhance-kbd { margin-left: auto; font-size: 10px; color: var(--fg-faint); }
  .enhance-error {
    display: flex; align-items: center; gap: 8px;
    font-size: var(--fs-sm);
    color: var(--danger);
  }
  .enhance-error-msg { flex: 1; }

  /* ── Magic "enchanting" state ─────────────────────────────────────────
     While the wand call is in flight, the effect lands on the message itself:
     the user's own draft turns into a sweeping gradient shimmer (clip-text)
     over a soft model-tinted aura with a few twinkling stars — not a spinner.
     The real textarea text + caret go transparent so the shimmer clone is the
     only thing visible. */
  .composer.enchanting textarea { color: transparent; caret-color: transparent; }
  .magic-text {
    position: absolute;
    inset: 0;
    z-index: 2;
    padding: 8px 10px 6px;
    font: inherit;
    font-size: var(--fs-md);
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    overflow: hidden;
    pointer-events: none;
    background-image: linear-gradient(
      100deg,
      color-mix(in oklch, var(--fg) 62%, transparent) 0%,
      color-mix(in oklch, var(--fg) 62%, transparent) 36%,
      color-mix(in oklch, var(--model-color) 55%, white) 46%,
      oklch(0.98 0.02 250) 50%,
      color-mix(in oklch, var(--model-color) 65%, white) 54%,
      color-mix(in oklch, var(--fg) 62%, transparent) 64%,
      color-mix(in oklch, var(--fg) 62%, transparent) 100%
    );
    background-size: 220% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    color: transparent;
    animation: magic-shimmer 1.15s linear infinite;
  }
  @keyframes magic-shimmer {
    0%   { background-position: 200% 0; }
    100% { background-position: -60% 0; }
  }
  .magic-aura {
    position: absolute;
    inset: -2px;
    z-index: 0;
    pointer-events: none;
    border-radius: 12px;
    background: radial-gradient(
      120% 80% at 50% 50%,
      color-mix(in oklch, var(--model-color) 16%, transparent),
      transparent 70%
    );
    animation: magic-aura-pulse 1.6s ease-in-out infinite;
  }
  @keyframes magic-aura-pulse {
    0%, 100% { opacity: 0.35; }
    50%      { opacity: 0.85; }
  }
  .magic-stars { position: absolute; inset: 0; z-index: 3; pointer-events: none; }
  .magic-stars i {
    position: absolute;
    left: var(--sx); top: var(--sy);
    width: var(--sz, 7px); height: var(--sz, 7px);
    background: color-mix(in oklch, white 78%, var(--model-color));
    /* 4-point sparkle — concave diamond reads as "magic" where a round dot
       read as a stray bug. */
    clip-path: polygon(50% 0%, 58% 42%, 100% 50%, 58% 58%, 50% 100%, 42% 58%, 0% 50%, 42% 42%);
    filter: drop-shadow(0 0 4px color-mix(in oklch, var(--model-color) 85%, transparent));
    opacity: 0;
    transform-origin: center;
    animation: magic-twinkle 1.6s ease-in-out infinite;
    animation-delay: var(--sd);
  }
  @keyframes magic-twinkle {
    0%, 100% { opacity: 0; transform: scale(0.2) rotate(0deg); }
    40%      { opacity: 1; transform: scale(1) rotate(40deg); }
    70%      { opacity: 0.5; transform: scale(0.7) rotate(70deg); }
  }

  /* Wand button glows + breathes while enchanting (instead of a spinner). */
  .wandbtn:hover:not(:disabled) { color: var(--model-color); }
  .wandbtn.enhancing {
    color: var(--model-color);
    border-color: color-mix(in oklch, var(--model-color) 40%, var(--border));
    opacity: 1;
    animation: wand-pulse 1.3s ease-in-out infinite;
  }
  @keyframes wand-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklch, var(--model-color) 40%, transparent); transform: scale(1); }
    50%      { box-shadow: 0 0 10px 2px color-mix(in oklch, var(--model-color) 32%, transparent); transform: scale(1.08); }
  }

  /* Reveal — each chunk of the enhanced text materializes out of blur,
     staggered. Delay capped so long outputs don't crawl in over seconds. */
  .ew {
    animation: word-materialize 420ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: min(calc(var(--i) * 14ms), 650ms);
  }
  /* While streaming, tokens already arrive staggered — drop the index delay so
     each word blurs in the moment its delta lands (typewriter feel), instead of
     queuing behind a growing per-word offset. */
  .ew.live {
    animation-delay: 0ms;
  }
  @keyframes word-materialize {
    from { opacity: 0; filter: blur(7px); }
    to   { opacity: 1; filter: blur(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .magic-text { animation: none; -webkit-text-fill-color: var(--fg-muted); }
    .magic-aura, .magic-stars i { animation: none; }
    .magic-stars i { opacity: 0.7; }
    .ew { animation: none; }
  }

  /* Phase 3a: hint popover. Replaces the dedicated hint row; lives adjacent
     to the mic in the composer row. Pop animates fade + 4px translate-y +
     scale 0.98→1 over 140ms. */
  .hint-wrap {
    position: relative;
    display: inline-flex;
    align-self: center;
  }
  /* .hintbtn inherits .iconbtn base (size, radius, hover). Only the
     aria-expanded "open" treatment is specific. */
  .hintbtn[aria-expanded="true"] {
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 25%, transparent);
    background: var(--accent-soft);
    opacity: 1;
  }
  /* Hint popover — viewport-fixed so it escapes .composer's overflow:hidden.
     Coords computed in positionHint(). `:global()` because Svelte 5 scopes
     style by the markup it sees; this element renders inside the composer
     but the CSS would otherwise be tree-shaken since the popover has been
     moved out of scoped scope by some HMR paths. */
  :global(.hint-pop) {
    position: fixed;
    min-width: 240px;
    padding: 10px 14px 8px;
    background: color-mix(in oklch, var(--bg-elev-1) 92%, transparent);
    backdrop-filter: blur(16px) saturate(140%);
    -webkit-backdrop-filter: blur(16px) saturate(140%);
    border: 1px solid color-mix(in oklch, var(--accent) 22%, var(--border));
    border-radius: 12px;
    box-shadow:
      0 16px 40px -8px oklch(0 0 0 / 0.6),
      0 0 0 1px color-mix(in oklch, var(--accent) 8%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    z-index: 9998;
    display: flex; flex-direction: column; gap: 2px;
    animation: hint-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
    transform-origin: bottom left;
  }
  :global(.hint-pop .hint-head) {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
    padding-bottom: 6px;
    margin-bottom: 4px;
    border-bottom: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
  }
  @keyframes hint-in {
    from { opacity: 0; transform: translateY(4px) scale(0.98); }
    to   { opacity: 1; transform: translateY(0) scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.hint-pop) { animation: none; }
  }
  :global(.hint-pop .hint-row) {
    display: grid;
    grid-template-columns: 92px 1fr;
    align-items: center;
    gap: 12px;
    padding: 4px 2px;
    font-size: var(--fs-xs);
    color: var(--fg-2);
  }
  :global(.hint-pop .hint-keys) {
    display: inline-flex; align-items: center; gap: 3px;
    justify-content: flex-start;
  }
  :global(.hint-pop .hint-row > span:last-child) {
    color: var(--fg-muted);
    text-align: left;
    line-height: 1.3;
  }
  :global(.hint-pop kbd) {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 18px; height: 18px;
    padding: 0 6px;
    font-family: var(--font-ui);
    font-size: 10.5px;
    font-weight: 600;
    background: color-mix(in oklch, var(--bg-elev-2) 80%, transparent);
    border: 1px solid color-mix(in oklch, var(--border-strong) 70%, transparent);
    border-radius: 4px;
    color: var(--fg);
    line-height: 1;
    letter-spacing: 0.01em;
  }

  /* Unified settings pill — one icon-only control opening the model /
     thinking-depth / permission-mode panel. `data-mode` faintly tints the
     icon so the current permission posture (unguarded vs cautious) reads at
     a glance without spelling it out. */
  .settings-pill {
    align-self: center;
    display: inline-flex; align-items: center; gap: 3px;
    padding: 0 8px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 75%, transparent);
    border-radius: 999px;
    color: var(--fg-2);
    cursor: pointer;
    font: inherit;
    height: 26px;
    overflow: hidden;
    transition: background 140ms ease-out, color 140ms ease-out, border-color 140ms ease-out;
  }
  .settings-pill:hover {
    background: color-mix(in oklch, var(--bg-elev-2) 90%, transparent);
    color: var(--fg);
    border-color: var(--border);
  }
  .settings-pill.open {
    border-color: color-mix(in oklch, var(--accent) 55%, var(--border));
    color: var(--fg);
  }
  .settings-pill:hover .pill-caret { color: var(--fg-muted); transform: translateY(1px); }
  /* Bypass = unguarded. Don't flood the whole pill in warning-amber (it washed
     the model name + clashed with the violet ultracode marker). Keep the label
     neutral; the posture reads from the dot alone. */
  .settings-pill[data-mode="bypassPermissions"] { color: var(--fg-2); }
  .settings-pill[data-mode="default"] { color: var(--accent); }
  /* Current-model label on the pill — replaces the icon-only rest state so
     the active model reads at a glance without hovering. */
  .pill-label {
    font-size: 11px;
    font-weight: 600;
    line-height: 1;
    letter-spacing: 0.01em;
    max-width: 72px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mode-icon { display: inline-flex; align-items: center; }
  /* Signal-bar effort indicator — 3 vertical bars growing left-to-right.
     `data-level` (1|2|3) fills bars in current color; unfilled bars stay
     dim. Same visual vocab as wifi/battery so the ladder reads instantly. */
  .effort-bars {
    display: inline-flex; align-items: flex-end; gap: 2px;
    height: 11px;
    animation: effort-bars-in 320ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .effort-bars .bar {
    width: 2.5px;
    background: color-mix(in oklch, currentColor 22%, transparent);
    border-radius: 1px;
    transition: background 200ms ease-out, height 200ms ease-out;
  }
  .effort-bars .bar:nth-child(1) { height: 35%; }
  .effort-bars .bar:nth-child(2) { height: 65%; }
  .effort-bars .bar:nth-child(3) { height: 100%; }
  .effort-bars[data-level="1"] .bar:nth-child(1),
  .effort-bars[data-level="2"] .bar:nth-child(-n+2),
  .effort-bars[data-level="3"] .bar { background: currentColor; }
  /* Active bar gets a tiny pulse — draws the eye to the current rung. */
  .effort-bars[data-level="1"] .bar:nth-child(1),
  .effort-bars[data-level="2"] .bar:nth-child(2),
  .effort-bars[data-level="3"] .bar:nth-child(3) {
    animation: effort-bar-tip 1.8s ease-in-out infinite;
  }
  @keyframes effort-bars-in {
    from { opacity: 0; transform: translateY(-2px) scale(0.6); }
    to   { opacity: 1; transform: translateY(0)    scale(1); }
  }
  @keyframes effort-bar-tip {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.6; }
  }
  @media (prefers-reduced-motion: reduce) {
    .effort-bars { animation: none; }
    .effort-bars .bar { animation: none; }
  }
  /* Permission-mode dot — one consistent at-a-glance signal for all five
     modes (the pill's text-tint only covered ask/bypass). Colored per mode:
     ask=accent, edit=ok, plan=blue, auto=accent, bypass=warn. */
  .mode-dot {
    width: 6px; height: 6px; border-radius: 50%;
    flex-shrink: 0;
    background: var(--fg-faint);
    transition: background 140ms ease-out;
  }
  .settings-pill[data-mode="default"] .mode-dot       { background: var(--accent); }
  .settings-pill[data-mode="acceptEdits"] .mode-dot   { background: var(--ok); }
  .settings-pill[data-mode="plan"] .mode-dot          { background: oklch(0.74 0.13 230); }
  .settings-pill[data-mode="auto"] .mode-dot          { background: var(--accent); }
  .settings-pill[data-mode="bypassPermissions"] .mode-dot {
    background: oklch(0.72 0.165 55);
    box-shadow: 0 0 5px color-mix(in oklch, oklch(0.72 0.165 55) 70%, transparent);
  }
  .pill-caret {
    font-size: 8px;
    color: var(--fg-faint);
    margin-left: 1px;
    line-height: 1;
    transition: color 140ms ease-out, transform 140ms ease-out;
  }
  .mention-menu { max-height: 280px; }
  .mention-item {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: baseline;
    gap: 10px;
    padding: 5px 10px;
  }
  .mention-base {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--fs-sm);
    color: var(--fg);
    font-weight: 500;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mention-dir {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
  }
  .mention-item.active .mention-base { color: var(--accent); }

  .model-menu {
    padding: 6px;
    background: color-mix(in oklch, var(--surface) 86%, transparent);
    backdrop-filter: blur(14px) saturate(140%);
    -webkit-backdrop-filter: blur(14px) saturate(140%);
    border: 1px solid color-mix(in oklch, var(--border) 80%, transparent);
    box-shadow:
      0 18px 44px -8px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklch, var(--accent) 6%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
  }
  .model-menu .model-header {
    display: flex; align-items: center; gap: 10px;
    padding: 8px 12px 6px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-faint);
  }
  .model-menu .model-header::after {
    content: "";
    flex: 1;
    height: 1px;
    background: linear-gradient(to right,
      color-mix(in oklch, var(--border) 80%, transparent),
      transparent);
  }
  .model-dot {
    width: 8px; height: 8px;
    border-radius: 999px;
    background: var(--model-color, var(--fg-muted));
    box-shadow:
      0 0 0 2px color-mix(in oklch, var(--model-color, var(--fg-muted)) 16%, transparent),
      0 0 8px color-mix(in oklch, var(--model-color, var(--fg-muted)) 55%, transparent);
  }

  /* Unified settings panel — compact, right-anchored under the pill. Model is
     a 2×2 card grid; thinking depth + permission mode sit side-by-side below.
     Descriptions live in hover tooltips so the panel stays short (no scroll)
     and content-width instead of spanning the whole composer. */
  .settings-menu {
    left: auto; right: 0;
    width: max-content;
    min-width: 440px; max-width: 560px;
    max-height: min(82vh, 600px);
    padding: 8px 10px 4px;
  }
  .settings-section { padding: 0 2px; }
  .settings-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px;
    padding: 2px 2px 8px;
  }
  .settings-cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    align-items: start;
  }
  .settings-stack {
    display: flex; flex-direction: column; gap: 2px;
    padding: 2px 2px 4px;
  }
  .settings-item {
    position: relative;
    display: flex; align-items: center; gap: 8px;
    width: 100%;
    padding: 7px 10px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 9px;
    color: var(--fg);
    text-align: left;
    cursor: pointer;
    font: inherit;
    transition: background 140ms ease-out, border-color 140ms ease-out;
    animation: slash-item-in 280ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(var(--idx, 0) * 16ms);
  }
  .settings-item:hover,
  .settings-item.active {
    background: color-mix(in oklch, var(--accent) 11%, transparent);
  }
  .settings-item.current {
    background: color-mix(in oklch, var(--accent) 13%, transparent);
    border-color: color-mix(in oklch, var(--accent) 38%, var(--border));
  }
  /* Model card — name + version on top, ctx badge beneath; tagline in tooltip. */
  .model-card {
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 9px 10px;
  }
  .model-card .card-head { display: inline-flex; align-items: center; gap: 7px; }
  .model-card[data-id="sonnet"]          { --model-color: oklch(0.74 0.13 230); }
  .model-card[data-id="opus"]            { --model-color: oklch(0.70 0.18 295); }
  .model-card[data-id="claude-opus-4-7"] { --model-color: oklch(0.70 0.18 295); }
  .model-card[data-id="haiku"]           { --model-color: oklch(0.78 0.14 180); }
  .settings-item.current .model-label { color: var(--accent); }
  .settings-item.current .model-version {
    color: var(--accent);
    background: color-mix(in oklch, var(--accent) 16%, transparent);
  }
  /* Effort + mode rows — single line: indicator + label, desc in tooltip. */
  .effort-row .effort-bars { height: 12px; color: var(--fg-muted); animation: none; }
  .effort-row .effort-bars .bar { animation: none; }
  .effort-row.current .effort-bars { color: var(--accent); }
  .mode-row .mode-icon { color: var(--fg-muted); }
  .mode-row.current .mode-icon { color: var(--accent); }
  /* Ultracode tier — the top rung, set apart from the bar ladder. Violet (the
     CLI's own ultracode accent) + a sparkle glyph signal "beyond the ladder":
     xhigh effort + autonomous multi-agent workflow orchestration. */
  .effort-ultra-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 12px; height: 12px;
    color: oklch(0.72 0.19 300);
    opacity: 0.7;
    transition: opacity 160ms ease-out, filter 160ms ease-out;
  }
  .effort-row.ultra.current .effort-ultra-icon,
  .effort-row.ultra.active .effort-ultra-icon {
    opacity: 1;
    filter: drop-shadow(0 0 5px color-mix(in oklch, oklch(0.72 0.19 300) 55%, transparent));
  }
  .effort-row.ultra.current .model-label { color: oklch(0.79 0.14 300); }
  /* Glanceable pill marker when ultracode is the active tier. */
  .pill-ultra {
    display: inline-flex; align-items: center;
    color: oklch(0.75 0.18 300);
    filter: drop-shadow(0 0 4px color-mix(in oklch, oklch(0.72 0.19 300) 55%, transparent));
    animation: ultra-pulse 2.6s ease-in-out infinite;
  }
  .settings-pill.ultra {
    border-color: color-mix(in oklch, oklch(0.72 0.19 300) 42%, var(--border));
  }
  @keyframes ultra-pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.62; }
  }
  @media (prefers-reduced-motion: reduce) {
    .pill-ultra { animation: none; }
  }

  .model-label {
    font-weight: 600;
    color: var(--fg);
    font-size: var(--fs-sm);
  }
  .model-version {
    font-size: 10px;
    font-weight: 600;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
    padding: 1px 5px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border-radius: 4px;
  }
  .model-ctx {
    font-size: 10px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    padding: 3px 8px;
    border-radius: 999px;
    white-space: nowrap;
    color: var(--fg-muted);
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 75%, transparent);
  }
  .model-ctx.wide {
    color: var(--accent);
    background: color-mix(in oklch, var(--accent) 8%, transparent);
    border-color: color-mix(in oklch, var(--accent) 30%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--accent) 8%, transparent);
  }

  .model-menu .model-hint {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 12px 6px;
    margin-top: 4px;
    border-top: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
  }
  .model-menu .model-hint kbd {
    display: inline-block;
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 9.5px;
    font-weight: 600;
    line-height: 1;
    padding: 2px 5px;
    margin-right: 5px;
    border-radius: 4px;
    background: color-mix(in oklch, var(--bg-elev-2) 75%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    color: var(--fg-muted);
    vertical-align: 1px;
  }
</style>
