//! R1 (per `docs/design/assistant-mod-split.md`) — Claude CLI discovery:
//! locate every install on the machine, rank them, cache the active binary,
//! and build the windowless spawn `Command`. Lifted verbatim from
//! `assistant/mod.rs` 2026-06-09; only visibility changed (`pub(super)` on the
//! four symbols the parent module calls).

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

/// Cached absolute path to the user's `claude` CLI. Windows' `Command::new`
/// does NOT apply PATHEXT lookup (no auto-append of `.cmd`/`.exe`), so we
/// resolve via `where.exe claude` (or `which` on Unix) and reuse the
/// absolute path for all spawn sites. Outer `Option` = is-cached;
/// inner = path-or-not.
///
/// #64: previously `OnceLock<Option<PathBuf>>` — cached forever per process.
/// An upgrade or reinstall of the CLI required a full Rift restart. The
/// fast path now stats the cached file; a missing-file triggers a fresh
/// re-resolution, so CLI installs/moves take effect on the next spawn.
pub(super) static CLAUDE_EXE: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

/// One detected Claude Code CLI installation. A single machine can carry
/// several at once — the classic case is an npm-global install AND Anthropic's
/// native installer side by side, which silently drift to different versions.
/// Rift enumerates every one, runs turns on the newest, and updates all of
/// them so the versions can't diverge (the dual-install "stuck out of date"
/// bug). All fields camelCase for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeInstall {
    /// Absolute path to the runnable binary (never a `.cmd`/`.bat` shim).
    pub path: String,
    /// "npm" | "native" | "unknown" — drives the correct update command.
    pub method: String,
    /// `claude --version` output for THIS binary, or None if it failed to run.
    pub version: Option<String>,
    /// Resolvable via `where`/`which` — i.e. what a plain shell would launch.
    pub on_path: bool,
    /// The install Rift currently spawns (newest version wins).
    pub active: bool,
}

/// Lowercase + backslash-normalize a path string for case-insensitive compares
/// (Windows paths are case-insensitive; `where.exe` and our probes can differ
/// in case/separator for the same file).
fn norm_path(s: &str) -> String {
    s.to_ascii_lowercase().replace('/', "\\")
}

/// Run `where claude` (Windows) / `which -a claude` (unix) and return every
/// non-blank line. Empty on failure (no CLI on PATH).
fn where_claude_lines() -> Vec<String> {
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("where.exe", &["claude"])
    } else {
        ("which", &["-a", "claude"])
    };
    let mut cmd = std::process::Command::new(program);
    cmd.args(args).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// The user's real npm global prefix (`npm config get prefix`), so we probe the
/// ACTUAL node_modules drop-site, not just the default `%APPDATA%\npm`. A user
/// who ran `npm config set prefix D:\tools` installs claude elsewhere. None when
/// npm isn't runnable; the caller falls back to the default site.
#[cfg(windows)]
fn npm_global_prefix() -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let out = std::process::Command::new("npm.cmd")
        .args(["config", "get", "prefix"])
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() || s == "undefined" {
        return None;
    }
    Some(PathBuf::from(s))
}

/// Classify how a `claude` binary at `p` was installed, from its path.
/// npm-global installs must update via `npm install -g …@latest`; native
/// installs self-update and accept `claude update`.
fn classify_install_method(p: &Path) -> &'static str {
    let s = p.to_string_lossy().to_ascii_lowercase();
    if s.contains("\\npm\\node_modules\\")
        || s.contains("/npm/node_modules/")
        || s.ends_with(".cmd")
        || s.ends_with(".bat")
    {
        "npm"
    } else if s.contains("anthropicclaude")
        || s.contains("\\.local\\bin\\")
        || s.contains("/.local/bin/")
    {
        "native"
    } else {
        "unknown"
    }
}

/// Run `<exe> --version` and return its trimmed output (None if it can't run).
/// Strips a stray `ANTHROPIC_API_KEY` like every other spawn; hides the window.
/// Bounded at 5s — a hung binary here used to block the auth probe (and with it
/// the first-run gate) forever.
fn probe_version_at(exe: &Path) -> Option<String> {
    use std::io::Read;
    use std::time::{Duration, Instant};
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    cmd.env_remove("ANTHROPIC_API_KEY");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().ok()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                log::warn!("claude --version timed out after 5s: {}", exe.display());
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    let mut s = String::new();
    child.stdout.take()?.read_to_string(&mut s).ok()?;
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Pull a `major.minor.patch` triple out of a version string, tolerating a
/// leading `v` and trailing noise like `"2.1.111 (Claude Code)"`.
pub(super) fn parse_semver(v: &str) -> Option<(u64, u64, u64)> {
    let cleaned: String = v
        .chars()
        .map(|c| if c.is_ascii_digit() || c == '.' { c } else { ' ' })
        .collect();
    for tok in cleaned.split_whitespace() {
        let parts: Vec<&str> = tok.split('.').collect();
        if parts.len() >= 3 {
            if let (Ok(a), Ok(b), Ok(c)) = (parts[0].parse(), parts[1].parse(), parts[2].parse()) {
                return Some((a, b, c));
            }
        }
    }
    None
}

/// Enumerate EVERY Claude CLI install on this machine — PATH hits plus the
/// known native + npm drop sites — de-duplicated, each probed for its version
/// and classified by install method. Only real `.exe` binaries are kept on
/// Windows (a `.cmd`/`.bat` shim mangles the newline-bearing stream-json spawn
/// args, CVE-2024-24576 mitigation); the npm shim's bundled `.exe` is probed
/// directly instead. Synchronous (spawns several short `--version` children) —
/// call from a blocking task on the async paths.
pub(super) fn enumerate_claude_installs() -> Vec<ClaudeInstall> {
    let where_lines = where_claude_lines();
    let where_norm: Vec<String> = where_lines.iter().map(|l| norm_path(l)).collect();

    let mut paths: Vec<PathBuf> = Vec::new();
    let add = |p: PathBuf, list: &mut Vec<PathBuf>| {
        if p.is_file() {
            let n = norm_path(&p.to_string_lossy());
            if !list.iter().any(|q| norm_path(&q.to_string_lossy()) == n) {
                list.push(p);
            }
        }
    };

    if cfg!(windows) {
        // Real `.exe` entries directly on PATH.
        for l in &where_lines {
            if l.to_ascii_lowercase().ends_with(".exe") {
                add(PathBuf::from(l), &mut paths);
            }
        }
        // Native installer drop sites (not always wired into PATH).
        if let Some(lad) = std::env::var_os("LOCALAPPDATA") {
            add(PathBuf::from(&lad).join("AnthropicClaude").join("claude.exe"), &mut paths);
            add(
                PathBuf::from(&lad).join("Programs").join("AnthropicClaude").join("claude.exe"),
                &mut paths,
            );
        }
        // npm-global bundled exe (PATH only carries the `.cmd` shim). Probe the
        // user's REAL npm prefix first (custom-prefix installs), then the default
        // %APPDATA%\npm site — both, so a non-default prefix is covered without
        // regressing the default-install detection.
        let npm_roots = {
            let mut v: Vec<PathBuf> = Vec::new();
            if let Some(prefix) = npm_global_prefix() {
                v.push(prefix);
            }
            if let Some(appdata) = std::env::var_os("APPDATA") {
                v.push(PathBuf::from(&appdata).join("npm"));
            }
            v
        };
        for root in npm_roots {
            add(
                root.join("node_modules")
                    .join("@anthropic-ai")
                    .join("claude-code")
                    .join("bin")
                    .join("claude.exe"),
                &mut paths,
            );
        }
        // ~/.local/bin native-script install.
        if let Some(home) = std::env::var_os("USERPROFILE") {
            add(PathBuf::from(&home).join(".local").join("bin").join("claude.exe"), &mut paths);
        }
        // Last-resort: `.cmd`/`.bat` shims on PATH (custom npm prefix with no
        // bundled `.exe` at the default site). Ranked below any real binary and
        // collapsed into its bundled `.exe` (same version) by the dedup below.
        for l in &where_lines {
            let low = l.to_ascii_lowercase();
            if low.ends_with(".cmd") || low.ends_with(".bat") {
                add(PathBuf::from(l), &mut paths);
            }
        }
    } else {
        for l in &where_lines {
            add(PathBuf::from(l), &mut paths);
        }
        if let Some(home) = std::env::var_os("HOME") {
            add(PathBuf::from(&home).join(".local").join("bin").join("claude"), &mut paths);
        }
    }

    let raw: Vec<ClaudeInstall> = paths
        .into_iter()
        .map(|p| {
            let pstr = p.to_string_lossy().to_string();
            let pn = norm_path(&pstr);
            let method = classify_install_method(&p);
            // on_path: the exe itself is a `where` hit, OR (npm) its prefix's
            // `.cmd` shim is — the bundled exe lives one dir deeper than PATH.
            let on_path = where_norm.contains(&pn)
                || (method == "npm"
                    && where_norm.iter().any(|w| w.contains("\\npm\\") || w.contains("/npm/")));
            ClaudeInstall {
                version: probe_version_at(&p),
                method: method.to_string(),
                on_path,
                active: false,
                path: pstr,
            }
        })
        .collect();

    // Collapse entries that are really the SAME install reached two ways — the
    // npm `.cmd` shim and the bundled `.exe` it forwards to share method +
    // version. Keep the real binary, fold in the shim's on_path flag.
    let mut deduped: Vec<ClaudeInstall> = Vec::new();
    for inst in raw {
        if let Some(dup) = deduped
            .iter_mut()
            .find(|e| {
                e.method == inst.method
                    && e.version == inst.version
                    && inst.version.is_some()
                    // Only fold a shim into its real exe (or vice-versa) — never
                    // collapse two distinct real installs that share a version.
                    && (is_shim(&e.path) || is_shim(&inst.path))
            })
        {
            dup.on_path = dup.on_path || inst.on_path;
            if is_shim(&dup.path) && !is_shim(&inst.path) {
                dup.path = inst.path.clone();
            }
            continue;
        }
        deduped.push(inst);
    }
    deduped
}

fn method_rank(m: &str) -> u8 {
    match m {
        "native" => 2,
        "npm" => 1,
        _ => 0,
    }
}

/// A `.cmd`/`.bat` forwarder, not a real binary. These mangle the newline-
/// bearing stream-json spawn args (CVE-2024-24576), so they're only ever an
/// active pick of last resort.
fn is_shim(path: &str) -> bool {
    let s = path.to_ascii_lowercase();
    s.ends_with(".cmd") || s.ends_with(".bat")
}

/// True if `a` is the better "active" pick than `b`. Priority order:
///   1. a real binary beats a `.cmd`/`.bat` shim (shims mangle stream-json args);
///   2. an on-PATH copy beats an off-PATH one — this is the install the user's
///      shell uses and ran `claude login` against, so its OAuth/subscription
///      session is the one that authenticates;
///   3. newest version wins among equally-reachable installs;
///   4. method rank breaks final ties.
///
/// on_path outranks version deliberately (#auth): a newer copy sitting off-PATH
/// (e.g. a native install under LOCALAPPDATA the user never logged into) would
/// otherwise be spawned and 401 even while the terminal `claude` works fine —
/// the exact "works in my terminal, not in Rift" trap a fresh collaborator hit.
fn install_is_better(a: &ClaudeInstall, b: &ClaudeInstall) -> bool {
    let (a_shim, b_shim) = (is_shim(&a.path), is_shim(&b.path));
    if a_shim != b_shim {
        return !a_shim;
    }
    if a.on_path != b.on_path {
        return a.on_path;
    }
    match (
        a.version.as_deref().and_then(parse_semver),
        b.version.as_deref().and_then(parse_semver),
    ) {
        (Some(va), Some(vb)) if va != vb => return va > vb,
        (Some(_), None) => return true,
        (None, Some(_)) => return false,
        _ => {}
    }
    method_rank(&a.method) > method_rank(&b.method)
}

/// Index of the install Rift should spawn — newest usable one. None if empty.
pub(super) fn select_active_index(installs: &[ClaudeInstall]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, inst) in installs.iter().enumerate() {
        match best {
            None => best = Some(i),
            Some(b) if install_is_better(inst, &installs[b]) => best = Some(i),
            _ => {}
        }
    }
    best
}

fn resolve_claude_exe_uncached() -> Option<PathBuf> {
    let installs = enumerate_claude_installs();
    select_active_index(&installs).map(|i| PathBuf::from(&installs[i].path))
}

pub(super) fn resolve_claude_exe() -> Option<PathBuf> {
    // Fast path: return cached value if the file still exists. is_file()
    // catches the "CLI uninstalled or moved" case without forcing a full
    // re-resolution every call.
    {
        let g = match CLAUDE_EXE.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(Some(p)) = g.as_ref() {
            // Only a still-present cached path is authoritative. A cached
            // `Some(None)` ("no CLI on PATH") is NOT sticky — re-resolving each
            // spawn lets a CLI installed AFTER the first failed resolve become
            // visible without a restart (the `#64` re-stat only revives a path
            // that vanished, never an empty cache → green pill but "not found"
            // sends). Cost is one cheap re-enumeration per spawn while no CLI
            // exists; it self-heals the moment one appears.
            if p.is_file() {
                return Some(p.clone());
            }
        }
    }
    // Slow path: re-resolve, then cache. Re-check under the write lock so a
    // concurrent slow-path that already cached a usable value wins (avoids a
    // conflicting double-write when two callers race the empty/stale cache).
    let resolved = resolve_claude_exe_uncached();
    let mut g = match CLAUDE_EXE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(Some(p)) = g.as_ref() {
        if p.is_file() {
            return Some(p.clone());
        }
    }
    *g = Some(resolved.clone());
    resolved
}

/// Build a `tokio::process::Command` for `claude`, hiding the console window
/// on Windows. Returns `None` if the CLI isn't on PATH. `pub(crate)` so the
/// `stt::cleanup` hop can reuse the same resolution + windowing path.
pub(crate) fn claude_command() -> Option<Command> {
    let exe = resolve_claude_exe()?;
    let mut cmd = Command::new(exe);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    // #63 follow-up: kill the CLI child if the spawning tokio task is
    // dropped before `wait()` returns (panic mid-turn, IPC handle teardown,
    // app shutdown). Without this the child outlives the spawn task and the
    // PID-tracker-based `assistant_stop` is the only kill path — which itself
    // depends on `set_session_pid` having completed.
    cmd.kill_on_drop(true);
    // Single source of truth for auth identity: strip any inherited system
    // `ANTHROPIC_API_KEY` from EVERY claude spawn. A stray env key would
    // otherwise silently authenticate the CLI under a different identity than
    // Rift's keychain/login model implies — green auth pill, then a 401 (the
    // trap that cost a collaborator hours). The only sanctioned API-key path
    // re-adds it explicitly on the configured-key send branch (`assistant_send`).
    cmd.env_remove("ANTHROPIC_API_KEY");
    Some(cmd)
}

/// Cached `(exe_path, parsed_version)` for the active install. Keyed on the exe
/// path so a CLI upgrade/move (which `resolve_claude_exe` already re-resolves)
/// also re-probes the version instead of serving a stale triple. Inner version
/// is `Option` because `--version` can fail to run (treated as conservative-old
/// upstream — see `cli_caps`).
static CLAUDE_VERSION: Mutex<Option<(PathBuf, Option<(u64, u64, u64)>)>> = Mutex::new(None);

/// RR10: drop the cached version triple. An in-place CLI update leaves the
/// binary at the SAME path, so the path-keyed cache would return the PRE-update
/// version (gating new flags off) until restart. Callers clear it alongside
/// `CLAUDE_EXE` after an update.
pub(super) fn clear_version_cache() {
    let mut g = CLAUDE_VERSION.lock().unwrap_or_else(|p| p.into_inner());
    *g = None;
}

/// Parsed `major.minor.patch` of the install Rift currently spawns, or `None`
/// when no CLI is resolvable OR its `--version` can't be read/parsed. Cached
/// behind the active-exe path so the (5s-bounded) probe runs at most once per
/// install, re-running only after an upgrade/move. Callers MUST treat `None` as
/// "assume old / gate off bleeding-edge flags" (`CliCaps::from_version`).
pub(super) fn active_cli_version() -> Option<(u64, u64, u64)> {
    let exe = resolve_claude_exe()?;
    {
        let g = match CLAUDE_VERSION.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some((cached_exe, ver)) = g.as_ref() {
            if *cached_exe == exe {
                return *ver;
            }
        }
    }
    let ver = probe_version_at(&exe).as_deref().and_then(parse_semver);
    let mut g = match CLAUDE_VERSION.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *g = Some((exe, ver));
    ver
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_parses_and_tolerates_prefixes() {
        assert_eq!(parse_semver("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_semver("v0.7.0"), Some((0, 7, 0)));
        // Embedded in a longer string (e.g. `--version` banner output).
        assert_eq!(parse_semver("claude 1.10.0 (Claude Code)"), Some((1, 10, 0)));
        assert_eq!(parse_semver("not a version"), None);
        assert_eq!(parse_semver("1.2"), None, "needs all three components");
    }
}
