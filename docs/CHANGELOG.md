# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.8.14 — 2026-06-09 — fix: the update dialog could never open (it crashed on render)

> **Why.** The real end of the v0.8.3→v0.8.12 "can't click update" saga. Every prior fix chased layout / compositor / z-index — but the click was never the problem. The pill's handler fired, set `dialogOpen = true`, then `UpdateDialog` **threw on render**, so the overlay never reached the DOM. Clicking looked dead because the dialog silently crashed every time.

- **Root cause: duplicate `{#each}` key.** The "What's new" notes loop keyed each line on `kind + '|' + text`. Every blank line collapses to the same `blank|` key, so any release notes with two or more blank lines (i.e. all of them) threw `each_key_duplicate` and aborted the render. Data-dependent — which is why dev "worked": it just hadn't hit notes with consecutive blanks. Keyed on index now (the notes array is a fully-recomputed `$derived`, never reordered — index is correct and collision-proof).
- **Verified live.** Reproduced the crash against the installed v0.8.12 build via CDP to confirm causation, then drove the dev store with notes containing multiple consecutive blank lines (the exact crashing shape) → dialog renders clean: header, version diff, formatted notes, both footer buttons, zero exceptions.
- Folds in backend hardening: swarm worktree-escape guard + 6 defensive fixes (git_local · mcp_server · browser · stt · usage).

**How to verify.** With an update available, click the pill → the dialog opens first try with the release notes rendered.

**Known follow-up.** SvelteKit injects a CSP `nonce` that nullifies `'unsafe-inline'` in `style-src`, blocking Svelte's inline transition styles + the download progress-bar fill (cosmetic — download/apply still work). Tracked in ISSUES.

**Verify.** `npm run check` 0/0 (4070) · CDP-verified (crash reproduced on prod, fix confirmed in dev).

## Older versions

v0.8.13 Claude Fable 5 limited-run model in the picker (accent name + "UNTIL JUN 22" badge; self-heals to Sonnet/Opus after Jun 22; Fable pricing in the cost cockpit) · v0.8.12 fix: an update can never go invisible again — pill `×` is a 24h snooze (`{version,until}`), never a permanent dismissal; snooze-proof accent dot on the Settings gear; `backdrop-filter` stripped from dialog/toasts (WebView2 mis-composite) · v0.8.11 Settings redesign (single-column titled cards) + Harness one-viewport overhaul (Telemetry · Cost · Swarm) · v0.8.10 fix: update button no longer 50/50 — stable singleton `UpdatePill` replaces the sticky toast that slid out from under the cursor + WebView2 backdrop-filter garbage fixed on the pill · v0.8.9 first tag-driven CI release · v0.8.7 fix: update toast was unclickable — host z-index 60→2000 above transient overlays · v0.8.5 fix: corrupted install no longer masquerades as "up to date" · v0.8.3 fix: updater can no longer hang forever (mutex released before network + check timeout + stall watchdog) · v0.8.1 visible + always-recoverable app-update failures (rotating `rift.log` + sticky failure toast) · v0.8.0 one-click 401 recovery + edit-swarm + opt-in context compression · v0.7.0 cost cockpit + multi-provider list · v0.6.2 in-app-update child-lock fix · v0.6.0 in-app browser dock + harness redesign · v0.5.0 Harness telemetry + Steer. Full detail: `git log -- docs/CHANGELOG.md`.
