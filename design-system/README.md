# Rift Design System — local mirror

Local half of the **Claude Design** project *Rift Design System* (claude.ai/design,
project `3d8d7dda-6c1b-4c60-bbea-27786867ea7e`). Each file is a self-contained
preview (HTML + inline CSS, no build step) that renders as a card in the Design
System pane. Tokens are copied verbatim from `src/app.css` — when the real tokens
change, update the matching preview here.

## Layout

| Path | Card group | Source of truth |
|---|---|---|
| `foundations/colors.html` | Foundations | `src/app.css` surface stack, accent ramp, status, text ladder |
| `foundations/type.html` | Foundations | `--font-ui`/`--font-mono`, `--fs-*` scale |
| `foundations/spacing-shape.html` | Foundations | radius / density / shadow / motion tokens |
| `components/tool-chips.html` | Components | `ToolChip.svelte` `.chip` lane |
| `components/buttons.html` | Components | `app.css` `.btn` family |
| `components/pills-badges.html` | Components | `.pill` / `.count-pip` / `.kbd` |
| `components/menu.html` | Components | `.rift-menu` dropdown language |
| `components/page-hero.html` | Components | `shared/PageHero.svelte` |
| `components/environment-float.html` | Components | `environment/EnvironmentFloat.svelte` |

The first line of every preview is a `<!-- @dsCard group="…" name="…" subtitle="…" -->`
marker — that drives the card index in the Design System pane (no explicit
`register_assets` needed).

## Sync workflow (in Rift)

The CLI's `DesignSync` tool (admitted through Rift's allowlist as of v0.20.4) drives it.
Ordering is enforced: **read → finalize_plan → write/delete**.

1. `list_files` / `get_file` — inspect the cloud project.
2. Edit / add previews under this directory.
3. `finalize_plan` with the exact `writes` (+ `deletes`) — fires a permission prompt
   showing the path list and source dir.
4. `write_files` with each `{ path, localPath }` — `localPath` is relative to the repo
   root (the plan's `localDir`); the tool reads from disk and uploads directly.

Auth: rides the user's claude.ai login (design scopes are granted on first use) —
no separate `/design-login` needed in Rift. Cloud-only; dead under local-LLM/`--bare`.
