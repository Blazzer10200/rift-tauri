<script lang="ts">
  type Props = { holder: string; tooltip?: string };
  let { holder, tooltip }: Props = $props();
</script>

<span class="lock" title={tooltip ?? `Locked by ${holder}`}>
  🔒 {holder}
</span>

<style>
  .lock {
    display: inline-block;
    margin-left: 8px;
    background: #15101E;
    color: #F0B95C;
    border: 1px solid #F0B95C;
    border-radius: 999px;
    padding: 0 7px;
    font-size: 10px;
    line-height: 16px;
    font-family: Consolas, monospace;
    vertical-align: middle;
  }
</style>
