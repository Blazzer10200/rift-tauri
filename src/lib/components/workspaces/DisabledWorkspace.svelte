<script lang="ts">
  import type { WorkspaceIcon } from "./index";

  type Props = {
    title: string;
    icon: WorkspaceIcon;
    message?: string;
  };
  let { title, icon: Icon, message = "Coming soon — Phase B." }: Props = $props();
</script>

<div class="disabled-ws" data-workspace-disabled>
  <div class="card">
    <span class="ico"><Icon size={28}/></span>
    <h2>{title}</h2>
    <p>{message}</p>
  </div>
</div>

<style>
  .disabled-ws {
    flex: 1;
    min-height: 0; min-width: 0;
    display: flex; align-items: center; justify-content: center;
    padding: 32px;
    background: var(--bg);
    color: var(--fg-muted);
  }
  .card {
    max-width: 360px;
    padding: 28px 32px;
    background: var(--bg-elev-1);
    border: 1px dashed var(--border);
    border-radius: 12px;
    text-align: center;
  }
  .ico { display: inline-flex; color: var(--fg-faint); margin-bottom: 8px; }
  h2 { margin: 6px 0 8px; font-size: 16px; font-weight: 600; color: var(--fg); }
  p { margin: 0; font-size: var(--fs-sm); line-height: 1.5; color: var(--fg-muted); }
</style>
