<script lang="ts">
  import {
    Download, X, RefreshCw, AlertTriangle, CheckCircle2,
    ArrowRight, ExternalLink, Clock, WifiOff, FlaskConical,
  } from "@lucide/svelte";
  import { fade, fly } from "svelte/transition";
  import { updates } from "../../state/updates.svelte";

  // One tone drives the hero accents per state; "muted" is the calm default
  // (checking, idle, dev build).
  type Tone = "accent" | "ok" | "warn" | "danger" | "muted";
  const tone = $derived<Tone>(
    updates.state === "available" || updates.state === "downloading" ? "accent" :
    updates.state === "installing" || updates.state === "uptodate" ? "ok" :
    updates.state === "error"
      ? (updates.devUnavailable ? "muted" : updates.installBroken ? "danger" : "warn")
      : "muted"
  );

  // Only the brief pre-exit install window locks the dialog. A download is
  // closable — the top banner keeps showing live progress (UpdateBanner gates
  // on busy states too), so hiding the dialog loses nothing.
  const busy = $derived(updates.state === "installing");

  let shellEl: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (updates.dialogOpen) {
      void Promise.resolve().then(() => shellEl?.focus());
    }
  });

  function onBackdrop(e: MouseEvent) {
    if (e.target === e.currentTarget && !busy) updates.close();
  }
  function onKey(e: KeyboardEvent) {
    if (!updates.dialogOpen) return;
    if (e.key === "Escape" && !busy) { updates.close(); return; }
    if (e.key === "Tab") {
      if (!shellEl) return;
      e.preventDefault();
      const focusable = Array.from(
        shellEl.querySelectorAll<HTMLElement>("button:not([disabled]), [href], input, summary, [tabindex]:not([tabindex='-1'])")
      );
      if (!focusable.length) return;
      const cur = document.activeElement as HTMLElement;
      const idx = focusable.indexOf(cur);
      if (e.shiftKey) {
        const prev = idx <= 0 ? focusable[focusable.length - 1] : focusable[idx - 1];
        prev.focus();
      } else {
        const next = idx < 0 || idx >= focusable.length - 1 ? focusable[0] : focusable[idx + 1];
        next.focus();
      }
    }
  }

  // Render the markdown body as plain text w/ light formatting. We control
  // the source (our own CHANGELOG snippet) so HTML injection isn't a concern;
  // still mount as text nodes for defense-in-depth.
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

  /** "just now" / "3 min ago" / "2 h ago" — computed on each render pass (the
   *  dialog re-renders on every state change, which is fresh enough). */
  function ago(ts: number | null): string {
    if (ts == null) return "";
    const s = Math.max(0, (Date.now() - ts) / 1000);
    if (s < 45) return "just now";
    if (s < 3600) return `${Math.round(s / 60)} min ago`;
    return `${Math.round(s / 3600)} h ago`;
  }

  /** "5.2 of 12.4 MB" derived from the total size + percent ticks. */
  const mbProgress = $derived.by(() => {
    const total = updates.info?.sizeBytes ?? 0;
    if (total <= 0) return "";
    const mb = (n: number) => (n / (1024 * 1024)).toFixed(1);
    return `${mb(total * (updates.progress / 100))} of ${mb(total)} MB`;
  });
</script>

<svelte:window onkeydown={onKey} />

{#if updates.dialogOpen}
  <div class="dialog-overlay" onclick={onBackdrop} role="presentation" transition:fade={{ duration: 120 }}>
    <div
      class="upd-shell"
      role="dialog"
      aria-modal="true"
      aria-label="Updates"
      tabindex="-1"
      bind:this={shellEl}
      transition:fly={{ y: 8, duration: 180 }}
    >
      <!-- Slim top bar -->
      <div class="upd-top">
        <span class="upd-top-t">Updates</span>
        {#if updates.lastCheckedAt != null && updates.state !== "checking"}
          <span class="upd-top-when"><Clock size={10} /> checked {ago(updates.lastCheckedAt)}</span>
        {/if}
        <button class="upd-x" type="button" onclick={() => updates.close()} aria-label="Close" disabled={busy}>
          <X size={14} />
        </button>
      </div>

      <!-- Hero — the state IS the interface -->
      <div class="upd-hero" data-tone={tone}>
        {#if updates.state === "checking"}
          <div class="hero-ic"><RefreshCw size={20} class="spin" /></div>
          <div class="hero-title">Checking for updates…</div>
          <div class="hero-sub">Asking the release feed what's newest.</div>

        {:else if updates.state === "available"}
          <div class="hero-vers">
            <span class="hv-cur mono">v{updates.currentVersion}</span>
            <ArrowRight size={15} class="hv-arr" />
            <span class="hv-new mono">v{updates.info?.version}</span>
          </div>
          <div class="hero-title">A new version of Rift is ready</div>
          <div class="hero-sub">
            {#if updates.sizeLabel}{updates.sizeLabel} download — {/if}installs and relaunches in one click. Your chats and settings carry over.
          </div>

        {:else if updates.state === "downloading"}
          <div class="hero-pct mono">{updates.progress}<span class="hero-pct-sign">%</span></div>
          <div class="hero-track" role="progressbar" aria-valuenow={updates.progress} aria-valuemin={0} aria-valuemax={100} aria-label="Download progress">
            <div class="hero-fill" style="width:{updates.progress}%"></div>
          </div>
          <div class="hero-sub">Downloading v{updates.info?.version}{mbProgress ? ` — ${mbProgress}` : ""}</div>

        {:else if updates.state === "installing"}
          <div class="hero-ic"><RefreshCw size={20} class="spin" /></div>
          <div class="hero-title">Installing v{updates.info?.version}</div>
          <div class="hero-sub">Rift will close, swap in the new version, and relaunch itself in a moment.</div>

        {:else if updates.state === "uptodate"}
          <div class="hero-ic"><CheckCircle2 size={20} /></div>
          <div class="hero-ver-big mono">v{updates.currentVersion}</div>
          {#if updates.justUpdatedFrom}
            <div class="hero-title">Updated — you're on the latest</div>
            <div class="hero-sub">Just moved up from v{updates.justUpdatedFrom}. Rift keeps itself current, re-checking every 6 hours and at every launch.</div>
          {:else}
            <div class="hero-title">You're up to date</div>
            <div class="hero-sub">Rift re-checks automatically — every 6 hours and at every launch.</div>
          {/if}

        {:else if updates.state === "error"}
          {#if updates.devUnavailable}
            <div class="hero-ic"><FlaskConical size={20} /></div>
            <div class="hero-title">Dev build — updates don't apply here</div>
            <div class="hero-sub">This binary is rebuilt by the dev server, not the updater. Installed copies of Rift update themselves from the release feed.</div>
          {:else if updates.installBroken}
            <div class="hero-ic"><AlertTriangle size={20} /></div>
            <div class="hero-title">Auto-update is unavailable on this install</div>
            <div class="hero-sub">Rift can't find its install manifest — usually after files were moved by hand. One reinstall with the latest Setup.exe restores auto-updates; your chats and settings are kept.</div>
          {:else}
            <div class="hero-ic"><WifiOff size={20} /></div>
            <div class="hero-title">Couldn't reach the update feed</div>
            <div class="hero-sub">You may be offline, or a firewall or proxy is blocking the feed. Rift keeps retrying in the background.</div>
          {/if}
          {#if !updates.devUnavailable}
            <details class="hero-tech">
              <summary>Technical details</summary>
              <pre>{updates.error}</pre>
            </details>
          {/if}

        {:else}
          <div class="hero-ic"><Download size={20} /></div>
          <div class="hero-title">No update info yet</div>
          <div class="hero-sub">Check the release feed to see if something newer shipped.</div>
        {/if}
      </div>

      <!-- Body — release notes / download-failure strip (available state only) -->
      {#if updates.state === "available"}
        <div class="upd-body">
          {#if updates.downloadError}
            <div class="err-strip">
              <AlertTriangle size={14} />
              <div class="err-text">
                <div class="err-title">The install didn't finish — nothing was changed.</div>
                <details class="err-tech">
                  <summary>Technical details</summary>
                  <pre>{updates.downloadError}</pre>
                </details>
                <div class="err-hint">Try again, or grab the installer from GitHub below.</div>
              </div>
            </div>
          {/if}

          {#if notes.length > 0}
            <div class="notes-card">
              <div class="notes-head">What's new in v{updates.info?.version}</div>
              <div class="notes-body">
                {#each notes as ln, i (`${i}:${ln.kind}`)}
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

          <button class="link-row" type="button" onclick={() => void updates.openReleasePage()}>
            <ExternalLink size={11} />
            <span>{notes.length > 0 ? "View this release on GitHub" : "Read the release notes on GitHub"}</span>
          </button>
        </div>
      {/if}

      <!-- Footer adapts per state -->
      <div class="upd-foot">
        {#if updates.state === "available"}
          <button class="btn ghost" type="button" onclick={() => updates.snooze()}>
            <Clock size={11} /> Remind me tomorrow
          </button>
          <div class="foot-spacer"></div>
          <button class="btn primary glow" type="button" onclick={() => updates.download()}>
            <Download size={11} /> Update now
          </button>

        {:else if updates.state === "downloading"}
          <span class="foot-status">You can keep using Rift — the top bar tracks progress.</span>
          <div class="foot-spacer"></div>
          <button class="btn ghost" type="button" onclick={() => updates.close()}>Hide</button>

        {:else if updates.state === "installing"}
          <span class="foot-status">Installing… Rift will relaunch.</span>
          <div class="foot-spacer"></div>
          <button class="btn" type="button" disabled>
            <RefreshCw size={11} class="spin" /> Please wait
          </button>

        {:else if updates.state === "error" && updates.installBroken && !updates.devUnavailable}
          <button class="btn ghost" type="button" onclick={() => updates.close()}>Close</button>
          <div class="foot-spacer"></div>
          <button class="btn" type="button" onclick={() => updates.refresh()}>
            <RefreshCw size={11} /> Check again
          </button>
          <button class="btn primary" type="button" onclick={() => void updates.openLatestRelease()}>
            <ExternalLink size={11} /> Get Setup.exe
          </button>

        {:else}
          <button class="btn ghost" type="button" onclick={() => updates.close()}>Close</button>
          <div class="foot-spacer"></div>
          <button
            class="btn"
            type="button"
            onclick={() => updates.refresh()}
            disabled={updates.state === "checking"}
          >
            <RefreshCw size={11} class={updates.state === "checking" ? "spin" : ""} />
            {updates.state === "checking" ? "Checking…" : "Check now"}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .upd-shell {
    width: 500px;
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

  /* ── Top bar ────────────────────────────────────────────── */
  .upd-top {
    display: flex; align-items: center; gap: 10px;
    height: 42px;
    padding: 0 8px 0 16px;
    border-bottom: 1px solid var(--border);
    flex: none;
  }
  .upd-top-t {
    font-size: var(--fs-sm);
    font-weight: 650;
    letter-spacing: -0.01em;
    color: var(--fg);
  }
  .upd-top-when {
    display: inline-flex; align-items: center; gap: 4px;
    margin-left: auto;
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    white-space: nowrap;
  }
  .upd-top-when + .upd-x { margin-left: 0; }
  .upd-x {
    margin-left: auto;
    background: transparent;
    border: 0;
    color: var(--fg-faint);
    cursor: pointer;
    width: 28px; height: 28px;
    border-radius: var(--radius-xs);
    display: flex; align-items: center; justify-content: center;
    flex: none;
    transition: background var(--dur-fast) ease, color var(--dur-fast) ease;
  }
  .upd-x:hover:not(:disabled) { background: var(--surface-hover); color: var(--fg); }
  .upd-x:disabled { opacity: 0.4; cursor: not-allowed; }

  /* ── Hero ───────────────────────────────────────────────── */
  .upd-hero {
    --tone: var(--fg-muted);
    --tone-soft: color-mix(in oklab, var(--fg-muted) 10%, transparent);
    position: relative;
    display: flex; flex-direction: column; align-items: center;
    gap: 8px;
    padding: 28px 24px 24px;
    text-align: center;
    overflow: hidden;
  }
  .upd-hero[data-tone="accent"] { --tone: var(--accent); --tone-soft: color-mix(in oklab, var(--accent) 12%, transparent); }
  .upd-hero[data-tone="ok"]     { --tone: var(--ok);     --tone-soft: color-mix(in oklab, var(--ok) 12%, transparent); }
  .upd-hero[data-tone="warn"]   { --tone: var(--warn);   --tone-soft: color-mix(in oklab, var(--warn) 12%, transparent); }
  .upd-hero[data-tone="danger"] { --tone: var(--danger); --tone-soft: color-mix(in oklab, var(--danger) 12%, transparent); }
  /* Soft state-tinted wash behind the hero — glow without a heavy banner. */
  .upd-hero::before {
    content: "";
    position: absolute;
    inset: -60% -30% auto -30%;
    height: 130%;
    background: radial-gradient(50% 55% at 50% 0%, var(--tone-soft), transparent 75%);
    pointer-events: none;
  }
  .upd-hero > :global(*) { position: relative; }

  .hero-ic {
    width: 44px; height: 44px;
    border-radius: 13px;
    display: grid; place-items: center;
    color: var(--tone);
    background: color-mix(in oklab, var(--tone) 14%, var(--bg-elev-2));
    border: 1px solid color-mix(in oklab, var(--tone) 26%, transparent);
    box-shadow: 0 0 22px var(--tone-soft);
    margin-bottom: 2px;
  }

  .hero-vers {
    display: flex; align-items: baseline; gap: 12px;
    margin-bottom: 2px;
  }
  .hv-cur { font-size: 15px; font-weight: 550; color: var(--fg-faint); }
  .hero-vers :global(.hv-arr) { color: var(--fg-faint); align-self: center; }
  .hv-new {
    font-size: 32px;
    font-weight: 680;
    letter-spacing: -0.02em;
    color: var(--tone);
    text-shadow: 0 0 26px var(--tone-soft);
    line-height: 1.1;
  }

  .hero-ver-big {
    font-size: 26px;
    font-weight: 660;
    letter-spacing: -0.02em;
    color: var(--fg);
    line-height: 1.1;
  }

  .hero-title {
    font-size: 15px;
    font-weight: 620;
    color: var(--fg);
    letter-spacing: -0.01em;
  }
  .hero-sub {
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    line-height: 1.55;
    max-width: 380px;
  }

  /* Download hero */
  .hero-pct {
    font-size: 40px;
    font-weight: 680;
    letter-spacing: -0.02em;
    color: var(--tone);
    line-height: 1;
    text-shadow: 0 0 28px var(--tone-soft);
    font-variant-numeric: tabular-nums;
  }
  .hero-pct-sign { font-size: 20px; font-weight: 600; color: color-mix(in oklab, var(--tone) 65%, transparent); margin-left: 2px; }
  .hero-track {
    width: 100%;
    max-width: 320px;
    height: 7px;
    border-radius: 999px;
    background: var(--bg-elev-3);
    overflow: hidden;
    margin: 4px 0 2px;
  }
  .hero-fill {
    height: 100%;
    background: linear-gradient(90deg, color-mix(in oklab, var(--tone) 78%, transparent), var(--tone));
    border-radius: 999px;
    transition: width var(--dur-fast) ease;
  }

  /* Collapsed raw error — human copy leads, the dump is opt-in. */
  .hero-tech, .err-tech {
    margin-top: 6px;
    max-width: 400px;
    width: 100%;
    text-align: left;
  }
  .hero-tech summary, .err-tech summary {
    font-size: var(--fs-xs);
    color: var(--fg-faint);
    cursor: pointer;
    user-select: none;
    text-align: center;
    list-style-position: inside;
  }
  .err-tech summary { text-align: left; }
  .hero-tech summary:hover, .err-tech summary:hover { color: var(--fg-subtle); }
  .hero-tech pre, .err-tech pre {
    margin: 6px 0 0;
    padding: 8px 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.5;
    color: var(--fg-subtle);
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    max-height: 96px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* ── Body ───────────────────────────────────────────────── */
  .upd-body {
    padding: 0 20px 16px;
    overflow-y: auto;
    display: flex; flex-direction: column; gap: 10px;
  }

  .err-strip {
    display: grid; grid-template-columns: 16px 1fr; gap: 10px;
    padding: 11px 13px;
    background: color-mix(in oklab, var(--danger) 8%, var(--bg-elev-2));
    border: 1px solid color-mix(in oklab, var(--danger) 28%, var(--border));
    border-radius: var(--radius-sm);
  }
  .err-strip :global(svg) { color: var(--danger); margin-top: 1px; }
  .err-title { font-size: var(--fs-sm); color: var(--fg); }
  .err-hint  { font-size: var(--fs-xs); color: var(--fg-subtle); margin-top: 5px; }

  .notes-card {
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .notes-head {
    padding: 9px 13px;
    border-bottom: 1px solid var(--border);
    color: var(--accent);
    font-size: var(--fs-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 650;
  }
  .notes-body {
    padding: 10px 14px 12px;
    font-size: var(--fs-sm);
    color: var(--fg-2);
    line-height: 1.55;
    max-height: 190px;
    overflow-y: auto;
  }
  .notes-h {
    color: var(--fg);
    font-weight: 600;
    margin: 8px 0 4px;
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
    transition: color var(--dur-fast) ease;
  }
  .link-row:hover { color: var(--accent); }

  /* ── Footer ─────────────────────────────────────────────── */
  .upd-foot {
    display: flex; align-items: center; gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-elev-2);
    flex: none;
  }
  .foot-spacer { flex: 1; }
  .foot-status {
    font-size: var(--fs-xs);
    color: var(--fg-subtle);
    min-width: 0;
  }

  .btn.glow {
    box-shadow:
      0 0 0 1px color-mix(in oklab, var(--accent) 40%, transparent),
      0 0 18px color-mix(in oklab, var(--accent) 35%, transparent);
  }

  :global(.spin) { animation: spin 1s linear infinite; }
  @keyframes spin { from { transform: rotate(0); } to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) {
    :global(.spin) { animation: none; }
    .hero-fill { transition: none; }
  }
</style>
