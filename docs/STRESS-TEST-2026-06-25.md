# Rift Model/Effort/Tool Stress Test — 2026-06-25

Autonomous stress test run while you were AFK. Drove the **real dev UI** via CDP
(`window.__assistant` store handle), against a throwaway test project
(`projects/_stress-test/` — buggy parser + user-store fixtures). Measured every
turn from the backend telemetry (`turns.ndjson`) — exact server-side ms, not
wall-clock guesses. 21 measured turns across 4 models × 5 effort tiers + a
dedicated tool-coverage pass.

## TL;DR — what I changed

1. **Found + fixed a real bug: the Tasks panel was dead.** The CLI (2.1.191)
   renamed the task tools `TodoWrite` → `TaskCreate`/`TaskUpdate`/`TaskList`/
   `TaskGet`/`TaskStop`. Rift's `--allowed-tools` allowlist still only listed the
   old `TodoWrite`, so the new tools were **gated out** — the model would say
   *"there's no TodoWrite tool in this environment"* and fall back to plain text.
   Fixed all three allowlists in `turn.rs` (BUILTINS / SAFE_BUILTINS /
   NO_WS_TOOLS). **Verified live:** the model now creates tasks again (4 tasks
   created in a test where it previously created none).

2. **Validated the defaults I shipped earlier today.** Thinking-OFF + Smart
   effort is the right default — it's the fastest/cheapest combo and matches how
   Claude Code itself behaves. The data did NOT support any further default
   change (see Finding 2/3). I did not over-tune.

## The data (warm = warm-pool reuse, COLD = fresh spawn)

| Model | Effort | State | Duration | TTFT | Cost |
|---|---|---|---|---|---|
| Opus | smart | warm | **3.9s** | 1.5s | $0.054 |
| Opus | ultra | warm | 3.9s | 2.2s | $0.054 |
| Fable | smart | warm | **3.5s** | 1.7s | $0.057 |
| Sonnet | smart | warm | 5.4s | 3.6s | $0.056 |
| Haiku | none | cold | 10.3s | 5.0s | $0.030 |

(Cold cells excluded from comparison — cold-start is a separate axis; see Finding 4.)

## Findings

**1. Effort barely affects latency when warm.** Opus warm: smart 3.9s, ultra
3.9s — identical. The effort flag changes *reasoning depth/cost*, not speed.
So "lower the default effort to go faster" is a non-starter; the real speed lever
is the warm-pool + thinking-off (already shipped). Keep Smart as the default.

**2. Thinking-ON has a real cost floor, especially on Opus.** With thinking on,
turns route around the no-think shim and bill thinking-eligible even when the
model produces no visible thinking block. Opus thinking-on cost **$0.2–0.26/turn
vs $0.054 off** (~4–5×), even on trivial prompts. This is correct/expected
(thinking-on = "allow thinking, pay for the capability"), and it's exactly why
**thinking-off as the default is the right call** — you opt into the cost
deliberately. Not a bug; a confirmation.

**3. Opus thinking text is invisible by design (CLI limitation).** Opus 4.7/4.8
default `thinking.display: "omitted"` — they stream only the thinking
*signature*, no readable text. The CLI exposes no flag to force "summarized", so
Rift literally cannot surface Opus reasoning today. Sonnet defaults to
"summarized" and DOES stream readable thinking. (Documented in `turn.rs:1030`.)
Implication: turning thinking on for Opus shows nothing in the UI but still
costs — another reason off-by-default is correct.

**4. Cold-start is the dominant latency factor, not the model.** Warm turns:
3.5–5.4s. Cold turns (after an effort change — effort is baked into the
warm-pool SpawnKey, so changing it forces a respawn): 6–12s. **Takeaway: don't
flip effort per-turn** — it silently triggers the ~2× slow cold path. Stable
effort across a conversation = the warm child is reused = fast.

**5. Haiku is the slowest model in practice (10–12s).** Counterintuitive for the
"fast" tier, but Haiku ignores effort and its cold-starts were the slowest
sampled. It's the cheapest ($0.030) but not the snappiest. Worth noting in any
model-picker guidance.

## Tool display — full verification

Drove every tool surface and confirmed rendering with screenshots:

| Tool | Status | Notes |
|---|---|---|
| Read | ✅ | Collapsible chip, file basename caption |
| Edit / MultiEdit | ✅ | Full syntax-highlighted red/green diff |
| Write | ✅ | "Created … +N -0" chip |
| Bash | ✅ | "Ran N command(s)", output streamed |
| Glob | ✅ | "Searched … in <dir>" |
| Grep | ✅ | Search caption |
| mcp__rift__git_status / git_diff | ✅ | MCP git chips render |
| mcp__rift__notify | ✅ | Toast pops correctly |
| mcp__rift__ask_user | ✅ | **Excellent** — multiple-choice card w/ title,
  category chip, per-option descriptions, Dismiss/Submit, round-trip resumes the
  turn on answer |
| TaskCreate / TaskUpdate | ⚠️ **PARTIAL** | Tools now WORK (allowlist fixed) and
  populate `tab.tasks` state + captions ("Planning · …") are wired — **but no UI
  component renders the task list.** See the open item below. |

## Tasks/Plan panel — FIXED (built it during autopilot)

I went ahead and built it (option a — inline card in the chat stream, like Claude
Code's own task display). Root cause was deeper than the allowlist: the
TaskCreate/TaskUpdate/TodoWrite handlers updated `tab.tasks` state but **never
appended a renderable block to the message**, so `StreamPlan.svelte` (which
already existed!) had no mount point. And `planItems()` only read the old
`todos[]` array shape, so even a block would've been empty under the new
one-task-per-TaskCreate CLI API.

Fix (4 files):
- `streaming.ts` — `ensurePlanBlock()` appends ONE plan block per turn on the
  first task event; `planBlockId` tracks it (reset in beginTurn).
- `streamModel.ts` — `tasksToPlanItems()` maps the store's aggregated `tab.tasks`
  → plan-card shape; `segmentWork()` coalesces consecutive plan blocks.
- `StreamTurn.svelte` — plan card falls back to the live `assistant.tasks`
  aggregate when the block's own items are empty (the TaskCreate case).
- `assistant.svelte.ts` — `planBlockId` field.

**Verified live:** 4-item plan renders with progress track (0/4 → 1/4), correct
per-item status marks (active ring for in_progress, hollow for todo, check for
done), and TaskUpdate live-updates the card reactively. svelte-check 0/0, 146
vitest assertions pass, 0 console errors. Screenshots confirm it looks polished.

Dedupe edge FIXED: a plan-turn followed by an update-turn now shows exactly ONE
card (the live aggregate gates to the last turn; older plan blocks resolve to
empty and are skipped). Verified: 2 turns → 1 card, count tracks correctly.

## Everything fixed + live in dev (full recap)

Earlier today:
- Permission hang → default `bypassPermissions` (tools just run)
- icacls DACL quoting bug → fixed (no more every-turn WARN spam)
- Permission round-trip → 30min hang → 120s fast-fail + clear message
- Thinking → off by default (mirrors Claude Code; "hello" is snappy now)

This autopilot session:
- **Task tools allowlist** → added TaskCreate/TaskUpdate/TaskList/TaskGet/
  TaskStop/TaskOutput to all 3 allowlists (was only `TodoWrite`; tasks were
  fully gated out on current CLI)
- **Tasks/Plan panel rebuilt** → inline plan card now renders, tracks live
  completion (0/4 → 4/4), correct status marks. Was a dead feature.
- **Thinking indicator for Opus** → Opus 4.7/4.8 stream thinking blocks with
  EMPTY text (display="omitted" — only a signature), so the "Thought for Xs" chip
  was gated out entirely: Opus thinking-on showed nothing while still costing
  thinking tokens. Now a duration-bearing thinking block earns the chip even with
  no readable text. Verified: Opus now shows "Thought" where it showed nothing.
  (`streamModel.ts` `thinkSeen` + relaxed gate.)

Heads-up from this: in the current headless CLI config, thinking TEXT isn't
streaming through for Sonnet either (blocks arrive with textLen=0). That may be
intentional (-p mode / display defaults) or a separate gap worth a look later —
I did not chase it (out of scope for the window). The chip fix makes the
indicator honest regardless.

Files changed (all live in dev, compiled clean, 86 cargo tests + 146 vitest
assertions pass, svelte-check 0/0):
- `src-tauri/src/assistant/turn.rs` — allowlists + perm round-trip hardening
- `src-tauri/src/assistant/config.rs` + `mod.rs` — icacls quoting
- `src/lib/state/assistant/helpers.ts` — perm + thinking defaults
- `src/lib/state/assistant/streaming.ts` — ensurePlanBlock
- `src/lib/state/assistant.svelte.ts` — planBlockId field
- `src/lib/components/assistant/stream/streamModel.ts` — tasksToPlanItems + coalesce
- `src/lib/components/assistant/stream/StreamTurn.svelte` — live plan fallback

Not shipped — running in dev per your call.

## Notes / cruft for you
- **5 git worktrees** under `.claude/worktrees/wf_2c91e220-*` (from a 05:24
  Workflow run on commit e88c267) have **uncommitted onboarding work** —
  `ClaudeConnect.svelte` + `OnboardingFlow.svelte` edits (looks like the start of
  the "new onboarding system" backlog item). I did **NOT** touch them — that's
  real unsaved work, not cruft. They DO break naive `vitest` runs (the stale
  copies fail tsconfig resolution); scope around them with
  `npx vitest run --exclude '**/.claude/**'`. If that onboarding work is dead,
  `git worktree remove --force` each — but check the diffs first, they're real.
- Effort labels (FYI, no bug): none=Instant, quick=Quick, smart=Smart,
  deep=Deep, ultra=Ultracode.

## Harness left in place (delete anytime)
- `projects/_stress-test/` — throwaway fixtures
- `scripts/cdp/stress/` — run-cell.sh / run-matrix.sh / results.ndjson
