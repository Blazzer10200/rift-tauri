# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — composer ctx ring + live-turn fixes + tool-interaction fixes

> Follow-up polish on v0.24.0 (committed, not yet shipped). Pure frontend. `npm run check` 0/0 (4107) · vitest 222/222.

- **Faster replies (less waiting before the first words)** — every message used to spin up a brand-new assistant process from scratch, which re-ran a pile of developer-machine startup work (memory loading, background prefetches, config auto-discovery) before the model could even begin. That startup work is now scoped out of each turn, so the model starts sooner. (The deeper win — keeping the assistant process warm between messages, the way the editor extension does — is tracked as a follow-up; this is the safe first cut.)
- **Interactive prompts now work from any tab** — the "Ask me a question" card and the Allow/Deny permission bar were permanently disabled (stuck on "Connecting to the chat session…") whenever the turn that raised them wasn't in the foreground tab. They now resolve through the conversation that actually owns the request, so the buttons are live regardless of which tab/pane you're looking at. (This is why an ask_user prompt could appear to "never pop up.")
- **No more flickering green bar in edit diffs** — the green "added line" marker on the left of code edits could blink in and out as syntax highlighting finished loading. The row reveal animation no longer fades opacity (only slides), and highlighting is computed once up-front, so the bar stays solid.
- **Every tool gets a proper icon + label** — local-git tools (status/diff/log/pull/commit/push), the in-app browser opener, and notifications previously showed a generic wrench with a raw `Running <tool>` caption. They now have dedicated icons, verb-led captions ("Checking git status", "Committing: …", "Opening example.com"), and sensible colour tints (git writes read as consequential, reads stay calm). The sub-agent (`Task`) tool also gets its proper bot icon.
- **Context gauge is now a ring in the composer** — moved from the header bar to a small filling circle (Claude-Desktop style) bottom-right of the composer, by the model pill. Fills as the session uses up the model's context window; calm by default, warms past 75%, danger past 90%. Hover for `tokens / window (%)`. Accuracy unchanged (real CLI usage envelope).
- **No more duplicate "Thinking…"** — the thinking block only appears once reasoning is *done* (as the collapsible "Thought for Xs"); while the model is actively thinking, that state shows once via the live header + footer, not twice.
- **Footer numbers + units align (the real fix)** — the animated counters ("2s", "186 tokens") sat ~2.4px above their unit letters because the odometer glyph used `display:inline-block` (an inline-block rides above the text baseline; even removing `overflow:hidden` doesn't help). Both the component and the shared `stream.css` rule now use plain `display:inline` + `clip-path` for the roll, so the digits sit exactly on the unit's baseline. CDP-measured delta: 0.0px. (The earlier baseline pass only touched the scoped copy; the global one still forced inline-block.)

## v0.24.0 — Stream design-language pass (calm/boxless tools + live-turn polish)

> A full sweep of every streamed display to one calm, professional design language: accent colour is now reserved for *meaningful* signals (progress, live spinners, pass/fail) and everything idle reads as neutral/boxless so the page texture shows through. Plus the live-turn upgrades from `b4fd4e5` (animated token odometer, file-path surfacing, inline edit diffs). Verified: `npm run check` 0 errors / 0 warnings (4106 files) · vitest 222/222 · CDP screenshot review of every stream block. No backend change.

**Live-turn polish (b4fd4e5)**
- **Animated token odometer** — streaming token + elapsed counters ease toward their target (rAF ease-out cubic, per-glyph roll, reduced-motion safe) instead of popping raw numbers. New `AnimatedCount.svelte`; footer units now baseline-align with the digits.
- **File-path surfacing** — read/edit rows show *what* file is in play (collapsed `…/dir/dir/` crumb + bare filename, full path on hover).
- **Inline edit diffs** — Write/Edit/MultiEdit batches render real `+adds / −dels` (cheap `diffArrays`) with a collapsible inline `EditDiff` preview, not just "made an edit".

**Design-language pass (this release)**
- **Boxless/transparent everywhere** — plan checklist, sub-agent block, and the legacy ToolChip timeline drop their card fills + full borders for a thin neutral left rule, matching the inline-diff treatment.
- **Calm icon badges** — web/steer/agent/tool badges were loud accent-filled squares (each a different tint); all unified to one neutral badge (`fg 6%` bg, `fg-2` glyph). Web source chips + per-source dots de-loud'd to match the markdown code-chip.
- **Professional markdown** — inline `code` is now a calm neutral chip (faint surface + soft border) instead of bright accent text, so a code-dense paragraph reads as prose, not a wall of colour. Links get a restrained accent + subtle underline; code-block left-bar softened.
- **Composer** — model-selector pill made transparent to match the left toolbar; hero textarea tightened (no longer oversized); the old agents/shells/tools count pills that popped near the composer on every bash/PowerShell run were removed (that status lives inline in the turn footer now) — only the "N queued" signal remains.
- **Semantic colour kept** — pass=green, fail=red, live=accent spinner/progress, error=danger are unchanged (those are real signals, not decoration).

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.23.0** — Latency auto-scale (trivial turns downshift `smart→quick`) + turn-phase probe + live-status pill fix + round-12 `src/lib/state/` sweep (6 fixes: STT corrupt-resume guard, mic over-rail, split-pane scrub, toast timer leak).
- **v0.22.0** — Rounds 10–11 hardening (27 fixes across under-swept surfaces; skeptic-verify multi-agent rounds).
- **v0.21.0** — Rounds 5–9 hardening (1 critical + 42 fixes): config race · IPC/update/browser · secrets/commands/state/frontend · diagnostics/UI-resilience · turn-lifecycle/git-DoS/streaming/composer.
- **v0.20.9** — Recovery tools + per-project chat scoping + 4-round hardening pass.
- **v0.20.8** — Overnight efficiency + cleanup polish (stress-test pass on v0.20.7).
- **v0.20.7** — Full redesign port (all 7 surfaces rebuilt to spec, CDP-verified) + backend dead-code sweep.
