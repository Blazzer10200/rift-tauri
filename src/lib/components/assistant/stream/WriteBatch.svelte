<script lang="ts">
  import { VERB_PAST, VERB_ING, type StreamTool } from "./streamModel";
  let { tools }: { tools: StreamTool[] } = $props();
</script>

<div class="wbatch">
  {#each tools as t (t.id)}
    <div class="wbrow {t.status === 'pending' ? 'active' : ''}">
      <span class="wb-verb">{t.status === "pending" ? VERB_ING[t.kind] : VERB_PAST[t.kind]}</span>
      <span class="wb-label"><span class="wb-name">{t.cap}</span></span>
      {#if t.add != null || t.del != null}
        <span class="wb-diff">
          {#if t.add != null}<span class="wb-add">+{t.add}</span>{/if}
          {#if t.del != null}<span class="wb-del {t.del > 0 ? 'real' : ''}">−{t.del}</span>{/if}
        </span>
      {/if}
      <span class="wb-dot {t.status === 'pending' ? 'live' : ''}"></span>
    </div>
  {/each}
</div>
