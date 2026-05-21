# Plan — Rift git + RCON tooling for the Assistant

Status: v2.2 draft 2026-05-20 · scope: design, not implementation
Reframe: **autonomy-first.** Goal is "Claude wields Rift productively w/ minimum ceremony." One-time setup, then no friction.

## 1. Summary

- Add two new capability surfaces to Rift's Assistant MCP server: **local git** ops + **FiveM RCON** control.
- One unified setting: **Assistant trust level** (Read-only / Standard / Full) — no per-tool toggles, no per-call confirm dialogs.
- Auto-detect workspace shape on open (`.git`? `fxmanifest.lua`? `server.cfg`?) and self-configure where possible.
- Pre-checks fail soft + self-heal (auto-stash on dirty pull, wait-then-push on busy sync queue) — both gated by workspace-config flags so first-week rollout can be conservative.
- Add a `dev_cycle` bundle tool that chains commit → push → restart → verify into a single Claude call.
- Git impl: shell to system `git` (inherits SSH agent / credential helper). RCON impl: ~80-LoC module w/ ~30-LoC tokio UDP protocol core, ported from `dev/scripts/rcon.sh`.

## 2. Architecture

### Existing anchors

| What | Where |
|---|---|
| MCP tool defs + dispatch | `src-tauri/src/assistant/mcp_server.rs:512-602` |
| Gate envs (`bridge_enabled`, `remote_shell_enabled`) | `mcp_server.rs:294-304` |
| Loopback bridge protocol | `remote_bridge.rs:40-179` |
| Bridge dispatch (`match req.op`) | `remote_bridge.rs:165-179` |
| Workspace roots threaded into MCP tools | `roots: &[PathBuf]` (e.g. `mcp_server.rs:133, 154, 199`) |
| Active engine / folders | `engine(app).folders_clone()` — `remote_bridge.rs:181-185, 196-197` |
| Cross-user advisory lock pattern | `shell_lock_key` — `remote_bridge.rs:187-190` (we'll add `rcon_lock_key` alongside) |

### New modules

```
src-tauri/src/assistant/
  mcp_server.rs        ← + tool_git_*, tool_rcon_resource, tool_dev_cycle
  remote_bridge.rs     ← + ops: rcon_lookup, sync_drain_wait, rcon_execute
  rcon.rs              ← NEW ~80 LoC. UDP RCON client + response parser. No Tauri deps.
  git_local.rs         ← NEW ~150 LoC. Shells out to `git`, scoped to workspace root.
  workspace_config.rs  ← NEW ~100 LoC. Loads + writes `.rift/config.toml`, auto-detects on first open.
```

## 3. The one setting

In Rift's Assistant Settings page, replace today's `Allow remote shell` checkbox w/ a single radio:

```
Assistant trust level:
  ( ) Read-only       — read_file, list_dir, grep, sync_status, git_status, git_diff, git_log
  (•) Standard        — adds git pull/commit/push, rcon_resource (lifecycle), dev_cycle
  ( ) Full            — adds remote_bash + rcon_raw passthrough
```

`Standard` is the recommended default once a workspace is detected as a real project. The MCP child receives `RIFT_TRUST_LEVEL=readonly|standard|full` env at spawn. Tool listing + dispatch both check it. Two failure modes (model name-collides w/ unavailable tool / model fabricates a tool name) both reject server-side.

**Migration on upgrade:** existing users w/ `Allow remote shell` enabled auto-migrate to `Full`. Existing users w/ it disabled migrate to `Read-only` (safe default — they explicitly opted out of one tool, so don't grant five new ones silently). One-time notification in the Settings page on first launch after upgrade explains the new model + lets them flip to `Standard`.

## 4. Auto-detection on workspace open

When Rift opens a workspace, scan once:

| Detected | Action |
|---|---|
| `.git/` exists | enable git tools at current trust level (no prompt) |
| `fxmanifest.lua` anywhere ≤3 levels down OR `server.cfg` in workspace root | prompt once: "FiveM workspace detected — enable RCON? [Y/n]". On yes, ask for host/port + env var name for password (defaults: `192.168.1.170:30120`, `RCON_PASSWORD`). Write `.rift/config.toml`. |
| Neither | skip silently |

`.rift/config.toml` lives at workspace root, gitignored by convention. Format:

```toml
[rcon]
host = "192.168.1.170"
port = 30120
password_env = "ENDURE_RCON_PASSWORD"
snapshot_path = "txData/Qbox_F8F761.base/resources/[endure]/endure_devbridge/snapshot.json"

[git]
# Optional. Defaults shown.
auto_stash_on_pull = false        # set true after first week if behavior is comfortable
push_anyway_on_stuck_queue = false # ditto — see §6
```

Password never on disk. `password_env` resolved at call time from Rift's environment (which inherits from the user's shell on app launch). If the env var is missing → RCON tool returns a clear setup error.

## 5. Per-tool spec

### Git tools

All git tools run inside the MCP child via `Command::new("git").current_dir(workspace_root)` w/ the hardened env from §11. Output captured, parsed, returned as text.

| Tool | Trust gate | Behavior |
|---|---|---|
| `git_status` | read-only | `git status --porcelain=v1 -b`, formatted human-readable |
| `git_diff` | read-only | `git diff [--cached] -- <path>`, truncated at 64 KB w/ suffix |
| `git_log` | read-only | `git log --oneline -n <max>`, max ≤100 |
| `git_pull` | standard | `git pull --ff-only` (default) or `--rebase`. Dirty tree → refuse w/ stash command in error, OR (if `git.auto_stash_on_pull=true`) auto-stash → pull → pop. |
| `git_commit` | standard | `git add <paths\|--all>` + `git commit -m <msg>`. Refuse if message empty or nothing staged. |
| `git_push` | standard | Pre-check: if sync queue has pending uploads, wait ≤10 s for drain. If still pending: refuse w/ queue status (default), OR push anyway w/ warning (if `git.push_anyway_on_stuck_queue=true`). Then `git push <remote> <branch>`. |

`--force` push is not implemented. If the model passes `force: true` the tool rejects.

### RCON tool

#### `rcon_resource`
- Trust gate: **standard**
- Params: `{ action: "restart"|"ensure"|"start"|"stop"|"refresh", target?: string, verify?: bool (default true) }`
- Behavior: parent-bridge op `rcon_lookup` returns the workspace's RCON config + resolved password. Tool sends UDP packet, parses response, returns `rcon.sh`-shaped output. If `verify=true` and action ∈ {restart, start, ensure, stop}, polls `snapshot_path` (from config) for up to 6 s until observed state matches expected.
- Cross-user safety: takes a new workspace-scoped `rcon_lock_key` advisory lock (parallel to `shell_lock_key`, keyed on `<remote_root>/.rift-rcon`) for the duration of the call. Separate from the shell lock so a shell op and a `refresh` can run concurrently, but two simultaneous `stop`s can't.

#### `rcon_raw` (full trust only)
- Params: `{ command: string }`
- Same UDP transport, no response parsing, no verify. Reaches `quit`, txAdmin-level commands.
- Blocklist enforced server-side — see §11.

### Bundled tool — the autonomy payoff

#### `dev_cycle`
- Trust gate: **standard**
- Params: `{ message: string, resource?: string, paths?: string[] }`
- Behavior, in order:
  1. `git add <paths>` (or `-A` if omitted)
  2. `git commit -m <message>` — abort cycle on nothing-to-commit (return ok, note "no changes")
  3. wait ≤10 s for sync queue drain
  4. `git push origin <current-branch>`
  5. if `resource` provided + workspace has RCON config: `rcon_resource(action=restart, target=resource, verify=true)`
- Returns one structured response: per-step status, total wall-clock, final state. On any step failure: stop, return verbatim error from that step + what already happened (so a partial commit isn't invisible).

This is the "make it easy" tool. Common case = one Claude call, ~5 s wall-clock, your terminal stays closed.

## 6. Soft-fail / self-heal policies

| Case | Decision |
|---|---|
| `git_pull` on dirty tree | Default: refuse w/ copy-pasteable `git stash` command in the error. Workspace opt-in (`auto_stash_on_pull=true`) flips to auto-stash → pull → pop, surfacing stash sha. |
| `git_push` w/ pending sync uploads | Default: wait ≤10 s, then refuse if still pending (return queue status). Workspace opt-in (`push_anyway_on_stuck_queue=true`) flips to push-anyway w/ warning. |
| `rcon_resource` first time per session | No dialog. Standard trust = pre-approved. |
| Destructive RCON (`stop`) | No dialog. Same trust gate as other actions. Advisory lock prevents races. |
| Merge conflict on pull | Always surface — fail loud, return the unmerged paths list. Never pretend it worked. |
| Pull auto-stash pop conflicts | Keep the stash, surface "auto-stash conflicts on pop, your changes are saved as `stash@{0}`, resolve manually". |
| RCON resource not found | Surface verbatim `✗ resource not found`. |
| Wrong RCON password | Surface clearly + point at the env var name (without printing the value). |

**Why both auto-stash and push-anyway default to OFF in MVP:** I'm <85% confident on these two calls (see §8). The conservative default protects against the failure mode I haven't seen yet; the workspace flag lets the user flip them on once the system has proven boring. After ~1 week of real use, if no surprises, defaults can move to ON in a minor version.

Loud failure ≠ friction. The rule is: never silently swallow, but also never gate routine work behind a click.

## 7. Decisions table (revised)

| # | Unknown | Decision |
|---|---|---|
| 1 | Surface (MCP / UI / both) | MCP first. UI is phase 2 (the Rust core is the same anyway). |
| 2 | `git_commit` MVP? | **Yes** — autonomy requires the full chain |
| 3 | RCON named vs raw | **Named in standard, raw in full** — same module, different trust gate |
| 4 | Per-workspace config | `.rift/config.toml` written by auto-detect prompt, not by hand |
| 5 | Approval model | **One trust-level radio, no per-call dialogs** |
| 6 | Git impl | Shell to system `git` (auth via SSH agent) |
| 7 | RCON impl | Raw tokio UDP, ~30 LoC core, ~80 LoC module |
| 8 | Sync lock coord | Git ops: no lock (local fs). RCON destructive ops: new `rcon_lock_key` parallel to existing `shell_lock_key`. |
| 9 | Git push auth | System git config — Rift stays out of credentials |
| 10 | Push w/ pending sync queue | **Wait-then-refuse** by default, opt-in flag for wait-then-push |
| 11 | `dev_cycle` bundle | **Ship in MVP** — it's the autonomy payoff in tool form |
| 12 | Pull on dirty tree | **Refuse w/ copy-pasteable stash hint** by default, opt-in flag for auto-stash |

## 8. Risks + open questions

- **Loosens Endure RP "Local-Only Hard Rule"** intentionally. That CLAUDE.md was written when RCON meant "Claude shells `bash dev/scripts/rcon.sh` w/ no automation guardrails". The new tool moves the same capability into Rift w/ lock coordination + verify. Update `FiveM Server/CLAUDE.md` once shipped: replace the "user owns push + restart" section w/ "Claude owns push + restart via Rift tooling; user owns shipping changes to prod via terminal as a fallback".

- **Trey parity** — concrete setup checklist for any new collaborator:
  1. Install Rift (same minor version as the others on the team)
  2. Install Git for Windows (or system git on Linux/macOS) — gives `git.exe` + `sh.exe` on PATH
  3. SSH key added to GitHub + `ssh-agent` running (Windows: `Get-Service ssh-agent | Set-Service -StartupType Automatic; Start-Service ssh-agent; ssh-add <keypath>`)
  4. RCON password exported in shell rc as the env var named in `.rift/config.toml` (e.g. `export ENDURE_RCON_PASSWORD=...`)
  5. Launch Rift from that shell so it inherits the env. (App-launcher icon launches won't inherit — document this.)
  6. Open the workspace; auto-detect prompts to enable RCON; accept. Tools light up.

- **Trey credential helper unknown.** Risk: ~5%. If his git uses a Windows Credential Manager flavor that doesn't work in child processes, `git_push` will hang or fail w/ a non-obvious error. First-push test on his box will tell us. Mitigation: the `GIT_TERMINAL_PROMPT=0` env (§11) at least makes it fail fast instead of hanging.

- **`dev_cycle` wall-clock target (≤8s) is unvalidated.** Could be 5s, could be 20s if sync drain has to wait the full 10s. Measure on first run; if drain consistently blocks, revisit the wait window.

- **Open**: should `dev_cycle` ever try to recover from a push reject (e.g. non-FF)? MVP: no — surface the reject, let me fetch/rebase explicitly. Auto-rebase-and-retry is a phase-3 footgun.

- **Open**: how does the auto-detect prompt UI work in Rift's current Svelte structure? Needs a non-modal toast w/ inline form. If that's a heavier lift than expected, fall back to "write `.rift/config.toml.example`, surface a one-liner in Settings, user fills it in". Decide during impl.

## 9. Phased rollout

**Phase 1 (MVP) — ship the autonomy loop**
- `workspace_config.rs` + auto-detect prompt at workspace open
- `rcon.rs` + bridge op `rcon_lookup`
- `git_local.rs`
- Tools: `git_status/diff/log/pull/commit/push`, `rcon_resource`, `dev_cycle`
- Settings page: single trust-level radio replacing `Allow remote shell` + migration toast

**Phase 2 — polish + edge cases**
- `rcon_raw` behind full trust
- UI buttons in Rift toolbar wrapping the same Tauri commands
- Native snapshot parsing (drop the script's `jq` dep)
- Multi-remote RCON (dev/staging/prod targets in one workspace config)

**Phase 3 — power-user**
- Branch ops, stash, fetch-without-pull
- Push pre-hook (run `cargo check` / `npm run check` before push)
- Auto-rebase on non-FF push reject (off by default)
- Pre-commit secret-scan (GitHub MCP pattern — block commits containing recognizable secret formats)

## 10. Test plan

**End-to-end smoke (the headline test):**

> Edit `endure_zombies/config.lua` via `Edit` tool → `dev_cycle(message="lower spawn rate", resource="endure_zombies")` → expect: commit sha returned, push reports `1 commit pushed`, RCON returns `✓ verified: endure_zombies is started`, total wall-clock ≤8 s, zero user clicks.

**Per-tool verification matrix:**

| Tool | Verify |
|---|---|
| `git_status` | Dirty file → expect `M path/to/file`. Non-repo dir → clean error. |
| `git_diff` | Modify file, expect unified diff. >64 KB diff → truncation suffix. |
| `git_log` | ≥3 commits → 3 rows. `max=0` rejected. |
| `git_pull` | Clean repo behind origin → fast-forwards. Dirty repo (default config) → refuses w/ stash hint. Dirty repo + `auto_stash_on_pull=true` → stashes, pulls, pops cleanly. Stash-pop conflict → stash preserved, clear error. Non-FF remote → fails cleanly, no merge commit. |
| `git_commit` | Stage + commit, expect `[branch sha] message`. Empty message → refused. Nothing to commit → refused. |
| `git_push` | After commit, push to test remote. Pending sync queue (default) → wait+refuse. `push_anyway=true` → push w/ warning. `force: true` arg → rejected. |
| `rcon_resource restart endure_zombies` | Snapshot reports `started`. Missing target → `✗ resource not found`. Wrong password → `✗ invalid RCON password` (no password in error). Server down → timeout error. |
| `rcon_resource refresh` | Add resource folder, refresh, expect it in next snapshot. |
| `rcon_resource stop` | Concurrent stops from Blazzer + Trey → second one blocks on `rcon_lock_key`, then runs. No race. |
| Security: arg injection | Pass `branch: "main; rm -rf ~"` → rejected at regex layer, never reaches `Command`. |
| Security: path traversal | Pass `path: "../../../etc/passwd"` → rejected at canonicalize layer. |

## 11. Security hardening (added 2026-05-20 after web research)

Web search surfaced two relevant 2026 incidents that shape the impl:

- **Anthropic's official MCP git server CVEs (Jan 2026)** — argument injection via insufficient param validation. Researchers used prompt injection to push malicious git CLI args through tool calls, reaching RCE in chained scenarios. Root cause: 18 unconstrained string params w/ no `maxLength` / `pattern` / `enum` validation.
- **Anthropic's reference server still refuses push/pull/merge after 14 months of community requests** — "commit but don't push" is the conservative baseline. Our plan deliberately diverges (autonomy goal), so the validation bar is higher, not lower.

### Mandatory hardening in `git_local.rs`

1. **Every string param that becomes a CLI arg gets validated against a strict regex before passing to `Command`:**
   - `path`: must canonicalize inside workspace root, no `..` traversal, no leading `-` (prevents `--upload-pack=…` style injection).
   - `branch` / `remote` / `ref`: `^[A-Za-z0-9._/-]{1,200}$`.
   - `message`: capped at 4 KB, no embedded NULs.
   - `paths[]`: every entry validated independently w/ path rule.
2. **All git invocations use `--` separator** before any user-controlled paths: `git diff --cached -- <path>`. Prevents `<path>` being parsed as a flag.
3. **No shell.** Always `Command::new("git").args([...])` w/ pre-split args, never `Command::new("sh").arg("-c").arg(...)`.
4. **Refuse `force`/`-f` everywhere.** Whitelist of allowed subcommands per tool — `git push` tool only allows `push` + remote + branch, rejects any other arg.

### Mandatory env on `git` child spawn (Windows-specific gotchas)

Web search confirmed several Windows landmines:

```rust
cmd.env("GIT_TERMINAL_PROMPT", "0")        // fail fast, don't hang waiting for cred input
   .env("GCM_INTERACTIVE", "never")        // no modal popups from Git Credential Manager
   .env("GIT_ASKPASS", "")                 // no askpass fallback
   .env_remove("GIT_DIR")                  // don't inherit a foreign GIT_DIR
   .current_dir(workspace_root);
```

**Do NOT clear PATH** — credential helpers are shell scripts that need `sh.exe` from Git for Windows on PATH. Preserve PATH, HOME, USERPROFILE, SSH_AUTH_SOCK, GIT_SSH, and all `GIT_*` vars.

**SSH agent on Windows:** if push fails w/ auth error, surface a clear "is `ssh-agent` running? `Get-Service ssh-agent | Set-Service -StartupType Automatic; Start-Service ssh-agent`" message rather than a raw libssh2 error. Same applies for Trey.

### RCON-side hardening

1. `target` param validated as `^[A-Za-z0-9_\-]{1,64}$` — FiveM resource names are this character set. Anything else rejected before being concat'd into the rcon command string.
2. `password_env` value name validated as `^[A-Z][A-Z0-9_]{0,64}$` so a malicious config can't pull env values it shouldn't.
3. Password never logged, never appears in error messages or telemetry.
4. `rcon_raw` (phase 2, full trust): explicit blocklist of dangerous tokens — `quit`, `sv_endmatch`, `txAdmin-`. Surface these as errors at the tool layer, don't trust the model.

### What this changes about the autonomy story

The autonomy stance stands — **trust level Standard still means no per-call dialogs**. Hardening is server-side validation, invisible in the happy path. The principle is "trust the trust setting, distrust the params on every call." A compromised conversation or prompt-injected tool arg shouldn't be able to escape the workspace or run arbitrary git args.

This is the same posture as the existing `tool_remote_bash` / `tool_grep` — they don't pop dialogs either, but they validate their args.

## 12. Out of scope

- Merge conflict resolution (always surface, never attempt)
- GitHub PR creation (use a future MCP tool or `gh` via `remote_bash`)
- Force push, force-with-lease (phase 3)
- Server cvar editing via RCON (phase 2 raw passthrough)
- Per-resource secrets management — env vars only in MVP. OS keychain integration (Windows Credential Manager / macOS Keychain / libsecret) is a phase 3 nice-to-have but not on the critical path.
- Multi-repo workspaces (one git tree per workspace in MVP; phase 3 if it bites)
