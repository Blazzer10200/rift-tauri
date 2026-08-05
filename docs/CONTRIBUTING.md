# Contributing

Small, focused PRs land fastest. The practical guide lives in
[`DEVELOPING.md`](DEVELOPING.md) — §2 covers everything from clone to the
pre-PR checklist; [`ARCHITECTURE.md`](ARCHITECTURE.md) is the system map.

- **Bugs / feature requests** — open a GitHub issue. For bugs, include your
  Rift version (Settings → System), active provider/route, the relevant Claude
  or Codex CLI version, and repro steps.
- **Security issues** — don't open a public issue; see
  [`SECURITY.md`](SECURITY.md) for private reporting.
- **Be respectful** — participation is governed by the
  [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) (Contributor Covenant).
- **Before opening a PR** — run the checks in DEVELOPING §2
  (`npm run check`, `npm test`, and Cargo with the `src-tauri/Cargo.toml`
  manifest), and don't bump versions — release tooling owns version lockstep.
