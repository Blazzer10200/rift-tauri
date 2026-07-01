# Rift Design System — local mirror

Source-of-truth mirror for the **Rift** design-system project on `claude.ai/design`. Every card is a
self-contained HTML doc that links the shared `styles.css`; the cloud pane renders exactly the files listed
in `_ds_manifest.json`'s `cards` array. This folder is rebuilt to be **100% accurate to the live app**
(verified against real component source + CDP screenshots of all four workspaces), not aspirational.

## Workflow — how design (Claude) and code (Svelte) stay accurate on both ends

Rift's app is **Svelte 5 / SvelteKit**, but Claude Design's canvas runtime is **React-only** — compiled
Svelte can't render there, and the `/design-sync` *skill* that auto-extracts a design system assumes a React
stack (`anthropics/claude-code#71523`, open). So we do **not** try to make Claude Design render Svelte.
Instead we run a **layered** setup where each side owns what it's good at:

| Layer | Where it lives | Job |
|---|---|---|
| **Tokens** (the real contract) | `src/app.css` `:root` → mirrored to `styles.css` + `_ds_manifest.json` `tokens[]` | The one thing shared verbatim across design + code. Colors / space / type / motion. |
| **Visual reference** | these HTML cards (authored by us, pushed via DesignSync) | What each surface *looks like* — reference docs, never compiled. |
| **Real components** | `.svelte` files in `src/` | The actual UI. Claude Code writes these; Claude Design never does. |
| **Codebase context** *(optional)* | claude.ai/design → `+` → **Link local code** / GitHub | Lets Claude Design *read* our Svelte for smarter, on-brand generations. Additive context, NOT a source of truth — it does not replace the cards or render Svelte. |

**What keeps it honest (so nobody has to remember):**
- **Token drift** → `check-tokens.mjs` runs inside `npm run check`, which **CI runs on every PR + push**
  (`.github/workflows/check.yml`). If `styles.css`/manifest fall out of sync with `src/app.css`, the check
  goes red. This is the load-bearing guard — the token layer is the only thing shared byte-for-byte.
- **Visual drift** → rebuild the affected card from live component source + a CDP screenshot, then re-push.
  There's no automated guard here (HTML mockups can't self-verify); treat cards as reference, refresh on
  UI changes.

**Traps (learned the hard way — don't):**
- Don't run the `/design-sync` **skill** to auto-generate the system from this Svelte repo — it's React-only
  and will proceed as if React is implied. We author the HTML cards ourselves and push via the DesignSync
  tool, which sidesteps that.
- Don't treat **Link local code** as a replacement for the design-system cards — it's context, not the
  validated brand contract.
- Don't expect the canvas to preview Svelte. For a *real* Svelte component playground, the future move is
  Storybook (`@storybook/addon-svelte-csf`) alongside — not inside — Claude Design.

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

### Drift guard — `check-tokens.mjs`

Don't rely on remembering to update the mirror. `node design-system/check-tokens.mjs` (also
`npm run check:tokens`, and folded into `npm run check`) extracts every `--token` from both
`src/app.css` and `styles.css`, resolves `var()` aliases so an alias and its literal compare equal,
and **fails on any shared token whose resolved value differs** — plus a manifest cross-check
(`_ds_manifest.json` `tokens[]` must equal the styles.css value, since the pane renders the manifest).
`app-only` / `ds-only` tokens are advisory (never fail). This is the automated version of the manual
diff; run it before pushing token changes to the cloud.

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
