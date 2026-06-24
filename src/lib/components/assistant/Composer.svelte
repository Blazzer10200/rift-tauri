<script lang="ts">
  import { Send, Square, X, Mic, Loader2, Wand2, Paperclip,
    Sparkles, Eye, ChevronUp, Undo2, Cpu } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { localLlm } from "../../state/localLlm.svelte";
  import { workspace } from "../../state/workspace.svelte";
  import { notify } from "../../state/toast.svelte";
  import type { PermissionMode } from "../../state/assistant/types";
  import Markdown from "./Markdown.svelte";
  import { modelFamily } from "../../state/assistant/helpers";
  import { fuzzyScore, isFileDrag, attachImageFiles, summarizeAttach, attachTextFiles, summarizeTextAttach } from "./composer/helpers";
  import { quickStartsFor } from "./composer/quickStarts";
  import AttachmentsRow from "./composer/AttachmentsRow.svelte";
  import QueueRail from "./composer/QueueRail.svelte";
  import LivePills from "./composer/LivePills.svelte";
  import EnhanceBar from "./composer/EnhanceBar.svelte";
  import SlashMenu from "./composer/SlashMenu.svelte";
  import UsagePanel from "./composer/UsagePanel.svelte";
  import MentionPopover from "./composer/MentionPopover.svelte";
  import SettingsMenu from "./composer/SettingsMenu.svelte";
  import CtxRing from "./composer/CtxRing.svelte";
  import PermMenu from "./composer/PermMenu.svelte";
  import {
    EFFORT_OPTIONS, MODEL_OPTIONS, MODE_OPTIONS,
    effortStopsFor, clampEffortIdx, permToneFor,
    type ModelOpt, type ModeOpt,
  } from "./composer/modelMatrix";
  import { stt } from "../../state/stt.svelte";
  import { uiPrefs } from "../../state/ui-prefs.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { tick, onMount, onDestroy, untrack } from "svelte";

  // Mic-button visibility binds to stt.config.enabled, so load the backend
  // stt config eagerly — otherwise users with STT enabled wouldn't see the
  // mic until they opened Settings → Speech once.
  onMount(() => { void stt.init(); void localLlm.refresh(); });

  // RR2 unmount hygiene — the Composer is destroyed when its tab/split-pane
  // closes (parent gates rendering on tab presence). Without this, pending
  // timers fire on torn-down $state, a PTT hold leaves the mic recording on the
  // global stt singleton, and an in-flight enhance keeps billing a Haiku spawn.
  onDestroy(() => {
    if (steerFlashTimer) clearTimeout(steerFlashTimer);
    if (undoTimer) clearTimeout(undoTimer);
    pttRelease();
    // RR10: pttRelease only stops a held PTT; in tap-to-toggle mode pttActive is
    // false while recording, so closing this tab would orphan the live mic
    // (recording=true, events firing into a destroyed tab). Stop any recording
    // this tab owns.
    if (stt.recording && stt.targetTabId === tabId) void stt.cancel();
    // RR9: bump the seq UNCONDITIONALLY so any pending enhance callback (incl.
    // one still in the network round-trip before onRequestId populated the id)
    // is invalidated; cancel the backend subprocess only once we have the id.
    // The old `enhancing && enhanceRequestId` guard skipped cancellation in the
    // window between `enhancing = true` and the async onRequestId callback,
    // leaking a billed Haiku spawn when a split-pane closed mid-init.
    if (enhancing) {
      enhanceSeq++;
      if (enhanceRequestId) assistant.cancelEnhance(enhanceRequestId);
    }
  });

  let {
    onsubmit,
    tabId = null,
    hero = false,
  }: {
    onsubmit: (text: string) => void;
    tabId?: string | null;
    hero?: boolean;
  } = $props();

  // Hero-mode quick-starts — stack-aware launchpad chips that sit above the
  // input on the home surface. Reads the workspace-file walk that
  // AssistantWelcome (always mounted alongside the hero composer) kicks off, so
  // there's a single writer of `workspaceFiles`.
  const quickStarts = $derived(quickStartsFor(assistant.workspaceFiles));
  function pickChip(prompt: string) {
    setDraft(prompt);
    void tick().then(() => { ta?.focus(); autosize(); });
  }

  // Per-pane composer: bind to THIS tab's draft/attachments/queue/streaming
  // rather than the focused-pane shims, so two panes can compose & stream
  // concurrently. Tab can be null transiently (empty pane during drag/drop);
  // the parent (AssistantPane) gates Composer rendering on tab presence.
  const tab = $derived(assistant.tabFor(tabId));
  const draft = $derived(tab?.draft ?? "");
  const attachments = $derived(tab?.attachments ?? []);
  const textAttachments = $derived(tab?.textAttachments ?? []);
  const queue = $derived(tab?.queue ?? []);
  const streaming = $derived(tab?.streaming ?? false);

  // Pending rail (queue chips + steer/clear) extracted to composer/QueueRail.svelte
  // (C3) — `steer()`/`steerFlash` stay here and flow down as props.

  // Live-activity pills + idle kbd-hint (the toolbar's middle slot) extracted
  // to composer/LivePills.svelte (C4) — incl. the 1s `now` ticker.

  function setDraft(v: string) { if (tab) tab.draft = v; }
  function setAttachments(
    v: { id: string; mime: string; dataBase64: string; sizeBytes: number }[],
  ) {
    if (tab) tab.attachments = v;
  }

  let ta = $state<HTMLTextAreaElement | undefined>();
  // Tracks whether the input has grown past one line — flips the well to
  // bottom-align so the inline send arrow rides the textarea's last line.
  let multiline = $state(false);
  let atMaxHeight = $state(false);

  type SlashCmd = { name: string; desc: string };
  // Grouped: conversation lifecycle → model + composition → flow control → info.
  const SLASH_COMMANDS: SlashCmd[] = [
    { name: "new",       desc: "Start a new conversation (saves current)" },
    { name: "clear",     desc: "Clear this chat in place (saves current to History)" },
    { name: "model",     desc: "Switch model — opens picker" },
    { name: "retry",     desc: "Re-fire the last prompt" },
    { name: "copy",      desc: "Copy last response to clipboard" },
    { name: "stop",      desc: "Halt the current turn" },
    { name: "tools",     desc: "List available workspace tools" },
    { name: "cost",      desc: "Show session cost" },
    { name: "usage",     desc: "Plan limits — 5-hour & weekly windows" },
    { name: "stats",     desc: "Session telemetry summary (inline)" },
    { name: "openincli", desc: "Print the claude --resume command for this session" },
    { name: "diag",      desc: "Copy full telemetry JSON to clipboard" },
    { name: "help",      desc: "List slash commands" },
    // CLI passthrough (Claude Design) — runSlash doesn't match these, so they
    // ride straight to the Claude CLI as skills. Listed here for discoverability.
    { name: "design-sync",  desc: "Sync this workspace with a claude.ai/design project" },
    { name: "design-login", desc: "Authorize Claude Design access (terminal sessions only)" },
  ];

  // Model picker rows — version + tagline + context window. The CLI takes
  // the alias (`sonnet`/`opus`/`haiku`); version is display-only and pulled
  // from CLAUDE.md's source-of-truth section on the current model family.
  // Model / effort / permission-mode option tables + pure helpers live in
  // composer/modelMatrix.ts (C7) — shared with SettingsMenu + PermMenu so the
  // keyboard nav here and the rendered rows there can never disagree.

  // Idle placeholder — static ghost with `/` `@` keycaps (mock `.ph-ghost`).
  let composerFocused = $state(false);

  function autosize() {
    if (!ta) return;
    ta.style.height = "auto";
    const h = Math.min(ta.scrollHeight, 340);
    ta.style.height = h + "px";
    multiline = h > 40;
    // Only allow the inner scrollbar once content actually hits the cap —
    // otherwise `overflow:auto` paints a phantom gutter in the idle composer.
    atMaxHeight = ta.scrollHeight > 340;
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
  // RR9: this Composer instance persists across tabId changes (AssistantPane
  // renders it un-keyed), but mentionState holds a character offset into the
  // PREVIOUS tab's draft. Switching tabs with the @-popover open + then picking
  // a mention spliced at a stale offset into the new tab's draft, corrupting it.
  // Clear transient per-draft popover state whenever the active tab changes.
  $effect(() => {
    void tabId;
    mentionState = null;
  });
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
  const currentModel = $derived(MODEL_OPTIONS.find((m) => m.id === assistant.effectiveModel));

  // Effort derives the parent still needs (pill label, settingsRows, onKey
  // ←/→) — same matrix helpers SettingsMenu uses, so they can't drift.
  const currentEffort = $derived(EFFORT_OPTIONS.find((e) => e.id === assistant.thinkingEffort) ?? EFFORT_OPTIONS[2]);
  const effortApplies = $derived(currentModel?.effort ?? true);
  const effortStops = $derived(effortStopsFor(currentModel));
  const effortIdx = $derived(
    Math.min(
      Math.max(0, EFFORT_OPTIONS.findIndex((e) => e.id === assistant.thinkingEffort)),
      Math.max(0, effortStops.length - 1),
    ),
  );
  function setEffortByIdx(i: number) {
    const c = clampEffortIdx(effortStops, i);
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
  // Caption + pointer-drag slider live in composer/SettingsMenu.svelte (C7).

  // Permission-mode picker — option table in modelMatrix.ts (C7).
  const currentMode = $derived(MODE_OPTIONS.find((m) => m.id === assistant.permissionMode) ?? MODE_OPTIONS[4]);
  const PermIcon = $derived(currentMode.icon);
  // Flat-bar perm button tone — shared with the PermMenu rows (permToneFor) so
  // the bar pill + popover can't disagree. Drives `.cbtn.cperm.tone-*`.
  const permTone = $derived(permToneFor(currentMode.id));
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
    | { kind: "effort" };
  const settingsRows = $derived.by<SettingsRow[]>(() => {
    const rows: SettingsRow[] = MODEL_OPTIONS.map((m) => ({ kind: "model" as const, model: m }));
    if (effortApplies && effortStops.length > 0) rows.push({ kind: "effort" });
    return rows;
  });
  // Re-seed the cursor to the current model row whenever the panel opens.
  $effect(() => {
    if (settingsOpen) {
      const i = settingsRows.findIndex((r) => r.kind === "model" && r.model.id === assistant.effectiveModel);
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
    // Claude Design rides the user's claude.ai login — it can't authenticate
    // under local-LLM mode (--bare strips the cloud session). Warn instead of
    // firing a turn that's doomed to fail at the design OAuth step.
    if (c.name.startsWith("design-") && localLlm.enabled) {
      notify.warn("Claude Design needs cloud Claude", {
        detail: "Turn off local-LLM mode in Settings to sync with claude.ai/design.",
      });
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
      notify.danger("Claude isn't set up", {
        detail: assistant.auth?.summary ?? "Open Settings to sign in or add an API key.",
      });
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
    // Snapshot attachments before clearing (pass with the steer).
    const steerAttachments = attachments.map((a) => ({ mime: a.mime, dataBase64: a.dataBase64 }));
    stt.consume();
    // Clear draft + attachments only after the IPC resolves (defect 2 + 3).
    void assistant.steer(text, tabId, steerAttachments.length > 0 ? steerAttachments : undefined).then((result) => {
      if (result === "steered") {
        setDraft("");
        setAttachments([]);
        steerFlash = true;
        if (steerFlashTimer) clearTimeout(steerFlashTimer);
        steerFlashTimer = setTimeout(() => { steerFlash = false; }, 1400);
      }
      // On "queued"/"no_active_turn" the text/attachments stay so the user
      // can see what was queued / retry.
    });
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
  // Grounded-lookup progress ("Reading src/…") + cost footer from the backend.
  let enhanceStatus = $state<string | null>(null);
  let enhanceMeta = $state<{ costUsd: number | null; durationMs: number | null } | null>(null);
  // In-flight request id — Discard kills the actual CLI spawn through it.
  let enhanceRequestId: string | null = null;
  // Restore-point after Accept (the raw draft we enhanced from). Cleared on
  // typing or after a grace window.
  let undoDraft = $state<string | null>(null);
  let undoTimer: ReturnType<typeof setTimeout> | undefined;
  // Opt-in: let the rewrite read the real workspace (read-only). Slower, more
  // specific. Explicit choice persists in localStorage; until the user touches
  // the toggle, code-anchored drafts (paths/symbols) auto-enable it per run.
  const GROUND_KEY = "rift.enhanceGround";
  let groundEnhance = $state(localStorage.getItem(GROUND_KEY) === "1");
  let groundTouched = localStorage.getItem(GROUND_KEY) !== null;
  function toggleGround() {
    groundEnhance = !groundEnhance;
    groundTouched = true;
    localStorage.setItem(GROUND_KEY, groundEnhance ? "1" : "0");
  }
  const CODE_ANCHOR_RE =
    /[\w-]+\.(rs|ts|tsx|js|jsx|svelte|py|css|html|json|toml|ya?ml|md)\b|src\/|src-tauri|\w+::\w+|\w+\(\)/;
  // Draft preview (eye) — render the composer draft as Markdown before sending.
  let previewing = $state(false);
  // Preview panel markup + word-stagger render live in composer/EnhanceBar.svelte
  // (C5); the state machine stays here (wand button + onKey Escape/Ctrl+E drive
  // it). `directive` steers a refine pass (chips or freeform); omitted for the
  // first run + plain Regenerate.
  // Generation token: accept/dismiss bumps it so a still-in-flight enhance
  // can't write its stream/result back into a closed preview.
  let enhanceSeq = 0;
  // Conversation tail for the rewrite — resolves mid-thread references ("that
  // bug", "the same file") into the names the conversation established. Text
  // blocks only; per-message + total caps keep the arg small.
  function buildEnhanceContext(): string | undefined {
    const msgs = tab?.messages ?? [];
    const parts: string[] = [];
    let total = 0;
    for (let i = msgs.length - 1; i >= 0 && parts.length < 8 && total < 3000; i--) {
      const m = msgs[i];
      if (m.role === "system") continue;
      const text = m.blocks
        .map((b) => (b.type === "text" ? b.text : ""))
        .filter(Boolean)
        .join("\n")
        .trim();
      if (!text) continue;
      const clipped = text.length > 600 ? `${text.slice(0, 600)} …` : text;
      parts.unshift(`${m.role}: ${clipped}`);
      total += clipped.length;
    }
    return parts.length ? parts.join("\n\n") : undefined;
  }
  async function runEnhance(directive?: string) {
    const text = (enhanceOriginal ?? draft).trim();
    if (!text || enhancing) return;
    if (enhanceOriginal === null) {
      enhanceOriginal = text;
      if (!groundTouched && !groundEnhance && !!assistant.workspace.current && CODE_ANCHOR_RE.test(text)) {
        groundEnhance = true;
      }
    }
    // A directive edits the current rewrite (iterative); Regenerate re-rolls
    // fresh from the original.
    const previous = directive && enhancedPreview ? enhancedPreview : undefined;
    const seq = ++enhanceSeq;
    enhancing = true;
    enhanceError = null;
    enhancedPreview = "";
    enhanceStatus = null;
    enhanceMeta = null;
    try {
      // Stream: deltas fill the preview live; the resolved value is the
      // authoritative final text. Grounded mode passes the workspace cwd.
      const result = await assistant.enhancePrompt(
        text,
        (full) => { if (seq === enhanceSeq) { enhancedPreview = full; enhanceStatus = null; } },
        {
          directive,
          previous,
          context: buildEnhanceContext(),
          cwd: groundEnhance ? (assistant.workspace.current ?? undefined) : undefined,
          onRequestId: (id) => { if (seq === enhanceSeq) enhanceRequestId = id; },
          onStatus: (s) => { if (seq === enhanceSeq) enhanceStatus = s; },
          onMeta: (m) => { if (seq === enhanceSeq) enhanceMeta = m; },
        },
      );
      if (seq === enhanceSeq) enhancedPreview = result;
    } catch (e) {
      if (seq === enhanceSeq) {
        enhanceError = String(e);
        enhancedPreview = null;
      }
    } finally {
      if (seq === enhanceSeq) {
        enhancing = false;
        enhanceRequestId = null;
        enhanceStatus = null;
      }
    }
  }
  function acceptEnhanced() {
    if (!enhancedPreview) return;
    enhanceSeq++;
    enhancing = false;
    undoDraft = enhanceOriginal;
    clearTimeout(undoTimer);
    undoTimer = setTimeout(() => (undoDraft = null), 12000);
    setDraft(enhancedPreview);
    enhancedPreview = null;
    enhanceError = null;
    enhanceOriginal = null;
    enhanceStatus = null;
    enhanceMeta = null;
    void tick().then(() => { autosize(); ta?.focus(); });
  }
  function dismissEnhanced() {
    // Kill the actual spawn — a dismissed grounded pass otherwise runs (and
    // bills) to completion in the background.
    if (enhancing && enhanceRequestId) assistant.cancelEnhance(enhanceRequestId);
    enhanceRequestId = null;
    enhanceSeq++;
    enhancing = false;
    enhancedPreview = null;
    enhanceError = null;
    enhanceOriginal = null;
    enhanceStatus = null;
    enhanceMeta = null;
    void tick().then(() => ta?.focus());
  }
  function undoEnhanced() {
    if (undoDraft === null) return;
    setDraft(undoDraft);
    undoDraft = null;
    clearTimeout(undoTimer);
    void tick().then(() => { autosize(); ta?.focus(); });
  }

  // ── Dictation integration ───────────────────────────────────────────────
  // Hold-Space push-to-talk (CC CLI style): empty composer + plain Space held
  // ≥300ms starts dictation; release stops. A quick tap stays inert, and any
  // draft text disables the path so Space types spaces normally.
  let pttTimer: ReturnType<typeof setTimeout> | null = null;
  let pttActive = false;
  function pttKeydown(e: KeyboardEvent): boolean {
    if (e.key !== " ") return false;
    // Swallow auto-repeat while engaged — once words land, draft is non-empty
    // and repeats would otherwise type spaces into the transcript.
    if (pttActive || pttTimer !== null) {
      e.preventDefault();
      return true;
    }
    if (
      e.ctrlKey || e.metaKey || e.altKey || e.shiftKey || e.repeat ||
      draft.length > 0 || attachments.length > 0 || streaming ||
      !stt.config.enabled || stt.recording
    ) return false;
    e.preventDefault();
    pttTimer = setTimeout(() => {
      pttTimer = null;
      pttActive = true;
      void stt.start(tabId);
    }, 300);
    return true;
  }
  // Release listens at window level — keyup never reaches the textarea if
  // focus left the composer mid-hold (menu click, alt-tab), which used to
  // leave the mic running. Idempotent, so the bubbled textarea keyup is fine.
  function pttRelease() {
    if (pttTimer) { clearTimeout(pttTimer); pttTimer = null; }
    if (pttActive) {
      pttActive = false;
      void stt.stop();
    }
  }
  function onKeyUp(e: KeyboardEvent) {
    if (e.key !== " ") return;
    pttRelease();
  }
  // Voice command "send it" — the stt store commits the draft then raises the
  // flag; fire() runs the same path as the Send button.
  $effect(() => {
    // Only the pane the dictation was bound to fires — in split mode every
    // composer mounts this effect, so gate on the STT target tab.
    if (stt.sendRequested && stt.targetTabId === tabId) {
      // untrack the reset — writing a value this effect subscribes to would
      // schedule a spurious re-run ($state_unsafe_mutation in dev).
      untrack(() => { stt.sendRequested = false; });
      fire();
    }
  });

  // S88: mic toggle. The stt store writes recognized text into THIS tab's
  // draft (bound via stt.start(tabId) → stt.targetTabId) as it arrives
  // (interim + final), so dictation lands in the pane the mic was clicked in
  // rather than the focused-pane shim. We just start/stop and let the
  // autosizer catch up. Composer focus is restored on stop so the user can
  // hit Enter without an extra click.
  let micBusy = $state(false);
  async function toggleMic() {
    if (micBusy) return;
    micBusy = true;
    try {
      await stt.init();
      if (stt.recording) {
        await stt.stop();
        void tick().then(() => { autosize(); ta?.focus(); });
      } else {
        await stt.start(tabId);
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
  // Permission-mode menu portals to <body> — positioning + outside-mousedown
  // close live in composer/PermMenu.svelte (C7); permWrap anchors it.
  let permWrap = $state<HTMLButtonElement | null>(null);
  // Model/effort menu portals to <body> too — same anchor pattern; modelWrap
  // is the trigger pill it positions against.
  let modelWrap = $state<HTMLButtonElement | null>(null);

  let attachError = $state<string | null>(null);
  async function onPaste(e: ClipboardEvent) {
    const items = e.clipboardData?.items;
    if (!items) return;
    // Only engage on image files — a non-image clipboard payload falls through
    // to the normal text paste.
    const imageFiles = Array.from(items)
      .filter((it) => it.kind === "file" && it.type.startsWith("image/"))
      .map((it) => it.getAsFile())
      .filter((f): f is File => f != null);
    if (imageFiles.length === 0) return;
    e.preventDefault();
    const res = await attachImageFiles(imageFiles, (a) => assistant.addAttachment(a, tabId));
    attachError = summarizeAttach(res);
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
    if (undoDraft !== null && e.key === "Escape") {
      e.preventDefault();
      undoDraft = null;
      return;
    }
    // Ctrl/Cmd+E — full keyboard loop: enhance the draft; with the preview
    // settled, accept it.
    if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "e") {
      e.preventDefault();
      if (enhancing) return;
      if (enhancedPreview) acceptEnhanced();
      else void runEnhance();
      return;
    }
    if (pttKeydown(e)) return;
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
      // Slash menu open with no match (e.g. "/zzz") — swallow Enter instead of
      // firing it as a real chat turn.
      if (slashOpen) return;
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
  // RR9: Chromium empties dataTransfer.types on a drag cancelled OUTSIDE the
  // page (Escape, alt-tab, mouse-up off-window), so isFileDrag() returns false
  // and onDragLeave's early-return never drains the counter → the "Drop image"
  // overlay sticks until unmount. A document-level dragend unconditionally
  // resets it.
  $effect(() => {
    const reset = () => { dragDepth = 0; };
    document.addEventListener("dragend", reset);
    return () => document.removeEventListener("dragend", reset);
  });
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
  // Stage a dropped/picked file set: images → binary attachments, everything
  // else → inlined text attachments. Each helper skips the other's files, so
  // running both over the same set partitions cleanly. Merges both notices.
  async function stageFiles(files: Iterable<File>) {
    const imgRes = await attachImageFiles(files, (a) => assistant.addAttachment(a, tabId));
    const txtRes = await attachTextFiles(files, (a) => assistant.addTextAttachment(a, tabId));
    attachError = [summarizeAttach(imgRes), summarizeTextAttach(txtRes)].filter(Boolean).join(" · ") || null;
  }
  async function onDrop(e: DragEvent) {
    if (!isFileDrag(e)) return;
    e.preventDefault();
    e.stopPropagation(); // handled here — don't let the window-level guard re-attach
    dragDepth = 0;
    const files = e.dataTransfer?.files;
    if (!files || files.length === 0) return;
    await stageFiles(files);
  }

  // ── Click-to-attach ───────────────────────────────────────────────────────
  // Paste + drag-drop already stage attachments; this adds the discoverable
  // path the placeholder has long advertised. Same staging + caps as onDrop.
  let fileInput = $state<HTMLInputElement | undefined>();
  function openFilePicker() { fileInput?.click(); }
  async function onFilePick(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = input.files;
    if (!files || files.length === 0) return;
    await stageFiles(files);
    input.value = ""; // allow re-picking the same file
  }
</script>

<svelte:window onkeyup={onKeyUp} onblur={pttRelease} />

<div class="composer-wrap" data-model={modelFamily(assistant.effectiveModel)}>
  <QueueRail
    tab={tab ?? null}
    {tabId}
    {queue}
    {streaming}
    {steerFlash}
    {draft}
    onSteer={steer}
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
    <EnhanceBar
      {enhancing}
      {enhancedPreview}
      {enhanceError}
      {enhanceOriginal}
      {enhanceStatus}
      {enhanceMeta}
      {groundEnhance}
      hasWorkspace={!!assistant.workspace.current}
      undoAvailable={undoDraft !== null}
      onToggleGround={toggleGround}
      onAccept={acceptEnhanced}
      onDismiss={dismissEnhanced}
      onRefine={(directive) => void runEnhance(directive)}
      onEditPreview={(text) => (enhancedPreview = text)}
      onUndo={undoEnhanced}
    />

    {#if stt.polishUndo}
      <div class="dictate-undo" role="region" aria-label="Transcript cleaned">
        <Sparkles size={12} />
        <span class="du-label">Cleaned up</span>
        <button type="button" class="du-btn" onclick={() => { stt.revertPolish(); void tick().then(autosize); }} use:tooltip={"Restore the raw transcript"}>
          <Undo2 size={12} /> Show raw
        </button>
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
      <SlashMenu commands={slashFiltered} activeIdx={slashIdx} query={draft.slice(1).toLowerCase()} onPick={pickSlash} />
    {/if}

    {#if assistant.ui.usageOpen}
      <UsagePanel onClose={() => (assistant.ui.usageOpen = false)} />
    {/if}

    {#if mentionState && mentionResults.length > 0}
      <MentionPopover
        results={mentionResults}
        activeIdx={mentionIdx}
        fileCount={assistant.workspaceFiles.length}
        onPick={pickMention}
      />
    {/if}

    {#if hero && draft.length === 0 && !streaming && attachments.length === 0}
      <div class="quick-chips">
        {#each quickStarts as s (s.title)}
          <button class="quick-chip" type="button" onclick={() => pickChip(s.prompt)}>
            <span class="qc-ic"><s.icon size={14}/></span>{s.title}
          </button>
        {/each}
      </div>
    {/if}

    <div class="composer" class:hero={hero} class:streaming={streaming} class:enchanting={enhancing} data-mode={mode}>
      <!-- WELL: attachments + input + inline send arrow (Claude-Code style).
           All chrome (border/glass/focus-ring/streaming edge) lives here now. -->
      <div class="composer-box" class:multiline={multiline}>
      <AttachmentsRow
        {attachments}
        {textAttachments}
        {attachError}
        onRemove={(id) => assistant.removeAttachment(id, tabId)}
        onRemoveText={(id) => assistant.removeTextAttachment(id, tabId)}
        onDismissError={() => (attachError = null)}
      />
      <div class="cbox-row">
      <div class="textarea-wrap" class:polishing={stt.polishing}>
        <textarea
          bind:this={ta}
          value={draft}
          oninput={(e) => {
            setDraft((e.currentTarget as HTMLTextAreaElement).value);
            undoDraft = null;
            stt.dismissPolishUndo();
            stt.cancelPolish();
            resetRecall(); autosize(); refreshMention();
          }}
          onkeyup={(e) => { if (e.key.startsWith("Arrow") || e.key === "Home" || e.key === "End") refreshMention(); }}
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
          class:scrollable={atMaxHeight}
          rows="1"
        ></textarea>
        {#if hero && draft.length === 0 && !streaming && attachments.length === 0}
          <span class="placeholder-ghost static" aria-hidden="true">What are we working on today?</span>
        {:else if draft.length === 0 && !streaming && attachments.length === 0}
          <span class="placeholder-ghost static" aria-hidden="true">Ask {localLlm.askLabel} · <span class="ph-k">/</span> for commands · <span class="ph-k">@</span> to mention a file{#if stt.config.enabled} · hold <span class="ph-k">Space</span> to talk{/if}</span>
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

        <!-- Inline send — bare arrow riding the trailing edge of the well.
             Same multi-mode action button (send / queue / stop), restyled. -->
        <button
          class="send-inline"
          class:ready={canFire && mode !== "stop"}
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

      <!-- FLAT control bar below the well (Claude-Code style): perm + tool
           icons on the left, live pills in the middle, model pill on the right.
           No border, no internal ctx-gauge hairline. -->
      <div class="composer-bar">
        <div class="cbar-l">
          <button
            type="button"
            class="cbtn cperm"
            class:open={permOpen}
            class:tone-ok={permTone === "ok"}
            class:tone-warn={permTone === "warn"}
            class:tone-info={permTone === "info"}
            bind:this={permWrap}
            onclick={() => { permOpen = !permOpen; settingsOpen = false; void tick().then(() => ta?.focus()); }}
            aria-haspopup="listbox"
            aria-expanded={permOpen}
            aria-label="Permission mode"
            use:tooltip={{ text: `Permission mode — ${currentMode.label}`, kbd: "⇧Tab" }}
          >
            <PermIcon size={13} />
            <span class="perm-label">{currentMode.short}</span>
            <ChevronUp size={12} class="cbtn-chev" />
          </button>

          {#if permOpen}
            <PermMenu
              {permIdx}
              anchor={permWrap}
              onPick={pickMode}
              onRequestClose={() => (permOpen = false)}
            />
          {/if}

          <input
            bind:this={fileInput}
            type="file"
            multiple
            class="file-input-hidden"
            onchange={onFilePick}
            tabindex="-1"
            aria-hidden="true"
          />
          <button
            type="button"
            class="cbtn ic attachbtn"
            onclick={openFilePicker}
            use:tooltip={"Attach a file or image — or paste / drag-drop"}
            aria-label="Attach a file"
          >
            <Paperclip size={15} />
          </button>
          {#if stt.config.enabled && (
            (stt.config.engine === "web_speech" && stt.supported) ||
            (stt.config.engine === "whisper" && stt.backendAvailable)
          )}
          <button
            class="cbtn ic micbtn"
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
              <Loader2 size={15} class="mic-spin" />
            {:else if stt.recording}
              <span class="mic-wave" aria-hidden="true">
                <span></span><span></span><span></span>
              </span>
            {:else}
              <Mic size={15} />
            {/if}
          </button>
          {/if}
          {#if draft.trim().length > 0}
          <button
            class="cbtn ic enhance wandbtn reveal"
            class:enhancing
            type="button"
            onclick={() => runEnhance()}
            disabled={enhancing}
            use:tooltip={enhancing ? "Enhancing…" : "Improve prompt — clean up & clarify (Ctrl+E)"}
            aria-label="Improve prompt"
          >
            <Wand2 size={15} />
          </button>
          <button
            class="cbtn ic previewbtn reveal"
            class:active={previewing}
            type="button"
            onclick={() => (previewing = !previewing)}
            aria-pressed={previewing}
            use:tooltip={previewing ? "Hide preview" : "Preview as Markdown"}
            aria-label="Preview message"
          >
            <Eye size={15} />
          </button>
          <button
            class="cbtn ic clearbtn reveal"
            type="button"
            onclick={() => { setDraft(""); ta?.focus(); }}
            use:tooltip={"Clear draft"}
            aria-label="Clear draft"
          >
            <X size={15} />
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

        <LivePills tab={tab ?? null} {queue} {streaming} {composerFocused} />

        <div class="cbar-r">
          {#if localLlm.enabled}
            <!-- Experimental local-mode indicator (cont.127). The model/effort
                 pill lies in local mode (cloud model pin is bypassed), so this
                 shows what the turn actually runs against. Click → settings. -->
            <button
              type="button"
              class="local-pill"
              onclick={() => workspace.setActive("local-llm")}
              use:tooltip={`Local LLM mode — turns run against ${localLlm.baseUrl || "your local endpoint"}\nClick to configure`}
              aria-label="Local LLM mode active — configure"
            >
              <Cpu size={12} />
              <span class="local-pill-label">{localLlm.pillLabel}</span>
            </button>
          {/if}

          <!-- Cloud model + effort picker. Hidden in local mode — local routing
               bypasses the model pin + effort entirely, so showing cloud options
               (e.g. "Opus 4.8") would misrepresent what the turn runs against.
               The local-pill above already names the active local model. -->
          {#if !localLlm.enabled}
          <button
            type="button"
            class="model-pill"
            class:open={settingsOpen}
            class:ultra={effortApplies && assistant.thinkingEffort === "ultra"}
            data-model={currentModel ? modelFamily(currentModel.id) : ""}
            bind:this={modelWrap}
            onclick={() => { settingsOpen = !settingsOpen; permOpen = false; void tick().then(() => ta?.focus()); }}
            aria-haspopup="listbox"
            aria-expanded={settingsOpen}
            aria-label="Model & thinking depth"
            use:tooltip={effortApplies
              ? `Model · thinking depth\n${currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.effectiveModel} · ${currentEffort.label}`
              : `Model\n${currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.effectiveModel} · no extended thinking`}
          >
            <span class="model-dot" aria-hidden="true"></span>
            <span class="pill-label">{currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.effectiveModel}</span>
            {#if effortApplies}
              <span class="pill-effort">· {currentEffort.label}</span>
            {/if}
            {#if effortApplies && assistant.thinkingEffort === "ultra"}
              <span class="pill-ultra" aria-hidden="true" use:tooltip={"Ultracode — max reasoning + autonomous workflows"}><Sparkles size={11} /></span>
            {/if}
            <ChevronUp size={13} class="pill-chev" />
          </button>

          {#if settingsOpen && !localLlm.enabled}
            <SettingsMenu
              {settingsIdx}
              activeKind={settingsRows[settingsIdx]?.kind ?? null}
              anchor={modelWrap}
              onPickModel={pickModel}
              onRequestClose={() => (settingsOpen = false)}
            />
          {/if}
          {/if}

          {#if !localLlm.enabled && assistant.ctxTokens > 0}
            <CtxRing
              pct={assistant.ctxPct}
              tokens={assistant.ctxTokens}
              window={assistant.ctxWindow}
              open={assistant.ui.usageOpen}
              onClick={() => (assistant.ui.usageOpen = !assistant.ui.usageOpen)}
            />
          {/if}
        </div>
      </div>
    </div>
  </div>

</div>

<style>
  .composer-wrap {
    position: relative;
    padding: 6px 18px 10px;
    max-width: min(var(--chat-col-max), 880px);
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }
  /* Composer chrome is emerald-only — the ring/divider/send/ripple no longer
     tint by model. Every accent inside reads from this single source; model
     identity lives on the model-card swatch in the picker. */
  .composer-wrap                      { --model-color: var(--accent); }
  .composer-shell { position: relative; z-index: 1; }
  .composer-shell.drag-over .composer-box {
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

  /* ── Composer v4 (Claude-Code flat bar) ──────────────────────────────
     The outer .composer is now a transparent column shell. All chrome
     (glass surface / focus ring / streaming edge) lives on the .composer-box
     WELL; controls sit on a flat .composer-bar BELOW the well. */
  .composer {
    position: relative;
    display: flex; flex-direction: column;
  }
  /* WELL — the bordered glass surface that holds attachments + input + the
     inline send arrow. */
  .composer-box {
    position: relative;
    display: flex; flex-direction: column;
    padding: 2px;
    background: color-mix(in oklch, var(--surface) 88%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid color-mix(in oklch, var(--border) 90%, transparent);
    border-radius: 14px;
    box-shadow:
      0 10px 28px -10px oklch(0 0 0 / 0.45),
      inset 0 1px 0 color-mix(in oklch, white 4%, transparent);
    transition: border-color 220ms cubic-bezier(0.22, 1, 0.36, 1),
                box-shadow 220ms cubic-bezier(0.22, 1, 0.36, 1),
                transform 140ms ease-out;
    overflow: hidden;
  }
  /* Input row inside the well: textarea grows, send arrow rides the edge. */
  .cbox-row { display: flex; align-items: center; gap: 4px; }
  .composer-box.multiline .cbox-row { align-items: flex-end; }
  /* Calm focus ring — a tight 2px tint that HUGS the well. The old soft halo
     (0 8px 22px) projected downward onto the flat toolbar below, reading as a
     stray rounded "overlay" box around the controls. Keep the glow symmetric
     and contained (0 0 16px, no Y-offset) so it traces the well, not the bar. */
  .composer-box:focus-within {
    border-color: color-mix(in oklch, var(--model-color) 45%, transparent);
    box-shadow:
      0 0 0 2px color-mix(in oklch, var(--model-color) 13%, transparent),
      0 0 16px -6px color-mix(in oklch, var(--model-color) 16%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
  }

  /* Hero mode — home surface. The composer is the centerpiece: a larger,
     more rounded card with stack-aware quick-start chips floating above it on
     the column's transparent background (mockup `.quick-chips`). */
  .quick-chips {
    display: flex; flex-wrap: wrap; gap: 7px;
    padding: 0 2px 12px;
  }
  .quick-chip {
    display: inline-flex; align-items: center; gap: 7px;
    height: 28px; padding: 0 12px 0 10px;
    border-radius: 8px;
    font: inherit; font-size: 12px; color: var(--fg-muted);
    background: transparent; border: 1px solid var(--border);
    cursor: pointer;
    transition: background var(--dur-fast, 140ms), color var(--dur-fast, 140ms),
                border-color var(--dur-fast, 140ms),
                transform var(--dur-fast, 140ms) var(--ease-page, ease-out);
  }
  .qc-ic { display: inline-flex; color: var(--fg-faint); flex: none; transition: color var(--dur-fast, 140ms); }
  .quick-chip:hover { background: var(--surface-hover); color: var(--fg-2); border-color: var(--border-strong); transform: translateY(-1px); }
  .quick-chip:hover .qc-ic { color: var(--accent); }
  .quick-chip:active { transform: translateY(0) scale(0.97); }
  @media (prefers-reduced-motion: reduce) { .quick-chip { transition: none; } }

  .composer.hero .composer-box { border-radius: 16px; padding: 3px; }
  .composer.hero textarea { font-size: 14.5px; line-height: 1.55; padding: 11px 12px 11px 14px; min-height: 30px; letter-spacing: -0.003em; }
  /* Keep the hero placeholder in lockstep with the hero textarea so the ghost
     prompt and the text you type read at the same size. */
  .composer.hero .placeholder-ghost { font-size: 14.5px; line-height: 1.55; top: 11px; left: 14px; right: 12px; }

  /* Streaming = ONE coherent signal: a thin model-tinted border + the
     animated top-edge bar below (synced 2.6s with the model-pill breathe).
     No aurora swirl, no extra glow — calm and in-sync. */
  .composer.streaming .composer-box {
    border-color: color-mix(in oklch, var(--model-color) 42%, var(--border));
  }
  /* Full-frame streaming ring — a model-tinted border that breathes around
     the entire composer (synced 2.6s with the model-pill breathe). Sits as a
     border-only overlay (transparent center, pointer-events none) so it traces
     the frame without crossing the input text. */
  .composer.streaming .composer-box::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: 14px;
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
    .composer.streaming .composer-box::before { animation: none; opacity: 0.7; }
  }

  .textarea-wrap {
    position: relative;
    z-index: 1;
    display: flex;
    flex: 1;
    min-width: 0;
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
    overflow-y: hidden;
  }
  textarea.scrollable { overflow-y: auto; }
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
  .composer-box:focus-within .placeholder-ghost { color: var(--fg-faint); }
  @keyframes placeholder-fade {
    from { opacity: 0; transform: translateY(8px); filter: blur(3px); }
    to   { opacity: 1; transform: translateY(0);   filter: blur(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .placeholder-ghost { animation: none; }
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

  /* ── Flat control bar (Claude-Code style) ─────────────────────────────
     Sits below the well, no border. Left cluster = perm + tool icons;
     LivePills float in the middle; right cluster = model pill. */
  .composer-bar {
    position: relative;
    z-index: 1;
    display: flex; align-items: center; gap: 3px;
    padding: 7px 2px 0;
  }
  .cbar-l { display: flex; align-items: center; gap: 2px; min-width: 0; }
  .cbar-r { margin-left: auto; display: flex; align-items: center; gap: 5px; position: relative; }

  /* Flat control button — transparent base; .ic = square icon variant. */
  .cbtn {
    display: inline-flex; align-items: center; gap: 6px;
    height: 30px; padding: 0 9px;
    border-radius: 8px;
    font: inherit; font-size: 12px;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid transparent;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 140ms, color 140ms, border-color 140ms, transform 140ms;
  }
  .cbtn:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg-2); }
  .cbtn:active:not(:disabled) { transform: scale(0.96); }
  .cbtn:disabled { opacity: 0.4; cursor: default; }
  .cbtn:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .cbtn.ic { width: 32px; padding: 0; justify-content: center; color: var(--fg-subtle); }
  .cbtn.ic:hover:not(:disabled) { color: var(--fg-2); }
  .cbtn.ic.active { background: var(--accent-soft); color: var(--accent); }
  .cbtn.enhance:hover:not(:disabled) { color: var(--accent); }
  :global(.cbtn .cbtn-chev) { color: var(--fg-faint); transition: color 140ms, transform 140ms; }
  /* Permission button — colored text by posture, no border (flat). */
  .cbtn.cperm { font-weight: 600; }
  .cbtn.cperm .perm-label { font-size: 12px; font-weight: 600; line-height: 1; white-space: nowrap; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .cbtn.cperm > :global(svg:first-child) { color: currentColor; flex-shrink: 0; }
  .cbtn.cperm.tone-ok   { color: var(--ok);   }
  .cbtn.cperm.tone-warn { color: var(--warn); }
  .cbtn.cperm.tone-info { color: var(--info); }
  .cbtn.cperm.tone-ok:hover:not(:disabled)   { background: var(--ok-soft);   color: var(--ok);   }
  .cbtn.cperm.tone-warn:hover:not(:disabled) { background: var(--warn-soft); color: var(--warn); }
  .cbtn.cperm.tone-info:hover:not(:disabled) { background: var(--info-soft); color: var(--info); }
  .cbtn.cperm.open :global(.cbtn-chev) { transform: rotate(180deg); }

  /* Mic — recording / transcribing states inherit .cbtn.ic base + override. */
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
  /* Live-pills + lp-dot styles moved to composer/LivePills.svelte (C4). */

  /* Send — primary CTA, accent surface w/ glow. Bigger than v2 (32px), more
     pronounced shadow, smoother mode-swap (send → stop → queue). */
  /* Inline-blend send (spec `.send-inline`): a bare arrow that lives in the
     well — subtle grey when idle, accent when ready, soft-bg on hover. No
     filled box / glow. */
  .send-inline {
    position: relative;
    width: 34px; height: 34px;
    margin: 0 4px 0 2px;
    display: flex; align-items: center; justify-content: center;
    background: transparent;
    color: var(--fg-subtle);
    border: 1px solid transparent; border-radius: 50%;
    cursor: pointer;
    flex-shrink: 0;
    overflow: hidden;
    transition: color 140ms ease-out, background 140ms ease-out, transform 140ms ease-out;
  }
  .composer-box.multiline .send-inline { align-self: flex-end; margin-bottom: 5px; }
  .send-inline:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg-2); }
  .send-inline:active:not(:disabled) { transform: scale(0.88); }
  .send-inline:disabled { cursor: default; color: var(--fg-subtle); }
  .send-inline:focus-visible { outline: none; box-shadow: 0 0 0 3px var(--ring); }
  /* ready (send / queue) → bare accent arrow; soft-bg only on hover. */
  .send-inline.ready { color: var(--accent); }
  .send-inline.ready:hover:not(:disabled) { background: var(--accent-soft); color: var(--accent); }
  /* stop → danger tint while a run is in flight. */
  .send-inline.stop { background: var(--danger-soft); color: var(--danger); }
  .send-inline.stop:hover { background: var(--danger-soft); filter: brightness(1.08); }
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

  /* Pending-rail styles moved to composer/QueueRail.svelte (C3). */

  /* Slash + mention popover styles moved to composer/SlashMenu.svelte +
     composer/MentionPopover.svelte (C6). slash-in stays — .preview-panel
     below still uses it. */
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }

  /* Prompt-enhancer panel styles moved to composer/EnhanceBar.svelte (C5). */
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

  /* Dictation: gentle text pulse while Haiku polishes the final transcript. */
  .textarea-wrap.polishing textarea {
    animation: dictate-polish 1.2s ease-in-out infinite;
  }
  @keyframes dictate-polish {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.55; }
  }
  /* Post-polish restore chip — mirrors the enhance undo-mini pill. */
  .dictate-undo {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    display: flex; align-items: center; gap: 7px;
    padding: 6px 10px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--surface) 88%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid color-mix(in oklch, var(--model-color) 32%, var(--border));
    color: var(--model-color);
    font-size: var(--fs-sm);
    z-index: 10;
  }
  .du-label { font-weight: 600; color: var(--fg); }
  .du-btn {
    display: inline-flex; align-items: center; gap: 4px;
    padding: 3px 9px; border-radius: 999px;
    font: inherit; font-size: 11px; font-weight: 600;
    color: var(--fg-muted);
    background: transparent;
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    cursor: pointer;
    transition: color 130ms, background 130ms, border-color 130ms;
  }
  .du-btn:hover { color: var(--fg); background: color-mix(in oklch, var(--surface-hover) 70%, transparent); border-color: var(--border-strong); }
  @media (prefers-reduced-motion: reduce) {
    .textarea-wrap.polishing textarea { animation: none; }
  }

  /* Word-materialize reveal (.ew) moved to composer/EnhanceBar.svelte (C5). */
  @media (prefers-reduced-motion: reduce) {
    .magic-text { animation: none; -webkit-text-fill-color: var(--fg-muted); }
    .magic-aura, .magic-stars i { animation: none; }
    .magic-stars i { opacity: 0.7; }
  }

  /* Compose-tools (improve/preview) reveal once the draft has text — the empty
     composer stays calm. Reuses the global `enter` keyframe. */
  .cbtn.reveal { animation: enter 160ms ease-out; }
  @media (prefers-reduced-motion: reduce) {
    .cbtn.reveal { animation: none; }
  }
  /* kbd-hint styles moved to composer/LivePills.svelte (C4). */

  /* Unified settings pill — one icon-only control opening the model /
     thinking-depth / permission-mode panel. `data-mode` faintly tints the
     icon so the current permission posture (unguarded vs cautious) reads at
     a glance without spelling it out. */
  .model-pill {
    align-self: center;
    display: inline-flex; align-items: center; gap: 7px;
    height: 30px; padding: 0 8px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 9px;
    color: var(--fg-2);
    cursor: pointer;
    font: inherit;
    overflow: hidden;
    transition: background 140ms ease-out, color 140ms ease-out, border-color 140ms ease-out;
  }
  .model-pill:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }
  .model-pill.open {
    background: var(--surface-hover);
    border-color: color-mix(in oklab, var(--accent) 55%, var(--border));
    color: var(--fg);
  }
  .model-pill:hover :global(.pill-chev) { color: var(--fg-muted); }
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
  /* Leading dot on the model pill — emerald (model identity lives in the
     model-card dropdown, not the always-visible pill). */
  .model-dot {
    width: 7px; height: 7px; border-radius: 50%;
    flex-shrink: 0;
    background: var(--accent);
    transition: background 140ms ease-out;
  }
  /* Chevron-up caret on the model pill; rotates 180° when its menu opens. */
  :global(.model-pill .pill-chev) {
    color: var(--fg-faint);
    transition: color 140ms ease-out, transform 140ms ease-out;
  }
  .model-pill.open :global(.pill-chev) { transform: rotate(180deg); color: var(--fg-muted); }

  /* Experimental local-mode pill (cont.127) — accent-tinted so the active
     "talking to a local model" state reads at a glance, distinct from the
     neutral model/perm pills. Only mounts when local mode is on. */
  .local-pill {
    align-self: center;
    display: inline-flex; align-items: center; gap: 5px;
    height: 30px; padding: 0 10px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in oklab, var(--accent) 38%, transparent);
    border-radius: 9px;
    color: var(--accent);
    cursor: pointer; font: inherit;
    transition: background 140ms ease-out, border-color 140ms ease-out;
  }
  .local-pill:hover { background: color-mix(in oklab, var(--accent) 22%, transparent); border-color: color-mix(in oklab, var(--accent) 55%, transparent); }
  .local-pill :global(svg) { color: var(--accent); flex-shrink: 0; }
  .local-pill-label {
    font-size: 11px; font-weight: 600; line-height: 1; letter-spacing: 0.01em;
    max-width: 96px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  /* Perm-menu popover styles moved to composer/PermMenu.svelte (C7).
     The flat perm button itself = .cbtn.cperm (see flat control bar above). */

  /* Settings panel (model rows + effort slider) styles moved to composer/SettingsMenu.svelte (C7). */


  /* Glanceable pill marker when ultracode is the active tier. */
  /* Glanceable pill marker when ultracode is the active tier. */
  .pill-ultra {
    display: inline-flex; align-items: center;
    color: var(--accent);
    filter: drop-shadow(0 0 4px color-mix(in oklab, var(--accent) 55%, transparent));
    animation: ultra-pulse 2.6s ease-in-out infinite;
  }
  .model-pill.ultra {
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
