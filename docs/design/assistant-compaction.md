# Assistant — Conversation Compaction

> **Author:** S101 research, 2026-05-19. Read-only plan — no code shipped. Ready to execute on user's signal.
>
> **Companion to** [assistant-roadmap.md](assistant-roadmap.md). That doc's Phase 5 ("Background Agents") was deferred; this is the next unblocked piece of Assistant work.

## TL;DR

The Claude CLI cannot be asked to compact via `-p`. `/compact` is interactive-only ([GH #14472](https://github.com/anthropics/claude-code/issues/14472), official commands docs). The user also has `DISABLE_AUTO_COMPACT=1` set globally in `~/.claude/settings.json`, so even the CLI's own auto-compact is off. There is no CLI flag for compaction at all.

Therefore: **Rift owns compaction end-to-end.** When `ctxPct` crosses threshold, Rift summarizes the current conversation via a one-shot `claude -p` call, retires the CLI session id, mints a new session, and seeds the first turn of the new session with the summary. The user's Rift-side conversation record (`~/.rift/assistant/conversations/<convo>.json`) keeps the full history; the CLI-side JSONL (`~/.claude/projects/<cwd-hash>/<uuid>.jsonl`) is abandoned per compaction.

This is feasible, low-risk if we sequence carefully, and the UX scaffolding is already 80% there (context pill, sessionId/--resume separation, sidecar cwd pin).

---

## Prior art (validates the shape)

Pattern-matching against established compaction systems:

- **Cline** ([docs](https://docs.cline.bot/features/auto-compact)) — auto-compacts via an inline summarization tool when approaching the context limit. Summary replaces prior history in-conversation; same session continues. Surfaces the cost. **This is the closest analog to Rift's plan** minus the session-id remint (Cline doesn't have to manage that — it owns the model conversation directly).
- **Aider** ([overview](https://dev.to/crabtalk/context-compaction-in-agent-frameworks-4ckk)) — `ChatSummary` triggers at ~70% token budget; keeps last N turns verbatim + rolling summary of older turns via a cheaper model. **Rift's "keep last 3 turns + boundary message + summary" mirrors this directly.**
- **Cursor** ([docs](https://cursor.com/docs/agent/chat/summarization)) — two-layer: a smaller "flash model" auto-summarizes near limit + manual `/compress`. Composer model is RL-trained to self-summarize at fixed triggers. Sophisticated; not realistic for us, but validates "use the cheapest model for summarization" as the right default — matches our Haiku 4.5 pick.
- **Continue.dev** — manual `@`-mentions only, no auto-compact. Worth knowing as the counter-example: pure-manual is what users complain about on long sessions.

**Implication:** Rift's design isn't novel — it's the conventional shape, with the one Rift-specific twist that we have to remint the CLI session id because we don't own the model conversation, the CLI does.

---

## Context (60s read)

**Today's compaction story = "tell the user to Ctrl+T".** Header pill at [AssistantHeader.svelte:174-180](src/lib/components/assistant/AssistantHeader.svelte#L174) goes yellow at 70%, red at 90%; tooltip nudges toward new tab. No automated action, no summary preservation. Closing the tab drops all context.

**Architectural facts that gate the design:**

1. **Rift `currentConvoId` ≠ CLI `sessionId`.** Rift's convo id is local (lives in `~/.rift/assistant/conversations/<id>.json`); the CLI session id is what feeds `--session-id`/`--resume`. Today they're the same UUID by accident — `assistant.svelte.ts:857` mints one UUID for both. Compaction needs them split.
2. **CWD sidecar at `~/.rift/assistant/sessions/<uuid>.cwd`** (S98) — used by `load_session_cwd()` at [mod.rs:916](src-tauri/src/assistant/mod.rs#L916) to make `--resume` survive workspace swaps. On compaction we mint a new session uuid → must copy/move the sidecar.
3. **`isFirstTurn` gate at [assistant.svelte.ts:855](src/lib/state/assistant.svelte.ts#L855)** — `!convoCreatedAt` triggers `--session-id`; otherwise `--resume`. Compaction needs to flip this back to first-turn for the new uuid.
4. **`result.subtype !== "success"` lights up as error** at [assistant.svelte.ts:1359](src/lib/state/assistant.svelte.ts#L1359). If a compaction-adjacent CLI emits anything unusual (it shouldn't, since we're not invoking `/compact`), this gate fires. Worth whitelisting known subtypes anyway.
5. **Token usage is already wired.** `lastTurnUsage` + `sessionUsage` carry input/output/cache_read/cache_create per turn ([assistant.svelte.ts:1223](src/lib/state/assistant.svelte.ts#L1223)). The pill computation at [AssistantHeader.svelte:85-92](src/lib/components/assistant/AssistantHeader.svelte#L85) already gives us the threshold trigger.
6. **`AssistantConfig` pattern is established** at [mod.rs:177](src-tauri/src/assistant/mod.rs#L177). Three existing flags (`use_full_config`, `max_budget_usd`, `allow_remote_shell`) each have `get_*`/`set_*` Tauri cmds + Settings.svelte rows. Compaction settings drop in cleanly.

**What's NOT there yet:**
- No "abandon and remint" path for `sessionId` mid-conversation.
- No archival semantics — Rift's convo JSON is one flat array, no notion of "pre-compaction tail".
- No system/synthetic message kind for the UI bubble that announces a compaction boundary.
- No throttle / cooldown around auto-trigger (necessary — runaway compaction on a stuck 95% pill would burn money).

---

## Don't-Touch (perpetual)

- **Don't invoke CLI `/compact`.** It is interactive-only and broken in `-p` mode (GH #14472). Sending the literal text `/compact` as a user prompt would silently get processed as a regular message.
- **Don't disable `DISABLE_AUTO_COMPACT=1`** on the user's behalf. That's the user's deliberate choice in `~/.claude/settings.json`; Rift's compaction layer is the replacement, not an opt-back-in.
- **Don't use `--input-format stream-json` for the summarize call** unless GH #5034 (duplicate JSONL entries) is confirmed fixed via live probe. Stick to `text` input format for the summarize call regardless of whether the visible turn had attachments.
- **Preserve full Rift-side history.** The CLI JSONL gets abandoned, but the user's `~/.rift/assistant/conversations/<id>.json` keeps every original message. Compaction is about the *CLI's working context*, not destroying the user's record.
- **Cwd sidecar must move with the new session uuid.** If compaction mints a new uuid and the sidecar doesn't follow, the next `--resume` fails and S98's bug returns.

---

## Open Questions to Resolve Before Phase 1

1. **GH #5034 confirmed closed-won't-fix** ([issue](https://github.com/anthropics/claude-code/issues/5034) — closed 2025-08-03). The bloat affects JSONL on multi-turn `--input-format stream-json`. **Doesn't actually bite our hot path** because Rift's compaction never parses the JSONL — we abandon it on remint and start fresh. The risk only materializes if Phase E2's "expand archived turns" feature reads CLI-side JSONL (which we shouldn't — read Rift's own convo JSON instead). De-rated to "verify CLI version on Blazzer's machine isn't worse than the documented behavior."
2. **Fresh `--session-id` mid-conversation behavior.** What happens if we kill an in-flight stream, then immediately spawn a fresh `--session-id <new-uuid>` against the same cwd? Does the old JSONL get reaped? Does `--resume <old-uuid>` still work afterward (= can we offer a "rollback compaction" feature)?
3. **Per-turn `usage` accuracy on a summarize-only call.** Does a one-shot summarize call (no MCP, no tools) emit a `result.usage` envelope we can trust to size the new session's seeded context?
4. **Skill behavior across compaction.** Skills docs say auto-compact "carries invoked skills forward within a token budget" (up to 25K). Our manual compaction doesn't get that machinery for free. Mitigation: include the active TodoWrite tasks (already in `assistant.tasks`) in the summary prompt verbatim.

A **single CDP-driven dev probe** answers (2)-(3) in ~15min. Plan A1 below.

---

## Plan

Five phases. Each ships as its own version bump and can be reverted independently.

### Phase A — Probe + scaffolding (~1 hr, no user-visible change)

**Goal:** confirm the load-bearing CLI behaviors before committing to the design, and lay the field for the real work.

A1. **Live probe.** Hand-driven via CDP:
   - Start a convo, send 3 turns. Confirm `~/.claude/projects/<cwd-hash>/<uuid>.jsonl` exists and grows by O(turn-content) per turn, not O(N²).
   - Kill the streaming child mid-turn (assistant_stop) and confirm subsequent `--resume <uuid>` against the same uuid still works (= we can compact safely after aborting a turn).
   - Manually mint a fresh UUID in dev console and call `assistant_send` with it + `isFirstTurn: true` → confirm `--session-id <new-uuid>` lands in a new JSONL alongside the old one (both files present in the cwd-hash dir).
   - Trigger a one-shot `claude -p --resume <uuid> --model haiku` from a separate Bash with prompt "summarize this in 200 tokens"; capture the NDJSON. Confirm a `result.usage` envelope arrives + `total_cost_usd` is non-zero + the streamed text is usable summary content.
   - Skip the GH #5034 probe — confirmed closed-won't-fix and not on our hot path.

A2. **Decouple Rift convo id from CLI session id.** In `assistant.svelte.ts`, add a `cliSessionId: string | null` field to `ConversationRecord` (default = same as `id` for legacy convos). Initialize on first send. The `assistant_send` invoke passes `cliSessionId`, not `currentConvoId`. Frontend continues to identify the chat by `currentConvoId`. This is the precondition for compaction: we need to be able to remint `cliSessionId` without breaking tab persistence / history.

A3. **Whitelist `result.subtype`.** At [assistant.svelte.ts:1359](src/lib/state/assistant.svelte.ts#L1359) — accept `"success"` silently as today, log unknown subtypes to console without surfacing as `lastError`. Pre-emptive safety: if any post-compaction CLI behavior emits an unusual subtype, we won't false-alarm the user.

A4. **`AssistantConfig` fields.** Add to [mod.rs:177](src-tauri/src/assistant/mod.rs#L177):
   ```rust
   /// Auto-compact threshold as fraction of context window (0.0-1.0).
   /// `None` = disabled (manual only). User has `DISABLE_AUTO_COMPACT=1` set
   /// globally so default to None — opt-in, not opt-out.
   #[serde(default)]
   auto_compact_threshold: Option<f32>,
   /// Model alias to use for the summarize call. `None` = "haiku" (cheap +
   /// fast; sufficient for prose summarization w/ explicit preservation prompt).
   /// $0.91 at 900K vs $2.73 on sonnet.
   #[serde(default)]
   compact_model: Option<String>,
   ```
   Plus `assistant_get_*`/`assistant_set_*` cmds, wired through the existing pattern.

A5. **No UI yet.** Settings rows wait for Phase D.

**Verify:** `npm run check`; `cargo check` (only if dev not alive); CDP probe of A1 above; confirm A2 doesn't regress tab persistence by reloading dev w/ open tabs.

---

### Phase B — Summarize primitive (~half day)

**Goal:** a Rust-side primitive that, given a session id and an optional focus string, produces a high-fidelity prose summary of the current conversation. No state change anywhere — this is a pure read.

B1. **New Tauri command:** `assistant_summarize_session(session_id: String, focus: Option<String>) -> Result<SummarizeResult, String>` in `assistant/mod.rs`.

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummarizeResult {
    pub summary: String,
    pub model: String,        // "claude-haiku-4-5" etc
    pub cost_usd: f64,
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

B2. **Implementation:** spawn `claude -p --resume <session_id> --output-format stream-json --verbose --model haiku --max-budget-usd 0.50` (hard cap on summarize cost). Pipe a prompt like:

> The user is approaching their context window cap. Produce a structured summary of this conversation that another instance of you could read in <2K tokens and pick up where we left off without losing critical state. Preserve verbatim: (1) any pending TodoWrite items + their statuses, (2) file paths actively being worked on + the last revision direction for each, (3) decisions explicitly made by the user, (4) open questions or blockers. Drop: tool-call mechanics, exploratory dead-ends, verbose tool outputs. Focus: {focus_or_"general continuation"}. Output format: 4 sections — "Active task", "Files in play", "Decisions", "Open questions". No preamble or sign-off.

B3. **NDJSON parse.** Reuse the existing stream parser shape — capture the `result.usage` for cost reporting, accumulate text deltas from `stream_event:content_block_delta:text_delta`. Don't emit `assistant://stream` events (this call is internal, not UI-visible).

B4. **Frontend hook.** New method on `AssistantStore`: `async summarizeCurrentSession(focus?: string): Promise<SummarizeResult | null>`. Doesn't mutate `messages`; just returns the result. Used by Phases C-E.

B5. **No threshold logic yet.** Settings row in Phase D wires this to a debug button so we can dry-run before automating.

**Cost sanity** (current Anthropic pricing per [platform.claude.com/docs/en/about-claude/pricing](https://platform.claude.com/docs/en/about-claude/pricing) — verified 2026-05-19):

| Model | Input $/M | Output $/M | 900K summarize (input + ~2K output) |
|---|---|---|---|
| Haiku 4.5 | $1 | $5 | **$0.91** |
| Sonnet 4.6 | $3 | $15 | $2.73 |
| Opus 4.7 | $5 | $25 | $4.55 |

Haiku is the obvious default — 3× cheaper than Sonnet, sufficient for prose summarization. Opus is over-budget and its newer tokenizer chunks text ~35% denser, so even more tokens for same input. No 1M-context surcharge on any model.

`--max-budget-usd 1.50` hard-caps the call (covers Haiku w/ ~60% headroom for tokenizer drift; rejects Sonnet runs that should be flagged before they fire).

**Verify:** add a `/summarize` debug slash command in `runSlash` that calls `summarizeCurrentSession()` and renders the result in `lastNotice`. Manual dev verification only — no shipped behavior.

---

### Phase C — Compaction action (~half day to full day)

**Goal:** a single user-triggerable button that compacts the current conversation: summarize → archive → mint fresh CLI session → seed first turn with summary.

C1. **New Rift convo schema.** Extend `ConversationRecord` w/:
   ```ts
   compactionHistory?: Array<{
     at: number;              // epoch ms
     turnIndex: number;       // first message index AFTER the boundary
     priorSessionId: string;  // the CLI uuid that was retired
     newSessionId: string;    // the CLI uuid that took over
     summary: string;
     summaryCostUsd: number;
     summaryModel: string;
   }>;
   ```
   Default `[]` for new convos; absent for legacy (handled as empty).

C2. **New synthetic message block + `"system"` role.** Add a `BoundaryBlock` to the `Block` union:
   ```ts
   type BoundaryBlock = { type: "boundary"; summary: string; at: number; archivedCount: number };
   ```
   Rendered as a collapsed "Conversation compacted · 47 turns archived" pill in `MessageBubble.svelte`, expandable to show the summary text. Boundary messages have role `"system"` (new — third role).

   **Blast radius — verified safe with 3 additive edits** (recon grep found 19 hits across 4 files; every existing check is positive equality or matched-role pop, so `"system"` falls through harmlessly):
   - **Type union widening** at [assistant.svelte.ts:63](src/lib/state/assistant.svelte.ts#L63): `"user" | "assistant"` → `"user" | "assistant" | "system"`. One line.
   - **CSS rule** for `[data-role="system"]` in `MessageBubble.svelte` (avatar background + text styling). Mirror the assistant-side rule at [:384](src/lib/components/assistant/MessageBubble.svelte#L384) with a distinct accent color so boundary pills are visually separable from regular Claude turns.
   - **Label conditional** at [MessageBubble.svelte:258](src/lib/components/assistant/MessageBubble.svelte#L258): `isUser ? "You" : isSystem ? "Compaction" : "Claude"`.

   Persistence is opaque (`serde_json::Value` at [mod.rs:255](src-tauri/src/assistant/mod.rs#L255)) — Rust round-trips any role string without validation, no backend changes needed. `retryLast` at [:1532-1533](src/lib/state/assistant.svelte.ts#L1532-L1533) won't pop a tail boundary message (correct — retry shouldn't delete compaction history). `deriveTitle` / `promptHistory` / cost-count filter on specific roles and harmlessly skip `"system"`.

C3. **`compactConversation()` flow** in `AssistantStore`:
   1. If `streaming` → abort: "Wait for current turn to finish."
   2. If `messages.length < 4` → abort: "Conversation too short to compact."
   3. Call `summarizeCurrentSession(focus)`.
   4. Mint `newSessionId = crypto.randomUUID()`.
   5. Backend call `assistant_remint_session(convoId, oldSessionId, newSessionId)` which: copies the cwd sidecar from old → new, leaves the old CLI JSONL on disk untouched (no destructive cleanup yet — Phase E can add an archival sweep).
   6. Push compaction record to `compactionHistory`.
   7. Replace `messages` with: `[boundary message, ...messages.slice(-3)]` (keep last 3 turns visible for continuity). The boundary message carries the summary. Full pre-boundary history is still on disk in the convo JSON — UI can render an "expand archived" affordance.
   8. Set `cliSessionId = newSessionId`, `convoCreatedAt = null` (so next send fires `--session-id`, not `--resume`), `resetUsage()`, `lastNotice = "Conversation compacted — $X spent, Y% → Z% context"`.
   9. Persist via `scheduleSave(flush=true)`.

C4. **New Tauri command `assistant_remint_session`** in `assistant/mod.rs`. Single responsibility: copy/move `~/.rift/assistant/sessions/<old-uuid>.cwd` → `<new-uuid>.cwd`. Keep both during a transition window (1 hour or one Rift restart, whichever first) so a frontend bug doesn't strand the user.

C5. **First send after compaction.** The new session starts fresh. The summary lives in the synthetic boundary message in the UI, not in any CLI-side state. We need to seed it. Approach validated by reading the existing reminder logic at [mod.rs:1130-1161](src-tauri/src/assistant/mod.rs#L1130-L1161): the `<system-reminder>` wrap fires whenever `reminder_parts` is non-empty on **every** turn including first, so the existing infra already handles first-turn-of-new-session correctly.

   **Implementation:** add a new `prior_context_summary: Option<String>` param to the `assistant_send` Tauri command. When present, the backend pushes it as a fourth entry into `reminder_parts` server-side (formatted as `"Prior conversation summary (compacted at <ts>):\n{summary}"`). Frontend tracks `pendingCompactionSummary: string | null` on the store, sets it during `compactConversation()`, passes it on the next `send()` invoke, clears it after the send returns. The summary stays attached to the *next* user message only — once that message lands, the new CLI session has it in its native context and the normal `--resume` chain takes over.

   **Why param over string-splicing:** keeps the boundary between Rift state and Claude state clean. Splicing would require the frontend to know about the `<system-reminder>` envelope format, which is currently a backend-only concern.

C6. **Manual trigger UI.** Add a "Compact" button to the ctx-pill area in `AssistantHeader.svelte`. Visible only when `ctxPct >= 50` (below that the action would be cost-negative). Confirm dialog: "Compact conversation? Spend ~$X · drop ctx from Y% → est Z%."

**Verify:** CDP-driven smoke. Fire 5 turns, click Compact, fire 1 more turn referring to a fact from turn 2 → confirm the new session has continuity via the seeded summary. Inspect `~/.claude/projects/<cwd-hash>/` for the new JSONL alongside the old.

---

### Phase D — Settings + auto-trigger (~3-4 hr)

D1. **Settings.svelte rows** in the Assistant section:
   - "Auto-compact threshold" — select: Off · 70% · 80% · 85% · 90%. Default Off (matches `DISABLE_AUTO_COMPACT=1` philosophy; user explicitly disabled the CLI's auto-compact so opt-in is correct). Aider triggers at ~70%; if the user opts in, 70% is the conventional recommended pick — surface as "(recommended)" tag on that option.
   - "Compact model" — haiku / sonnet radio. Default haiku ($0.91 vs $2.73 per 900K-token compact).
   - "Compact now" debug button — fires `compactConversation()` regardless of threshold.

D2. **Threshold effect** in `AssistantStore`:
   ```ts
   $effect(() => {
     if (!this.autoCompactThreshold) return;
     if (this.ctxPct < this.autoCompactThreshold * 100) return;
     if (this.streaming) return;
     if (this.compactingNow) return;
     if (Date.now() - this.lastCompactionAt < 5 * 60_000) return; // 5min cooldown
     void this.compactConversation();
   });
   ```
   The cooldown guards against runaway re-trigger if compaction itself fails (summarize errors → pill stays high → next render re-fires). 5min is conservative — successful compaction drops ctx well below threshold so the cooldown rarely matters in the happy path; it only matters on failure. A failed compaction at $0.91/attempt × runaway = real money, so erring long.

D3. **Pre-emption banner.** When `ctxPct >= threshold - 10pp` (5pp of warning), show a toast: "Approaching auto-compact at X%". Lets the user choose to compact early w/ a focus string if they want fine control.

**Verify:** CDP — set threshold to 70%, send turns until ctx hits 70%, confirm compaction fires once + cooldown holds. Inspect cost report.

---

### Phase E — Polish + archival (~half day, ship piecemeal)

E1. **Pre/post stats in boundary bubble.** "Ctx 87% → est 12% · $0.43 saved per future turn (assumed)". Comes from `lastTurnUsage` at compaction time vs. the seeded summary's token count from the SummarizeResult.

E2. **"Expand archived turns" affordance** in `MessageBubble.svelte` for boundary blocks. Reads the original `compactionHistory[N].priorSessionId` and pulls messages from a sidecar archive file `~/.rift/assistant/conversations/<convo>.archive.json` (new file — Phase C step 7 writes the archived slice here on compaction, instead of leaving the full array in the main convo JSON).

E3. **Compact-on-tab-close prompt.** When closing a tab w/ ctxPct > 50%, offer "Compact and keep, or just close?" before destroying tab state. Optional; user may find it annoying.

E4. **Old JSONL cleanup sweep.** Optional Rust-side housekeeping: every Rift startup, scan `~/.claude/projects/<cwd-hash>/` for `<uuid>.jsonl` files where `<uuid>` matches a known retired session in any `compactionHistory.priorSessionId` AND mtime > 30 days. Delete. (Conservative — user's CLI may store other unrelated sessions in the same dir.)

E5. **Conversation search by summary.** `HistoryDrawer.svelte` search currently matches on title only. Extend to also match `compactionHistory[*].summary` so long-running compacted convos remain searchable.

---

## Verification approach (all phases)

Same CDP pattern as `assistant-roadmap.md`: `npm run cdp:serve` once per dev session, then `bash scripts/cdp/c.sh state|eval|type|click|wait`. Each phase ships a smoke script in `scripts/cdp/`:

- `smoke-compact-A.sh` — confirms A2 decoupling doesn't regress tab persistence.
- `smoke-compact-B.sh` — `/summarize` debug command produces non-empty summary + cost.
- `smoke-compact-C.sh` — full compact + re-send + continuity assertion.
- `smoke-compact-D.sh` — auto-threshold fires once, cooldown holds.
- `smoke-compact-E.sh` — archived-turns expansion renders + search hits compacted convo.

Each smoke is the verification gate for its phase — don't ship without green.

---

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| GH #5034 duplicate JSONL on stream-json input | low | low | Confirmed closed-won't-fix; not on our hot path (we abandon JSONLs on remint, never parse them). If Phase E2 archived-view ever reads JSONL, dedupe by message UUID |
| Cwd sidecar drift after remint | low | high (session-lost loop) | Phase C4: copy not move; keep old for transition window |
| Summarize call fails mid-flight | medium | medium | Wrap compactConversation in try/catch; on fail, restore prior state + surface notice |
| Runaway auto-compact loop | low | high (cost burn) | Phase D2 cooldown + `compactingNow` guard |
| User loses critical detail from summary | medium | medium | Phase C: preserve last 3 turns verbatim + full archive readable via E2 |
| GH #36583 UUID collision on resume | low | medium | Phase C: always mint via `crypto.randomUUID()` (UUIDv4, 122 bits — collision negligible) |
| Auto-trigger fires mid-tool-streaming | medium | medium | Phase D2: explicit `streaming` guard before invoking |

---

## File anchors (so the next session doesn't grep)

Backend: [`src-tauri/src/assistant/mod.rs`](src-tauri/src/assistant/mod.rs) (1327L now; +200-300 estimated). Hot regions:
- `AssistantConfig` struct [:177](src-tauri/src/assistant/mod.rs#L177) (add fields)
- `session_cwd_path` / `save_session_cwd` / `load_session_cwd` [:276](src-tauri/src/assistant/mod.rs#L276) (clone for remint)
- `assistant_send` [:889](src-tauri/src/assistant/mod.rs#L889) (extend to splice summary into first post-compact prompt)
- Per-turn reminder construction [:1139](src-tauri/src/assistant/mod.rs#L1139) (extend w/ summary)

Frontend: [`src/lib/state/assistant.svelte.ts`](src/lib/state/assistant.svelte.ts) (1602L now; +200 estimated):
- `ConversationRecord` type [:81](src/lib/state/assistant.svelte.ts#L81) (extend w/ `cliSessionId` + `compactionHistory`)
- `Block` union [:59](src/lib/state/assistant.svelte.ts#L59) (add `BoundaryBlock`)
- `send()` [:840](src/lib/state/assistant.svelte.ts#L840) (split `currentConvoId` from `cliSessionId`)
- `recordTurnUsage` / `resetUsage` [:1223](src/lib/state/assistant.svelte.ts#L1223) (already correct)
- `onStream:result` subtype handling [:1359](src/lib/state/assistant.svelte.ts#L1359) (whitelist)
- `runSlash` [:1461](src/lib/state/assistant.svelte.ts#L1461) (add `/compact` + `/summarize` debug)

UI: [`AssistantHeader.svelte`](src/lib/components/assistant/AssistantHeader.svelte) (405L; +30):
- Ctx-pill region [:174](src/lib/components/assistant/AssistantHeader.svelte#L174) (add Compact button)

[`MessageBubble.svelte`](src/lib/components/assistant/MessageBubble.svelte) (565L; +60):
- New BoundaryBlock render branch

[`Settings.svelte`](src/lib/components/settings/Settings.svelte) (Assistant section ~line 510):
- Two new rows + one debug button

CDP smokes: [`scripts/cdp/`](scripts/cdp/) — five new shell scripts.

---

## Recommended sequence

A (probe + scaffolding) → B (summarize primitive) → C (manual compact) → D (auto-trigger) → E (polish). Each phase shippable as `v0.4.12-alpha` through `v0.4.16-alpha`. The probe in A1 is the single highest-leverage moment — if GH #5034 is biting us, Phase B/C designs shift.

Total estimate: 3-4 focused sessions to land A-D, plus piecemeal E. Phases B-D are the user-visible value; A is mandatory plumbing; E is nice-to-have.
