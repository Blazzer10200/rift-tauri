<script lang="ts">
  import { Sparkles, Copy, Check, Brain, ChevronDown, ChevronRight, AlertCircle, Navigation, X, Ban } from "lucide-svelte";
  import { onDestroy } from "svelte";
  import { fade, slide } from "svelte/transition";
  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;
  import { assistant, type Block, type ChatMessage } from "../../state/assistant.svelte";
  import { GitCommit, Clock, RotateCcw } from "lucide-svelte";
  import Markdown from "./Markdown.svelte";
  import EditDiff from "./EditDiff.svelte";
  import ToolChip from "./ToolChip.svelte";
  import PermissionBar from "./PermissionBar.svelte";
  import { isInlineDiffTool, shortToolName, parseTextBlock, reconcileSplitHeaders, mergeSplitProse, statusOf, nodeKind,
    formatBoundaryAt, formatDuration, formatDurationMs, groupDurationMs, elapsedFor, summarizeGroup, shortModel, lineDelta,
    coalesceToolGroups, numberActions, type TimelineUnit } from "./bubble/helpers";

  import { tooltip } from "$lib/actions/tooltip";
  import { portal } from "$lib/actions/portal";
  import { contextMenu, copyText, type CtxMenuItem } from "../../state/contextMenu.svelte";

  // In-app image lightbox. window.open("data:…") is a no-op in WebView2 (no
  // browser tabs; CSP blocks data: navigation), so a thumbnail click opens a
  // portal'd full-screen overlay instead — click-anywhere / Esc to close.
  let lightboxSrc = $state<string | null>(null);
  $effect(() => {
    if (!lightboxSrc) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === "Escape") lightboxSrc = null; };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  let { message, streaming = false, isLast = false }: { message: ChatMessage; streaming?: boolean; isLast?: boolean } = $props();

  function onBubbleContext(e: MouseEvent) {
    if (e.defaultPrevented) return;
    const target = e.target as Element | null;
    // Fields, links, and code blocks get richer items from the global fallback.
    if (target?.closest("a[href], input, textarea, pre")) return;
    const sel = window.getSelection();
    const selText = sel && !sel.isCollapsed ? sel.toString() : "";
    const items: CtxMenuItem[] = [];
    if (selText) items.push({ label: "Copy", icon: Copy, action: () => copyText(selText) });
    if (plainText) items.push({ label: "Copy message", icon: Copy, action: () => copy() });
    if (!items.length) return;
    e.preventDefault();
    contextMenu.open(e.clientX, e.clientY, items);
  }

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
  let expandedThinking = $state(new Set<string>());
  // Live tick so `active` thinking blocks + the role-row heartbeat show
  // real-time elapsed seconds. Runs while either an active thinking block
  // is present OR this bubble is the in-flight streaming message.
  let tickNow = $state(Date.now());
  const hasActiveThinking = $derived(
    streaming && message.blocks.some((b) => b.type === "thinking" && b.status === "active"),
  );
  let tickHandle: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    // Unconditional clear-at-entry — reactive batch can run this effect twice
    // before the !tickHandle guard catches up, registering two intervals. (#161)
    if (tickHandle) { clearInterval(tickHandle); tickHandle = null; }
    if (hasActiveThinking || streaming) {
      tickHandle = setInterval(() => (tickNow = Date.now()), 500);
    }
  });
  onDestroy(() => {
    if (tickHandle) clearInterval(tickHandle);
    if (copyTimer) clearTimeout(copyTimer);
  });

  // Role-row heartbeat — elapsed since turn start. Re-reads tickNow so it
  // updates every 500ms while streaming.
  const heartbeatLabel = $derived.by<string | null>(() => {
    if (!streaming) return null;
    const start = assistant.activity.turnStartedAt;
    if (!start) return null;
    void tickNow;
    const s = Math.floor((Date.now() - start) / 1000);
    return s < 60 ? `${s}s` : `${Math.floor(s / 60)}m ${s % 60}s`;
  });

  // Live activity label + token tally now live in the composer's LivePills
  // (single source of in-flight truth, down by the prompt) — the in-bubble
  // strips were de-duplicated to a quiet dots pulse (see .stage-strip below).

  const plainText = $derived(
    message.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim(),
  );

  const isUser = $derived(message.role === "user");
  const isSystem = $derived(message.role === "system");
  // Compaction boundary: the system-role message owns a single
  // BoundaryBlock. Extract it here so the boundary render path stays
  // out of the normal block-snippet rendering loop below.
  const boundaryBlock = $derived(
    isSystem
      ? (message.blocks.find((b) => b.type === "boundary") as
          | Extract<Block, { type: "boundary" }>
          | undefined)
      : undefined,
  );
  let boundaryExpanded = $state(false);

  // Terminal stop reason, surfaced only once the turn settles. max_tokens =
  // output truncated at the cap; refusal = model declined. Normal completions
  // carry no stopReason, so the notice stays hidden.
  const stopNotice = $derived(!isUser && !streaming ? message.stopReason ?? null : null);

  function toggleThinking(i: string) {
    const next = new Set(expandedThinking);
    if (next.has(i)) next.delete(i);
    else next.add(i);
    expandedThinking = next;
  }

  // Collapsed tool-group expand state, keyed by group key.
  let expandedGroups = $state(new Set<string>());
  function toggleGroup(key: string) {
    const next = new Set(expandedGroups);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    expandedGroups = next;
  }

  async function copy() {
    if (!plainText) return;
    try {
      await navigator.clipboard.writeText(plainText);
      copied = true;
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => { copied = false; copyTimer = null; }, 1200);
    } catch (e) {
      console.warn("copy failed", e);
    }
  }

  const modelLabel = $derived(message.model ? shortModel(message.model) : null);
  // Local-LLM turns run a non-Anthropic model (id never starts with "claude").
  // Label them "Local model" so the header identity matches what the model
  // actually is — derived per-message so history stays correct when the live
  // toggle later flips.
  const isLocalModel = $derived(!!message.model && !/^claude/i.test(message.model));
  // Family key for aurora tinting — drives the bubble's left rail + avatar
  // halo color so each assistant turn carries the same hue as the composer
  // that produced it.
  const modelFamily = $derived.by<"sonnet" | "opus" | "haiku" | null>(() => {
    if (!message.model) return null;
    const m = /claude-(opus|sonnet|haiku)/i.exec(message.model);
    return m ? (m[1].toLowerCase() as "sonnet" | "opus" | "haiku") : null;
  });
  const costLabel = $derived(
    typeof message.costUsd === "number"
      ? (message.costUsd > 0 && message.costUsd < 0.01 ? "<$0.01" : `$${message.costUsd.toFixed(2)}`)
      : null,
  );

  // ── TurnSummary — caps a completed assistant turn that touched files.
  // Stats derive from the turn's own Edit/Write/MultiEdit blocks (same line
  // diff as EditDiff), duration from summed block work-time, cost from the
  // message. Edits are ALREADY applied (per-tool, via PermissionBar) — so this
  // is an honest recap + mode consequence, not a fake turn-level Apply/Undo.
  const turnStats = $derived.by(() => {
    if (isUser) return { files: 0, adds: 0, dels: 0, firstEditId: null as string | null, firstEditFile: null as string | null };
    const files = new Set<string>();
    let adds = 0, dels = 0, firstEditId: string | null = null, firstEditFile: string | null = null;
    for (const b of message.blocks) {
      if (b.type !== "tool") continue;
      const name = b.name.replace(/^mcp__rift__/, "");
      const inp = (b.input ?? {}) as Record<string, unknown>;
      const fp = typeof inp.file_path === "string" ? inp.file_path
        : typeof inp.notebook_path === "string" ? inp.notebook_path : null;
      if (name === "Edit" || name === "NotebookEdit") {
        const d = lineDelta(inp.old_string, inp.new_string); adds += d.adds; dels += d.dels;
        if (fp) { files.add(fp); firstEditId ??= b.id; firstEditFile ??= fp; }
      } else if (name === "MultiEdit" && Array.isArray(inp.edits)) {
        for (const e of inp.edits as Array<Record<string, unknown>>) {
          const d = lineDelta(e?.old_string, e?.new_string); adds += d.adds; dels += d.dels;
        }
        if (fp) { files.add(fp); firstEditId ??= b.id; firstEditFile ??= fp; }
      } else if (name === "Write") {
        const c = inp.content; adds += typeof c === "string" && c.length > 0 ? c.split("\n").length : 0;
        if (fp) { files.add(fp); firstEditId ??= b.id; firstEditFile ??= fp; }
      }
    }
    return { files: files.size, adds, dels, firstEditId, firstEditFile };
  });
  const turnDurationMs = $derived.by(() => {
    let ms = 0;
    for (const b of message.blocks) {
      if ((b.type === "thinking" || b.type === "tool") && b.durationMs != null) ms += b.durationMs;
    }
    return ms;
  });
  const autoApplied = $derived(
    assistant.permissionMode === "acceptEdits" ||
    assistant.permissionMode === "bypassPermissions" ||
    assistant.permissionMode === "auto",
  );
  const bypassApplied = $derived(assistant.permissionMode === "bypassPermissions");
  const showSummary = $derived(!isUser && !streaming && turnStats.files > 0);

  // Walk the message's blocks → flat TimelineUnit list. Step headers in
  // prose become dividers; everything else becomes a node on the chain.
  const grouped = $derived.by<TimelineUnit[]>(() => {
    const units: TimelineUnit[] = [];
    const blocks = mergeSplitProse(reconcileSplitHeaders(message.blocks));
    for (let i = 0; i < blocks.length; i++) {
      const b = blocks[i];
      // Builtin AskUserQuestion is always auto-denied + steered to ask_user (see
      // mod.rs handle_permission_request) — never render its dead chip.
      if (b.type === "tool" && shortToolName(b.name) === "AskUserQuestion") continue;
      if (b.type === "text") {
        const segments = parseTextBlock(b.text);
        for (let si = 0; si < segments.length; si++) {
          const seg = segments[si];
          if (seg.kind === "header") {
            units.push({
              kind: "divider",
              stepNum: seg.stepNum,
              title: seg.title,
              key: `d_${i}_${si}`,
            });
          } else {
            const proseBlock: Block = { type: "text", text: seg.text };
            units.push({
              kind: "block",
              block: proseBlock,
              status: "neutral",
              key: `t_${i}_${si}`,
            });
          }
        }
        continue;
      }
      units.push({ kind: "block", block: b, status: statusOf(b), key: `b_${i}` });
    }
    // Fold runs of back-to-back completed thinking units into one row —
    // summed duration, prose joined. Kills the "Thought for 1s / Thought for
    // <1s" stack between tool calls without losing the expandable text.
    const folded: TimelineUnit[] = [];
    for (const u of units) {
      const prev = folded[folded.length - 1];
      if (
        u.kind === "block" && u.block.type === "thinking" && u.block.status === "done" &&
        prev?.kind === "block" && prev.block.type === "thinking" && prev.block.status === "done"
      ) {
        const a = prev.block, b = u.block;
        prev.block = {
          ...a,
          text: [a.text, b.text].filter(Boolean).join("\n\n"),
          hasSignature: a.hasSignature || b.hasSignature,
          durationMs: (a.durationMs ?? 0) + (b.durationMs ?? 0),
        };
        continue;
      }
      folded.push(u);
    }
    return numberActions(coalesceToolGroups(folded));
  });

  // Key of the last in-flight or final node — drives the bullet pulse for
  // the streaming bubble (last node = "current activity").
  const lastBlockKey = $derived.by<string | null>(() => {
    for (let i = grouped.length - 1; i >= 0; i--) {
      const u = grouped[i];
      if (u.kind === "block" || u.kind === "toolgroup") return u.key;
    }
    return null;
  });
</script>

{#if isSystem && boundaryBlock && boundaryBlock.source === "cli"}
  {@const hasPct =
    typeof boundaryBlock.ctxPctBefore === "number" &&
    typeof boundaryBlock.ctxPctEstAfter === "number"}
  <div class="boundary boundary-cli" data-role="system">
    <span class="boundary-line" aria-hidden="true"></span>
    <span
      class="boundary-pill"
      use:tooltip={"Claude Code automatically summarized older messages to free up the context window. The conversation continues normally — nothing on screen was deleted."}
    >
      <Sparkles size={11} />
      <span>Conversation compacted{boundaryBlock.trigger === "manual" ? " · manual" : ""}</span>
      {#if hasPct}
        <span class="boundary-meta mono">
          Ctx {Math.round(boundaryBlock.ctxPctBefore ?? 0)}% → {Math.round(boundaryBlock.ctxPctEstAfter ?? 0)}%
        </span>
      {/if}
    </span>
    <span class="boundary-line" aria-hidden="true"></span>
  </div>
{:else if isSystem && boundaryBlock}
  {@const isCompacting = boundaryBlock.streaming === true}
  {@const showBody = isCompacting || boundaryExpanded}
  <div class="boundary" data-role="system" class:streaming={isCompacting}>
    <button
      type="button"
      class="boundary-head"
      onclick={() => (boundaryExpanded = !boundaryExpanded)}
      aria-expanded={boundaryExpanded}
      disabled={isCompacting}
      use:tooltip={isCompacting ? "Summarizing…" : `Click to ${boundaryExpanded ? "hide" : "show"} the compaction summary`}
    >
      <span class="boundary-line" aria-hidden="true"></span>
      <span class="boundary-pill">
        <Sparkles size={11} />
        {#if isCompacting}
          <span class="live-dot" aria-label="Compacting" use:tooltip={"Summarizing in progress"}></span>
          <span>Compacting · {boundaryBlock.archivedCount} message{boundaryBlock.archivedCount === 1 ? "" : "s"} · {boundaryBlock.summary.length.toLocaleString()} chars</span>
        {:else}
          <span>Conversation compacted · {boundaryBlock.archivedCount} message{boundaryBlock.archivedCount === 1 ? "" : "s"} archived</span>
          {#if typeof boundaryBlock.ctxPctBefore === "number" && typeof boundaryBlock.ctxPctEstAfter === "number"}
            <span class="boundary-meta mono" use:tooltip={"Context window utilization — pre-compact → estimated post-compact"}>
              Ctx {Math.round(boundaryBlock.ctxPctBefore)}% → est {Math.round(boundaryBlock.ctxPctEstAfter)}%
            </span>
          {/if}
          <span class="boundary-meta mono">
            ${boundaryBlock.costUsd.toFixed(4)} · {boundaryBlock.summaryModel} · {formatBoundaryAt(boundaryBlock.at)}
          </span>
          <ChevronDown size={11} class="chev" />
        {/if}
      </span>
      <span class="boundary-line" aria-hidden="true"></span>
    </button>
    {#if showBody && boundaryBlock.summary.length > 0}
      <div class="boundary-body"><Markdown text={boundaryBlock.summary} /></div>
    {/if}
  </div>
{:else}
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="bubble" data-role={message.role} data-model={modelFamily} data-streaming={streaming ? "true" : null} oncontextmenu={onBubbleContext}>
  {#if !isUser}
    <div class="turn-rail" aria-hidden="true"></div>
    <div class="avatar" aria-hidden="true">
      <Sparkles size={13} />
    </div>
  {/if}

  <div class="body">
    {#if !isUser}
      <div class="turn-head">
        <span class="role-name">{isLocalModel ? (modelLabel ?? "Local model") : "Claude"}</span>
        {#if modelLabel && !isLocalModel}
          <span class="head-sep" aria-hidden="true">·</span>
          <span class="head-model" use:tooltip={"Model for this turn"}>{modelLabel}</span>
        {/if}
        {#if streaming}
          <!-- #39 P0-4: while a thinking block is on screen it carries its own
               ticking timer one line below — don't stack a second live counter
               in the head. Show the heartbeat only in non-thinking phases
               (tool/text), falling back to the dot before the first tick. -->
          {#if hasActiveThinking}
            <span class="live-dot" aria-label="Streaming" use:tooltip={"Streaming response"}></span>
          {:else if heartbeatLabel}
            <!-- Timer carries liveness (it ticks) — no separate pulsing dot
                 needed alongside it. The dot is a fallback only for the brief
                 window before turnStartedAt resolves a heartbeat number. -->
            <span class="heartbeat mono" use:tooltip={"Elapsed since turn started"}>{heartbeatLabel}</span>
          {:else}
            <span class="live-dot" aria-label="Streaming" use:tooltip={"Streaming response"}></span>
          {/if}
        {/if}
        {#if plainText.length > 0 || (!streaming && (costLabel || isLast))}
          <div class="turn-actions">
            {#if !streaming && costLabel}
              <span class="cost-pill mono" use:tooltip={"Turn cost in USD — total for this assistant turn"}>{costLabel}</span>
            {/if}
            {#if plainText.length > 0}
              <button class="copybtn" type="button" onclick={copy} use:tooltip={"Copy"} aria-label="Copy message">
                {#if copied}
                  <Check size={11} />
                {:else}
                  <Copy size={11} />
                {/if}
              </button>
            {/if}
            {#if isLast && !streaming}
              <button
                class="copybtn"
                type="button"
                onclick={() => assistant.retryLast()}
                use:tooltip={"Retry this turn — re-runs your last prompt"}
              >
                <RotateCcw size={11} />
              </button>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    {#if !isUser && streaming && grouped.length === 0}
      <div class="stage-strip" out:fade={{ duration: 220 }}>
        <span class="stage-dots" aria-hidden="true">
          <span class="stage-dot"></span>
          <span class="stage-dot"></span>
          <span class="stage-dot"></span>
        </span>
      </div>
    {/if}

    <div class="content">
      {#snippet renderBlock(b: Block, bi: string, caption: string | null = null, revealing: boolean = false)}
        {#if b.type === "image"}
          {@const safeMime = /^image\/(png|jpeg|gif|webp|svg\+xml|avif|bmp)$/.test(b.mime ?? "") ? b.mime : "image/png"}
          <button
            type="button"
            class="user-image-thumb"
            aria-label="View full size image"
            use:tooltip={"Click to view full size"}
            onclick={() => (lightboxSrc = `data:${safeMime};base64,${b.dataBase64}`)}
          >
            <img
              src={`data:${safeMime};base64,${b.dataBase64}`}
              alt=""
              loading="lazy"
            />
          </button>
        {:else if b.type === "text" && b.text.length > 0}
          {#if isUser}
            <div class="text">{b.text}</div>
          {:else}
            <div class="text"><Markdown text={b.text} streaming={revealing} /></div>
          {/if}
        {:else if b.type === "thinking"}
          {@const isActive = b.status === "active"}
          {@const hasText = b.text.length > 0}
          {@const isOpen = expandedThinking.has(bi)}
          {@const elapsed = elapsedFor(b, tickNow)}
          <div class="tn-think" class:active={isActive} class:expandable={hasText}>
            <button
              type="button"
              class="tn-think-head"
              onclick={() => hasText && toggleThinking(bi)}
              disabled={!hasText}
              aria-expanded={isOpen}
            >
              <Brain size={11} />
              <span class="tn-think-label">
                {#if isActive}Thinking{:else}Thought{/if} for <span class="tn-think-meta mono">{elapsed}</span>
              </span>
              {#if isActive}
                <span class="dots" aria-hidden="true">
                  <span class="dot"></span><span class="dot"></span><span class="dot"></span>
                </span>
              {/if}
              {#if hasText}
                <span class="chev" class:open={isOpen}><ChevronDown size={11} /></span>
              {/if}
            </button>
            {#if hasText && isOpen}
              <div class="tn-think-body" transition:slide={{ duration: reducedMotion ? 0 : 180 }}><Markdown text={b.text} /></div>
            {/if}
          </div>
        {:else if b.type === "steer"}
          <div class="steer-marker">
            <Navigation size={11} class="steer-marker-icon" />
            <span class="steer-marker-label">You steered</span>
            <span class="steer-marker-text">{b.text}</span>
          </div>
        {:else if b.type === "tool" && isInlineDiffTool(b.name)}
          {#if b.name === "MultiEdit" && Array.isArray(b.input.edits)}
            {@const inp = b.input as { file_path?: string; edits?: Array<{ file_path?: string; old_string?: string; new_string?: string }> }}
            {#each (inp.edits ?? []) as edit, ei (ei)}
              <EditDiff input={{ ...edit, file_path: inp.file_path }} />
            {/each}
          {:else}
            <EditDiff input={b.input} />
          {/if}
          <PermissionBar toolUseId={b.id} toolName={b.name} />
        {:else if b.type === "tool"}
          <ToolChip tool={b} variant={isUser ? "card" : "timeline"} {caption} />
          <PermissionBar toolUseId={b.id} toolName={b.name} />
        {/if}
      {/snippet}

      {#each grouped as unit, ui (unit.key)}
        {#if unit.kind === "divider"}
          <div class="tl-divider">
            <span class="tl-divider-label">Step {unit.stepNum} — {unit.title}</span>
          </div>
        {:else if unit.kind === "toolgroup"}
          {@const isLastNode = !isUser && unit.key === lastBlockKey}
          {@const nodeStatus =
            streaming && isLastNode && unit.status === "done" ? "pending" : unit.status}
          {@const defaultOpen = (streaming && isLastNode) || unit.status === "error"}
          {@const open = expandedGroups.has(unit.key) !== defaultOpen}
          {@const groupMs = groupDurationMs(unit.blocks)}
          <div
            class="tl-node work"
            class:live={nodeStatus === "pending"}
            data-kind="tool"
            data-status={nodeStatus}
            data-open={open ? "true" : null}
            style="--idx: {Math.min(ui, 6)}"
          >
            <button class="work-head" type="button" onclick={() => toggleGroup(unit.key)} aria-expanded={open}>
              <span class="work-chev" class:open><ChevronRight size={12} /></span>
              <span class="work-spark" aria-hidden="true"><Sparkles size={12} /></span>
              <span class="work-sum">
                <span class="work-sum-t">{unit.caption ?? `${unit.blocks.length} tools`}</span>
                <span class="work-sum-s">{summarizeGroup(unit.blocks)}</span>
              </span>
              {#if groupMs > 0}<span class="work-dur">{formatDurationMs(groupMs)}</span>{/if}
              <span class="work-count">{unit.blocks.length}</span>
            </button>
            {#if open}
              <div class="work-body" transition:slide={{ duration: reducedMotion ? 0 : 200 }}>
                <div class="work-rail">
                  {#each unit.blocks.filter((gb) => gb.type !== "thinking") as gb, gi (gb.type === "tool" ? gb.id : gi)}
                    {@render renderBlock(gb, `tg_inner_${ui}_${gi}`)}
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {:else}
          {@const isLastNode = !isUser && unit.key === lastBlockKey}
          {@const nodeStatus =
            streaming && isLastNode && unit.status === "neutral"
              ? "pending"
              : unit.status}
          {@const prev = ui > 0 ? grouped[ui - 1] : null}
          {@const groupCont =
            unit.block.type !== "text" &&
            prev?.kind === "block" &&
            prev.block.type !== "image"}
          <div
            class="tl-node"
            id={unit.block.type === "tool" ? `actnode-${unit.block.id}` : undefined}
            data-kind={nodeKind(unit.block)}
            data-status={nodeStatus}
            data-group-cont={groupCont ? "true" : null}
            data-quick={(unit.block.type === "thinking" && unit.block.status === "done" && (unit.block.text.length === 0 || (unit.block.durationMs != null && unit.block.durationMs < 3000))) ? "true" : null}
            data-numbered={unit.stepNum ? "true" : null}
            style="--idx: {Math.min(ui, 6)}"
          >
            {#if unit.stepNum}<span class="tl-stepdot mono" aria-hidden="true">{unit.stepNum}</span>{/if}
            {@render renderBlock(unit.block, unit.key, unit.caption, streaming && !isUser && unit.block.type === "text" && unit.key === lastBlockKey)}
          </div>
        {/if}
      {/each}

      {#if stopNotice === "max_tokens"}
        <div class="stop-notice" data-kind="truncated" in:fade={{ duration: reducedMotion ? 0 : 160 }}>
          <AlertCircle size={13} class="stop-notice-ic" />
          <span class="stop-notice-txt">Response cut off — it reached the output length limit.</span>
          {#if isLast}
            <button
              type="button"
              class="stop-notice-btn"
              onclick={() => assistant.send("Continue from where you left off.")}
              use:tooltip={"Ask Claude to continue the truncated response"}
            >Continue</button>
          {/if}
        </div>
      {:else if stopNotice === "refusal"}
        <div class="stop-notice" data-kind="refusal" in:fade={{ duration: reducedMotion ? 0 : 160 }}>
          <Ban size={13} class="stop-notice-ic" />
          <span class="stop-notice-txt">The model declined to complete this response.</span>
        </div>
      {/if}

      {#if showSummary}
        <div class="turn-summary" data-auto={autoApplied ? "true" : null} class:mode-bypass={bypassApplied} in:fade={{ duration: reducedMotion ? 0 : 160 }}>
          <div class="ts-stats">
            {#if autoApplied}
              <span class="ts-applied" class:danger={bypassApplied}><Check size={13} />Applied automatically</span>
            {:else}
              <span class="ts-item"><GitCommit size={13} />{turnStats.files} file{turnStats.files === 1 ? "" : "s"} changed</span>
            {/if}
            <span class="ts-stat mono"><span class="ts-add">+{turnStats.adds}</span>{#if turnStats.dels > 0}<span class="ts-del">−{turnStats.dels}</span>{/if}</span>
            {#if turnDurationMs > 0}
              <span class="ts-dot" aria-hidden="true"></span>
              <span class="ts-item mono"><Clock size={12} />{formatDuration(turnDurationMs)}</span>
            {/if}
            {#if costLabel}<span class="ts-item mono">{costLabel}</span>{/if}
          </div>
          <div class="ts-actions">
            {#if autoApplied}
              <span class="ts-mode" class:mode-bypass={bypassApplied} use:tooltip={bypassApplied ? "All tools ran without prompting (bypass permissions)" : "Edits were applied without prompting (permission mode)"}><RotateCcw size={12} />{bypassApplied ? "bypass" : "auto"}</span>
            {/if}
          </div>
        </div>
      {/if}

    </div>
  </div>
</div>
{/if}

{#if lightboxSrc}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="lightbox"
    use:portal
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label="Full size image — click anywhere or press Escape to close"
    transition:fade={{ duration: reducedMotion ? 0 : 140 }}
    onclick={() => (lightboxSrc = null)}
  >
    <img class="lightbox-img" src={lightboxSrc} alt="" />
    <button class="lightbox-close" type="button" aria-label="Close" onclick={() => (lightboxSrc = null)}>
      <X size={18} />
    </button>
  </div>
{/if}

<style>
  .boundary {
    width: 100%;
    padding: 8px 4px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .boundary-cli {
    flex-direction: row;
    align-items: center;
    gap: 10px;
  }
  .stop-notice {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    padding: 7px 11px;
    border-radius: 10px;
    font-size: 12px;
    border: 1px solid var(--border);
    background: color-mix(in oklch, var(--bg-elev-1) 60%, transparent);
  }
  .stop-notice[data-kind="truncated"] {
    border-color: color-mix(in oklab, var(--warn) 40%, var(--border));
    background: var(--warn-soft);
  }
  .stop-notice :global(.stop-notice-ic) { flex-shrink: 0; }
  .stop-notice[data-kind="truncated"] :global(.stop-notice-ic) { color: var(--warn); }
  .stop-notice[data-kind="refusal"] :global(.stop-notice-ic) { color: var(--fg-faint); }
  .stop-notice-txt { color: var(--fg-muted); flex: 1; }
  .stop-notice-btn {
    flex-shrink: 0;
    font-size: 11.5px;
    font-weight: 600;
    padding: 3px 10px;
    border-radius: 7px;
    border: 1px solid color-mix(in oklab, var(--accent) 30%, var(--border));
    background: color-mix(in oklab, var(--accent) 12%, transparent);
    color: var(--accent);
    cursor: pointer;
    transition: background 140ms ease, border-color 140ms ease;
  }
  .stop-notice-btn:hover {
    background: color-mix(in oklab, var(--accent) 20%, transparent);
    border-color: color-mix(in oklab, var(--accent) 50%, var(--border));
  }
  .boundary-head {
    display: flex;
    align-items: center;
    gap: 10px;
    background: none;
    border: 0;
    padding: 0;
    color: inherit;
    cursor: pointer;
    width: 100%;
  }
  .boundary-line {
    flex: 1;
    height: 1px;
    background: linear-gradient(to right,
      transparent,
      color-mix(in oklch, var(--border) 80%, transparent),
      color-mix(in oklab, var(--accent) 30%, transparent) 50%,
      color-mix(in oklch, var(--border) 80%, transparent),
      transparent);
  }
  .boundary-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--bg-elev-1) 86%, transparent);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid color-mix(in oklab, var(--accent) 25%, var(--border));
    box-shadow: 0 4px 14px -4px color-mix(in oklab, var(--accent) 25%, transparent);
    font-size: 11px;
    color: var(--fg-muted);
    white-space: nowrap;
    transition: background 140ms ease, border-color 140ms ease, transform 140ms ease;
  }
  .boundary-head:not(:disabled):hover .boundary-pill {
    background: color-mix(in oklch, var(--bg-elev-1) 95%, transparent);
    border-color: color-mix(in oklab, var(--accent) 45%, var(--border));
    transform: translateY(-1px);
  }
  .boundary-pill :global(svg) { color: var(--accent); }
  .boundary-pill :global(.chev) {
    transition: transform 120ms ease;
    opacity: 0.6;
  }
  .boundary-head[aria-expanded="true"] :global(.chev) {
    transform: rotate(180deg);
  }
  .boundary-meta {
    opacity: 0.55;
    font-size: 10px;
  }
  .boundary-body {
    margin: 4px 24px 0;
    padding: 10px 14px;
    background: color-mix(in oklch, var(--bg-elev-1) 60%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 70%, transparent);
    border-radius: 10px;
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--fg-2);
  }
  .bubble {
    display: grid;
    grid-template-columns: 26px 1fr;
    column-gap: 12px;
    padding: 2px 0;
    position: relative;
    animation: enter 240ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  /* Thread is emerald-only — rail/avatar/hover-glow/halo all use --accent.
     Model identity lives on the Composer model-card, not the thread. */
  /* User turns: right-aligned tinted bubble, no rail/avatar/header — position
     + tint carry the role, so turn boundaries scan at a glance. Copy lives in
     the right-click menu. */
  .bubble[data-role="user"] .body {
    align-items: flex-end;
    display: flex; flex-direction: column;
  }
  .bubble[data-role="user"] .content {
    align-items: flex-end;
  }
  /* User nodes flex so the fit-content text bubble can actually right-align —
     a long message clamps the node to full width, and align-self on .text is
     inert under a block parent. */
  .bubble[data-role="user"] .tl-node {
    display: flex; flex-direction: column; align-items: flex-end;
    width: 100%;
  }

  /* Continuous accent rail down the LEFT of every assistant turn — visually
     groups the avatar header + content + cost as one unit without boxing
     them in (per the 2026 left-rail-spans-turn convention). */
  .turn-rail {
    grid-column: 1;
    grid-row: 1;
    align-self: stretch;
    justify-self: center;
    width: 2px;
    margin-top: 18px;
    border-radius: 2px;
    /* Quiet hairline spine that fades at both ends so each turn's segment reads
       as a soft thread, not a hard-cut colored bar. Color lives in the node
       markers now; the rail just connects them. Faint model tint keeps the
       aurora identity without shouting. */
    background: linear-gradient(to bottom,
      transparent 0,
      color-mix(in oklab, var(--accent) 20%, var(--border)) 10px,
      color-mix(in oklab, var(--accent) 20%, var(--border)) calc(100% - 10px),
      transparent 100%);
    transition: opacity 200ms ease-out, box-shadow 200ms ease-out;
  }
  /* Hovering a turn lights its spine — a faint model-hue glow ties the pointer
     to the thread it's reading. */
  .bubble[data-role="assistant"]:hover .turn-rail {
    box-shadow: 0 0 6px 0 color-mix(in oklab, var(--accent) 22%, transparent);
  }
  .bubble[data-streaming="true"] .turn-rail {
    background: color-mix(in oklab, var(--accent) 55%, transparent);
    animation: rail-stream 2.4s ease-in-out infinite;
  }
  @keyframes rail-stream {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 0%, transparent); }
    50%      { box-shadow: 0 0 6px 0 color-mix(in oklab, var(--accent) 32%, transparent); }
  }
  @media (prefers-reduced-motion: reduce) {
    .bubble[data-streaming="true"] .turn-rail { animation: none; }
  }

  /* Avatar lives INSIDE the turn-head row now (not a separate grid column).
     Reads as the "head node" sitting on the rail. */
  .avatar {
    grid-column: 1;
    grid-row: 1;
    align-self: start;
    justify-self: center;
    z-index: 1;
    width: 22px; height: 22px;
    border-radius: 50%;
    display: flex; align-items: center; justify-content: center;
    background: color-mix(in oklab, var(--accent) 14%, var(--bg));
    color: var(--accent);
    flex-shrink: 0;
    /* bg ring masks the rail so the avatar reads as the head node on the wire. */
    box-shadow: 0 0 0 3px var(--bg);
    transition: box-shadow 240ms ease-out, background 240ms ease-out, color 240ms ease-out;
  }
  .bubble[data-streaming="true"] .avatar {
    animation: avatar-halo 1.8s ease-in-out infinite;
  }
  @keyframes avatar-halo {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 0%, transparent); }
    50%      { box-shadow: 0 0 0 3px color-mix(in oklab, var(--accent) 28%, transparent); }
  }
  @media (prefers-reduced-motion: reduce) {
    .bubble[data-streaming="true"] .avatar { animation: none; }
  }

  .body { min-width: 0; grid-column: 2; }
  /* Body fills the centered reading column (no 78ch cap). The cap used to
     left-align the text inside a wider column, so even though the column was
     centered the *text* read as shoved-left. The column width itself
     (--chat-col-max) now sets the measure, so text, user bubbles + composer
     all share one centered band. */
  .bubble[data-role="assistant"] .body {
    width: 100%;
  }

  /* Unified turn header — [avatar] Claude · Opus 4.7 [streaming dot heartbeat] [copy]
     One row carries name + model + liveness instead of role-row + turn-badge
     fighting for space. */
  .turn-head {
    display: flex; align-items: center; gap: 8px;
    margin-bottom: 6px;
    min-height: 22px;
  }
  .role-name {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--fg-2);
  }
  /* Model + cost recede so the turn reads "Claude → content" first; the
     metadata brightens on hover when you actually want it. */
  .head-sep { color: var(--fg-faint); font-size: 11px; opacity: 0.4; transition: opacity 140ms ease-out; }
  .head-model {
    font-size: 10.5px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.01em;
    white-space: nowrap;
    opacity: 0.6;
    transition: opacity 140ms ease-out;
  }
  .bubble:hover .head-sep { opacity: 0.7; }
  .bubble:hover .head-model { opacity: 1; }

  /* Stage strip — fills the void between turn-head and first content block
     while Claude is "thinking" but hasn't emitted any blocks yet. Pre-block
     window is typically 1-15s of silence (longer w/ Opus extended thinking);
     w/o this the bubble reads as dead until the first prose token lands.
     Replaces the composer-top StatusHub strip — single source of in-flight
     truth in the bubble itself. */
  .stage-strip {
    display: flex; align-items: center;
    padding: 2px 0 6px;
    animation: enter 240ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .stage-dots {
    display: inline-flex; gap: 4px; align-items: center;
    height: 8px;
  }
  .stage-dot {
    width: 5px; height: 5px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 6px color-mix(in oklab, var(--accent) 45%, transparent);
    animation: stage-bounce 1.1s ease-in-out infinite;
  }
  .stage-dot:nth-child(2) { animation-delay: 0.18s; }
  .stage-dot:nth-child(3) { animation-delay: 0.36s; }
  @keyframes stage-bounce {
    0%, 80%, 100% { opacity: 0.35; transform: translateY(0) scale(0.85); }
    40%           { opacity: 1;    transform: translateY(-2px) scale(1.1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .stage-dot { animation: none; }
  }

  /* Cost chip — lives in the turn header's action cluster (see .turn-actions),
     a quiet residue of the turn's spend next to the copy button. */
  .cost-pill {
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--bg-elev-2) 70%, transparent);
    border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
    color: var(--fg-muted);
    font-size: 10px;
    line-height: 1.4;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.02em;
    opacity: 0.6;
    transition: opacity 140ms ease-out;
  }
  .bubble:hover .cost-pill { opacity: 1; }
  .body { display: flex; flex-direction: column; }

  /* Timeline node — every block sits on the 2px turn-rail via an absolutely
     positioned bullet. Bullet color = status; thinking = hollow. The chain
     visual is the rail (already drawn by .turn-rail in grid-column 1); we
     just hang dots off it. */
  .tl-node {
    position: relative;
    animation: enter 240ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(var(--idx, 0) * 35ms);
  }
  /* Streaming cell entrance (mock ct-enter) — each node blur-ins as it streams.
     Scoped to streaming so resting/loaded turns keep the calm plain entrance and
     captures render solid. Prose is excluded: its per-word reveal IS its entrance
     (mock skips the block blur-in on .ct-r-prose). */
  .bubble[data-streaming="true"] .tl-node:not([data-kind="text"]) {
    animation: node-stream-enter 480ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  @keyframes node-stream-enter {
    from { opacity: 0; transform: translateY(3px); filter: blur(4px); }
    to   { opacity: 1; transform: none; filter: blur(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .bubble[data-streaming="true"] .tl-node:not([data-kind="text"]) { animation: none; }
  }
  /* #2 Option A — narration-grouping spacing. A tool/edit/thinking node that
     follows a prose or tool block in the same turn sits tight (4px) so it
     visually belongs to the preceding narration. A tool/edit/thinking that
     starts a new group, or a prose node that follows a tool (= new narration),
     gets a wider gap (12px) — telegraphs "new beat" so multi-Edit turns stop
     reading as if the message ended after the first big block. */
  .bubble[data-role="assistant"] .tl-node:not(:first-child) {
    margin-top: 0.5rem;
  }
  .bubble[data-role="assistant"] .tl-node[data-group-cont="true"] {
    margin-top: 0.25rem;
  }
  .tl-node::before {
    content: "";
    position: absolute;
    /* Lane is 26px wide + 12px column-gap → .body left edge is 38px from the
       bubble; the rail centers at 13px → -25px from .body. A 9px bullet centered
       on the rail wants left = -25 - 4.5 = -29.5px. */
    left: -30px;
    top: 8px;
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--bg-elev-2);
    border: 1.5px solid color-mix(in oklch, var(--fg-faint) 60%, transparent);
    z-index: 1;
    transition: background 220ms ease-out, border-color 220ms ease-out, box-shadow 220ms ease-out;
    /* Each circle draws itself onto the rail as its node lands — staggered down
       the spine so a multi-tool turn reads as a timeline assembling in real
       time (live during streaming, and replays on load). Pending nodes override
       this with the pulse below, so the active tool throbs instead of popping. */
    transform-origin: center;
    animation: bullet-pop 340ms cubic-bezier(0.34, 1.56, 0.64, 1) both;
    animation-delay: calc(var(--idx, 0) * 35ms);
  }
  @keyframes bullet-pop {
    0%   { transform: scale(0);    opacity: 0; }
    60%  { transform: scale(1.3);  opacity: 1; }
    100% { transform: scale(1);    opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .tl-node::before { animation: none; }
  }
  .tl-node[data-kind="thinking"]::before {
    width: 8px; height: 8px;
    background: transparent;
    border-color: color-mix(in oklch, var(--fg-faint) 85%, transparent);
  }
  .tl-node[data-kind="thinking"][data-status="pending"]::before {
    background: transparent;
    border-color: color-mix(in oklab, var(--accent) 70%, transparent);
    animation: tl-bullet-pulse 1.6s ease-in-out infinite;
  }
  /* Prose paragraphs no longer hang a bullet off the rail. One faint dot per
     paragraph read as left-margin noise on text-only answers; the continuous
     rail already groups the turn. Bullets remain for tool/edit/thinking nodes
     where they carry real status (done/pending/error). */
  .tl-node[data-kind="prose"]::before { display: none; }
  /* Numbered nodes (tools / tool-groups) carry the step circle as their sole
     rail marker — drop the small status bullet so the spine threads through ONE
     clean node, not a dot stacked beside a circle. */
  .tl-node[data-numbered="true"]::before { display: none; }
  .tl-node[data-kind="tool"][data-status="done"]::before,
  .tl-node[data-kind="edit"][data-status="done"]::before {
    background: var(--ok, oklch(0.74 0.15 145));
    border-color: color-mix(in oklch, var(--ok, oklch(0.74 0.15 145)) 75%, transparent);
    box-shadow: inset 0 1px 0 color-mix(in oklch, white 22%, transparent),
                0 1px 2px color-mix(in oklch, var(--ok, oklch(0.74 0.15 145)) 25%, transparent);
  }
  .tl-node[data-status="error"]::before {
    background: var(--danger);
    border-color: color-mix(in oklab, var(--danger) 80%, transparent);
    box-shadow: inset 0 1px 0 color-mix(in oklch, white 22%, transparent),
                0 1px 2px color-mix(in oklab, var(--danger) 30%, transparent);
  }
  .tl-node[data-status="pending"]:not([data-kind="thinking"])::before {
    background: var(--accent);
    border-color: color-mix(in oklab, var(--accent) 75%, transparent);
    animation: tl-bullet-pulse 1.6s ease-in-out infinite;
  }
  @keyframes tl-bullet-pulse {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 0%, transparent),
                          inset 0 1px 0 color-mix(in oklch, white 25%, transparent); }
    50%      { box-shadow: 0 0 0 5px color-mix(in oklab, var(--accent) 20%, transparent),
                          inset 0 1px 0 color-mix(in oklch, white 25%, transparent); }
  }
  /* Short (<3s) or text-less done thinking blocks — visually recede so they
     don't create a wall of "Thought for <1s" noise between tool calls. Smaller
     + dimmer so they read as a faint timestamp, not a content row. Still
     readable and expandable on hover, just not competing for attention. */
  .tl-node[data-quick="true"] .tn-think-head {
    opacity: 0.52;
    font-size: 10px;
    padding-top: 0; padding-bottom: 0;
  }
  .tl-node[data-quick="true"] .tn-think-label { font-size: 10px; }
  .tl-node[data-quick="true"] .tn-think-meta { font-size: 9.5px; }
  .tl-node[data-quick="true"] .tn-think-head :global(svg) { width: 10px; height: 10px; }
  .tl-node[data-quick="true"]:hover .tn-think-head {
    opacity: 0.72;
  }
  /* The hollow rail bullet on a quick thought is also noise — recede it. */
  .tl-node[data-quick="true"][data-kind="thinking"]::before {
    opacity: 0.4;
    transform: scale(0.8);
  }
  /* Tighter vertical margin for quick thinking nodes — 8px default leaves too
     much gap when several appear between tool calls. */
  .bubble[data-role="assistant"] .tl-node[data-quick="true"] {
    margin-top: 1px;
  }

  /* Hover lifts the bullet — small but signals interactivity on tool/edit rows. */
  .tl-node[data-kind="tool"]:hover::before,
  .tl-node[data-kind="edit"]:hover::before {
    transform: scale(1.15);
    transition: transform 160ms cubic-bezier(0.22, 1, 0.36, 1), background 220ms ease-out, border-color 220ms ease-out, box-shadow 220ms ease-out;
  }
  /* Image nodes don't need a bullet — the user thumbnail carries its own
     framing + sits on the right side anyway. */
  .tl-node[data-kind="image"]::before { display: none; }
  /* User-side bubbles drop the rail (single-column grid), so the bullets
     would float in space — kill them. */
  .bubble[data-role="user"] .tl-node::before { display: none; }

  /* Collapsed tool group — a run of GROUP_MIN+ consecutive status chips folds
     into one header row so multi-tool turns stop reading as a wall of rows.
     Click to expand the chips; the live (last) node auto-opens while streaming
     so tools are watchable as they land, then collapses when the turn moves on.
     The group's rail bullet is drawn by .tl-node::before (data-kind="tool"). */
  /* Concept-D step card — the group reads as a self-contained card hanging off
     the spine, carrying a left status rail (quiet green when done, accent while
     running, loud red + tinted on error). Replaces the spine bullet so the rail
     isn't double-signalled. */
  /* Grouped work block (the agentic timeline) — ported 1:1 from the redesign
     spec (.work / .work-head / .work-body / .work-rail). The card hangs free in
     the turn (no spine bullet); live state pulses the spark, error tints border. */
  .work {
    position: relative;
    margin: 11px 0;
    border: 1px solid var(--border);
    border-radius: 11px;
    overflow: hidden;
    background: color-mix(in oklab, var(--bg-elev-1) 60%, transparent);
    transition: border-color var(--dur-fast);
  }
  .work::before { display: none; }
  .work.live { border-color: color-mix(in oklab, var(--accent) 26%, var(--border)); }
  .work[data-status="error"] {
    border-color: color-mix(in oklab, var(--danger) 42%, var(--border));
    background: color-mix(in oklab, var(--danger) 6%, var(--bg));
  }
  .work-head {
    display: flex; align-items: center; gap: 9px;
    width: 100%; padding: 9px 12px; min-height: 38px;
    background: transparent; border: 0; cursor: pointer;
    color: inherit; font: inherit; text-align: left;
    transition: background var(--dur-fast);
  }
  .work-head:hover { background: var(--surface-hover); }
  .work-chev { display: inline-flex; color: var(--fg-faint); flex: none; transition: transform var(--dur-fast); }
  .work-chev.open { transform: rotate(90deg); }
  .work-spark {
    display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; flex: none;
    background: var(--accent-soft); color: var(--accent);
  }
  .work.live .work-spark { animation: nodePulse var(--pulse-live) var(--ease-soft) infinite; }
  @keyframes nodePulse {
    0%, 100% { box-shadow: 0 0 0 0 transparent; }
    50% { box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 20%, transparent); }
  }
  .work-sum { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .work-sum-t { font-size: 12px; font-weight: 600; color: var(--fg); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .work-sum-s { font-size: 10.5px; color: var(--fg-faint); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .work-dur { font-size: 10.5px; color: var(--fg-faint); font-family: var(--font-mono); flex: none; font-variant-numeric: tabular-nums; }
  .work-count {
    display: inline-grid; place-items: center; min-width: 18px; height: 18px; padding: 0 5px; border-radius: 999px; flex: none;
    background: color-mix(in oklab, var(--fg) 7%, transparent); color: var(--fg-muted); font-size: 10px; font-weight: 700;
  }
  .work-body {
    border-top: 1px solid var(--border); padding: 10px 12px 11px; background: var(--bg-inset);
    animation: workOpen 0.34s var(--ease-page) both; overflow: hidden;
  }
  @keyframes workOpen { from { transform: translateY(-4px); } to { transform: none; } }
  /* chips-rail variant — each chip threaded on a vertical rail (spec L389-398) */
  .work-rail { position: relative; margin: 0; padding-left: 22px; display: flex; flex-direction: column; gap: 7px; }
  .work-rail::before {
    content: ""; position: absolute; left: 9px; top: 6px; bottom: 6px; width: 1.5px;
    background: linear-gradient(180deg, color-mix(in oklab, var(--accent) 30%, var(--border)), var(--border) 70%, transparent);
  }
  .work-rail :global(.chip) { position: relative; margin: 0; }
  .work-rail :global(.chip::before) {
    content: ""; position: absolute; left: -17px; top: 13px; width: 9px; height: 1.5px; background: var(--border-strong);
  }
  .work-rail :global(.chip::after) {
    content: ""; position: absolute; left: -22px; top: 9px; width: 9px; height: 9px; border-radius: 50%;
    background: var(--bg-inset); border: 1.5px solid var(--border-strong);
  }
  .work-rail :global(.chip[data-status="pending"]::after) { border-color: var(--accent); background: var(--accent-soft); }
  /* re-show each chip's own status icon (timeline variant hides it for the spine) */
  .work-rail :global(.chip[data-variant="timeline"] .chip-status) { display: inline-flex; }

  /* Step divider — uppercased label flanked by a faint line, sitting on
     the rail. Quiet visual punctuation between turn sections. */
  /* Step number as the rail bullet — replaces the ::before dot on numbered nodes; color tracks status. */
  .tl-node[data-numbered="true"]::before { display: none; }
  .tl-stepdot {
    position: absolute;
    left: -33px;
    top: 3px;
    min-width: 16px;
    height: 16px;
    padding: 0 3px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    z-index: 1;
    background: var(--bg);
    border: 1.5px solid color-mix(in oklch, var(--fg-faint) 55%, transparent);
    color: var(--fg-muted);
    /* bg halo masks the rail in a clean ring around the node so the spine
       appears to thread through it — the timeline "node on a wire" look. */
    box-shadow: 0 0 0 3px var(--bg);
    font-size: 9.5px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    transform-origin: center;
    animation: bullet-pop 340ms cubic-bezier(0.34, 1.56, 0.64, 1) both;
    animation-delay: calc(var(--idx, 0) * 35ms);
    transition: background 220ms ease-out, border-color 220ms ease-out, color 220ms ease-out, box-shadow 220ms ease-out;
  }
  @media (prefers-reduced-motion: reduce) { .tl-stepdot { animation: none; } }
  .tl-node[data-status="done"] .tl-stepdot {
    background: color-mix(in oklch, var(--ok, oklch(0.74 0.15 145)) 18%, var(--bg));
    border-color: color-mix(in oklch, var(--ok, oklch(0.74 0.15 145)) 58%, transparent);
    color: color-mix(in oklch, var(--ok, oklch(0.74 0.15 145)) 90%, var(--fg));
  }
  .tl-node[data-status="pending"] .tl-stepdot {
    background: color-mix(in oklab, var(--accent) 26%, var(--bg));
    border-color: color-mix(in oklab, var(--accent) 70%, transparent);
    color: color-mix(in oklab, var(--accent) 92%, var(--fg));
    animation: tl-bullet-pulse 1.6s ease-in-out infinite;
  }
  .tl-node[data-status="error"] .tl-stepdot {
    background: color-mix(in oklab, var(--danger) 24%, var(--bg));
    border-color: color-mix(in oklab, var(--danger) 65%, transparent);
    color: color-mix(in oklab, var(--danger) 90%, var(--fg));
  }
  .tl-node[data-numbered="true"]:hover .tl-stepdot {
    transform: scale(1.12);
    transition: transform 160ms cubic-bezier(0.22, 1, 0.36, 1), background 220ms ease-out, border-color 220ms ease-out, color 220ms ease-out;
  }

  /* Mockup `.ct-divider`: a quiet small-caps label, not a heavy condensed bar —
     lighter weight, tighter tracking, fading rule. */
  .tl-divider {
    display: flex; align-items: center; gap: 10px;
    margin: 10px 0 4px;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-subtle);
    animation: enter 240ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .tl-divider::after {
    content: "";
    flex: 1;
    height: 1px;
    background: linear-gradient(to right, var(--border), transparent);
  }
  .tl-divider-label {
    padding-right: 2px;
  }

  /* Mid-turn steer marker — the user's interjection, shown inline in the
     assistant timeline at the point it landed. Accent-tinted so it reads as
     a user action breaking into the agent's flow, distinct from prose. */
  .steer-marker {
    display: flex; align-items: baseline; gap: 6px;
    margin: 6px 0;
    padding: 5px 10px;
    background: color-mix(in oklab, var(--accent) 9%, transparent);
    border-left: 2px solid color-mix(in oklab, var(--accent) 60%, var(--border));
    border-radius: 4px;
    font-size: 12px;
    line-height: 1.4;
    animation: enter 240ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  :global(.steer-marker-icon) {
    color: color-mix(in oklab, var(--accent) 85%, var(--fg));
    flex: none;
    transform: translateY(1px);
  }
  .steer-marker-label {
    font-weight: 600;
    letter-spacing: 0.02em;
    color: color-mix(in oklab, var(--accent) 70%, var(--fg-muted));
    flex: none;
  }
  .steer-marker-text {
    color: var(--fg);
    overflow-wrap: anywhere;
  }
  @media (prefers-reduced-motion: reduce) {
    .steer-marker { animation: none; }
  }

  @media (prefers-reduced-motion: reduce) {
    .tl-node, .tl-divider, .cost-pill { animation: none; }
    .tl-node[data-status="pending"]::before,
    .tl-node[data-kind="thinking"][data-status="pending"]::before { animation: none; }
  }
  /* Copy + cost grouped inline right after the model label so they read as
     one action cluster next to the header — not pinned to the far column edge
     (which left them floating ~1000px from the content). Cost-first so the
     hover-only copy button doesn't reserve a gap at rest. */
  .turn-actions {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: 2px;
  }
  .copybtn {
    opacity: 0.42;
    background: transparent;
    border: 0;
    color: var(--fg-faint);
    padding: 2px 4px;
    border-radius: 4px;
    cursor: pointer;
    transition: opacity 120ms ease-out, color 120ms ease-out, background 120ms ease-out;
  }
  .bubble:hover .copybtn { opacity: 1; }
  .copybtn:hover { color: var(--fg); background: var(--surface-hover); }

  .content {
    display: flex; flex-direction: column;
    gap: 5px;
  }
  /* Per-block reveal handled by .stagger wrapper now (see grouped each loop).
     Don't double-animate here — would re-fire on inner text deltas. */
  .text {
    word-wrap: break-word;
    font-size: var(--fs-md);
    line-height: 1.55;
    color: var(--fg);
  }
  /* User text is raw — preserve newlines they typed.
     Assistant text is rendered HTML (Markdown component) so the default
     whitespace mode applies — `pre-wrap` here would render every `\n`
     between `</li>` and `<li>` in the marked output as a full line of
     empty space, stacking ~20px under every list item. */
  /* User prose: right-aligned accent-tinted bubble with a bottom-right tail.
     Position + tint signal "you" without an avatar or role label. */
  .bubble[data-role="user"] .text {
    padding: 10px 14px;
    background: color-mix(in oklab, var(--accent) 8%, var(--bg-inset));
    border: 1px solid color-mix(in oklab, var(--accent) 14%, var(--border));
    border-radius: 14px 14px 4px 14px;
    color: var(--fg);
    line-height: 1.6;
    align-self: flex-end;
    text-align: left;
    max-width: min(100%, 62ch);
    width: fit-content;
    white-space: pre-wrap;
  }
  .bubble[data-role="assistant"] .text { max-width: 78ch; }

  /* User image thumbnails — pasted/dropped into the composer get persisted
     on the user message and render here above the text bubble. Click opens
     the data URL in a new window for full-size view. */
  .user-image-thumb {
    align-self: flex-start;
    display: inline-flex;
    padding: 0;
    background: none;
    border: 1px solid var(--border);
    border-radius: 10px;
    cursor: pointer;
    overflow: hidden;
    max-width: min(100%, 320px);
    max-height: 240px;
    transition: border-color 140ms ease-out, transform 140ms ease-out, box-shadow 140ms ease-out;
  }
  .user-image-thumb:hover {
    border-color: color-mix(in oklab, var(--accent) 45%, var(--border));
    transform: translateY(-1px);
    box-shadow: 0 6px 18px color-mix(in oklab, var(--accent) 14%, transparent);
  }
  .user-image-thumb img {
    display: block;
    max-width: 100%;
    max-height: 240px;
    width: auto;
    height: auto;
    object-fit: cover;
    /* image-rendering for high-DPI sharpness on raster sources */
    image-rendering: auto;
  }
  .bubble[data-role="assistant"] .user-image-thumb { align-self: flex-start; }
  .bubble[data-role="user"] .user-image-thumb { align-self: flex-end; }

  /* Flat thinking node — no bordered surface; bullet on the rail carries
     the "this is a reasoning beat" signal. Label is single-line, prose
     opens inline below on click. */
  .tn-think {
    align-self: flex-start;
    max-width: min(100%, 78ch);
  }
  .tn-think-head {
    display: flex; align-items: center; gap: 6px;
    padding: 1px 4px 1px 0;
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    font-weight: 500;
    cursor: default;
    text-align: left;
    border-radius: 3px;
    transition: color 140ms ease-out, background 140ms ease-out;
  }
  .tn-think.expandable .tn-think-head { cursor: pointer; }
  .tn-think.expandable .tn-think-head:hover {
    background: color-mix(in oklch, var(--surface-hover) 60%, transparent);
    color: var(--fg-2);
  }
  .tn-think-head :global(svg) { opacity: 0.7; flex-shrink: 0; }
  .tn-think.active .tn-think-head { color: var(--accent); }
  .tn-think.active .tn-think-head :global(svg) { opacity: 0.9; color: var(--accent); }
  .tn-think-label {
    font-variant-numeric: tabular-nums;
    font-size: 11px;
  }
  .tn-think-meta {
    color: var(--fg-muted);
    font-size: 10.5px;
    opacity: 0.85;
  }
  .tn-think.active .tn-think-meta { color: color-mix(in oklab, var(--accent) 80%, var(--fg-muted)); }
  .chev {
    display: inline-flex;
    color: var(--fg-faint);
    transition: transform 160ms ease-out;
  }
  .chev.open { transform: rotate(180deg); }
  .dots { display: inline-flex; gap: 3px; margin-left: 2px; }
  .dots .dot {
    width: 4px; height: 4px; border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.1s ease-in-out infinite;
  }
  .dots .dot:nth-child(2) { animation-delay: 0.15s; }
  .dots .dot:nth-child(3) { animation-delay: 0.3s; }
  .tn-think-body {
    margin-top: 4px;
    padding: 6px 10px;
    border-left: 2px solid color-mix(in oklab, var(--accent) 28%, var(--border));
    background: color-mix(in oklch, var(--bg-elev-1) 70%, transparent);
    border-radius: 0 5px 5px 0;
    font-size: var(--fs-sm);
    line-height: 1.5;
    color: var(--fg-2);
    font-style: italic;
  }

  /* Liveness — live dot + heartbeat timer in role-row while streaming. */
  .live-dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 50%, transparent);
    animation: live-pulse 1.4s ease-in-out infinite;
    flex-shrink: 0;
  }
  @keyframes live-pulse {
    0%, 100% { opacity: 0.55; box-shadow: 0 0 0 0 color-mix(in oklab, var(--accent) 50%, transparent); }
    50%      { opacity: 1;    box-shadow: 0 0 0 4px color-mix(in oklab, var(--accent) 0%, transparent); }
  }
  .heartbeat {
    font-size: 10px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.01em;
  }
  .mono { font-family: var(--font-mono, monospace); }

  @keyframes pulse {
    0%, 60%, 100% { opacity: 0.3; transform: scale(0.85); }
    30% { opacity: 1; transform: scale(1); }
  }

  /* Streaming-only chrome — soft fade on the last line so newly-arriving
     text looks like it's materializing in. The role-row live-dot + whim
     label + heartbeat already carry the "still streaming" signal — no
     trailing caret (it leaked across bubbles + felt jittery). */
  .bubble[data-streaming="true"] .content {
    -webkit-mask-image: linear-gradient(180deg, #000 0, #000 calc(100% - 1.1em), rgba(0, 0, 0, 0.55) 100%);
    mask-image: linear-gradient(180deg, #000 0, #000 calc(100% - 1.1em), rgba(0, 0, 0, 0.55) 100%);
  }

  /* ── TurnSummary — caps a file-touching assistant turn ─────────────────── */
  .turn-summary {
    display: flex; align-items: center; justify-content: space-between; gap: 12px;
    flex-wrap: wrap;
    margin-top: 10px; padding: 9px 13px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 10px;
    font-size: var(--fs-sm);
  }
  .turn-summary[data-auto="true"] {
    background: var(--accent-soft);
    border-color: color-mix(in oklab, var(--accent) 28%, var(--border));
  }
  /* bypass permissions = dangerous → amber, matching the composer bypass pill */
  .turn-summary.mode-bypass {
    background: color-mix(in srgb, var(--warn) 9%, transparent);
    border-color: color-mix(in oklab, var(--warn) 42%, var(--border));
  }
  .ts-stats { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; color: var(--fg-muted); }
  .ts-item { display: inline-flex; align-items: center; gap: 6px; }
  .ts-item :global(svg) { color: var(--fg-subtle); }
  .ts-applied { display: inline-flex; align-items: center; gap: 6px; color: var(--accent); font-weight: 600; }
  .ts-applied :global(svg) { color: var(--accent); }
  .ts-applied.danger, .ts-applied.danger :global(svg) { color: var(--warn); }
  .ts-stat { display: inline-flex; gap: 6px; font-variant-numeric: tabular-nums; }
  .ts-add { color: var(--ok); }
  .ts-del { color: var(--danger); }
  .ts-dot { width: 3px; height: 3px; border-radius: 50%; background: var(--fg-faint); }
  .ts-actions { display: flex; align-items: center; gap: 8px; }
  .ts-mode {
    display: inline-flex; align-items: center; gap: 5px;
    padding: 2px 8px; border-radius: 999px;
    font-size: 10px; font-weight: 600; font-family: var(--font-mono);
    color: var(--accent);
    border: 1px solid var(--ghost-border);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    white-space: nowrap;
  }
  .ts-mode :global(svg) { color: inherit; }
  .ts-mode.mode-bypass {
    color: var(--warn);
    border-color: color-mix(in oklab, var(--warn) 42%, transparent);
    background: color-mix(in srgb, var(--warn) 12%, transparent);
  }

  /* Cross-link flash — pinged by Review-diff + the dock Steps stream. */
  :global(.tid-flash) { animation: tid-flash 1.4s var(--ease-soft); }
  @keyframes tid-flash {
    0%, 100% { box-shadow: 0 0 0 0 transparent; }
    12%      { box-shadow: 0 0 0 2px color-mix(in oklab, var(--accent) 55%, transparent), 0 0 14px color-mix(in oklab, var(--accent) 35%, transparent); }
  }
  @media (prefers-reduced-motion: reduce) { :global(.tid-flash) { animation: none; } }

  /* Full-screen image lightbox (portal'd to <body>). Sits above toasts (z-2000),
     below tooltips/splash (z-9999). Opaque scrim — WebView2 mis-composites
     backdrop-filter on fixed layers. */
  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 3000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px;
    background: oklch(0 0 0 / 0.78);
    cursor: zoom-out;
  }
  .lightbox-img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 8px;
    box-shadow: 0 24px 64px -16px oklch(0 0 0 / 0.7);
  }
  .lightbox-close {
    position: fixed;
    top: 16px; right: 16px;
    width: 34px; height: 34px;
    display: inline-flex; align-items: center; justify-content: center;
    color: var(--fg);
    background: color-mix(in oklab, var(--bg-elev-2) 80%, transparent);
    border: 1px solid var(--border-strong);
    border-radius: 50%;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .lightbox-close:hover { background: var(--surface-hover); }
</style>
