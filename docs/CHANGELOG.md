# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.27.1 — Activity stats, drag-a-chat-into-a-pane, and a dead-code cleanup

> A small follow-up to v0.27.0. Two features that were built but never wired up are now live, plus an unreachable old home screen got removed.

- **See your activity at a glance.** A new **Activity** button on the home welcome opens a stats panel — total messages, sessions, tool calls, and spend, a current/best streak, your peak hour, a 12-week activity heatmap, and a per-model breakdown. Toggle between all-time, last 30 days, and last 7 days.
- **Drag a conversation straight into a split pane.** Grab any chat in the sidebar and drop it onto the editor to open it in a split — drop on the left or right half to choose the side. The drop target already existed; now there's something to drag into it.
- **Removed a leftover home screen** that was no longer reachable (the "Home is a verb" redesign routes Home to a fresh empty chat). No visible change — the Home button works exactly as before.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.27.0** — Faster replies on every model (Smart → balanced reasoning that streams immediately, especially on Sonnet; retired the auto-downshift cold-restart), a sidebar that stops reshuffling on open/click, finished split-view (top-bar "Split editor" button, no overlap at 3–4 panes, per-pane project folders), cleaner collapsed tool rows + a calm "waiting for you" footer on ask_user.

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
