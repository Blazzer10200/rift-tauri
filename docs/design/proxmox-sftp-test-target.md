# Proxmox SFTP integration-test target — design

> Mapped + **BUILT 2026-05-30** (Opus session). Goal: convert Rift's largest untested
> surface (live SFTP sync — #265 / #21) into testable surface using the home Proxmox box.
> **Status: LIVE.** Container 121 created, provisioned, verified (SSH + SFTP put/rename/
> get/rm round-trip, byte-for-byte match), baseline-snapshotted. See §0 for live details.

## 0. LIVE — as built

| Field | Value |
|---|---|
| LXC | **121 `rift-sftp-test`** on `blazzer-labs`, unprivileged, Debian 12, 1c/512MB/8GB, `onboot=1` |
| IP | **192.168.1.16** (DHCP — may drift; re-resolve: `ssh blazzer-labs 'pct exec 121 -- hostname -I'`) |
| SSH user | `rift`, **key-auth only** — key `/c/AI Workflow/.secrets/rift-sftp-test` (ed25519, no passphrase) |
| Seed tree | `/home/rift/fxserver/{server.cfg, resources/a_seed/init.lua, resources/b_seed/fxmanifest.lua}` |
| sshd | hardened (`/etc/ssh/sshd_config.d/rift-ci.conf`): `MaxSessions 50`, `MaxStartups 30:50:100`, `PasswordAuthentication no`, keepalives — survives rollback |
| Baseline | snapshot `baseline` (pristine + hardened) — reset via helper `reset` or `pct rollback 121 baseline` |
| Helper | **`scripts/sftp-test-target.sh {health\|ip\|status\|reset\|env\|ssh\|tree}`** — resolves DHCP IP live (no hardcode drift) |
| Env (`.secrets/env.sh`) | `RIFT_TEST_SFTP_HOST/PORT/USER/KEY` exported (or `eval $(… env)` for live IP) |
| Verified (2026-05-30) | SSH exec ✓ · SFTP put/rename/get/rm ✓ · **atomic rename-overwrite** ✓ (transfer.rs path) · mkdir/chmod/rmdir ✓ · **25 MiB round-trip integrity** ✓ · 10 parallel conns ✓ · **reset→pristine cycle** ✓ |
| Created via | **host root SSH** (`blazzer-labs` alias, `pct create`) — MCP stayed read-only, no standing elevation |
| Teardown | `ssh blazzer-labs 'pct stop 121 && pct destroy 121'` (fully reversible) |

**Not yet done (next session):** write the actual `#[ignore]` integration tests in Rust
(§3) that read the env vars and exercise `transfer.rs` / `flush_batch` / `drift_scanner`
against this target. The infra is ready; the test code is the remaining work.

## What's actually on the box (queried live 2026-05-30)

Node **`blazzer-labs`** — single node, freshly rebooted (power blip), idle:
- RAM **13.54 GB total, ~2.4 GB used (~11 GB free)**; load 0.16.
- Storage: `local` (dir, 94 GB — templates/ISO/backups, 10% used) + `local-lvm`
  (lvmthin, **1.67 TB, 0.9% used** — rootfs/images go here, effectively unlimited room).
- Templates ready: **Debian 12** (`debian-12-standard_12.12-1`) + Debian 13.
- Next free VMID: **100** (also 101, 102, 121+ free; 103 + 120 taken).

Already running (both LXC):
- **103 `web-search-mcp`** = the `blazzer-search` MCP at `http://192.168.1.172:8080/mcp`.
  Confirms the stateless-service-on-Proxmox pattern is already in place.
- **120 `fxserver`** = the REAL FXServer Trey connects to (Tailscale `100.122.178.19`,
  user `treyday`). **Do NOT test against this** — live target, tests write/rename/delete.

Bridge is `vmbr0` on `192.168.1.0/24` (blazzer-search sits at `.172`), so a new
container gets a `192.168.1.x` LAN address.

## Why this is the right Proxmox play for Rift

Rift's core risk (ISSUES #21/#265): `flush_batch`, `transfer.rs` atomic upload/rename,
and the drift reconciler move real files over SFTP and are **largely untested** — Wave B
mocks them with `SftpOps`/`MockSftp` precisely because there's no live server to hit.
A dedicated, isolated, resettable sshd container unlocks a new test tier the mocks can't
cover faithfully: **real rename semantics, real SFTP error codes, real perms/ownership,
real concurrent-write behavior.**

## 1. Container spec

| Field | Value | Rationale |
|---|---|---|
| VMID | **121** | adjacent to real fxserver 120 ("real + test" pair); 100 also fine |
| Hostname | `rift-sftp-test` | |
| Template | Debian 12 standard | stable, matches likely fxserver base |
| Cores / RAM | 1 vCPU / 512 MB | sshd + a dir tree is tiny |
| Disk | 8 GB on `local-lvm` | room for multi-hundred-MB transfer tests (#89) |
| Net | DHCP on `vmbr0` → `192.168.1.x` | LAN-direct = low latency, no Tailscale/VPN needed |
| Features | `nesting=0`, unprivileged | minimal attack surface, LAN-only |

## 2. Provisioning inside the container

```
apt update && apt install -y openssh-server rsync
# dedicated test user, key auth only (mirrors russh key-based flow — no passwords)
useradd -m -s /bin/bash rift
mkdir -p /home/rift/.ssh && chmod 700 /home/rift/.ssh
# <paste rift-sftp-test.pub into authorized_keys>
chmod 600 /home/rift/.ssh/authorized_keys && chown -R rift:rift /home/rift/.ssh
# FXServer-shaped seed tree so drift_scanner sees realistic structure
mkdir -p /home/rift/fxserver/resources/{a_seed,b_seed}
# (seed a few files + a nested dir; tests create/modify/delete from here)
systemctl enable --now ssh
```

Keypair: generate `rift-sftp-test` (ed25519) on the dev machine, **private key →
`/c/AI Workflow/.secrets/`**, public key → container `authorized_keys`. Never commit either.

## 3. How Rift tests consume it

- New tier: **live integration tests behind `#[ignore]`** (run explicitly, never in the
  default `cargo test` so other machines / no-box CI stay green).
- Env-gated connection — tests `return` early if unset:
  `RIFT_TEST_SFTP_HOST=192.168.1.<x>`, `RIFT_TEST_SFTP_USER=rift`,
  `RIFT_TEST_SFTP_KEY=<path in .secrets>`.
- First targets (highest untested risk):
  1. `transfer.rs` atomic upload → tmp → rename round-trip + SHA verify.
  2. `flush_batch` end-to-end dispatch against real sshd (closes #21.1's blocker —
     pairs with the `SftpOps` trait already landed in `64b79ef`).
  3. `drift_scanner` 3-way diff vs a seeded baseline (Conflict / suspicious-shrink /
     jitter-collapse cases that need real mtimes).
  4. #89 large-file streaming path (>16 MB) once implemented.

## 4. Reset between runs

Proxmox **snapshot** of the pristine container post-provision → `pct rollback 121 baseline`
before a test session. Cleaner than a wipe script; instant, deterministic.

## 5. How it was built (resolved)

The `proxmox` MCP is read-only by design (`PROXMOX_ALLOW_ELEVATED=false`), so the create
path was **host root SSH** instead — the `blazzer-labs` alias (`192.168.1.150`, key
`~/.ssh/id_ed25519`) already grants `pct` access. One-time `pct create`/`pct exec`/
`pct snapshot` over SSH; the MCP was never elevated. Least standing privilege: the container
is now driven purely over SSH by the (future) test suite — no ongoing Proxmox elevation.

**Hardening follow-ups (optional):** promote the DHCP lease to a static IP / router
reservation so `RIFT_TEST_SFTP_HOST` never drifts; currently DHCP + re-resolve helper.

## Phase 2 (optional, later) — offload runner

11 GB free RAM could also host a Debian LXC running `cargo check` + `svelte-check` +
`cargo test` for **cross-platform** Rust/Svelte, so verification never collides with
`tauri dev`'s file lock (the hazard in CLAUDE.md). Caveat: Windows-specific Tauri/WebView2/
NSIS code won't compile on Linux — release builds stay local. Lower priority than the SFTP
target; revisit if the test suite grows.
