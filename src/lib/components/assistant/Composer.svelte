<script lang="ts">
  import { Send, Square, X, Mic, Loader2, HelpCircle, Wand2, Check, Paperclip,
    Hand, Code2, ClipboardList, Zap, Infinity as InfinityIcon,
    Bot, Terminal, Wrench, ListPlus, Sparkles, Eye, ChevronUp,
    RefreshCw, FolderSearch, GitCompare, Clock, Pencil, Navigation } from "lucide-svelte";
  import { fly } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import { assistant } from "../../state/assistant.svelte";
  import type { ModelSel, PermissionMode } from "../../state/assistant/types";
  import Markdown from "./Markdown.svelte";
  import EditDiff from "./EditDiff.svelte";
  import { modelFamily, liveActivity, fableAvailable } from "../../state/assistant/helpers";
  import { fmtClock, fuzzyScore, effortIdxFromX, bytesToBase64, isFileDrag } from "./composer/helpers";
  import AttachmentsRow from "./composer/AttachmentsRow.svelte";
  import { portal } from "$lib/actions/portal";
  import { stt } from "../../state/stt.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";
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

  // ── Pending Rail (queue chips) ────────────────────────────────────────
  // Inline edit-before-fire for a queued message. editingId pins the chip in
  // edit mode; commit writes back into tab.queue, Esc/blur cancels.
  let editingId = $state<string | null>(null);
  let editText = $state("");
  function startEditQueued(q: { id: string; text: string }) {
    editingId = q.id;
    editText = q.text;
    void tick().then(() => {
      const el = document.querySelector<HTMLInputElement>(`[data-qedit="${q.id}"]`);
      el?.focus();
      el?.select();
    });
  }
  function commitEditQueued() {
    if (!editingId || !tab) { editingId = null; return; }
    const next = editText.trim();
    const id = editingId;
    editingId = null;
    if (!next) { tab.queue = tab.queue.filter((it) => it.id !== id); return; }
    tab.queue = tab.queue.map((it) => (it.id === id ? { ...it, text: next } : it));
  }
  function onEditKey(e: KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); commitEditQueued(); }
    else if (e.key === "Escape") { e.preventDefault(); editingId = null; }
  }
  function removeQueued(id: string) {
    if (tab) tab.queue = tab.queue.filter((it) => it.id !== id);
  }
  // Promote a parked chip into the live turn: steer it now + drop it from the
  // queue. Only meaningful while streaming (assistant.steer re-queues on miss,
  // so no message is lost if the turn ends mid-click).
  function sendQueuedNow(q: { id: string; text: string }) {
    if (!tab || !streaming) return;
    removeQueued(q.id);
    void assistant.steer(q.text, tabId);
  }
  // Drag-to-reorder queued chips — the queue order IS the send order. Reorder
  // live on dragover so the rail rearranges under the cursor.
  let dragId = $state<string | null>(null);
  function onChipDragStart(e: DragEvent, id: string) {
    dragId = id;
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }
  function onChipDragOver(e: DragEvent, overId: string) {
    if (!dragId || dragId === overId || !tab) return;
    e.preventDefault();
    const from = tab.queue.findIndex((q) => q.id === dragId);
    const to = tab.queue.findIndex((q) => q.id === overId);
    if (from < 0 || to < 0) return;
    const next = tab.queue.slice();
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    tab.queue = next;
  }
  function onChipDragEnd() { dragId = null; }

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
  const toolCount = $derived(liveItems.filter((i) => i.kind === "tool").length);
  const turnStartedAt = $derived(tab?.activity.turnStartedAt ?? null);
  const turnElapsed = $derived(
    streaming && turnStartedAt != null ? fmtClock(now - turnStartedAt) : null,
  );
  // tok/s is session-global telemetry; recompute each tick by touching `now`.
  const tokPerSec = $derived.by(() => {
    void now;
    return streaming ? assistant.telemetry.snapshot().summary.outputTokensPerSec : null;
  });
  const showLivePills = $derived(streaming || agentCount > 0 || shellCount > 0 || toolCount > 0 || queue.length > 0);
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
    v: { id: string; mime: string; dataBase64: string; sizeBytes: number }[],
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
  // Effort tiers, low→high. Used by both the slider and per-model capability caps.
  type EffortId = "none" | "quick" | "deep" | "ultra";
  type ModelOpt = {
    id: ModelSel;
    label: string;
    version: string;
    tagline: string;
    ctx: string;
    suffix: string;   // muted inline tag beside the name (e.g. "1M context")
    legacy: boolean;   // previous-generation — grouped under a "Legacy" subhead
    limited?: boolean; // limited-run — accent name + "until" badge (Fable)
    // ── Capability matrix (source of truth for what each model can actually do).
    // Drives every affordance gate so the panel never offers a mode the model
    // ignores server-side. Grounded in the model capability docs:
    //   • effort     — accepts the CLI `--effort` flag at all. Haiku skips
    //                  extended thinking wholesale, so this is false for Haiku.
    //   • maxEffort  — highest effort tier the model honors. Opus reaches
    //                  "ultra" (xhigh + ultracode); Sonnet tops out at "deep"
    //                  (high) — xhigh/ultracode are Opus-tier only.
    //   • fastMode   — Opus-only faster-output mode (CC's `/fast`).
    effort: boolean;
    maxEffort: EffortId;
    fastMode: boolean;
  };
  // Flat single-column list (Claude-Code-Desktop layout): current models first,
  // legacy generations grouped below. `opus` is the alias → newest Opus (4.8,
  // 1M-ctx beta); `claude-opus-4-7` pins the prior generation. The CLI takes
  // the alias / pinned id; name + suffix are display-only.
  // Fable 5 is a limited run — row exists only while fableAvailable() (through
  // Jun 22 2026); after sunset the list collapses back to the standard four.
  const MODEL_OPTIONS: ModelOpt[] = [
    ...(fableAvailable() ? [{ id: "claude-fable-5" as ModelSel, label: "Fable", version: "5", tagline: "Anthropic's most capable model — limited run, retired after Jun 22", ctx: "1M ctx", suffix: "1M context", legacy: false, limited: true, effort: true, maxEffort: "ultra" as EffortId, fastMode: false }] : []),
    { id: "opus",            label: "Opus",   version: "4.8", tagline: "Newest + most capable — complex reasoning & agentic coding", ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: "ultra", fastMode: true  },
    { id: "sonnet",          label: "Sonnet", version: "4.6", tagline: "Best speed + intelligence balance — the default",            ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: "deep",  fastMode: false },
    { id: "haiku",           label: "Haiku",  version: "4.5", tagline: "Fastest, near-frontier — quick edits & lookups",             ctx: "200K ctx", suffix: "200K context", legacy: false, effort: false, maxEffort: "none",  fastMode: false },
    { id: "claude-opus-4-7", label: "Opus",   version: "4.7", tagline: "Previous-generation Opus — proven for complex reasoning",    ctx: "1M ctx",   suffix: "1M context",   legacy: true,  effort: true,  maxEffort: "ultra", fastMode: true  },
  ];
  const currentModels = MODEL_OPTIONS.filter((m) => !m.legacy);
  const legacyModels = MODEL_OPTIONS.filter((m) => m.legacy);
  // 1-based number shortcut → model id (digit keys pick directly in the menu).
  const modelShortcut = (id: ModelSel) => MODEL_OPTIONS.findIndex((m) => m.id === id) + 1;

  // Idle placeholder — static ghost with `/` `@` keycaps (mock `.ph-ghost`).
  let composerFocused = $state(false);

  function autosize() {
    if (!ta) return;
    ta.style.height = "auto";
    ta.style.height = Math.min(ta.scrollHeight, 340) + "px";
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
  const mentionResults = $derived.by(() => {
    if (!mentionState) return [] as string[];
    const q = mentionState.query;
    const files = assistant.workspaceFiles;
    if (q.length === 0) return files.slice(0, 12);
    const scored: { path: string; score: number }[] = [];
    for (const f of files) {
      const s = fuzzyScore(f, q);
      if (s !== null) scored.push({ path: f, score: s });
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
  // Two toolbar pills (mock split): `settingsOpen` drives the model+effort
  // popover (settings-pill), `permOpen` drives the permission-mode popover
  // (perm-pill). `settingsIdx`/`permIdx` are the keyboard cursors for each.
  let settingsOpen = $state(false);
  let settingsIdx = $state(0);
  let permOpen = $state(false);
  let permIdx = $state(0);
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
  type EffortOpt = { id: EffortId; label: string; hint: string; level: 1 | 2 | 3 | 4 };
  const EFFORT_OPTIONS: EffortOpt[] = [
    { id: "none",  label: "Instant", level: 1, hint: "Instant — straight to the answer, no thinking time" },
    { id: "quick", label: "Smart",   level: 2, hint: "Smart — thinks briefly before answering (~5s extra)" },
    { id: "deep",  label: "Deep",    level: 3, hint: "Deep — heavy reasoning (~15s extra) for hard problems" },
    { id: "ultra", label: "Ultracode", level: 4, hint: "Ultracode — max reasoning + autonomous multi-agent workflows. Claude orchestrates fleets of subagents for the most exhaustive answer." },
  ];
  const currentEffort = $derived(EFFORT_OPTIONS.find((e) => e.id === assistant.thinkingEffort) ?? EFFORT_OPTIONS[1]);
  // ── Capability gates, all derived from the current model's matrix entry. The
  // panel offers ONLY what the selected model honors server-side, so the UI
  // never promises a mode that silently does nothing.
  // `effortApplies`: does this model use extended-thinking effort at all (false
  // for Haiku — the backend drops `--effort` / never sets the ultracode key).
  const effortApplies = $derived(currentModel?.effort ?? true);
  // `effortStops`: the slider's allowed tiers — EFFORT_OPTIONS truncated at the
  // model's ceiling (Opus → Ultracode/xhigh; Sonnet → Deep/high). A prefix
  // slice, so an index into it equals the absolute tier index.
  const effortStops = $derived.by(() => {
    if (!currentModel?.effort) return [] as EffortOpt[];
    const cap = EFFORT_OPTIONS.findIndex((e) => e.id === currentModel.maxEffort);
    return EFFORT_OPTIONS.slice(0, cap >= 0 ? cap + 1 : EFFORT_OPTIONS.length);
  });
  // Fast mode is Opus-only (CC's `/fast`). It is also not yet plumbed to the CLI
  // spawn (ui-prefs TODO), so the row stays hidden behind FAST_MODE_WIRED until
  // the backend honors it — showing a dead toggle would be the exact false
  // signal we're removing. Flip to true once wired; it then appears Opus-only.
  const FAST_MODE_WIRED = false;
  const fastModeApplies = $derived((currentModel?.fastMode ?? false) && FAST_MODE_WIRED);
  // Effort rendered as a Faster↔Smarter slider over `effortStops`. Index drives
  // the knob position + keyboard ←/→ nudge.
  const effortIdx = $derived(
    Math.min(
      Math.max(0, EFFORT_OPTIONS.findIndex((e) => e.id === assistant.thinkingEffort)),
      Math.max(0, effortStops.length - 1),
    ),
  );
  function setEffortByIdx(i: number) {
    const max = Math.max(0, effortStops.length - 1);
    const c = Math.min(max, Math.max(0, i));
    if (effortStops[c]) assistant.setThinkingEffort(effortStops[c].id);
  }
  // Switching to a lower-ceiling model (e.g. Ultracode → Sonnet) must pull the
  // stored effort down to that model's max, so we never send xhigh/ultracode to
  // a model that rejects it.
  $effect(() => {
    if (!effortApplies || effortStops.length === 0) return;
    if (!effortStops.some((e) => e.id === assistant.thinkingEffort)) {
      assistant.setThinkingEffort(effortStops[effortStops.length - 1].id);
    }
  });
  // Plain-language summary of what the current model + effort selection actually
  // does — so users aren't guessing what a mode gets them into. Model-aware:
  // reflects each model's real ceiling, and flags the heavy Ultracode tier.
  const modelCaption = $derived.by(() => {
    const m = currentModel;
    if (!m) return "";
    if (!m.effort) return `${m.label} ${m.version} answers right away — it doesn't use extended-thinking effort modes.`;
    return currentEffort.hint;
  });
  // Pointer-drag the slider: map clientX → nearest stop. Pointer-capture on the
  // track keeps move/up flowing even when the cursor leaves the row.
  let effortTrackEl: HTMLDivElement | null = $state(null);
  let draggingEffort = $state(false);
  function effortIdxFromClientX(clientX: number): number {
    if (!effortTrackEl) return effortIdx;
    return effortIdxFromX(clientX, effortTrackEl.getBoundingClientRect(), effortStops.length);
  }
  function startEffortDrag(e: PointerEvent) {
    e.preventDefault();
    draggingEffort = true;
    setEffortByIdx(effortIdxFromClientX(e.clientX));
    effortTrackEl?.setPointerCapture?.(e.pointerId);
  }
  function moveEffortDrag(e: PointerEvent) {
    if (!draggingEffort) return;
    setEffortByIdx(effortIdxFromClientX(e.clientX));
  }
  function endEffortDrag() { draggingEffort = false; }

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
  const PermIcon = $derived(currentMode.icon);
  function pickMode(m: ModeOpt) {
    assistant.setPermissionMode(m.id);
    permOpen = false;
    void tick().then(() => ta?.focus());
  }
  // Shift+Tab cycles the permission mode (mock affordance) without opening the menu.
  function cyclePerm() {
    const i = MODE_OPTIONS.findIndex((m) => m.id === assistant.permissionMode);
    assistant.setPermissionMode(MODE_OPTIONS[(i + 1) % MODE_OPTIONS.length].id);
  }

  // Flat, navigable row list spanning all three sections of the unified
  // settings panel. Effort is dropped on Haiku (ignored server-side), exactly
  // as the old standalone effort pill was hidden there. Drives ArrowUp/Down
  // + the active highlight; mouse clicks call the per-kind pick fns directly.
  type SettingsRow =
    | { kind: "model"; model: ModelOpt }
    | { kind: "fast" }
    | { kind: "effort" };
  const settingsRows = $derived.by<SettingsRow[]>(() => {
    const rows: SettingsRow[] = MODEL_OPTIONS.map((m) => ({ kind: "model" as const, model: m }));
    if (fastModeApplies) rows.push({ kind: "fast" });
    if (effortApplies && effortStops.length > 0) rows.push({ kind: "effort" });
    return rows;
  });
  // Re-seed the cursor to the current model row whenever the panel opens.
  $effect(() => {
    if (settingsOpen) {
      const i = settingsRows.findIndex((r) => r.kind === "model" && r.model.id === assistant.model);
      settingsIdx = i >= 0 ? i : 0;
    }
  });
  // Re-seed the perm cursor to the current mode whenever the perm menu opens.
  $effect(() => {
    if (permOpen) {
      const i = MODE_OPTIONS.findIndex((m) => m.id === assistant.permissionMode);
      permIdx = i >= 0 ? i : 0;
    }
  });
  function pickRow(row: SettingsRow) {
    if (row.kind === "model") pickModel(row.model);
    else if (row.kind === "fast") uiPrefs.toggleFastMode();
    else { settingsOpen = false; void tick().then(() => ta?.focus()); }
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

  // Alt+Enter while streaming: steer the running turn instead of queueing.
  // Injects the draft into the live CLI stdin (assistant.steer) so the agent
  // course-corrects mid-turn. Shift+Enter stays newline; Enter stays queue.
  // Brief "Steered ✓" confirmation on the rail button — feedback at the point
  // of action, not just a corner toast.
  let steerFlash = $state(false);
  let steerFlashTimer: ReturnType<typeof setTimeout> | null = null;
  function steer() {
    const text = draft.trim();
    if (!text || !streaming) return;
    setDraft("");
    stt.consume();
    void assistant.steer(text, tabId);
    steerFlash = true;
    if (steerFlashTimer) clearTimeout(steerFlashTimer);
    steerFlashTimer = setTimeout(() => { steerFlash = false; }, 1400);
    void tick().then(autosize);
  }

  // ── Prompt enhancer (wand) ───────────────────────────────────────────────
  // One-shot Haiku rewrite of the current draft into a clearer prompt. Result
  // shows as an editable preview above the composer — Accept drops it into the
  // textarea, Discard dismisses. Never auto-sends, never overwrites silently.
  let enhancing = $state(false);
  let enhancedPreview = $state<string | null>(null);
  let enhanceError = $state<string | null>(null);
  // The draft we enhanced FROM — kept so Regenerate/refine re-run on the
  // original (not the already-enhanced text) and the diff has a baseline.
  let enhanceOriginal = $state<string | null>(null);
  // Toggle the body between the enhanced text and a diff vs the original.
  let showEnhanceDiff = $state(false);
  // Opt-in: let the rewrite read the real workspace (read-only). Slower, more
  // specific. Preference persists across regenerates within the session.
  let groundEnhance = $state(false);
  // Draft preview (eye) — render the composer draft as Markdown before sending.
  let previewing = $state(false);
  // Split preserving whitespace so the reveal can stagger word-by-word while
  // keeping spacing/newlines intact. Each chunk gets its own materialize delay.
  const enhancedWords = $derived(
    enhancedPreview === null ? [] : enhancedPreview.split(/(\s+)/),
  );
  // `directive` steers a refine pass (Concise / More detail / + Acceptance);
  // omitted for the first run + plain Regenerate.
  // Generation token: accept/dismiss bumps it so a still-in-flight enhance
  // can't write its stream/result back into a closed preview.
  let enhanceSeq = 0;
  async function runEnhance(directive?: string) {
    const text = (enhanceOriginal ?? draft).trim();
    if (!text || enhancing) return;
    if (enhanceOriginal === null) enhanceOriginal = text;
    const seq = ++enhanceSeq;
    enhancing = true;
    enhanceError = null;
    enhancedPreview = "";
    try {
      // Stream: deltas fill the preview live; the resolved value is the
      // authoritative final text. Grounded mode passes the workspace cwd.
      const result = await assistant.enhancePrompt(
        text,
        (full) => { if (seq === enhanceSeq) enhancedPreview = full; },
        { directive, cwd: groundEnhance ? (assistant.workspace.current ?? undefined) : undefined },
      );
      if (seq === enhanceSeq) enhancedPreview = result;
    } catch (e) {
      if (seq === enhanceSeq) {
        enhanceError = String(e);
        enhancedPreview = null;
      }
    } finally {
      if (seq === enhanceSeq) enhancing = false;
    }
  }
  function acceptEnhanced() {
    if (!enhancedPreview) return;
    enhanceSeq++;
    enhancing = false;
    setDraft(enhancedPreview);
    enhancedPreview = null;
    enhanceError = null;
    enhanceOriginal = null;
    showEnhanceDiff = false;
    void tick().then(() => { autosize(); ta?.focus(); });
  }
  function dismissEnhanced() {
    enhanceSeq++;
    enhancing = false;
    enhancedPreview = null;
    enhanceError = null;
    enhanceOriginal = null;
    showEnhanceDiff = false;
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
  // Permission-mode menu — portals to <body> like the hint pop, so it escapes
  // the composer's `overflow: hidden` + backdrop-filter containing block.
  let permWrap = $state<HTMLButtonElement | null>(null);
  let permPop = $state<HTMLDivElement | null>(null);
  let permPos = $state<{ top: number; left: number }>({ top: 0, left: 0 });
  function positionPerm() {
    if (!permWrap || !permPop) return;
    const a = permWrap.getBoundingClientRect();
    const ph = permPop.offsetHeight || 220;
    const pw = permPop.offsetWidth || 252;
    let top = a.top - ph - 8;
    if (top < 8) top = a.bottom + 8;
    let left = a.left;
    const maxLeft = window.innerWidth - pw - 8;
    if (left > maxLeft) left = maxLeft;
    if (left < 8) left = 8;
    permPos = { top, left };
  }
  function onDocPermMousedown(ev: MouseEvent) {
    if (!permOpen) return;
    if (permWrap && ev.target instanceof Node && permWrap.contains(ev.target)) return;
    if (permPop && ev.target instanceof Node && permPop.contains(ev.target)) return;
    permOpen = false;
  }
  $effect(() => {
    window.addEventListener("mousedown", onDocPermMousedown);
    return () => window.removeEventListener("mousedown", onDocPermMousedown);
  });
  $effect(() => {
    if (!permOpen) return;
    void tick().then(positionPerm);
    const onResize = () => positionPerm();
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

  // Up-arrow recall offset (0 = newest). Reset whenever the user types or
  // switches focus so the next Up starts at the newest prompt again.
  let recallOffset = $state(-1);
  function resetRecall() { recallOffset = -1; }

  function onKey(e: KeyboardEvent) {
    if (e.isComposing) return;
    // Shift+Tab cycles permission mode (mock affordance), regardless of menus.
    if (e.key === "Tab" && e.shiftKey) {
      e.preventDefault();
      cyclePerm();
      return;
    }
    // Enhance preview claims Escape first — dismiss it before menu handlers.
    if ((enhancedPreview !== null || enhanceError !== null) && e.key === "Escape") {
      e.preventDefault();
      dismissEnhanced();
      return;
    }
    // Permission-mode menu nav (mirrors the settings-menu nav below).
    if (permOpen) {
      const n = MODE_OPTIONS.length;
      if (e.key === "ArrowDown") { e.preventDefault(); permIdx = (permIdx + 1) % n; return; }
      if (e.key === "ArrowUp") { e.preventDefault(); permIdx = (permIdx - 1 + n) % n; return; }
      if (e.key === "Enter" || e.key === "Tab") { e.preventDefault(); pickMode(MODE_OPTIONS[permIdx]); return; }
      if (e.key === "Escape") { e.preventDefault(); permOpen = false; return; }
      if (e.key.length === 1) permOpen = false;
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
      const cur = settingsRows[settingsIdx];
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
      // ←/→ drives the effort slider when the cursor is parked on it.
      if ((e.key === "ArrowRight" || e.key === "ArrowLeft") && cur?.kind === "effort") {
        e.preventDefault();
        setEffortByIdx(effortIdx + (e.key === "ArrowRight" ? 1 : -1));
        return;
      }
      // Digit 1–N jumps straight to that model row.
      if (/^[1-9]$/.test(e.key)) {
        const m = MODEL_OPTIONS[Number(e.key) - 1];
        if (m) { e.preventDefault(); pickModel(m); return; }
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        pickRow(cur);
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
    // Alt+Enter steers the running turn (must precede the plain-Enter branch
    // below, which also matches when Alt is held).
    if (e.key === "Enter" && e.altKey && streaming && draft.trim().length > 0) {
      e.preventDefault();
      steer();
      return;
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
  {#if streaming || queue.length > 0}
    <div class="pending-rail" class:working={streaming && queue.length === 0} transition:fly={{ y: 14, duration: 260, easing: quintOut }}>
      <span class="rail-sweep" aria-hidden="true"></span>
      <Clock size={12} class="rail-lead" />
      <span class="rail-caption">
        {queue.length === 0
          ? "Working…"
          : queue.length === 1
          ? "Sends when ready"
          : `${queue.length} queued`}
      </span>
      {#each queue as q, i (q.id)}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="pchip"
          class:dragging={dragId === q.id}
          style="--idx: {i}"
          draggable={queue.length > 1 && editingId !== q.id}
          ondragstart={(e) => onChipDragStart(e, q.id)}
          ondragover={(e) => onChipDragOver(e, q.id)}
          ondragend={onChipDragEnd}
          in:fly={{ y: 10, duration: 220, delay: 30 * i, easing: quintOut }}
          out:fly={{ y: 12, duration: 200, easing: quintOut }}
        >
          {#if editingId === q.id}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="pchip-edit"
              data-qedit={q.id}
              bind:value={editText}
              onkeydown={onEditKey}
              onblur={commitEditQueued}
              aria-label="Edit queued message"
            />
          {:else}
            <span class="pchip-text" class:grab={queue.length > 1} use:tooltip={q.text}>{q.text}</span>
            {#if streaming}
              <button class="pchip-btn accent" type="button" onclick={() => sendQueuedNow(q)} aria-label="Send now into the running turn" use:tooltip={"Send now — inject into the running turn"}>
                <Navigation size={11} />
              </button>
            {/if}
            <button class="pchip-btn" type="button" onclick={() => startEditQueued(q)} aria-label="Edit queued message" use:tooltip={"Edit"}>
              <Pencil size={11} />
            </button>
          {/if}
          <button class="pchip-btn" type="button" onclick={() => removeQueued(q.id)} aria-label="Remove from queue" use:tooltip={"Remove"}>
            <X size={11} />
          </button>
        </div>
      {/each}
      <div class="rail-actions">
        {#if streaming}
          <button
            class="rail-steer"
            class:flashed={steerFlash}
            type="button"
            onclick={steer}
            disabled={!steerFlash && !draft.trim()}
            aria-label="Steer the running turn"
            use:tooltip={draft.trim()
              ? "Redirect the running turn with your draft (Alt+Enter)"
              : "Type a message, then Steer to redirect the running turn"}
          >
            {#if steerFlash}
              <Check size={11} />
              Steered
            {:else}
              <Navigation size={11} />
              Steer
            {/if}
          </button>
        {/if}
        {#if queue.length >= 2}
          <button
            class="rail-clear"
            type="button"
            onclick={() => { if (tab) tab.queue = []; }}
          >
            Clear
          </button>
        {/if}
      </div>
    </div>
  {/if}

  <AttachmentsRow
    {attachments}
    {attachError}
    onRemove={(id) => assistant.removeAttachment(id, tabId)}
    onDismissError={() => (attachError = null)}
  />

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
            <span class="enhance-title">{enhancing ? (groundEnhance ? "Consulting workspace…" : "Enhancing…") : "Enhanced prompt"}</span>
            <div class="enhance-head-tools">
              {#if assistant.workspace.current}
                <button
                  type="button"
                  class="enhance-toggle"
                  class:on={groundEnhance}
                  onclick={() => (groundEnhance = !groundEnhance)}
                  disabled={enhancing}
                  aria-pressed={groundEnhance}
                  use:tooltip={"Ground the rewrite in your real code (read-only). Slower, more specific. Re-run to apply."}
                >
                  <FolderSearch size={12} /> Ground
                </button>
              {/if}
              {#if enhanceOriginal && !enhancing}
                <button
                  type="button"
                  class="enhance-toggle"
                  class:on={showEnhanceDiff}
                  onclick={() => (showEnhanceDiff = !showEnhanceDiff)}
                  aria-pressed={showEnhanceDiff}
                  use:tooltip={showEnhanceDiff ? "Show enhanced text" : "Show what changed vs your draft"}
                >
                  <GitCompare size={12} /> Diff
                </button>
              {/if}
            </div>
          </div>
          {#if showEnhanceDiff && enhanceOriginal}
            <div class="enhance-diff">
              <EditDiff input={{ old_string: enhanceOriginal, new_string: enhancedPreview }} hideHead compact />
            </div>
          {:else}
            <div class="enhance-text">
              {#each enhancedWords as w, i (i)}<span class="ew" class:live={enhancing} style="--i:{i}">{w}</span>{/each}
            </div>
          {/if}
          <div class="enhance-actions">
            <button type="button" class="enhance-btn enhance-accept" onclick={acceptEnhanced} disabled={enhancing || !enhancedPreview}>
              <Check size={13} /> Use this
            </button>
            <button type="button" class="enhance-btn enhance-discard" onclick={dismissEnhanced}>
              Discard
            </button>
            <span class="enhance-sep" aria-hidden="true"></span>
            <button type="button" class="enhance-refine" onclick={() => runEnhance()} disabled={enhancing} use:tooltip={"Regenerate from your original draft"}>
              <RefreshCw size={12} /> Regenerate
            </button>
            <button type="button" class="enhance-refine" onclick={() => runEnhance("Make it more concise — cut to the essentials, keep every technical specific.")} disabled={enhancing}>Concise</button>
            <button type="button" class="enhance-refine" onclick={() => runEnhance("Add more implementation detail and the edge cases worth handling.")} disabled={enhancing}>More detail</button>
            <button type="button" class="enhance-refine" onclick={() => runEnhance("Append a short acceptance-criteria checklist of what 'done' looks like.")} disabled={enhancing}>+ Acceptance</button>
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

    {#if previewing && draft.trim().length > 0}
      <div class="preview-panel" role="region" aria-label="Message preview">
        <div class="preview-head">
          <Eye size={12} />
          <span class="preview-title">Preview</span>
          <span class="preview-sub">rendered Markdown</span>
        </div>
        <div class="preview-body"><Markdown text={draft} /></div>
      </div>
    {/if}

    {#if slashOpen && slashFiltered.length > 0}
      <div class="rift-menu slash-menu" role="menu">
        {#each slashFiltered as c, i (c.name)}
          <button
            type="button"
            role="menuitem"
            class="rift-menu-row slash-row"
            class:active={i === slashIdx}
            style="--idx: {i}"
            onmousedown={(e) => { e.preventDefault(); pickSlash(c); }}
          >
            <span class="rift-menu-row-body">
              <span class="rift-menu-row-t slash-cmd">/{c.name}</span>
              <span class="rift-menu-row-d">{c.desc}</span>
            </span>
          </button>
        {/each}
        <div class="slash-hint">↑↓ select · Tab/Enter pick · Esc cancel</div>
      </div>
    {/if}

    {#if mentionState && mentionResults.length > 0}
      <div class="rift-menu slash-menu mention-menu" role="menu">
        {#each mentionResults as path, i (path)}
          {@const slash = path.lastIndexOf("/")}
          {@const dir = slash > 0 ? path.slice(0, slash + 1) : ""}
          {@const base = slash >= 0 ? path.slice(slash + 1) : path}
          <button
            type="button"
            role="menuitem"
            class="rift-menu-row mention-item"
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
      {@const stops = effortStops.length}
      {@const effortPct = stops > 1 ? (effortIdx / (stops - 1)) * 100 : 0}
      <div class="rift-menu settings-menu" role="menu">
        <div class="rift-menu-head">Model</div>
        {#each MODEL_OPTIONS as m, i (m.id)}
          {#if m.legacy && (i === 0 || !MODEL_OPTIONS[i - 1].legacy)}
            <div class="rift-menu-sub">Legacy</div>
          {/if}
          <button
            type="button"
            role="menuitemradio"
            aria-checked={m.id === assistant.model}
            class="rift-menu-row model-row"
            class:active={i === settingsIdx}
            class:current={m.id === assistant.model}
            use:tooltip={m.tagline}
            onmousedown={(e) => { e.preventDefault(); pickModel(m); }}
          >
            <span class="rift-menu-row-t model-row-name" class:limited={m.limited}>{m.label} {m.version}</span>
            {#if m.limited}<span class="model-badge">Until Jun 22</span>{/if}
            {#if m.suffix}<span class="model-suffix" class:legacy={m.legacy}>{m.suffix}</span>{/if}
            {#if m.id === assistant.model}
              <Check size={14} class="rift-menu-row-chk" />
            {:else}
              <kbd class="model-num">{modelShortcut(m.id)}</kbd>
            {/if}
          </button>
        {/each}

        {#if fastModeApplies}
          <div class="rift-menu-divider"></div>
          <div class="rift-menu-sub">Fast mode</div>
          <button
            type="button"
            class="rift-menu-row toggle-row"
            class:active={settingsRows[settingsIdx]?.kind === "fast"}
            onmousedown={(e) => { e.preventDefault(); uiPrefs.toggleFastMode(); }}
            role="menuitemcheckbox"
            aria-checked={uiPrefs.fastMode}
          >
            <span class="rift-menu-row-body">
              <span class="rift-menu-row-t">Enable fast mode</span>
              <span class="rift-menu-row-d">Opus with faster output</span>
            </span>
            <span class="rift-toggle" class:on={uiPrefs.fastMode} aria-hidden="true">
              <span class="rift-toggle-knob"></span>
            </span>
          </button>
        {/if}

        <div class="rift-menu-divider"></div>
        {#if effortApplies}
          <div class="effort-head" class:ultra={currentEffort.id === "ultra"}>
            <span class="effort-head-l">Effort <b>{currentEffort.label}</b></span>
            <button
              type="button"
              role="menuitem"
              class="effort-help"
              use:tooltip={currentEffort.hint}
              onmousedown={(e) => e.preventDefault()}
              aria-label="What does effort do?"
            ><HelpCircle size={12} /></button>
          </div>
          <div
            class="effort-slider"
            class:active={settingsRows[settingsIdx]?.kind === "effort"}
            class:ultra={currentEffort.id === "ultra"}
            class:dragging={draggingEffort}
            role="slider"
            tabindex="0"
            aria-label="Effort"
            onkeydown={(e) => { if (e.key === 'ArrowRight') { e.preventDefault(); setEffortByIdx(effortIdx + 1); } else if (e.key === 'ArrowLeft') { e.preventDefault(); setEffortByIdx(effortIdx - 1); } }}
            aria-valuemin={1}
            aria-valuemax={stops}
            aria-valuenow={effortIdx + 1}
            aria-valuetext={currentEffort.label}
          >
            <div
              class="effort-track"
              role="presentation"
              bind:this={effortTrackEl}
              onpointerdown={startEffortDrag}
              onpointermove={moveEffortDrag}
              onpointerup={endEffortDrag}
              onpointercancel={endEffortDrag}
            >
              <div class="effort-fill" style="width: {effortPct}%"></div>
              {#each effortStops as e, i (e.id)}
                <button
                  type="button"
                  class="effort-notch"
                  class:on={i <= effortIdx}
                  class:cur={i === effortIdx}
                  class:ultra={e.id === "ultra"}
                  style="left: {stops > 1 ? (i / (stops - 1)) * 100 : 0}%"
                  use:tooltip={e.hint}
                  aria-label={e.label}
                  onmousedown={(ev) => { ev.preventDefault(); setEffortByIdx(i); }}
                ></button>
              {/each}
              <div class="effort-knob" style="left: {effortPct}%"></div>
            </div>
            <div class="effort-ends"><span>Faster</span><span>Smarter</span></div>
          </div>
        {/if}
        <p class="model-caption" class:warn={effortApplies && currentEffort.id === "ultra"}>{modelCaption}</p>

        <div class="rift-menu-hint">
          <span><kbd>1–{MODEL_OPTIONS.length}</kbd>model</span>
          {#if effortApplies}<span><kbd>←→</kbd>effort</span>{/if}
          <span><kbd>↵</kbd>pick</span>
          <span><kbd>Esc</kbd>close</span>
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
          <span class="placeholder-ghost static" aria-hidden="true">Ask Claude · <span class="ph-k">/</span> for commands · <span class="ph-k">@</span> to mention a file</span>
        {:else if streaming && draft.length === 0}
          <span class="placeholder-ghost static" aria-hidden="true">Type to queue for after this turn · <span class="ph-k">/stop</span> halts</span>
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
          <span class="ctx-readout" data-tone={ctxTone} aria-hidden="true">{ctxPct < 1 ? "<1" : Math.round(ctxPct)}% context</span>
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
          {#if draft.trim().length > 0}
          <span class="tb-div reveal" aria-hidden="true"></span>
          <button
            class="iconbtn wandbtn reveal"
            class:enhancing
            type="button"
            onclick={() => runEnhance()}
            disabled={enhancing}
            use:tooltip={enhancing ? "Enhancing…" : "Improve prompt — clean up & clarify"}
            aria-label="Improve prompt"
          >
            <Wand2 size={14} />
          </button>
          <button
            class="iconbtn previewbtn reveal"
            class:on={previewing}
            type="button"
            onclick={() => (previewing = !previewing)}
            aria-pressed={previewing}
            use:tooltip={previewing ? "Hide preview" : "Preview as Markdown"}
            aria-label="Preview message"
          >
            <Eye size={14} />
          </button>
          <button
            class="iconbtn clearbtn reveal"
            type="button"
            onclick={() => { setDraft(""); ta?.focus(); }}
            use:tooltip={"Clear draft"}
            aria-label="Clear draft"
          >
            <X size={14} />
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
                aria-label="Current turn — elapsed · output speed. Click to open Activity."
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
                aria-label={`${agentCount} sub-agent${agentCount === 1 ? "" : "s"} running. Click to open Activity.`}
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
                aria-label={`${shellCount} shell${shellCount === 1 ? "" : "s"} running. Click to open Activity.`}
                use:tooltip={`${shellCount} shell${shellCount === 1 ? "" : "s"} running. Click to open Activity.`}
              >
                <Terminal size={12} />
                <span class="mono">{shellCount}</span>
              </button>
            {/if}
            {#if toolCount > 0}
              <button
                type="button"
                class="live-pill"
                onclick={openActivity}
                aria-label={`${toolCount} tool${toolCount === 1 ? "" : "s"} running. Click to open Activity.`}
                use:tooltip={`${toolCount} tool${toolCount === 1 ? "" : "s"} running. Click to open Activity.`}
              >
                <Wrench size={12} />
                <span class="mono">{toolCount}</span>
              </button>
            {/if}
            {#if queue.length > 0}
              <button
                type="button"
                class="live-pill queued"
                onclick={openActivity}
                aria-label={`${queue.length} message${queue.length === 1 ? "" : "s"} queued to send after this turn.`}
                use:tooltip={`${queue.length} message${queue.length === 1 ? "" : "s"} queued to send after this turn.`}
              >
                <ListPlus size={12} />
                <span class="mono">{queue.length}</span>
              </button>
            {/if}
          </div>
        {:else if composerFocused}
          <div class="kbd-hint" aria-hidden="true">
            <kbd>↵</kbd><span class="kh-t">send</span>
            <span class="kh-sep">·</span>
            <kbd>⇧↵</kbd><span class="kh-t">new line</span>
          </div>
        {/if}

        <div class="toolbar-cluster toolbar-right">
          <button
            type="button"
            class="perm-pill"
            class:open={permOpen}
            data-mode={currentMode.id}
            bind:this={permWrap}
            onclick={() => { permOpen = !permOpen; settingsOpen = false; void tick().then(() => ta?.focus()); }}
            aria-haspopup="listbox"
            aria-expanded={permOpen}
            aria-label="Permission mode"
            use:tooltip={{ text: `Permission mode — ${currentMode.label}`, kbd: "⇧Tab" }}
          >
            <PermIcon size={13} />
            <span class="perm-label">{currentMode.label}</span>
            <ChevronUp size={12} class="pill-chev" />
          </button>

          {#if permOpen}
            <div
              class="perm-menu"
              role="menu"
              bind:this={permPop}
              use:portal
              style="top: {permPos.top}px; left: {permPos.left}px;"
            >
              <div class="mm-head">Permission mode <kbd class="perm-kbd">⇧Tab</kbd></div>
              {#each MODE_OPTIONS as m, i (m.id)}
                {@const Icon = m.icon}
                <button
                  type="button"
                  role="menuitemradio"
                  aria-checked={m.id === assistant.permissionMode}
                  class="perm-row"
                  class:active={i === permIdx}
                  class:current={m.id === assistant.permissionMode}
                  data-mode={m.id}
                  use:tooltip={m.hint}
                  onmousedown={(ev) => { ev.preventDefault(); pickMode(m); }}
                >
                  <span class="perm-row-ic"><Icon size={13} /></span>
                  <span class="perm-row-tt">
                    <span class="perm-row-t">{m.label}</span>
                    <span class="perm-row-d">{m.hint.split(" — ")[1] ?? m.hint}</span>
                  </span>
                  {#if m.id === assistant.permissionMode}<Check size={13} class="perm-row-chk" />{/if}
                </button>
              {/each}
            </div>
          {/if}

          <button
            type="button"
            class="settings-pill"
            class:open={settingsOpen}
            class:ultra={effortApplies && assistant.thinkingEffort === "ultra"}
            data-model={currentModel ? modelFamily(currentModel.id) : ""}
            onclick={() => { settingsOpen = !settingsOpen; permOpen = false; void tick().then(() => ta?.focus()); }}
            aria-haspopup="listbox"
            aria-expanded={settingsOpen}
            aria-label="Model & thinking depth"
            use:tooltip={effortApplies
              ? `Model · thinking depth\n${currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.model} · ${currentEffort.label}`
              : `Model\n${currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.model} · no extended thinking`}
          >
            <span class="mode-dot" aria-hidden="true"></span>
            <span class="pill-label">{currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.model}</span>
            {#if effortApplies}
              <span class="pill-effort">· {currentEffort.label}</span>
            {/if}
            {#if effortApplies && assistant.thinkingEffort === "ultra"}
              <span class="pill-ultra" aria-hidden="true" use:tooltip={"Ultracode — max reasoning + autonomous workflows"}><Sparkles size={11} /></span>
            {/if}
            <ChevronUp size={13} class="pill-chev" />
          </button>
          <button
            class="sendbtn"
            class:stop={mode === "stop"}
            class:queue={mode === "queue"}
            type="button"
            onclick={onBtnClick}
            disabled={!canFire}
            aria-label={mode === 'stop' ? 'Stop current turn' : mode === 'queue' ? 'Queue message' : 'Send message'}
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

  <div class="composer-disclaimer">
    <span class="cd-tag">Beta</span>
    AI can make mistakes — review changes before relying on them.
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
  /* Persistent beta + AI-disclaimer line under the input. Quiet, single-line. */
  .composer-disclaimer {
    display: flex; align-items: center; justify-content: center; gap: 6px;
    margin-top: 7px;
    font-size: 10px; line-height: 1.2; color: var(--fg-faint);
    text-align: center;
  }
  .composer-disclaimer .cd-tag {
    padding: 1px 6px; border-radius: 999px;
    font-size: 9px; font-weight: 700; letter-spacing: 0.05em; text-transform: uppercase;
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 28%, transparent);
  }
  /* Composer chrome is emerald-only — the ring/divider/send/ripple no longer
     tint by model. Every accent inside reads from this single source; model
     identity lives on the model-card swatch in the picker. */
  .composer-wrap                      { --model-color: var(--accent); }
  .composer-shell { position: relative; z-index: 1; }
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
    min-height: 28px; max-height: 340px;
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
  .placeholder-ghost .ph-k { font-family: var(--font-mono); color: var(--fg-faint); font-size: 11px; padding: 0 2px; }
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
    flex-shrink: 0; white-space: nowrap;
    font-family: var(--font-mono);
    font-size: 10px; line-height: 1;
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
    box-shadow: 0 0 6px color-mix(in oklab, var(--warn) 50%, transparent);
  }
  .composer-divider[data-tone="red"] .composer-divider-fill {
    background: var(--danger);
    box-shadow: 0 0 8px color-mix(in oklab, var(--danger) 55%, transparent);
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
    color: var(--fg-subtle);
    border: 1px solid transparent;
    border-radius: 8px;
    cursor: pointer;
    flex-shrink: 0;
    opacity: 0.9;
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
  /* Hairline splitting compose-tools (attach/improve/mic) from view-tools (preview/help). */
  .tb-div {
    width: 1px; height: 16px; flex-shrink: 0;
    margin: 0 3px;
    background: color-mix(in oklch, var(--border) 80%, transparent);
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
    border-color: color-mix(in oklab, var(--accent) 40%, var(--border));
    opacity: 1;
  }
  :global(.mic-spin) { animation: mic-spin 0.9s linear infinite; }
  @keyframes mic-spin { to { transform: rotate(360deg); } }
  @keyframes mic-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklab, var(--danger) 45%, transparent); }
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
    .micbtn.recording { animation: none; }
    :global(.mic-spin) { animation: none; }
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
    border-color: color-mix(in oklab, var(--warn) 35%, var(--border));
    background: color-mix(in oklab, var(--warn) 10%, transparent);
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
      0 0 0 1px color-mix(in oklab, var(--accent) 40%, transparent),
      0 6px 18px -4px color-mix(in oklab, var(--accent) 60%, transparent);
  }
  .sendbtn:hover:not(:disabled) {
    background: var(--accent-hover);
    transform: translateY(-1px);
    box-shadow:
      inset 0 1px 0 color-mix(in oklch, white 22%, transparent),
      0 0 0 1px color-mix(in oklab, var(--accent) 55%, transparent),
      0 10px 28px -4px color-mix(in oklab, var(--accent) 75%, transparent);
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
      0 6px 18px -4px color-mix(in oklab, var(--accent) 60%, transparent);
  }
  .sendbtn.stop {
    background: var(--danger);
    color: oklch(0.98 0.01 22);
    box-shadow:
      inset 0 1px 0 color-mix(in oklch, white 22%, transparent),
      0 0 0 1px color-mix(in oklab, var(--danger) 50%, transparent),
      0 6px 18px -4px color-mix(in oklab, var(--danger) 60%, transparent);
  }
  .sendbtn.stop:hover { filter: brightness(1.08); transform: translateY(-1px); }
  .sendbtn.queue {
    background: color-mix(in oklab, var(--accent) 70%, var(--surface));
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

  /* ── Pending Rail ────────────────────────────────────────────────────
     Queue chips docked to the composer's top lip. Rounded top corners +
     square bottom + a downward tuck (margin-bottom) make it read as a shelf
     the composer grew, not a floating box. The composer card (z-index 1)
     overlaps the rail's bottom edge so chips appear to rise out of it. */
  .pending-rail {
    position: relative;
    z-index: 0;
    display: flex; flex-wrap: wrap; align-items: center; gap: 6px;
    margin: 0 6px -10px;
    padding: 7px 12px 15px;
    overflow: hidden;
    background: color-mix(in oklch, var(--surface) 70%, transparent);
    backdrop-filter: blur(10px) saturate(130%);
    -webkit-backdrop-filter: blur(10px) saturate(130%);
    border: 1px solid color-mix(in oklch, var(--accent) 22%, var(--border));
    border-bottom: 0;
    border-radius: 14px 14px 0 0;
    box-shadow: inset 0 1px 0 color-mix(in oklch, white 4%, transparent);
    font-size: var(--fs-xs);
  }
  /* One-time accent sweep on activation — plays once when the rail mounts. */
  .rail-sweep {
    position: absolute; inset: 0;
    pointer-events: none;
    background: linear-gradient(
      100deg,
      transparent 0%,
      color-mix(in oklch, var(--accent) 0%, transparent) 35%,
      color-mix(in oklch, var(--accent) 28%, transparent) 50%,
      transparent 65%
    );
    background-size: 220% 100%;
    background-position: 120% 0;
    animation: rail-sweep 760ms ease-out 1;
  }
  @keyframes rail-sweep {
    from { background-position: 120% 0; }
    to   { background-position: -40% 0; }
  }
  :global(.rail-lead) { color: color-mix(in oklch, var(--accent) 85%, var(--fg-muted)); flex: none; }
  /* While "Working…" (no queue), breathe the lead + caption so the rail reads
     as a live turn-in-progress surface, not a static shelf. */
  .pending-rail.working :global(.rail-lead),
  .pending-rail.working .rail-caption { animation: rail-breathe 1.8s ease-in-out infinite; }
  @keyframes rail-breathe {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.55; }
  }
  .rail-caption {
    font-weight: 600;
    letter-spacing: 0.01em;
    color: color-mix(in oklch, var(--accent) 55%, var(--fg-muted));
    margin-right: 2px;
  }
  .pchip {
    display: inline-flex; align-items: center; gap: 3px;
    padding: 2px 4px 2px 9px;
    max-width: 260px;
    background: var(--field);
    border: 1px solid color-mix(in oklch, var(--border) 92%, transparent);
    border-radius: 999px;
    color: var(--fg);
    box-shadow: 0 1px 3px -1px oklch(0 0 0 / 0.3);
  }
  .pchip.dragging { opacity: 0.45; }
  .pchip-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pchip-text.grab { cursor: grab; }
  .pchip.dragging .pchip-text.grab { cursor: grabbing; }
  .pchip-edit {
    min-width: 120px; max-width: 240px;
    padding: 1px 4px;
    background: var(--bg-inset);
    border: 1px solid color-mix(in oklch, var(--accent) 45%, var(--border));
    border-radius: 6px;
    color: var(--fg);
    font: inherit; font-size: var(--fs-xs);
    outline: none;
  }
  .pchip-btn {
    display: inline-flex; align-items: center; justify-content: center;
    width: 17px; height: 17px;
    background: transparent;
    border: 0; border-radius: 50%;
    color: var(--fg-faint);
    cursor: pointer;
    padding: 0;
    transition: background 120ms ease-out, color 120ms ease-out;
  }
  .pchip-btn:hover { background: var(--bg-elev-2); color: var(--fg); }
  .pchip-btn.accent { color: color-mix(in oklab, var(--accent) 80%, var(--fg-faint)); }
  .pchip-btn.accent:hover { background: color-mix(in oklab, var(--accent) 20%, transparent); color: color-mix(in oklab, var(--accent) 95%, white); }
  .rail-actions {
    display: inline-flex; align-items: center; gap: 6px;
    margin-left: auto;
  }
  .rail-steer {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 2px 10px;
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 45%, var(--border));
    border-radius: 999px;
    color: color-mix(in oklab, var(--accent) 85%, var(--fg));
    cursor: pointer;
    font: inherit;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    transition: background 140ms ease-out, color 140ms ease-out, opacity 140ms ease-out;
  }
  .rail-steer:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent) 26%, transparent);
    color: color-mix(in oklab, var(--accent) 95%, white);
  }
  .rail-steer:disabled { opacity: 0.4; cursor: default; }
  .rail-steer.flashed {
    background: color-mix(in oklab, var(--accent) 30%, transparent);
    color: color-mix(in oklab, var(--accent) 95%, white);
    border-color: color-mix(in oklab, var(--accent) 70%, var(--border));
    opacity: 1;
  }
  .rail-clear {
    padding: 2px 10px;
    background: transparent;
    border: 1px solid color-mix(in oklab, var(--danger) 30%, var(--border));
    border-radius: 999px;
    color: color-mix(in oklab, var(--danger) 80%, var(--fg-muted));
    cursor: pointer;
    font: inherit;
    font-size: 10px;
    letter-spacing: 0.04em;
    transition: background 140ms ease-out, color 140ms ease-out;
  }
  .rail-clear:hover { background: var(--danger-soft); color: oklch(0.95 0.04 22); }
  @media (prefers-reduced-motion: reduce) {
    .rail-sweep { animation: none; }
    .pending-rail.working :global(.rail-lead),
    .pending-rail.working .rail-caption { animation: none; }
  }

  /* Slash + mention popovers — share the .rift-menu chrome; this only carries
     positioning (full-width, anchored above the composer) + the entry tween. */
  .slash-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: 100%;
    max-height: 280px;
    overflow-y: auto;
    z-index: 10;
    animation: slash-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  /* Per-row staggered entry — driven by inline style="--idx: {i}". */
  .slash-row, .mention-item {
    animation: slash-item-in 280ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(var(--idx, 0) * 22ms);
  }
  @keyframes slash-item-in {
    from { opacity: 0; transform: translateY(6px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .slash-row, .mention-item { animation: none; }
  }
  .slash-cmd { font-family: var(--font-mono, ui-monospace, monospace); color: var(--accent); }
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
  /* Draft preview (eye) — same glass panel as enhance, neutral chrome. */
  .preview-panel {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: 100%;
    box-sizing: border-box;
    background: color-mix(in oklch, var(--surface) 88%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid var(--border-strong);
    border-radius: 14px;
    box-shadow:
      0 18px 44px -8px oklch(0 0 0 / 0.55),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    padding: 12px;
    z-index: 10;
    animation: slash-in 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .preview-head {
    display: flex; align-items: center; gap: 7px;
    margin-bottom: 8px;
    color: var(--fg-muted);
  }
  .preview-title { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); }
  .preview-sub {
    font-size: 10px; font-weight: 500;
    color: var(--fg-faint);
    margin-left: auto;
    letter-spacing: 0.02em;
  }
  .preview-body {
    font-size: var(--fs-md);
    line-height: 1.55;
    color: var(--fg);
    max-height: 280px;
    overflow-y: auto;
    padding: 2px 4px;
  }
  .previewbtn.on { color: var(--accent); background: var(--accent-soft); }
  .enhance-title { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); }
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
  .enhance-actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
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
  /* Refine + diff/ground controls on the enhance panel. */
  .enhance-head-tools { margin-left: auto; display: inline-flex; align-items: center; gap: 5px; }
  .enhance-toggle {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 3px 8px; border-radius: 999px;
    font: inherit; font-size: 10.5px; font-weight: 600;
    color: var(--fg-muted);
    background: color-mix(in oklch, var(--bg-elev-2) 60%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    cursor: pointer;
    transition: color 130ms, background 130ms, border-color 130ms;
  }
  .enhance-toggle:hover:not(:disabled) { color: var(--fg); border-color: var(--border-strong); }
  .enhance-toggle:disabled { opacity: 0.5; cursor: default; }
  .enhance-toggle.on {
    color: var(--model-color);
    border-color: color-mix(in oklab, var(--model-color) 45%, var(--border));
    background: color-mix(in oklab, var(--model-color) 12%, transparent);
  }
  .enhance-diff {
    max-height: 240px; overflow-y: auto;
    margin-bottom: 10px;
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    border-radius: 10px;
    padding: 4px;
  }
  .enhance-sep { width: 1px; height: 18px; background: color-mix(in oklch, var(--border) 70%, transparent); margin: 0 2px; }
  .enhance-refine {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 4px 9px; border-radius: 8px;
    font: inherit; font-size: 11px; font-weight: 600;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    cursor: pointer;
    transition: color 130ms, background 130ms, border-color 130ms, transform 120ms;
  }
  .enhance-refine:hover:not(:disabled) { color: var(--fg); background: color-mix(in oklch, var(--surface-hover) 70%, transparent); border-color: var(--border-strong); }
  .enhance-refine:active:not(:disabled) { transform: scale(0.96); }
  .enhance-refine:disabled { opacity: 0.45; cursor: default; }
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
  @media (prefers-reduced-motion: reduce) {
    .wandbtn.enhancing { animation: none; }
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

  /* Compose-tools (improve/preview) reveal once the draft has text — the empty
     composer stays calm. Reuses the global `enter` keyframe. */
  .iconbtn.reveal, .tb-div.reveal { animation: enter 160ms ease-out; }
  @media (prefers-reduced-motion: reduce) {
    .iconbtn.reveal, .tb-div.reveal { animation: none; }
  }
  /* Keyboard hint — occupies the toolbar's middle slot while the composer is
     focused (and no turn is live); keeps the idle bar empty + calm. */
  .kbd-hint {
    display: inline-flex; align-items: center; gap: 5px;
    font-size: 10.5px; color: var(--fg-faint);
    user-select: none; white-space: nowrap;
    animation: enter 160ms ease-out;
  }
  .kbd-hint .kh-t { letter-spacing: 0.01em; }
  .kbd-hint .kh-sep { color: var(--fg-subtle); opacity: 0.55; margin: 0 1px; }
  .kbd-hint kbd {
    display: inline-flex; align-items: center; justify-content: center;
    min-width: 16px; height: 16px; padding: 0 4px;
    font-family: var(--font-ui); font-size: 10px; font-weight: 600; line-height: 1;
    color: var(--fg-muted);
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    border-radius: 4px;
  }
  @media (prefers-reduced-motion: reduce) { .kbd-hint { animation: none; } }

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
    border-color: color-mix(in oklab, var(--accent) 55%, var(--border));
    color: var(--fg);
  }
  .settings-pill:hover :global(.pill-chev) { color: var(--fg-muted); }
  /* Current-model label on the pill. */
  .pill-label {
    font-size: 11px;
    font-weight: 600;
    line-height: 1;
    letter-spacing: 0.01em;
    max-width: 84px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Effort label trails the model name (mock `.pill-effort`). */
  .pill-effort { font-size: 11px; font-weight: 500; color: var(--fg-faint); line-height: 1; white-space: nowrap; }
  /* Permission-mode dot — one consistent at-a-glance signal for all five
     modes (the pill's text-tint only covered ask/bypass). Colored per mode:
     ask=accent, edit=ok, plan=blue, auto=accent, bypass=warn. */
  /* Leading dot on the collapsed settings-pill — emerald (model identity lives
     in the model-card dropdown, not the always-visible pill). */
  .mode-dot {
    width: 6px; height: 6px; border-radius: 50%;
    flex-shrink: 0;
    background: var(--accent);
    transition: background 140ms ease-out;
  }
  /* Chevron-up caret on both composer pills; rotates 180° when its menu opens. */
  :global(.settings-pill .pill-chev),
  :global(.perm-pill .pill-chev) {
    color: var(--fg-faint);
    transition: color 140ms ease-out, transform 140ms ease-out;
  }
  .settings-pill.open :global(.pill-chev),
  .perm-pill.open :global(.pill-chev) { transform: rotate(180deg); color: var(--fg-muted); }

  /* ── Permission-mode pill + menu (mock split from the model pill) ──────── */
  .toolbar-right { position: relative; }
  .perm-pill {
    align-self: center;
    display: inline-flex; align-items: center; gap: 5px;
    height: 26px; padding: 0 7px 0 9px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 75%, transparent);
    border-radius: 999px; color: var(--fg-2); cursor: pointer; font: inherit;
    transition: background 140ms ease-out, color 140ms ease-out, border-color 140ms ease-out;
  }
  .perm-pill:hover, .perm-pill.open {
    background: color-mix(in oklch, var(--bg-elev-2) 90%, transparent);
    color: var(--fg); border-color: var(--border);
  }
  .perm-pill > :global(svg:first-child) { color: var(--fg-muted); flex-shrink: 0; }
  .perm-label { font-size: 11px; font-weight: 600; line-height: 1; white-space: nowrap; }
  /* acceptEdits + auto read as "edits flow" → accent; bypass → warn; rest neutral. */
  .perm-pill[data-mode="acceptEdits"], .perm-pill[data-mode="auto"] {
    color: var(--accent); border-color: color-mix(in oklab, var(--accent) 38%, var(--border));
    background: color-mix(in oklab, var(--accent) 11%, transparent);
  }
  .perm-pill[data-mode="acceptEdits"] > :global(svg:first-child),
  .perm-pill[data-mode="auto"] > :global(svg:first-child),
  .perm-pill[data-mode="acceptEdits"] :global(.pill-chev),
  .perm-pill[data-mode="auto"] :global(.pill-chev) { color: var(--accent); }
  .perm-pill[data-mode="bypassPermissions"] {
    color: var(--warn); border-color: color-mix(in oklab, var(--warn) 42%, var(--border));
    background: color-mix(in oklab, var(--warn) 10%, transparent);
  }
  .perm-pill[data-mode="bypassPermissions"] > :global(svg:first-child),
  .perm-pill[data-mode="bypassPermissions"] :global(.pill-chev) { color: var(--warn); }

  :global(.perm-menu) {
    position: fixed; width: 252px; padding: 5px;
    background: color-mix(in oklch, var(--surface) 86%, transparent);
    backdrop-filter: blur(16px) saturate(135%);
    -webkit-backdrop-filter: blur(16px) saturate(135%);
    border: 1px solid color-mix(in oklch, var(--border) 80%, transparent);
    border-radius: 14px;
    box-shadow:
      0 18px 44px -8px oklch(0 0 0 / 0.55),
      0 0 0 1px color-mix(in oklab, var(--accent) 6%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    z-index: 9998;
    animation: hint-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
    transform-origin: bottom left;
  }
  :global(.perm-menu .mm-head) {
    display: flex; align-items: center; gap: 7px;
    font-size: 9.5px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em;
    color: var(--fg-faint); padding: 4px 8px 6px;
  }
  :global(.perm-menu .perm-kbd) {
    font-family: var(--font-mono); font-size: 9.5px; color: var(--fg-muted);
    background: var(--bg-inset); border: 1px solid var(--border); border-radius: 4px;
    padding: 1px 5px; text-transform: none; letter-spacing: 0;
  }
  :global(.perm-menu .perm-row) {
    position: relative; display: flex; align-items: flex-start; gap: 9px; width: 100%;
    padding: 6px 8px; border-radius: 7px; border: 0; background: transparent;
    color: var(--fg-2); cursor: pointer; font: inherit; text-align: left;
    transition: background 120ms;
  }
  :global(.perm-menu .perm-row:hover), :global(.perm-menu .perm-row.active) { background: var(--surface-hover); }
  :global(.perm-menu .perm-row-ic) {
    width: 16px; flex-shrink: 0; display: inline-flex; align-items: center; justify-content: center;
    color: var(--fg-subtle); margin-top: 1px; transition: color 130ms ease;
  }
  :global(.perm-menu .perm-row:hover .perm-row-ic), :global(.perm-menu .perm-row.active .perm-row-ic) { color: var(--fg-2); }
  :global(.perm-menu .perm-row-tt) { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  :global(.perm-menu .perm-row-t) { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); line-height: 1.25; }
  :global(.perm-menu .perm-row-d) { font-size: 10.5px; color: var(--fg-subtle); line-height: 1.3; }
  :global(.perm-menu .perm-row-chk) { color: var(--accent); flex-shrink: 0; margin-top: 1px; }
  /* current row: accent left-bar + accent icon, no slab (matches mock) */
  :global(.perm-menu .perm-row.current::before) {
    content: ""; position: absolute; left: 0; top: 6px; bottom: 6px; width: 2.5px;
    border-radius: 0 3px 3px 0; background: var(--accent); box-shadow: 0 0 8px var(--ring);
  }
  :global(.perm-menu .perm-row.current .perm-row-ic) { color: var(--accent); }
  :global(.perm-menu .perm-row[data-mode="bypassPermissions"].current::before) { background: var(--warn); box-shadow: 0 0 8px color-mix(in oklab, var(--warn) 55%, transparent); }
  :global(.perm-menu .perm-row[data-mode="bypassPermissions"].current .perm-row-ic),
  :global(.perm-menu .perm-row[data-mode="bypassPermissions"].current .perm-row-chk) { color: var(--warn); }
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

  /* Unified settings panel — flat single-column list (Claude-Code-Desktop
     layout) on the shared .rift-menu chrome: model rows, a fast-mode toggle,
     and a Faster↔Smarter effort slider. Right-anchored, content-width. */
  .settings-menu {
    position: absolute;
    bottom: calc(100% + 8px);
    left: auto; right: 0;
    width: 320px;
    max-height: min(82vh, 600px);
    overflow-y: auto;
    z-index: 10;
    animation: slash-in 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  /* Model row — name + muted suffix on one line, number shortcut / ✓ trailing. */
  .model-row { align-items: center; gap: 8px; }
  .model-row .model-row-name { flex: 0 0 auto; }
  .model-suffix {
    font-size: 11px; font-weight: 500; color: var(--fg-subtle);
    margin-right: auto;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .model-suffix.legacy { color: var(--fg-faint); }
  /* Limited-run row (Fable) — accent name + uppercase "until" chip. */
  .model-row-name.limited { color: var(--accent); }
  .model-badge {
    flex-shrink: 0;
    font-size: 9px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase;
    line-height: 1; padding: 3px 6px; border-radius: 999px;
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 14%, transparent);
    border: 1px solid color-mix(in oklab, var(--accent) 35%, transparent);
  }
  .model-row.current .model-suffix { color: color-mix(in oklab, var(--accent) 65%, var(--fg-muted)); }
  .model-num {
    flex-shrink: 0;
    font-family: var(--font-mono); font-size: 10px; font-weight: 600; line-height: 1;
    color: var(--fg-faint); background: var(--bg-inset);
    border: 1px solid var(--border); border-radius: 4px; padding: 2px 5px;
  }
  .model-row:hover .model-num, .model-row.active .model-num { color: var(--fg-muted); }

  /* Fast-mode toggle row. */
  .toggle-row { align-items: center; }
  .rift-toggle {
    position: relative; flex-shrink: 0; align-self: center;
    width: 30px; height: 17px; border-radius: 999px;
    background: color-mix(in oklch, var(--fg-faint) 38%, transparent);
    border: 1px solid var(--border);
    transition: background 160ms ease, border-color 160ms ease;
  }
  .rift-toggle.on { background: var(--accent); border-color: transparent; }
  .rift-toggle-knob {
    position: absolute; top: 1px; left: 1px;
    width: 13px; height: 13px; border-radius: 999px;
    background: oklch(0.97 0 0);
    box-shadow: 0 1px 2px oklch(0 0 0 / 0.4);
    transition: transform 160ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .rift-toggle.on .rift-toggle-knob { transform: translateX(13px); }

  /* Effort slider — Faster↔Smarter, stops = EFFORT_OPTIONS levels. */
  .effort-head {
    display: flex; align-items: center;
    padding: 6px 8px 3px;
    font-size: 11px; color: var(--fg-muted);
  }
  .effort-head .effort-head-l { letter-spacing: 0.01em; }
  .effort-head .effort-head-l b {
    color: var(--fg); font-weight: 650; margin-left: 2px;
    transition: color 180ms ease;
  }
  .effort-head.ultra .effort-head-l b { color: var(--accent); }
  .effort-head .effort-help {
    margin-left: auto; display: inline-flex; padding: 2px; border: 0;
    background: transparent; color: var(--fg-faint); cursor: help;
    transition: color 140ms ease;
  }
  .effort-head .effort-help:hover { color: var(--fg-muted); }
  .effort-slider {
    padding: 13px 16px 8px; margin: 0 2px; border-radius: 11px;
    transition: background 160ms ease, box-shadow 160ms ease;
  }
  .effort-slider.active {
    background: var(--surface-hover);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .effort-track {
    position: relative; height: 5px; border-radius: 999px;
    background: color-mix(in oklch, var(--fg-faint) 24%, transparent);
    box-shadow: inset 0 1px 2px oklch(0 0 0 / 0.28);
    cursor: grab; touch-action: none;
  }
  /* Invisible vertical hit-area so the 5px track is easy to grab. */
  .effort-track::before { content: ""; position: absolute; inset: -11px 0; }
  .effort-slider.dragging .effort-track { cursor: grabbing; }
  .effort-fill {
    position: absolute; left: 0; top: 0; height: 100%; border-radius: 999px;
    background: linear-gradient(
      90deg,
      color-mix(in oklab, var(--accent) 68%, var(--fg-faint)),
      var(--accent)
    );
    box-shadow: 0 0 8px color-mix(in oklab, var(--accent) 42%, transparent);
    transition: width 260ms cubic-bezier(0.22, 1, 0.36, 1),
                background 220ms ease, box-shadow 220ms ease;
  }
  .effort-slider.ultra .effort-fill {
    background: linear-gradient(90deg, color-mix(in oklab, var(--accent) 68%, var(--fg-faint)), var(--accent));
    box-shadow: 0 0 11px color-mix(in oklab, var(--accent) 55%, transparent);
  }
  .effort-notch {
    position: absolute; top: 50%; width: 9px; height: 9px; padding: 0;
    transform: translate(-50%, -50%);
    border-radius: 999px; border: 1.5px solid var(--border-strong);
    background: var(--surface); cursor: pointer;
    transition: border-color 160ms ease, background 160ms ease,
                transform 220ms cubic-bezier(0.34, 1.4, 0.5, 1);
  }
  .effort-notch:hover { transform: translate(-50%, -50%) scale(1.3); }
  .effort-notch.on {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 24%, var(--surface));
  }
  .effort-notch.cur { transform: translate(-50%, -50%) scale(0); }
  .effort-slider.ultra .effort-notch.on { border-color: var(--accent); }
  .effort-knob {
    position: absolute; top: 50%; width: 15px; height: 15px; z-index: 2;
    transform: translate(-50%, -50%);
    border-radius: 999px;
    background: radial-gradient(circle at 35% 30%, oklch(1 0 0), var(--fg));
    border: 2px solid var(--accent);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 15%, transparent),
                0 2px 5px oklch(0 0 0 / 0.4);
    transition: left 260ms cubic-bezier(0.34, 1.4, 0.5, 1),
                border-color 220ms ease, box-shadow 220ms ease;
    pointer-events: none;
  }
  .effort-slider.dragging .effort-fill { transition: background 220ms ease, box-shadow 220ms ease; }
  .effort-slider.dragging .effort-knob { transition: border-color 220ms ease, box-shadow 220ms ease; }
  .effort-slider.active .effort-knob {
    box-shadow: 0 0 0 5px color-mix(in oklab, var(--accent) 22%, transparent),
                0 2px 6px oklch(0 0 0 / 0.45);
  }
  .effort-slider.ultra .effort-knob {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 22%, transparent),
                0 0 11px color-mix(in oklab, var(--accent) 55%, transparent),
                0 2px 5px oklch(0 0 0 / 0.45);
  }
  .effort-ends {
    display: flex; justify-content: space-between;
    margin-top: 9px; font-size: 9px; font-weight: 600;
    letter-spacing: 0.06em; text-transform: uppercase; color: var(--fg-faint);
  }
  /* Plain-language "what you're getting" line under the effort slider. Amber on
     the Ultracode tier to flag its higher cost / autonomous behavior. */
  .model-caption {
    margin: 8px 10px 2px; padding: 0;
    font-size: 10.5px; line-height: 1.4; color: var(--fg-muted);
    transition: color 180ms ease;
  }
  .model-caption.warn { color: var(--warn); }
  @media (prefers-reduced-motion: reduce) {
    .effort-fill, .effort-knob, .effort-notch { transition: none; }
  }

  /* Glanceable pill marker when ultracode is the active tier. */
  /* Glanceable pill marker when ultracode is the active tier. */
  .pill-ultra {
    display: inline-flex; align-items: center;
    color: var(--accent);
    filter: drop-shadow(0 0 4px color-mix(in oklab, var(--accent) 55%, transparent));
    animation: ultra-pulse 2.6s ease-in-out infinite;
  }
  .settings-pill.ultra {
    border-color: color-mix(in oklab, var(--accent) 42%, var(--border));
  }
  @keyframes ultra-pulse {
    0%, 100% { opacity: 1; }
    50%      { opacity: 0.62; }
  }
  @media (prefers-reduced-motion: reduce) {
    .pill-ultra { animation: none; }
  }

</style>
