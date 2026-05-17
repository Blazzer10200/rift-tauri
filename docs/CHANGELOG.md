# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.5-alpha — 2026-05-17 — Embedded Claude tool allowlist + STT alternates

**S91 priority 1 — permission denials in the Assistant tab.** Blazzer + Trey hit recurring "tool not in allowlist" / "permission denied" messages whenever the embedded Claude tried to spawn a subagent, run a background bash command, edit a notebook, etc. S88's fix added `Skill` to all three `--allowed-tools` branches in [src-tauri/src/assistant/mod.rs](src-tauri/src/assistant/mod.rs) but stopped short of the rest of the built-in surface. S91 widens to the full CLI built-in set: `Agent` (subagent spawning — used by `/plan`, `/quick-review`, `/check`), `AskUserQuestion`, `BashOutput` + `KillBash` + `KillShell` (the CLI auto-invokes these whenever Bash runs with `run_in_background: true`), `ExitPlanMode`, `MultiEdit`, `NotebookEdit`, `SlashCommand`. Refactored the three branch bodies to share a single `BUILTINS` const so future additions land in one place. MCP scope is unchanged — full-config keeps `mcp__*`, scoped branches keep the explicit `mcp__rift__*` entries (+ `mcp__rift__remote_bash` when the remote-shell toggle is on). Verified via CDP: a fresh Assistant tab successfully called Bash + spawned an Agent subagent with zero permission denials.

**S91 priority 2 — STT slurred-speech tolerance.** [src/lib/state/stt.svelte.ts](src/lib/state/stt.svelte.ts) requested only one recognition alternate (`r.maxAlternatives = 1`), so the engine's first guess won regardless of confidence. Bumped to 3 and added a `pickBestAlternate` helper in `onResult` that walks `results[i][0..length]` and returns the highest-confidence transcript. When WebView2's Azure backend bothers to score the alternates (sometimes it returns 0 for every one — spec-allowed), a cleaner lower-ranked variant can now win over the slurred primary. When all confidences are 0, behavior collapses back to `alt[0]` (no regression). Vocabulary hints + Azure-direct fallback deferred (stretch goals).

Verify: CDP-driven smoke test — fresh chat tab, "run pwd && ls then spawn a recon Agent to summarize" → assistant completed both calls cleanly, no denial strings in body text. svelte-check + cargo check both clean (auto-verifier).

