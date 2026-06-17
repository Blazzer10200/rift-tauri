# Local model (qwen3.6-iq3-rift) — Rift local-mode stress test

Harness (`harness-rift.mjs`) faithfully replicates Rift's local mode: proxy :11435,
Anthropic /v1/messages, the EXACT Rift tool surface (Read/Edit/Write/Bash/Glob/Grep +
mcp__rift__* helpers) with the CLI's own name-rejection + Read-before-Write guard +
workspace path-jail, running the LIVE addendum extracted from `turn.rs`. Iterate the
prompt fast here; confirm the winner in the Rift UI. (Rounds 1-2 below used an earlier
`harness.mjs` with write_file/read_file/run_bash — kept for the findings history.)

## Findings

### Run 1 — build CLI todo app from scratch (prompt v1)
- 3 steps, 3 tool calls, **textLeak=false**, 11.2s (after cold load).
- Wrote code → ran → hit `require is not defined` (pkg is `type:module`) →
  **self-diagnosed** → fixed (renamed .cjs) → re-verified working output.
- Verdict: **strong agentic loop.** Build/run/diagnose/fix/verify all present.
- Nit: fixed via `.cjs` rename rather than converting to ESM — workaround, not ideal.

### Run 2 — find+fix subtle bug (prompt v1) — THE ORIGINAL FAILURE MODE
- Seeded `slugify.js` with a real bug; tests pass green but
  `slugify("  Hello,  World!  ")` → `"-hello-world-"`.
- 8 steps, 8 tool calls, **textLeak=false**, 13.6s.
- Model **read code + test, diagnosed the bug itself, fixed, added a test, ran it.**
  The old "tests pass = nothing to fix, like a dumbass" behavior is **GONE.**
- BUT fix was **incomplete**: added leading/trailing dash strip, **missed internal
  doubled dashes**. `"Foo -- Bar"` → `"foo----bar"`. Independent check: 4/5 cases.
- Root cause = small-model weakness: shallow edge-case coverage; its own test only
  re-checked the happy path.

### Run 3 — same find+fix, prompt v2 (added TESTING RIGOR clause) — WIN
- 11 steps, 11 tool calls, **textLeak=false**, 21.7s.
- Wrote a test with **varied cases** (leading/trailing, consecutive spaces, punctuation
  runs, internal dashes) — its OWN test **caught the internal-dash failure** v1 shipped.
- Self-corrected across 3 fix attempts, even fixed a wrong test expectation.
- Final fix: `.replace(/-+/g,"-").replace(/^-+/,"").replace(/-+$/,"")`.
- **Independent verification: 7/7** cases pass (incl. ones it never saw: `---Lead---`→`lead`,
  `a___b`→`ab`). v1 was 4/5; **v2 closed the gap completely.**
- DECISION: **v2 TESTING RIGOR clause baked into `turn.rs`.** Costs ~10s more (deeper
  iteration) but ships correct code instead of plausible-but-incomplete.

## Net verdict
The model is NOT stupid — it builds, runs, diagnoses, self-corrects, and ships working
code with no tool-leak. The original "tests pass = nothing to fix" failure was a PROMPT
defect (v0 said "act first, don't plan"), now fixed. Remaining small-model weakness =
edge-case shallowness, materially reduced by the TESTING RIGOR clause (4/5 → 7/7).
Frontier-model sharpness is still not matched (it's a 3-bit 35B), but it is now a
genuinely useful local build agent.

## Round 2 — FAITHFUL harness (`harness-rift.mjs`), 2026-06-17

Root cause of the 2026-06-17 "stupid" chat, found in the real CLI transcript
(`ac121010…jsonl`): the local model **mangled mcp__rift__ tool names** — called
bare `mcp__rift` and `mcp__rift_git_commit` (single underscore) → CLI rejected
both with "No such tool available". Also invented absolute paths (`/home/user/…`).
NOT model stupidity — the `mcp__servername__tool` convention doesn't survive a
3-bit model's tokenizer.

New harness offers the EXACT Rift tool surface (Read/Edit/Write/Bash/Glob/Grep +
mcp__rift__* helpers), replicates the CLI's name-rejection + Read-before-Write
guard + workspace path-jail, and runs the LIVE addendum extracted from turn.rs.

Prompt fix shipped to `turn.rs`:
- **All git → plain Bash** (`git commit -m …`), not mcp__rift__git_* (removed).
- **"MCP names are literal"** rule — exact `mcp__rift__NAME`, two underscores,
  copy verbatim, never collapse to one / never bare prefix; if unsure use native.
- **Always relative paths**, never absolute /home/user or C:/.

| # | Task | Steps | badNames | readBeforeWrite | textLeak | Verdict |
|---|------|-------|----------|-----------------|----------|---------|
| 1 | git commit (pre-jail) | 16 | **0** | 0 | no | named ok; thrashed on absolute path |
| 2 | git commit (jailed+hardened) | 3 | 0 | 0 | no | **clean** — relative path, Bash commit |
| 3 | multi-file calc build + edit | 7 | 0 | 0 | no | self-fixed ESM, 15 tests pass |
| 4 | ambiguous choice | 6 | 0 | 0 | no | asked in TEXT (under-uses ask_user card) |
| 5 | find+fix slugify (orig failure) | 10 | 0 | 0 | no | **diagnosed, Edit-after-Read recovery, 11/11, internal-dash caught** |
| 6 | heavy 2-commit git workflow | 6 | 0 | 0 | no | both commits + log + clean status |

**Net: every failure family from the real chats is gone.** 0 bad tool names
across all 6 tasks (was the headline bug). 0 path thrash after the relative-path
rule. Original "tests pass = nothing to fix" + edge-case shallowness stay fixed.
Remaining soft spot: model under-uses `mcp__rift__ask_user` (asks in text) — not
a breakage, left as-is to avoid over-steering.

## Round 3 — premature-termination + edit-precision battery, 2026-06-17

Root cause of the 2026-06-17 screenshot bug ("model read one file on 'debug the
codebase', described it, quit; on 'you didn't do what I requested' said 'I don't
see a request'"): NOT a context bug — `--resume` carries history fine. It was a
PROMPT defect — the old addendum said "keep replies short / bias toward action"
with **no completion rule and no conversation-memory rule**. The model quit after
step one and treated a follow-up as a brand-new empty request.

Prompt fixes shipped to `turn.rs` this round:
- **FINISH THE WHOLE TASK** — a broad ask ("debug the codebase", "audit this") means
  read ALL relevant files, FIND concrete problems, report a real findings list WITH
  fixes; keep calling tools across as many turns as it takes; describing one file is
  quitting after step one. "Done" only when the complete request is satisfied.
- **CONVERSATION MEMORY** — "you didn't do it" / "do it" / "continue" means the
  request is in an EARLIER message; re-read the conversation and act on it; never
  reply that you cannot see a request.
- **EDIT PRECISION** — old_string must be copied verbatim from Read, byte-for-byte
  incl. exact existing indentation; never retype from memory, never add tabs/spaces
  the file lacks; the `123\t` line-number prefix is NOT part of the file. On
  'old_string not found', re-Read the region or use Write — don't retry the same text.
- **CLAUDE_CODE_MAX_OUTPUT_TOKENS=8192** (turn.rs env) — stops mid-turn guillotining.

| # | Task | Steps | badNames | rBeforeW | turn2Amnesia | Verdict |
|---|------|-------|----------|----------|--------------|---------|
| A | chat-only (no task)            | 0  | 0 | 0 | —     | no spurious tools |
| B | build todo CLI + run           | ~3 | 0 | 0 | —     | built & ran clean |
| C | screenshot repro + "you didn't do it" | — | 0 | 0 | **false** | resumed original task; bug dead |
| D | audit 2 C# files + "fix all"   | 7  | 0 | 0 | **false** | fixed both; Edit-fail→cat-A→Write recovery |
| E | JSDoc all 4 math files         | 8  | 0 | 0 | —     | **all 4 edited** (batched), re-verified |
| F | build string lib + "continue"  | 11 | 0 | 0 | **false** | ESM+slugify debug; 22 tests pass (indep.) |
| G | tab-indented in-place Edit     | 3  | 0 | 0 | —     | **Edit 1st try, no whitespace fab** (was D's failure) |

**Net: the premature-termination + amnesia family is gone.** turn2Amnesia=false on
every multi-turn test (C/D/F). EDIT PRECISION (G) eliminated the phantom-`\t` Edit
failure that cost D a recovery round — model now copies Read bytes verbatim, Edit
succeeds first try. All metrics clean across the battery; every fix independently
verified by reading on-disk files / running generated tests, not model self-reports.

## Prompt versions
- `system-prompt.txt` = v1 (current `turn.rs`: IMAGES + DIAGNOSE-BEFORE-FIX clauses).
- `system-prompt-v2.txt` = v1 + TESTING RIGOR clause (handle whole bug family,
  vary test inputs, a fix passing only the reported example is INCOMPLETE).
- Live addendum now ~6817 chars; `system-prompt-rift-live.txt` is regenerated per
  run from `turn.rs` (gitignored) — that is the source of truth, not the v1/v2 files.
