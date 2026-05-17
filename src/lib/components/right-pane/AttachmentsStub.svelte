<script lang="ts">
  import type { PanelIcon } from "./index";
  let { title, icon: Icon }: { title: string; icon: PanelIcon } = $props();
</script>

<div class="stub">
  <span class="stub-icon"><Icon size={14}/></span>
  <span>{title} panel — coming in Phase B</span>
</div>

<style>
  .stub {
    display: flex; align-items: center; gap: 8px;
    padding: 14px 12px;
    color: var(--fg-muted);
    font-size: var(--fs-sm);
  }
  .stub-icon {
    display: inline-flex; align-items: center; justify-content: center;
    width: 18px; height: 18px;
    color: var(--fg-faint);
  }
</style>
