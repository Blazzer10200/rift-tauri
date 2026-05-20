<script lang="ts">
  import {
    Download, X, RefreshCw, AlertTriangle, CheckCircle2,
    Sparkles, ArrowRight, ExternalLink, Rocket, Clock,
  } from "lucide-svelte";
  import { fade, fly } from "svelte/transition";
  import { updates } from "../../state/updates.svelte";

  type Variant = "accent" | "ok" | "warn" | "danger" | "info";

  const variant = $derived<Variant>(
    updates.state === "available"   ? "accent" :
    updates.state === "downloading" ? "info"   :
    updates.state === "ready"       ? "ok"     :
    updates.state === "applying"    ? "info"   :
    updates.state === "uptodate"    ? "ok"     :
    updates.state === "error"       ? "danger" :
                                      "info"
  );

  const subTitle = $derived(
    updates.state === "available"   ? "A new release is ready" :
    updates.state === "downloading" ? "Downloading update" :
    updates.state === "ready"       ? "Update ready to install" :
    updates.state === "applying"    ? "Applying update" :
    updates.state === "uptodate"    ? "You're up to date" :
    updates.state === "error"       ? "Update check failed" :
                                      "Checking for updates"
  );

  function iconFor(v: Variant) {
    switch (v) {
      case "ok":     return CheckCircle2;
      case "accent": return Sparkles;
      case "danger": return AlertTriangle;
      case "info":   return Download;
      case "warn":   return Download;
    }
  }

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget) updates.close();
  }
  function onKey(e: KeyboardEvent) {
    if (!updates.dialogOpen) return;
    if (e.key === "Escape") {
      // Don't let Escape kill a download mid-flight.
      if (updates.state === "downloading" || updates.state === "applying") return;
      updates.close();
    }
  }

  // Render the markdown body as plain text w/ light formatting. Avoids
  // pulling a markdown lib for a release-notes block — we control the source
  // (our own CHANGELOG snippet) so HTML injection isn't a concern, but we
  // still escape everything by mounting as text nodes.
  function notesLines(md: string): { kind: "h" | "li" | "p" | "blank"; text: string }[] {
    if (!md) return [];
    return md.split(/\r?\n/).slice(0, 200).map((raw) => {
      const line = raw.trimEnd();
      if (!line.trim()) return { kind: "blank" as const, text: "" };
      if (/^#{1,6}\s+/.test(line)) return { kind: "h" as const, text: line.replace(/^#{1,6}\s+/, "") };
      if (/^[-*+]\s+/.test(line))  return { kind: "li" as const, text: line.replace(/^[-*+]\s+/, "") };
      return { kind: "p" as const, text: line };
    });
  }

  const notes = $derived(notesLines(updates.info?.notesMarkdown ?? ""));

  async function openReleasePage() {
    const url = updates.info?.releaseUrl;
    if (!url) return;
    try {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
    } catch (e) {
      console.warn("openUrl failed", e);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if updates.dialogOpen}
  {@const Ico = iconFor(variant)}
  <div class="dialog-overlay" onclick={onBackdrop} role="presentation" transition:fade={{ duration: 120 }}>
    <div
      class="upd-shell"
      role="dialog"
      aria-modal="true"
      aria-label="Updates"
      transition:fly={{ y: 8, duration: 180 }}
    >
      <!-- Gradient header -->
      <div class="upd-head" data-variant={variant}>
        <div class="head-glow"></div>
        <div class="head-row">
          <div class="head-icon" data-variant={variant}>
            {#if updates.state === "checking" || updates.state === "downloading" || updates.state === "applying"}
              <RefreshCw size={16} class="spin"/>
            {:else}
              <Ico size={16}/>
            {/if}
          </div>
          <div class="head-text">
            <div class="head-title">Rift Updates</div>
            <div class="head-sub">{subTitle}</div>
          </div>
          <button
            class="head-close"
            type="button"
            onclick={() => updates.close()}
            aria-label="Close"
            disabled={updates.state === "downloading" || updates.state === "applying"}
          >
            <X size={14}/>
          </button>
        </div>

        <!-- Version diff strip (only when we know the target) -->
        {#if updates.info && (updates.state === "available" || updates.state === "downloading" || updates.state === "ready" || updates.state === "applying")}
          <div class="head-diff">
            <span class="diff-chip current mono">v{updates.currentVersion}</span>
            <ArrowRight size={12} class="diff-arrow"/>
            <span class="diff-chip target mono">v{updates.info.version}</span>
            {#if updates.sizeLabel}
              <span class="diff-meta">· {updates.sizeLabel}</span>
            {/if}
            {#if updates.publishedLabel}
              <span class="diff-meta">· {updates.publishedLabel}</span>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Body -->
      <div class="upd-body">
        {#if updates.state === "checking"}
          <p class="lead">Talking to GitHub releases…</p>

        {:else if updates.state === "available" || updates.state === "downloading" || updates.state === "ready" || updates.state === "applying"}
          {#if updates.state === "downloading" || updates.state === "applying"}
            <div class="progress-card">
              <div class="progress-head">
                <span class="progress-label">
                  {updates.state === "applying" ? "Applying…" : "Downloading…"}
                </span>
                <span class="progress-pct mono">{updates.progress}%</span>
              </div>
              <div class="progress-track">
                <div class="progress-fill" style="width: {updates.progress}%"></div>
                <div class="progress-shimmer"></div>
              </div>
              <div class="progress-foot">
                {#if updates.state === "applying"}
                  Closing Rift, swapping the binary, relaunching.
                {:else}
                  {updates.sizeLabel
                    ? `${(updates.info?.sizeBytes ? (updates.info.sizeBytes * updates.progress / 100 / (1024*1024)).toFixed(1) : "0.0")} of ${updates.sizeLabel}`
                    : "Streaming from GitHub…"}
                {/if}
              </div>
            </div>
          {:else if updates.state === "ready"}
            <div class="ready-card">
              <Rocket size={18}/>
              <div class="ready-text">
                <div class="ready-title">Update downloaded — ready to install.</div>
                <div class="ready-sub">Rift will close, swap the binary, and relaunch into v{updates.info?.version}.</div>
              </div>
            </div>
          {/if}

          {#if updates.info?.notesMarkdown && notes.length > 0}
            <div class="notes-card">
              <div class="notes-head">
                <Sparkles size={11}/>
                <span>What's new</span>
              </div>
              <div class="notes-body">
                {#each notes as ln, i (i)}
                  {#if ln.kind === "h"}
                    <div class="notes-h">{ln.text}</div>
                  {:else if ln.kind === "li"}
                    <div class="notes-li"><span class="bullet">•</span><span>{ln.text}</span></div>
                  {:else if ln.kind === "blank"}
                    <div class="notes-spacer"></div>
                  {:else}
                    <div class="notes-p">{ln.text}</div>
                  {/if}
                {/each}
              </div>
            </div>
          {/if}

          {#if updates.info?.releaseUrl}
            <button class="link-row" type="button" onclick={openReleasePage}>
              <ExternalLink size={11}/>
              <span>View release on GitHub</span>
            </button>
          {/if}

        {:else if updates.state === "uptodate"}
          <div class="ok-card">
            <CheckCircle2 size={20}/>
            <div>
              <div class="ok-title">You're on the latest release.</div>
              <div class="ok-sub">Running v{updates.currentVersion}. We re-check GitHub on every launch.</div>
            </div>
          </div>

        {:else if updates.state === "error"}
          <div class="err-card">
            <AlertTriangle size={16}/>
            <div class="err-text">
              <div class="err-title">Couldn't reach the update feed.</div>
              <div class="err-detail mono">{updates.error}</div>
              <div class="err-hint">GitHub unreachable, feed not published, or rate-limited. Try again shortly.</div>
            </div>
          </div>

        {:else}
          <p class="lead">No update info yet. Click "Check now" to query GitHub.</p>
        {/if}
      </div>

      <!-- Footer adapts per state -->
      <div class="upd-foot">
        {#if updates.state === "available"}
          <button class="btn ghost" type="button" onclick={() => updates.snooze()}>
            <Clock size={11}/> Remind me later
          </button>
          <div class="foot-spacer"></div>
          <button class="btn primary" type="button" onclick={() => updates.download()}>
            <Download size={11}/> Download update
          </button>

        {:else if updates.state === "downloading"}
          <span class="foot-status">Hang tight — keep Rift open.</span>
          <div class="foot-spacer"></div>
          <button class="btn" type="button" onclick={() => updates.close()}>Hide</button>

        {:else if updates.state === "ready"}
          <button class="btn ghost" type="button" onclick={() => updates.close()}>Apply on next launch</button>
          <div class="foot-spacer"></div>
          <button class="btn primary glow" type="button" onclick={() => updates.applyNow()}>
            <Rocket size={11}/> Restart & install
          </button>

        {:else if updates.state === "applying"}
          <span class="foot-status">Rift is closing…</span>
          <div class="foot-spacer"></div>

        {:else}
          <button class="btn ghost" type="button" onclick={() => updates.close()}>Close</button>
          <div class="foot-spacer"></div>
          <button
            class="btn"
            type="button"
            onclick={() => updates.refresh()}
            disabled={updates.state === "checking"}
          >
            <RefreshCw size={11} class={updates.state === "checking" ? "spin" : ""}/>
            {updates.state === "checking" ? "Checking…" : "Check now"}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .upd-shell {
    width: 520px;
    max-width: calc(100vw - 32px);
    max-height: calc(100vh - 48px);
    background: var(--bg-elev-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg, 12px);
    box-shadow:
      0 24px 64px -16px rgba(0, 0, 0, 0.55),
      0 8px 24px -8px rgba(0, 0, 0, 0.4),
      0 0 0 1px rgba(255, 255, 255, 0.02);
    overflow: hidden;
    display: flex; flex-direction: column;
    color: var(--fg);
  }

  /* ── Header ─────────────────────────────────────────────── */
  .upd-head {
    position: relative;
    padding: 16px 18px 12px;
    background:
      linear-gradient(180deg,
        color-mix(in oklch, var(--accent) 14%, var(--bg-elev-2)) 0%,
        var(--bg-elev-2) 100%);
    border-bottom: 1px solid var(--border);
    overflow: hidden;
  }
  .upd-head[data-variant="ok"] {
    background: linear-gradient(180deg,
      color-mix(in oklch, var(--ok) 14%, var(--bg-elev-2)) 0%,
      var(--bg-elev-2) 100%);
  }
  .upd-head[data-variant="danger"] {
    background: linear-gradient(180deg,
      color-mix(in oklch, var(--danger) 14%, var(--bg-elev-2)) 0%,
      var(--bg-elev-2) 100%);
  }
  .upd-head[data-variant="info"] {
    background: linear-gradient(180deg,
      color-mix(in oklch, var(--info) 14%, var(--bg-elev-2)) 0%,
      var(--bg-elev-2) 100%);
  }

  .head-glow {
    position: absolute;
    inset: -40% -20% auto -20%;
    height: 80%;
    background: radial-gradient(60% 100% at 50% 0%,
      color-mix(in oklch, var(--accent) 25%, transparent) 0%,
      transparent 70%);
    pointer-events: none;
    opacity: 0.7;
  }
  .upd-head[data-variant="ok"] .head-glow     { background: radial-gradient(60% 100% at 50% 0%, color-mix(in oklch, var(--ok) 22%, transparent), transparent 70%); }
  .upd-head[data-variant="danger"] .head-glow { background: radial-gradient(60% 100% at 50% 0%, color-mix(in oklch, var(--danger) 22%, transparent), transparent 70%); }
  .upd-head[data-variant="info"] .head-glow   { background: radial-gradient(60% 100% at 50% 0%, color-mix(in oklch, var(--info) 22%, transparent), transparent 70%); }

  .head-row {
    position: relative;
    display: grid;
    grid-template-columns: 32px 1fr auto;
    gap: 12px; align-items: center;
  }
  .head-icon {
    width: 32px; height: 32px;
    border-radius: 8px;
    display: flex; align-items: center; justify-content: center;
    background: color-mix(in oklch, var(--accent) 25%, var(--bg-elev-3));
    color: var(--accent);
    border: 1px solid color-mix(in oklch, var(--accent) 30%, transparent);
    box-shadow: 0 0 16px color-mix(in oklch, var(--accent) 22%, transparent);
  }
  .head-icon[data-variant="ok"]     { background: color-mix(in oklch, var(--ok) 25%, var(--bg-elev-3));     color: var(--ok);     border-color: color-mix(in oklch, var(--ok) 30%, transparent);     box-shadow: 0 0 16px color-mix(in oklch, var(--ok) 22%, transparent); }
  .head-icon[data-variant="danger"] { background: color-mix(in oklch, var(--danger) 25%, var(--bg-elev-3)); color: var(--danger); border-color: color-mix(in oklch, var(--danger) 30%, transparent); box-shadow: 0 0 16px color-mix(in oklch, var(--danger) 22%, transparent); }
  .head-icon[data-variant="info"]   { background: color-mix(in oklch, var(--info) 25%, var(--bg-elev-3));   color: var(--info);   border-color: color-mix(in oklch, var(--info) 30%, transparent);   box-shadow: 0 0 16px color-mix(in oklch, var(--info) 22%, transparent); }

  .head-text { min-width: 0; }
  .head-title {
    font-size: var(--fs-md, 14px);
    font-weight: 600;
    color: var(--fg);
    line-height: 1.2;
  }
  .head-sub {
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    margin-top: 2px;
  }
  .head-close {
    background: transparent;
    border: 0;
    color: var(--fg-faint);
    cursor: pointer;
    padding: 4px;
    border-radius: var(--radius-xs);
    display: flex; align-items: center; justify-content: center;
    transition: background 120ms ease, color 120ms ease;
  }
  .head-close:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg); }
  .head-close:disabled { opacity: 0.4; cursor: not-allowed; }

  .head-diff {
    position: relative;
    margin-top: 12px;
    display: flex; align-items: center; gap: 8px;
    flex-wrap: wrap;
  }
  .diff-chip {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: var(--fs-xs);
    border: 1px solid var(--border);
  }
  .diff-chip.current {
    color: var(--fg-faint);
    background: var(--bg-elev-3);
  }
  .diff-chip.target {
    color: var(--accent);
    background: color-mix(in oklch, var(--accent) 14%, var(--bg-elev-3));
    border-color: color-mix(in oklch, var(--accent) 35%, transparent);
  }
  .upd-head[data-variant="ok"]     .diff-chip.target { color: var(--ok);     background: color-mix(in oklch, var(--ok) 14%, var(--bg-elev-3));     border-color: color-mix(in oklch, var(--ok) 35%, transparent); }
  .upd-head[data-variant="info"]   .diff-chip.target { color: var(--info);   background: color-mix(in oklch, var(--info) 14%, var(--bg-elev-3));   border-color: color-mix(in oklch, var(--info) 35%, transparent); }
  .upd-head[data-variant="danger"] .diff-chip.target { color: var(--danger); background: color-mix(in oklch, var(--danger) 14%, var(--bg-elev-3)); border-color: color-mix(in oklch, var(--danger) 35%, transparent); }

  :global(.diff-arrow) { color: var(--fg-faint); }
  .diff-meta {
    font-size: var(--fs-xs);
    color: var(--fg-faint);
  }

  /* ── Body ───────────────────────────────────────────────── */
  .upd-body {
    padding: 16px 18px;
    overflow-y: auto;
    display: flex; flex-direction: column; gap: 12px;
  }
  .lead { color: var(--fg-2); font-size: var(--fs-sm); line-height: 1.5; margin: 0; }

  /* Progress card */
  .progress-card {
    padding: 12px 14px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .progress-head {
    display: flex; justify-content: space-between; align-items: baseline;
    margin-bottom: 8px;
  }
  .progress-label { color: var(--fg-2); font-size: var(--fs-sm); }
  .progress-pct   { color: var(--accent); font-size: var(--fs-md); font-weight: 600; }
  .progress-track {
    position: relative;
    height: 6px;
    background: var(--bg-elev-3);
    border-radius: 999px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg,
      color-mix(in oklch, var(--accent) 70%, transparent),
      var(--accent));
    border-radius: 999px;
    transition: width 240ms cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 0 8px color-mix(in oklch, var(--accent) 50%, transparent);
  }
  .progress-shimmer {
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg,
      transparent 0%,
      color-mix(in oklch, white 12%, transparent) 50%,
      transparent 100%);
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
    pointer-events: none;
  }
  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
  @media (prefers-reduced-motion: reduce) {
    .progress-shimmer { animation: none; opacity: 0; }
  }
  .progress-foot {
    margin-top: 8px;
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
  }

  /* Ready card */
  .ready-card {
    display: grid; grid-template-columns: 24px 1fr; gap: 12px;
    align-items: start;
    padding: 12px 14px;
    background: color-mix(in oklch, var(--ok) 10%, var(--bg-elev-2));
    border: 1px solid color-mix(in oklch, var(--ok) 30%, var(--border));
    border-radius: var(--radius-sm);
    color: var(--fg);
  }
  .ready-card :global(svg) { color: var(--ok); margin-top: 2px; }
  .ready-title { font-size: var(--fs-sm); color: var(--fg); margin-bottom: 2px; }
  .ready-sub   { font-size: var(--fs-xs); color: var(--fg-subtle); line-height: 1.4; }

  /* OK / error cards */
  .ok-card {
    display: grid; grid-template-columns: 24px 1fr; gap: 12px;
    align-items: start;
    padding: 12px 14px;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .ok-card :global(svg) { color: var(--ok); margin-top: 2px; }
  .ok-title { font-size: var(--fs-sm); color: var(--fg); }
  .ok-sub   { font-size: var(--fs-xs); color: var(--fg-subtle); margin-top: 4px; }

  .err-card {
    display: grid; grid-template-columns: 18px 1fr; gap: 10px;
    padding: 12px 14px;
    background: color-mix(in oklch, var(--danger) 8%, var(--bg-elev-2));
    border: 1px solid color-mix(in oklch, var(--danger) 30%, var(--border));
    border-radius: var(--radius-sm);
  }
  .err-card :global(svg) { color: var(--danger); }
  .err-title  { font-size: var(--fs-sm); color: var(--fg); margin-bottom: 4px; }
  .err-detail { font-size: var(--fs-xs); color: var(--danger); margin-bottom: 6px; word-break: break-all; }
  .err-hint   { font-size: var(--fs-xs); color: var(--fg-subtle); }

  /* Notes card */
  .notes-card {
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .notes-head {
    display: flex; align-items: center; gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    color: var(--accent);
    font-size: var(--fs-xs);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
  }
  .notes-body {
    padding: 10px 14px 12px;
    font-size: var(--fs-sm);
    color: var(--fg-2);
    line-height: 1.55;
    max-height: 200px;
    overflow-y: auto;
  }
  .notes-h {
    color: var(--fg);
    font-weight: 600;
    margin-top: 8px;
    margin-bottom: 4px;
    font-size: var(--fs-sm);
  }
  .notes-h:first-child { margin-top: 0; }
  .notes-li {
    display: grid; grid-template-columns: 14px 1fr; gap: 4px;
    margin: 2px 0;
  }
  .notes-li .bullet { color: var(--accent); }
  .notes-p { margin: 2px 0; }
  .notes-spacer { height: 6px; }

  .link-row {
    display: inline-flex; align-items: center; gap: 6px;
    background: transparent;
    border: 0;
    color: var(--fg-subtle);
    font: inherit;
    font-size: var(--fs-xs);
    padding: 2px 0;
    cursor: pointer;
    align-self: flex-start;
    transition: color 120ms ease;
  }
  .link-row:hover { color: var(--accent); }

  /* ── Footer ─────────────────────────────────────────────── */
  .upd-foot {
    display: flex; align-items: center; gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-elev-2);
  }
  .foot-spacer { flex: 1; }
  .foot-status {
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
  }

  .btn.glow {
    box-shadow:
      0 0 0 1px color-mix(in oklch, var(--accent) 40%, transparent),
      0 0 18px color-mix(in oklch, var(--accent) 35%, transparent);
  }

  :global(.spin) { animation: spin 1s linear infinite; }
  @keyframes spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    :global(.spin) { animation: none; }
  }
</style>
