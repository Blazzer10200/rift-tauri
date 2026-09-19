---
name: rift-ui
description: Inspect, navigate, interact with, and visually verify the Rift Tauri desktop UI through its local WebView2 CDP bridge. Use for Rift UI implementation, layout checks, app navigation, console-error checks, accessibility inspection, screenshots, or reproducing frontend behavior in the running desktop app.
---

# Rift UI

Rift already has a CDP bridge for driving its own WebView. Use it before any
generic desktop screen control: it reads the DOM, the dev-only assistant store,
accessibility structure, console errors, and pixels without taking focus from
the user.

**Read `.agents/skills/rift-ui/SKILL.md` now — it is the single source of truth
for this bridge** (probe selection, the full command surface, the embedded
browser target, honesty rules, and safety constraints). This file exists only so
the skill is reachable from Claude Code; do not fork its contents.

Run everything from the repository root.

## The 10-second version

```bash
npm run cdp:doctor                       # is the app + bridge up?
npm run cdp:dev                          # supported launcher (background it)
bash scripts/cdp/c.sh inspect            # state + console errors + a11y, no screenshot
bash scripts/cdp/c.sh map                # actionable controls with verified selectors
bash scripts/cdp/c.sh act click "<sel>"  # act, settle, verify in one request
bash scripts/cdp/c.sh look "<sel>"       # only when the claim is visual
```

Prefer `act` over a click followed by a sleep. Prefer `inspect`/`map` over
guessing a selector. Use `look` only to support a claim about pixels.

## Non-negotiable

- Never kill `rift-tauri.exe` by image name — an installed Rift shares it.
  Cleanup is PID/path-scoped (`reap`) only.
- Never run `cargo check` while `tauri dev` is running; they share build state.
- Requires a running Windows desktop app. In a Linux or cloud container this
  bridge is unavailable — say the check could not be run rather than inferring
  that the UI is fine.
