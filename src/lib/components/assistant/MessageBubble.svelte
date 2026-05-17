<script lang="ts">
  import { User, Sparkles, Copy, Check, Brain, ChevronDown } from "lucide-svelte";
  import { onDestroy } from "svelte";
  import type { ChatMessage, ThinkingBlock } from "../../state/assistant.svelte";
  import Markdown from "./Markdown.svelte";

  let { message, streaming = false }: { message: ChatMessage; streaming?: boolean } = $props();

  let copied = $state(false);
  let expandedThinking = $state(new Set<number>());
  // Live tick so `active` thinking blocks show real-time elapsed seconds.
  // Only runs while at least one active block is present in this bubble.
  let tickNow = $state(Date.now());
  const hasActiveThinking = $derived(
    streaming && message.blocks.some((b) => b.type === "thinking" && b.status === "active"),
  );
  let tickHandle: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    if (hasActiveThinking && !tickHandle) {
      tickHandle = setInterval(() => (tickNow = Date.now()), 250);
    } else if (!hasActiveThinking && tickHandle) {
      clearInterval(tickHandle);
      tickHandle = null;
    }
  });
  onDestroy(() => {
    if (tickHandle) clearInterval(tickHandle);
  });

  const plainText = $derived(
    message.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim(),
  );

  const hasContent = $derived(message.blocks.length > 0);
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

    <div class="content">
      {#each message.blocks as b, bi (bi)}
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
        {/if}
      {/each}

      {#if !isUser && streaming && !hasContent}
        <div class="thinking">
          <span class="d"></span><span class="d"></span><span class="d"></span>
        </div>
      {/if}
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
    gap: 4px;
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

  .thinking {
    display: flex; gap: 4px;
    padding: 4px 0;
  }
  .thinking .d {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--fg-subtle);
    animation: pulse 1.1s ease-in-out infinite;
  }
  .thinking .d:nth-child(2) { animation-delay: 0.15s; }
  .thinking .d:nth-child(3) { animation-delay: 0.3s; }
  @keyframes pulse {
    0%, 60%, 100% { opacity: 0.3; transform: scale(0.85); }
    30% { opacity: 1; transform: scale(1); }
  }

  /* Streaming-only chrome — blinking tail caret + soft fade on the last line
     so newly-arriving text looks like it's materializing in. Mask is applied
     to the whole content column; once a line is no longer the last one it
     pops to full opacity (the "reveal" feel). */
  .bubble[data-streaming="true"] .content {
    -webkit-mask-image: linear-gradient(180deg, #000 0, #000 calc(100% - 1.1em), rgba(0, 0, 0, 0.4) 100%);
    mask-image: linear-gradient(180deg, #000 0, #000 calc(100% - 1.1em), rgba(0, 0, 0, 0.4) 100%);
  }
  .bubble[data-streaming="true"] .content > .text:last-of-type :global(p:last-of-type)::after,
  .bubble[data-streaming="true"] .content > .text:last-of-type :global(li:last-child)::after {
    content: "▍";
    display: inline-block;
    margin-left: 2px;
    font-weight: 200;
    opacity: 0.55;
    color: currentColor;
    vertical-align: -1px;
    animation: caret-blink 1s ease-in-out infinite;
  }
  @keyframes caret-blink {
    0%, 55% { opacity: 0.55; }
    78% { opacity: 0; }
    100% { opacity: 0.55; }
  }
</style>
