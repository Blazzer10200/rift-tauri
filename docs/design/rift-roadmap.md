# Rift — Roadmap & Vision (the map)

> Where Rift is going, why, what's done, and what's next. Written 2026-06-07.
> Plain-English on purpose — read top to bottom any time you feel lost.
> **The how lives in [idea-phase-plan.md](idea-phase-plan.md)** — file-accurate build sheet. This is the why.

---

## The one-line vision

**Rift is the cockpit for Claude Code.** It sits on top of Claude Code, watches every
session, and turns that data into something useful — so you spend less, work better,
and the tool gets smarter the more you use it.

## Why now — June 15, 2026

Anthropic is changing how subscriptions work on June 15. Plain version:

- **Interactive** Claude (typing in the terminal/app) keeps using your normal subscription.
- **Programmatic** Claude — which is how Rift runs under the hood (`claude -p`) — moves to a
  **separate, metered credit pool** ($20 Pro / $100 Max-5x / $200 Max-20x), billed at full
  price, no rollover. When it runs out, the automation stops.

So Rift sits on the squeezed side. That's not a disaster — it's the **reason the whole plan
exists**:

- It makes the **escape hatch** (use cheaper providers) necessary.
- It makes the **cost cockpit** (watch your spend) something everyone suddenly needs.

The people who can *see* and *control* their usage win. That's Rift's whole job.

## Where we are today (already built)

- **The harness** — Rift already watches each session: cost, turns, tools, tokens, cache,
  speed. This is the foundation everything else stands on. *(You built this. It's the prize.)*
- **The escape hatch** — NEW (2026-06-07). A setting that routes Rift to cheaper AI providers
  (DeepSeek, etc.) instead of Claude-only. Built, tested live in the running app, one bug
  found + fixed, verified working. **Not shipped yet** (in the working tree, no version bump).
- **Hardening pass** — 70+ robustness/security fixes (prior session). Committed,
  **not shipped** (owed a live smoke-test first).

---

## The plan — four pillars

### Pillar 1 — Don't be locked to Claude (the escape hatch) → mostly done
- **What:** route turns to cheaper providers when you want.
- **Why:** survives June 15.
- **Left:** ship it; optionally route the side-features (prompt-enhance, compaction) too.

### Pillar 2 — The fuel gauge (cost cockpit) → next big build
- **What:** turn the harness into a real spend dashboard for the new credit pool. See how much
  of your monthly credit is left, what's burning it, and get warned before it runs dry.
- **Why:** after June 15 everyone's on a limited budget — this is the feature people come for.
- **Good news (verified 2026-06-07):** Rift *already saves* the per-turn data (cost, tokens, cache,
  speed) to disk. So this is mostly **aggregating + pricing + budgeting**, not building from scratch.
- **Blueprint:** `ryoppippi/ccusage` already does this as a CLI — copy its data model, price-table
  approach, and its 5-hour-block view (which maps to Claude's billing windows). Beat it on *visuals +
  live cockpit*. **Our differentiator:** `headroom`-style compression can *cut* spend, not just show it.

### Pillar 3 — Grows with you (the heart of it) → the long game
- **What:** a memory/learning layer on top of the harness data. Rift remembers how you work,
  spots patterns, surfaces insights, and improves over time.
- **Why:** this is your real vision — "improve Claude Code over time with its data."
- **Inspiration:** **Hermes Agent** (Nous Research, 185k stars, MIT-licensed, tagline
  "the agent that grows with you") proves people want exactly this. We copy the *idea*, not the
  Python code — build our own lean version on the data Rift already has.
- **Good news:** the substrate is the *same* saved per-turn data the cost cockpit uses. Pillar 3 is a
  *read/learn layer* over it — start observational ("Rift noticed you…"), no new capture needed.
- **Substrate choice (open):** `supermemory` (TS, matches our frontend) vs MemPalace (Python) vs
  roll-your-own over Rift's SQLite. `compound-engineering-plugin` = design inspiration for *what* to learn.

### Pillar 4 — Many agents, cheaply (multi-agent) → later
- **What:** run several agents at once — cheap models for grunt work, Claude for the hard
  orchestration — and watch them all in the harness.
- **Why:** fan-out is too expensive at full price, but the escape hatch + cost cockpit make it
  affordable and visible. This is the "runs ridiculous" setup, made safe and watchable.
- **Heads-up:** `stablyai/orca` already ships a "fleet of parallel agents, bring your own subscription"
  product. The safety piece we were missing now exists: `anthropic-experimental/sandbox-runtime`
  (official, OS-level limits, no container) — which also de-risks the parked edit-applying swarm idea.

---

## The order we do it in

- **Phase 0 — now / around June 15:** ship the escape hatch + the hardening pass. Insurance in
  place before the change hits.
- **Phase 1:** cost cockpit — the fuel gauge for the credit pool.
- **Phase 2:** "grows with you" foundation — start the memory/learning layer.
- **Phase 3:** multi-agent cost-aware routing.
- **Anytime / parked:** the back-burner ideas in [IDEAS.md](../IDEAS.md).

## Next session — first jobs

1. **Ship what's done:** commit + ship the escape hatch and the hardening pass (smoke-test,
   bump the 3 version files + `Cargo.lock`, `/git-ship`).
2. **Study Hermes:** read NousResearch/hermes-agent source for the "grows with you" design —
   what it remembers, how it adapts — and decide what Rift copies.
3. **Scope the cost cockpit** (Pillar 2) once the above is clear.

## Parked (not now) — see [IDEAS.md](../IDEAS.md)

Skills-GUI (run/manage Claude Code skills visually), generative UI (agents draw rich UI, not
just text), drop-in agent memory, agent isolation/sandboxing. Good ideas — revisit when a
pillar needs them.
