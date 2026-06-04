<script lang="ts">
  // First-run flow. Pure-assistant build: a single Claude-auth step. The full
  // multi-step SSH/server/sync walkthrough was removed when Rift dropped its
  // SFTP/sync half. Mounted by AppShell when the first-run gate is open; onDone
  // fires on finish OR skip and persists the dismissal.
  import "$lib/styles/onboarding.css";
  import ClaudeAuth from "./ClaudeAuth.svelte";
  import RiftLogo from "$lib/components/shell/RiftLogo.svelte";
  import { Check } from "lucide-svelte";

  type Props = { onDone: () => void };
  const { onDone }: Props = $props();
</script>

<div class="ob-overlay">
  <div class="ob-main ob-enter ob-solo">
    <div class="ob-brand">
      <span class="ob-brand-mark"><RiftLogo size={30} /></span>
      <span class="ob-brand-name">Rift</span>
      <span class="ob-brand-tag">Setup</span>
    </div>

    <div class="ob-body">
      <div class="ob-step-wrap">
        <ClaudeAuth />
      </div>
    </div>

    <footer class="ob-foot">
      <button type="button" class="ob-skip-link" onclick={onDone}>Skip — configure later</button>
      <button type="button" class="ob-btn primary" onclick={onDone}>
        <Check size={15} /> Finish setup
      </button>
    </footer>
  </div>
</div>

<style>
  .ob-solo {
    max-width: 560px;
    margin: 0 auto;
    width: 100%;
  }
  .ob-solo .ob-brand {
    padding: 22px 28px 0;
  }
  .ob-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
</style>
