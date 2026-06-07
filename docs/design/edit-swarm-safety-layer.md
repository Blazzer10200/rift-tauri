# Phase 3a — Write-mode safety layer: evaluation + chosen architecture

> Companion to [idea-phase-plan.md](idea-phase-plan.md) §2 Phase 3 and the parked 3b seed in
> [../IDEAS.md](../IDEAS.md). Written 2026-06-07 (cont.70, Session E). Decision: **build a cross-platform
> worktree+verify harness; defer OS-level FS/net sandboxing to an optional non-Windows tier.**

## 1. The question

3b (edit-applying swarm) fans out write-capable agents. Before that ships it needs a *safety layer*.
The plan named two candidates to evaluate and prototype one:
`anthropic-experimental/sandbox-runtime` (TS) vs `NVIDIA/OpenShell` (Rust).

The decisive constraint: **Rift is Windows-first** (Tauri 2 desktop, the user runs Win11 Pro). Both
candidates are built on Unix-kernel primitives.

## 2. Evaluation (researched + cross-verified 2026-06-07)

| Dimension | `sandbox-runtime` | `OpenShell` |
|---|---|---|
| **Native Windows** | ❌ "Not yet supported" (issue #105 open, #17 closed unresolved) | ❌ — **WSL2 only** |
| **Mechanism** | macOS `sandbox-exec`/Seatbelt + Linux `bubblewrap`; proxy net filter | Linux-only kernel LSMs: **Landlock + seccomp BPF** + HTTP CONNECT/OPA proxy |
| **Shape** | Lightweight CLI **+ Node library** (`SandboxManager.wrapWithSandbox`), no container | Control-plane: gateway + policy engine + sandbox, runs in a **K3s cluster**; needs Docker/Podman/MicroVM |
| **Language** | TS / Node | Rust core (`openshell-sandbox` crates) — but it's a K8s service, not a linkable lib |
| **Maturity** | "Beta Research Preview" | "**Alpha — single-player proof-of-life**", NVIDIA/GPU-oriented |
| **Pedigree** | Exactly what Claude Code's own `/sandbox` wraps | NVIDIA infra play for agent-fleet-at-scale |

### Verdict
- **OpenShell — rejected.** A Dockerized/K3s control-plane with Linux-only kernel LSMs, alpha, GPU-flavored.
  The opposite of an embeddable desktop safety primitive. Its Rust is a Kubernetes service, not a library.
- **sandbox-runtime — right design, wrong platform.** Lightweight, no container, clean library API, and it's
  the same engine CC's `/sandbox` uses — but it **does not run on native Windows**. On Windows it works only
  *inside* WSL2.

**Neither is a drop-in write-mode sandbox on Rift's primary platform.** Both assume a Unix kernel.

## 3. What Rift's write surface actually is

Rift's MCP server is **read + git only** ([mcp_server.rs](../../src-tauri/src/assistant/mcp_server.rs) —
`read_file/list_dir/grep` + `git_*`, writes gated behind trust ≥ "standard"). File **writes happen through
the Claude CLI subprocess's own** Write/Edit/Bash tools. So the 3b safety layer = constraining those
claude-CLI children and their working trees — not adding a sandbox around Rift's MCP.

This reframes "safety": for a swarm the first-order risk is **blast radius** (parallel agents corrupting a
shared tree, a bad edit breaking the build) — not arbitrary syscalls. Blast-radius containment is
cross-platform and ships today; OS-level syscall/net restriction is a second, deferrable tier.

## 4. Chosen architecture — the worktree + verify harness

Matches the parked 3b seed in IDEAS.md exactly:

```
confirmed-findings[]  (from the audit swarm: file:line + evidence + suggested_fix)
        │  group by file
        ▼
  ┌──────────────────────────────────────────────────────────┐
  │  per file → one agent (serialized within file)            │
  │  1. git worktree add --detach <tmp> HEAD   (isolation)    │
  │  2. junction node_modules (+ shared CARGO_TARGET_DIR)     │
  │  3. claude-CLI child applies hash-anchored edits          │
  │  4. VERIFY GATE: npm run check / cargo check  ── fail ──▶ auto-revert (discard worktree), flag for human
  │  5. ADVERSARIAL DIFF REVIEW: 2nd agent: "fix the finding, no regression?" ── reject ──▶ discard
  │  6. pass+accept → cherry-pick/merge the single-file diff back to main
  │  7. cleanup: remove junction (rmdir, NOT recursive), then git worktree remove
  └──────────────────────────────────────────────────────────┘
```

- **Isolation** = `isolation: "worktree"` per agent (one-file-one-agent, no shared-file collision).
- **Verify gate** = the same commands `/check` runs — break the build → auto-revert.
- **Adversarial review** = mirror of the audit's verify stage, diff-vs-finding.
- **Hash-anchored edits** (`oh-my-pi` pattern) keep parallel edits collision-safe.

Cross-platform, no new heavy dependency, and it *is* the spine of 3b regardless of any sandbox choice.

### Windows frictions discovered (the value of prototyping)
1. **A fresh worktree has no `node_modules`** (gitignored) → svelte-check can't run. Solution: **junction**
   `node_modules` from the main checkout into the worktree (`New-Item -ItemType Junction`). Same problem for
   Rust: a worktree has no `target/` → cold `cargo check` (minutes). Solution: shared `CARGO_TARGET_DIR`
   (cheaper than a per-worktree cold build; trade-off: serializes cargo across worktrees).
2. **Junction cleanup is dangerous.** `Remove-Item -Recurse` / `git worktree remove --force` can follow a
   junction and delete the *target's* contents (the real node_modules). MUST remove the junction first with
   `cmd /c rmdir` (removes the reparse point only, leaves target intact), *then* remove the worktree.
3. **Don't collide with a running `tauri dev`.** Per project rule, never `cargo check` the main checkout while
   dev runs. A separate worktree has its own dir, but to be safe the gate during a dev session should prefer
   the frontend (`npm run check`, read-only) or run the Rust gate only when dev is quit.

## 5. Optional OS-sandbox tier (deferred, non-Windows)

When the swarm runs on Linux/macOS — or inside WSL2 on a Windows host — layer `sandbox-runtime` (or simply
`claude --sandbox`, same engine) around each claude-CLI child for true FS/net restriction:
`SandboxManager.wrapWithSandbox(cmd)` → FS allow = workspace + that agent's worktree; net = proxy allowlist.
This is additive to the harness, not a replacement. On native Windows the feature is unavailable and
degrades to the worktree+verify blast-radius safety. Revisit native Windows isolation (AppContainer /
restricted token / Job Objects — Chromium's approach) only if a real threat model demands it; CC itself does
not sandbox on native Windows either.

## 6. Prototype

`scripts/proto/swarm-harness.ps1` — proves the load-bearing mechanic on Windows end-to-end:
worktree add → node_modules junction → apply edit → `npm run check` gate → verdict (pass/auto-revert) →
safe junction+worktree cleanup, **main tree never touched**. Runs two agents — one clean edit (gate PASS),
one type-error edit (gate FAIL → auto-revert) — to prove the gate discriminates.

Run: `pwsh -File scripts/proto/swarm-harness.ps1` (or `powershell -NoProfile -File ...`). Read-only against
the main checkout; safe to run while dev is alive (frontend gate only).

## 7. Open questions — RESOLVED + built (cont.71, 2026-06-07, `db01c70`)
- **Cargo gate during dev → dedicated target dir.** The worktree `cargo check` uses a DEDICATED, persistent
  `CARGO_TARGET_DIR` (`%TEMP%/rift-swarm-target`) disjoint from dev/main, so it never collides with a running
  `tauri dev` — no quit-dev, no fragile process detection. (Third option, cleaner than either original.)
- **Merge on accept → `git cherry-pick`.** The orchestrator commits the single-file diff (`commit --only -- <file>`)
  in the worktree, then cherry-picks that sha onto main. One-file-one-agent ⇒ no conflict; aborts on the rare clash.
- **Module home → new `src-tauri/src/swarm/`.** assistant/mod.rs is already the largest file; the swarm reuses
  `assistant::claude_command()` but lives separately.
- **Adversarial review → dedicated diff-vs-finding prompt.** Pure reasoning over the diff text (no tools);
  verdict `{accept,reason}`, unparseable → reject (fail-safe).

**Built + verified:** Rust orchestrator + `swarm_run`/`swarm_env_check` commands + Harness→Swarm UI sub-tab.
Proven: `cargo check` 0/0, `npm run check` 0/0, a deterministic `#[ignore]` mechanics test
(`swarm_mechanics_discriminates_and_isolates`: gate discrimination + main-tree isolation + no worktree leak),
AND a live end-to-end LLM run (edit agent → review accept → cherry-pick → `merged:true`, main tree intact, zero
leak). **Not yet shipped** (committed only). **3c (compression toggle) is the remaining Phase-3 item.**
