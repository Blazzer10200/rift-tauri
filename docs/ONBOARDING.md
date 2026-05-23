# Onboarding — getting Rift running from a fresh install

This walkthrough takes you from a downloaded `Setup.exe` to a first
successful sync in about five minutes. If you hit a snag, the
[Troubleshooting](#troubleshooting) section at the bottom covers the most
common failure modes.

---

## 1. Install

Download the latest `Rift-Setup-vX.Y.Z-alpha.exe` from
[Blazzer10200/rift-releases](https://github.com/Blazzer10200/rift-releases/releases/latest),
then run it. The installer is per-user (no admin prompt needed) and drops
Rift into `%LocalAppData%\Programs\Rift\`.

### Windows SmartScreen warning

> **"Windows protected your PC — Don't run"**
>
> Rift's installer is currently unsigned (issue #15 — code-signing
> deferred until budget is in place). SmartScreen flags any unsigned
> installer until it earns reputation through downloads.
>
> Click **"More info"** → **"Run anyway"** to proceed. You can verify the
> SHA256 of the downloaded file against the value published in the
> release notes if you want to be paranoid.

---

## 2. SSH key setup

Rift uses SSH key authentication for SFTP — no passwords are stored.
Open Rift, go to **Settings → Network → SSH keys**, and choose either
"Generate new" or "Import existing".

### Generate a new key

Rift runs `ssh-keygen -t ed25519 -f ~/.ssh/id_rift_ed25519`. You'll get a
fresh key pair scoped to Rift's use. The private half stays on your
machine; you need to copy the public half (`.pub` file) into your game
server's `~/.ssh/authorized_keys`.

Rift's UI shows the public key inline with a one-click "Copy" button. On
your server:

```bash
mkdir -p ~/.ssh && chmod 700 ~/.ssh
echo 'ssh-ed25519 AAAA…YOUR-KEY-HERE… rift' >> ~/.ssh/authorized_keys
chmod 600 ~/.ssh/authorized_keys
```

### Import an existing key

If you already use an SSH key (`~/.ssh/id_rsa`, `~/.ssh/id_ed25519`,
etc.), pick the **private** key file from your filesystem. Rift just
records the path; it never copies the key.

### Permissions matter

On Windows, OpenSSH refuses to use a key file that "too many people can
read." If Rift's SFTP connection fails with `Permissions … are too open`,
run:

```powershell
icacls $env:USERPROFILE\.ssh\id_rift_ed25519 /inheritance:r
icacls $env:USERPROFILE\.ssh\id_rift_ed25519 /grant:r "$env:USERNAME:(R)"
```

---

## 3. Pick a local workspace folder

This is where Rift mirrors remote resources. Pick a folder on a fast
local SSD, NOT one synced via OneDrive / Dropbox / iCloud — cloud-sync
tools collide with Rift's atomic-rename pattern and produce phantom
drift.

Examples that work:

- `C:\Projects\my-fivem-server`
- `D:\dev\redm`

Avoid:

- `C:\Users\<you>\OneDrive\fivem` (cloud sync)
- `\\nas\fivem` (UNC paths — auto-watcher unstable over SMB)
- Drive roots like `C:\` (Rift refuses by design)

---

## 4. Add your first server

In **Settings → Network → Servers**, click **+ Add server** and fill in:

| Field | Example | Notes |
|---|---|---|
| Name | `Production · my-fivem` | Display name only |
| Host | `game.example.com` | DNS or IP |
| Port | `22` | SSH port (NOT the FXServer port) |
| User | `root` or `cfx` | Username that owns the SSH key |
| SSH key path | `C:\Users\you\.ssh\id_rift_ed25519` | Picked or generated in step 2 |
| Remote root | `/opt/cfx-server` | FXServer's base directory |
| Local root | `C:\Projects\my-fivem-server` | From step 3 |

The first connection probes the server's fingerprint. Accept once — Rift
remembers it (TOFU pattern) and refuses to connect later if it changes
(MITM defense). To change a fingerprint deliberately (server rebuild,
key rotation), edit the server profile and clear the stored fingerprint.

---

## 5. Connect Claude (optional)

Rift ships an embedded Claude assistant that reads your code, edits
files, runs shell commands, and so on. It uses your existing
`claude` CLI authentication — Rift never stores API keys.

**If you don't already have the CLI:** install from
[docs.claude.com/en/docs/claude-code](https://docs.claude.com/en/docs/claude-code).
Then run `claude` once in any terminal to log in. After login, click
**Re-check** in Rift's auth panel.

You can skip this step entirely; Rift's sync features work without
Claude. Toggle it on later from **Settings → Assistant**.

---

## 6. First sync

Click **Sync** in the activity bar (left rail). The first scan can take
30 – 90 seconds on a busy FXServer — it walks both sides and computes
SHA1s for files under 64 MiB. You'll see five buckets after the scan:

- **Synced** — both sides match.
- **To push** — local newer than remote.
- **To pull** — remote newer than local.
- **To delete (local)** — exists locally but not remotely + has snapshot.
- **To delete (remote)** — Mirror mode only; opt-in destructive op.
- **Conflict** — both sides changed since the last baseline.

**Nothing moves until you click Apply.** The summary always shows
counts first so you can sanity-check. Once you apply, the autosync
watcher takes over — file edits inside the local root flush to the
remote every ~700ms (debounced).

The mass-delete circuit breaker (≥ 30% of files or ≥ 25 deletes) blocks
any batch that smells like an accidental `rm -rf` and stops the watcher
until you re-enable it.

---

## Troubleshooting

### "Can't connect to SFTP" / `connection refused`

- Confirm the host responds: `ssh -p 22 user@host` from a terminal.
- Check the port — SFTP uses the SSH port (default 22), not the FXServer
  port (default 30120).
- Firewall: confirm port 22 is open inbound on the server's host.

### `Permission denied (publickey)` on connect

- The public half of your key isn't in the server's `authorized_keys`,
  or the server's SSH config has `PubkeyAuthentication no`.
- File permission issue on the key (see step 2 above).

### `Host key verification failed` / fingerprint mismatch

- Server's host key changed (rebuild? key rotation?). If you trust the
  change, edit the server profile in **Settings → Network**, clear the
  stored fingerprint, and reconnect.
- If you didn't change anything on the server, **don't accept** — it
  could be a man-in-the-middle.

### "Claude CLI not found"

- `claude --version` works from a terminal but Rift can't find it: Rift
  spawns the CLI without your shell's `PATH`. Add the install dir to the
  system PATH (Windows: System Properties → Environment Variables), then
  restart Rift.
- You haven't installed the CLI: see step 5.

### Autosync not flushing edits

- Edits inside the local root land in the autosync watcher's dirty queue
  with a ~700ms debounce. Watch the bottom-right status pill for the
  flush cycle.
- Edits OUTSIDE the local root are ignored by design.
- Cloud-sync tools (OneDrive / Dropbox) can rename files mid-flush.
  Move your workspace to a non-synced folder.

### "BLOCKED — N deletes in one batch" in the activity feed

- The mass-delete circuit breaker tripped. This is intentional — Rift
  refuses to delete more than 30% of a folder in one batch (cap: 25
  files). The watcher stops; nothing was deleted remotely.
- Inspect the local folder — did you mean to delete that many files? If
  yes, restart the watcher from **Settings → Network**. If no, the
  remote is intact and you can restore the local copy from your VCS or
  by pulling from remote.

### Where do logs live?

- **App logs:** `%LocalAppData%\Rift\logs\rift.log` (rotated by size).
- **Config:** `%USERPROFILE%\.rift\rift.json` (server profiles).
- **Cache (snapshots):** `%LocalAppData%\Rift\cache\snapshot-<key>.json`
  per server. Delete to force a rebaseline.

---

## Where to next

- **Drift drawer details:** [docs/HANDOFF.md](HANDOFF.md) covers the
  current state of the sync pipeline.
- **Issues + roadmap:** [docs/ISSUES.md](ISSUES.md).
- **Assistant deep dives:** the assistant page has `/diag` for telemetry,
  `/compact` for manual compaction, and `/help` inside Claude itself.
- **Updating Rift:** auto-update via Velopack. Settings → About → Check
  for updates. The app downloads in the background and applies on next
  launch.
