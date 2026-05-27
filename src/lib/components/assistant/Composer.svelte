<script lang="ts">
  import { Send, Square, X, Mic, Loader2, HelpCircle } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
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

  function setDraft(v: string) { if (tab) tab.draft = v; }
  function setAttachments(
    v: { id: string; mime: string; dataBase64: string; previewUrl: string; sizeBytes: number }[],
  ) {
    if (tab) tab.attachments = v;
  }

  let ta = $state<HTMLTextAreaElement | undefined>();

  type SlashCmd = { name: string; desc: string };
  // Grouped: conversation lifecycle → model + composition → flow control → info.
  // `/clear` is intentionally NOT listed — it's an alias of /new and surfacing
  // both clutters the picker. runSlash() still accepts it.
  const SLASH_COMMANDS: SlashCmd[] = [
    { name: "new",       desc: "Start a new conversation (saves current)" },
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
    id: "sonnet" | "opus" | "haiku";
    label: string;
    version: string;
    tagline: string;
    ctx: string;
  };
  const MODEL_OPTIONS: ModelOpt[] = [
    { id: "sonnet", label: "Sonnet", version: "4.6", tagline: "Best speed + intelligence balance — the default", ctx: "1M ctx" },
    { id: "opus",   label: "Opus",   version: "4.7", tagline: "Most capable — complex reasoning + agentic coding", ctx: "1M ctx" },
    { id: "haiku",  label: "Haiku",  version: "4.5", tagline: "Fastest, near-frontier — quick edits & lookups", ctx: "200K ctx" },
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
  // `modelPickerOpen` is a separate state that flips when /model is picked
  // and replaces the command list with a model chooser.
  let modelPickerOpen = $state(false);
  const slashOpen = $derived(
    !modelPickerOpen &&
      draft.startsWith("/") &&
      !draft.includes(" ") &&
      draft.length >= 1,
  );
  const slashFiltered = $derived.by(() => {
    const q = draft.slice(1).toLowerCase();
    return SLASH_COMMANDS.filter((c) => c.name.startsWith(q));
  });
  let slashIdx = $state(0);
  let modelIdx = $state(0);
  $effect(() => {
    const _v = slashFiltered.length;
    void _v;
    slashIdx = 0;
  });
  // Re-seed the model picker cursor to the current model whenever it opens.
  $effect(() => {
    if (modelPickerOpen) {
      const i = MODEL_OPTIONS.findIndex((m) => m.id === assistant.model);
      modelIdx = i >= 0 ? i : 0;
    }
  });
  // Current model row — drives the composer's bottom-right pill label.
  const currentModel = $derived(MODEL_OPTIONS.find((m) => m.id === assistant.model));

  // Effort ladder. Haiku skips extended thinking server-side regardless, so
  // hide the pill on Haiku to avoid implying it does something. Cycle on click:
  // none → quick → deep → none. Names describe quality not speed — "Fast"
  // and "Quick" were ambiguous siblings; Instant/Smart/Deep is a real ladder.
  type EffortOpt = { id: "none" | "quick" | "deep"; label: string; hint: string; level: 1 | 2 | 3 };
  const EFFORT_OPTIONS: EffortOpt[] = [
    { id: "none",  label: "Instant", level: 1, hint: "Instant — straight to the answer, no thinking time" },
    { id: "quick", label: "Smart",   level: 2, hint: "Smart — thinks briefly before answering (~5s extra)" },
    { id: "deep",  label: "Deep",    level: 3, hint: "Deep — heavy reasoning (~15s extra) for hard problems" },
  ];
  const currentEffort = $derived(EFFORT_OPTIONS.find((e) => e.id === assistant.thinkingEffort) ?? EFFORT_OPTIONS[1]);
  function cycleEffort() {
    const i = EFFORT_OPTIONS.findIndex((e) => e.id === assistant.thinkingEffort);
    const next = EFFORT_OPTIONS[(i + 1) % EFFORT_OPTIONS.length];
    assistant.setThinkingEffort(next.id);
  }

  function pickSlash(c: SlashCmd) {
    if (c.name === "model") {
      // Open the model picker instead of inserting `/model ` text.
      setDraft("");
      stt.consume();
      modelPickerOpen = true;
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
    modelPickerOpen = false;
    void tick().then(() => ta?.focus());
  }

  // Bumps on every fire() — drives the send-button ripple + chat-column
  // upward sweep keyed off `{#key}`. Both pulses are pure-CSS one-shots,
  // mounted by the key flip and self-removed after their animation ends.
  let fireKey = $state(0);

  function fire() {
    const text = draft.trim();
    // Allow attachments-only sends (paste-and-go); only block if both empty.
    if (!text && attachments.length === 0) return;
    setDraft("");
    stt.consume();
    fireKey++;
    onsubmit(text);
    void tick().then(autosize);
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
    if (!modelPickerOpen && !slashOpen && !mentionState && (empty || atStart)) {
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
    if (modelPickerOpen) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        modelIdx = (modelIdx + 1) % MODEL_OPTIONS.length;
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        modelIdx = (modelIdx - 1 + MODEL_OPTIONS.length) % MODEL_OPTIONS.length;
        return;
      }
      if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        pickModel(MODEL_OPTIONS[modelIdx]);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        modelPickerOpen = false;
        return;
      }
      // Any other key cancels the picker so the user can type normally.
      if (e.key.length === 1) modelPickerOpen = false;
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
</script>

<div class="composer-wrap" data-model={assistant.model}>
  {#key fireKey}
    {#if fireKey > 0}
      <span class="send-sweep" aria-hidden="true"></span>
    {/if}
  {/key}
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

    {#if modelPickerOpen}
      <div class="slash-menu model-menu" role="listbox">
        <div class="model-header"><span>Model</span></div>
        {#each MODEL_OPTIONS as m, i (m.id)}
          <button
            type="button"
            class="slash-item model-item"
            class:active={i === modelIdx}
            class:current={m.id === assistant.model}
            data-id={m.id}
            style="--idx: {i}"
            onmousedown={(e) => { e.preventDefault(); pickModel(m); }}
          >
            <span class="model-dot" aria-hidden="true"></span>
            <span class="model-name">
              <span class="model-label">{m.label}</span>
              <span class="model-version">{m.version}</span>
            </span>
            <span class="slash-desc">{m.tagline}</span>
            <span class="model-ctx" class:wide={m.ctx === "1M ctx"}>{m.ctx}</span>
          </button>
        {/each}
        <div class="slash-hint model-hint">
          <span><kbd>↑↓</kbd> navigate</span>
          <span><kbd>↵</kbd> pick</span>
          <span><kbd>Esc</kbd> close</span>
        </div>
      </div>
    {/if}

    <div class="composer" class:streaming={streaming} data-mode={mode}>
      <span class="composer-aurora" aria-hidden="true"></span>
      <span class="composer-aurora-2" aria-hidden="true"></span>
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
        {:else if streaming}
          <span class="placeholder-ghost static" aria-hidden="true">Type to queue — Enter sends, /stop halts</span>
        {:else if attachments.length > 0 && draft.length === 0}
          <span class="placeholder-ghost static" aria-hidden="true">Ask about the image…</span>
        {/if}
      </div>

      <div class="composer-toolbar">
        <div class="toolbar-cluster">
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
          {#if draft.length > 0}
            <span
              class="char-count"
              class:warn={draft.length > 4000}
              use:tooltip={draft.length > 4000 ? `${draft.length.toLocaleString()} characters · long prompts may slow first reply` : `${draft.length.toLocaleString()} characters in this draft`}
            >
              {draft.length.toLocaleString()}
            </span>
          {/if}
        </div>

        <div class="toolbar-cluster toolbar-right">
          {#if assistant.model !== "haiku"}
            <button
              type="button"
              class="effort-pill"
              class:effort-none={currentEffort.id === "none"}
              class:effort-quick={currentEffort.id === "quick"}
              class:effort-deep={currentEffort.id === "deep"}
              onclick={cycleEffort}
              use:tooltip={`${currentEffort.hint}\nClick to cycle thinking depth`}
            >
              {#key currentEffort.id}
                <span class="effort-bars" aria-hidden="true" data-level={currentEffort.level}>
                  <span class="bar"></span>
                  <span class="bar"></span>
                  <span class="bar"></span>
                </span>
                <span class="pill-label">{currentEffort.label}</span>
              {/key}
            </button>
          {/if}
          <button
            type="button"
            class="model-pill"
            class:streaming
            data-model={assistant.model}
            onclick={() => { modelPickerOpen = !modelPickerOpen; void tick().then(() => ta?.focus()); }}
            use:tooltip={streaming ? `Streaming with ${currentModel?.label ?? assistant.model}` : `Switch model · current: ${currentModel?.label ?? assistant.model}`}
          >
            <span class="model-dot-mini" aria-hidden="true"></span>
            {#if currentModel}
              <span class="pill-label">{currentModel.label}</span>
              <span class="pill-version">{currentModel.version}</span>
              <span class="pill-caret" aria-hidden="true">▾</span>
            {:else}
              model: {assistant.model}
            {/if}
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
     send-sweep, model-pill pulse) reads from the same single source. */
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
     unified under .iconbtn (mic/help) + .effort-pill / .model-pill /
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
  .composer:focus-within {
    border-color: color-mix(in oklch, var(--model-color) 55%, transparent);
    box-shadow:
      0 0 0 3px color-mix(in oklch, var(--model-color) 18%, transparent),
      0 12px 32px -8px color-mix(in oklch, var(--model-color) 32%, transparent),
      inset 0 1px 0 color-mix(in oklch, white 6%, transparent);
  }
  /* ── Aurora layer ──────────────────────────────────────────────────────
     Two slow-rotating conic gradients tinted by --model-color. Layer 1
     spins clockwise, layer 2 counter-clockwise at a different period so
     the two interfere and produce a slow-drifting hue field behind the
     composer chrome. Opacity ramps with focus + streaming. */
  .composer-aurora,
  .composer-aurora-2 {
    position: absolute;
    inset: -60%;
    pointer-events: none;
    opacity: 0;
    transition: opacity 320ms ease-out;
    z-index: 0;
    filter: blur(22px);
    border-radius: 50%;
  }
  .composer-aurora {
    background: conic-gradient(
      from 0deg,
      transparent 0deg,
      color-mix(in oklch, var(--model-color) 35%, transparent) 60deg,
      transparent 140deg,
      color-mix(in oklch, var(--model-color) 22%, transparent) 220deg,
      transparent 320deg
    );
    animation: aurora-spin 16s linear infinite;
  }
  .composer-aurora-2 {
    background: conic-gradient(
      from 180deg,
      transparent 0deg,
      color-mix(in oklch, var(--model-color) 18%, transparent) 80deg,
      transparent 200deg,
      color-mix(in oklch, var(--model-color) 28%, transparent) 290deg,
      transparent 360deg
    );
    animation: aurora-spin-rev 22s linear infinite;
  }
  .composer:focus-within .composer-aurora,
  .composer:focus-within .composer-aurora-2 { opacity: 0.75; }
  .composer.streaming .composer-aurora,
  .composer.streaming .composer-aurora-2 { opacity: 0.95; }
  @keyframes aurora-spin {
    to { transform: rotate(360deg); }
  }
  @keyframes aurora-spin-rev {
    to { transform: rotate(-360deg); }
  }
  @media (prefers-reduced-motion: reduce) {
    .composer-aurora, .composer-aurora-2 { animation: none; opacity: 0.25; }
    .composer:focus-within .composer-aurora,
    .composer:focus-within .composer-aurora-2 { opacity: 0.4; }
  }

  .composer.streaming {
    border-color: color-mix(in oklch, var(--model-color) 50%, var(--border));
  }
  /* Animated top-edge streaming bar — tinted to current model. */
  .composer.streaming::before {
    content: "";
    position: absolute;
    top: 0; left: 14%; right: 14%;
    height: 1.5px;
    background: linear-gradient(90deg,
      transparent,
      var(--model-color),
      color-mix(in oklch, var(--model-color) 70%, white 30%),
      var(--model-color),
      transparent);
    background-size: 200% 100%;
    animation: composer-stream 2.6s ease-in-out infinite;
    z-index: 2;
    border-radius: 0 0 2px 2px;
  }
  @keyframes composer-stream {
    0%   { background-position: 200% 0; opacity: 0.4; }
    50%  { opacity: 0.95; }
    100% { background-position: -100% 0; opacity: 0.4; }
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
    opacity: 0.75;
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

  /* Live character count — surfaces only when draft is non-empty. Warns
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
  /* Upward chat-column sweep — a thin gradient bar shoots upward from the
     composer into the chat scroller on every fire(). Anchored to
     .composer-wrap (which is position: relative), travels 100vh up via
     translateY animation; clipped invisibly by the scroller's overflow. */
  .send-sweep {
    position: absolute;
    left: 18px; right: 18px;
    top: 0;
    height: 2px;
    border-radius: 999px;
    background: linear-gradient(90deg,
      transparent 0%,
      color-mix(in oklch, var(--model-color) 65%, transparent) 30%,
      color-mix(in oklch, var(--model-color) 85%, white 10%) 50%,
      color-mix(in oklch, var(--model-color) 65%, transparent) 70%,
      transparent 100%);
    box-shadow: 0 0 14px color-mix(in oklch, var(--model-color) 55%, transparent);
    pointer-events: none;
    z-index: 5;
    animation: send-sweep 720ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  @keyframes send-sweep {
    0%   { transform: translateY(0)     scaleX(0.3); opacity: 0; }
    18%  { transform: translateY(-12px) scaleX(1);   opacity: 1; }
    100% { transform: translateY(-72vh) scaleX(0.4); opacity: 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .send-sweep { animation: none; opacity: 0; }
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

  .model-pill {
    align-self: center;
    display: inline-flex; align-items: center; gap: 6px;
    padding: 0 8px 0 10px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 75%, transparent);
    border-radius: 999px;
    color: var(--fg-2);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-xs);
    height: 26px;
    transition: background 140ms ease-out, color 140ms ease-out, border-color 140ms ease-out;
  }
  /* Per-model dot — same color logic as the picker. Lets the user scan the
     pill and know which model is loaded without reading the label. */
  .model-dot-mini {
    width: 6px; height: 6px;
    border-radius: 999px;
    background: var(--model-color, var(--fg-muted));
    box-shadow:
      0 0 0 1.5px color-mix(in oklch, var(--model-color, var(--fg-muted)) 16%, transparent),
      0 0 6px color-mix(in oklch, var(--model-color, var(--fg-muted)) 45%, transparent);
    flex-shrink: 0;
  }
  .model-pill[data-model="sonnet"] { --model-color: oklch(0.74 0.13 230); }
  .model-pill[data-model="opus"]   { --model-color: oklch(0.70 0.18 295); }
  .model-pill[data-model="haiku"]  { --model-color: oklch(0.78 0.14 180); }
  /* Streaming = breathing glow around the pill + dot grows. Loops in
     sync (2.6s) w/ the composer top-edge streaming bar. */
  .model-pill.streaming {
    border-color: color-mix(in oklch, var(--model-color) 50%, var(--border));
    animation: model-pill-breathe 2.6s ease-in-out infinite;
  }
  .model-pill.streaming .model-dot-mini {
    animation: model-dot-breathe 2.6s ease-in-out infinite;
  }
  @keyframes model-pill-breathe {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklch, var(--model-color) 0%, transparent); }
    50%      { box-shadow: 0 0 14px color-mix(in oklch, var(--model-color) 45%, transparent); }
  }
  @keyframes model-dot-breathe {
    0%, 100% { transform: scale(1);   filter: brightness(1); }
    50%      { transform: scale(1.25); filter: brightness(1.3); }
  }
  @media (prefers-reduced-motion: reduce) {
    .model-pill.streaming,
    .model-pill.streaming .model-dot-mini { animation: none; }
  }

  .effort-pill {
    align-self: center;
    display: inline-flex; align-items: center; gap: 5px;
    padding: 0 10px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 75%, transparent);
    border-radius: 999px;
    color: var(--fg-2);
    cursor: pointer;
    font: inherit;
    font-size: 10.5px;
    font-weight: 600;
    height: 26px;
    letter-spacing: 0.02em;
    overflow: hidden;
    transition: background 140ms ease-out, color 140ms ease-out, border-color 140ms ease-out;
  }
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
  .effort-pill:hover {
    background: color-mix(in oklch, var(--accent) 14%, var(--bg-elev-2));
    color: var(--fg);
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
  }
  .effort-none {
    color: oklch(0.78 0.14 145);
    border-color: color-mix(in oklch, oklch(0.78 0.14 145) 35%, var(--border));
    background: color-mix(in oklch, oklch(0.78 0.14 145) 8%, var(--bg-elev-2));
  }
  .effort-quick {
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 35%, var(--border));
    background: color-mix(in oklch, var(--accent) 8%, var(--bg-elev-2));
  }
  .effort-deep {
    color: oklch(0.75 0.16 50);
    border-color: color-mix(in oklch, oklch(0.75 0.16 50) 40%, var(--border));
    background: color-mix(in oklch, oklch(0.75 0.16 50) 10%, var(--bg-elev-2));
  }
  .pill-label { font-weight: 600; }
  .pill-version {
    font-size: 10px;
    font-weight: 600;
    color: var(--model-color, var(--accent));
    padding: 1px 6px;
    background: color-mix(in oklch, var(--model-color, var(--accent)) 14%, transparent);
    border-radius: 999px;
  }
  .pill-caret {
    font-size: 8px;
    color: var(--fg-faint);
    margin-left: 1px;
    line-height: 1;
    transition: color 140ms ease-out, transform 140ms ease-out;
  }
  .model-pill:hover .pill-caret { color: var(--fg-muted); transform: translateY(1px); }
  .model-pill:hover {
    background: color-mix(in oklch, var(--bg-elev-2) 95%, transparent);
    color: var(--fg);
    border-color: color-mix(in oklch, var(--model-color, var(--accent)) 35%, var(--border));
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
  .model-item {
    position: relative;
    display: grid;
    grid-template-columns: 10px minmax(96px, auto) 1fr auto;
    align-items: center;
    gap: 12px;
    padding: 10px 14px 10px 18px;
    border-radius: 8px;
    transition: background 140ms ease-out;
  }
  .model-item::before {
    content: "";
    position: absolute;
    left: 4px;
    top: 50%;
    width: 3px;
    height: 60%;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 10px color-mix(in oklch, var(--accent) 55%, transparent);
    transform: translateY(-50%) scaleY(0);
    transform-origin: center;
    transition: transform 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .model-item.current::before { transform: translateY(-50%) scaleY(1); }
  .model-item.current {
    background: linear-gradient(90deg,
      color-mix(in oklch, var(--accent) 14%, transparent),
      color-mix(in oklch, var(--accent) 4%, transparent) 65%,
      transparent);
  }
  .model-item.active:not(.current) {
    background: color-mix(in oklch, var(--accent) 9%, transparent);
  }

  .model-dot {
    width: 8px; height: 8px;
    border-radius: 999px;
    background: var(--model-color, var(--fg-muted));
    box-shadow:
      0 0 0 2px color-mix(in oklch, var(--model-color, var(--fg-muted)) 16%, transparent),
      0 0 8px color-mix(in oklch, var(--model-color, var(--fg-muted)) 55%, transparent);
  }
  .model-item[data-id="sonnet"] { --model-color: oklch(0.74 0.13 230); }
  .model-item[data-id="opus"]   { --model-color: oklch(0.70 0.18 295); }
  .model-item[data-id="haiku"]  { --model-color: oklch(0.78 0.14 180); }

  .model-name {
    display: inline-flex; align-items: baseline; gap: 6px;
    min-width: 96px;
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
  .model-item.current .model-label { color: var(--accent); }
  .model-item.current .model-version {
    color: var(--accent);
    background: color-mix(in oklch, var(--accent) 16%, transparent);
  }
  .model-menu .slash-desc {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
