# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.7-alpha — 2026-05-17 — Settings → Accessibility (dyslexia-friendly Assistant)

Trey-driven feature. New **Settings → Accessibility** section between *Assistant* and *Speech* (lucide `Accessibility` icon). One master toggle plus three independent dials, all persisted to localStorage and applied via `data-a11y-*` attributes on `<html>` so CSS overrides land instantly with no reflow shrapnel.

**Master toggle — Dyslexia-friendly mode.** First time on, seeds the recommended bundle: Lexend font + increased line-height. Subsequent toggles preserve whatever fine-grained dials the user has tweaked. When on, [src/lib/state/assistant.svelte.ts](src/lib/state/assistant.svelte.ts) forwards a `dyslexiaMode: true` arg to the new `assistant_send` signature, and [src-tauri/src/assistant/mod.rs](src-tauri/src/assistant/mod.rs) appends a single-line addendum to the per-turn system prompt: *"…phonetic typos (\"wair\"/\"where\", \"nite\"/\"night\"), letter-swap typos (b/d, p/q), and slurred-speech transcription artifacts are expected — interpret the most likely intended meaning charitably and proceed; only ask for clarification when meaning is genuinely ambiguous, not for ordinary typos."* Zero per-turn cost, zero latency; Claude 4.6/4.7 handles it natively. Also covers STT slur turns.

**Three independent dials:**
- **UI font** — System (Inter, default) ↔ Lexend (research-tuned for dyslexia, bundled via `@fontsource-variable/lexend` — offline, ~150KB). Applies to body + textarea + input + button + select.
- **Increased line + letter spacing** — line-height 1.85, letter-spacing +0.012em, word-spacing +0.06em. Scoped to `.bubble` / `.markdown-body` / textarea so Files / Sync / Terminal keep their compact density.
- **Warm reading tint** — sepia overlay (`oklch(0.225 0.014 60)` bg, warm off-white fg) on message bubbles + code blocks only. Softens the bright-on-dark contrast that triggers glare for some readers without disturbing the rest of the dark theme.

**Implementation:** new [src/lib/state/accessibility.svelte.ts](src/lib/state/accessibility.svelte.ts) store (mirrors `ui-prefs.svelte.ts` pattern — class + `$state` + localStorage round-trip + `apply()` to `documentElement.dataset`). New CSS block at the bottom of [src/app.css](src/app.css) under `/* Accessibility — dyslexia-friendly overrides */`. Settings section block mirrors the Speech section's card pattern exactly — same `.asst-card` / `.asst-row` / `.switch` styling, same toggle phrasing convention. Wired through `+layout.svelte` `onMount`.

3-file version bump 0.4.6 → 0.4.7-alpha. Cargo.lock auto-syncs on build.

Verify: svelte-check + cargo check clean (auto-verifier `last-ok`). Functional verify via CDP after install — toggle the master switch in Settings → Accessibility, confirm font + spacing + tint apply, send a typo-laden prompt and confirm Claude responds without grammar pedantry.
