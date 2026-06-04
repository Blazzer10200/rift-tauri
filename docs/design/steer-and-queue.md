# Steer & Queue — mid-turn intervention on a running turn

> Status: **DESIGN, not implemented (2026-06-04)**. Core feasibility
> **PROVEN** — a standalone probe injected a second `user` envelope mid-turn
> into the user's real `claude` CLI; the agent abandoned its in-flight tool
> loop and obeyed the injected instruction within a **single `result` frame**
> (one turn, no restart). Confirmed against Anthropic's official Streaming
> Input docs + the claude-agent-sdk-python interactive-streaming reference.
> Confidence ~97%. Scope this session: doc only (a dev/CDP session was live, so
> no `cargo check`). Implementation deferred to a dedicated session.
>
> **All code anchors + behavioral claims below verified against the tree
> 2026-06-04** (not memory-drafted). Verification-pass corrections: registry is
> `Mutex<Option<HashMap>>` not `DashMap` (§4.1); steer trigger is **Alt+Enter**
> not Shift+Enter — Shift+Enter is the existing newline (§5.1); queue is
> confirmed not-persisted (§6.5); `select!` serializes stdin writes so no
> frame-split risk (§4.4).
>
> Goal: give the running turn a graceful middle gear between "wait for it to
> finish" (queue) and "kill it" (`/stop`). Steering injects a course-correction
> the agent reads at its next step boundary, keeping all in-flight work.

---

## 1. The problem — the queue is the only redirect tool, and it's the wrong one

Today every "I want to change what it's doing" funnels into two extremes:

- **Queue** ([assistant.svelte.ts:2108](../../src/lib/state/assistant.svelte.ts) `drainQueue`) — the
  message waits for the `result` frame, then fires as a **brand-new turn**. The
  agent finishes whatever it's doing — even if now wrong — before it reads you.
- **Stop** (`/stop`) — hard-kills the turn. Nukes in-flight work.

There is no graceful middle. That is *why* the queue feels neglected: it is the
only tool for "adjust course" and it is structurally incapable of doing it
promptly. The fix is not a better queue — it is a **third tier** that the queue
was never meant to cover.

---

## 2. The three tiers

| Tier | Wire op | What it does | When |
|---|---|---|---|
| **Steer** (new) | write `user` envelope to the **live** stdin | agent course-corrects at its next loop step, keeps all work | "actually use the other API", "don't touch that file", "yes, all of them" |
| **Queue** (keep) | write `user` envelope **after** `result` | fires as a fresh turn | "next, also do X" — additive follow-ups |
| **Interrupt** (keep, upgradable) | `control_request` interrupt, or process kill | hard stop | it's going completely wrong |

Steer and Queue are the **same wire operation** — a `user` envelope written to
the CLI's stdin. They differ only in **when** Rift sends it: now (into the
running turn) vs. on `DONE` (as the next turn). That symmetry is what makes this
cheap to build on top of the existing turn machinery.

---

## 3. Why this works — the plumbing already exists

The main turn ([mod.rs:2364](../../src-tauri/src/assistant/mod.rs) `assistant_send`) already:

- spawns the CLI with `--input-format stream-json` + piped stdin ([mod.rs:2547,2565](../../src-tauri/src/assistant/mod.rs)),
- **holds stdin open for the whole turn** — the reader task owns it and only
  drops it (EOF) on the `result` frame ([mod.rs:2783](../../src-tauri/src/assistant/mod.rs)),
- already **writes back into that live stdin mid-stream** every time it answers
  a `can_use_tool` permission ask ([mod.rs:2886-2888](../../src-tauri/src/assistant/mod.rs) `handle_permission_request`).

Steering reuses that exact write path. We are not adding a new capability — we
are stopping the queue from throwing an existing one away.

### Proof (probe, 2026-06-04)

A throwaway Node script spawned `claude` with the same args, wrote the
`initialize` handshake, sent a 5-step Bash echo task, then injected a second
`user` envelope (*"stop, reply PIVOTED"*) right after the first `tool_use`
landed. Result, verbatim:

```
TOOL  Bash {"command":"echo step-1"...}   ← agent starts the loop
OUT   >>> STEER injected after first tool_use <<<
ASST  "PIVOTED"                            ← next step obeys, steps 2-5 dropped
DONE  success | "PIVOTED"                  ← ONE result frame, same turn
```

One `result` frame containing `PIVOTED` — not a result for the echo task
followed by a second turn. The injected message was consumed **within the
running turn at the next step boundary**. (Probe removed after the run.)

### Docs corroboration

- Official [Streaming Input](https://code.claude.com/docs/en/agent-sdk/streaming-vs-single-mode):
  streaming-input mode's benefits explicitly include *"Queued Messages — send
  multiple messages that process sequentially, with ability to interrupt."*
  One-shot (`-p`) mode explicitly does **not** support dynamic queueing or
  real-time interruption. Rift is on the streaming path → on the right side.
- [claude-agent-sdk-python interactive-streaming](https://deepwiki.com/anthropics/claude-agent-sdk-python/7.1-interactive-streaming):
  the control channel requires the reader to be **actively consuming stdout** —
  Rift's reader task always is, so the precondition is met.
- "Sequentially" (docs) vs. "mid-turn" (probe) reconcile cleanly: the envelope
  lands at the **next agent-loop step**, not instantly mid-step — exactly how
  Claude Code's own steering feels.

---

## 4. Backend map

Three changes, all in `assistant/`.

### 4.1 Steer registry
A per-session sender, mirroring the **exact** existing registry convention
([mod.rs:46-105](../../src-tauri/src/assistant/mod.rs)) — a const-initialized
`Mutex<Option<HashMap>>` with a poison-recovering `with_*` helper and thin
accessor fns. **Not** `DashMap`/`Lazy` (neither is used in this file):
```rust
static STEER_TX: Mutex<Option<HashMap<String, mpsc::UnboundedSender<SteerMsg>>>> = Mutex::new(None);
struct SteerMsg { text: String, attachments: Vec<Attachment> }

fn with_steer_tx<R>(f: impl FnOnce(&mut HashMap<String, mpsc::UnboundedSender<SteerMsg>>) -> R) -> Option<R> {
    // identical poison-recovery shape to with_session_pids (mod.rs:60)
}
fn register_steer_tx(session_id: &str, tx: mpsc::UnboundedSender<SteerMsg>) { with_steer_tx(|m| { m.insert(session_id.into(), tx); }); }
fn clear_steer_tx(session_id: &str) { with_steer_tx(|m| { m.remove(session_id); }); }
fn get_steer_tx(session_id: &str) -> Option<mpsc::UnboundedSender<SteerMsg>> { with_steer_tx(|m| m.get(session_id).cloned()).flatten() }
```
Create the `unbounded_channel()` in `assistant_send` before the reader task
spawns; `register_steer_tx` the `tx` (keyed by `session_id`), move the `rx` into
the reader task. `clear_steer_tx` on `result`/DONE so a late steer can't write to
a dead stdin — mirror the spot where `clear_session_pid` already runs.

### 4.2 New command `assistant_steer(session_id, text)`
Looks up the sender, pushes a `SteerMsg`. Returns an enum so the frontend knows
what happened: `Steered` | `NoActiveTurn` (→ frontend falls back to queue) .
Register in [lib.rs](../../src-tauri/src/lib.rs) alongside `assistant_send`.

### 4.3 Reader-task `select!` refactor (the real work)
The reader loop ([mod.rs:2856](../../src-tauri/src/assistant/mod.rs)) is today a plain
`loop { lines.next_line().await }`. It becomes a `tokio::select!` over:
- `lines.next_line()` — unchanged stdout handling, and
- `steer_rx.recv()` — on a `SteerMsg`, build the **same envelope shape** as
  `user_line` ([mod.rs:2798](../../src-tauri/src/assistant/mod.rs)) and `write_all` + flush to stdin.

Envelope (add `parent_tool_use_id` for SDK forward-compat):
```json
{"type":"user","parent_tool_use_id":null,
 "message":{"role":"user","content":[{"type":"text","text":"<steer>"}]}}
```

### 4.4 Edge cases (must-spec)
| Case | Handling |
|---|---|
| Steer before `initialize` ack (`user_sent == false`) | buffer the `SteerMsg`; flush after the first user turn is sent |
| Steer after `result` lands | sender already removed → command returns `NoActiveTurn` → frontend queues it as a new turn |
| Steer while a `can_use_tool` ask is pending | The reader task owns stdin exclusively, so `select!` **serializes** every write — a steer and a permission `control_response` cannot interleave or split a frame. Only the *ordering* is a choice: a steer that wins the `select!` race lands before the pending permission response, which is harmless (both are valid stdin frames, consumed in order). No locking needed. |
| Background child (dev server) keeps `claude` alive past `result` | already handled — DONE fires on `result`, not exit ([mod.rs:2894](../../src-tauri/src/assistant/mod.rs)); steer registry is keyed off the same `result` |

---

## 5. Frontend map

### 5.1 Trigger — modifier key (decided: **Alt+Enter**)
**Not Shift+Enter.** The key handler is `if (e.key === "Enter" && !e.shiftKey)
→ fire()` ([Composer.svelte:789](../../src/lib/components/assistant/Composer.svelte)),
so **Shift+Enter already inserts a newline** (default textarea behavior).
Binding steer to Shift+Enter would break multiline input. Use a free modifier:

- **Enter** = send / queue (unchanged — `fire()` at Composer.svelte:795),
- **Shift+Enter** = newline (unchanged),
- **Alt+Enter** = steer the running turn (Ctrl/Cmd+Enter also free if preferred).

Add a branch in `onKey` ([Composer.svelte:648](../../src/lib/components/assistant/Composer.svelte)),
*before* the `!e.shiftKey` send branch:
```ts
if (e.key === "Enter" && e.altKey && streaming && draft.trim().length > 0) {
  e.preventDefault();
  steer();          // sibling of fire(); calls assistant.steer(tabId, draft)
  return;
}
```
The button `mode` derived ([Composer.svelte:811](../../src/lib/components/assistant/Composer.svelte))
stays `send | stop | queue` — steer is keyboard-only in v1. A split send-button
("Steer" / "Queue") while streaming is a follow-up, not v1.

Update the streaming placeholder (*"Type to queue — Enter sends, /stop halts"* at
[Composer.svelte:1257](../../src/lib/components/assistant/Composer.svelte)) to
advertise **Alt+Enter to steer**.

### 5.2 State
- New `assistant.steer(tabId, text)` in
  [assistant.svelte.ts](../../src/lib/state/assistant.svelte.ts): a sibling of
  `send()`, it `await invoke("assistant_steer", { sessionId, text })` — mirror
  the existing `invoke("assistant_send", …)` (assistant.svelte.ts:1975) and
  `invoke("assistant_stop", { sessionId })` (assistant.svelte.ts:2151) calls.
  It **bypasses `tab.queue`** entirely (unlike `send()`, whose streaming branch
  enqueues at assistant.svelte.ts:1850-1852).
- `sessionId` must be the **target tab's** `cliSessionId`, not the active tab's
  (see §8 multi-tab).
- On `NoActiveTurn` return → fall through to the enqueue path so a steer that
  just missed the `result` isn't lost (push onto `tab.queue` like
  assistant.svelte.ts:1851).

### 5.3 Transcript + persistence
- Render steered text as a distinct inline `↳ steered` user bubble so history
  shows the redirect (vs. a normal turn-start bubble).
- The CLI records the injected message in its own session transcript, so it
  survives `--resume`; Rift's mirror (telemetry.ts / persistence.ts) must
  capture it too, or a reloaded session will show the agent reacting to a
  message that isn't in Rift's copy.

---

## 6. Queue improvements (independent of steer)

The queue stays — for additive follow-ups it's the right tool. Neglected-debt
items, in rough priority:

1. **Per-item fire mode** — at drain time, choose "fire as new turn" (today) vs.
   "fire as steer into the *next* turn's first step". Unifies the two systems.
2. **Edit-before-fire** — edit a queued item's text before it drains.
3. **Reorder** — drag queued items (`tab.queue` is an ordered array already).
4. **Collapse** — merge multiple queued items into one turn.
5. **Persist across restart** — confirmed **not persisted**: `host.queue` is
   reset to `[]` on every conversation load
   ([persistence.ts:261](../../src/lib/state/assistant/persistence.ts)), and the
   serialized `queue` field (persistence.ts:64) is never restored. So a
   reload/convo-switch strands queued work today. Persisting it (write on
   enqueue/drain, restore on load) is the fix.

---

## 7. Tier 3 upgrade (future, optional)

`/stop` today is a process kill. The SDK exposes a graceful `interrupt()`
control request (same channel as permissions) that finalizes the turn with a
proper `result` instead of severing it. Upgrading `/stop` to that would make
interrupted turns resumable and clean.

**Gotcha if we do** ([typescript-sdk #338](https://github.com/anthropics/claude-agent-sdk-typescript/issues/338)):
`interrupt()` can emit a **truncated** assistant message that is structurally
indistinguishable from a completed one — consumers can't tell it was cut off.
If we wire graceful interrupt, tag the turn as interrupted on our side rather
than trusting the message shape. **Not relevant to steer** (steer never
interrupts the model stream) — noted here so the future work doesn't trip on it.

---

## 8. Open questions

- **Steer latency** — lands at the *next* step boundary, which if the model has
  already dispatched its next API call could be the step-after-next. Acceptable
  (matches CC), but worth a transient "steering…" pill so the user knows it
  registered before it visibly takes effect.
- **Multi-tab** — steer is per-`session_id`; a steer typed in a background tab
  must route to that tab's session, not the active one. The registry is keyed by
  `session_id` so this is free, but the Composer must pass the *tab's* session,
  not `activeTab`.
- **Empty-draft Alt+Enter while streaming** — no-op, or treat as a "nudge"
  (inject a generic "continue"/"status?")? Default: no-op (the `onKey` branch
  already guards `draft.trim().length > 0`).
