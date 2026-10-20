# Handoff — Rift — 2026-09-25

## Current Objective

**Stream tile cleanup (in flight, 2026-09-25 evening).** Owner: the chat
stream reads "tacky, unsmooth, confusing" — duplicated status, boxes on
boxes. Owner chose "full pass on every tile type, then show screenshots".
UNCOMMITTED edits in the tree (svelte-check 0/0, check-ui PASS, 134 stream
vitest green):
- `StreamAskUser.svelte`: an errored ask (`status === "error"`, bad input
  shape → retried) renders as one compact "Question couldn't be shown" row,
  not a second full card; header pill `align-self: flex-start` (was a
  full-width bar).
- `StreamTurn.svelte`: `.sturn-head` only renders for compaction (the
  "Waiting for you" head was the 3rd waiting signal); footer always leads
  with the state verb (`footerVerbShown = footerVerb`) so a past "Read X 46s"
  trail stops reading as a stuck read.
- `WorkLine.svelte`: `.is-bad` frame only when ALL tools failed; partial →
  `N failed` pill.
- `streamModel.ts` `caption()`: Grep pattern trimmed to 32, Glob to 40.
- `stream.css`: dead `.awaiting-head` rule removed.
Next: audit StreamPlan / StreamWeb / StreamResult / StreamAgent /
StreamExitPlan / WriteBatch / GrepResult / ReadResult / ActivityHud / PlanHud
/ PermissionBar / EditDiff for the same noise classes (a recon agent was
started for this and did not finish — re-run it), apply, verify, then
`npm run cdp:dev` + `c.sh look` screenshots per tile type for owner review.
Not verified live yet: none of the above has been seen in the running app.

After that: keep swapping hand-rolled overlays onto `components/ui/`.

## Current State (2026-09-25, v0.160.0 shipped)

- `main` = `origin/main` = `536c2bf8` (comment-only commit after the
  `d0ba1298` release commit). Local `ui/primitives` is fully merged (1 behind
  main); safe to delete when convenient. Working tree clean. Dev stack DOWN.
- Shipped: `components/ui/` primitive layer (shadcn-svelte 1.7 + bits-ui
  2.19.3, restyled via `recipes.ts`); `design-system/check-ui.mjs` ratchet in
  `npm run check` (baseline: use-portal 11, aria-role 17, hex 21, rgb-hsl 16,
  oklch 131, arbitrary 2); PermMenu / ContextMenuHost / shared/Select /
  CloseConfirm swapped; `/dev/ui` catalog (Ctrl+Alt+U, DEV only); project
  **Delete…** in ProjectSwitcher right-click → `DeleteProjectDialog`
  (config-only removal via `assistant_delete_project`).
- Notices regenerated; `gen-third-party-notices.py` now sniffs "MIT License"
  from LICENSE when package.json has no `license` (svelte-toolbelt).
- Gates: `npm run verify` green 2026-09-25 (svelte-check 0/0, check-ui PASS,
  vitest 776, clippy clean, cargo test 272). Release run 36197142341 green;
  independently verified: R2 `releases.win.json` lists 0.160.0 full+delta,
  both packages HTTP 200 with sizes + SHA-256 matching the feed and GitHub
  digests; GitHub release final (not draft) with Setup.exe, full, delta,
  `releases.win.json`, `RELEASES`.
- Live CDP verified (throwaway project `zz-delete-test`, now removed; the 12
  real projects untouched, config backup in session scratchpad): delete flow,
  PermMenu (arrows, Esc, focus return, ⇧Tab while open), context menu
  (re-open at new point keeps focus target, danger row), Select (mic picker,
  unchanged), CloseConfirm (Esc only), `/dev/ui` toggle, 0 console errors.
- Not verified live: click-outside dismissal (bits-ui default, untested here).
- Next swap candidates: SettingsMenu (still hand-rolled `.rift-menu`, see its
  style comment), ProjectSwitcher's own `.sw-menu`, GhPopover, FilePathMenu,
  MentionPopover, SlashMenu.

## Previous batch (2026-09-25 morning, shipped inside v0.160.0)

- `44d054fc` chore(deps): npm + cargo lockfiles refreshed within semver
  (tauri 2.11.6, svelte 5.57.1, vite 8.3.1, 76 crates). Majors held:
  typescript 7, vitest 5, @types/node 26.
- `npm run verify` green 2026-09-25: svelte-check 0/0, vitest 771, clippy
  clean, cargo test 272. `cargo audit`: 0 vulns (old #109 rustls advisory
  gone — lock is on rustls 0.23.45), 9 allowed unmaintained/Linux-only warns.
- Git tidied: both old stashes exported to
  `~/.claude/archive/rift-stashes/*.patch` then dropped; draft PR #7 closed +
  branch deleted (its non-Windows `npm_global_prefix` stub was NOT ported —
  owner: Windows-only). Merged remote `mega-audit-cont228` kept (owner call).

## Previous batch (2026-09-22, shipped as v0.159.0)

- Backend audit batch landed on top of the docs pass (newest first):
  `docs` commit, `7ea3fd4f`-pick oneshot runner, turn split (prompts.rs,
  tool_allowlist.rs, spawn helpers), cli_install `run_bounded` + branch sync
  via `run_git`, Codex warm spare, PendingRegistry, `1b81e995` dead code + MCP
  annotations, `f4fcd767` convo cache + certs off main thread, `26632c16`
  rustls. Working tree clean. Dev stack DOWN.
- `npm run verify` green 2026-09-22 (771 vitest, 272 cargo, clippy clean).
- Live-verified: `assistant_codex_prewarm` parks one `codex.exe app-server`
  ("warm spare ready in 3974 ms" in rift.log); overview's own server exits;
  app shutdown leaves 0 codex.exe. NOT verified live: a real ChatGPT turn on
  the spare (costs a message), the 5-min expiry, the composer trigger.
- Folder cleanup 2026-09-22: removed 4 agent worktrees + `worktree-agent-*`
  branches (cherry-picked already), `scripts/cdp/.tmp`, `build/`,
  `src-tauri/node_modules`; archived `docs/design/rift-redesign.html` to
  `~/.claude/archive/rift-design/`. `git gc`: .git 487 MB → 64 MB.
- Release prep: v0.159.0 bumped via `scripts/bump.ps1` (6 fields), CHANGELOG
  Unreleased promoted, v0.158.0 section dropped (current-release-only policy).
- Audit leftovers, deliberately not done: `resolve_spawn` (~800 lines) and
  `stream_one_turn` split; stt `stt_start_recording`/`download_file` length;
  bridge.rs emit skeleton; codex/openai emit-helper dedup; `--fork-session`
  (needs a UI feature first). Checked + rejected with numbers: STT partial
  window, audio peek copy, fast-tier model/list (1-2 ms warm), codex stream
  idle timeout, unsupported control_request (already logged as warn).

## Commits this batch (newest first)

- `e9657f2e` Opus 5.5 (`claude-opus-5-5`, config.rs `OPUS_MODEL`) is the main
  Opus row; Opus 5 is the `claude-opus-5` row under Other models.
  `canonical_model_alias` pins `opus` like `sonnet`. `rejects_disabled_thinking`
  keeps Fable + Opus 5.5 off the no-think shim (their API rejects
  `thinking:{type:"disabled"}`; source: CLI 2.1.280 model catalog).
- `fd676365` Claude is the default again. The one-time ChatGPT-default
  migration is gone; `restoreClaudeDefault` (helpers.ts, key
  `rift.assistant.claudeDefault.v1`) drops a GPT baseline + project pins equal
  to it. Verified live: dev app came back on `sonnet`.
- `c9a96847` No-project home (`AssistantWelcome.svelte` `isLocalMode` branch)
  fits one screen: "What's on your mind?", ctx chips, Open a project, 3 recents.
- `e49a072f` "No project" folderless mode replaces "All chats":
  `assistant_clear_root` / `clearRoot`; turns run in Documents\Rift Workspace.
- `834ca67f` Stream-tile design system: one family for all chat blocks
  (`.tile*` + `--tile-*` in `src/app.css`, DESIGN.md §7).
- `40cc15c0` Faster dev loop (rlib-only lib, line-tables debug: ~13s → ~6s
  incremental) + source tidy.
- `4876c08e` Claude Code parity batch from 2026-09-18 (models, `--add-dir`
  extra folders, output style, `/mcp` add/remove, UI polish).

## Checks run (2026-09-22)

- `npx svelte-check`: 0 errors / 0 warnings. `vitest`: 30 files, 771 passed.
- CDP: picker shows Fable 5.1 / Opus 5.5 / Sonnet 5 / Haiku 4.5 + Other
  models (1); composer `model=sonnet` after reload; 0 console errors.
- NOT run: cargo clippy/test (see above). No live Opus 5.5 turn was sent, so
  the disabled-thinking rejection is from the CLI catalog, not reproduced.

## Known Problems / caveats

- Permission prompts can't fire on this machine in full-config mode: owner's
  `~/.claude/settings.json` allows `Bash(*)`/`Edit(*)`/`Write(*)`. User config,
  not a Rift bug.
- `/mcp` on a ChatGPT pane is swallowed silently by `handleChatGptSlash`.
  Consider a one-line notice.
- Fast-mode row in the model menu clamps `.pi-sub` to two lines, so the warn
  caption truncates ("Skip when latenc…"). Shorten `modelMatrix.ts` `avoid`.
- Test leftovers kept on purpose: "rift-stress" chats, `claude-sandbox/rift-stress{,-extra}`.
- `C:\cargo-targets` is SHARED by ~10 projects (cuepilot, sweep, capturex,
  video-reader, loot-lens, ...; also holds ffmpeg/yt-dlp copies) — never wipe
  it whole. 2026-09-22: deleted Rift's 482 stale `incremental/rift_tauri*`
  dirs (124 GB) + dead `flycheck0`; C: free 617 → 737 GB.
- Open issues: `docs/ISSUES.md` (#101–#105, #107).

## Next Actions

1. Framework conversion (Electron) REJECTED 2026-09-25 — shell doesn't affect
   UI. Replaced by the UI primitives pilot: `docs/design/ui-primitives-plan.md`
   (shadcn-svelte restyled to Rift). Awaiting owner go; start at Phase 0.
2. Optional live check: one short Opus 5.5 turn with thinking at Low (costs a
   turn) to confirm it no longer routes through the shim.

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

## Canonical Commands

- Full verification: `npm run verify`
- Dev launch: `npm run cdp:dev` then `npm run cdp:serve` (background)
- Dev cleanup: `bash scripts/cdp/c.sh reap --all` then `bash scripts/cdp/c.sh shutdown`
- Never run cargo while `tauri dev` is up; never kill `rift-tauri.exe` by image name.
