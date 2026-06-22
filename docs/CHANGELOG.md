# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.27.0 — Faster replies, a sidebar that stays put, and real split-pane multitasking

> A batch aimed squarely at the day-to-day annoyances: Sonnet feeling sluggish, the chat list reshuffling under your cursor, and split-view being half-finished. Each chat pane is now its own world — you can run two (or four) projects side by side at once.

- **Replies start fast on every model, especially Sonnet.** "Smart" (the default) used to make the model think hard before showing a single word, which on Sonnet looked like a frozen window for 10–20s. It now uses a balanced reasoning level that streams almost immediately — Anthropic's own recommended setting for interactive use. "Deep" still does the heavy thinking when you ask for it. Also removed an auto-downshift that was secretly forcing a ~1.7s cold restart on simple turns.
- **Your chat list stops jumping around.** Opening or clicking between conversations no longer reshuffles the sidebar — each chat holds a fixed position based on real activity, not on the bookkeeping save that fired every time you switched tabs. Spam-click through your chats all you want; the order stays put.
- **Split view, finished.** There's now a "Split editor" button in the top bar (it was keyboard-only before, `Ctrl+\`), panes no longer overlap or collide when you open three or four, and **each pane can run in a different project folder at the same time** — click the folder chip in a pane's header to point it anywhere. Two projects, side by side, each with its own conversation, context meter, and cost.
- **Cleaner work rows + calmer waiting.** Collapsed tool rows name the files (e.g. "Read a.ts, b.ts") and skip filler narration; when a turn is parked on *your* answer (a multiple-choice prompt) the footer says so calmly instead of running a fake "working…" clock.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.26.3** — Warm CLI process now idles for 30 minutes instead of 5, so it survives normal read-and-think pauses (prod log showed ~60% of turns were re-paying a ~1.7s cold start after the 5-minute eviction; real think-time runs to ~7.5 min at p90). Memory stays bounded — surplus idle processes are still reclaimed quickly.

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
