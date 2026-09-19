# Rift — Claude Code Instructions

Claude-native entrypoint for Rift. This file carries only what is Claude-specific
or not written down elsewhere.

**`AGENTS.md` is the shared project contract — read it first, every session.**
Stack, verification policy, provider rules, and load-bearing guardrails live
there and apply identically to Claude. Do not restate or fork them here; if a
rule changes, it changes in `AGENTS.md`.

## Start Here

1. Read `AGENTS.md` — the shared contract.
2. Read `docs/HANDOFF.md` before project work, and `docs/ISSUES.md` when the task
   touches a known bug or deferred item. Both are gitignored working notes and
   are absent from a fresh clone or a cloud container — that is expected, not an
   error. Treat their absence as "no in-flight context", not as missing files to
   recreate.
3. `git status --short` and preserve user-owned changes.
4. Search before creating files, components, commands, or abstractions. Rift has
   deliberately removed whole subsystems; `docs/ARCHITECTURE.md` closes with the
   list that must stay removed. Git history is the archive.

## Orientation

`docs/ARCHITECTURE.md` is the durable system map and has a fast-navigation table
keyed by the kind of change you are making. Start there rather than crawling the
tree. `docs/DEVELOPING.md` covers running, building, and releasing;
`docs/SECURITY.md` the threat model; `docs/CHATGPT.md` the ChatGPT route,
model, effort, and billing contracts.

Two orientation facts that are easy to get wrong:

- `TabState` (`src/lib/state/assistant.svelte.ts`) is the frontend's unit of
  truth — workspace, conversation, provider/model, permissions, and draft all
  hang off the tab, never off a global. The workspace hub selection is only a
  default.
- `src-tauri/src/lib.rs` registers the whole Tauri command surface at the bottom
  of `run()`. Command bodies live under `commands/`, one file per domain.

## Verification

Run the gates from `AGENTS.md`. What that file does not say is which gates are
reachable from where:

| Environment | Available |
|---|---|
| Windows dev machine | Everything. `npm run verify` is the complete gate. |
| Linux / cloud container | `npm run check`, `npm test`, and `cargo build` after installing the system deps in `docs/DEVELOPING.md`. |

Clippy and `cargo test` still need Windows or CI. A Rust change made in a cloud
session is **unverified** in that sense even when it compiles — say so plainly
rather than implying the suite passed. CI (self-hosted Windows) runs
`cargo clippy -D warnings` plus the Rust tests and is the real signal.

The Linux build needs `--no-default-features` (the default `parakeet` feature
pulls an ONNX Runtime with no Linux prebuilt for the Windows-only `directml`
provider) and the GTK/WebKit/ALSA packages listed in `docs/DEVELOPING.md`. It
can be run headless under Xvfb to check layout, flows, and provider detection —
but the renderer is WebKitGTK, not WebView2, so never present it as evidence
about pixels, blur, or font rendering.

`npm test` and `npm run check` both run `svelte-kit sync` first by design:
`tsconfig.json` extends the generated `.svelte-kit/tsconfig.json`, and without it
Vite fails to resolve `node:module` with a misleading "Tsconfig not found". Do
not remove those `sync` prefixes.

Node must be 24.x (`.node-version`, `engines`). On Node 22 the Vite 8 / Rolldown
toolchain fails at startup.

## Working on the running app

`.claude/skills/rift-ui/` is the app-navigation skill: inspect, interact with,
and visually verify the running desktop UI through Rift's CDP bridge, instead of
guessing at selectors or taking the user's focus. It defers to
`.agents/skills/rift-ui/SKILL.md`, which is the single source of truth for the
bridge's commands.

## Honesty Requirements

Rift's docs are precise, and its guardrails assume that precision holds.

- Do not call a provider path verified until its real authenticated route ran. A
  passing contract test is not a live account call.
- Do not report a gate as passing that you could not run in this environment.
- Prefer deleting state that has gone write-only over migrating it. The repo has
  a track record of recording such removals in the code comment that replaces
  them; keep that habit.

## Continuity

Update `docs/HANDOFF.md` when work remains in flight or the user asks to save
progress: exact checks run, results, open risks, and the next executable step.
In an ephemeral cloud container that file does not survive the session — put
durable conclusions in the commit message and the PR body instead.
