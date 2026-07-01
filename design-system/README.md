# Rift Design System — local mirror

Source-of-truth mirror for the **Rift** design-system project on `claude.ai/design`. Every card is a
self-contained HTML doc that links the shared `styles.css`; the cloud pane renders exactly the files listed
in `_ds_manifest.json`'s `cards` array. This folder is rebuilt to be **100% accurate to the live app**
(verified against real component source + CDP screenshots of all four workspaces), not aspirational.

## Layout

```
design-system/
  styles.css              canonical token + doc-primitive stylesheet (mirrors src/app.css 1:1)
  _ds_manifest.json       the ONLY thing the cloud pane reads — cards[] + tokens[] + brandFonts[]
  assets/                 rift-logo.png (real brand mark, mirrored from src/lib/assets — used by rail mockups)
  foundations/            colors · type · spacing-shape
  components/             buttons · pills-badges · menu · tool-chips · page-hero · forms · sidebar
  pages/                  home · chat · workspace · settings · ai-health  (full ~1400×880 workspace mockups)
  _briefs/                point-in-time design briefs (history, NOT manifest cards) — see _briefs/README.md
```

15 cards total: **3 foundations + 7 components + 5 pages.** Every card is CDP-verified against the live app
(2026-07-01 pass) — this set documents what ships, nothing speculative.

## Tokens — Graphite Ink, dark only

One themeable hue (`--accent-h: 163`, emerald) drives the entire accent ramp; status colors
(ok/warn/danger/info) are accent-independent. Surfaces step `--bg 0.142` → `--bg-inset 0.178` →
`--bg-elev-1/2/3` (0.215/0.262/0.300). Text ladder `--fg 0.925` down to `--fg-faint 0.430`. All values
live in `styles.css :root` and are duplicated into `_ds_manifest.json` `tokens[]` for the pane's token panel.
**`styles.css` mirrors `src/app.css`** — when the real tokens change, change both.

## The manifest-card rule (read before pushing)

The Design System pane renders **only** entries in `_ds_manifest.json`'s `cards` array. `@dsCard` first-line
markers in each HTML file are the human-readable intent, but they only fold into the manifest when the app's
in-app self-check recompiles it — which does **not** run on an external DesignSync push. So after writing
files to the cloud you must also push a `cards[]` that already lists every doc. Keep the `@dsCard` marker and
the manifest entry in agreement.

Each card entry: `{ "path", "group", "name", "subtitle" }`. `group` is one of `Foundations` / `Components` /
`Pages` (drives the pane's section grouping). Leave `tokens` / `brandFonts` / `globalCssPaths` intact.

## Accuracy notes (what the 2026-07-01 pass corrected)

- **Navigation moved to the sidebar.** The app body has NO titlebar nav — the C+ switcher-led `Sidebar`
  owns navigation via a footer icon strip (Workspace · Chat · AI Health), with Settings in the status strip.
  A minimal `Topbar` (title + window controls) sits above the content; a `StatusBar` grounds the bottom.
  (`Titlebar.svelte` with the old horizontal nav survives ONLY in onboarding `setupMode`.) The old page
  mockups that drew a titlebar nav + tab strip were stale and were rebuilt on this chrome.
- **New `components/sidebar.html`** documents that rail: switcher (+ branch pill) → New chat + search →
  scope segment → grouped conversation list (pinned wash · active bar · working dots · hover pin/more) →
  footer icon nav → status strip (model + connection dot).
- **`Local LLM` became `AI Health`.** The old Ollama-endpoint cockpit was replaced by a usage dashboard
  (analyze-usage hero, API-latency banner, plan-limit windows + credits, speed & efficiency KPIs). The
  `pages/local-llm.html` mockup was deleted; `pages/ai-health.html` is its live-accurate replacement.
- **Settings gained a theme system.** Appearance now has a Theme/Layout sub-nav, a 6-preset theme grid
  (Graphite · Midnight · Ember · Orchid · Forest · Focus), and an accent hue slider + Vividness slider —
  not just the old single accent-swatch row.
- **Tool chips** are a *unified-accent* family — every category (read/write/shell/agent/meta) uses
  `--accent` for its icon; **status** (pending pulse / done dim / error danger-bar) drives color, not category.

## Pushing to cloud

Use the **DesignSync** tool (ordering enforced: read → finalize_plan → write/delete). `finalize_plan` needs
both `writes` and `deletes` (pass `[]` when empty) and fires a permission prompt. After `write_files`,
re-push the patched `_ds_manifest.json` so the pane picks up the card list.
