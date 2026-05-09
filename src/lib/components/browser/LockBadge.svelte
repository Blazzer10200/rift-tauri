<script lang="ts">
  import { Lock } from "lucide-svelte";
  type Props = { holder: string; tooltip?: string };
  let { holder, tooltip }: Props = $props();

  const initials = $derived.by(() => {
    const at = holder.indexOf("@");
    const u = at === -1 ? holder : holder.slice(0, at);
    return u.slice(0, 2).toUpperCase();
  });
</script>

<span class="lock" title={tooltip ?? `Locked by ${holder}`}>
  <Lock size={10}/>
  <span class="initials mono">{initials}</span>
</span>

<style>
  .lock {
    display: inline-flex; align-items: center; gap: 4px;
    margin-left: 8px;
    padding: 0 6px;
    height: 16px;
    border-radius: 999px;
    background: var(--warn-soft);
    color: var(--warn);
    border: 1px solid color-mix(in oklch, var(--warn) 30%, transparent);
    font-size: 10px;
    line-height: 1;
    vertical-align: middle;
  }
  .initials { font-weight: 600; }
</style>
