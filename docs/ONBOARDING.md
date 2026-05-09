# Onboarding — first-run

For someone who's been handed a `Setup.exe` and wants to start syncing.

## 1. Install

- Run `Rift-Setup.exe`. Per-user install, no admin required. Lands at `%LOCALAPPDATA%\Rift\rift-tauri.exe` w/ a Start-menu shortcut.
- If SmartScreen flags it: **More info → Run anyway** (we don't have a code-signing cert yet — H4 on the audit list).

## 2. Generate / pick an SSH key

Rift needs an OpenSSH ed25519 keypair to talk to your FXServer.

- Open Rift → **Settings → SSH key**. If `~/.ssh/id_ed25519` exists, Rift uses it. Otherwise click **Generate** — writes to `~/.ssh/id_ed25519` + `.pub`.
- Send the public key (`~/.ssh/id_ed25519.pub`) to whoever runs the FXServer. They append it to `~/.ssh/authorized_keys`.

## 3. Add a server

- **Sidebar → ＋ Add server**. Fill in:
  - **Name** — display label (e.g. `Homelab FX`)
  - **Host / Port / User** — SSH endpoint
  - **Identity file** — pre-filled w/ the default key
  - **Remote root** — path to your `txData/<base>/resources` (or whatever folder you want mirrored)
  - **Local root** — local mirror destination (Rift creates it if missing)
- **Save**. First connect prompts for a fingerprint (TOFU) — confirm and pin.

## 4. First sync

- **Drift** tab → **Scan**. Shows what's different btw local + remote.
- **Pull all** if local is empty. **Push all** if remote needs catching up. Otherwise resolve per-file.
- **AutoSync** tab → **Start** once you're happy with the baseline. Local edits now stream up.

## 5. Updates

Rift checks for updates on launch. When a new build ships, the dialog auto-pops — **Install & restart** swaps the binary in place.

## Trouble?

- `~/.rift/` is the config + log dir. `rift.json` = profiles, `rift-autosync.log` = sync activity.
- If sync looks stuck: **Stop → Start** in the AutoSync tab.
- File a bug or ping Blazzer with the log tail.
