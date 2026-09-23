# rift-tauri — Changelog

> Current release only. Older release notes remain in Git history and on the
> [GitHub releases page](https://github.com/Blazzer10200/rift-tauri/releases).

## Unreleased — Current Claude, more control

- The model picker and usage views now know the current Claude line-up: Fable
  5.1, Opus 5, Sonnet 5, and Haiku 4.5, with the full effort ladder through
  **Max** on capable models.
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

## v0.158.0 — Calmer chat, clearer control

- Alerts is now one reliable footer destination with a full-size hit target,
  readable label, correctly placed unread badge, visible keyboard focus, and a
  notification panel that stays anchored inside the window.
- Older conversation history loads in batches of 40 instead of rendering the
  entire archive at once, keeping large workspaces responsive while preserving
  access to every chat.
- Completed-work receipts open as compact summaries. Terminal output stays
  collapsed until requested, and one **Expand output** action can reveal or
  collapse every terminal result in the receipt.
- Message copy and retry actions are easier to find, user messages can be copied
  directly, and the turn navigator now sits beside the reading column with
  larger controls and clearer accessibility semantics.
- The composer has steadier guidance and larger attachment, dictation, model,
  context, and send targets. Windows commands display readable path separators,
  while copied command text remains unchanged.
- The new-chat launchpad makes better use of wide panes, falls back to a calm
  single-column layout in split view, and gives **Switch folder**, **View
  activity**, and **Continue** actions more deliberate emphasis.
- Workspace transitions now use a structured loading skeleton, status actions
  announce what they open, and natural-language lists keep their intended
  single-column reading order.

## Known issues

- Elevated windows cannot accept drag-and-drop from lower-integrity Explorer;
  use the attachment picker instead.
- Web Speech may mask profanity; the on-device Parakeet engine is verbatim.
