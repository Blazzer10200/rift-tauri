# Contributing

Small, focused PRs land fastest. The practical guide lives in
[`DEVELOPING.md`](DEVELOPING.md) — §2 covers everything from clone to the
pre-PR checklist; [`ARCHITECTURE.md`](ARCHITECTURE.md) is the system map.

- **Bugs / feature requests** — open a GitHub issue. For bugs, include your
  Rift version (Settings → About), Claude CLI version (`claude --version`),
  and repro steps.
- **Security issues** — don't open a public issue; see
  [`SECURITY.md`](SECURITY.md) for private reporting.
- **Before opening a PR** — run the checks in DEVELOPING §2
  (`npm run check` + `cargo check`, `cargo test` if you touched anything
  testable), and don't bump versions — release tooling owns the three-file
  lockstep.
