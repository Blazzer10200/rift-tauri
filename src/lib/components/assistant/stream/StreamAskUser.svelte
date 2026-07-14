<script lang="ts">
  // Interactive ask_user card for STREAM mode. The legacy ToolChip path had this
  // card; StreamTurn never did, so mcp__rift__ask_user silently fell through to a
  // dead WorkLine in stream mode — the question never rendered. This restores the
  // full interactive surface, reusing the same store binding/submit API.
  import { untrack } from "svelte";
  import { CheckCircle2, Circle, Square, Loader2, MessageCircleQuestion, Check } from "lucide-svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { parseAskUserResult, type StreamTool } from "./streamModel";

  let { tool }: { tool: StreamTool } = $props();

  type AskQuestion = {
    question: string;
    header: string;
    multiSelect?: boolean;
    options: Array<{ label: string; description?: string }>;
  };
  const askQuestions = $derived.by<AskQuestion[]>(() => {
    const raw = tool.input?.questions;
    if (!Array.isArray(raw)) return [];
    return (raw as Array<Record<string, unknown>>).map((q) => ({
      question: typeof q.question === "string" ? q.question : "",
      header: typeof q.header === "string" ? q.header : "",
      multiSelect: q.multiSelect === true,
      options: Array.isArray(q.options)
        ? (q.options as Array<Record<string, unknown>>).map((o) => ({
            label: typeof o.label === "string" ? o.label : "",
            description: typeof o.description === "string" ? o.description : undefined,
          })).filter((o) => o.label.length > 0)
        : [],
    }));
  });

  const OTHER_IDX = -1;
  let askSingleIdx = $state<number[]>([]);
  let askMultiSet = $state<Set<number>[]>([]);
  let askOtherText = $state<string[]>([]);
  $effect(() => {
    const n = askQuestions.length;
    untrack(() => {
      if (n === askSingleIdx.length) return;
      askSingleIdx = Array.from({ length: n }, (_, i) => askSingleIdx[i] ?? -2);
      askMultiSet = Array.from({ length: n }, (_, i) => askMultiSet[i] ?? new Set<number>());
      askOtherText = Array.from({ length: n }, (_, i) => askOtherText[i] ?? "");
    });
  });

  let askSubmitting = $state(false);
  let askError = $state<string | null>(null);
  const askRequestId = $derived(assistant.askUserRequestIdFor(tool.id));
  const askAnswered = $derived(tool.status === "done");

  // The backend tool_result is plain text Claude reads ("Q: …\nA: …" pairs, or
  // a dismissal sentence). Rendering it raw in a <pre> read as an unstyled
  // dump. Parse it back into structured {header, question, answer[]} so the
  // answered state can render as clean chips that mirror the question chrome —
  // headers pulled from tool.input (the result text only has the question body).
  const askDismissed = $derived(
    askAnswered && /^User dismissed the question/i.test(tool.result ?? ""),
  );
  const answeredPairs = $derived.by(() => {
    if (!askAnswered || askDismissed) return [];
    // Header only exists in tool.input (the result text has just the body) —
    // re-attach it by matching the parsed question against the original.
    return parseAskUserResult(tool.result).map((p) => ({
      ...p,
      header: askQuestions.find((q) => q.question === p.question)?.header ?? "",
    }));
  });

  function toggleAskMulti(qi: number, oi: number) {
    const cur = askMultiSet[qi] ?? new Set<number>();
    const next = new Set(cur);
    if (next.has(oi)) next.delete(oi); else next.add(oi);
    askMultiSet = askMultiSet.map((s, i) => (i === qi ? next : s));
  }

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
      askSubmitting = false;
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

<div class="sask" class:answered={askAnswered}>
  <div class="sask-head">
    <span class="sask-head-ic" aria-hidden="true">
      {#if askAnswered}<Check size={13} strokeWidth={2.5} />{:else}<MessageCircleQuestion size={13} />{/if}
    </span>
    <span class="sask-head-label">{askAnswered ? "Your answer" : "Rift needs your input"}</span>
  </div>

  {#if askAnswered}
    {#if askDismissed}
      <div class="sask-empty">Dismissed — no answer given.</div>
    {:else if answeredPairs.length > 0}
      {#each answeredPairs as p (p.question)}
        <div class="sask-answered">
          {#if p.header}<span class="sask-q-header">{p.header}</span>{/if}
          <div class="sask-q-text">{p.question}</div>
          <div class="sask-chips">
            {#each p.answers as a (a)}
              <span class="sask-chip"><Check size={11} strokeWidth={2.5} />{a}</span>
            {/each}
          </div>
        </div>
      {/each}
    {:else}
      <div class="sask-empty">(no answer recorded)</div>
    {/if}
  {:else}
    {#each askQuestions as q, qi (qi)}
      <div class="sask-question">
        {#if q.header}<span class="sask-q-header">{q.header}</span>{/if}
        <div class="sask-q-text">{q.question}</div>
        <div class="sask-options" role={q.multiSelect ? "group" : "radiogroup"} aria-label={q.question}>
          {#each q.options as opt, oi (oi)}
            {@const selected =
              q.multiSelect
                ? (askMultiSet[qi] ?? new Set()).has(oi)
                : askSingleIdx[qi] === oi}
            <button
              type="button"
              class="sask-option"
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
              <span class="sask-marker" aria-hidden="true">
                {#if q.multiSelect}
                  {#if selected}<CheckCircle2 size={12} />{:else}<Square size={12} />{/if}
                {:else}
                  {#if selected}<CheckCircle2 size={12} />{:else}<Circle size={12} />{/if}
                {/if}
              </span>
              <span class="sask-opt-text">
                <span class="sask-opt-label">{opt.label}</span>
                {#if opt.description}
                  <span class="sask-opt-desc">{opt.description}</span>
                {/if}
              </span>
            </button>
          {/each}
          {#if true}
          {@const otherSelected =
            q.multiSelect
              ? (askMultiSet[qi] ?? new Set()).has(OTHER_IDX)
              : askSingleIdx[qi] === OTHER_IDX}
          <button
            type="button"
            class="sask-option sask-option-other"
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
            <span class="sask-marker" aria-hidden="true">
              {#if q.multiSelect}
                {#if otherSelected}<CheckCircle2 size={12} />{:else}<Square size={12} />{/if}
              {:else}
                {#if otherSelected}<CheckCircle2 size={12} />{:else}<Circle size={12} />{/if}
              {/if}
            </span>
            <span class="sask-opt-text">
              <span class="sask-opt-label">Other (custom)</span>
            </span>
          </button>
          {#if otherSelected}
            <input
              type="text"
              class="sask-other-input"
              placeholder="Type your answer…"
              disabled={askSubmitting || askAnswered}
              bind:value={askOtherText[qi]}
            />
          {/if}
          {/if}
        </div>
      </div>
    {/each}
    <div class="sask-actions">
      <button
        type="button"
        class="sask-btn cancel"
        disabled={askSubmitting || !askRequestId}
        onclick={cancelAskUser}
      >Dismiss</button>
      <button
        type="button"
        class="sask-btn submit"
        disabled={!askCanSubmit || askSubmitting || !askRequestId}
        onclick={submitAskUser}
      >
        {#if askSubmitting}<Loader2 size={11} class="chip-spin" /> Sending…
        {:else}Submit{/if}
      </button>
    </div>
    {#if askError}
      <div class="sask-hint" style="color:var(--danger)">{askError}</div>
    {:else if tool.status === "error"}
      <div class="sask-hint">This turn ended before you answered.</div>
    {:else if !askRequestId}
      <div class="sask-hint">Connecting to the chat session…</div>
    {/if}
  {/if}
</div>

<style>
  .sask {
    display: flex;
    flex-direction: column;
    gap: 11px;
    margin: 10px 0;
    padding: 13px 15px 14px;
    border: 1px solid color-mix(in oklab, var(--accent) 38%, transparent);
    border-radius: 12px;
    background:
      linear-gradient(180deg,
        color-mix(in oklab, var(--accent) 7%, transparent),
        color-mix(in oklab, var(--accent) 3%, transparent));
    box-shadow: 0 1px 0 color-mix(in oklab, var(--accent) 10%, transparent) inset;
  }
  /* Answered: drop the call-to-action accent, settle into a quiet "done" card. */
  .sask.answered {
    border-color: var(--border, color-mix(in oklab, var(--fg) 12%, transparent));
    background: color-mix(in oklab, var(--fg) 3%, transparent);
    box-shadow: none;
  }

  /* card header — a small labelled rail so the card reads as a deliberate
     surface, not a floating button group */
  .sask-head {
    display: flex; align-items: center; gap: 7px;
    font-size: 11px; font-weight: 600; letter-spacing: 0.02em;
    color: var(--accent);
  }
  .sask.answered .sask-head { color: var(--fg-2, color-mix(in oklab, var(--fg) 62%, transparent)); }
  .sask-head-ic {
    display: grid; place-items: center; width: 19px; height: 19px;
    border-radius: 6px; flex: none;
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 13%, transparent);
  }
  .sask.answered .sask-head-ic {
    color: var(--ok);
    background: color-mix(in oklab, var(--ok) 14%, transparent);
  }

  /* answered summary — chips, not a monospace dump */
  .sask-answered { display: flex; flex-direction: column; gap: 6px; }
  .sask-chips { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 1px; }
  .sask-chip {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 3px 9px 3px 7px;
    font-size: 12px; line-height: 1.3;
    border-radius: 999px;
    color: var(--fg, inherit);
    border: 1px solid color-mix(in oklab, var(--ok) 40%, transparent);
    background: color-mix(in oklab, var(--ok) 11%, transparent);
  }
  .sask-chip :global(svg) { color: var(--ok); flex: none; }

  .sask-question { display: flex; flex-direction: column; gap: 6px; }
  .sask-q-header {
    align-self: flex-start;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 2px 7px;
    border-radius: 5px;
    color: var(--accent);
    background: color-mix(in oklab, var(--accent) 14%, transparent);
  }
  .sask-q-text { font-size: 13px; font-weight: 500; color: var(--fg, inherit); }
  .sask-options { display: flex; flex-direction: column; gap: 5px; }
  .sask-option {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 8px 10px;
    border: 1px solid var(--border, color-mix(in oklab, var(--fg) 12%, transparent));
    border-radius: 8px;
    background: transparent;
    cursor: pointer;
    transition: border-color 120ms, background 120ms;
  }
  .sask-option:hover:not(:disabled) {
    border-color: color-mix(in oklab, var(--accent) 45%, transparent);
    background: color-mix(in oklab, var(--accent) 5%, transparent);
  }
  .sask-option.selected {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 12%, transparent);
  }
  .sask-option:disabled { opacity: 0.55; cursor: default; }
  .sask-marker { display: inline-flex; margin-top: 1px; color: var(--accent); }
  .sask-opt-text { display: flex; flex-direction: column; gap: 2px; }
  .sask-opt-label { font-size: 12.5px; color: var(--fg, inherit); }
  .sask-opt-desc { font-size: 11px; color: var(--fg-2, color-mix(in oklab, var(--fg) 60%, transparent)); }
  .sask-other-input {
    width: 100%;
    padding: 7px 10px;
    font-size: 12.5px;
    border: 1px solid var(--accent);
    border-radius: 8px;
    background: var(--bg-0, transparent);
    color: var(--fg, inherit);
  }
  .sask-actions { display: flex; gap: 8px; justify-content: flex-end; }
  .sask-btn {
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    border-radius: 8px;
    border: 1px solid transparent;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .sask-btn.cancel {
    border-color: var(--border, color-mix(in oklab, var(--fg) 14%, transparent));
    background: transparent;
    color: var(--fg-2, inherit);
  }
  .sask-btn.submit { background: var(--accent); color: var(--accent-fg); }
  .sask-btn:disabled { opacity: 0.5; cursor: default; }
  .sask-hint { font-size: 11px; color: var(--fg-2, color-mix(in oklab, var(--fg) 55%, transparent)); }
  .sask-empty { font-size: 12px; color: var(--fg-2, inherit); font-style: italic; }
</style>
