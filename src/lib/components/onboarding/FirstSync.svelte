<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Check, CheckCircle2, Loader2, Info } from "lucide-svelte";

  // A simulated *preview* of what the first Sync-page scan will do — purely
  // illustrative. Real pull/push/delete counts are computed by the drift
  // scanner on the Sync page after the first scan; we never fabricate them here.
  type LogLine = { t: string; d: string };
  const SEQ: { phase: string; pct: number; log: LogLine }[] = [
    { phase: "Connecting…", pct: 30, log: { t: "Connect via SFTP", d: "ok" } },
    { phase: "Listing remote…", pct: 55, log: { t: "List remote root", d: "ok" } },
    { phase: "Scanning local…", pct: 80, log: { t: "Scan local workspace", d: "ok" } },
    { phase: "Comparing…", pct: 100, log: { t: "Three-way drift diff", d: "ok" } },
  ];

  let phase = $state("Starting…");
  let pct = $state(0);
  let logs = $state<LogLine[]>([]);
  let done = $state(false);
  let timers: ReturnType<typeof setTimeout>[] = [];

  onMount(() => {
    SEQ.forEach((s, i) => {
      timers.push(setTimeout(() => {
        phase = s.phase;
        pct = s.pct;
        logs = [...logs, s.log];
        if (i === SEQ.length - 1) {
          timers.push(setTimeout(() => { done = true; phase = "Ready to sync"; }, 420));
        }
      }, 480 + i * 620));
    });
  });
  onDestroy(() => { for (const t of timers) clearTimeout(t); });
</script>

<header class="ob-head">
  <span class="ob-eyebrow">Step 4 · You're set</span>
  <h1 class="ob-title">First sync</h1>
  <p class="ob-sub">Here's what the Sync page does on your first scan. <strong>Nothing moves</strong> until you review and apply.</p>
</header>

<div class="ob-sync">
  <div class="ob-sync-stage">
    <div class="ob-sync-head">
      <div class="ob-sync-spin" class:done>
        {#if done}<CheckCircle2 size={18} />{:else}<Loader2 size={18} class="spin" />{/if}
      </div>
      <div class="ob-sync-htext">
        <span class="ob-sync-phase">{phase}</span>
        <span class="ob-sync-meta">{pct}% · {done ? "preview complete" : "previewing"}</span>
      </div>
    </div>
    <div class="ob-bar"><div class="ob-bar-fill" class:done style="width:{pct}%"></div></div>
    <div class="ob-log">
      {#each logs as l (l.t)}
        <div class="ob-logline"><Check size={12} /> <span class="lt">{l.t}</span> <span class="ld">{l.d}</span></div>
      {/each}
    </div>
  </div>

  <div class="ob-summary">
    <div class="ob-spill pull"><span class="n">—</span><span class="l">To pull</span></div>
    <div class="ob-spill push"><span class="n">—</span><span class="l">To push</span></div>
    <div class="ob-spill del"><span class="n">—</span><span class="l">To delete</span></div>
  </div>

  <p class="ob-note">
    <Info size={13} />
    <span>Real counts appear on the <strong>Sync page</strong> after your first scan — Rift surfaces the full summary before any files move.</span>
  </p>
</div>
