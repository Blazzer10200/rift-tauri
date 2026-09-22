<script lang="ts">
  // Interactive ask_user card for STREAM mode. The legacy ToolChip path had this
  // card; StreamTurn never did, so mcp__rift__ask_user silently fell through to a
  // dead WorkLine in stream mode — the question never rendered. This restores the
  // full interactive surface, reusing the same store binding/submit API.
  import { untrack } from "svelte";
  import { CheckCircle2, Circle, Square, Loader2, MessageCircleQuestion, Check } from "@lucide/svelte";
  import { assistant } from "$lib/state/assistant.svelte";
  import { parseAskQuestions } from "$lib/state/assistant/askQuestions";
  import { parseAskUserResult, type StreamTool } from "./streamModel";

  let { tool, live = false }: { tool: StreamTool; live?: boolean } = $props();

  // Lenient shared parser — coerces sloppy model shapes (string options,
  // JSON-string questions) instead of rendering an empty card.
  const askQuestions = $derived(parseAskQuestions(tool.input));

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
  // Terminal-but-unanswered: the turn ended (timeout/stop/error/app restart)
  // before the user clicked. Without this state the card kept rendering its
  // interactive shell — orphaned Submit/Dismiss buttons over a question that
  // could no longer be answered (and, when the turn died mid-input, over NO
  // question at all). Render inert instead: question kept readable, no chrome
  // that promises an action the backend can't honor anymore.
  const askExpired = $derived(!askAnswered && (!live || tool.status === "error"));
  // Live turn, but the tool input is still streaming in (questions [] until the
  // input JSON finishes forming) — hold the buttons until there's a question.
  const askForming = $derived(!askAnswered && !askExpired && askQuestions.length === 0);
  // Still genuinely awaiting the user — the moment the tile's is-ask (accent,
  // stationary status edge) treatment applies. Settled (answered/expired)
  // drops back to a plain tile so a resolved question stops reading as live.
  const awaiting = $derived(!askAnswered && !askExpired);

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

<div class="tile sask" class:is-ask={awaiting} class:answered={askAnswered} class:expired={askExpired} class:dismissed={askDismissed}>
  <div class="tile-head sask-head">
    <span class="sask-head-ic" aria-hidden="true">
      {#if askAnswered && !askDismissed}<Check size={13} strokeWidth={2.5} />
      {:else if !askAnswered && !askExpired}<span class="sask-dot"></span>
      {:else}<MessageCircleQuestion size={13} />{/if}
    </span>
    <span class="tile-label sask-head-label">{askAnswered ? (askDismissed ? "Dismissed" : "Your answer") : askExpired ? "Question expired" : "Rift needs your input"}</span>
  </div>

  <div class="sask-body tile-body">
    {#if askExpired}
      <!-- Inert post-mortem: keep the question readable, drop every affordance. -->
      {#each askQuestions as q, qi (qi)}
        <div class="sask-question">
          {#if q.header}<span class="tile-pill is-live">{q.header}</span>{/if}
          <div class="sask-q-text">{q.question}</div>
        </div>
      {/each}
      <div class="sask-hint">The turn ended before this was answered — reply in chat to continue.</div>
    {:else if askAnswered}
      {#if askDismissed}
        <div class="sask-empty">Dismissed — no answer given.</div>
      {:else if answeredPairs.length > 0}
        {#each answeredPairs as p (p.question)}
          <div class="sask-answered">
            {#if p.header}<span class="tile-pill is-live">{p.header}</span>{/if}
            <div class="sask-q-text">{p.question}</div>
            <div class="sask-chips">
              {#each p.answers as a, ai (ai)}
                <span class="tile-pill is-ok"><Check size={11} strokeWidth={2.5} />{a}</span>
              {/each}
            </div>
          </div>
        {/each}
      {:else}
        <div class="sask-empty">(no answer recorded)</div>
      {/if}
    {:else if askForming}
      <div class="sask-empty">Preparing the question…</div>
    {:else}
      {#each askQuestions as q, qi (qi)}
        <div class="sask-question">
          {#if q.header}<span class="tile-pill is-live">{q.header}</span>{/if}
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
      <!-- In-card action bar (DESIGN §7): one solid primary, quiet secondary —
           reuses the same perm-btn-family idiom as PermissionBar/StreamExitPlan. -->
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
      {:else if !askRequestId}
        <div class="sask-hint">Connecting to the chat session…</div>
      {:else if askQuestions.length === 1}
        <div class="sask-hint">Or just type in the composer below — your message becomes the answer.</div>
      {/if}
    {/if}
  </div>
</div>

<style>
  /* The one card the turn is blocked on — global `tile` shell (border/radius/
     fill/hover from app.css "stream tiles"). `.is-ask` (awaiting) adds the
     accent stationary status edge; settled (answered/expired) drops it and
     reads as a plain quiet tile — a resolved question must not keep competing
     with the live conversation below it (DESIGN §8: stationary state only). */
  .sask {
    margin: 10px 0;
    animation: blockIn var(--dur-base) var(--ease-page) both;
  }

  /* card header — quiet eyebrow (tile-label recipe) + breathing dot while
     awaiting; settles to the tile-head default (fg-2) once resolved. */
  .sask-head-label { color: var(--accent); }
  .sask.answered .sask-head-label,
  .sask.expired .sask-head-label { color: var(--fg-2); }
  .sask-head-ic {
    display: grid; place-items: center; flex: none;
    color: var(--accent);
  }
  .sask.answered .sask-head-ic { color: var(--ok); }
  .sask.expired .sask-head-ic,
  .sask.dismissed .sask-head-ic { color: var(--fg-muted); }
  .sask-dot {
    width: 6px; height: 6px; border-radius: 50%; display: block;
    background: var(--accent);
    animation: streamLivePulse var(--pulse-live) ease-out infinite;
  }

  .sask-body {
    padding: 11px 14px 13px;
    display: flex; flex-direction: column;
    gap: 11px;
  }

  /* answered summary — chips, not a monospace dump */
  .sask-answered { display: flex; flex-direction: column; gap: 6px; }
  .sask-chips { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 1px; }
  .sask-chips :global(svg) { color: var(--ok); flex: none; }

  .sask-question { display: flex; flex-direction: column; gap: 6px; }
  .sask-q-text { font-size: var(--fs-md); font-weight: 500; color: var(--fg, inherit); }
  .sask-options { display: flex; flex-direction: column; gap: 6px; }
  /* Options are tint tiles inside the island (one level deep, per DESIGN §8) —
     quiet at rest, accent only on the picked one. */
  .sask-option {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 8px 11px;
    border: 1px solid color-mix(in oklab, var(--border) 70%, transparent);
    border-radius: var(--radius);
    background: color-mix(in oklab, var(--fg) 2.5%, transparent);
    cursor: pointer;
    transition: border-color var(--dur-fast), background var(--dur-fast);
  }
  .sask-option:hover:not(:disabled) {
    border-color: var(--border-strong);
    background: color-mix(in oklab, var(--fg) 4.5%, transparent);
  }
  .sask-option.selected {
    border-color: color-mix(in oklab, var(--accent) 55%, var(--border));
    background: color-mix(in oklab, var(--accent) 9%, transparent);
  }
  .sask-option:disabled { opacity: 0.55; cursor: default; }
  .sask-marker { display: inline-flex; margin-top: 1px; color: var(--fg-faint); transition: color var(--dur-fast); }
  .sask-option:hover:not(:disabled) .sask-marker { color: var(--fg-subtle); }
  .sask-option.selected .sask-marker { color: var(--accent); animation: dotPop 420ms var(--ease-spring, var(--ease-page)) both; }
  .sask-opt-text { display: flex; flex-direction: column; gap: 2px; }
  .sask-opt-label { font-size: var(--fs-sm); color: var(--fg, inherit); }
  .sask-opt-desc { font-size: var(--fs-xs); color: var(--fg-2, color-mix(in oklab, var(--fg) 60%, transparent)); }
  .sask-other-input {
    width: 100%;
    padding: 7px 10px;
    font-size: var(--fs-sm);
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    background: var(--bg-0, transparent);
    color: var(--fg, inherit);
  }
  .sask-actions { display: flex; gap: 8px; justify-content: flex-end; }
  .sask-btn {
    padding: 6px 14px;
    font-size: var(--fs-sm);
    font-weight: 500;
    border-radius: var(--radius);
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
  .sask-btn.submit :global(.chip-spin) { animation: sask-spin 1s linear infinite; }
  @keyframes sask-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .sask-btn.submit :global(.chip-spin) { animation: none; } }
  .sask-hint { font-size: var(--fs-xs); color: var(--fg-2, color-mix(in oklab, var(--fg) 55%, transparent)); }
  .sask-empty { font-size: var(--fs-sm); color: var(--fg-2, inherit); font-style: italic; }
</style>
