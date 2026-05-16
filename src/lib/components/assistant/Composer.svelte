<script lang="ts">
  import { Send, Square, X } from "lucide-svelte";
  import { assistant } from "../../state/assistant.svelte";
  import { tick } from "svelte";

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
      modelPickerOpen = true;
      void tick().then(() => ta?.focus());
      return;
    }
    // Direct-fire commands skip the textarea round-trip entirely.
    assistant.composerDraft = "";
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
    if (!text) return;
    assistant.composerDraft = "";
    onsubmit(text);
    void tick().then(autosize);
  }

  // Up-arrow recall offset (0 = newest). Reset whenever the user types or
  // switches focus so the next Up starts at the newest prompt again.
  let recallOffset = $state(-1);
  function resetRecall() { recallOffset = -1; }

  function onKey(e: KeyboardEvent) {
    // History recall — only when neither menu is open and the textarea is
    // either empty, or cursor is at position 0 (so plain Up in a multiline
    // draft still moves the caret normally).
    const empty = assistant.composerDraft.length === 0;
    const atStart = ta?.selectionStart === 0 && ta?.selectionEnd === 0;
    if (!modelPickerOpen && !slashOpen && (empty || atStart)) {
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
      (assistant.composerDraft.trim().length > 0 &&
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
      <textarea
        bind:this={ta}
        bind:value={assistant.composerDraft}
        oninput={() => { resetRecall(); autosize(); }}
        onkeydown={onKey}
        placeholder={assistant.streaming
          ? "Type to queue another message — Enter sends, /stop halts"
          : "Ask Claude — or type / for commands"}
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
