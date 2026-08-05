# Rift Design System

Static reference bundle for Rift's visual language. Every card is a standalone
HTML document sharing `styles.css`; `_ds_manifest.json` indexes the cards for
gallery/tooling use. It is not part of the production app runtime. The source
components remain authoritative when a card drifts.

## Layout

```
design-system/
  styles.css              shared-token mirror + documentation primitives
  _ds_manifest.json       card, token, and brand-font index
  assets/                 rift-logo.png (brand mark)
  foundations/            colors · type · spacing-shape
  components/             buttons · pills-badges · menu · tool-chips · page-hero · forms · sidebar
  pages/                  home · chat · workspace · settings · ai-health (full workspace mockups)
```

15 cards: **3 foundations + 7 components + 5 pages.**

## Tokens + drift guard

One themeable hue (`--accent-h`, emerald 163) drives the accent ramp; status colors
(ok/warn/danger/info) are accent-independent. `styles.css` mirrors the tokens it
shares with `src/app.css`, not the app's complete stylesheet. When a shared token
changes, change both. The guard is automated:
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
