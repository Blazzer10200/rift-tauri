<script lang="ts">
  import { User, Sparkles, Copy, Check } from "lucide-svelte";
  import type { ChatMessage } from "../../state/assistant.svelte";
  import Markdown from "./Markdown.svelte";

  let { message, streaming = false }: { message: ChatMessage; streaming?: boolean } = $props();

  let copied = $state(false);

  const plainText = $derived(
    message.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("")
      .trim(),
  );

  const hasContent = $derived(message.blocks.length > 0);
  const isUser = $derived(message.role === "user");

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
</script>

<div class="bubble" data-role={message.role}>
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
    white-space: pre-wrap;
    word-wrap: break-word;
    font-size: var(--fs-md);
    line-height: 1.55;
    color: var(--fg);
  }
  .bubble[data-role="user"] .text {
    padding: 8px 12px;
    background: var(--accent-soft);
    border: 1px solid color-mix(in oklch, var(--accent) 22%, transparent);
    border-radius: 10px;
    color: var(--fg);
    align-self: flex-start;
    max-width: min(100%, 78ch);
    width: fit-content;
  }
  .bubble[data-role="assistant"] .text { max-width: 78ch; }

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
</style>
