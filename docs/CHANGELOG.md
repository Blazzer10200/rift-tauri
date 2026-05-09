# rift-tauri — Changelog

> Live changelog = current version only. Older entries archive to `archive/CHANGELOG-archive.md` on bump.

## v0.2.1-alpha — 2026-05-09 — Refresh build + SSH commit signing

Maintenance bump w/ no code changes. Triggered by signing-pipeline configuration + first end-to-end verified build of the post-S13 codebase.

### Repo / signing
- SSH commit signing live globally — `gpg.format=ssh`, `user.signingkey=~/.ssh/id_ed25519.pub`, `commit.gpgsign=true`, `tag.gpgsign=true`. Verified by GitHub on push (`reason: valid`, green Verified badge). `~/.config/git/allowed_signers` configured for local `git log --show-signature` verification.
- Sessions 11+12+13 commit (`6e9d5f1`) landed on `origin/main` — pre-signing config, stays unverified by design (no force-push).

### Build pipeline
- Bumped `Cargo.toml`, `package.json`, `tauri.conf.json` → 0.2.1-alpha.
- Fresh NSIS installer at `src-tauri/target/release/bundle/nsis/Rift_0.2.1-alpha_x64-setup.exe`.
- Silent install + desktop shortcut refresh + icon cache bust + explorer restart.

v0.2.0-alpha entry archived.
