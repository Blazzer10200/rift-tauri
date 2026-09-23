# rift-tauri — Changelog

> Current release only. Older release notes remain in Git history and on the
> [GitHub releases page](https://github.com/Blazzer10200/rift-tauri/releases).

## v0.159.0 — Current Claude, more control

- The model picker and usage views now know the current Claude line-up: Fable
  5.1, **Opus 5.5**, Sonnet 5, and Haiku 4.5, with the full effort ladder
  through **Max** on capable models. Opus 5.5 is the main Opus: newer and
  cheaper than Opus 5, which moves to **Other models** for chats that need to
  stay on it. Opus 5.5 always reasons (like Fable), so Rift no longer tries to
  switch its thinking off, a request its API rejects.
- Projects can list up to eight **Extra folders**; Claude can read and edit
  those alongside the project root (Claude Code `--add-dir`).
- Settings › Providers › Claude session gains an **Output style** field
  (Explanatory, Learning, or any style in `~/.claude/output-styles`).
- The `/mcp` dialog can now **Add** a server (stdio, http, or sse; user,
  project, or local scope) and **Remove** one, using the same config a
  terminal `claude mcp add/remove` writes.
- The home launchpad is a single column matched to the composer width, so
  recent chats and shortcuts no longer sit awkwardly beside it.
- Quieter motion where it helps: the composer rises in on the home view,
  Settings tabs get a sliding underline, the Claude "connected" dot pings on
  each check, hot usage bars in AI Health get a slow sheen, and lifetime
  token tiles count up (and read `3.96B` instead of overflowing). All of it
  respects reduced-motion.
- AI Health tiles no longer spill their numbers: large counts read `11M` and
  spend reads `$8.16k`, with the exact figure on hover. The active project
  card on the Workspace hub keeps its **Continue** button inside the card.
- Internal cleanup: one shared limit bar replaces three copies, usage-stat
  helpers moved to `utils`, duplicated time and severity helpers consolidated,
  and the splash handoff no longer fires twice.
- Faster development loop: an incremental Rust rebuild drops from ~13s to
  ~6s (the library builds as `rlib` only, and dev builds carry line tables
  instead of full debug info). Vite skips watching docs, scripts, and other
  non-app folders and pre-warms the app shell, and `npm run cdp:dev`
  reports a ready window a few seconds sooner.
- **No project** replaces "All chats" in the sidebar switcher. It is a
  folderless chat mode, like Claude desktop's: new chats run in your Rift
  Workspace folder with full tools, and the sidebar lists every project's
  chats with their project tags. An open conversation keeps its own folder.
- Claude is the default model again. An older update switched every project
  to ChatGPT once your ChatGPT account connected; that switch is gone, and
  projects it moved go back to Claude. A project where you picked a different
  ChatGPT model yourself keeps it, and ChatGPT stays one click away in the
  picker.
- Chat blocks are one family now. Terminal runs, diffs, agents, plans,
  web lookups, reasoning, code blocks, and saved-history tool chips share
  one set of shapes, borders, headers, and badges. State is shown the same way
  everywhere: a tinted border while something runs, a red edge when it fails,
  and an accent edge when it needs you. File changes are labelled **new**,
  **edit**, or **deleted** instead of a tiny coloured dot. The activity
  summary leads with what happened and keeps time and cost quiet. The
  floating panels and the jump-to-latest button use one frosted style.
- Source tidy: the screen registry lives at `shell/workspaceRegistry.ts`,
  `Select` moved to `components/shared/`, the path-helper re-export shim is
  gone, and three unreferenced dev scripts were removed.
- **ChatGPT turns start ~3.5s sooner.** Codex's App Server takes about 3.5
  seconds to boot, and every ChatGPT-subscription turn used to wait for it.
  Rift now boots one in the background while you type and hands it to your
  next turn. It shuts down after 5 idle minutes and never starts unless a
  ChatGPT-subscription chat is open. The ChatGPT account and limits refresh is
  also ~0.5s faster.
- Your chat list no longer re-reads every saved conversation after each
  message. Only files that changed get parsed again, and one pass now feeds
  both the list and the usage stats.
- The Windows certificate store loads off the startup path, once instead of
  up to four times, so the window appears sooner.
- Rift's built-in tool server: a file in a project's **second** folder is no
  longer reported "not found", and every tool now tells Claude whether it is
  read-only, so safe lookups can run side by side.
- Backend cleanup: a write-only metrics registry and an unused Windows
  job-object prototype were removed. The permission and question prompts now
  share one pending-request registry. `turn.rs` hands its prompt text and tool
  allowlist to their own files, and the enhance-prompt and chat-title helpers
  share one runner. `rustls` 0.23.45 clears RUSTSEC-2026-0285.

## Known issues

- Elevated windows cannot accept drag-and-drop from lower-integrity Explorer;
  use the attachment picker instead.
- Web Speech may mask profanity; the on-device Parakeet engine is verbatim.
