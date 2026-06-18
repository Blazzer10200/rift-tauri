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
  foundations/            colors · type · spacing-shape
  components/             buttons · pills-badges · menu · tool-chips · page-hero · environment-float · forms
  pages/                  home · chat · settings · local-llm  (full 1280×800 workspace mockups)
```

14 cards total: **3 foundations + 7 components + 4 pages.** Concepts and the old "home concept" deck were
dropped in the 2026-06-18 rebuild — this set documents what ships, nothing speculative.

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

## Accuracy notes (what the rebuild corrected)

- **Tool chips** are a *unified-accent* family — every category (read/write/shell/agent/meta) uses
  `--accent` for its icon; **status** (pending pulse / done dim / error danger-bar) drives color, not category.
  The old multi-color-per-category version was stale.
- **Environment float** is a *neutral* `--surface` git pill that only warms toward accent on hover — not the
  old info-tinted treatment.
- **Workspaces are four**: Home · Chat · Settings · Local LLM (kbd 1–4). The titlebar nav shows Home / Chat /
  Local LLM (Settings opens via the gear); Local LLM carries a warn experimental dot.

## Pushing to cloud

Use the **DesignSync** tool (ordering enforced: read → finalize_plan → write/delete). `finalize_plan` needs
both `writes` and `deletes` (pass `[]` when empty) and fires a permission prompt. After `write_files`,
re-push the patched `_ds_manifest.json` so the pane picks up the card list.
