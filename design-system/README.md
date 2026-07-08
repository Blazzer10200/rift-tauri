# Rift Design System

Reference mirror of Rift's visual language. Every card is a self-contained HTML doc
sharing `styles.css`; the in-app **Design System pane** renders exactly the files
listed in `_ds_manifest.json`'s `cards` array. The set documents what ships —
rebuilt against real component source, not aspirational.

## Layout

```
design-system/
  styles.css              canonical token + doc-primitive stylesheet (mirrors src/app.css 1:1)
  _ds_manifest.json       what the pane renders — cards[] + tokens[] + brandFonts[]
  assets/                 rift-logo.png (brand mark)
  foundations/            colors · type · spacing-shape
  components/             buttons · pills-badges · menu · tool-chips · page-hero · forms · sidebar
  pages/                  home · chat · workspace · settings · ai-health (full workspace mockups)
```

15 cards: **3 foundations + 7 components + 5 pages.**

## Tokens + drift guard

One themeable hue (`--accent-h`, emerald 163) drives the accent ramp; status colors
(ok/warn/danger/info) are accent-independent. **`styles.css` mirrors `src/app.css`** —
when the real tokens change, change both. The guard is automated:
`node design-system/check-tokens.mjs` (folded into `npm run check`, which CI runs on
every push) extracts every `--token` from both files, resolves `var()` aliases, and
fails on any shared token whose resolved value differs — plus a cross-check that
`_ds_manifest.json` `tokens[]` matches `styles.css`. Cards have no automated guard:
treat them as reference and refresh from live component source when the UI changes.

## The manifest-card rule

The pane renders **only** entries in `_ds_manifest.json`'s `cards` array
(`{ "path", "group", "name", "subtitle" }`; `group` is `Foundations` / `Components` /
`Pages`). Each HTML file opens with a human-readable `@dsCard` marker — keep the
marker and its manifest entry in agreement.
