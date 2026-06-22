# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.26.1 — Interactive questions actually render + no homepage flash on launch

> Two long-standing annoyances, fixed at the root and verified live over CDP (the full ask → select → submit → answer cycle, and a 30-sample boot capture with zero flash). `npm run check` 0/0 (4108).

- **`ask_user` questions now render as a real interactive card (the big one).** When the agent asked you a multiple-choice question, the chip would appear and just *sit there* — no options, no buttons, nothing to click, the turn parked forever. Root cause: the v0.24 STREAM rendering mode (now the default) never had an `ask_user` case, so the question silently fell through to a dead one-line status row. The data pipeline was healthy the whole time — the card simply was never drawn in stream mode. There's now a dedicated interactive question card in stream mode (options, multi-select, "Other" freeform, Submit/Dismiss) that binds to the live request and returns your answer to the agent. End-to-end confirmed: select → Submit → the agent receives `A: <your choice>` and continues.
- **No more homepage layout flash on launch/refresh.** After the intro screen, a stray "Empty pane" card used to flash for a beat before the real home appeared. In single-pane mode (the default) a not-yet-loaded conversation now goes straight to the home hero — the "Empty pane" drag-a-tab card only shows in split view where it actually means something. The home surface paints correctly from the first frame.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.26.0** — Stop now always clears a hung "Calling ask_user…" prompt (the warm-pool deadlock: the session PID could be cleared before Stop, so the parked question never released — Stop now cancels any pending `ask_user` for the session first); interactive + colored context ring with a "this conversation · context" row in the usage panel; sub-agent activity dock visual overhaul; accent reset to emerald.
- **v0.25.0** — Warm CLI process: persistent child per session reused across turns (first reply ~1400ms → every reply after ~5ms), transparent respawn on death/idle/setting-change; steering image attachments forwarded end-to-end; interactive prompts resolve from any tab; EditDiff green-bar flicker fixed; per-tool icons/captions/tints; ctx gauge → composer ring; duplicate-"Thinking…" + footer baseline fixes. #48 + #49 closed.
- **v0.24.0** — Stream design-language pass (accent = meaningful-signal-only; idle blocks neutral/boxless with left-rule; calm icon badges + markdown code-chips) + live-turn polish (animated token odometer, file-path crumbs, inline EditDiff).
- **v0.23.0** — Latency auto-scale (trivial turns downshift `smart→quick`) + turn-phase probe + live-status pill fix + round-12 `src/lib/state/` sweep (6 fixes: STT corrupt-resume guard, mic over-rail, split-pane scrub, toast timer leak).
- **v0.22.0** — Rounds 10–11 hardening (27 fixes across under-swept surfaces; skeptic-verify multi-agent rounds).
- **v0.21.0** — Rounds 5–9 hardening (1 critical + 42 fixes): config race · IPC/update/browser · secrets/commands/state/frontend · diagnostics/UI-resilience · turn-lifecycle/git-DoS/streaming/composer.
- **v0.20.9** — Recovery tools + per-project chat scoping + 4-round hardening pass.
- **v0.20.8** — Overnight efficiency + cleanup polish (stress-test pass on v0.20.7).
- **v0.20.7** — Full redesign port (all 7 surfaces rebuilt to spec, CDP-verified) + backend dead-code sweep.
