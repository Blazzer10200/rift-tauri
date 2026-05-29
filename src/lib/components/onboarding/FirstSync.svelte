<script lang="ts">
  import { RefreshCw, ChevronLeft, CheckCircle2, Info } from "lucide-svelte";

  type Props = {
    serverKey: string | null;
    onDone: () => void;
    onBack?: () => void;
  };
  const { onDone, onBack }: Props = $props();
</script>

<div class="onboarding-step">
  <header class="onboarding-head">
    <div class="onboarding-icon"><RefreshCw size={28} /></div>
    <h2 class="onboarding-title">First sync</h2>
    <p class="onboarding-sub">
      Almost done. Here's what happens next when you open the Sync page.
    </p>
  </header>

  <ol class="walkthrough">
    <li>
      Rift connects via SFTP and lists your <code>remote root</code> contents.
      You'll see a confirm dialog if the host fingerprint isn't trusted yet
      (TOFU pattern — accept once, remembered after).
    </li>
    <li>
      The drift scanner compares remote files against your local folder. New
      files land in <strong>To pull</strong>; locally-edited files in
      <strong>To push</strong>; missing-on-one-side files in
      <strong>To delete</strong>.
    </li>
    <li>
      Nothing moves until you click <strong>Apply</strong>. Rift will surface a
      summary ("12 to pull · 3 to push · 0 to delete") before any I/O so you
      can sanity-check before committing.
    </li>
    <li>
      After the first apply, file changes auto-sync as you edit. You can
      toggle this off per-resource in the Sync page's per-folder header.
    </li>
  </ol>

  <p class="hint">
    <Info size={13} />
    Stuck? Check the project README for the full walkthrough,
    screenshots, and troubleshooting.
  </p>

  <footer class="onboarding-foot">
    <button class="btn ghost sm" type="button" onclick={() => onBack?.()}>
      <ChevronLeft size={14} /> Back
    </button>
    <button class="btn primary" type="button" onclick={onDone}>
      <CheckCircle2 size={14} /> Finish setup
    </button>
  </footer>
</div>

<style>
  .onboarding-step {
    display: flex; flex-direction: column; gap: 16px;
    max-width: 560px; margin: 56px auto; padding: 28px;
    background: var(--bg-elev-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .onboarding-head { display: flex; flex-direction: column; gap: 6px; }
  .onboarding-icon {
    width: 48px; height: 48px;
    display: flex; align-items: center; justify-content: center;
    border-radius: var(--radius-md);
    background: color-mix(in oklch, var(--accent) 14%, transparent);
    color: var(--accent);
  }
  .onboarding-title { font-size: 18px; font-weight: 600; color: var(--fg); margin: 0; }
  .onboarding-sub { font-size: 13px; color: var(--fg-muted); line-height: 1.5; margin: 0; }
  .walkthrough {
    display: flex; flex-direction: column; gap: 12px;
    padding-left: 20px; margin: 0;
    font-size: 13px; line-height: 1.55;
    color: var(--fg-2);
  }
  .walkthrough strong { color: var(--fg); }
  .walkthrough code { font-family: var(--font-mono); font-size: 12px; padding: 1px 5px; background: var(--bg-elev-2); border-radius: var(--radius-xs); }
  .hint { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--fg-muted); margin: 0; }
  .onboarding-foot {
    display: flex; justify-content: space-between; align-items: center;
    margin-top: 8px; padding-top: 16px;
    border-top: 1px solid var(--border);
  }
</style>
