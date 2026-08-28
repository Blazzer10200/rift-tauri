<script lang="ts">
  // The "Claude is asking you" card. Extracted verbatim from ToolChip so the
  // chip stays a router; this owns all AskUser state, the store round-trip, and
  // the emerald card styling. The parent `.chip.as-card.is-ask` frame (border,
  // max-width, pending pulse) lives in ToolChip — this renders the head + body.
  import { Loader2, CheckCircle2, AlertCircle, HelpCircle, Square, Circle } from "@lucide/svelte";
  import { assistant, type ToolBlock } from "../../../state/assistant.svelte";
  import { parseAskQuestions } from "../../../state/assistant/askQuestions";
  import { parseAskUserResult } from "../stream/streamModel";

  let { tool, expanded = true }: { tool: ToolBlock; expanded?: boolean } = $props();

  // ── AskUser state — single-select index OR multi-select set per question.
  //    `otherText` holds the freeform input when the user picks "Other".
  //    Lenient shared parser — coerces sloppy model shapes (string options,
  //    JSON-string questions) instead of rendering an empty card.
  const askQuestions = $derived(parseAskQuestions(tool.input));
  // Per-question UI state — index by position. "Other" is a sentinel value
  // outside the option indices; selecting it reveals the freeform input.
  const OTHER_IDX = -1;
  let askSingleIdx = $state<number[]>([]);
  let askMultiSet = $state<Set<number>[]>([]);
  let askOtherText = $state<string[]>([]);
  $effect(() => {
    // Grow the per-question arrays as questions stream in, but PRESERVE any
    // answers already made. askQuestions is $derived from streaming tool input,
    // so it churns token-by-token while the tool_use block fills in — and the
    // count can tick up (Q1 done, Q2 arrives). Rebuilding wholesale erased a
    // selection the user made on Q1 before Q2 finished streaming; index-aligned
    // growth keeps it. Shrinking (rare) just truncates the tail.
    const n = askQuestions.length;
    if (n === askSingleIdx.length) return;
    askSingleIdx = Array.from({ length: n }, (_, i) => askSingleIdx[i] ?? -2);
    askMultiSet = Array.from({ length: n }, (_, i) => askMultiSet[i] ?? new Set<number>());
    askOtherText = Array.from({ length: n }, (_, i) => askOtherText[i] ?? "");
  });
  // Submission state — flips on submit, reset by tool_result via parent.
  let askSubmitting = $state(false);
  let askError = $state<string | null>(null);
  const askRequestId = $derived(assistant.askUserRequestIdFor(tool.id));
  // Answered iff the tool_result has landed (status === "done").
  const askAnswered = $derived(tool.status === "done");
  // Parse the backend's "Q:/A:" answered text into structured pairs so the
  // answered card renders clean question + answer-chip rows instead of a raw
  // <pre> dump. Falls back to the <pre> when parse yields nothing (dismissal /
  // unparseable text).
  const answeredPairs = $derived(askAnswered ? parseAskUserResult(tool.result) : []);
  const askDismissed = $derived(askAnswered && /^User dismissed the question/i.test(tool.result ?? ""));

  function toggleAskMulti(qi: number, oi: number) {
    const cur = askMultiSet[qi] ?? new Set<number>();
    const next = new Set(cur);
    if (next.has(oi)) next.delete(oi); else next.add(oi);
    askMultiSet = askMultiSet.map((s, i) => (i === qi ? next : s));
  }

  /** Compose the answers payload and dispatch to the store. */
  async function submitAskUser() {
    if (askSubmitting || !askRequestId) return;
    const answers = askQuestions.map((q, qi) => {
      if (q.multiSelect) {
        const set = askMultiSet[qi] ?? new Set<number>();
        const otherText = askOtherText[qi]?.trim();
        const labels: string[] = [];
        for (const oi of set) {
          if (oi === OTHER_IDX) {
            if (otherText) labels.push(otherText);
          } else {
            const label = q.options[oi]?.label;
            if (label) labels.push(label);
          }
        }
        return { question: q.question, answer: labels };
      }
      const idx = askSingleIdx[qi];
      if (idx === OTHER_IDX) {
        return { question: q.question, answer: askOtherText[qi]?.trim() || "(no answer)" };
      }
      const label = q.options[idx]?.label ?? "(no answer)";
      return { question: q.question, answer: label };
    });
    askSubmitting = true;
    askError = null;
    try {
      await assistant.submitAskUserAnswer(tool.id, { answers });
      askSubmitting = false;
    } catch (e) {
      console.warn("submitAskUserAnswer failed", e);
      askError = e instanceof Error ? e.message : "Submit failed — please retry.";
      askSubmitting = false; // let the user retry
    }
  }

  async function cancelAskUser() {
    if (askSubmitting || !askRequestId) return;
    askSubmitting = true;
    askError = null;
    try {
      await assistant.submitAskUserAnswer(tool.id, { cancelled: true });
      askSubmitting = false;
    } catch (e) {
      console.warn("cancelAskUser failed", e);
      askError = e instanceof Error ? e.message : "Dismiss failed — please retry.";
      askSubmitting = false;
    }
  }

  const askCanSubmit = $derived.by<boolean>(() => {
    if (askQuestions.length === 0) return false;
    for (let qi = 0; qi < askQuestions.length; qi++) {
      const q = askQuestions[qi];
      if (q.multiSelect) {
        const set = askMultiSet[qi] ?? new Set<number>();
        if (set.size === 0) return false;
        if (set.has(OTHER_IDX) && !askOtherText[qi]?.trim()) return false;
      } else {
        const idx = askSingleIdx[qi];
        if (idx === undefined || idx === -2) return false;
        if (idx === OTHER_IDX && !askOtherText[qi]?.trim()) return false;
      }
    }
    return true;
  });
</script>

<!-- AskUser card head — purple-ish meta tone, "Question" pill + status. -->
<div class="ask-head">
  <span class="ask-icon"><HelpCircle size={14} /></span>
  <span class="ask-pill">{askQuestions.length > 1 ? `${askQuestions.length} Questions` : "Question"}</span>
  {#if askAnswered}
    <span class="ask-status-text answered">answered</span>
  {:else if askSubmitting}
    <span class="ask-status-text submitting">sending…</span>
  {:else if !askRequestId}
    <span class="ask-status-text waiting">connecting…</span>
  {:else}
    <span class="ask-status-text awaiting">awaiting reply</span>
  {/if}
  <span class="chip-status">
    {#if askAnswered}<CheckCircle2 size={12} />
    {:else if tool.status === "error"}<AlertCircle size={12} />
    {:else}<Loader2 size={12} class="chip-spin" />{/if}
  </span>
</div>

{#if expanded}
  <div class="ask-body">
    {#if askAnswered}
      <!-- Final state — parse the model's "Q:/A:" tool_result into clean
           question + answer-chip rows; fall back to the raw text on a
           dismissal or anything unparseable. -->
      {#if askDismissed}
        <div class="ask-empty">Dismissed — no answer given.</div>
      {:else if answeredPairs.length > 0}
        <div class="ask-answered">
          {#each answeredPairs as pair, pi (pi)}
            <div class="ask-answered-row">
              <div class="ask-answered-q">{pair.question}</div>
              <div class="ask-answered-a">
                {#each pair.answers as ans, ai (ai)}
                  <span class="ask-answered-chip">{ans}</span>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {:else if tool.result}
        <pre class="ask-result">{tool.result}</pre>
      {:else}
        <div class="ask-empty">(no answer recorded)</div>
      {/if}
    {:else}
      {#each askQuestions as q, qi (qi)}
        <div class="ask-question">
          {#if q.header}<span class="ask-q-header">{q.header}</span>{/if}
          <div class="ask-q-text">{q.question}</div>
          <div class="ask-options" role={q.multiSelect ? "group" : "radiogroup"} aria-label={q.question}>
            {#each q.options as opt, oi (oi)}
              {@const selected =
                q.multiSelect
                  ? (askMultiSet[qi] ?? new Set()).has(oi)
                  : askSingleIdx[qi] === oi}
              <button
                type="button"
                class="ask-option"
                class:selected
                disabled={askSubmitting || askAnswered}
                role={q.multiSelect ? "checkbox" : "radio"}
                aria-checked={selected}
                onclick={() => {
                  if (q.multiSelect) {
                    toggleAskMulti(qi, oi);
                  } else {
                    askSingleIdx = askSingleIdx.map((v, i) => (i === qi ? oi : v));
                  }
                }}
              >
                <span class="ask-opt-marker" aria-hidden="true">
                  {#if q.multiSelect}
                    {#if selected}<CheckCircle2 size={12} />{:else}<Square size={12} />{/if}
                  {:else}
                    {#if selected}<CheckCircle2 size={12} />{:else}<Circle size={12} />{/if}
                  {/if}
                </span>
                <span class="ask-opt-text">
                  <span class="ask-opt-label">{opt.label}</span>
                  {#if opt.description}
                    <span class="ask-opt-desc">{opt.description}</span>
                  {/if}
                </span>
              </button>
            {/each}
            <!-- "Other" — auto-added per AskUserQuestion contract.
                 {#if true} scopes the {@const} (required: @const must be an
                 immediate child of a block, not a sibling after the {#each}). -->
            {#if true}
            {@const otherSelected =
              q.multiSelect
                ? (askMultiSet[qi] ?? new Set()).has(OTHER_IDX)
                : askSingleIdx[qi] === OTHER_IDX}
            <button
              type="button"
              class="ask-option ask-option-other"
              class:selected={otherSelected}
              disabled={askSubmitting || askAnswered}
              role={q.multiSelect ? "checkbox" : "radio"}
              aria-checked={otherSelected}
              onclick={() => {
                if (q.multiSelect) {
                  toggleAskMulti(qi, OTHER_IDX);
                } else {
                  askSingleIdx = askSingleIdx.map((v, i) => (i === qi ? OTHER_IDX : v));
                }
              }}
            >
              <span class="ask-opt-marker" aria-hidden="true">
                {#if q.multiSelect}
                  {#if otherSelected}<CheckCircle2 size={12} />{:else}<Square size={12} />{/if}
                {:else}
                  {#if otherSelected}<CheckCircle2 size={12} />{:else}<Circle size={12} />{/if}
                {/if}
              </span>
              <span class="ask-opt-text">
                <span class="ask-opt-label">Other (custom)</span>
              </span>
            </button>
            {#if otherSelected}
              <input
                type="text"
                class="ask-other-input"
                placeholder="Type your answer…"
                disabled={askSubmitting || askAnswered}
                bind:value={askOtherText[qi]}
              />
            {/if}
            {/if}
          </div>
        </div>
      {/each}
      <div class="ask-actions">
        <button
          type="button"
          class="ask-btn cancel"
          disabled={askSubmitting || !askRequestId}
          onclick={cancelAskUser}
        >Dismiss</button>
        <button
          type="button"
          class="ask-btn submit"
          disabled={!askCanSubmit || askSubmitting || !askRequestId}
          onclick={submitAskUser}
        >
          {#if askSubmitting}<Loader2 size={11} class="chip-spin" /> Sending…
          {:else}Submit{/if}
        </button>
      </div>
      {#if askError}
        <div class="ask-hint" style="color:var(--danger)">{askError}</div>
      {:else if !askRequestId}
        <div class="ask-hint">Connecting to the chat session…</div>
      {:else if askQuestions.length === 1}
        <div class="ask-hint">Or just type in the composer below — your message becomes the answer.</div>
      {/if}
    {/if}
  </div>
{/if}

<style>
  /* AskUser card — emerald-only, ties the "Claude is asking you" card to the
     same --accent vocabulary as the avatar, rail, and composer. The outer
     `.chip.as-card.is-ask` frame (border / max-width / pending pulse) lives in
     ToolChip; --ask resolves on `.chip` and descendants inherit it. */
  .ask-head {
    display: flex; align-items: center; gap: 9px;
    padding: 7px 12px;
    border-bottom: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
  }
  .ask-icon {
    display: inline-flex;
    color: var(--ask);
    flex-shrink: 0;
  }
  .ask-pill {
    display: inline-flex; align-items: center;
    padding: 2px 9px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--ask) 16%, transparent);
    border: 1px solid color-mix(in oklch, var(--ask) 38%, var(--border));
    color: color-mix(in oklch, var(--ask) 72%, var(--fg));
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    flex-shrink: 0;
  }
  .ask-status-text {
    margin-left: auto;
    font-size: 10.5px;
    color: var(--fg-muted);
    font-variant: small-caps;
    letter-spacing: 0.04em;
  }
  /* Answered = done = the one place green (success) is semantically right. */
  .ask-status-text.answered { color: var(--ok); font-weight: 600; }
  .ask-status-text.submitting { color: var(--ask); }
  .ask-status-text.waiting { color: var(--fg-faint); font-style: italic; }
  .ask-status-text.awaiting { color: color-mix(in oklch, var(--ask) 70%, var(--fg-muted)); }

  .ask-body {
    padding: 11px 13px 13px;
    display: flex; flex-direction: column;
    gap: 12px;
  }
  .ask-question {
    display: flex; flex-direction: column;
    gap: 5px;
  }
  .ask-q-header {
    align-self: flex-start;
    padding: 1px 7px;
    border-radius: 4px;
    background: color-mix(in oklch, var(--ask) 13%, transparent);
    color: color-mix(in oklch, var(--ask) 65%, var(--fg));
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .ask-q-text {
    color: var(--fg);
    font-size: 13px;
    font-weight: 500;
    line-height: 1.4;
  }
  .ask-options {
    display: flex; flex-direction: column;
    gap: 4px;
    margin-top: 2px;
  }
  .ask-option {
    display: grid;
    grid-template-columns: 16px 1fr;
    align-items: start;
    gap: 9px;
    padding: 6px 11px;
    background: color-mix(in oklch, var(--bg-elev-1) 60%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 55%, transparent);
    border-radius: var(--radius-sm);
    text-align: left;
    cursor: pointer;
    transition: background var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out, transform 80ms ease-out;
    color: var(--fg-2);
    font: inherit;
    width: 100%;
  }
  .ask-option:hover:not(:disabled) {
    background: var(--surface-hover);
    border-color: color-mix(in oklch, var(--ask) 35%, var(--border));
  }
  .ask-option:active:not(:disabled) { transform: translateY(1px); }
  .ask-option:disabled { opacity: 0.55; cursor: default; }
  .ask-option.selected {
    background: color-mix(in oklch, var(--ask) 13%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--ask) 55%, var(--border));
    color: var(--fg);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--ask) 22%, transparent);
  }
  .ask-option.selected .ask-opt-marker { color: var(--ask); }
  .ask-opt-marker {
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--fg-faint);
    padding-top: 1px;
    flex-shrink: 0;
  }
  .ask-opt-text {
    display: flex; flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .ask-opt-label {
    font-size: 12.5px;
    line-height: 1.35;
    font-weight: 500;
    word-wrap: break-word;
  }
  .ask-opt-desc {
    font-size: 11px;
    line-height: 1.4;
    color: var(--fg-muted);
    word-wrap: break-word;
  }
  .ask-option-other .ask-opt-label { font-style: italic; }
  .ask-other-input {
    margin-top: 2px;
    padding: 6px 10px;
    background: var(--bg-elev-1);
    border: 1px solid color-mix(in oklch, var(--ask) 38%, var(--border));
    border-radius: 6px;
    color: var(--fg);
    font: inherit;
    font-size: 12px;
    outline: none;
    transition: border-color var(--dur-fast) ease-out, box-shadow var(--dur-fast) ease-out;
  }
  .ask-other-input:focus {
    border-color: color-mix(in oklch, var(--ask) 65%, transparent);
    box-shadow: 0 0 0 2px color-mix(in oklch, var(--ask) 14%, transparent);
  }

  .ask-actions {
    display: flex; gap: 8px; justify-content: flex-end;
    margin-top: 2px;
  }
  .ask-btn {
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elev-1);
    color: var(--fg-2);
    font: inherit;
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
    display: inline-flex; align-items: center; gap: 5px;
    transition: background var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out, color var(--dur-fast) ease-out;
  }
  .ask-btn:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--fg);
  }
  .ask-btn:disabled { opacity: 0.5; cursor: default; }
  .ask-btn.submit {
    background: color-mix(in oklch, var(--ask) 20%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--ask) 50%, var(--border));
    color: color-mix(in oklch, var(--ask) 82%, var(--fg));
  }
  .ask-btn.submit:hover:not(:disabled) {
    background: color-mix(in oklch, var(--ask) 30%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--ask) 68%, var(--border));
    color: var(--fg);
  }
  .ask-btn.submit :global(.chip-spin) { animation: chip-spin 1s linear infinite; }
  .ask-hint {
    font-size: 10.5px;
    color: var(--fg-muted);
    font-style: italic;
    text-align: right;
  }

  .ask-answered {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .ask-answered-row {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .ask-answered-q {
    font-size: 12px;
    color: var(--fg-2);
    line-height: 1.4;
  }
  .ask-answered-a {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
  }
  /* Answered chip: greyed/neutral, not accent (CC-UI ref §5 — a resolved question
     must not keep reading as live in scrollback). Matches StreamAskUser's answered
     collapse so both AskUser paths settle the same way. */
  .ask-answered-chip {
    padding: 2px 9px;
    border-radius: 5px;
    background: color-mix(in oklch, var(--fg) 6%, transparent);
    border: 1px solid var(--border);
    color: var(--fg-muted);
    font-size: 11.5px;
    font-weight: 600;
  }
  .ask-result {
    margin: 0;
    padding: 8px 10px;
    background: var(--bg-elev-1);
    border: 1px solid color-mix(in oklch, var(--ask) 25%, var(--border));
    border-radius: 6px;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    line-height: 1.55;
    color: var(--fg-2);
    white-space: pre-wrap;
    word-wrap: break-word;
    max-height: 240px;
    overflow: auto;
  }
  .ask-empty {
    font-size: 11.5px;
    color: var(--fg-muted);
    font-style: italic;
    padding: 4px 2px;
  }
  /* chip-spin is scoped per-component in Svelte — define it here (sibling cards
     do the same) so both the head status spinner and the submit-button spinner
     rotate. */
  .chip-status :global(.chip-spin) { animation: chip-spin 1s linear infinite; }
  @keyframes chip-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }
</style>
