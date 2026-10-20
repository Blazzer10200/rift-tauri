# Handoff — Rift — 2026-10-18

Trimmed 2026-10-18 to current state only. The full prior file (Sept 22/25
batch logs) is archived at
`~/.claude/archive/handoffs/rift-tauri-HANDOFF-20261018.md`; commit history
covers the rest.

## Session 2026-10-19 (late) — redesign: Ledger built, chat motion diagnosed (IN PROGRESS)

### Completed
- Audit + phased plan: `docs/design/redesign-plan.md` (gitignored; source of
  truth for this thread). Evidence shots `docs/design/redesign-audit/`
  (01–07 before, A1/A2 Ledger after).
- Canvas https://claude.ai/artifact/F9MZxDGhBHW4pwwDM89Gin — Current ×2 +
  A Ledger / B Spine / C Document. Owner picked **A · Ledger**.
- Ledger built on branch `design/chat-ledger` → commit `8bb71f4f` (LOCAL,
  not pushed, not merged): answer first + one ruled receipt line
  (StreamTurn, stream.css), user time readable once (MessageBubble), message
  column = composer edges (AssistantPane), composer workspace crumb removed
  (Composer), empty turns collapse. check PASS (0/0), vitest 776/776.
- Chat motion measured (dev, /dev/gallery, 70s rAF sampler): p50/p95 5.6ms,
  2 frames >20ms → no jank. "Laggy" = deliberate pacing: word reveal
  (`Markdown.svelte:169-172`, 42ms/word + ~520ms catch-up + 500ms blur) and
  scroll-follow lerp 0.2/frame (`AssistantPane.svelte:169-188`).

### Next Steps
1. 4 no-visible-change motion fixes (plan doc "Chat motion" → "Do now"):
   drop blur on `.md-w-hold`; remove `railGlint` (§8 traveling light — tell
   owner); `.jump-latest` backdrop-filter → `.hud-float` recipe; check if
   MessageBubble `data-streaming` blur rule is dead. Rerun sampler. Commit.
2. Offer owner 3 toggles ONE at a time, felt live: reveal (blur vs
   opacity+2px rise), pacing (WORD_MS 42→28, catch-up 520→300ms), follow
   (lerp 0.35 or instant pin). Separate revertable commits.
3. Scroll-follow is unmeasured (gallery uses its own `.stage`): needs one
   short real turn — billed, ask owner first.
4. Then Phase 1 tokens → parallel sessions for Phases 2/4/5 (plan doc).

### Key Decisions
- Turn clock time dropped from receipt (user msg owns it); tokens show on
  hover in the receipt (Rift tooltips never paint — `actions/tooltip.ts`).
- Composer crumb removed: footer owns project/branch/GitHub; split panes
  show folder in pane header. Branch/GitHub loaders kept in Composer.

### Hooks & Gotchas
- `c.sh look` returns empty every other call — loop until a path prints.
- `.tmp` keeps only ~20 snaps; copy keepers to `docs/design/redesign-audit/`.
- Opening the conversation: click `button[aria-label^="Continue latest"]`
  on the welcome; `div.crow` click sometimes doesn't load messages.
- Dev app + CDP wrapper LEFT RUNNING for the next session; reap with
  `bash scripts/cdp/c.sh reap --all` then `c.sh shutdown` when done.

## Session 2026-10-19 — Rift Style + design fixes (awaiting owner review)

- **Rift Style** Claude Design System artifact (private):
  https://claude.ai/artifact/WSAPanWKnSrrtj47mrW4Tu — tokens, brand-book
  README, rejection log, 10 component cards (static renditions styled by
  verbatim `app.css` CSS), fonts, logo. Generated from `main` @ `f2f4a82`.
  Verified by local render only (claude.ai sign-in blocks the pane). Blank
  preview there → suspect the `<!doctype>/<head>` wrapper in each preview.html.
- Branch `design/rift-style-fixes` (off `main`, NOT merged, NOT pushed):
  - `d71fff34` greeting font: `.greet-title` asked for `"Lexend"`, but only
    `"Lexend Variable"` is registered → it rendered in Inter. Measured live:
    314px (Inter) → 330px (Lexend).
  - `7ffa450a` `--fg-subtle` L 0.545 → 0.600 (`app.css`, `styles.css`,
    `_ds_manifest.json`). Contrast 4.01→5.03 on bg, 3.53→4.44 on elev-1,
    3.12→3.91 on elev-2. A taste call; revert that commit alone if it
    looks too loud. The alternative is to keep the token and re-point `.help`/`.dim` to `fg-muted`.
  - `npm run check` PASS, vitest 776 passed. Before/after shots in the
    session report.
- Owner approved: fast-forwarded into `main` and pushed to origin
  (`f2f4a82d..7ffa450a`); local branch `design/rift-style-fixes` kept. Rift Style republished v5 with the new fg-subtle
  (tokens.json, README, Input README, index `lastChange`).
- Flagged, not changed: `fg-faint` chat timestamps / turn meta
  ("Sep 25 · 9:09 PM", "↑ 22k") are near-invisible; `border-strong` 1.87:1
  (hairlines are deliberate); footer repeats the live tool caption;
  `design-system/` static cards overlap Rift Style. Consider retiring them
  (owner's call, nothing deleted).
- Dev stack DOWN (reaped), preview server stopped.

## Session 2026-10-18 — project cleanup (done)

- Branch `chore/project-cleanup` (off `main` @ `536c2bf8`), 1 commit:
  `4e1d4262` docs(developing): scripts index table in `docs/DEVELOPING.md`
  (§Build from source › Scripts). Not pushed, not merged.
- Deleted fully merged local branches (`git branch -d`): `ui/primitives`,
  `chore/harness-audit-20260929`, `claude/loving-edison-3cf8mz`. Remote copies
  `origin/claude/loving-edison-3cf8mz` + `origin/mega-audit-cont228` untouched
  (both 0 commits ahead of main; deleting = a push, owner's call).
- Removed caches: `.svelte-kit/` (regenerated by `npm test`),
  `scripts/cdp/.tmp/` snapshots + `dev-launch.log` (files only, dir kept).
  `C:\cargo-targets` has no stale `rift_tauri` incremental dirs (checked).
- Gitignored design notes fixed: `docs/design/README.md` now indexes
  `ui-primitives-plan.md`; that plan's status = shipped v0.160.0;
  `claude-code-ui-reference.md` no longer points at archived
  `rift-redesign.html`.
- Survey findings (no action needed): no orphan frontend modules, clippy
  clean, 4 TODOs, `allow(dead_code)` in `stt/` is feature-gated on purpose.
  Doc path references in AGENTS/CLAUDE/ARCHITECTURE/DEVELOPING all resolve.
- Checks: `npm test` 776 passed, svelte-check 0/0 (earlier today). Rust not
  re-run (no Rust changes; last green 2026-09-25: clippy clean, cargo 272).

## Stream tile cleanup — DONE 2026-10-18, awaiting owner review

Commit `f2f4a82d`, fast-forwarded into local `main` 2026-10-18 with
`4e1d4262` (branch deleted). `main` pushed to origin 2026-10-18 (`536c2bf8..f2f4a82d`).
Owner reviewed the screenshots: "Looks good." Dev stack DOWN (reaped). Full
list in the commit message:
every tile audited for duplicated status / boxes-on-boxes. Changed:
StreamAskUser, StreamTurn, WorkLine, streamModel caption, ReadResult,
WriteBatch, StreamResult, StreamAgent, StreamExitPlan, ActivityHud,
galleryFixtures. Already clean, untouched: StreamPlan, PlanHud, StreamWeb,
GrepResult, PermissionBar, EditDiff.
- Bugs found on the way: ReadResult 3-digit line numbers wrapped (global
  border-box ate the gutter padding) and Shiki code rendered double-spaced;
  MultiEdit rows opened to an empty well; `/dev/gallery` start()/finish()
  tools stuck "pending" (fixture mutated blocks in place, defeating
  `adaptCache` #100). All fixed.
- Checks: svelte-check 0/0, check:ui PASS, vitest assistant 245 passed;
  `/dev/gallery` replay screenshotted live + settled via CDP, 0 console errors.
- Owner call left open: the footer still shows the live tool caption
  ("Reading X") right under the in-flow live row that says the same thing.
  Kept on purpose (9/25 design: footer = the one honest status line).
- Not verified: a real (paid) turn; ActivityHud + PlanHud floating bars
  (they only show while their cards are scrolled off-screen, not exercised).
- Gotcha: `c.sh look "<selector>"` crops land offset at DPR 1.25 here —
  scrollIntoView + full-window `look` instead.

## Current state (v0.160.0 shipped 2026-09-25)

- `main` = `origin/main` = `536c2bf8`. Release run 36197142341 green; R2 feed
  + GitHub release verified.
- `components/ui/` primitive layer shipped (shadcn-svelte 1.7 + bits-ui
  2.19.3, `recipes.ts`); `design-system/check-ui.mjs` ratchet in
  `npm run check`; `/dev/ui` catalog (Ctrl+Alt+U, DEV only).
- Next primitive swap candidates: SettingsMenu (hand-rolled `.rift-menu`),
  ProjectSwitcher `.sw-menu`, GhPopover, FilePathMenu, MentionPopover,
  SlashMenu. Not verified live: click-outside dismissal (bits-ui default).

## Known problems / caveats

- Permission prompts can't fire on this machine in full-config mode (owner's
  `~/.claude/settings.json` allows `Bash(*)`/`Edit(*)`/`Write(*)`). Not a bug.
- `/mcp` on a ChatGPT pane is swallowed silently by `handleChatGptSlash`.
- Fast-mode row clamps `.pi-sub` to two lines → caption truncates. Shorten
  `modelMatrix.ts` `avoid`.
- Test leftovers kept on purpose: "rift-stress" chats,
  `claude-sandbox/rift-stress{,-extra}`.
- `C:\cargo-targets` is SHARED by ~10 projects — never wipe it whole.
- Open issues: `docs/ISSUES.md` (#101–#105, #107).
- Optional live check still unrun: one short Opus 5.5 turn at Low thinking to
  confirm it no longer routes through the no-think shim.

## CDP lessons (keep)

- Never type a slash command + Enter into the composer over CDP (SlashMenu
  fires the highlighted item; one real ChatGPT turn was sent this way).
  Safe path: set the textarea via `eval` + `input` event, click `.send-btn`.
- The home/hero view hides the composer bar (`{#if !hero}` in Composer.svelte),
  so `.model-pill` only exists inside a conversation. Open a sidebar chat first.
- The status-bar "Claude" button opens Settings › Providers and covers the
  composer. Return via the "Chat" nav button.
- `c.sh eval` strings: avoid `*=` attribute selectors and `\\` path splits
  (shell quoting breaks them). Iterate `querySelectorAll('button')` instead.
- The Other-models flyout opens on real pointer hover; a scripted `click`
  doesn't open it.
- Sidebar nav clicks get covered unless "Pin sidebar open" is on;
  Ctrl+1..9 switches workspaces (Ctrl+2 = Chat).

## Canonical commands

- Full verification: `npm run verify`
- Dev launch: `npm run cdp:dev` then `npm run cdp:serve` (background)
- Dev cleanup: `bash scripts/cdp/c.sh reap --all` then `bash scripts/cdp/c.sh shutdown`
- Never run cargo while `tauri dev` is up; never kill `rift-tauri.exe` by image name.
