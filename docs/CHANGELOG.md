# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.26.0 — Stop unblocks a stuck ask_user + composer/dock UI polish

> The fix that matters: **hitting Stop now always works**, even when an interactive `ask_user` prompt is hung. Plus a round of composer + sub-agent-dock polish. `npm run check` 0/0 (4107) · backend cargo-clean (dev rebuild `Finished` 0 errors) · live CDP (emerald accent verified, 0 console errors).

- **Stop now clears a hung "Calling ask_user…" prompt (the deadlock fix).** When the agent asked you a question and the prompt sat spinning, pressing Stop did nothing and the whole turn was wedged forever. Root cause: the warm-CLI-process change (v0.25.0) meant the session's process id could already be cleared by the time you hit Stop, so `assistant_stop` had nothing to kill and returned without releasing the parked question — the underlying tool call sat on its 10-minute timeout. Stop now **cancels any pending `ask_user` for the session first**, before it looks for the process: the parked tool call resolves immediately, the spinner clears, and the turn unblocks every time — whether or not the process is still alive.
- **Context ring is now interactive + colored.** The small ring by the model pill (added v0.25.0) was a flat gray dot. It's now a real button in the accent colour (emerald) — click it to open the usage panel, which now shows **this conversation's context fill** (track + token count) right above your plan's rate-limit rows, so the `/usage` info and the live context gauge live in one place.
- **Sub-agent activity dock — visual overhaul.** The dock that appears when the agent spawns sub-agents was restyled to match the rest of Rift: an accent badge in the header (glows while agents are live), each agent rendered as its own elevated card (running cards get an accent border + glow, errored cards a danger border), and a real progress bar with a step count when the dock is collapsed.
- **Accent reset to emerald.** A stale saved accent hue could tint the composer + steer rail purple; the default is back to emerald (163). (If your installed app still looks purple, reset the accent in Settings → Appearance.)

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.25.0** — Warm CLI process: persistent child per session reused across turns (first reply ~1400ms → every reply after ~5ms), transparent respawn on death/idle/setting-change; steering image attachments forwarded end-to-end; interactive prompts resolve from any tab; EditDiff green-bar flicker fixed; per-tool icons/captions/tints; ctx gauge → composer ring; duplicate-"Thinking…" + footer baseline fixes. #48 + #49 closed.
- **v0.24.0** — Stream design-language pass (accent = meaningful-signal-only; idle blocks neutral/boxless with left-rule; calm icon badges + markdown code-chips) + live-turn polish (animated token odometer, file-path crumbs, inline EditDiff).
- **v0.23.0** — Latency auto-scale (trivial turns downshift `smart→quick`) + turn-phase probe + live-status pill fix + round-12 `src/lib/state/` sweep (6 fixes: STT corrupt-resume guard, mic over-rail, split-pane scrub, toast timer leak).
- **v0.22.0** — Rounds 10–11 hardening (27 fixes across under-swept surfaces; skeptic-verify multi-agent rounds).
- **v0.21.0** — Rounds 5–9 hardening (1 critical + 42 fixes): config race · IPC/update/browser · secrets/commands/state/frontend · diagnostics/UI-resilience · turn-lifecycle/git-DoS/streaming/composer.
- **v0.20.9** — Recovery tools + per-project chat scoping + 4-round hardening pass.
- **v0.20.8** — Overnight efficiency + cleanup polish (stress-test pass on v0.20.7).
- **v0.20.7** — Full redesign port (all 7 surfaces rebuilt to spec, CDP-verified) + backend dead-code sweep.
