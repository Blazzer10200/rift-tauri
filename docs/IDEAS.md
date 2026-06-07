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

---

## "Grows with you" — Hermes-inspired memory/learning layer — captured 2026-06-07

Now **Pillar 3** in [design/rift-roadmap.md](design/rift-roadmap.md) — promoted out of pure backlog. Seed: **Hermes Agent** (NousResearch, 185k★, MIT, "the agent that grows with you") proves the market for an agent that remembers you + improves over time. Build Rift's own lean version on the harness data already collected. Next: read hermes-agent source for the design (what it remembers, how it adapts). Copy the idea, not the 300MB Python.

## Trending-mined ideas (GitHub trending sweep) — captured 2026-06-07

Parked from the trending scan; revisit when a pillar needs them:
- **Skills-GUI** — obra/superpowers (219k★, Claude skills framework). Rift as the visual browser/runner/manager for Claude Code skills — nobody has a good GUI for this. On-architecture, unique.
- **Generative UI / AG-UI protocol** — CopilotKit (33k★). Agents render interactive components (charts, diffs, forms) inline, not just markdown. Fits the "observable cockpit" thesis.
- **Drop-in agent memory** — MemPalace (54k★, MIT). Could seed Pillar 3's memory store.
- **Agent isolation / sandbox** — microsoft/mxc (Rust). Safety layer for multi-agent fan-out with workspace write access (Pillar 4).
- **Cheap agent web access** — Agent-Reach / last30days-skill (zero-API-fee web read) — keep fan-out off the metered pool.

## Deep trending re-sweep — captured 2026-06-07 (real, re-verified numbers)

> Star-count note (correction): the earlier "implausible" counts (superpowers 219k, hermes 185k, ECC 209k)
> are NOT hallucinated — GitHub's trending page really displays them. Whether GitHub's own numbers are
> gamed is unknowable from here, but treat the displayed figures as the source. Don't re-flag them as fake.

**The big reframe — Rift's pillars already exist as separate trending products.** Validation *and* a
competitive clock. The moat is *integration into one observable cockpit*; ship before the standalone
pieces define the category.

Direct overlaps / competitors:
- **`ryoppippi/ccusage`** (15.6k, Rust) — Pillar 2 already built as a CLI. Reads local coding-agent logs →
  token/cost; daily/monthly/session/**5h-block (Claude billing windows)**/per-project; cost via LiteLLM
  price table. **The blueprint for the cost cockpit.** → folded into [idea-phase-plan.md](design/idea-phase-plan.md).
- **`farion1231/cc-switch`** (93k, **Tauri+Rust**) — the escape hatch as a standalone product, same stack.
  Multi-provider list + 50+ presets + tray switch + SQLite store. Upgrade path for Rift's single-provider hatch.
- **`stablyai/orca`** (4.3k, TS) — Pillar 4 product: fleet of parallel agents, BYO subscription, desktop+mobile.
- **`ogulcancelik/herdr`** (4.8k, Rust) — terminal agent multiplexer (Pillar 4).

New seeds worth promoting (not in the list above):
- **Token-COMPRESSION layer** — `chopratejas/headroom` (16k). Cut spend, don't just measure it: deterministic
  local compression (AST-aware incl. Rust), `wrap claude` via `ANTHROPIC_BASE_URL` (same seam as the escape
  hatch). 47–92% token cut. Python-primary = opt-in, not hard dep. → Phase 3c in the plan.
- **Local code knowledge graph** — `colbymchenry/codegraph` (43k) + `Lum1104/Understand-Anything` (53k).
  Fewer tokens/tool-calls via pre-indexed local graph = cost lever **and** a Pillar 3 substrate.
- **Sandbox primitive** — `anthropic-experimental/sandbox-runtime` (4.3k, TS, official Anthropic) — OS-level
  FS/net restriction, no container. The missing safety piece for the edit-applying swarm. → Phase 3a.
- **Memory substrate decision (Pillar 3)** — `supermemoryai/supermemory` (25.9k, **TS**, matches frontend) vs
  MemPalace (Python) vs roll-your-own on Rift's SQLite. `EveryInc/compound-engineering-plugin` (20k) +
  `nesquena/hermes-webui` (13.7k, proves the "UI-over-agent" market) are design inspiration.
- **Agent proxy/gateway** — `agentgateway/agentgateway` (3.1k, Rust) — one layer for escape-hatch routing +
  harness observation.
- **Hash-anchored edits** — `can1357/oh-my-pi` (11k) — collision-safe parallel-edit pattern for the swarm (3b).
