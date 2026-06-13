# UI Polish Arc — "cleaner, less sloppy, more organized"

> Planning map for the next session. Goal (user, 2026-06-13): a **cleaner, less
> sloppy, more organized UI across the board** — not new features, *finish and
> tighten what's there*. Items below are grounded with `file:line` anchors so
> the executing session can go straight to the code. Tackle roughly top-down;
> each is independently shippable. The Harness rebuild (§7) is explicitly
> **deferred until the rest of this backlog is done**.

## Status legend
✅ done · 🔨 in progress (uncommitted) · ⬜ not started · 🧩 my-call (user delegated design decisions)

---

## 1. Live token counter — ✅ DONE + live-verified (`253a2b8`)
> Live-verify exposed a real bug: the counter sat frozen because the CLI emits
> no mid-stream `output_tokens` (only `=1` at message_start, final at the end).
> Rewrote to climb from a streamed-char estimate (`liveOutputChars/4`) layered
> on exact totals banked per completed message, snapping exact at each boundary.
> CDP-verified climbing 28→345; +2 playback regression tests.

Claude-Code-style cumulative output-token count climbing during a turn.

- **Done this session (5 files, committed `b450242`):**
  - `state/assistant/helpers.ts` — new `fmtTokens()` (`1.2k`/`45k`/`2.1M`, lowercase k).
  - `assistant.svelte.ts` — `liveOutputTokens` field on `TabState` (~:198) + store getter (~:431).
  - `state/assistant/streaming.ts` — reset to 0 at turn start (`beginTurn` ~:39); sum each `assistant` envelope's `output` in `recordTurnUsage` (`!accumulate` branch ~:429).
  - `MessageBubble.svelte` — `fmtTokens` import + `liveTokenLabel` derived (~:98); rendered in the spinner stage-strip (~:412) AND trailing-activity row (~:565); `.stage-sep`/`.stage-tokens` CSS.
  - `composer/LivePills.svelte` — replaced the `tok/s` pill with the cumulative `tokens` total (`liveTokens` derived + render).
- **Verified:** `npm run check` 0/0 · assistant playback+test nets 51/51.
- **REMAINING:** live CDP verify against a real turn (counter climbs, resets per turn, hidden when idle) → then commit. Decision per user: spinner line gets the count, pill **replaces** tok/s (both already done).

## 2. Notifications overhaul — ✅ DONE (`a0fc902`)
> ~22 transient `lastNotice` sites → severity-tuned toasts (new `notify.*`
> helpers); banner repurposed to one job: multi-line slash reference output
> (`/tools`,`/help`,`/stats`). Fixed the raw-path bug (prettyPath) + the
> invisible-Settings-errors bug (→ global toasts). UpdatePill/betaNotice left
> alone on purpose (UpdatePill was pulled OUT of the toast stack to fix the
> click bug — the doc's "fold in" was wrong). CDP-verified.

Two parallel notification systems that don't know about each other = the mess.

- **The good one:** unified `toast` stack — `state/toast.svelte.ts` + `components/ToastHost.svelte` (severity tones, icons, sticky, pause-on-hover, CTA, cap 3). Keep + lean on this.
- **The messy one:** ad-hoc `lastNotice` single-string banner — `AssistantPane.svelte:369`, hardcoded `ℹ`, no severity, **~20 call sites** (`send.ts`, `tabs.ts`, `workspace.ts`, `assistant.svelte.ts`) dumping confirmations + errors + state-changes into one slot that clobbers itself.
- **Visible bug:** `workspace.ts:54` does `` `Workspace: ${path}` `` with the **raw** path → shows `\\?\C:\…`. A `prettyPath()` helper already strips exactly that prefix (`tabsbar/helpers.ts:30`) — just isn't called here.
- **Stragglers:** `betaNotice.svelte.ts`, `UpdatePill.svelte` — more separate surfaces.
- **Plan (taxonomy):**
  - Transient confirmations (copied, model switched, workspace changed, telemetry copied) → **toast** (`ok`/`info`, auto-dismiss). ~80% of `lastNotice`.
  - Persistent/blocking (no auth, no folder open — they gate sending) → keep an inline banner but make it **severity-aware** (warn/danger tone+icon), not a raw glyph+string.
  - Async/background (turn done/failed, update ready) → toast (already there); fold in `betaNotice`/`UpdatePill`.
- **Net:** one visual language, paths always prettified, ephemeral stops blocking the composer, blocking actually looks blocking.
- **Scope:** quick-win (path fix + route confirmations to toast, ~3 files) vs full consolidation (~6 files). User leans toward fixing the *whole* thing → do full consolidation.

## 3. Activity panel repolish — ✅ DONE (5/7) (`4be207c`)
> Done: timestamp triplication killed (row=verb, sep=ago, right=duration);
> raw regex targets quoted+collapsed; category-coded step icons (green/blue/
> amber/neutral); session cost promoted to its own labelled line; live-token
> count in the Now strip. NOT done: #4 collapse same-tool runs (already
> mitigated by STEP_CAP=4 + "show more") and #6 section hierarchy (subjective,
> not sloppy) — both left intentionally. CDP-verified.

`components/assistant/ActivityPanel.svelte` (1002 L). Problems → fixes:

- **Timestamp triplication** — same "1m ago" in turn-sep (`:600`), every row sub-line (`agoLabel` `:582`), + duration right-col. → one home per timestamp (turn-sep owns relative "ago"; rows show only duration).
- **Raw regex as row target** — Grep steps render the literal pattern (`classifyTool` `:123`/`:140`) → unreadable `lastNotice|Workspace:|…`. → humanize/quote search targets, smart-truncate keeping the head.
- **Flat gray icons** — only write/ask/error/pending tinted (`:880-883`). → color-code each `StepCat` (read/search/shell/web/agent distinct).
- **47-row wall** — consecutive same-tool rows. → collapse runs (`Grep ×6`, expandable).
- **Cost buried** — `$5.10` lives in the ctx-meter foot (`:486`). → promote out (own line, or into the Last-turn recap grid).
- **Sections interchangeable** — Steps/Outputs/Sources share identical grammar (`:811-824`). → differentiate hierarchy (Outputs/artifacts heavier).
- **Tie-in:** surface §1's live-token count in the Now strip while streaming.

## 4. Image lightbox broken — ✅ DONE (`cc7dfa9`)
> Portal'd full-screen overlay (scrim z-3000, centered img, close btn); click
> anywhere / Esc to close (Esc listener scoped to an $effect). CDP-verified.

`MessageBubble.svelte:439` — `window.open(\`data:…\`)` is a **no-op in WebView2** (no browser tabs; CSP blocks `data:` nav). Tooltip/cursor imply clickable; nothing happens.
- **Fix:** in-app full-screen lightbox overlay via the existing `portal` action (`$lib/actions/portal.ts`) — centered image, click-anywhere/Esc to close. Same pattern as diff/dialog overlays. No new deps, CSP-safe.

## 5. Composer drag-and-drop broken — ✅ DONE (`aa9e2cb`)
> (a) window-level dragover/drop guard in AppShell — stray drops no longer
> navigate WebView2 to the file; they attach images to the active tab instead.
> (b) non-image rejects now surface (shared `attachImageFiles`/`summarizeAttach`
> → toast / inline). (c) deduped the 3 paste/drop/pick handlers. NOTE: the
> "@mention in-workspace files" idea isn't feasible — HTML5 drops expose no
> absolute path. CDP-verified + 22 helper tests.

`dragDropEnabled: false` is already correct (`tauri.conf.json:21`) — HTML5 file drops DO fire; breakage is in JS handling.
- **No global drop guard** — `AppShell.svelte` has only keyboard `preventDefault`s. A file dropped *outside* `.composer-shell` makes WebView2 **navigate to the file** (app appears to break). Worst symptom.
- **Image-only, silent reject** — `Composer.svelte:868` `if (!file.type.startsWith("image/")) continue;` → non-image files (code/text/pdf) silently dropped, no feedback.
- **Drop zone too narrow** — only `.composer-shell` (`:949`).
- **Fix:** (a) window-level `ondragover`/`ondrop` `preventDefault` catch-all in `AppShell` (foundational); (b) handle non-images — lean toward attaching in-workspace source files as a path/`@mention`, else reject with a clear toast; (c) widen drop target to the whole pane w/ one overlay.

## 6. Streaming / output rendering — ✅ DONE (conservative tuning) (`44cb8b2`)
> Tuned both stacked pacers to cut compounding latency WITHOUT an architectural
> rewrite: char pacer drains ~0.25s @180c/s (was 0.4s@120); word-reveal catch-up
> engages at ~520ms trail (was 800ms). CDP-verified smooth (bounded ~13-word
> blur trail, 0 errors). DEFERRED (high-risk on the core surface): full pacer
> merge, incremental tail-parse, code-block reveal unification (code blocks are
> rebuilt each frame by the {@html} re-parse → a mount-fade would flicker).

Two pacing systems stacked in series:
- **Pacer #1 (chars):** `streaming.ts:215` rAF drain, `rate = max(120, len/0.4)` c/s.
- **Pacer #2 (word blur):** `Markdown.svelte:123` — `WORD_MS=42`, 500ms blur, adaptive catch-up (`:188`).
- **Improvements:** collapse the double pacer into one authority (lower latency on fast bursts; catch-up exists *because* of the stacking); unify reveal across **code blocks** (currently skipped `:212-213` → code pops while prose blurs) + non-text blocks (chips/thinking pop in); consider incremental tail-parse (full re-parse per frame `:155` is O(n)/frame late in long replies); tune cadence/blur.
- User said "make the call" → chase **latency + motion-consistency** first, cadence second.

---

## 7. DEFERRED — Harness rebuild (AFTER the backlog above)
The Harness page + Cost cockpit + Swarm were intentionally removed in the
v0.9.0 minimal-core strip (commit `470845b strip(S3)`, 2026-06-12; full suite
across `cde962d`→`bd1709c`). User wants it **rebuilt properly** once the polish
backlog is done — "build it back properly," not a raw revert.
- **Recovery source:** everything intact up to `ba1f7dc` (commit before the strip). `HarnessPage`/`CostPage`/`SwarmPage` + workspace registry + palette rows + (for cost) the SQLite/rusqlite backend.
- **Approach TBD:** likely a clean re-implementation matching the new minimal-core aesthetic, NOT a wholesale `git revert` (would drag back the dead weight the strip removed). Scope when we get there.

---

## Working order (suggested)
1 (commit + verify §1) → 2 (notifications, unblocks visible mess) → 4 + 5 (broken features, high user-visible payoff, small) → 3 (activity polish) → 6 (streaming, subjective/iterative) → **then** 7 (Harness rebuild).

Overarching bar for every item: **cleaner, less sloppy, more organized.** When in doubt, fewer surfaces, one visual language, no silent failures.
