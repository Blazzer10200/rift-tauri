# Ideas / Backlog

Future-reference ideas captured mid-session. Not committed work — seeds to revisit.

---

## Edit-applying swarm (write-mode, not just review) — captured 2026-06-04

**Origin:** during the read-only audit swarm (`scripts/.archive/audit-swarm.workflow.js`, ~550 agents, find→verify→synthesize), the question came up: if a parallel swarm can *review* the whole codebase accurately, can a swarm safely *apply the fixes* too — not just describe them?

**The hard part:** edit agents are riskier than review agents. A bad read just produces a wrong finding (cheap, caught by verify). A bad *edit* mutates source — and parallel editors can collide on the same file, apply half a change, or "fix" something based on a stale read. Reliability is the whole game.

**Why today's design doesn't transfer 1:1:** the audit swarm is safe *because* it's read-only and findings are independent. Edits are NOT independent — two fixes in the same file interact; later edits depend on earlier ones compiling.

**Design seeds (revisit later):**
- **Feed it the audit output.** The confirmed-findings list (with file:line + evidence + suggested_fix) is already a perfect work-list for an edit pass. The two swarms chain: audit → fix.
- **Isolation per agent.** Use `isolation: "worktree"` so each edit agent works on its own copy — no parallel-collision on shared files. Merge/verify after.
- **One file = one agent, serialized within file.** Never two agents editing the same file concurrently. Group findings by file; one agent owns all fixes in that file, applied sequentially.
- **Verify gate, same as review.** After each edit: `cargo check` / `npm run check` MUST pass before the change is accepted. Edit that breaks the build → auto-revert, flag for human.
- **Adversarial review of the diff** (mirror of the verify stage): a second agent reviews each applied diff against the original finding — "did this actually fix it without introducing a regression?" Reject on doubt.
- **Human approval tier.** critical/security fixes → never auto-apply; queue for review. low/medium mechanical fixes (e.g. swallowed-error, missing-await) → candidates for auto-apply behind the verify+diff-review gates.
- **Scope discipline carries over:** the lesson from the audit — *many agents, tiny bites* — applies. One finding-fix per agent turn, narrow context, can't-bail structure.

**Status:** idea only. Not scoped, not scheduled. Revisit after the audit report lands and we see how clean the suggested_fix fields actually are.
