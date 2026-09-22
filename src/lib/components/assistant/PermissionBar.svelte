<script lang="ts">
  // Inline Allow / Deny bar for a gated tool call awaiting approval. Shown
  // below the tool block (ToolChip or EditDiff) when the CLI emitted a
  // `can_use_tool` ask in a prompting permission mode (default / acceptEdits /
  // plan). Pairs to the block by tool_use_id; renders nothing until the ask
  // lands and disappears once the user decides (or the turn ends).
  import { ShieldQuestion, Check, X, Loader2 } from "@lucide/svelte";
  import { assistant } from "../../state/assistant.svelte";

  let { toolUseId, toolName }: { toolUseId: string; toolName: string } = $props();

  const prompt = $derived(assistant.permissionPromptFor(toolUseId));
  let submitting = $state(false);

  async function decide(allow: boolean) {
    if (submitting) return;
    submitting = true;
    try {
      await assistant.submitPermissionDecision(toolUseId, allow);
    } catch (e) {
      console.warn("submitPermissionDecision failed", e);
      submitting = false; // let the user retry
    }
  }

  function label(name: string): string {
    return name.replace(/^mcp__rift__/, "");
  }
</script>

<!-- `kind:"plan"` asks belong to the plan card's action bar (StreamExitPlan) —
     never double-render the generic bar for them. -->
{#if prompt && prompt.kind !== "plan"}
  <!-- role="group" (not alertdialog): this prompt is intentionally NON-modal —
       it never traps focus or steals it from scrollback (CC-UI ref §8). An
       alertdialog role would promise focus containment we deliberately don't do.
       Tone: `.tile.is-ask` (app.css "stream tiles" — the same prompt variant as
       ask_user/plan) — this bar gates an arbitrary tool call, not specifically
       a cost or a destructive action, so it takes the accent needs-the-user
       treatment, not DESIGN §8's amber "costs money" reservation. -->
  <div class="tile is-ask perm-bar" role="group" aria-label="Tool permission request">
    <div class="tile-head perm-row">
      <span class="perm-icon"><ShieldQuestion size={14} /></span>
      <span class="perm-text">
        Allow <span class="perm-tool">{label(toolName || prompt.toolName)}</span>?
      </span>
      <div class="perm-actions">
        <button
          type="button"
          class="perm-btn deny"
          disabled={submitting}
          onclick={() => decide(false)}
        >
          <X size={12} /> Deny
        </button>
        <button
          type="button"
          class="perm-btn allow"
          disabled={submitting}
          onclick={() => decide(true)}
        >
          {#if submitting}<Loader2 size={12} class="perm-spin" />{:else}<Check size={12} />{/if} Allow
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Global `tile is-ask` (app.css) supplies border/radius/fill/hover + the
     accent stationary status edge — this file owns only the compact bar
     layout (fit-content, left-aligned under the tool block it answers) and
     the Allow/Deny button pair. */
  .perm-bar {
    align-self: flex-start;
    width: fit-content;
    max-width: min(100%, 78ch);
    margin: 3px 0 5px;
    animation: perm-enter var(--dur-base) cubic-bezier(0.22, 1, 0.36, 1);
  }
  .perm-row {
    gap: 9px;
  }
  .perm-icon {
    display: inline-flex;
    color: var(--accent);
    flex-shrink: 0;
  }
  .perm-text {
    font-size: var(--fs-sm);
    color: var(--fg);
    flex: 1;
    min-width: 0;
  }
  .perm-tool {
    font-family: var(--font-mono, monospace);
    font-weight: 600;
    color: var(--fg);
  }
  .perm-actions {
    display: inline-flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .perm-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 11px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elev-1);
    color: var(--fg-2);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: 600;
    cursor: pointer;
    transition: background var(--dur-fast) ease-out, border-color var(--dur-fast) ease-out, color var(--dur-fast) ease-out;
  }
  .perm-btn:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg); }
  .perm-btn:disabled { opacity: 0.5; cursor: default; }
  .perm-btn.allow {
    background: color-mix(in oklch, var(--ok) 20%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--ok) 50%, var(--border));
    color: color-mix(in oklch, var(--ok) 80%, var(--fg));
  }
  .perm-btn.allow:hover:not(:disabled) {
    background: color-mix(in oklch, var(--ok) 30%, var(--bg-elev-1));
    border-color: color-mix(in oklch, var(--ok) 65%, var(--border));
  }
  .perm-btn.deny:hover:not(:disabled) {
    border-color: color-mix(in oklab, var(--danger) 45%, var(--border));
    color: var(--danger);
  }
  .perm-btn :global(.perm-spin) { animation: perm-spin 1s linear infinite; }
  @keyframes perm-spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }
  @keyframes perm-enter {
    from { opacity: 0; transform: translateY(-3px); }
    to { opacity: 1; transform: translateY(0); }
  }
  /* Honour reduced-motion: drop the entrance slide (decorative) but keep the
     spinner (it's a state indicator, not decoration — CC-UI ref §2/§11). */
  @media (prefers-reduced-motion: reduce) {
    .perm-bar { animation: none; }
    .perm-btn { transition: none; }
  }
</style>
