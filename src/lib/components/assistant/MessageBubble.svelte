<script lang="ts">
  import { Sparkles, Copy, Check, Brain, ChevronDown, ChevronRight, User, Wrench, Loader2, CheckCircle2, AlertCircle } from "lucide-svelte";
  import { onDestroy } from "svelte";
  import { fade, slide } from "svelte/transition";
  const reducedMotion =
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;
  import { assistant, type Block, type ChatMessage, type ThinkingBlock } from "../../state/assistant.svelte";
  import { diffArrays } from "diff";
  import { GitCommit, Clock, FileText, RotateCcw } from "lucide-svelte";
  import Markdown from "./Markdown.svelte";
  import EditDiff from "./EditDiff.svelte";
  import ToolChip from "./ToolChip.svelte";
  import PermissionBar from "./PermissionBar.svelte";
  import { captionForTool, captionForGroup } from "./toolCaption";

  import { tooltip } from "$lib/actions/tooltip";
  // Tool blocks that render inline as a full side-by-side diff (vs the
  // compact ToolChip). Edit-family only — everything else gets a chip.
  function isInlineDiffTool(name: string): boolean {
    const sn = name.replace(/^mcp__rift__/, "");
    return sn === "Edit" || sn === "MultiEdit";
  }
  function shortToolName(name: string): string { return name.replace(/^mcp__rift__/, ""); }
  // Card-style tools render first-class chrome (their body IS the message) —
  // never fold them into a collapsed tool group.
  function isCardTool(name: string): boolean {
    return /^(mcp__rift__)?Agent$/.test(name)
      || /^(mcp__rift__)?TodoWrite$/.test(name)
      || /^(mcp__rift__)?AskUserQuestion$/.test(name)
      || /^mcp__rift__ask_user$/.test(name);
  }
  // Groupable = a plain status chip (Read/Grep/Bash/…): not an inline diff
  // (Edit/MultiEdit) and not a first-class card. Runs of these collapse.
  function isGroupableChip(name: string): boolean {
    return !isInlineDiffTool(name) && !isCardTool(name);
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
      const candidate = j < blocks.length ? blocks[j] : null;
      const tail = candidate && candidate.type === "text" ? candidate : null;
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

  // Timeline-flat units. Step-N headers from prose become dividers (small
  // inline labels on the rail) instead of numbered groups. Every other
  // block becomes its own node on the chain.
  type NodeStatus = "neutral" | "pending" | "done" | "error";
  type TimelineUnit =
    | { kind: "block"; block: Block; key: string; status: NodeStatus; stepNum?: number; caption?: string }
    | { kind: "toolgroup"; blocks: Block[]; key: string; status: NodeStatus; stepNum?: number; caption?: string }
    | { kind: "divider"; stepNum: number; title: string; key: string };

  function statusOf(b: Block): NodeStatus {
    if (b.type === "tool") {
      if (b.status === "error" || b.isError) return "error";
      if (b.status === "pending") return "pending";
      return "done";
    }
    if (b.type === "thinking") return b.status === "active" ? "pending" : "done";
    return "neutral";
  }
  function nodeKind(b: Block): "thinking" | "prose" | "tool" | "edit" | "image" {
    if (b.type === "thinking") return "thinking";
    if (b.type === "text") return "prose";
    if (b.type === "image") return "image";
    if (b.type === "tool") return isInlineDiffTool(b.name) ? "edit" : "tool";
    return "prose";
  }

  let { message, streaming = false, isLast = false }: { message: ChatMessage; streaming?: boolean; isLast?: boolean } = $props();

  let copied = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;
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

  function formatBoundaryAt(ms: number): string {
    return new Date(ms).toLocaleTimeString([], { hour12: true });
  }

  function formatDuration(ms: number): string {
    if (ms < 1000) return "<1s";
    const s = Math.floor(ms / 1000);
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    const rem = s % 60;
    return rem === 0 ? `${m}m` : `${m}m ${rem}s`;
  }

  function elapsedFor(b: ThinkingBlock, nowMs: number): string {
    // Done block → stored duration. Active block → live ms-from-start so the
    // role-row label ticks up as the model reasons (otherwise "Thinking …"
    // sits frozen for the full 17-40s on Opus tool-use turns).
    if (b.status === "done" && b.durationMs != null) return formatDuration(b.durationMs);
    if (b.status === "active") {
      const ms = Math.max(0, nowMs - b.startedAt);
      return formatDuration(ms);
    }
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

  // Collapsed tool-group expand state, keyed by group key.
  let expandedGroups = $state(new Set<string>());
  function toggleGroup(key: string) {
    const next = new Set(expandedGroups);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    expandedGroups = next;
  }
  // Compact "Read ×3 · Grep · Bash" rollup for a collapsed tool-group head.
  function summarizeGroup(blocks: Block[]): string {
    const counts = new Map<string, number>();
    for (const b of blocks) {
      if (b.type !== "tool") continue;
      const n = shortToolName(b.name);
      counts.set(n, (counts.get(n) ?? 0) + 1);
    }
    const parts = [...counts].map(([n, c]) => (c > 1 ? `${n} ×${c}` : n));
    const shown = parts.slice(0, 4).join(" · ");
    return parts.length > 4 ? `${shown} +${parts.length - 4}` : shown;
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
  function lineDelta(oldS: unknown, newS: unknown): { adds: number; dels: number } {
    if (typeof oldS !== "string" || typeof newS !== "string") return { adds: 0, dels: 0 };
    let adds = 0, dels = 0;
    for (const c of diffArrays(oldS.split("\n"), newS.split("\n"))) {
      if (c.added) adds += c.count ?? c.value.length;
      else if (c.removed) dels += c.count ?? c.value.length;
    }
    return { adds, dels };
  }
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
  // Open the Session Diff overlay, deep-linked to this turn's first edited file
  // (matched by basename). Replaces the old transcript-scroll that pointed at a
  // removed `actnode-*` anchor and silently no-op'd.
  function reviewDiff() {
    if (turnStats.files === 0) return;
    const fp = turnStats.firstEditFile;
    assistant.ui.diffTarget = fp ? (fp.replace(/\\/g, "/").split("/").pop() ?? null) : null;
    assistant.ui.diffOpen = true;
  }

  // Fold a run of GROUP_MIN+ consecutive plain tool chips into a single
  // collapsible "N tools" node so a multi-tool turn stops reading as a wall of
  // rows. Prose / thinking / edits / cards / images all break a run, so the
  // narration↔tool ordering is preserved — only back-to-back status chips
  // collapse. Runs shorter than GROUP_MIN stay inline as before.
  const GROUP_MIN = 3;
  function coalesceToolGroups(units: TimelineUnit[]): TimelineUnit[] {
    const out: TimelineUnit[] = [];
    let run: Extract<TimelineUnit, { kind: "block" }>[] = [];
    const flush = () => {
      if (run.length === 0) return;
      if (run.length < GROUP_MIN) {
        out.push(...run);
      } else {
        const status: NodeStatus = run.some((u) => u.status === "error")
          ? "error"
          : run.some((u) => u.status === "pending")
            ? "pending"
            : "done";
        out.push({
          kind: "toolgroup",
          blocks: run.map((u) => u.block),
          key: `tg_${run[0].key}`,
          status,
        });
      }
      run = [];
    };
    for (const u of units) {
      if (u.kind === "block" && u.block.type === "tool" && isGroupableChip(u.block.name)) {
        run.push(u);
      } else {
        flush();
        out.push(u);
      }
    }
    flush();
    return out;
  }

  // Number each action unit (chip/group/edit) sequentially; caption = preceding
  // "Step N" divider title if any, else synthesized. Orphan dividers kept.
  function numberActions(units: TimelineUnit[]): TimelineUnit[] {
    const out: TimelineUnit[] = [];
    let step = 0;
    let pending: { title: string; stepNum: number } | null = null;
    for (const u of units) {
      if (u.kind === "block" && u.block.type === "tool") {
        step++;
        const caption = pending?.title ?? captionForTool(u.block.name, u.block.input as Record<string, unknown>);
        pending = null;
        out.push({ ...u, stepNum: step, caption });
      } else if (u.kind === "toolgroup") {
        step++;
        const caption = pending?.title ?? captionForGroup(u.blocks);
        pending = null;
        out.push({ ...u, stepNum: step, caption });
      } else if (u.kind === "divider") {
        pending = { title: u.title, stepNum: u.stepNum };
      } else {
        if (pending) {
          out.push({ kind: "divider", stepNum: pending.stepNum, title: pending.title, key: `od_${u.key}` });
          pending = null;
        }
        out.push(u);
      }
    }
    if (pending) out.push({ kind: "divider", stepNum: pending.stepNum, title: pending.title, key: "od_tail" });
    return out;
  }

  // Walk the message's blocks → flat TimelineUnit list. Step headers in
  // prose become dividers; everything else becomes a node on the chain.
  const grouped = $derived.by<TimelineUnit[]>(() => {
    const units: TimelineUnit[] = [];
    const blocks = reconcileSplitHeaders(message.blocks);
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
    return numberActions(coalesceToolGroups(units));
  });

  // Trailing activity row — fills the "blank while working" gap the bare
  // bubble had once text started streaming (the stage-strip only shows
  // pre-first-block). Walk from the end of the block list: if the tail is a
  // pending tool or active thinking, that node ALREADY pulses (its own bullet
  // / spinner) — suppress the trailing row to avoid a double "working" signal.
  // If the tail is prose (model composing more text / a code block), nothing
  // else signals liveness down there → show the compact trailing pulse.
  const tailIsPending = $derived.by<boolean>(() => {
    for (let i = message.blocks.length - 1; i >= 0; i--) {
      const b = message.blocks[i];
      if (b.type === "tool") return b.status === "pending";
      if (b.type === "thinking") return b.status === "active";
      if (b.type === "text" && b.text.length > 0) return false;
    }
    return false;
  });
  const showTrailingActivity = $derived(
    !isUser && streaming && grouped.length > 0 && !tailIsPending,
  );

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

{#if isSystem && boundaryBlock}
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
<div class="bubble" data-role={message.role} data-model={modelFamily} data-streaming={streaming ? "true" : null}>
  <div class="turn-rail" aria-hidden="true"></div>
  <div class="avatar" class:avatar-user={isUser} aria-hidden="true">
    {#if isUser}<User size={13} />{:else}<Sparkles size={13} />{/if}
  </div>

  <div class="body">
    {#if !isUser}
      <div class="turn-head">
        <span class="role-name">Claude</span>
        {#if modelLabel}
          <span class="head-sep" aria-hidden="true">·</span>
          <span class="head-model" use:tooltip={"Model for this turn"}>{modelLabel}</span>
        {/if}
        {#if streaming}
          {#if heartbeatLabel}
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
              <button class="copybtn" type="button" onclick={copy} use:tooltip={"Copy"}>
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
    {:else}
      <div class="turn-head">
        <span class="role-name">You</span>
        {#if plainText.length > 0}
          <div class="turn-actions">
            <button class="copybtn" type="button" onclick={copy} use:tooltip={"Copy"}>
              {#if copied}
                <Check size={11} />
              {:else}
                <Copy size={11} />
              {/if}
            </button>
          </div>
        {/if}
      </div>
    {/if}

    {#if !isUser && streaming && grouped.length === 0}
      <div class="stage-strip" aria-live="polite" out:fade={{ duration: 220 }}>
        <div class="stage-line">
          <span class="stage-dots" aria-hidden="true">
            <span class="stage-dot"></span>
            <span class="stage-dot"></span>
            <span class="stage-dot"></span>
          </span>
          {#key stageLabel}
            <span class="stage-label">{stageLabel ?? "Thinking…"}</span>
          {/key}
        </div>
      </div>
    {/if}

    <div class="content">
      {#snippet renderBlock(b: Block, bi: number, caption: string | null = null, revealing: boolean = false)}
        {#if b.type === "image"}
          <button
            type="button"
            class="user-image-thumb"
            use:tooltip={"Click to view full size"}
            onclick={() => window.open(`data:${b.mime};base64,${b.dataBase64}`, "_blank")}
          >
            <img
              src={`data:${b.mime};base64,${b.dataBase64}`}
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
              <div class="tn-think-body"><Markdown text={b.text} /></div>
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
          {@const open = expandedGroups.has(unit.key) || (streaming && isLastNode)}
          <div
            class="tl-node tl-toolgroup"
            data-kind="tool"
            data-status={nodeStatus}
            data-open={open ? "true" : null}
            data-numbered={unit.stepNum ? "true" : null}
            style="--idx: {Math.min(ui, 6)}"
          >
            {#if unit.stepNum}<span class="tl-stepdot mono" aria-hidden="true">{unit.stepNum}</span>{/if}
            <button class="tg-head" type="button" onclick={() => toggleGroup(unit.key)} aria-expanded={open}>
              <span class="tg-chev" class:open><ChevronRight size={11} /></span>
              <Wrench size={12} class="tg-icon" />
              <span class="tg-cap">{unit.caption ?? `${unit.blocks.length} tools`}</span>
              <span class="tg-sep" aria-hidden="true">·</span>
              <span class="tg-sum mono">{summarizeGroup(unit.blocks)}</span>
              <span class="tg-status">
                {#if unit.status === "pending"}<Loader2 size={11} class="tg-spin" />
                {:else if unit.status === "error"}<AlertCircle size={11} />
                {:else}<CheckCircle2 size={11} />{/if}
              </span>
            </button>
            {#if open}
              <div class="tg-body" transition:slide={{ duration: reducedMotion ? 0 : 200 }}>
                {#each unit.blocks as gb, gi (gb.type === "tool" ? gb.id : gi)}
                  {@render renderBlock(gb, 10000 + ui * 100 + gi)}
                {/each}
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
            {@render renderBlock(unit.block, ui, unit.caption, streaming && !isUser && unit.block.type === "text" && unit.key === lastBlockKey)}
          </div>
        {/if}
      {/each}

      {#if showTrailingActivity}
        <div class="trailing-activity" aria-live="polite" in:fade={{ duration: reducedMotion ? 0 : 160 }} out:fade={{ duration: reducedMotion ? 0 : 160 }}>
          <span class="ta-dots" aria-hidden="true">
            <span class="ta-dot"></span><span class="ta-dot"></span><span class="ta-dot"></span>
          </span>
          {#key stageLabel}
            <span class="ta-label">{stageLabel ?? "Working…"}</span>
          {/key}
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
            <button type="button" class="ts-btn" onclick={reviewDiff}><FileText size={13} />Review diff</button>
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

<style>
  .boundary {
    width: 100%;
    padding: 8px 4px;
    display: flex;
    flex-direction: column;
    gap: 6px;
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
  /* User bubbles drop the rail entirely + right-align — the bubble shape
     already signals "you", and the position differentiates from Claude
     without forcing a rail on user messages. */
  /* User turns share Claude's grid + left edge so the thread reads as one calm
     left-aligned column instead of ping-ponging left↔right. The 2px rail column
     stays empty for user (no rail element), so user content lands at the same x
     as Claude's content. */
  .bubble[data-role="user"] .body {
    align-items: flex-start;
    display: flex; flex-direction: column;
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
  /* User turns share the spine but in a quiet neutral grey — so both roles read
     as "rail + content," differentiated by hue (your neutral vs Claude's model
     tint), not by box-vs-no-box. */
  .bubble[data-role="user"] .turn-rail {
    background: linear-gradient(to bottom,
      transparent 0,
      color-mix(in oklch, var(--fg-faint) 42%, var(--border)) 10px,
      color-mix(in oklch, var(--fg-faint) 42%, var(--border)) calc(100% - 10px),
      transparent 100%);
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
  .avatar.avatar-user {
    background: color-mix(in oklch, var(--bg-elev-2) 90%, var(--fg-faint));
    color: var(--fg-muted);
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
  .bubble[data-role="user"] .turn-head { justify-content: flex-start; }
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
    display: flex; flex-direction: column;
    gap: 9px;
    padding: 2px 0 6px;
    animation: enter 240ms cubic-bezier(0.22, 1, 0.36, 1) both;
  }
  .stage-line {
    display: inline-flex; align-items: center; gap: 9px;
    color: var(--fg-2);
    font-size: var(--fs-sm);
    min-height: 14px;
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
  .stage-label {
    color: var(--fg);
    font-weight: 500;
    letter-spacing: 0.01em;
    /* Crossfade when stageLabel changes (whim rotation: Thinking → Sussing
       → Spelunking …). Keyed via {#key stageLabel} in the template. */
    animation: stage-label-in 320ms ease-out;
  }
  @keyframes stage-label-in {
    from { opacity: 0; transform: translateY(-1px); }
    to   { opacity: 1; transform: translateY(0); }
  }
  @media (prefers-reduced-motion: reduce) {
    .stage-dot, .stage-label { animation: none; }
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

  /* Trailing activity row — the live "still working" cue that trails the last
     emitted block while streaming (prose composing, between code blocks, etc).
     Reuses the stage-strip's dot+label vocabulary at a smaller scale so the
     body never reads as dead mid-turn. Suppressed when the tail node already
     pulses (pending tool / active thinking) — see showTrailingActivity. Sits
     on the rail like any node via the bullet below. */
  .trailing-activity {
    position: relative;
    display: inline-flex; align-items: center; gap: 8px;
    margin-top: 0.5rem;
    color: var(--fg-muted);
    font-size: var(--fs-xs);
    min-height: 16px;
  }
  .trailing-activity::before {
    content: "";
    position: absolute;
    left: -19px; top: 4px;
    width: 8px; height: 8px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 6px color-mix(in oklab, var(--accent) 45%, transparent);
    animation: tl-bullet-pulse 1.6s ease-in-out infinite;
  }
  .ta-dots { display: inline-flex; gap: 4px; align-items: center; }
  .ta-dot {
    width: 4px; height: 4px;
    border-radius: 50%;
    background: var(--accent);
    animation: stage-bounce 1.1s ease-in-out infinite;
  }
  .ta-dot:nth-child(2) { animation-delay: 0.18s; }
  .ta-dot:nth-child(3) { animation-delay: 0.36s; }
  .ta-label {
    color: color-mix(in oklch, var(--fg-2) 85%, var(--accent));
    font-weight: 500;
    letter-spacing: 0.01em;
    animation: stage-label-in 320ms ease-out;
  }
  @media (prefers-reduced-motion: reduce) {
    .trailing-activity::before, .ta-dot, .ta-label { animation: none; }
  }

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
    opacity: 0.38;
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
  .tg-head {
    display: flex; align-items: center; gap: 6px;
    width: 100%;
    padding: 3px 8px;
    min-height: 22px;
    background: color-mix(in oklch, var(--bg-elev-1) 45%, transparent);
    border: 0;
    border-radius: 8px;
    color: var(--fg-2);
    font: inherit; font-size: 11px; text-align: left;
    cursor: pointer;
    transition: background 140ms ease-out, box-shadow 140ms ease-out, transform 140ms ease-out;
  }
  .tg-head:hover {
    background: color-mix(in oklch, var(--surface-hover) 80%, transparent);
    box-shadow: inset 2px 0 0 color-mix(in oklab, var(--accent) 55%, transparent);
    transform: translateX(1px);
  }
  .tl-toolgroup[data-status="pending"] .tg-head {
    box-shadow: inset 2px 0 0 color-mix(in oklab, var(--accent) 60%, transparent);
  }
  .tl-toolgroup[data-status="error"] .tg-head {
    box-shadow: inset 2px 0 0 color-mix(in oklab, var(--danger) 60%, transparent);
  }
  .tg-chev { display: inline-flex; color: var(--fg-faint); transition: transform 140ms ease-out; flex-shrink: 0; }
  .tg-chev.open { transform: rotate(90deg); }
  .tg-head :global(.tg-icon) { color: var(--accent); opacity: 0.85; flex-shrink: 0; }
  .tg-cap {
    font-weight: 500; color: var(--fg); font-size: 11px;
    flex: 0 1 auto; min-width: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .tl-toolgroup[data-status="error"] .tg-cap { color: oklch(0.85 0.10 22); }
  .tg-sep { color: var(--fg-faint); font-size: 10px; flex-shrink: 0; }
  .tg-sum {
    flex: 1; min-width: 0;
    color: var(--fg-muted); font-size: 10.5px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .tg-status { display: inline-flex; flex-shrink: 0; }
  .tl-toolgroup[data-status="pending"] .tg-status { color: var(--accent); }
  .tl-toolgroup[data-status="error"] .tg-status { color: var(--danger); }
  .tl-toolgroup[data-status="done"] .tg-status { color: var(--ok); }
  .tg-status :global(.tg-spin) { animation: tg-spin 1s linear infinite; }
  @keyframes tg-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }
  .tg-body {
    display: flex; flex-direction: column;
    gap: 4px;
    margin-top: 4px;
    padding-left: 6px;
  }
  /* No per-child rail bullet inside a group, so re-show each chip's own status
     icon (the timeline variant hides it, assuming the rail bullet carries it). */
  .tg-body :global(.chip[data-variant="timeline"] .chip-status) { display: inline-flex; }

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
  /* De-boxed user prose (mock `.ct-usertext`): plain text in the same column as
     Claude's prose, brighter ink, no bg/padding/border. */
  .bubble[data-role="user"] .text {
    padding: 0;
    background: transparent;
    border: 0;
    color: var(--fg);
    line-height: 1.6;
    align-self: flex-start;
    max-width: min(100%, 72ch);
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
  .ts-btn {
    display: inline-flex; align-items: center; gap: 6px;
    height: 28px; padding: 0 11px;
    background: transparent; border: 1px solid var(--ghost-border); border-radius: 8px;
    color: var(--fg-2); font: inherit; font-size: var(--fs-sm); font-weight: 600; cursor: pointer;
    transition: background 130ms var(--ease-soft), border-color 130ms var(--ease-soft), color 130ms var(--ease-soft);
  }
  .ts-btn:hover { background: var(--accent-soft); border-color: color-mix(in oklab, var(--accent) 45%, var(--ghost-border)); color: var(--fg); }
  .ts-btn :global(svg) { color: var(--accent); }
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
</style>
