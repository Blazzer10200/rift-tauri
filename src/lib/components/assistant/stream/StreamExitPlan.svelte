<script lang="ts">
  // ExitPlanMode rich block — plan mode's whole deliverable is the markdown in
  // `input.plan`, so it renders as a readable proposal card instead of the old
  // 48-char generic peek. Pending (the approval moment) = always open; settled
  // history = collapsed to a one-line summary with a toggle.
  //
  // The card is ALSO the approval gate (docs/design/plan-mode.md §1b): while
  // the backend's `can_use_tool` ask is parked (`kind:"plan"` in
  // permissionPrompts) the action bar answers it live — Approve & build flips
  // the mode and the SAME turn rolls into execution. If the ask never
  // round-tripped (old CLI, race, timeout), a settled last-turn card keeps
  // Approve/Refine as send-shaped fallbacks. One component, two transports.
  import { ScrollText, ChevronDown, Check, X, MessageSquareText, Loader2, Pencil } from "lucide-svelte";
  import Markdown from "../Markdown.svelte";
  import type { StreamTool } from "./streamModel";
  import { assistant, type TabState } from "$lib/state/assistant.svelte";
  import type { PlanAction } from "$lib/state/assistant/helpers";

  let { tool, tab = null, isLast = false }:
    { tool: StreamTool; tab?: TabState | null; isLast?: boolean } = $props();
  const plan = $derived(typeof tool.input?.plan === "string" ? (tool.input.plan as string) : "");
  const pending = $derived(tool.status === "pending");
  const prompt = $derived(assistant.permissionPromptFor(tool.id));
  // Live transport: the ask is parked on this block awaiting our answer.
  const live = $derived(pending && !!prompt);
  // Typewriter reveal — the CLI usually delivers the plan input in one burst
  // (measured ~67ms of forming), so the card paces the draft itself: `reveal`
  // chases plan.length at reading speed, and a genuinely token-paced stream
  // animates identically because the plan keeps growing ahead of the cursor.
  // History cards (settled on load) skip straight to the full text.
  const REVEAL_STEP = 20, REVEAL_TICK_MS = 30;
  let reveal = $state(
    tool.status === "pending" && !window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 0 : -1,
  );
  const revealing = $derived(reveal >= 0 && reveal < plan.length);
  $effect(() => {
    if (!revealing) return;
    const t = setInterval(() => (reveal = Math.min(plan.length, reveal + REVEAL_STEP)), REVEAL_TICK_MS);
    return () => clearInterval(t);
  });
  const shownMd = $derived(revealing ? plan.slice(0, reveal) : plan);
  // Still being written — forming upstream, or the reveal catching up.
  const drafting = $derived(pending && (!!tool.forming || revealing));
  // Send transport: settled card on the last turn whose plan was never
  // approved — Approve flips the mode + auto-sends the execute prompt.
  const settledActionable = $derived(
    !pending && !prompt && !!plan && isLast && !!tab && !tab.streaming
      && (!tab.plan || tab.plan.status === "proposed"),
  );
  const showActions = $derived(live || settledActionable);

  let userToggled = $state(false);
  let openPref = $state(false);
  // Open while the approval is live unless the user explicitly collapsed it.
  const open = $derived(userToggled ? openPref : (pending || showActions));
  function toggle() {
    openPref = !open;
    userToggled = true;
  }

  let refining = $state(false);
  let feedback = $state("");
  let submitting = $state(false);
  let editing = $state(false);
  let draft = $state("");
  // Approving an edited draft ships it via updatedInput.plan (live) or as the
  // execute prompt (fallback) — the model builds what the user actually wants.
  const edited = $derived(editing && draft.trim() !== "" && draft !== plan ? draft : undefined);

  function toggleEdit() {
    if (!editing) draft = plan;
    editing = !editing;
  }

  async function act(action: PlanAction, fb?: string) {
    if (submitting) return;
    submitting = true;
    try {
      if (live) {
        await assistant.answerPlan(tool.id, action, { feedback: fb, planMd: plan, editedPlan: edited });
      } else if (tab) {
        if (action === "build") assistant.approvePlanFallback(tab, edited ?? plan);
      }
      refining = false;
      feedback = "";
      editing = false;
    } catch (e) {
      console.warn("answerPlan failed", e);
    } finally {
      submitting = false;
    }
  }

  function onRefineClick() {
    if (live) {
      refining = !refining;
    } else if (tab) {
      assistant.refinePlanPrefill(tab);
    }
  }

  function onRefineKey(e: KeyboardEvent) {
    if (e.key === "Enter" && feedback.trim()) {
      e.preventDefault();
      void act("refine", feedback);
    } else if (e.key === "Escape") {
      refining = false;
      feedback = "";
    }
  }
</script>

<div class="sxplan" class:pending>
  <button class="sxplan-head" type="button" onclick={toggle} aria-expanded={open}>
    <ScrollText size={14} strokeWidth={2} class={drafting ? "sxplan-write" : ""} />
    <span class="sxplan-title">{drafting ? "Drafting plan…" : live ? "Plan proposed — approve to build" : pending ? "Plan proposed — review to continue" : "Proposed plan"}</span>
    <span class="sxplan-chev" class:open><ChevronDown size={13} strokeWidth={2.25} /></span>
  </button>
  {#if !plan}
    <div class="sxplan-empty">Drafting plan…</div>
  {:else if open}
    <div class="sxplan-body">
      {#if editing && showActions}
        <textarea class="sxplan-edit" bind:value={draft} rows={Math.min(18, Math.max(6, draft.split("\n").length + 1))} spellcheck="false"></textarea>
      {:else}
        <Markdown text={shownMd} />
        {#if drafting}<span class="sxplan-caret" aria-hidden="true"></span>{/if}
      {/if}
    </div>
  {/if}
  {#if showActions && plan && !revealing}
    <div class="sxplan-actions" role="group" aria-label="Plan approval">
      {#if refining}
        <!-- Focus moves into the input as the direct result of clicking Refine —
             the expected next step, not a focus steal. -->
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="sxplan-refine"
          type="text"
          placeholder="What should change?"
          bind:value={feedback}
          onkeydown={onRefineKey}
          autofocus
        />
        <button type="button" class="sxa-btn" disabled={submitting || !feedback.trim()} onclick={() => act("refine", feedback)}>
          {#if submitting}<Loader2 size={12} class="sxa-spin" />{:else}<Check size={12} />{/if} Send
        </button>
        <button type="button" class="sxa-btn quiet" disabled={submitting} onclick={() => { refining = false; feedback = ""; }}>
          Cancel
        </button>
      {:else}
        <button type="button" class="sxa-btn primary" disabled={submitting} onclick={() => act("build")}>
          {#if submitting}<Loader2 size={12} class="sxa-spin" />{:else}<Check size={12} />{/if} Approve &amp; build{edited ? " edits" : ""}
        </button>
        {#if live}
          <button type="button" class="sxa-btn" disabled={submitting} onclick={() => act("ask")}>
            Approve, ask before edits
          </button>
        {/if}
        <button type="button" class="sxa-btn quiet" class:on={editing} disabled={submitting} onclick={toggleEdit}>
          <Pencil size={12} /> {editing ? "Preview" : "Edit"}
        </button>
        <button type="button" class="sxa-btn quiet" disabled={submitting} onclick={onRefineClick}>
          <MessageSquareText size={12} /> Refine
        </button>
        {#if live}
          <button type="button" class="sxa-btn discard" disabled={submitting} onclick={() => act("discard")}>
            <X size={12} /> Discard
          </button>
        {/if}
      {/if}
    </div>
  {/if}
</div>
