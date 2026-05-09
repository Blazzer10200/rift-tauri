<script lang="ts">
  import { Download, X, RefreshCw, AlertTriangle, CheckCircle2, Info } from "lucide-svelte";
  import { updates } from "../../state/updates.svelte";

  type Variant = "ok" | "warn" | "danger" | "info";

  const variant = $derived<Variant>(
    updates.state === "available" ? "warn" :
    updates.state === "applying"  ? "warn" :
    updates.state === "uptodate"  ? "ok"   :
    updates.state === "error"     ? "danger" :
                                    "info"
  );

  const subTitle = $derived(
    updates.state === "available" ? "Update available" :
    updates.state === "applying"  ? "Installing…" :
    updates.state === "uptodate"  ? "Up to date" :
    updates.state === "error"     ? "Check failed" :
                                    "Checking…"
  );

  function iconFor(v: Variant) {
    switch (v) {
      case "ok":     return CheckCircle2;
      case "warn":   return Download;
      case "danger": return AlertTriangle;
      case "info":   return Info;
    }
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) updates.close();
  }
  function onKey(e: KeyboardEvent) {
    if (!updates.dialogOpen) return;
    if (e.key === "Escape") updates.close();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if updates.dialogOpen}
  {@const Ico = iconFor(variant)}
  <div class="dialog-overlay" onclick={onBackdrop} role="presentation">
    <div class="dialog-shell" style="width: 480px;" role="dialog" aria-modal="true" aria-label="Updates">
      <div class="dialog-head">
        <div class="dialog-icon {variant}">
          {#if updates.state === "checking"}
            <RefreshCw size={14} class="spin"/>
          {:else}
            <Ico size={14}/>
          {/if}
        </div>
        <div>
          <div class="dialog-title">Updates</div>
          <div class="dialog-sub">{subTitle}</div>
        </div>
        <button class="dialog-close" type="button" onclick={() => updates.close()} aria-label="Close">
          <X size={14}/>
        </button>
      </div>

      <div class="dialog-body">
        {#if updates.state === "checking"}
          <p class="lead">Talking to GitHub releases…</p>

        {:else if (updates.state === "available" || updates.state === "applying") && updates.info}
          <p class="lead">
            Rift <span class="mono">{updates.info.version}</span> is ready to install.
            You're currently on <span class="mono">{updates.currentVersion}</span>.
          </p>

          <div class="version-card">
            <div class="version-row">
              <span class="vk">Latest</span>
              <span class="vv mono">{updates.info.version}</span>
            </div>
            <div class="version-row">
              <span class="vk">Release</span>
              <span class="vv mono dim">{updates.info.releaseName}</span>
            </div>
          </div>

          {#if updates.state === "applying"}
            <p class="hint">Downloading and swapping the binary — Rift will close and relaunch.</p>
          {:else}
            <p class="hint">Installing closes Rift, swaps the binary, and relaunches. In-flight syncs are paused first.</p>
          {/if}

        {:else if updates.state === "uptodate"}
          <p class="lead">
            You're on <span class="mono">{updates.currentVersion}</span> — the latest published release.
            The updater re-checks GitHub on every launch.
          </p>

        {:else if updates.state === "error"}
          <p class="error mono"><AlertTriangle size={11}/> {updates.error}</p>
          <p class="hint">Usually means GitHub is unreachable, the feed isn't published yet, or rate-limit applied. Try again shortly.</p>

        {:else}
          <p class="lead">No update info yet. Click "Check again" to query GitHub.</p>
        {/if}
      </div>

      <div class="dialog-foot">
        <button class="btn ghost" type="button" onclick={() => updates.close()}>
          {updates.state === "available" ? "Later" : "Close"}
        </button>
        <div class="dialog-foot-spacer"></div>
        <button
          class="btn"
          type="button"
          onclick={() => updates.refresh()}
          disabled={updates.state === "checking"}
        >
          <RefreshCw size={11} class={updates.state === "checking" ? "spin" : ""}/>
          {updates.state === "checking" ? "Checking…" : "Check again"}
        </button>
        {#if updates.state === "available" || updates.state === "applying"}
          <button
            class="btn primary"
            type="button"
            onclick={() => updates.apply()}
            disabled={updates.state === "applying"}
          >
            <Download size={11} class={updates.state === "applying" ? "spin" : ""}/>
            {updates.state === "applying" ? "Installing…" : "Install & restart"}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .lead { color: var(--fg-2); font-size: var(--fs-sm); line-height: 1.5; margin: 0 0 12px; }
  .hint { font-size: var(--fs-xs); color: var(--fg-faint); margin: 8px 0 0; }
  .dim { color: var(--fg-faint); }
  .error {
    display: inline-flex; align-items: center; gap: 6px;
    color: var(--danger); font-size: var(--fs-xs);
    margin: 0 0 8px;
  }

  .version-card {
    margin-bottom: 12px;
    padding: 10px 12px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    display: flex; flex-direction: column; gap: 6px;
  }
  .version-row {
    display: flex; justify-content: space-between; align-items: baseline;
    gap: 12px;
  }
  .vk { font-size: var(--fs-xs); color: var(--fg-faint); }
  .vv { font-size: var(--fs-sm); color: var(--fg); }
</style>
