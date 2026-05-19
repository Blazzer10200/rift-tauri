<script lang="ts">
  import { User, Sparkles, Copy, Check, Brain, ChevronDown, Loader2 } from "lucide-svelte";
  import { onDestroy } from "svelte";
  import { assistant, type Block, type ChatMessage, type ThinkingBlock } from "../../state/assistant.svelte";
  import Markdown from "./Markdown.svelte";
  import EditDiff from "./EditDiff.svelte";
  import ToolChip from "./ToolChip.svelte";
  import StepGroup from "./StepGroup.svelte";

  // Tool blocks that render inline as a full side-by-side diff (vs the
  // compact ToolChip). Edit-family only — everything else gets a chip.
  function isInlineDiffTool(name: string): boolean {
    const sn = name.replace(/^mcp__rift__/, "");
    return sn === "Edit" || sn === "MultiEdit";
  }

  // Detect every "Step N — title" header line in a text block. Returns an
  // ordered list of prose segments interleaved with header markers so that a
  // single text block containing multiple headers (e.g. "## Step 1 — A\n##
  // Step 2 — B") splits into multiple step groups instead of collapsing the
  // tail into the first group's body.
  type TextSegment =
    | { kind: "prose"; text: string }
    | { kind: "header"; stepNum: number; title: string };
  const STEP_HEADER_LINE = /^\s*(?:#{1,6}\s+)?(?:\*\*)?Step\s+(\d+)\s*[—–\-:→»]\s*(.*?)(?:\*\*)?\s*$/i;
  function parseTextBlock(text: string): TextSegment[] {
    const out: TextSegment[] = [];
    const lines = text.split("\n");
    let prose: string[] = [];
    const flushProse = () => {
      const joined = prose.join("\n").trim();
      if (joined) out.push({ kind: "prose", text: joined });
      prose = [];
    };
    for (const line of lines) {
      const m = line.match(STEP_HEADER_LINE);
      if (m) {
        flushProse();
        const stepNum = parseInt(m[1], 10);
        const title = (m[2] ?? "").replace(/[*_`]+$/, "").trim() || `Step ${stepNum}`;
        out.push({ kind: "header", stepNum, title });
      } else {
        prose.push(line);
      }
    }
    flushProse();
    return out;
  }

  // When the model interleaves a tool call mid-header (e.g. emits "## S",
  // calls Bash, then continues "tep 1 — Long bash\n## Step 2 — …"), the text
  // arrives in two separate blocks separated by tool blocks. marked.js parses
  // the first chunk as a tear-shaped <h2>S</h2> and the second chunk as
  // prose, killing the header. This pre-pass detects that pattern (a text
  // block that's only a partial header prefix — `## ` plus a word fragment
  // with no terminator) and surgically reconstructs the first complete header
  // line, leaving any remaining tail (e.g. a *second* "## Step 2 …" header)
  // as its own text block AFTER the intervening tool calls — so the tools
  // attach to the recovered first step, not the second.
  const PARTIAL_HEADER = /^\s*#{1,6}\s+(?:\*\*)?[A-Za-z]+\s*$/;
  function reconcileSplitHeaders(blocks: Block[]): Block[] {
    const out: Block[] = [];
    let i = 0;
    while (i < blocks.length) {
      const b = blocks[i];
      if (b.type !== "text" || !PARTIAL_HEADER.test(b.text)) {
        out.push(b);
        i++;
        continue;
      }
      // Lookahead — collect non-text blocks until the next text block.
      let j = i + 1;
      const interim: Block[] = [];
      while (j < blocks.length && blocks[j].type !== "text") {
        interim.push(blocks[j]);
        j++;
      }
      const tail = j < blocks.length && blocks[j].type === "text" ? blocks[j] : null;
      if (!tail) {
        out.push(b);
        i++;
        continue;
      }
      const merged = b.text + tail.text;
      // Confirm the merge actually produces a Step header — otherwise this
      // partial wasn't a header at all, leave both blocks alone.
      if (!/(?:^|\n)\s*(?:#{1,6}\s+)?(?:\*\*)?Step\s+\d+/i.test(merged)) {
        out.push(b);
        i++;
        continue;
      }
      // Split the merged text at the first newline — that's the end of the
      // recovered header line. Everything after stays as the tail block.
      const firstNl = merged.indexOf("\n");
      const firstLine = firstNl === -1 ? merged : merged.slice(0, firstNl);
      const remainder = firstNl === -1 ? "" : merged.slice(firstNl + 1);
      out.push({ type: "text", text: firstLine });
      out.push(...interim);
      if (remainder.trim()) out.push({ type: "text", text: remainder });
      i = j + 1;
    }
    return out;
  }

  type StepStatus = "neutral" | "pending" | "done" | "error";
  type StepUnit =
    | { kind: "loose"; block: Block; key: string }
    | {
        kind: "step";
        stepNum: number;
        headerText: string;
        children: Block[];
        status: StepStatus;
        key: string;
      };

  // Roll up a step's tool children into a single status. Mirrors the chip
  // status semantics (pending = at least one running, error = at least one
  // errored, done = every tool resolved cleanly, neutral = no tools).
  function rollupStatus(children: Block[]): StepStatus {
    let saw = false, anyPending = false, anyError = false;
    for (const c of children) {
      if (c.type !== "tool") continue;
      saw = true;
      if (c.status === "error" || c.isError) anyError = true;
      else if (c.status === "pending") anyPending = true;
    }
    if (!saw) return "neutral";
    if (anyError) return "error";
    if (anyPending) return "pending";
    return "done";
  }

  let { message, streaming = false }: { message: ChatMessage; streaming?: boolean } = $props();

  let copied = $state(false);
  let expandedThinking = $state(new Set<number>());
  // Live tick so `active` thinking blocks + the role-row heartbeat show
  // real-time elapsed seconds. Runs while either an active thinking block
  // is present OR this bubble is the in-flight streaming message.
  let tickNow = $state(Date.now());
  const hasActiveThinking = $derived(
    streaming && message.blocks.some((b) => b.type === "thinking" && b.status === "active"),
  );
  let tickHandle: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    if ((hasActiveThinking || streaming) && !tickHandle) {
      tickHandle = setInterval(() => (tickNow = Date.now()), 500);
    } else if (!hasActiveThinking && !streaming && tickHandle) {
      clearInterval(tickHandle);
      tickHandle = null;
    }
  });
  onDestroy(() => {
    if (tickHandle) clearInterval(tickHandle);
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

  // Whimsical fallbacks for the bare "Thinking…" stage label. Cycles
  // every ~2.4s while streaming so the user feels the heartbeat is real
  // instead of staring at a static word. Per-tool labels ("Reading X",
  // "Running cargo check") flow through unchanged — only the plain
  // "Thinking" baseline gets the swap.
  const WHIM_WORDS = [
    "Thinking", "Sussing", "Spelunking", "Pondering", "Brewing",
    "Reckoning", "Mulling", "Cogitating", "Hatching", "Conjuring",
    "Noodling", "Untangling",
  ];
  let whimTick = $state(0);
  $effect(() => {
    if (!streaming) return;
    const id = setInterval(() => (whimTick = (whimTick + 1) % WHIM_WORDS.length), 2400);
    return () => clearInterval(id);
  });
  // Empty-bubble stage label: prefer the global activity.currentLabel
  // ("Reading auto_sync.rs", "Running cargo check", …); when it's the
  // bare "Thinking" baseline, cycle through the whim pool.
  const stageLabel = $derived.by(() => {
    if (!streaming) return null;
    const raw = assistant.activity.currentLabel ?? "Thinking…";
    if (/^thinking/i.test(raw)) return `${WHIM_WORDS[whimTick]}…`;
    return raw;
  });

  const plainText = $derived(
    message.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim(),
  );

  const isUser = $derived(message.role === "user");

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms} ms`;
    const s = ms / 1000;
    return s < 10 ? `${s.toFixed(1)}s` : `${Math.round(s)}s`;
  }

  function elapsedFor(b: ThinkingBlock, startSeed: number): string {
    // For done blocks use stored durationMs; for active, derive from tickNow.
    // startSeed is unused but referenced so Svelte tracks tickNow dep below.
    void startSeed;
    if (b.status === "done" && b.durationMs != null) return formatDuration(b.durationMs);
    return "…";
  }

  function previewOf(text: string): string {
    // First non-empty line, lightly trimmed for a glimpse-style preview.
    const line = text.split(/\n+/).find((l) => l.trim().length > 0) ?? "";
    const t = line.trim().replace(/^[#*>`\-_\s]+/, "");
    return t.length > 110 ? t.slice(0, 108) + "…" : t;
  }

  function toggleThinking(i: number) {
    const next = new Set(expandedThinking);
    if (next.has(i)) next.delete(i);
    else next.add(i);
    expandedThinking = next;
  }

  async function copy() {
    if (!plainText) return;
    try {
      await navigator.clipboard.writeText(plainText);
      copied = true;
      setTimeout(() => (copied = false), 1200);
    } catch (e) {
      console.warn("copy failed", e);
    }
  }

  // Tighten the resolved model id into a short human label.
  //   claude-sonnet-4-6-20251001  → Sonnet 4.6
  //   claude-opus-4-7[1m]         → Opus 4.7
  //   claude-haiku-4-5            → Haiku 4.5
  function shortModel(id: string): string {
    const m = /claude-(opus|sonnet|haiku)-(\d+)-(\d+)/i.exec(id);
    if (!m) return id;
    const name = m[1][0].toUpperCase() + m[1].slice(1).toLowerCase();
    return `${name} ${m[2]}.${m[3]}`;
  }
  const modelLabel = $derived(message.model ? shortModel(message.model) : null);
  const costLabel = $derived(
    typeof message.costUsd === "number" ? `$${message.costUsd.toFixed(4)}` : null,
  );

  // Walk the message's blocks and group them into "steps" — visual units
  // headed by a `Step N — title` text line. Loose blocks (pre-step prose,
  // reasoning, leading tool calls) render flat above the first group.
  const grouped = $derived.by<StepUnit[]>(() => {
    const units: StepUnit[] = [];
    let current: { stepNum: number; headerText: string; children: Block[] } | null = null;
    const flush = () => {
      if (current) {
        units.push({
          kind: "step",
          ...current,
          status: rollupStatus(current.children),
          key: `s_${current.stepNum}_${units.length}`,
        });
        current = null;
      }
    };
    const blocks = reconcileSplitHeaders(message.blocks);
    for (let i = 0; i < blocks.length; i++) {
      const b = blocks[i];
      // Reasoning blocks always render flat (they live at the top of a turn).
      if (b.type === "thinking") {
        flush();
        units.push({ kind: "loose", block: b, key: `l_t${i}` });
        continue;
      }
      if (b.type === "text") {
        const segments = parseTextBlock(b.text);
        for (let si = 0; si < segments.length; si++) {
          const seg = segments[si];
          if (seg.kind === "header") {
            flush();
            current = { stepNum: seg.stepNum, headerText: seg.title, children: [] };
          } else {
            const proseBlock: Block = { type: "text", text: seg.text };
            if (current) current.children.push(proseBlock);
            else units.push({ kind: "loose", block: proseBlock, key: `l_${i}_${si}` });
          }
        }
        continue;
      }
      // Tool blocks: attach to current step, or render loose if no step yet.
      if (current) current.children.push(b);
      else units.push({ kind: "loose", block: b, key: `l_${i}` });
    }
    flush();
    return units;
  });

  // Auto-collapse helpers for the each-loop — total step count + key of
  // the last step. Both drive the rule "collapse done mid-turn steps when
  // the turn has ≥4 steps, but never collapse the last one (most recent
  // context) or pending/error steps (need visibility)".
  const stepCount = $derived(grouped.reduce((n, u) => n + (u.kind === "step" ? 1 : 0), 0));
  const lastStepKey = $derived.by<string | null>(() => {
    for (let i = grouped.length - 1; i >= 0; i--) {
      const u = grouped[i];
      if (u.kind === "step") return u.key;
    }
    return null;
  });
</script>

<div class="bubble" data-role={message.role} data-streaming={streaming ? "true" : null}>
  <div class="avatar" aria-hidden="true">
    {#if isUser}
      <User size={13} />
    {:else}
      <Sparkles size={13} />
    {/if}
  </div>

  <div class="body">
    <div class="role-row">
      <span class="role-name">{isUser ? "You" : "Claude"}</span>
      {#if !isUser && streaming}
        <span class="live-dot" aria-label="Streaming" title="Streaming response"></span>
        {#if heartbeatLabel}<span class="heartbeat mono" title="Elapsed since turn started">{heartbeatLabel}</span>{/if}
      {/if}
      {#if !isUser && (modelLabel || costLabel)}
        <span class="turn-badge" title="Model · per-turn cost">
          {#if modelLabel}<span class="turn-model">{modelLabel}</span>{/if}
          {#if modelLabel && costLabel}<span class="turn-sep">·</span>{/if}
          {#if costLabel}<span class="turn-cost">{costLabel}</span>{/if}
        </span>
      {/if}
      {#if plainText.length > 0}
        <button class="copybtn" type="button" onclick={copy} title="Copy">
          {#if copied}
            <Check size={11} />
          {:else}
            <Copy size={11} />
          {/if}
        </button>
      {/if}
    </div>

    {#if !isUser && streaming && stageLabel}
      {@const isShell = /^\$\s/.test(stageLabel)}
      <div class="stream-status" title={stageLabel}>
        <Loader2 size={11} class="stage-spin" />
        {#key stageLabel}
          <span class="stream-status-text" class:mono={isShell} class:prose={!isShell}>
            {stageLabel}
          </span>
        {/key}
      </div>
    {/if}

    <div class="content">
      {#snippet renderBlock(b: Block, bi: number)}
        {#if b.type === "text" && b.text.length > 0}
          {#if isUser}
            <div class="text">{b.text}</div>
          {:else}
            <div class="text"><Markdown text={b.text} /></div>
          {/if}
        {:else if b.type === "thinking"}
          {@const isActive = b.status === "active"}
          {@const hasText = b.text.length > 0}
          {@const isOpen = expandedThinking.has(bi)}
          {@const elapsed = elapsedFor(b, tickNow)}
          <div class="reasoning" class:active={isActive} class:expandable={hasText}>
            <button
              type="button"
              class="reasoning-head"
              onclick={() => hasText && toggleThinking(bi)}
              disabled={!hasText}
              aria-expanded={isOpen}
            >
              <Brain size={12} />
              <span class="reasoning-label">
                {#if isActive}Thinking{:else}Reasoned{/if}
              </span>
              <span class="reasoning-meta">{elapsed}</span>
              {#if isActive}
                <span class="dots" aria-hidden="true">
                  <span class="dot"></span><span class="dot"></span><span class="dot"></span>
                </span>
              {/if}
              {#if hasText}
                <span class="chev" class:open={isOpen}><ChevronDown size={12} /></span>
              {/if}
            </button>
            {#if hasText && !isOpen && !isActive}
              <div class="reasoning-preview">{previewOf(b.text)}</div>
            {/if}
            {#if hasText && isOpen}
              <div class="reasoning-body"><Markdown text={b.text} /></div>
            {/if}
          </div>
        {:else if b.type === "tool" && isInlineDiffTool(b.name)}
          {#if b.name === "MultiEdit" && Array.isArray(b.input.edits)}
            {#each (b.input.edits as Array<Record<string, unknown>>) as edit, ei (ei)}
              <EditDiff input={{ ...edit, file_path: b.input.file_path }} />
            {/each}
          {:else}
            <EditDiff input={b.input} />
          {/if}
        {:else if b.type === "tool"}
          <ToolChip tool={b} />
        {/if}
      {/snippet}

      {#each grouped as unit, ui (unit.key)}
        {#if unit.kind === "loose"}
          {@render renderBlock(unit.block, ui)}
        {:else}
          {@const isLast = unit.key === lastStepKey}
          {@const collapsible = stepCount >= 4 && unit.status === "done" && !isLast}
          {@const toolCount = unit.children.reduce((n, b) => n + (b.type === "tool" ? 1 : 0), 0)}
          {@const childSummary = toolCount > 0 ? `${toolCount} tool${toolCount === 1 ? "" : "s"}` : null}
          <StepGroup
            stepNum={unit.stepNum}
            headerText={unit.headerText}
            status={unit.status}
            {collapsible}
            {childSummary}
          >
            {#snippet children()}
              {#each unit.children as child, ci (ci)}
                {@render renderBlock(child, ci)}
              {/each}
            {/snippet}
          </StepGroup>
        {/if}
      {/each}

    </div>
  </div>
</div>

<style>
  .bubble {
    display: grid;
    grid-template-columns: 28px 1fr;
    gap: 12px;
    padding: 4px 0;
    animation: msg-in 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes msg-in {
    from { opacity: 0; transform: translateY(4px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .avatar {
    width: 26px; height: 26px;
    border-radius: 50%;
    display: flex; align-items: center; justify-content: center;
    margin-top: 0;
  }
  .bubble[data-role="user"] .avatar {
    background: var(--bg-elev-2);
    color: var(--fg-muted);
    border: 1px solid var(--border);
  }
  .bubble[data-role="assistant"] .avatar {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .body { min-width: 0; }
  .role-row {
    display: flex; align-items: center; gap: 8px;
    margin-bottom: 2px;
    height: 16px;
  }
  .role-name {
    font-size: var(--fs-xs);
    font-weight: 600;
    color: var(--fg-2);
  }
  .turn-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    line-height: 1;
    padding: 2px 7px;
    border-radius: 999px;
    background: color-mix(in oklch, var(--accent-soft) 60%, var(--bg-elev-2));
    border: 1px solid color-mix(in oklch, var(--accent) 14%, var(--border));
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.01em;
    white-space: nowrap;
    flex-shrink: 0;
    margin-left: auto;
  }
  .turn-model { color: var(--fg-2); font-weight: 500; }
  .turn-sep { color: var(--fg-faint); }
  .turn-cost { color: var(--fg-muted); font-family: var(--font-mono, monospace); }
  .copybtn {
    opacity: 0;
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
    gap: 6px;
  }
  /* Per-block reveal — each direct child gently materializes on mount.
     Streaming chunks inside an already-mounted block don't re-trigger;
     the existing caret + bottom-mask handle in-place text growth. */
  .content > .text,
  .content > .reasoning {
    animation: block-in 220ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  @keyframes block-in {
    from { opacity: 0; transform: translateY(3px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .content > .text,
    .content > .reasoning { animation: none; }
  }
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
  .bubble[data-role="user"] .text {
    padding: 8px 12px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in oklch, var(--accent) 22%, transparent);
    border-radius: 10px;
    color: var(--fg);
    align-self: flex-start;
    max-width: min(100%, 78ch);
    width: fit-content;
    white-space: pre-wrap;
  }
  .bubble[data-role="assistant"] .text { max-width: 78ch; }

  /* Reasoning / thinking surface */
  .reasoning {
    align-self: flex-start;
    max-width: min(100%, 78ch);
    border: 1px solid var(--border);
    background: color-mix(in oklch, var(--accent-soft) 35%, var(--bg-elev-1));
    border-radius: 8px;
    overflow: hidden;
    transition: border-color 180ms ease-out, background 180ms ease-out;
  }
  .reasoning.active {
    border-color: color-mix(in oklch, var(--accent) 45%, transparent);
    background: color-mix(in oklch, var(--accent-soft) 60%, var(--bg-elev-1));
    animation: reasoning-glow 2.4s ease-in-out infinite;
  }
  @keyframes reasoning-glow {
    0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklch, var(--accent) 0%, transparent); }
    50%      { box-shadow: 0 0 0 3px color-mix(in oklch, var(--accent) 14%, transparent); }
  }
  .reasoning-head {
    display: flex; align-items: center; gap: 6px;
    width: 100%;
    padding: 5px 9px;
    background: transparent;
    border: 0;
    color: var(--fg-2);
    font-size: var(--fs-xs);
    font-weight: 500;
    cursor: default;
    text-align: left;
  }
  .reasoning.expandable .reasoning-head { cursor: pointer; }
  .reasoning.expandable .reasoning-head:hover { background: var(--surface-hover); }
  .reasoning-head[disabled] { opacity: 1; } /* keep visible state */
  .reasoning-label { color: var(--fg); }
  .reasoning.active .reasoning-label { color: var(--accent); }
  .reasoning-meta {
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    font-size: 11px;
  }
  .chev {
    margin-left: auto;
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
  .reasoning-preview {
    padding: 0 10px 7px 10px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--fg-muted);
    font-style: italic;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .reasoning-body {
    padding: 4px 10px 9px 10px;
    border-top: 1px solid var(--border);
    font-size: var(--fs-sm);
    line-height: 1.5;
    color: var(--fg-2);
    font-style: italic;
    background: color-mix(in oklch, var(--bg-elev-1) 60%, transparent);
  }

  /* Liveness — live dot + heartbeat timer in role-row while streaming. */
  .live-dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 0 color-mix(in oklch, var(--accent) 50%, transparent);
    animation: live-pulse 1.4s ease-in-out infinite;
    flex-shrink: 0;
  }
  @keyframes live-pulse {
    0%, 100% { opacity: 0.55; box-shadow: 0 0 0 0 color-mix(in oklch, var(--accent) 50%, transparent); }
    50%      { opacity: 1;    box-shadow: 0 0 0 4px color-mix(in oklch, var(--accent) 0%, transparent); }
  }
  .heartbeat {
    font-size: 10px;
    color: var(--fg-muted);
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.01em;
  }
  .mono { font-family: var(--font-mono, monospace); }

  /* Stream status sub-row — single source of truth for the live activity
     label (formerly split across role-row `.whim` + bubble-empty `.stage-row`).
     Sits directly under role-row, full-width, single-line truncated. Keeps
     long bash commands and absolute paths out of the role-row so the model
     badge + heartbeat never get pushed off-screen. */
  .stream-status {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    max-width: min(100%, 78ch);
    margin: -1px 0 4px;
    color: var(--fg-muted);
    animation: status-fade 260ms ease-out;
  }
  .stream-status :global(.stage-spin) {
    color: var(--accent);
    flex-shrink: 0;
    animation: stage-spin 1s linear infinite;
  }
  .stream-status-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 11px;
    letter-spacing: 0.01em;
    line-height: 1.3;
  }
  .stream-status-text.prose {
    font-style: italic;
    color: color-mix(in oklch, var(--accent) 75%, var(--fg-muted));
  }
  .stream-status-text.mono {
    font-family: var(--font-mono, monospace);
    font-size: 10.5px;
    color: var(--fg-muted);
  }
  @keyframes status-fade {
    from { opacity: 0; transform: translateY(-1px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @keyframes stage-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    .stream-status { animation: none; }
    .stream-status :global(.stage-spin) { animation: none; }
  }
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
</style>
