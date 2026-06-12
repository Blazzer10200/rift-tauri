# Ideas / Backlog

Future-reference seeds captured mid-session. Not committed work.

> **2026-06-12 pivot note:** the four-pillar strategy (cost cockpit · provider escape hatch · memory layer · multi-agent) and its design docs (`rift-roadmap.md`, `idea-phase-plan.md`, `edit-swarm-safety-layer.md`) were retired when v0.9.0 shipped the **minimal core** — those features were removed, not parked. Recover any of it via `git log`. The seeds below are the ones that still make sense for a minimal-core Rift.

## Skills-GUI — visual browser/runner for Claude Code skills

Nobody has a good GUI for browsing, running, and managing Claude Code skills (seed: obra/superpowers). On-architecture for Rift (it already shells the CLI), unique, and doesn't re-grow the cockpit. Strongest surviving idea.

## Generative UI / AG-UI protocol

Agents render interactive components (charts, diffs, forms) inline instead of only markdown (seed: CopilotKit). Fits Rift's "observable assistant" thesis; the SessionDiff overlay is a hand-rolled instance of this pattern.

## "Grows with you" memory/learning layer

An observational layer over per-turn telemetry ("Rift noticed you…"), inspired by Hermes Agent. The in-memory `telemetry.turns` substrate survives the strip; durable capture would have to be rebuilt deliberately if this is ever picked up.

## Cheap agent web access

Zero-API-fee web read for agents (seed: Agent-Reach / last30days-skill) — keeps any future fan-out off the metered pool.

## Hash-anchored edits

Collision-safe parallel-edit pattern (seed: can1357/oh-my-pi) — relevant only if multi-agent editing ever returns.
