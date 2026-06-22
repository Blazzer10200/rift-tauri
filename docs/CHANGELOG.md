# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.26.2 — Honest feedback when the API stalls (it's not Rift)

> A "Rift got slow" report turned out to be a single 138-second Anthropic API stall on one request — not a Rift regression. Proven from the prod log: Rift's own per-turn overhead is ~1s median (and ~5ms on a warm-pool reuse), the model's first token is ~4s median, and exactly one turn out of 38 hit 138s with zero Rift activity during the gap. The plumbing is fast; the API occasionally isn't. This release makes that legible instead of looking like a freeze. `npm run check` 0/0 (4108).

- **Live stall watchdog on a turn.** When a turn is running but nothing has come back yet — no tool in flight, no tokens — and it crosses ~20s (then ~60s), the footer stops the whimsical "Unfurling…" shimmer (which implies local progress) and tells you the truth: the model is slow to respond right now, this is the Anthropic API and not Rift, and you can keep waiting or press Stop. Normal turns (first token ~4s) never see it.
- **The "slow turn start" alert no longer self-silences on a bad one.** A genuinely egregious stall (>30s before any output) now re-surfaces every time it happens and names the API as the cause, instead of being muted after the first mild occurrence in a session.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

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
