<script lang="ts">
  import { Send, Square, X, Mic, Loader2, Wand2, Paperclip,
    Sparkles, Eye, ChevronUp, Undo2, Cpu, Folder, GitBranch } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { localLlm } from "../../state/localLlm.svelte";
  import { workspace } from "../../state/workspace.svelte";
  import { notify } from "../../state/toast.svelte";
  import { clampEffort, modelFamily } from "../../state/assistant/helpers";
  import { requestPrewarm, resetPrewarmDedup } from "../../state/assistant/prewarm";
  import { fuzzyScore, slashScore, isFileDrag, attachImageFiles, summarizeAttach, attachTextFiles, summarizeTextAttach } from "./composer/helpers";
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
  import PreviewPanel from "./composer/PreviewPanel.svelte";
  import {
    MODEL_OPTIONS, MODE_OPTIONS,
    dialStopsFor, dialIdxFor, clampEffortIdx, permToneFor,
    type ModelOpt, type ModeOpt,
  } from "./composer/modelMatrix";
  import { stt } from "../../state/stt.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { tick, onMount, onDestroy, untrack } from "svelte";

  // Mic-button visibility binds to stt.config.enabled, so load the backend
  // stt config eagerly — otherwise users with STT enabled wouldn't see the
  // mic until they opened Settings → Speech once.
  // localLlm.refresh() intentionally NOT called — local-LLM feature disabled 2026-06-25.
  // Leaving `enabled` at its false default keeps every local-mode branch (pill,
  // placeholders, gating) dead. Re-enable: restore refresh() + the nav page.
  onMount(() => { void stt.init(); });

  // RR2 unmount hygiene — the Composer is destroyed when its tab/split-pane
  // closes (parent gates rendering on tab presence). Without this, pending
  // timers fire on torn-down $state, a PTT hold leaves the mic recording on the
  // global stt singleton, and an in-flight enhance keeps billing a CLI spawn.
  onDestroy(() => {
    if (undoTimer) clearTimeout(undoTimer);
    pttRelease();
    dictKeyHeld = false;
    dictStartedRecording = false;
    // RR10: pttRelease only stops a held PTT; in tap-to-toggle mode pttActive is
    // false while recording, so closing this tab would orphan the live mic
    // (recording=true, events firing into a destroyed tab). Stop any recording
    // this tab owns. (Same orphan risk for a Ctrl+D tap-toggle — same guard.)
    if (stt.recording && stt.targetTabId === tabId) void stt.cancel();
    // RR9: bump the seq UNCONDITIONALLY so any pending enhance callback (incl.
    // one still in the network round-trip before onRequestId populated the id)
    // is invalidated; cancel the backend subprocess only once we have the id.
    // The old `enhancing && enhanceRequestId` guard skipped cancellation in the
    // window between `enhancing = true` and the async onRequestId callback,
    // leaking a billed CLI spawn when a split-pane closed mid-init.
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

  // Per-pane composer: bind to THIS tab's draft/attachments/queue/streaming
  // rather than the focused-pane shims, so two panes can compose & stream
  // concurrently. Tab can be null transiently (empty pane during drag/drop);
  // the parent (AssistantPane) gates Composer rendering on tab presence.
  const tab = $derived(assistant.tabFor(tabId));
  const draft = $derived(tab?.draft ?? "");
  const hasDraft = $derived(draft.trim().length > 0);
  const attachments = $derived(tab?.attachments ?? []);
  const textAttachments = $derived(tab?.textAttachments ?? []);
  const queue = $derived(tab?.queue ?? []);
  const streaming = $derived(tab?.streaming ?? false);
  // Context chips (in-chat only) — passive workspace · branch readout above
  // the well. Hero suppresses them: the welcome card already shows both.
  const wsFolderName = $derived(assistant.workspace.current?.split(/[\\/]/).filter(Boolean).pop() ?? "");
  $effect(() => {
    if (!hero && assistant.workspace.current && assistant.workspaceBranch == null) void assistant.loadWorkspaceBranch();
  });
  // Per-pane context readout — the bare assistant.ctx* getters delegate to the
  // focused activeTab, so in split-pane both composers showed the focused
  // pane's ctx%. Read this pane's own tab instead.
  const paneCtxTokens = $derived(assistant.ctxTokensFor(tab));
  const paneCtxPct = $derived(assistant.ctxPctFor(tab));
  const paneCtxWindow = $derived(assistant.ctxWindowFor(tab));
  // Per-pane model — `assistant.effectiveModel` delegates to the focused
  // activeTab (modelOverride ?? store.model), so in split-pane the background
  // pane's pill / settings highlight / data-model showed the FOCUSED pane's
  // model. Read this pane's own tab override, falling back to the global model.
  const paneEffectiveModel = $derived(tab?.modelOverride ?? assistant.model);

  // Pending rail (queue chips + clear) extracted to composer/QueueRail.svelte (C3).

  // Live-activity pills + idle kbd-hint (the toolbar's middle slot) extracted
  // to composer/LivePills.svelte (C4) — incl. the 1s `now` ticker.

  function setDraft(v: string) { if (tab) tab.draft = v; }

  let ta = $state<HTMLTextAreaElement | undefined>();
  // Tracks whether the input has grown past one line — flips the well to
  // bottom-align so the inline send arrow rides the textarea's last line.
  let multiline = $state(false);
  let atMaxHeight = $state(false);

  type SlashCmd = {
    name: string;
    desc: string;
    // Present on entries discovered from the user's Claude Code setup
    // (`~/.claude` + `<root>/.claude` skills/commands). These aren't run by
    // runSlash — they ride to the CLI as `/name`, where its own skill
    // resolution takes over.
    custom?: { source: "user" | "project"; kind: "skill" | "command"; hint?: string };
  };
  // Grouped: conversation lifecycle → model + composition → flow control → info.
  const SLASH_COMMANDS: SlashCmd[] = [
    { name: "new",       desc: "Start a new conversation (saves current)" },
    { name: "clear",     desc: "Clear this chat in place (saves current to History)" },
    { name: "model",     desc: "Switch model — opens picker" },
    { name: "retry",     desc: "Re-fire the last prompt" },
    { name: "copy",      desc: "Copy last response to clipboard" },
    { name: "stop",      desc: "Halt the current turn" },
    { name: "tools",     desc: "List available workspace tools" },
    { name: "mcp",       desc: "MCP server status for this session" },
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

  // Idle placeholder — ONE quiet rotating phrase (the old single-line kbd
  // soup "Ask · / · @ · Ctrl+D" read as a manual page; owner killed it
  // 2026-07-14). Cycles every 7s through short plain-text hints via the
  // shared placeholder-fade rise; frozen on the first hint under
  // reduced-motion.
  const IDLE_HINTS = $derived.by(() => {
    const hints = [
      `Ask ${localLlm.askLabel} anything`,
      "Type / for a command",
      "Mention a file with @",
    ];
    if (stt.config.enabled) hints.push("Ctrl+D to dictate");
    return hints;
  });
  let hintIdx = $state(0);

  function autosize() {
    if (!ta) return;
    ta.style.height = "auto";
    // Dictation ghost renders outside the textarea value — take whichever
    // mirror is taller so in-flight speech reserves its own lines.
    const want = Math.max(ta.scrollHeight, ghostEl?.scrollHeight ?? 0);
    const h = Math.min(want, 340);
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
      // Refocus the textarea on programmatic draft writes (e.g. recall, mention
      // insert), but NOT when focus legitimately lives elsewhere — the EnhanceBar
      // steer input would otherwise lose focus mid-keystroke every time STT writes
      // an interim transcript into the draft.
      if (draft && ta && (document.activeElement === ta || document.activeElement === document.body || document.activeElement === null)) {
        ta.focus();
      }
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
  // renders it un-keyed), but transient per-tab UI state (mention popover offset,
  // recall cursor, open menus, and any in-flight enhance) belongs to the PREVIOUS
  // tab. Picking a mention at a stale offset corrupts the new draft; an in-flight
  // enhance from Tab A would resolve and render its result over Tab B (and Accept
  // would overwrite Tab B's draft). Reset it all when the active tab changes.
  $effect(() => {
    void tabId;
    untrack(() => {
      mentionState = null;
      recallOffset = -1;
      settingsOpen = false;
      permOpen = false;
      // The attach banner describes the PREVIOUS tab's paste/drop outcome.
      attachError = null;
      // Cancel + clear any enhance bound to the tab we're leaving so its async
      // result can't land in this tab (the seq bump invalidates pending callbacks).
      if (enhancing && enhanceRequestId) assistant.cancelEnhance(enhanceRequestId);
      enhanceRequestId = null;
      enhanceSeq++;
      enhancing = false;
      enhancedPreview = null;
      enhanceError = null;
      enhanceOriginal = null;
      enhanceStatus = null;
      enhanceMeta = null;
      // The enhance-undo affordance is likewise un-keyed to the tab — without
      // this, Undo on the newly-focused tab would clobber its draft with the
      // previous tab's pre-enhance text.
      undoDraft = null;
      clearTimeout(undoTimer);
      // #67: drop the pre-warm dedup latch so the newly-focused fresh tab can
      // request its own spare even if its signature matches the prior tab's.
      resetPrewarmDedup();
    });
  });

  // #67 pre-warming: request a warm `claude` spare for a tab that has no live
  // child yet (a fresh tab, or a history chat opened after an app restart), so
  // the first turn skips cold-boot + the SessionStart-hook tax. Re-runs when the
  // picker signature, tab, root, or auth changes; the debounce + per-signature
  // dedup inside requestPrewarm keep it cheap (no spawn while streaming or for an
  // identical spare). With the persistent-process model the child survives the
  // whole session, so there's no mid-session re-warm scramble — the backend
  // no-ops when a live child already exists.
  $effect(() => {
    // Touch the signature inputs so the effect re-runs on any change.
    void tabId;
    void assistant.effectiveModel;
    void assistant.thinkingEffort;
    void assistant.thinkingEnabled;
    void assistant.permissionMode;
    void tab?.convoCreatedAt;
    void tab?.workspaceRoot;
    void assistant.workspace.current;
    void assistant.auth?.pill;
    requestPrewarm(assistant);
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
  // Custom skills/commands ride the CLI's own config resolution: local-LLM
  // (--bare) strips them all; sandbox mode (useFullConfig off) drops the
  // `user` setting source, so personal entries can't run — hide accordingly.
  // Builtins always win a name collision.
  const customSlash = $derived.by<SlashCmd[]>(() => {
    if (localLlm.enabled) return [];
    const seen = new Set(SLASH_COMMANDS.map((c) => c.name));
    const out: SlashCmd[] = [];
    for (const c of assistant.customCommands) {
      if (c.source === "user" && !assistant.useFullConfig) continue;
      if (seen.has(c.name)) continue;
      seen.add(c.name);
      out.push({
        name: c.name,
        desc: c.description || (c.kind === "skill" ? "Custom skill" : "Custom command"),
        custom: { source: c.source, kind: c.kind, hint: c.argumentHint },
      });
    }
    return out;
  });
  const slashFiltered = $derived.by(() => {
    const q = draft.slice(1).toLowerCase();
    const all = [...SLASH_COMMANDS, ...customSlash];
    if (!q) return all;
    return all
      .map((c) => ({ c, s: slashScore(c.name, q) }))
      .filter((x): x is { c: SlashCmd; s: number } => x.s !== null)
      .sort((a, b) => b.s - a.s)
      .map((x) => x.c);
  });
  let slashIdx = $state(0);
  $effect(() => {
    const _v = slashFiltered.length;
    void _v;
    slashIdx = 0;
  });
  // Rescan the custom catalog on each menu OPEN (not per keystroke) so a skill
  // added mid-session shows up on the next `/`. Plain (non-$state) latch —
  // this is edge detection, not render state.
  let slashScanLatch = false;
  $effect(() => {
    if (slashOpen && !slashScanLatch) void assistant.loadCustomCommands();
    slashScanLatch = slashOpen;
  });
  // Current model row — drives the composer's bottom-right pill label.
  const currentModel = $derived(MODEL_OPTIONS.find((m) => m.id === paneEffectiveModel));

  // Reasoning-ladder derives the parent still needs (pill label, settingsRows,
  // onKey ←/→) — same matrix helpers SettingsMenu uses, so they can't drift.
  // ONE ladder over the store pair (thinkingEnabled, thinkingEffort): rung 0 =
  // fastest (thinking off → wire `--effort low`), higher rungs reason at their
  // tier. See modelMatrix DIAL_STOPS for the wire-truth rationale.
  const effortStops = $derived(dialStopsFor(currentModel));
  const dialApplies = $derived(effortStops.length > 0);
  const effortIdx = $derived(dialIdxFor(effortStops, assistant.thinkingEnabled, assistant.thinkingEffort));
  const currentEffort = $derived(effortStops[effortIdx] ?? effortStops[0]);
  function setEffortByIdx(i: number) {
    const s = effortStops[clampEffortIdx(effortStops, i)];
    if (!s) return;
    if (s.effort === null) assistant.setThinkingDial(false);
    else assistant.setThinkingDial(true, s.effort);
  }
  // Switching to a lower-ceiling model (e.g. Opus@ultra → a capped model) must
  // pull the stored TIER down to that model's ceiling, so we never send a flag
  // the model rejects. (setModel already clamps on pick; this guards the case
  // where the model changed by another path, e.g. a tab's modelOverride.)
  // Clamps the tier directly — the ladder's rung 0 (effort:null) is not a tier,
  // so a stops-membership check would false-positive on `none`.
  $effect(() => {
    if (!dialApplies) return;
    const clamped = clampEffort(assistant.thinkingEffort, paneEffectiveModel);
    if (clamped !== assistant.thinkingEffort) assistant.setThinkingEffort(clamped);
  });
  // Caption + pointer-drag dial live in composer/SettingsMenu.svelte (C7).

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
    if (dialApplies) rows.push({ kind: "effort" });
    return rows;
  });
  // Re-seed the cursor to the current model row whenever the panel opens.
  $effect(() => {
    if (settingsOpen) {
      const i = settingsRows.findIndex((r) => r.kind === "model" && r.model.id === paneEffectiveModel);
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

  // Tab (or picking a command that wants arguments): insert `/name ` into the
  // draft instead of firing, so the user can type args — the trailing space
  // closes the menu (slashOpen requires a space-free draft) and Enter sends.
  function fillSlash(c: SlashCmd) {
    const text = `/${c.name} `;
    setDraft(text);
    stt.consume();
    void tick().then(() => {
      ta?.focus();
      ta?.setSelectionRange(text.length, text.length);
      autosize();
    });
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
    // A custom command with an argument hint expects args — fill, don't fire.
    if (c.custom?.hint) {
      fillSlash(c);
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
    if (!assistant.authReady) {
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

  // ── Prompt enhancer (wand) ───────────────────────────────────────────────
  // One-shot Sonnet rewrite of the current draft into a clearer prompt. Result
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
    // Undo restores what accept REPLACED — the draft as it is right now, not
    // the pre-enhance snapshot. Text typed after the preview settled would
    // otherwise be unrecoverable (undo would roll back past it).
    undoDraft = draft || enhanceOriginal;
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
  // Ctrl/Cmd+D dictation — tap vs hold, resolved on release:
  //   • quick tap (< HOLD_MS held) → toggle mode: starts now, next tap stops.
  //   • hold (≥ HOLD_MS) → push-to-talk: records while held, release stops.
  // Recording starts on keydown either way (instant feedback); the gesture is
  // only classified on keyup, so the two models share one start path.
  const DICT_HOLD_MS = 400;
  let dictKeyDownAt = 0;
  let dictKeyHeld = false;
  // True only when THIS Ctrl+D press started the recording — so releasing a
  // hold stops the right session and a tap that began a *new* recording can
  // latch into toggle mode without the keyup immediately killing it.
  let dictStartedRecording = false;
  function dictKeydown(e: KeyboardEvent): boolean {
    if (!((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "d"))
      return false;
    e.preventDefault(); // always — kills the browser's Ctrl+D bookmark
    if (e.repeat || dictKeyHeld) return true; // swallow auto-repeat while held
    if (micBusy || stt.transcribing) return true;
    // The press is now "engaged" regardless of branch — record the time so a
    // stale value from an earlier press can never leak into this release.
    dictKeyHeld = true;
    dictKeyDownAt = performance.now();
    if (stt.recording) {
      // Already recording (a prior tap latched it on) → this press stops it.
      // dictStartedRecording=false so the keyup won't double-stop.
      dictStartedRecording = false;
      void toggleMic();
      return true;
    }
    if (!micAvailable) {
      dictKeyHeld = false;
      return true;
    }
    // Fresh start; tap-vs-hold is classified on keyup.
    dictStartedRecording = true;
    void toggleMic();
    return true;
  }
  function dictRelease() {
    if (!dictKeyHeld) return;
    dictKeyHeld = false;
    const heldMs = performance.now() - dictKeyDownAt;
    const startedThisPress = dictStartedRecording;
    dictStartedRecording = false;
    // Only a HOLD that STARTED a recording this press stops on release (push-to-
    // talk). A tap leaves it on (toggle — next tap stops). A press that stopped
    // an already-running recording started nothing, so it never re-stops here.
    // Don't gate on stt.recording: a fast hold can release before stt.start()
    // flips recording=true, and stt.stop() is idempotent for that window.
    if (startedThisPress && heldMs >= DICT_HOLD_MS) {
      void stt.stop();
      void tick().then(() => { autosize(); ta?.focus(); });
    }
  }
  function onKeyUp(e: KeyboardEvent) {
    // Classify the Ctrl+D gesture on the "d" keyup only. (Not on Control/Meta —
    // releasing the modifier first while D is still held must NOT end a hold.)
    if (e.key.toLowerCase() === "d") dictRelease();
    if (e.key !== " ") return;
    pttRelease();
  }
  // Window blur mid-hold: we can't see the keyup, so force-stop a held Ctrl+D
  // (don't leave the mic running on focus-loss) and release any held Space PTT.
  function onWindowBlur() {
    if (dictKeyHeld) {
      dictKeyHeld = false;
      dictStartedRecording = false;
      if (stt.recording && stt.targetTabId === tabId) void stt.stop();
    }
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
  // Auto-stop countdown — only for the pane that owns the active dictation.
  const silenceFrac = $derived(
    stt.targetTabId === tabId ? stt.silenceFrac : null,
  );
  // In-flight spoken words for THIS pane — rendered as a dim ghost tail after
  // the solid draft; they "turn white" when the final transcript commits.
  let ghostEl = $state<HTMLDivElement | null>(null);
  const dictating = $derived(
    stt.targetTabId === tabId && (stt.recording || stt.transcribing),
  );
  // Rotate the idle hint only while the idle ghost is actually showing.
  const idleGhost = $derived(!dictating && !hero && draft.length === 0 && !streaming && attachments.length === 0);
  $effect(() => {
    if (!idleGhost) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const t = setInterval(() => { hintIdx = (hintIdx + 1) % IDLE_HINTS.length; }, 7000);
    return () => clearInterval(t);
  });
  const dictGhost = $derived(dictating ? stt.ghostTail : "");
  $effect(() => {
    const _g = dictGhost;
    void _g;
    void tick().then(autosize);
  });
  // Dictation availability — gates both the mic button render and the Ctrl+D
  // shortcut so they stay in lockstep.
  const micAvailable = $derived(
    stt.config.enabled &&
      ((stt.config.engine === "web_speech" && stt.supported) ||
        (stt.config.engine === "whisper" && stt.backends.whisper) ||
        (stt.config.engine === "parakeet" && stt.backends.parakeet)),
  );
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
    // Snapshot the paste target: base64-encoding a multi-MB image is async and
    // `tabId` is reactive — a tab switch mid-encode would land the attachment
    // on the NEW tab. Same pattern as AppShell's window-drop capture.
    const targetTabId = tabId;
    const res = await attachImageFiles(imageFiles, (a) => assistant.addAttachment(a, targetTabId));
    if (tabId === targetTabId) attachError = summarizeAttach(res);
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
    // Ctrl/Cmd+D — tap-toggle or hold-to-talk dictation (see dictKeydown).
    if (dictKeydown(e)) return;
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
    if (slashOpen) {
      if (slashFiltered.length > 0) {
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
        // Tab = insert `/name ` for arg typing; Enter (below) runs it.
        if (e.key === "Tab") {
          e.preventDefault();
          fillSlash(slashFiltered[slashIdx]);
          return;
        }
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
      // Slash menu open with no match (e.g. "/zzz") — swallow Enter instead of
      // firing it as a real chat turn.
      if (slashOpen) return;
      fire();
    }
  }

  function onBtnClick() {
    // Staged attachments count as a draft (fire() allows attachments-only
    // sends), so a streaming turn + image-only composer queues, not stops.
    if (streaming && draft.trim().length === 0 && attachments.length === 0) {
      void assistant.stop(tabId);
      return;
    }
    fire();
  }

  // Three modes for the action button:
  //   idle + draft                    → Send
  //   streaming + empty               → Stop (kill the running turn)
  //   streaming + draft/attachments   → Queue (append to message queue)
  const mode = $derived.by<"send" | "stop" | "queue">(() => {
    if (streaming && !hasDraft && attachments.length === 0) return "stop";
    if (streaming) return "queue";
    return "send";
  });
  const canFire = $derived(
    mode === "stop" ||
      ((hasDraft || attachments.length > 0) && assistant.authReady),
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
    // Same snapshot as onPaste — the encode awaits below outlive a tab switch.
    const targetTabId = tabId;
    const imgRes = await attachImageFiles(files, (a) => assistant.addAttachment(a, targetTabId));
    const txtRes = await attachTextFiles(files, (a) => assistant.addTextAttachment(a, targetTabId));
    if (tabId === targetTabId) {
      attachError = [summarizeAttach(imgRes), summarizeTextAttach(txtRes)].filter(Boolean).join(" · ") || null;
    }
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

<svelte:window onkeyup={onKeyUp} onblur={onWindowBlur} />

<div class="composer-wrap" data-model={modelFamily(paneEffectiveModel)}>
  <QueueRail
    tab={tab ?? null}
    {queue}
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

    {#if previewing && hasDraft}
      <PreviewPanel {draft} />
    {/if}

    {#if slashOpen}
      <SlashMenu commands={slashFiltered} activeIdx={slashIdx} query={draft.slice(1).toLowerCase()} onPick={pickSlash} />
    {/if}

    {#if assistant.ui.usageOpen}
      <UsagePanel {tab} mode={assistant.ui.usageOpen === "full" ? "full" : "ctx"} onClose={() => (assistant.ui.usageOpen = false)} />
    {/if}

    {#if mentionState && mentionResults.length > 0}
      <MentionPopover
        results={mentionResults}
        activeIdx={mentionIdx}
        fileCount={assistant.workspaceFiles.length}
        onPick={pickMention}
      />
    {/if}

    <div class="composer" class:hero={hero} class:streaming={streaming} class:enchanting={enhancing} data-mode={mode}>
      {#if !hero && assistant.workspace.current}
        <div class="ctx-chips" aria-label="Workspace context">
          <span class="ctx-chip" use:tooltip={assistant.workspace.current}>
            <Folder size={11} />
            <span class="cc-label">{wsFolderName}</span>
          </span>
          {#if assistant.workspaceBranch}
            <span class="ctx-chip" use:tooltip={"Current git branch"}>
              <GitBranch size={11} />
              <span class="cc-label">{assistant.workspaceBranch}</span>
            </span>
          {/if}
        </div>
      {/if}
      <!-- WELL: attachments + input only. All chrome (border/glass/focus-ring/
           streaming edge) lives here; controls sit on the flat bar BELOW. -->
      <div class="composer-box" class:multiline={multiline}>
      {#if tab?.promptSuggestion && !hasDraft && !streaming}
        <!-- #87: ghost suggestion from the CLI's --prompt-suggestions — one
             predicted next prompt per turn. Click inserts it into the draft;
             typing anything hides it (hasDraft gate); beginTurn clears it. -->
        <div class="ps-row">
          <button
            type="button"
            class="ps-chip"
            onclick={() => {
              if (!tab?.promptSuggestion) return;
              const s = tab.promptSuggestion;
              tab.promptSuggestion = null;
              setDraft(s);
              requestAnimationFrame(() => { ta?.focus(); autosize(); });
            }}
            use:tooltip={"Suggested next prompt — click to insert"}
            aria-label="Insert suggested prompt"
          >
            <Sparkles size={12} />
            <span class="ps-text">{tab.promptSuggestion}</span>
          </button>
          <button
            type="button"
            class="ps-dismiss"
            onclick={() => { if (tab) tab.promptSuggestion = null; }}
            use:tooltip={"Dismiss suggestion"}
            aria-label="Dismiss suggestion"
          >
            <X size={11} />
          </button>
        </div>
      {/if}
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
          onblur={() => {
            if (!mentionState) return;
            requestAnimationFrame(() => { mentionState = null; });
          }}
          onkeydown={onKey}
          onpaste={onPaste}
          placeholder=""
          class:scrollable={atMaxHeight}
          rows="1"
          readonly={enhancing}
        ></textarea>
        {#if dictating && draft.length === 0}
          {#if !dictGhost}<span class="placeholder-ghost static" aria-hidden="true">Listening…</span>{/if}
        {:else if hero && draft.length === 0 && !streaming && attachments.length === 0}
          <span class="placeholder-ghost static" aria-hidden="true">What are we working on today?</span>
        {:else if draft.length === 0 && !streaming && attachments.length === 0}
          {#key hintIdx}
            <span class="placeholder-ghost" aria-hidden="true">{IDLE_HINTS[hintIdx % IDLE_HINTS.length]}</span>
          {/key}
        {:else if streaming && draft.length === 0}
          <span class="placeholder-ghost static" aria-hidden="true"><span class="ph-k">Enter</span> queues for the next turn · <span class="ph-k">/stop</span> halts</span>
        {:else if attachments.length > 0 && draft.length === 0}
          <span class="placeholder-ghost static" aria-hidden="true">Ask about the image…</span>
        {/if}
        {#if dictGhost}
          <!-- Mirror overlay: invisible copy of the committed draft positions
               the ghost tail exactly where the caret would land. -->
          <div class="dict-ghost" bind:this={ghostEl} aria-hidden="true"><span class="dg-committed">{draft}{#if draft.length > 0 && !/\s$/.test(draft)}{" "}{/if}</span><span class="dg-tail" class:pending={stt.transcribing}>{dictGhost}</span></div>
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
      </div>
      </div>

      {#if !hero}
      <!-- Control deck — flat row below the well: perm + tools left, live
           pills middle, model + ctx ring + send right. Hidden while the hero
           idles; it materializes as the composer descends on engagement. -->
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

          <span class="cbar-sep" aria-hidden="true"></span>

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
          {#if micAvailable}
          <button
            class="cbtn ic micbtn"
            class:recording={stt.recording}
            class:transcribing={stt.transcribing}
            class:mic-error={!!stt.lastError && !stt.recording && !stt.transcribing}
            type="button"
            onclick={toggleMic}
            disabled={micBusy || stt.transcribing}
            use:tooltip={
              stt.recording ? "Stop recording" :
              stt.transcribing ? "Transcribing…" :
              stt.currentState === "loading_model" ? "Loading model…" :
              stt.lastError ? stt.lastError :
              stt.config.engine === "parakeet"
                ? "Dictate — Parakeet (local) · Ctrl+D or hold Space"
                : stt.config.engine === "whisper"
                  ? "Dictate — Whisper (local) · Ctrl+D or hold Space"
                  : "Dictate — Web Speech · Ctrl+D or hold Space"
            }
            aria-label={stt.recording ? "Stop recording" : stt.lastError ? `Start recording (last error: ${stt.lastError})` : "Start recording"}
          >
            {#if stt.transcribing || (micBusy && !stt.recording)}
              <Loader2 size={15} class="mic-spin" />
            {:else if stt.recording}
              <span
                class="mic-wave"
                class:silent={stt.level < 0.04}
                style="--lvl:{stt.level}"
                aria-hidden="true"
              >
                <span></span><span></span><span></span><span></span><span></span>
              </span>
            {:else}
              <Mic size={15} />
            {/if}
            {#if stt.recording && silenceFrac !== null}
              <!-- Auto-stop warning: a depleting ring hugging the button.
                   Drains over the final seconds of silence; speech refills
                   (frac → null) and it vanishes. No layout shift. -->
              <svg class="mic-ring" viewBox="0 0 38 36" aria-hidden="true">
                <rect
                  x="1.25" y="1.25" width="35.5" height="33.5" rx="10"
                  pathLength="100"
                  style="stroke-dashoffset: {100 - Math.max(0, Math.min(1, silenceFrac)) * 100}"
                />
              </svg>
            {/if}
          </button>
          {/if}
          {#if hasDraft}
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

        <LivePills {queue} />

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
            class:ultra={dialApplies && currentEffort?.id === "xhigh"}
            data-model={currentModel ? modelFamily(currentModel.id) : ""}
            bind:this={modelWrap}
            onclick={() => { settingsOpen = !settingsOpen; permOpen = false; void tick().then(() => ta?.focus()); }}
            aria-haspopup="listbox"
            aria-expanded={settingsOpen}
            aria-label="Model & effort"
            use:tooltip={dialApplies
              ? `Model · effort\n${currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.effectiveModel} · ${currentEffort?.label} effort — ${effortIdx === 0 ? "replies immediately" : "reasons before replying"}`
              : `Model\n${currentModel ? `${currentModel.label} ${currentModel.version}` : assistant.effectiveModel} · no extended thinking`}
          >
            <span class="model-dot" aria-hidden="true"></span>
            <span class="pill-label">{currentModel ? `${currentModel.label} ${currentModel.version}` : paneEffectiveModel}</span>
            {#if dialApplies}
              <span class="pill-effort" class:dim={effortIdx === 0}>{currentEffort?.label}</span>
            {/if}
            {#if dialApplies && currentEffort?.id === "xhigh"}
              <span class="pill-ultra" aria-hidden="true" use:tooltip={"X-High — deepest reasoning + autonomous workflows"}><Sparkles size={11} /></span>
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

          {#if !localLlm.enabled && paneCtxTokens > 0}
            <CtxRing
              pct={paneCtxPct}
              tokens={paneCtxTokens}
              window={paneCtxWindow}
              open={!!assistant.ui.usageOpen}
              onClick={() => (assistant.ui.usageOpen = assistant.ui.usageOpen ? false : "ctx")}
            />
          {/if}

          <!-- Send — the deck's anchor. Same multi-mode action (send / queue /
               stop) with a queued-count badge; accent-filled when ready. -->
          <button
            class="send-btn"
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
              ? { text: "Queue for the next turn — ⇧↵ for a new line", kbd: "Enter" }
              : { text: "Send — ⇧↵ for a new line", kbd: "Enter" }}
          >
            <span class="icon-stack">
              <span class="icon-slot" class:active={mode === "send" || mode === "queue"}><Send size={14} /></span>
              <span class="icon-slot" class:active={mode === "stop"}><Square size={12} fill="currentColor" /></span>
            </span>
            {#if queue.length > 0}
              <span class="send-count" aria-hidden="true">{queue.length}</span>
            {/if}
            {#key fireKey}
              {#if fireKey > 0}
                <span class="send-ripple" aria-hidden="true"></span>
                <span class="send-ripple send-ripple-2" aria-hidden="true"></span>
              {/if}
            {/key}
          </button>
        </div>
      </div>
      {/if}
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

  /* AI disclaimer → global StatusBar (ambient info belongs to the ambient bar). */
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
    animation: drop-in var(--dur-fast) cubic-bezier(0.22, 1, 0.36, 1) both;
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
    border-radius: 12px;
    box-shadow: inset 0 1px 0 color-mix(in oklch, white 3%, transparent);
    transition: border-color var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1),
                box-shadow var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1),
                transform var(--dur-fast) ease-out;
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
    border-color: color-mix(in oklch, var(--model-color) 38%, var(--border));
    box-shadow:
      0 0 0 1px color-mix(in oklch, var(--model-color) 10%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
  }

  /* Hero mode — home surface. The composer is the centerpiece: a larger,
     more rounded card. */
  .composer.hero .composer-box { border-radius: 14px; padding: 3px; }
  .composer.hero textarea { font-size: 14.5px; line-height: 1.55; padding: 11px 12px 11px 14px; min-height: 30px; letter-spacing: -0.003em; }
  /* Keep the hero placeholder in lockstep with the hero textarea so the ghost
     prompt and the text you type read at the same size. */
  .composer.hero .placeholder-ghost { font-size: 14.5px; line-height: 1.55; top: 11px; left: 14px; right: 12px; }

  /* Streaming = the composer visibly breathes: tinted ring (::before), an
     orbiting comet arc (::after), and an outer halo carried on the box's OWN
     box-shadow — pseudo shadows get clipped by the well's overflow:hidden,
     so a halo on ::before never actually painted. */
  .composer.streaming .composer-box {
    border-color: color-mix(in oklch, var(--model-color) 55%, var(--border));
    box-shadow:
      0 0 20px -4px color-mix(in oklch, var(--model-color) 40%, transparent),
      0 0 42px -4px color-mix(in oklch, var(--model-color) 22%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 4%, transparent);
  }
  .composer.streaming .composer-box::before {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    border: 1.5px solid color-mix(in oklch, var(--model-color) 55%, transparent);
    box-shadow: inset 0 0 10px color-mix(in oklch, var(--model-color) 18%, transparent);
    pointer-events: none;
    animation: composer-stream 2.6s ease-in-out infinite;
    z-index: 2;
  }
  /* Comet — a bright model-tinted arc tracing the frame edge (conic gradient
     masked to a border-width ring, angle driven by a registered @property).
     Asymmetric stops = long faint tail building into a hot head. */
  .composer.streaming .composer-box::after {
    content: "";
    position: absolute;
    inset: 0;
    border-radius: inherit;
    padding: 2px;
    background: conic-gradient(from var(--stream-angle),
      transparent 0deg,
      color-mix(in oklch, var(--model-color) 60%, transparent) 34deg,
      color-mix(in oklch, var(--model-color) 45%, white) 54deg,
      transparent 62deg);
    -webkit-mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
    -webkit-mask-composite: xor;
    mask: linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0);
    mask-composite: exclude;
    pointer-events: none;
    animation: composer-orbit 3.4s linear infinite;
    z-index: 3;
  }
  @property --stream-angle { syntax: "<angle>"; initial-value: 0deg; inherits: false; }
  @keyframes composer-stream {
    0%, 100% { opacity: 0.5; }
    50%      { opacity: 0.85; }
  }
  @keyframes composer-orbit { to { --stream-angle: 360deg; } }
  @media (prefers-reduced-motion: reduce) {
    .composer.streaming .composer-box::before { animation: none; opacity: 0.8; }
    .composer.streaming .composer-box::after { display: none; }
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
    padding: 9px 12px;
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
    top: 9px; left: 12px; right: 12px;
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

  /* ── Control deck — flat transparent row BELOW the well (Claude-Desktop
     layout): the input reads as one clean object; everything a turn needs
     (perm · attach · dictate · draft tools | queue | model · ctx · send)
     sits quietly underneath on the page itself. */
  .composer-bar {
    position: relative;
    z-index: 1;
    display: flex; align-items: center; gap: 3px;
    margin-top: 6px;
    padding: 0 2px;
    /* Width-query container for the narrow-pane ladder below. Safe: both
       popovers (PermMenu/SettingsMenu) portal to <body>, so containment
       can't re-anchor them. */
    container-type: inline-size;
    /* Materialize — the deck rises in as the composer docks (engage/convo). */
    animation: enter var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  @media (prefers-reduced-motion: reduce) { .composer-bar { animation: none; } }
  /* overflow:hidden is the paint-over stopper: min-width:0 lets the box
     shrink below its nowrap children, which otherwise keep painting PAST
     its edge and over .cbar-r (the split-pane "glyph soup", 2026-07-10). */
  .cbar-l { display: flex; align-items: center; gap: 2px; min-width: 0; overflow: hidden; }
  .cbar-r { margin-left: auto; display: flex; align-items: center; gap: 5px; position: relative; min-width: 0; }

  /* Narrow-pane ladder — split panes and small windows shed decoration
     before anything can collide: effort text → labels/reveals → mic+attach.
     Every control stays reachable (dictate = Ctrl+D, attach = paste/drop). */
  @container (max-width: 440px) {
    .pill-effort, .char-count { display: none; }
    .pill-label { max-width: 64px; }
  }
  @container (max-width: 380px) {
    .perm-label, .local-pill-label { display: none; }
    .cbtn.reveal { display: none; }
    .pill-label { max-width: 56px; }
  }
  @container (max-width: 330px) {
    /* .cbar-l prefix beats the later `.cbtn { display:inline-flex }` rule. */
    .cbar-l .micbtn, .cbar-l .attachbtn { display: none; }
  }

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
    transition: background var(--dur-fast), color var(--dur-fast), border-color var(--dur-fast), transform var(--dur-fast);
  }
  .cbtn:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg-2); }
  .cbtn:active:not(:disabled) { transform: scale(0.96); }
  .cbtn:disabled { opacity: 0.4; cursor: default; }
  .cbtn:focus-visible { outline: none; box-shadow: 0 0 0 2px var(--ring); }
  .cbtn.ic { width: 32px; padding: 0; justify-content: center; color: var(--fg-subtle); }
  .cbtn.ic:hover:not(:disabled) { color: var(--fg-2); }
  .cbtn.ic.active { background: var(--accent-soft); color: var(--accent); }
  .cbtn.enhance:hover:not(:disabled) { color: var(--accent); }
  /* Chevrons are hover/open affordances, not idle furniture — hidden until
     the control is intended (mirrored on the model pill below). */
  :global(.cbtn .cbtn-chev) { color: var(--fg-faint); opacity: 0; transition: color var(--dur-fast), transform var(--dur-fast), opacity var(--dur-fast); }
  .cbtn:hover :global(.cbtn-chev), .cbtn.open :global(.cbtn-chev) { opacity: 1; }
  /* Permission button — the TONE lives on the icon; the label stays quiet
     text. Exception: bypass (warn) keeps the full amber label — guardrails-off
     must stay loud (DESIGN.md §8 warn family). */
  .cbtn.cperm { font-weight: 500; color: var(--fg-2); }
  .cbtn.cperm .perm-label { font-size: 12px; font-weight: 500; line-height: 1; white-space: nowrap; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .cbtn.cperm > :global(svg:first-child) { color: currentColor; flex-shrink: 0; }
  .cbtn.cperm.tone-ok   > :global(svg:first-child) { color: var(--ok);   }
  .cbtn.cperm.tone-info > :global(svg:first-child) { color: var(--info); }
  .cbtn.cperm.tone-warn { color: var(--warn); font-weight: 600; }
  /* Hairline between the mode control and the draft tools — two families. */
  .cbar-sep { flex: none; width: 1px; height: 14px; margin: 0 4px; background: color-mix(in oklch, var(--border) 80%, transparent); }
  .cbtn.cperm.tone-ok:hover:not(:disabled)   { background: var(--ok-soft);   color: var(--ok);   }
  .cbtn.cperm.tone-warn:hover:not(:disabled) { background: var(--warn-soft); color: var(--warn); }
  .cbtn.cperm.tone-info:hover:not(:disabled) { background: var(--info-soft); color: var(--info); }
  .cbtn.cperm.open :global(.cbtn-chev) { transform: rotate(180deg); }

  /* Mic — recording / transcribing states inherit .cbtn.ic base + override.
     Recording stays QUIET: soft danger tint + the live waveform is the only
     motion. The waveform already says "hearing you" — no halo pulse. */
  .micbtn { position: relative; }
  .micbtn.recording {
    background: var(--danger-soft);
    color: var(--danger);
    border-color: color-mix(in oklab, var(--danger) 40%, var(--border));
    opacity: 1;
  }
  .micbtn.transcribing {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 40%, var(--border));
    opacity: 1;
  }
  .micbtn.mic-error:not(.recording):not(.transcribing) {
    color: var(--danger);
    border-color: color-mix(in oklab, var(--danger) 40%, var(--border));
    opacity: 1;
  }
  :global(.mic-spin) { animation: mic-spin 0.9s linear infinite; }
  @keyframes mic-spin { to { transform: rotate(360deg); } }
  /* Live recording waveform — 5 bars driven by real mic amplitude. `--lvl`
     (0..1) comes from stt.level (Whisper RMS event / web_speech AnalyserNode).
     Per-bar weights shape a center-tall waveform; each bar's height = a floor
     plus the level scaled by its weight, so the strip visibly reacts to voice.
     The .recording state on .micbtn paints danger-tinted bars via
     currentColor on the soft fill. */
  .mic-wave {
    display: inline-flex; align-items: center; gap: 1.5px;
    height: 13px;
  }
  .mic-wave span {
    width: 2px;
    border-radius: 999px;
    background: currentColor;
    transform-origin: center;
    /* min 22% tall, growing to 100% as level × per-bar weight approaches 1. */
    height: clamp(22%, calc(22% + var(--lvl, 0) * var(--w, 1) * 78%), 100%);
    transition: height 80ms ease-out;
  }
  .mic-wave span:nth-child(1) { --w: 0.55; }
  .mic-wave span:nth-child(2) { --w: 0.85; }
  .mic-wave span:nth-child(3) { --w: 1.1; }
  .mic-wave span:nth-child(4) { --w: 0.85; }
  .mic-wave span:nth-child(5) { --w: 0.55; }
  /* No input → gentle idle breathing so a silent/blocked mic still reads as
     live rather than frozen. */
  .mic-wave.silent span {
    animation: mic-idle var(--pulse-live) ease-in-out infinite;
  }
  .mic-wave.silent span:nth-child(2) { animation-delay: 0.12s; }
  .mic-wave.silent span:nth-child(3) { animation-delay: 0.24s; }
  .mic-wave.silent span:nth-child(4) { animation-delay: 0.36s; }
  .mic-wave.silent span:nth-child(5) { animation-delay: 0.48s; }
  @keyframes mic-idle {
    0%, 100% { height: 22%; }
    50%      { height: 38%; }
  }
  /* Auto-stop warning — an amber ring hugging the mic button that drains over
     the final seconds of silence (stt.silenceFrac 1→0). Speech refills it
     (frac → null, ring unmounts). Stroke-dashoffset transitions between the
     store's 250ms ticks so the drain reads as continuous. */
  .mic-ring {
    position: absolute;
    inset: -3px;
    width: calc(100% + 6px); height: calc(100% + 6px);
    pointer-events: none;
    overflow: visible;
  }
  .mic-ring rect {
    fill: none;
    stroke: var(--warn);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-dasharray: 100;
    transition: stroke-dashoffset 260ms linear;
  }
  @media (prefers-reduced-motion: reduce) {
    .mic-wave span { transition: none; }
    .mic-wave.silent span { animation: none; height: 30%; }
    .mic-ring rect { transition: none; }
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
    animation: enter var(--dur-fast) ease-out;
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

  /* Send — the deck's anchor: a filled circular CTA. Dim when empty, accent-
     tinted when ready, danger square while streaming; queued-count badge rides
     the top-right corner. */
  .send-btn {
    position: relative;
    width: 30px; height: 30px;
    margin-left: 3px;
    display: flex; align-items: center; justify-content: center;
    background: transparent;
    color: var(--fg-subtle);
    border: 1px solid transparent; border-radius: 999px;
    cursor: pointer;
    flex-shrink: 0;
    transition: color var(--dur-fast) ease-out, background var(--dur-fast) ease-out,
                border-color var(--dur-fast) ease-out, box-shadow var(--dur-fast) ease-out,
                transform var(--dur-fast) ease-out;
  }
  .send-btn:active:not(:disabled) { transform: scale(0.9); }
  .send-btn:disabled { cursor: default; opacity: 0.55; }
  .send-btn:focus-visible { outline: none; box-shadow: 0 0 0 3px var(--ring); }
  .send-btn.ready {
    background: var(--model-color);
    border-color: transparent;
    color: oklch(0.16 0.01 250);
  }
  .send-btn.ready:hover:not(:disabled) {
    filter: brightness(1.08);
    transform: translateY(-1px);
  }
  /* stop → danger while a run is in flight. */
  .send-btn.stop {
    background: var(--danger-soft);
    border-color: color-mix(in oklch, var(--danger) 40%, transparent);
    color: var(--danger);
  }
  .send-btn.stop:hover { filter: brightness(1.1); }
  .send-count {
    position: absolute; top: -4px; right: -4px;
    min-width: 15px; height: 15px; padding: 0 4px;
    display: grid; place-items: center;
    border-radius: 999px;
    background: var(--model-color);
    color: oklch(0.16 0.01 250);
    font-size: 9.5px; font-weight: 700; line-height: 1;
    font-variant-numeric: tabular-nums;
    box-shadow: 0 0 0 2px var(--bg);
  }
  /* Launch ripple — two concentric rings expand outward on every fire().
     Mounted by {#key fireKey}; self-removed when the animation ends via
     the unmount on the next key flip. */
  .send-ripple {
    position: absolute;
    inset: -2px;
    border-radius: 999px;
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
    transition: opacity var(--dur-fast) ease-out, transform var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1);
  }
  .icon-slot.active {
    opacity: 1;
    transform: scale(1) rotate(0);
  }

  /* Pending-rail styles moved to composer/QueueRail.svelte (C3). */

  /* Slash + mention popover styles moved to composer/SlashMenu.svelte +
     composer/MentionPopover.svelte (C6). */
  /* Draft preview panel styles moved to composer/PreviewPanel.svelte. */

  /* Prompt-enhancer panel styles moved to composer/EnhanceBar.svelte (C5). */

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
    animation: magic-aura-pulse var(--pulse-live) ease-in-out infinite;
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
    animation: magic-twinkle var(--pulse-live) ease-in-out infinite;
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

  /* Dictation: gentle text pulse while Claude polishes the final transcript. */
  /* Dictation ghost — mirror div over the textarea. The committed-draft span
     is invisible but occupies space, so the ghost tail wraps exactly where the
     caret sits. Ghost text reads one ladder-step down (--fg-subtle); it turns
     "white" by committing into the textarea, not by restyling. */
  .dict-ghost {
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
  }
  .composer.hero .dict-ghost { font-size: 14.5px; line-height: 1.55; padding: 11px 12px 11px 14px; }
  .dg-committed { visibility: hidden; }
  .dg-tail { color: var(--fg-subtle); }
  .dg-tail.pending { animation: dictate-polish 1.2s ease-in-out infinite; }

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
    transition: color var(--dur-fast), background var(--dur-fast), border-color var(--dur-fast);
  }
  .du-btn:hover { color: var(--fg); background: color-mix(in oklch, var(--surface-hover) 70%, transparent); border-color: var(--border-strong); }
  @media (prefers-reduced-motion: reduce) {
    .textarea-wrap.polishing textarea { animation: none; }
    .dg-tail.pending { animation: none; opacity: 0.7; }
  }

  /* Word-materialize reveal (.ew) moved to composer/EnhanceBar.svelte (C5). */
  @media (prefers-reduced-motion: reduce) {
    .magic-text { animation: none; -webkit-text-fill-color: var(--fg-muted); }
    .magic-aura, .magic-stars i { animation: none; }
    .magic-stars i { opacity: 0.7; }
  }

  /* Compose-tools (improve/preview) reveal once the draft has text — the empty
     composer stays calm. Reuses the global `enter` keyframe. */
  .cbtn.reveal { animation: enter var(--dur-fast) ease-out; }
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
    min-width: 0;
    height: 30px; padding: 0 8px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 9px;
    color: var(--fg-2);
    cursor: pointer;
    font: inherit;
    overflow: hidden;
    transition: background var(--dur-fast) ease-out, color var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out;
  }
  .model-pill:hover {
    background: var(--surface-hover);
    color: var(--fg);
  }
  .model-pill.open {
    background: var(--surface-hover);
    border-color: var(--border-strong);
    color: var(--fg);
  }
  .model-pill:hover :global(.pill-chev) { color: var(--fg-muted); }
  /* Current-model label on the pill. */
  .pill-label {
    font-size: 12px;
    font-weight: 500;
    line-height: 1;
    letter-spacing: 0.005em;
    max-width: 96px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Effort label trails the model name — plain quiet word, no separator dot
     (the 7px pill gap is the separator). */
  .pill-effort { font-size: 11px; font-weight: 500; color: var(--fg-faint); line-height: 1; white-space: nowrap; }
  /* Low rung (replies immediately) reads quieter than the reasoning rungs. */
  .pill-effort.dim { opacity: 0.72; }
  /* Permission-mode dot — one consistent at-a-glance signal for all five
     modes (the pill's text-tint only covered ask/bypass). Colored per mode:
     ask=accent, edit=ok, plan=blue, auto=accent, bypass=warn. */
  /* Leading dot on the model pill — emerald (model identity lives in the
     model-card dropdown, not the always-visible pill). */
  .model-dot {
    width: 7px; height: 7px; border-radius: 50%;
    flex-shrink: 0;
    background: var(--accent);
    transition: background var(--dur-fast) ease-out;
  }
  /* Chevron-up caret on the model pill; rotates 180° when its menu opens. */
  :global(.model-pill .pill-chev) {
    color: var(--fg-faint); opacity: 0;
    transition: color var(--dur-fast) ease-out, transform var(--dur-fast) ease-out, opacity var(--dur-fast) ease-out;
  }
  .model-pill:hover :global(.pill-chev), .model-pill.open :global(.pill-chev) { opacity: 1; }
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
    transition: background var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out;
  }
  .local-pill:hover { background: color-mix(in oklab, var(--accent) 22%, transparent); border-color: color-mix(in oklab, var(--accent) 55%, transparent); }
  .local-pill :global(svg) { color: var(--accent); flex-shrink: 0; }
  .local-pill-label {
    font-size: 11px; font-weight: 600; line-height: 1; letter-spacing: 0.01em;
    max-width: 96px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  /* Context chips row — layout only; the chip itself (.ctx-chip/.cc-label)
     lives in app.css, shared w/ the Welcome launchpad (one chip dialect). */
  .ctx-chips {
    display: flex; align-items: center; gap: 4px;
    margin: 0 2px 6px;
    min-width: 0;
  }

  /* #87: ghost prompt-suggestion chip (CLI --prompt-suggestions). Quiet by
     default — faint text + dashed hairline on the well fill; accent on hover. */
  .ps-row {
    display: flex; align-items: center; gap: 4px;
    padding: 7px 10px 0;
    min-width: 0;
  }
  .ps-chip {
    display: inline-flex; align-items: center; gap: 6px;
    min-width: 0;
    padding: 4px 10px;
    background: color-mix(in oklch, var(--bg-elev-2) 55%, transparent);
    border: 1px dashed color-mix(in oklch, var(--border) 70%, transparent);
    border-radius: 999px;
    color: var(--fg-muted);
    cursor: pointer; font: inherit; font-size: 11.5px; line-height: 1.2;
    transition: color var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out, background var(--dur-fast) ease-out;
  }
  .ps-chip:hover {
    color: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 45%, transparent);
    background: var(--accent-soft);
  }
  .ps-chip :global(svg) { flex-shrink: 0; opacity: 0.85; }
  .ps-text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ps-dismiss {
    display: inline-flex; align-items: center; justify-content: center;
    width: 18px; height: 18px; flex-shrink: 0;
    background: transparent; border: none; border-radius: 6px;
    color: var(--fg-faint); cursor: pointer;
    transition: color var(--dur-fast) ease-out, background var(--dur-fast) ease-out;
  }
  .ps-dismiss:hover { color: var(--fg); background: color-mix(in oklch, var(--bg-elev-2) 80%, transparent); }

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
