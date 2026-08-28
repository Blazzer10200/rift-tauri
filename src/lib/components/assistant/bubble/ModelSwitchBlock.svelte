<script lang="ts">
  import { ArrowRightLeft } from "@lucide/svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { shortModel } from "./helpers";
  import { MODEL_OPTIONS } from "../composer/modelMatrix";
  import type { Block } from "../../../state/assistant.svelte";

  let { block }: { block: Extract<Block, { type: "modelSwitch" }> } = $props();

  // Picker aliases ("sonnet") aren't parseable by shortModel, and gated rows
  // (Fable/Haiku) can be absent from MODEL_OPTIONS — resolve via the picker
  // table first, then the alias→id map, then shortModel on the raw id.
  const ALIAS_ID: Record<string, string> = {
    sonnet: "claude-sonnet-5",
    opus: "claude-opus-5",
    haiku: "claude-haiku-4-5",
  };
  function label(id: string): string {
    const m = MODEL_OPTIONS.find((o) => o.id === id);
    return m ? `${m.label} ${m.version}` : shortModel(ALIAS_ID[id] ?? id);
  }
</script>

<div class="switch" data-role="system">
  <span class="switch-line" aria-hidden="true"></span>
  <span
    class="switch-pill"
    use:tooltip={"The model was switched mid-chat — turns from here on run on the new model."}
  >
    <ArrowRightLeft size={11} />
    <span>Model switched</span>
    <span class="switch-meta mono">{label(block.from)} → {label(block.to)}</span>
  </span>
  <span class="switch-line" aria-hidden="true"></span>
</div>

<style>
  .switch {
    width: 100%;
    padding: 8px 4px;
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 10px;
  }
  .switch-line {
    flex: 1;
    height: 1px;
    background: linear-gradient(to right,
      transparent,
      color-mix(in oklch, var(--border) 80%, transparent),
      color-mix(in oklab, var(--accent) 30%, transparent) 50%,
      color-mix(in oklch, var(--border) 80%, transparent),
      transparent);
  }
  .switch-pill {
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
  }
  .switch-pill :global(svg) { color: var(--accent); }
  .switch-meta {
    opacity: 0.55;
    font-size: 10px;
  }
</style>
