# Self-hosted Windows runner on Proxmox — plan

> Goal: stop renting GitHub `windows-latest` minutes (2× billed, ~30-40 min/release, hit the spending wall on 2026-06-09). Run releases on `blazzer-labs` (Proxmox) for free + faster. Researched 2026-06-09 (cont.87).

## Host reality (`blazzer-labs`)
Single node · ~13.5 GB RAM total (~11.5 GB free; `web-search-mcp` LXC uses 2 GB) · **12 cores** · 1.65 TB free on `local-lvm`. **RAM is the binding constraint.** CPU/disk abundant (12 cores → fast builds vs windows-latest's 2-4).

## Verified access (cont.87) — I can drive this nearly solo
- **SSH to host works:** `ssh blazzer-labs` (192.168.1.150, key auth, BatchMode) → **Proxmox VE 9.1.1**, `qm` CLI present. So the whole hypervisor side is scriptable from the host shell (not just the read-only MCP).
- This **closes the Windows-install gap**: download WS2022 Core + virtio-win ISOs to `/var/lib/vz/template/iso/`, `qm create` the VM, attach an **`autounattend.xml` ISO** (unattended install + VirtIO driver paths) as a 2nd CD, and a `FirstLogonCommands`/`SetupComplete.cmd` that installs guest-agent + Rust/Node/.NET/VS-BuildTools + registers the runner service. Fully headless, driven from host SSH + `qm`.
- **Autonomy split (revised):** I can do VM create, ISO fetch, autounattend authoring, in-guest toolchain + runner registration, and the repo workflow edits. **Human-only bits:** (a) GitHub billing must be cleared first; (b) likely babysit 1-2 autounattend iterations (XML is finicky); (c) a GitHub runner registration token (I mint via `gh api`). Do in a FRESH session — it's long + multi-step.

## Decisions (researched)
1. **Guest OS = Windows Server 2022 Core, 8 GB RAM.** Core is headless (no Desktop Experience), idles ~1 GB. Rust release **link** peaks ~3-4 GB; + MSVC tools/Node/.NET baseline → ~5.5-6.5 GB under load. **7 GB floor, 8 GB safe.** 8 GB leaves ~3.5 GB for host + LXC — tight but viable. (GitHub's own windows-latest = WS2022 @ 7 GB.) NOT Tiny11 (strips serviceability; CI-risky), NOT full Win11 (heavier).
2. **Persistent runner, NOT ephemeral.** Warm `target/` cuts cold 30-40 min → ~8-12 min incremental. Ephemeral needs an orchestrator (JIT tokens + per-job VM spin) — no turnkey Proxmox path, not worth it for a low-frequency private single-repo build. Private repo = no fork-PR attack surface → persistent token risk is low.
3. **Cross-compile from Linux = HARD NO.** WebView2 is fine (runtime bootstrapper, not build-time). The blocker is **Velopack `vpk`** — `vpk pack` needs Windows PE tooling + .NET, no Linux support. A real Windows VM is required. (cargo-xwin only works for a Velopack-free NSIS build.)
4. **Caching:** persistent `target/` on a dedicated disk is the primary lever; layer **`sccache`** (`RUSTC_WRAPPER`, local-disk mode) for reuse across `cargo clean`. Leave `~/.cargo/registry`+`git` on the VM disk.
5. Ruled out: ARC (k8s — too heavy for one node); Windows-in-LXC (impossible, Windows needs full KVM).

## Stacks with our earlier finding
The release build is **LINK-bound** (measured cont.87: opt-3 28.6s vs opt-1 30-32s root rebuild — opt-level is a no-op). Warm `target/` helps the compile half; the link still runs every release. So **also pursue `rust-lld`** (`.cargo/config.toml` `[target.x86_64-pc-windows-msvc] linker="rust-lld"`) — it attacks the link directly and stacks on top of the warm-cache win. Measure on the VM.

## Setup checklist
1. KVM VM: WS2022 Core ISO, 4 vCPU, 8 GB RAM, 120 GB OS disk + 200 GB secondary (build artifacts). VirtIO drivers.
2. Install in guest: Git, Node LTS, Rust stable (MSVC), VS Build Tools 2022 (C++ workload + Win11 SDK), .NET 8 SDK, `dotnet tool install -g vpk --version 1.2.0` (MUST match the `velopack` crate pin).
3. Runner: extract `actions/runner` release to `C:\actions-runner` → `.\config.cmd --url https://github.com/Blazzer10200/rift-tauri --token <TOKEN> --name proxmox-win --labels self-hosted,windows,tauri-build --runasservice`.
4. SCM auto-restart: `sc failure actions-runner.rift-tauri actions= restart/5000/restart/10000/restart/30000 reset= 86400`.
5. System env: `CARGO_TARGET_DIR=D:\cargo-target`, `RUSTC_WRAPPER=sccache`, `SCCACHE_DIR=D:\sccache`.
6. `release.yml`: `runs-on: windows-latest` → `runs-on: [self-hosted, windows, tauri-build]`. (rust-cache step is then redundant-but-harmless; or drop it since the disk is warm.) Same for the `check` workflow if you want it self-hosted too.
7. Harden: repo → Settings → Actions → disable fork-PR runs; runner as non-admin local account, write only to workspace; outbound-only via Proxmox NAT. Snapshot the VM (vzdump) after toolchain install as a restore point.

## Caveats
- 8 GB is RAM-tight on a 13.5 GB host. If link OOMs, the fix is a cheap stick upgrade (16→32 GB) → VM to 12-16 GB, comfortable + faster.
- Keep `vpk` CLI version == `velopack` crate version (both `=1.2.0`) — drift packs an incompatible updater.
- `windows-latest` stays the zero-infra fallback (and unblocks once GitHub billing is cleared).

Sources: GitHub self-hosted/secure-use/add-runners docs · Tauri 2 Windows-installer docs · MS WS2022 hardware reqs · sccache docs.
