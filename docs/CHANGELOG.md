# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.26 — 2026-06-11 — Composer slim-down + open-issue sweep (#29/#30/#12/CR-UX)

> **Why.** The composer had grown into a four-layer stack (Working-rail + card + gauge + disclaimer) spanning the full 1100px column; meanwhile four tracker items were actionable in one pass.

**Composer (`Composer.svelte`, `composer/QueueRail.svelte`):**
- **Persistent "Working…" rail gone** — the pending rail now renders only with queue chips present, or while streaming with a steerable draft (typing mid-turn slides it in; Steer/✓Steered unchanged, Alt+Enter still steers).
- Beta/"AI can make mistakes" disclaimer line removed.
- Width capped at 880px (was full chat column), wrap padding 10/14→6/10, radius 18→14.

**Activity panel (`ActivityPanel.svelte`):** last-turn reply-preview quote removed — recap keeps the duration/tools/files/cost stat grid only.

**Tracker fixes:**
- **#30** — resumed tabs pinned to a different folder are now visible: new `assistant_session_cwd` command (convo_store.rs) exposes the cwd sidecar, `loadConversation` hydrates `TabState.sessionCwd`, and the tabs bar shows a warn-tinted folder badge (full path in tooltip) when it differs from the selected workspace.
- **#29** — inline styles unblocked in prod: the style-src nonce came from **Tauri's asset-CSP rewriter** (not SvelteKit); `dangerousDisableAssetCspModification: ["style-src"]` keeps `'unsafe-inline'` effective while script-src nonce hardening stays. 🧪 verify on this build: transitions animate, update progress fills, zero CSP console violations.
- **#12** — tool chips read as openable: chevron `fg-muted`, accent + nudge on hover, Expand/Collapse tooltip on the chip head.
- **CR-UX** (user signed off in-session) — trust enum collapsed to `readonly|standard`. Dead `full` rejected for new writes; persisted ternary-era configs and legacy `RIFT_TRUST_LEVEL=full` env migrate read-side to `standard`; turn.rs git-write allowlist gate simplified; frontend `TrustLevel` narrowed; tests updated both sides of the gate.

**Verify.** cargo check clean · trust tests 2/2 · svelte-check 0/0 (4093 files) · vitest 122/122. Pixel pass on the slimmed composer + cwd badge pending first live run of this build.

## Older versions

v0.8.25 dictation polish data-fence (`<transcript>` guard — questions stay questions, masked profanity restores, "send it" polishes first) + PTT stuck-mic fix + #32 ctx meter on restore + #31 401 helpers/legacy provider cmd removal · v0.8.24 enhance wand v2 + dictation uncensored 3-layer + voice commands + hold-Space PTT · v0.8.23 Activity panel polish (MCP steps humanized, turn separators, Sources, recap) · v0.8.22 multi-tab stream survival + Harness mission control + dead-code sweep · v0.8.21 self-aware Rift (loopback UI bridge: ask_user/open_browser/notify) · v0.8.20 live plan limits · v0.8.19 custom context menus + Fable 1M ctx fix · v0.8.18 UI sweep (9 audit findings, per-chat model scoping) · v0.8.17 Rail-v2 steer chips + registry race fix · v0.8.16 backend split COMPLETE · v0.8.15 hot-file splits · v0.8.14 update-dialog crash fix · v0.8.13 Claude Fable 5 · v0.8.11 Settings redesign · v0.8.9 first tag-driven CI release · v0.8.0 one-click 401 recovery + edit-swarm · v0.7.0 cost cockpit · v0.6.2 update child-lock fix · v0.6.0 browser dock · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
