# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## v0.4.46 — 2026-05-31 — feat: permanent activity dock with live "reacts as it works" feedback

> **Why this release exists.** The assistant activity dock graduated from a peek-on-demand panel into a permanent, live workspace. Work no longer silently appears and vanishes — each tool call visibly lands with its result, the dock gained quick actions, and file outputs now show how much changed.

**The activity dock is always open.** It used to default closed and snap shut on every new or cleared conversation; now it's a permanent surface that stays put across new / clear / tab-switch (you can still hide it from the composer when you want the room). [assistant.svelte.ts](src/lib/state/assistant.svelte.ts)

**Quick actions, surfaced.** A compact toolbar at the top of the dock: copy the whole transcript as Markdown, compact the conversation, and jump to the latest message — plus a one-click Stop while a turn is streaming. [ActivityPanel.svelte](src/lib/components/assistant/ActivityPanel.svelte)

**File outputs show their churn.** Every file Claude writes or edits now carries a `+added / −removed` line count, accumulated across repeat edits, so a glance tells you what changed where.

**Live work reacts as it happens.** The headline change: a finished tool no longer just disappears. Each call now lands with a green check and its duration (a red ✗ on error), holds for a moment, then eases out — and rows slide and reorder instead of popping in and out. The redundant idle pulses were cut down to a single live indicator, and a completed turn ends on a calm "Done · {time}" confirmation. All motion respects your reduced-motion setting.

**Verify.** `npm run check` 0 / 0 (4110 files); live-verified across real streaming turns via CDP (live row → completion tick → turn-end confirmation). NSIS bundle + SHA256 round-trip via `release.ps1` at ship.
