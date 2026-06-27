<script lang="ts">
  import { Eye } from "lucide-svelte";
  import Markdown from "../Markdown.svelte";

  let {
    draft,
  }: {
    draft: string;
  } = $props();
</script>

<div class="preview-panel" role="region" aria-label="Message preview">
  <div class="preview-head">
    <Eye size={12} />
    <span class="preview-title">Preview</span>
    <span class="preview-sub">rendered Markdown</span>
  </div>
  <div class="preview-body"><Markdown text={draft} /></div>
</div>

<style>
  @keyframes slash-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .preview-panel {
    position: absolute;
    bottom: calc(100% + 8px);
    left: 0;
    width: 100%;
    box-sizing: border-box;
    background: color-mix(in oklch, var(--surface) 88%, transparent);
    backdrop-filter: blur(14px) saturate(135%);
    -webkit-backdrop-filter: blur(14px) saturate(135%);
    border: 1px solid var(--border-strong);
    border-radius: 14px;
    box-shadow:
      0 18px 44px -8px oklch(0 0 0 / 0.55),
      inset 0 1px 0 color-mix(in oklch, white 5%, transparent);
    padding: 12px;
    z-index: 10;
    animation: slash-in 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }
  .preview-head {
    display: flex; align-items: center; gap: 7px;
    margin-bottom: 8px;
    color: var(--fg-muted);
  }
  .preview-title { font-size: var(--fs-sm); font-weight: 600; color: var(--fg); }
  .preview-sub {
    font-size: 10px; font-weight: 500;
    color: var(--fg-faint);
    margin-left: auto;
    letter-spacing: 0.02em;
  }
  .preview-body {
    font-size: var(--fs-md);
    line-height: 1.55;
    color: var(--fg);
    max-height: 280px;
    overflow-y: auto;
    padding: 2px 4px;
  }
</style>
