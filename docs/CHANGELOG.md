# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.29.0 — AI Health: a coach for your Claude plan

> New workspace tab (sidebar, shortcut **5**). Most apps show you a usage bar; AI Health *coaches* you. It reads how you actually use Claude through Rift, then asks **your own Claude** — your subscription, your private data — for a few plain-English ways to get more out of your plan. Built for newcomers: no jargon, no dashboards to decode, just "here's what to change and why."

- **See where it all goes.** Your plan limits (5-hour + weekly windows with reset times), all-time usage (conversations, messages, tool calls, est. spend, day streak, per-model split), and a live this-session readout — all on one screen, no scrolling.
- **"Analyze my usage" — advice from your own Claude, not a canned script.** One tap spawns a private, off-the-record reasoning pass over your setup + usage. It comes back with a few ranked suggestions (high/medium/low impact), each grounded in *your* actual numbers — not generic tips. Because it's your subscription doing the thinking, the advice adapts to your plan tier, your models, and how you work.
- **One-tap apply, fully undoable.** When a suggestion maps to a Rift setting (default effort, default model, per-turn spend cap), the card shows the exact before→after ("No cap → $2.00/turn") and an Apply button. Tap it and Rift tunes itself; an Undo sits right there if you change your mind. Behavior tips that don't map to a single switch just explain themselves. **Nothing changes until you choose** — and it only ever touches Rift's own settings, never your global Claude config.
- **Your current setup, at a glance.** A small panel shows the live knobs AI Health can tune, so the advice always has something concrete to point at.
- **A friendly wait.** The first analysis can take up to a minute (it warms up your Claude); an animated card walks you through what it's doing instead of leaving you staring at a spinner.

## v0.28.0 — Top-to-bottom hardening pass (quieter, safer, leak-free)

> No new features — this is a full front-to-back audit and fix release. A multi-wave, multi-agent review swept the whole app (145 confirmed findings across two waves) and every fix landed here: tighter security, fewer ways to leak memory over a long session, and a pile of subtle correctness bugs squashed. Nothing you do changes; it just holds up better the longer you run it.

- **Safer with untrusted content.** Page text pulled into a turn is now sanitized against prompt-injection (control-char/sentinel stripping), oversized pasted drafts are capped before they can bloat a request, and the browser/opener surface rejects a dozen more executable file types. Backend git and CLI plumbing strips verbose-trace env vars and runs with system git-config disabled, so nothing leaks through a subprocess.
- **Leak-free over long sessions.** Telemetry, transcript, and model-list buffers are now ring-capped instead of growing forever; stray timers and event listeners are torn down on unmount; and several reactive effects that could quietly re-run themselves were fixed. The longer Rift stays open, the more these matter.
- **Subtle correctness fixes.** A rare init race on rapid conversation switches, a queued-steer message that could reorder itself, a slash-menu Enter that double-fired, the stats panel's clock not advancing, voice-input restart edge cases, and more — all squashed. Update/repair toasts now say what actually happened, and stray terminal color codes no longer show up in error text.
- **Memory and speed on the hot path.** The streaming diff view recomputes only what changed when syntax highlighting warms up (instead of re-walking the whole diff), oversized tool and sub-agent results are truncated before rendering, and a per-frame diff recount is memoized.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.27.1** — Activity stats panel (messages/sessions/tools/spend, streaks, peak hour, 12-week heatmap, per-model breakdown, All/30d/7d toggle), drag a sidebar chat straight into a split pane, and removal of an unreachable old home screen.

- **v0.27.0** — Faster replies on every model (Smart → balanced reasoning that streams immediately, especially on Sonnet; retired the auto-downshift cold-restart), a sidebar that stops reshuffling on open/click, finished split-view (top-bar "Split editor" button, no overlap at 3–4 panes, per-pane project folders), cleaner collapsed tool rows + a calm "waiting for you" footer on ask_user.

- **v0.26.3** — Warm CLI process now idles for 30 minutes instead of 5, so it survives normal read-and-think pauses (prod log showed ~60% of turns were re-paying a ~1.7s cold start after the 5-minute eviction; real think-time runs to ~7.5 min at p90). Memory stays bounded — surplus idle processes are still reclaimed quickly.

- **v0.26.2** — Honest feedback when the API stalls (it's not Rift): a "Rift got slow" report was a single 138s Anthropic API stall on one request, not a Rift regression (Rift overhead ~1s median, ~5ms warm; one turn of 38 hit 138s with zero Rift activity in the gap). Live stall watchdog (>20s/>60s with no token + no tool → footer says the API is slow, not Rift, wait or Stop) + de-latched egregious-stall alert so it re-fires.

- **v0.26.1** — `ask_user` questions render as a real interactive card in stream mode (options/multi-select/freeform/Submit — the chip used to appear and park forever with no buttons; the v0.24 stream mode never had an `ask_user` case) + no more "Empty pane" homepage flash on launch (single-pane null-tab goes straight to the home hero). Both CDP-verified end-to-end.

- **v0.26.0** — Stop now always clears a hung "Calling ask_user…" prompt (the warm-pool deadlock: the session PID could be cleared before Stop, so the parked question never released — Stop now cancels any pending `ask_user` for the session first); interactive + colored context ring with a "this conversation · context" row in the usage panel; sub-agent activity dock visual overhaul; accent reset to emerald.
- **v0.25.0** — Warm CLI process: persistent child per session reused across turns (first reply ~1400ms → every reply after ~5ms), transparent respawn on death/idle/setting-change; steering image attachments forwarded end-to-end; interactive prompts resolve from any tab; EditDiff green-bar flicker fixed; per-tool icons/captions/tints; ctx gauge → composer ring; duplicate-"Thinking…" + footer baseline fixes. #48 + #49 closed.
- **v0.24.0** — Stream design-language pass (accent = meaningful-signal-only; idle blocks neutral/boxless with left-rule; calm icon badges + markdown code-chips) + live-turn polish (animated token odometer, file-path crumbs, inline EditDiff).
- **v0.23.0** — Latency auto-scale (trivial turns downshift `smart→quick`) + turn-phase probe + live-status pill fix + round-12 `src/lib/state/` sweep (6 fixes: STT corrupt-resume guard, mic over-rail, split-pane scrub, toast timer leak).
- **v0.20.7–v0.22.0** — Earlier hardening arc (rounds 5–11, ~70 fixes total) + the full redesign port (all 7 surfaces rebuilt to spec) + per-project chat scoping + recovery tools. Detail via `git log`.
