# Review Wave-2 Backlog (cont.174, 2026-06-23 — trimmed cont.207)

> From the 507-agent frontend review (161 raised → 73 adversarially-verified). The cont.174 fix-pass
> + later work landed **all of Tier A + B + C1–C22**; only the items below stay open. Full FIXED
> detail + the original A/B/C tables via `git log -- docs/review-wave2-backlog.md` (pre-trim revision).
>
> **Discipline:** findings are HINTS, not facts. The two parses disagreed on line numbers — **re-grep
> each by symbol before editing.** Line numbers below are pre-fix-pass and shifted; anchor by snippet.

## STILL OPEN (Tier A/B/C1–C22 scope, post cont.189 triage)

- **C7** — `promptPreview` (120 chars of user text) stored in every `TurnRecord` with no TTL
  (`send.ts:119`), a mild privacy/retention concern. NOT auto-fixed: redacting/omitting may break
  whatever surfaces read the preview (telemetry, recent-turn UI). Needs a retention-vs-feature policy
  call before editing. *(C5 — `agentSpawns` unbounded — is FIXED: `streaming.ts::capSpawns`, cap 200.)*

## NOT re-triaged — status unknown, re-grep before acting (C23–C35 + Tier D)

These were never re-verified against current code; some may already be fixed, some may be false
positives. Re-grep by symbol before touching any.

| # | Title | File (re-grep) | Fix |
|---|---|---|---|
| C23 | STT `init()` listeners leak when `destroy()` races mid-await sub() loop | `state/stt.svelte.ts` | Abort flag checked in `sub()` before pushing to `unlisten[]` |
| C24 | STT unbounded `segments[]` + O(n) join per utterance commit | `state/stt.svelte.ts` | Cap 500 / collapse committed segments |
| C25 | `modelDownloads` full object spread per progress chunk | `state/stt.svelte.ts` | Throttle progress 150ms/key |
| C26 | Voice auto-send triggerable by ambient audio, no confirm | `state/stt.svelte.ts` | Debounce / grace-window cancel |
| C27 | STT `destroy()` doesn't invalidate in-flight polish | `state/stt.svelte.ts` | `polishGuard++` in `destroy()` (= B5) |
| C28 | CommandPalette double `$effect` subscription | `dialogs/CommandPalette.svelte` | Read only `openTick` |
| C29 | `repair()` no `state==="available"` guard — discards pending update | `state/updates.svelte.ts` | `\|\| state === "available"` in early-return |
| C30 | UpdateDialog release notes index-keyed | `dialogs/UpdateDialog.svelte` | Key `` `${i}:${ln.kind}` `` |
| C31 | Tauri error objects stringified into Settings UI — leak error chains | `settings/SettingsPage.svelte` | `console.error` full; show generic summary |
| C32 | `refreshConversations` no size bound — sort-all on every palette open | `dialogs/CommandPalette.svelte` | Rust-side limit / memoize sort |
| C33 | WebBrowserPage `$effect` reads+writes `pendingUrl` w/o untrack | `webview/WebBrowserPage.svelte` | `untrack()` the clear write |
| C34 | StatsPanel `now` frozen at mount — wrong day boundary past midnight | `home/StatsPanel.svelte` | `let now = $state`, refresh in `$effect` |
| C35 | Context-menu `innerText` on `<pre>` forces reflow | `state/contextMenu.svelte.ts` | `innerText`→`textContent` |
| D1–D12 | Tier D (votes 2/3, verify hardest — some likely false-positive) | various | see `git log` revision for the full table |

## Patterns (sweep candidates, not one-offs)

- **Timer/listener cleanup gap** (C23, C27, D2): systemic missing `$effect`/`onDestroy` teardown.
- **Unbounded arrays → OOM** (C7, C24, D11, D12): cap-on-write discipline.
- **Svelte5 `$effect` self-invalidation** (C33): `untrack()` writes that feed reads.
- **Index-keyed `{#each}`** (C30, D6): stable-id keys.
- **`stt.svelte.ts` is the single densest hotspot** (~7 remaining findings) — an STT-focused pass
  clears a big chunk at once.
