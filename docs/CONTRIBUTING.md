# Contributing — build from source

For working on Rift itself. End-user install lives in [`ONBOARDING.md`](ONBOARDING.md).

## Prerequisites

- **Windows 11** (primary target). macOS / Linux build but aren't packaged.
- **Rust** stable (1.78+). Install via [rustup](https://rustup.rs/).
- **Node.js** 20+ via [`nvm-windows`](https://github.com/coreybutler/nvm-windows) or installer.
- **`npm`** — *not* pnpm (the lockfile is npm).
- **Git Bash** for shell scripts. PowerShell works for everything except a couple of dev helpers.

## Clone + bootstrap

```bash
git clone https://github.com/Blazzer10200/rift-tauri.git
cd rift-tauri
npm install
```

The first `cargo` build under `src-tauri/` is slow (~5 min cold) — pure-Rust russh + Tauri compile a lot. Subsequent incremental builds are seconds.

## Run dev

```bash
scripts/run-dev.bat        # red-tinted icon, separate from installed Rift
# or
npm run tauri dev
```

The dev process watches `src/` (Svelte) + `src-tauri/src/` (Rust) and hot-reloads. **Don't run `cargo check` while dev is alive** — it kills the running dev process via incremental-rebuild collision.

## Project layout

| Where | What |
|---|---|
| `src/` | SvelteKit frontend (Svelte 5 runes, Tailwind 4) |
| `src-tauri/src/` | Rust backend — Tauri commands + russh + auto-sync engine |
| `src-tauri/capabilities/` | Tauri 2 permission grants |
| `docs/` | Live state — read `HANDOFF.md` first each session |
| `scripts/` | Dev launcher + release pipeline |

## Before opening a PR

- `npm run check` — Svelte + TS clean
- `cargo check --manifest-path src-tauri/Cargo.toml` — Rust clean
- `cargo test --manifest-path src-tauri/Cargo.toml` if you touched anything testable
- Don't bump versions in `package.json` / `Cargo.toml` / `tauri.conf.json` — release pipeline handles those in lockstep.

## Releases

Maintainers only — `scripts/release.ps1` drives bump → build → `vpk pack` → `vpk upload`.
