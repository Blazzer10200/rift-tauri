# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.26.3 — Warm process stays warm through your think-time (the real "feels slow sometimes" fix)

> The warm CLI process from v0.25.0 makes replies near-instant *within* a session — but it was quietly giving up after 5 minutes of quiet and re-paying the ~1.7s cold start on your next message. Mining the prod log made it obvious: the warm pool is reused in 3–18ms, but ~60% of turns were paying a cold respawn, and every one of them followed an idle-eviction. Your real pause-between-messages runs to ~7.5 minutes at the 90th percentile (median ~90s), so a 5-minute timer was aging the process out right in the middle of normal reading-and-thinking. This raises the idle window so the process survives the way it does in VSCode. `cargo check` 0/0.

- **The warm process now idles for 30 minutes, not 5.** You can read a reply, think, go make coffee, and come back to an instant next reply instead of a one-and-a-half-second cold start. Tuned against real session data — 30 minutes clears the 90th-percentile think-time with room to spare, so an active session effectively never evicts mid-flow.
- **Memory stays bounded.** If several sessions pile up warm, the oldest idle ones are still reclaimed quickly (each warm process is ~450MB), so the long window never leaks. The common one-or-two-session case keeps the full 30 minutes.
- **Not new in v0.26.x — it was there since the warm pool shipped.** The display side was checked too and left alone: token streaming is already smooth (paced on the frontend), and the occasional slow *first* token is the model thinking, not Rift.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.26.2** — Honest feedback when the API stalls (it's not Rift): a "Rift got slow" report was a single 138s Anthropic API stall on one request, not a Rift regression (Rift overhead ~1s median, ~5ms warm; one turn of 38 hit 138s with zero Rift activity in the gap). Live stall watchdog (>20s/>60s with no token + no tool → footer says the API is slow, not Rift, wait or Stop) + de-latched egregious-stall alert so it re-fires.

- **v0.26.1** — `ask_user` questions render as a real interactive card in stream mode (options/multi-select/freeform/Submit — the chip used to appear and park forever with no buttons; the v0.24 stream mode never had an `ask_user` case) + no more "Empty pane" homepage flash on launch (single-pane null-tab goes straight to the home hero). Both CDP-verified end-to-end.

- **v0.26.0** — Stop now always clears a hung "Calling ask_user…" prompt (the warm-pool deadlock: the session PID could be cleared before Stop, so the parked question never released — Stop now cancels any pending `ask_user` for the session first); interactive + colored context ring with a "this conversation · context" row in the usage panel; sub-agent activity dock visual overhaul; accent reset to emerald.
- **v0.25.0** — Warm CLI process: persistent child per session reused across turns (first reply ~1400ms → every reply after ~5ms), transparent respawn on death/idle/setting-change; steering image attachments forwarded end-to-end; interactive prompts resolve from any tab; EditDiff green-bar flicker fixed; per-tool icons/captions/tints; ctx gauge → composer ring; duplicate-"Thinking…" + footer baseline fixes. #48 + #49 closed.
- **v0.24.0** — Stream design-language pass (accent = meaningful-signal-only; idle blocks neutral/boxless with left-rule; calm icon badges + markdown code-chips) + live-turn polish (animated token odometer, file-path crumbs, inline EditDiff).
- **v0.23.0** — Latency auto-scale (trivial turns downshift `smart→quick`) + turn-phase probe + live-status pill fix + round-12 `src/lib/state/` sweep (6 fixes: STT corrupt-resume guard, mic over-rail, split-pane scrub, toast timer leak).
- **v0.22.0** — Rounds 10–11 hardening (27 fixes across under-swept surfaces; skeptic-verify multi-agent rounds).
- **v0.21.0** — Rounds 5–9 hardening (1 critical + 42 fixes): config race · IPC/update/browser · secrets/commands/state/frontend · diagnostics/UI-resilience · turn-lifecycle/git-DoS/streaming/composer.
- **v0.20.9** — Recovery tools + per-project chat scoping + 4-round hardening pass.
- **v0.20.8** — Overnight efficiency + cleanup polish (stress-test pass on v0.20.7).
- **v0.20.7** — Full redesign port (all 7 surfaces rebuilt to spec, CDP-verified) + backend dead-code sweep.
