# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.23.0 — Latency auto-scale + live-pill fix + round-12 hardening (6 fixes)

> Per-turn effort auto-scale (trivial greetings stop paying full reasoning latency), the live-status pill rendered correctly above the stream, a turn-phase probe to attribute slow turns, plus a full state-layer adversarial sweep (every `src/lib/state/` file audited; the 3 STT + 3 frontend bugs below were the real findings). Verified: `npm run check` 0 errors / 0 warnings (4105 files) · vitest 214/214 · `cargo check` clean (dev binary down — no lock collision).

**Latency**
- **Auto-scale effort on trivial turns (#244)** — the default `smart` tier maps to `--effort high`, so the model burns heavy hidden reasoning before any visible token (~10s TTFT on Sonnet for "Hello", worse on Opus). `autoScaleEffort` trims the *per-send* tier `smart → quick` for clearly-trivial prompts only (short, attachment-free greetings/acks, no question/code/imperative verb). Only ever downshifts; `deep`/`ultra` pass through verbatim; the stored choice is never mutated. Wired into the telemetry record + CLI spawn flag.
- **Turn-phase probe (#244)** — `turn.rs` logs first-thinking-delta + first-text-delta vs TTFT, so a slow turn is attributable (big gap = invisible Opus reasoning; small = spawn/prefill/gen). Plus a spawn-time `effort tier=… flag=… model=…` line.

**UI**
- **Live-status pill renders above the stream, never frozen** — the footer verb only showed while a tool was in flight (`{#if streaming && liveTool}`), so a turn thinking before its first tool call read as "frozen" and the misplaced "Conjuring…" pill rendered below the composer. Now the footer shows whenever `streaming`, cycling a whimsical present-participle (~2.4s) with no live tool and the real verb ("Reading X") with one — mirroring the DS `StreamFooter`.

**Hardened (round 12 — full `src/lib/state/` sweep; every file audited inline)**
- **STT model resume can't promote a corrupt file (HIGH)** — `model_manager.rs` renamed `.partial → final` *before* the sha256 verify on the resumed path; `known_models()` trusts presence+size (no hash check), so a crash/concurrent call served a corrupt GGML as "complete". Now verifies + quarantines to `.badhash` *before* rename — mirroring the fresh + 416-resume paths (every `final_path` promotion now gated).
- **Mic samples no longer over-rail (MED)** — i16 callback divided by `i16::MAX` (32767); the range is asymmetric (−32768..=32767) so the min sample mapped to −1.00003, past the ±1.0 rail. Now `÷32768`. u16 callback used `(s−32767−1)/32767`, over-railing both ends → now `(s−32768)/32768`.
- **Split panes scrubbed on close-others / close-to-right (MED)** — `closeOtherTabs`/`closeTabsToRight` dropped tabs + pruned UI but never called `scrubTabFromPanes`, so a split pane kept a dangling pointer to a dropped TabState. Now both scrub (mirroring `closeTab`/`closeAllTabs`).
- **Toast `clear()` no longer leaks paused-timer state (LOW)** — cleared `timers`+`deadlines` but left `remaining` entries for paused toasts, accumulating every `clear()`. Now `remaining.clear()` too.

## v0.22.0 — Rounds 10–11 hardening (27 fixes across under-swept surfaces)

> Two more multi-agent adversarial rounds on top of v0.21.0 (review-dimension → per-finding skeptic verify, default isReal=false). R10: 31 raised → 27 confirmed / 4 rejected, 22 fixed. R11: 11 raised → 5 confirmed / 6 rejected, 5 fixed. Verified each round: cargo check clean (isolated target) · svelte-check 0/0 (4105) · playback 33/33.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.21.0** — Rounds 5–9 hardening (1 critical + 42 fixes): config race · IPC/update/browser · secrets/commands/state/frontend · diagnostics/UI-resilience · turn-lifecycle/git-DoS/streaming/composer.
- **v0.20.9** — Recovery tools + per-project chat scoping + 4-round hardening pass.
- **v0.20.8** — Overnight efficiency + cleanup polish (stress-test pass on v0.20.7).
- **v0.20.7** — Full redesign port (all 7 surfaces rebuilt to spec, CDP-verified) + backend dead-code sweep.
