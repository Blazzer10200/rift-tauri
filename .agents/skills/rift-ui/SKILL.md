---
name: rift-ui
description: Inspect, navigate, interact with, and visually verify the Rift Tauri desktop UI through its local WebView2 CDP bridge. Use for Rift UI implementation, layout checks, app navigation, console-error checks, accessibility inspection, screenshots, or reproducing frontend behavior in the running desktop app.
---

# Rift UI

Use Rift's existing CDP bridge before generic desktop screen control. It exposes
the WebView DOM, the dev-only assistant store, accessibility structure, console
errors, and screenshots without taking focus from the user.

Run commands from the repository root.

## Choose the Cheapest Reliable Probe

- Start with `bash scripts/cdp/c.sh inspect [selector]`. It combines live state,
  current-generation console errors, and the accessibility tree in one
  screenshot-free request.
- Use `map [selector]` when navigating an unfamiliar surface. It lists every
  visible actionable control with a verified selector ready for `act click`.
- Use `peek` for state and errors only, `text` for exact rendered copy, `find`
  to discover a robust selector, and `measure` for computed layout and tokens.
- Use `look [selector]` only for visual claims such as spacing, overlap, color,
  contrast, or alignment. Open the returned absolute path with the local image
  viewer at original detail when exact pixels matter.
- Use desktop screen control only for native title bars, OS dialogs, UAC, or
  surfaces outside the WebView.

## Connect

Check before launching another process:

```bash
npm run cdp:doctor
```

If the app and bridge are not running, use the supported medium-integrity
launcher, then start the wrapper if needed:

```bash
npm run cdp:dev
npm run cdp:serve
```

Long-lived launchers must run in the background. Reuse healthy processes.

## Inspect and Act

```bash
bash scripts/cdp/c.sh inspect
bash scripts/cdp/c.sh inspect ".settings-page"
bash scripts/cdp/c.sh map
bash scripts/cdp/c.sh map ".settings-page"
bash scripts/cdp/c.sh find "Settings"
bash scripts/cdp/c.sh text ".chat"
bash scripts/cdp/c.sh measure ".sidebar"
bash scripts/cdp/c.sh act click '[aria-label="Settings"]'
bash scripts/cdp/c.sh act key "Ctrl+4" ".sb-main"
bash scripts/cdp/c.sh look ".settings-page"
```

Use `act`, not a click followed by a foreground sleep. It performs the action,
waits for DOM quiescence, then returns state, errors, and a screenshot. If a
selector is unknown, run `map` for the whole actionable surface or `find` for a
known label rather than guessing.

For the embedded browser target, put target selection before the command:

```bash
bash scripts/cdp/c.sh -t browser inspect
bash scripts/cdp/c.sh -t browser look
```

## Verify Honestly

- Structural or copy claim: cite `inspect`, `ax`, `text`, or store state.
- Visual claim: inspect the returned screenshot; DOM data cannot prove pixels.
- Interaction claim: use `act` and check its action verdict plus settled result.
- Runtime-error claim: check current-generation errors; stale errors are evidence
  of a prior page generation, not the current one.
- If the bridge reports DOM-scrape fallback, treat store-level claims as degraded.
- If a check was not run, say so rather than inferring success.

## Safety

- Never kill `rift-tauri.exe` by image name. Use the path-scoped `reap` command
  only when cleanup is required.
- Do not run `cargo check` while `tauri dev` is running.
- Do not use screen navigation when the user asks for focus-safe work; CDP is
  focus-safe for WebView inspection and interaction.
- Keep screenshots in `scripts/cdp/.tmp`; they are generated scratch artifacts.
