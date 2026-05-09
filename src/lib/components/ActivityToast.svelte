<script lang="ts">
  import { connection } from "../state/connection.svelte";

  let visible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | null = null;

  $effect(() => {
    if (connection.lastActivity) {
      visible = true;
      if (hideTimer) clearTimeout(hideTimer);
      hideTimer = setTimeout(() => {
        visible = false;
      }, 4500);
    }
  });

  const kindColor: Record<string, string> = {
    sync: "#8B6BE6",
    pull: "#4ADE80",
    delete: "#FF5C6B",
    conflict: "#FF5C6B",
    conflict_resolved: "#4ADE80",
    drift: "#F0B95C",
    bridge: "#8B6BE6",
    block: "#F0B95C",
    error: "#FF5C6B",
    system: "#7A7A85",
  };
</script>

{#if visible && connection.lastActivity}
  {@const a = connection.lastActivity}
  <div class="toast">
    <span class="kind" style="background: {kindColor[a.kind] ?? '#7A7A85'}">
      {a.kind}
    </span>
    <span class="resource">{a.resource}</span>
    <span class="file">{a.file}</span>
    <span class="action">{a.action}</span>
  </div>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: 16px;
    left: 50%;
    transform: translateX(-50%);
    background: #17171C;
    border: 1px solid #26262E;
    border-radius: 4px;
    padding: 8px 14px;
    display: flex; align-items: center; gap: 10px;
    font-size: 12px;
    color: #E8E8EE;
    box-shadow: 0 6px 20px rgba(0,0,0,0.5);
    max-width: 80vw;
    z-index: 50;
  }
  .kind {
    color: white;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 2px 6px;
    border-radius: 2px;
    letter-spacing: 0.4px;
  }
  .resource { color: #8B6BE6; font-family: Consolas, monospace; }
  .file { color: #E8E8EE; font-family: Consolas, monospace; overflow: hidden; text-overflow: ellipsis; }
  .action { color: #7A7A85; font-style: italic; }
</style>
