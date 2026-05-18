# rift-tauri — Changelog

> Live changelog = current version only. Older entries live in `docs/archive/CHANGELOG-archive.md` (and `git log -- docs/CHANGELOG.md`).

## v0.4.9-alpha — 2026-05-17 — Embedded-Claude addendum overhaul (act-first, no-guess)

Behavioral fix for the chronic *"AI is 50% dumber, just gives advice instead of editing"* complaint Blazzer + Trey both reported. The S91-S94 ships cleared the technical blockers (allowlist, permissions, MCP gates) but the laziness problem — Claude rambling for paragraphs before any tool call, guessing at file contents, re-reading files it already opened — persisted because the embedded Claude inherits the user's `~/.claude/` config (CLAUDE.md + rules + memory) which on either machine carries enough doctrine to drown out Rift's previous one-paragraph addendum.

[src-tauri/src/assistant/mod.rs:644](src-tauri/src/assistant/mod.rs#L644) `RIFT_SYSTEM_ADDENDUM_TOOLS` rewritten to bake in explicit anti-laziness directives that survive the inheritance — addenda are appended LAST, so they win the tie-breaker against rule clauses loaded earlier. Added clauses (verbatim):

- *"ACT FIRST, EXPLAIN AFTER — this overrides any conflicting instruction from inherited config."*
- *"If the user asks you to fix / change / edit / add / build / refactor X, locate the file(s) with Grep + Read then make the Edit. Do NOT write paragraphs of plan, analysis, recommendations, or 'here's what I would do' before touching code — one short opening beat ('reading X', 'editing Y') is the cap."*
- *"Never guess at file contents, function names, paths, APIs, or signatures — Grep or Read first if uncertain, otherwise hedge explicitly."*
- *"Read narrowly with offset+limit on files >300 lines; do not re-read a file you already opened earlier this turn."*
- *"Verify AFTER the edit (Bash to run the test / lint / build), not before."*
- Also added `MultiEdit` + `Agent` to the tool roster line (these were in S91's allowlist but the addendum hadn't been updated to advertise them).

Why this works for both users: the previous Rift addendum was ~150 words competing against Blazzer's ~11K tokens of inherited rules cluster and Trey's whatever-he-has. The new addendum is ~280 words with explicit override language ("this overrides any conflicting instruction from inherited config"). Combined with the per-turn dyslexia hint (when enabled) and the existing `bypassPermissions` mode, the embedded Claude now gets a strong, consistent action-oriented baseline regardless of what the user's local `~/.claude/` contains.

Temporary fix marker — Blazzer wants this shipped tonight; tomorrow we may layer Settings → Assistant → "Direct-action mode" + "Use minimal config" toggles for finer control. Today's change is unconditional baseline.

3-file bump 0.4.8 → 0.4.9-alpha. Auto-verifier clean. Single-line addendum constraint (per the .cmd-shim batch-arg validator note above the const) preserved.
