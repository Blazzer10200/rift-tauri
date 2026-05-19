<script lang="ts">
  import { Send, Square, X, Mic, Loader2 } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { stt } from "../../state/stt.svelte";
  import { tick, onMount } from "svelte";

  // Mic-button visibility binds to stt.config.enabled, so load the backend
  // stt config eagerly — otherwise users with STT enabled wouldn't see the
  // mic until they opened Settings → Speech once.
  onMount(() => { void stt.init(); });

  let { onsubmit }: { onsubmit: (text: string) => void } = $props();

  let ta = $state<HTMLTextAreaElement | undefined>();

  type SlashCmd = { name: string; desc: string };
  // Grouped: conversation lifecycle → model + composition → flow control → info.
  // `/clear` is intentionally NOT listed — it's an alias of /new and surfacing
  // both clutters the picker. runSlash() still accepts it.
  const SLASH_COMMANDS: SlashCmd[] = [
    { name: "new",     desc: "Start a new conversation (saves current)" },
    { name: "history", desc: "Open conversation history" },
    { name: "model",   desc: "Switch model — opens picker" },
    { name: "retry",   desc: "Re-fire the last prompt" },
    { name: "copy",    desc: "Copy last response to clipboard" },
    { name: "stop",    desc: "Halt the current turn" },
    { name: "tools",   desc: "List available workspace tools" },
    { name: "cost",    desc: "Show session cost" },
    { name: "help",    desc: "List slash commands" },
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
    { id: "sonnet", label: "Sonnet", version: "4.6", tagline: "Balanced speed + quality — the default", ctx: "200K ctx" },
    { id: "opus",   label: "Opus",   version: "4.7", tagline: "Heavy reasoning — slower, ~5× cost",     ctx: "1M ctx"   },
    { id: "haiku",  label: "Haiku",  version: "4.5", tagline: "Fastest, cheapest — quick edits & lookups", ctx: "200K ctx" },
  ];

  function autosize() {
    if (!ta) return;
    ta.style.height = "auto";
    ta.style.height = Math.min(ta.scrollHeight, 220) + "px";
  }

  $effect(() => {
    const _v = assistant.composerDraft;
    void _v;
    void tick().then(() => {
      autosize();
      if (assistant.composerDraft && ta) ta.focus();
    });
  });

  // ── `@`-file mention picker ────────────────────────────────────────────
  // Triggers when the caret is just past an `@` token, e.g. `look at @sr|`
  // matches but `you@example.com` does not (preceding char must be ws/start).
  // Insert path as plain text — Claude resolves via Read tool from there.
  type MentionState = { start: number; query: string };
  function detectMention(): MentionState | null {
    if (!ta) return null;
    const draft = assistant.composerDraft;
    const caret = ta.selectionStart ?? draft.length;
    if (ta.selectionStart !== ta.selectionEnd) return null;
    let i = caret - 1;
    while (i >= 0) {
      const ch = draft[i];
      if (ch === "@") {
        const prev = i === 0 ? " " : draft[i - 1];
        if (!/\s/.test(prev) && i !== 0) return null;
        return { start: i, query: draft.slice(i + 1, caret) };
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
    const draft = assistant.composerDraft;
    const caret = ta.selectionStart ?? draft.length;
    const before = draft.slice(0, mentionState.start);
    const after = draft.slice(caret);
    const insertion = `@${path} `;
    assistant.composerDraft = before + insertion + after;
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
      assistant.composerDraft.startsWith("/") &&
      !assistant.composerDraft.includes(" ") &&
      assistant.composerDraft.length >= 1,
  );
  const slashFiltered = $derived.by(() => {
    const q = assistant.composerDraft.slice(1).toLowerCase();
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

  function pickSlash(c: SlashCmd) {
    if (c.name === "model") {
      // Open the model picker instead of inserting `/model ` text.
      assistant.composerDraft = "";
      stt.consume();
      modelPickerOpen = true;
      void tick().then(() => ta?.focus());
      return;
    }
    // Direct-fire commands skip the textarea round-trip entirely.
    assistant.composerDraft = "";
    stt.consume();
    onsubmit(`/${c.name}`);
    void tick().then(autosize);
  }

  function pickModel(m: ModelOpt) {
    assistant.setModel(m.id);
    modelPickerOpen = false;
    void tick().then(() => ta?.focus());
  }

  function fire() {
    const text = assistant.composerDraft.trim();
    // Allow attachments-only sends (paste-and-go); only block if both empty.
    if (!text && assistant.composerAttachments.length === 0) return;
    assistant.composerDraft = "";
    stt.consume();
    onsubmit(text);
    void tick().then(autosize);
  }

  // S88: mic toggle. The stt store writes recognized text directly into
  // `assistant.composerDraft` as it arrives (interim + final), so we just
  // start/stop and let the autosizer catch up. Composer focus is restored
  // on stop so the user can hit Enter without an extra click.
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
      const _v = assistant.composerDraft;
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
        });
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
    const empty = assistant.composerDraft.length === 0;
    const atStart = ta?.selectionStart === 0 && ta?.selectionEnd === 0;
    if (!modelPickerOpen && !slashOpen && !mentionState && (empty || atStart)) {
      if (e.key === "ArrowUp") {
        const next = assistant.recallPrompt(recallOffset + 1);
        if (next !== null) {
          e.preventDefault();
          recallOffset += 1;
          assistant.composerDraft = next;
          void tick().then(() => {
            autosize();
            if (ta) ta.setSelectionRange(next.length, next.length);
          });
          return;
        }
      } else if (e.key === "ArrowDown" && recallOffset >= 0) {
        e.preventDefault();
        recallOffset -= 1;
        const prev = recallOffset < 0 ? "" : assistant.recallPrompt(recallOffset);
        assistant.composerDraft = prev ?? "";
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
        assistant.composerDraft = "";
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
    if (assistant.streaming && assistant.composerDraft.trim().length === 0) {
      void assistant.stop();
      return;
    }
    fire();
  }

  // Three modes for the action button:
  //   idle + draft           → Send
  //   streaming + empty      → Stop (kill the running turn)
  //   streaming + draft      → Queue (append to message queue)
  const mode = $derived.by<"send" | "stop" | "queue">(() => {
    const hasDraft = assistant.composerDraft.trim().length > 0;
    if (assistant.streaming && !hasDraft) return "stop";
    if (assistant.streaming && hasDraft) return "queue";
    return "send";
  });
  const canFire = $derived(
    mode === "stop" ||
      ((assistant.composerDraft.trim().length > 0 || assistant.composerAttachments.length > 0) &&
        (assistant.auth?.pill === "green" || assistant.auth?.pill === "yellow")),
  );
</script>

<div class="composer-wrap">
  {#if assistant.queue.length > 0}
    <div class="queue">
      <span class="queue-label">Queued ({assistant.queue.length}):</span>
      {#each assistant.queue as q (q.id)}
        <span class="qpill" title={q.text}>
          <span class="qtext">{q.text}</span>
          <button class="qx" type="button" onclick={() => assistant.removeQueued(q.id)} aria-label="Remove">
            <X size={11} />
          </button>
        </span>
      {/each}
      {#if assistant.queue.length >= 2}
        <button class="qclear" type="button" onclick={() => assistant.clearQueue()}>
          Clear all
        </button>
      {/if}
    </div>
  {/if}

  {#if assistant.composerAttachments.length > 0 || attachError}
    <div class="attachments">
      {#each assistant.composerAttachments as a (a.id)}
        <div class="attach-chip" title={`${a.mime} · ${fmtSize(a.sizeBytes)}`}>
          <img class="attach-thumb" src={a.previewUrl} alt="pasted attachment" />
          <span class="attach-meta">
            <span class="attach-name">image</span>
            <span class="attach-size">{fmtSize(a.sizeBytes)}</span>
          </span>
          <button
            class="attach-x"
            type="button"
            onclick={() => assistant.removeAttachment(a.id)}
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

  <div class="composer-shell">
    {#if slashOpen && slashFiltered.length > 0}
      <div class="slash-menu" role="listbox">
        {#each slashFiltered as c, i (c.name)}
          <button
            type="button"
            class="slash-item"
            class:active={i === slashIdx}
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
        <div class="model-header">Select model</div>
        {#each MODEL_OPTIONS as m, i (m.id)}
          <button
            type="button"
            class="slash-item model-item"
            class:active={i === modelIdx}
            class:current={m.id === assistant.model}
            onmousedown={(e) => { e.preventDefault(); pickModel(m); }}
          >
            <span class="model-check">{m.id === assistant.model ? "✓" : ""}</span>
            <span class="model-name">
              <span class="model-label">{m.label}</span>
              <span class="model-version">{m.version}</span>
            </span>
            <span class="slash-desc">{m.tagline}</span>
            <span class="model-ctx">{m.ctx}</span>
          </button>
        {/each}
        <div class="slash-hint">↑↓ select · Enter pick · Esc cancel</div>
      </div>
    {/if}

    <div class="composer" class:streaming={assistant.streaming}>
      {#if stt.config.enabled && stt.supported}
      <button
        class="micbtn"
        class:recording={stt.recording}
        class:transcribing={stt.transcribing}
        type="button"
        onclick={toggleMic}
        disabled={micBusy || stt.transcribing}
        title={stt.recording ? "Stop recording" : stt.transcribing ? "Transcribing…" : "Dictate (speech-to-text)"}
        aria-label={stt.recording ? "Stop recording" : "Start recording"}
      >
        {#if stt.transcribing}
          <Loader2 size={14} class="mic-spin" />
        {:else if stt.recording}
          <Square size={12} fill="currentColor" />
        {:else}
          <Mic size={14} />
        {/if}
      </button>
      {/if}
      <textarea
        bind:this={ta}
        bind:value={assistant.composerDraft}
        oninput={() => { resetRecall(); autosize(); refreshMention(); }}
        onkeyup={refreshMention}
        onclick={refreshMention}
        onblur={() => { mentionState = null; }}
        onkeydown={onKey}
        onpaste={onPaste}
        placeholder={assistant.streaming
          ? "Type to queue another message — Enter sends, /stop halts"
          : assistant.composerAttachments.length > 0
          ? "Add a question or hit Send to ask about the image"
          : "Ask Claude — paste images, or type / for commands"}
        rows="1"
      ></textarea>
      <button
        class="sendbtn"
        class:stop={mode === "stop"}
        class:queue={mode === "queue"}
        type="button"
        onclick={onBtnClick}
        disabled={!canFire}
        title={mode === "stop" ? "Stop current turn" : mode === "queue" ? "Queue this message" : "Send (Enter)"}
      >
        <span class="icon-stack">
          <span class="icon-slot" class:active={mode === "send" || mode === "queue"}><Send size={13} /></span>
          <span class="icon-slot" class:active={mode === "stop"}><Square size={13} fill="currentColor" /></span>
        </span>
      </button>
    </div>
  </div>

  <div class="hint">
    <span><kbd>Enter</kbd> send</span>
    <span><kbd>Shift</kbd>+<kbd>Enter</kbd> newline</span>
    <span><kbd>/</kbd> commands</span>
    <button
      type="button"
      class="model-pill"
      onclick={() => { modelPickerOpen = !modelPickerOpen; void tick().then(() => ta?.focus()); }}
      title="Switch model"
    >
      {#if currentModel}
        <span class="pill-label">{currentModel.label}</span>
        <span class="pill-version">{currentModel.version}</span>
      {:else}
        model: {assistant.model}
      {/if}
    </button>
  </div>
</div>

<style>
  .composer-wrap {
    padding: 10px 18px 14px;
    max-width: 860px;
    margin: 0 auto;
    width: 100%;
    box-sizing: border-box;
  }
  .composer-shell { position: relative; }
  .composer {
    display: flex; align-items: flex-end; gap: 8px;
    padding: 8px 8px 8px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    transition: border-color 140ms ease-out, box-shadow 140ms ease-out;
  }
  .composer:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .composer.streaming {
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
  }
  textarea {
    flex: 1;
    resize: none;
    min-height: 22px; max-height: 220px;
    padding: 6px 0;
    background: transparent;
    border: 0; outline: none;
    color: var(--fg);
    font: inherit;
    font-size: var(--fs-md);
    line-height: 1.5;
    overflow-y: auto;
  }
  textarea::placeholder { color: var(--fg-subtle); }

  .sendbtn {
    position: relative;
    width: 32px; height: 32px;
    display: flex; align-items: center; justify-content: center;
    background: var(--accent);
    color: var(--accent-fg);
    border: 0; border-radius: 10px;
    cursor: pointer;
    transition: background 200ms ease-out, transform 140ms ease-out, opacity 140ms ease-out, color 200ms ease-out;
    flex-shrink: 0;
    overflow: hidden;
  }
  .sendbtn:hover:not(:disabled) { background: var(--accent-hover); transform: scale(1.04); }
  .sendbtn:disabled { opacity: 0.4; cursor: default; }
  .sendbtn.stop {
    background: var(--danger);
    color: oklch(0.98 0.01 22);
  }
  .sendbtn.stop:hover { filter: brightness(1.1); }

  .micbtn {
    width: 30px; height: 30px;
    display: inline-flex; align-items: center; justify-content: center;
    background: transparent;
    color: var(--fg-muted);
    border: 1px solid var(--border);
    border-radius: 10px;
    cursor: pointer;
    flex-shrink: 0;
    align-self: flex-end;
    transition: color 140ms, border-color 140ms, background 140ms, box-shadow 140ms;
  }
  .micbtn:hover:not(:disabled) { color: var(--fg); border-color: var(--border-strong); background: var(--surface-hover); }
  .micbtn:disabled { opacity: 0.55; cursor: default; }
  .micbtn.recording {
    background: var(--danger);
    color: oklch(0.98 0.01 22);
    border-color: var(--danger);
    animation: mic-pulse 1.1s ease-in-out infinite;
  }
  .micbtn.transcribing {
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
  }
  :global(.mic-spin) { animation: mic-spin 0.9s linear infinite; }
  @keyframes mic-spin { to { transform: rotate(360deg); } }
  @keyframes mic-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklch, var(--danger) 45%, transparent); }
    50%      { box-shadow: 0 0 0 6px transparent; }
  }
  .sendbtn.queue {
    background: color-mix(in oklch, var(--accent) 70%, var(--surface));
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
    background: color-mix(in oklch, var(--accent) 8%, var(--surface));
    border: 1px dashed color-mix(in oklch, var(--accent) 35%, var(--border));
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
    bottom: calc(100% + 6px);
    left: 0;
    width: 100%;
    max-height: 240px;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 10px 24px oklch(0 0 0 / 0.35);
    padding: 4px;
    z-index: 10;
    animation: slash-in 140ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  .slash-item {
    display: flex; align-items: baseline; gap: 10px;
    width: 100%;
    padding: 6px 10px;
    background: transparent;
    border: 0; border-radius: 6px;
    color: var(--fg);
    text-align: left;
    cursor: pointer;
    font: inherit;
  }
  .slash-item:hover, .slash-item.active {
    background: color-mix(in oklch, var(--accent) 14%, transparent);
  }
  .slash-name {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--fs-sm);
    font-weight: 600;
    color: var(--accent);
    min-width: 64px;
  }
  .slash-desc {
    font-size: var(--fs-xs);
    color: var(--fg-muted);
  }
  .slash-hint {
    padding: 6px 10px 2px;
    font-size: 10px;
    color: var(--fg-faint);
    border-top: 1px solid var(--border);
    margin-top: 2px;
  }

  .hint {
    margin-top: 7px;
    display: flex; gap: 14px; align-items: center;
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    padding-left: 4px;
  }
  .hint kbd {
    font-family: inherit;
    font-size: 10px;
    padding: 1px 5px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--fg-muted);
  }
  .model-pill {
    margin-left: auto;
    display: inline-flex; align-items: center; gap: 5px;
    padding: 2px 4px 2px 9px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--fg-2);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    font: inherit;
    font-size: var(--fs-xs);
    transition: background 140ms ease-out, color 140ms ease-out, border-color 140ms ease-out;
  }
  .pill-label { font-weight: 600; }
  .pill-version {
    font-size: 10px;
    font-weight: 600;
    color: var(--accent);
    padding: 1px 6px;
    background: color-mix(in oklch, var(--accent) 12%, transparent);
    border-radius: 999px;
  }
  .model-pill:hover {
    background: color-mix(in oklch, var(--accent) 14%, var(--bg-elev-2));
    color: var(--fg);
    border-color: color-mix(in oklch, var(--accent) 40%, var(--border));
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

  .model-menu .model-header {
    padding: 6px 10px 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-faint);
  }
  .model-item {
    display: grid;
    grid-template-columns: 14px auto 1fr auto;
    align-items: center;
    gap: 12px;
    padding: 8px 12px;
  }
  .model-check {
    color: var(--accent);
    font-weight: 700;
    text-align: center;
    font-size: 11px;
  }
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
    background: var(--bg-elev-2);
    border-radius: 4px;
  }
  .model-item.current .model-label { color: var(--accent); }
  .model-item.current .model-version {
    color: var(--accent);
    background: color-mix(in oklch, var(--accent) 14%, transparent);
  }
  .model-ctx {
    font-size: 10px;
    font-weight: 600;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    padding: 2px 7px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    white-space: nowrap;
  }
  .model-item.current .model-ctx {
    color: var(--accent);
    border-color: color-mix(in oklch, var(--accent) 30%, var(--border));
    background: color-mix(in oklch, var(--accent) 8%, transparent);
  }
</style>
