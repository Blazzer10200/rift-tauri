# rift-tauri — Handoff

> Live = current session + RESUME HERE + CRITICAL DON'T-TOUCH. Older history via `git log -- docs/HANDOFF.md`. Cap ≤600 words.

## Session 2026-06-02 (cont. 5) — redesign verified COMPLETE + UNSHIPPED; docs/tree cleaned for new session

### 🟢 RESUME HERE — the ONLY remaining work is P6: SHIP the redesign
The "UI off big time / missing UI" was **never a code gap** — the redesign is done on `main` but was never built, so the user's *installed* app still shows the old UI:
- Installed `%LOCALAPPDATA%\rift\rift-tauri.exe` = **v0.4.46, built 5/31** (activity-dock release, PRE-redesign). Dev server (current `main`) has the full redesign.
- Whole P1–P6 graphite-ink arc landed on `main` 6/02, **unbumped, no changelog entry, never built.**
- Re-verified live via CDP this session: ALL 5 surfaces (home/chat/sync/files/settings) + token layer (`app.css :root` == `DESIGN_TOKENS.md`: accent `oklch(0.78 0.15 163)`=#34D399, radii 4/6/8/10/12, motion vars, fs scale) + `FbDetail.svelte` faithfully match the mockup. "Design reference suspect" hypothesis DISPROVEN — implementation == spec.

**Next action → `/git-ship`** (user-initiated): bump 3 files + Cargo.lock → write redesign CHANGELOG entry → check → build (self-replace dance, dev must quit) → install. Puts the new UI on the user's machine.

### Cleanup done this session
- CHANGELOG trimmed 9558w → current-version-only (v0.4.46) per convention; history in `git log -- docs/CHANGELOG.md`.
- `redesign-gap-audit.md` → `docs/archive/` (P1–P6 plan executed + verified); CLAUDE.md design-brief pointer updated.
- ⌘K Titlebar search committed (was done+verified but uncommitted). Tree clean: no scratch/orphan/untracked files; legacy `settings/Settings.svelte` already gone.

### Deferred (blocked, none ship-blocking)
Lua code-preview (needs read-only backend file-read cmd — user declined 2nd backend exception; offer again) · RCON live round-trip (needs live FXServer) · Files populated LED pips live pixels (code present; needs varied file states).

### Env + lessons
- Dev app RUNNING (connected Endure RP, CDP wrapper :9223 / :9222) — re-attach via `bash scripts/cdp/c.sh`, no relaunch unless dead. v0.4.46 unbumped.
- **Don't run `npm run check` while `tauri dev` is alive** — `svelte-kit sync` reloads the webview → drops `connection.servers` (backend SSH stays; full restart re-restores). Same family as cargo-check-during-dev.
- Clean dev restart: TaskStop npm task → `taskkill` orphaned `rift-tauri.exe` by EXACT PID (never `rift*` glob, never `rift.exe`) → kill orphaned vite :1420 → relaunch.

## CRITICAL DON'T-TOUCH (frontend/redesign)
- **Onboarding gate (#7):** `showOnboarding` keeps all 4 conditions (`!dismissed && serversLoaded && servers.length===0 && defaultSshKeyExists===false`). Loosening flashes onboarding for existing users.
- **Accent themeable via `--accent-h`** (app.css `:root` only): `oklch(L C var(--accent-h))`; never hard-code an accent hue. Components use `var(--accent)`/`--accent-soft` (already retheme) — do NOT add `--accent-h` to them. Status LEDs (`--ok/warn/danger/info`) stay fixed.
- **IA:** kbds home·1 chat·2 sync·3 files·4 settings·5; Activity is a Sync tab (`syncPage.tab`). Home wiring real-data only.
- **AssistantPane drop handlers on `.pane` outer only**; `tauri.conf.json dragDropEnabled:false`; `.shell` `position:fixed; inset:0`.
- **RCON:** pw plaintext over UDP (how FXServer RCON works) — keychain only, never disk/IPC. Targets `bridge_port`, not SSH port.
- **Versions lockstep** across `package.json`+`Cargo.toml`+`tauri.conf.json` (+`Cargo.lock`) — only at `/git-ship`.
