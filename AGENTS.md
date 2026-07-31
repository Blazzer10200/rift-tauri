# Rift Codex Instructions

This file is the Codex-native entrypoint for Rift. `CLAUDE.md` remains useful
project reference material; direct user requests and this file control Codex
workflow.

## Start Here

1. Read `docs/HANDOFF.md` before project work. Read `docs/ISSUES.md` when the
   task touches a known bug or deferred item.
2. Inspect `git status --short` and preserve user-owned changes.
3. Search before creating files, components, commands, or abstractions.

## Stack and Verification

- Desktop shell: Tauri 2 / Rust under `src-tauri/`.
- UI: SvelteKit / Svelte 5 under `src/`; use runes, not legacy stores.
- Routine checks: `npm run check`, `npm test`, and focused Rust tests from
  `src-tauri/` when Rust changes.
- Do not run `cargo check` while `tauri dev` is running. Do not run a production
  build as a routine correctness check.

## Rift UI Workflow

Use the repo-local `$rift-ui` skill for app navigation, inspection, interaction,
and screenshots. The preferred tool is `scripts/cdp/c.sh`: it reads WebView DOM,
Rift dev stores, accessibility state, console errors, and pixels directly.

- Start structural checks with `bash scripts/cdp/c.sh inspect`.
- Discover actionable controls and verified selectors with `bash scripts/cdp/c.sh map`.
- Use `act` for action, render settle, and verification in one request.
- Use `look` or selector screenshots only when the claim is visual.
- Use desktop screen control only for native OS chrome, permission prompts, or
  other surfaces outside the WebView.
- Diagnose availability with `npm run cdp:doctor`. The supported launcher is
  `npm run cdp:dev`.

Never kill `rift-tauri.exe` by image name. Cleanup must stay PID/path-scoped so
an installed Rift instance cannot be touched. Do not recursively delete CDP
scratch data; remove only confirmed generated files.

## Provider Work

Rift has two production assistant routes: Claude through the official CLI and
OpenAI through the native Responses API. Keep provider work additive and
provider-neutral:

- Keep existing Claude behavior working while extracting shared contracts.
- Treat model names, capabilities, auth, streaming events, tool calls, usage,
  and conversation metadata as provider data rather than UI constants.
- Preserve existing conversations and settings through migrations.
- OpenAI uses an API key, not a ChatGPT subscription. Keep that distinction
  explicit in product copy and diagnostics.
- Preserve `store: false`, local history ownership, OS-keychain secrets, and
  shared workspace/permission enforcement on the OpenAI route.
- Do not label a provider feature verified until its real authenticated path is
  tested; distinguish automated contract coverage from a live account call.

## Continuity

Update `docs/HANDOFF.md` when work remains in flight or the user asks to save
progress. Record exact checks run, results, open risks, and the next executable
step.
