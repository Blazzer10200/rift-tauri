// Full WPF AutoSync.ShouldIgnore parity port. Replaces drift_scanner's
// `should_ignore_basic` w/ the complete rule set:
//   - editor lock/swap files (~$, .~lock., .#)
//   - exact filenames (4913, .DS_Store, Thumbs.db, desktop.ini)
//   - extensions (.swp, .tmp, .bak, .backup, .pyc, .rift-tmp, .rift-lock)
//   - .backup.<digits>, .tmp.<digits> editor patterns
//   - path-segment ignores (.git, node_modules, obj, bin, dist, build, __pycache__,
//     .cache, cache, [disabled], .vscode, .idea, .svelte-kit, .next, .nuxt,
//     .turbo, .parcel-cache, .pytest_cache, .ruff_cache, .mypy_cache, coverage,
//     target, .venv, venv)
//   - FiveM `web/build/` + `web/dist/` bypass for ui_page bundles
//
// Returns the matched rule label (stable string) so callers can log + bucket.

const IGNORE_EXTS: &[&str] = &[
    ".swp", ".tmp", ".bak", ".backup", ".pyc", ".rift-tmp", ".rift-lock",
];

const IGNORE_FILE_EXACT: &[&str] = &[
    "4913", ".DS_Store", "Thumbs.db", "desktop.ini",
    // Rift bookkeeping — written via direct SFTP (EditTrail), MUST NOT pass
    // through the watch/push/pull pipeline or we get a self-loop:
    // pull trail → notify → push → EditTrail rewrites remote trail → drift
    // sees newer remote → pull again → forever.
    ".rift-trail.jsonl",
];

/// Path-segment ignores. Each entry is the bare name; the matcher synthesizes
/// `/<name>/` (and `\<name>\` on Windows).
const IGNORE_SEGMENTS: &[&str] = &[
    ".git",
    "node_modules",
    "obj",
    "bin",
    "dist",
    "build",
    "__pycache__",
    ".cache",
    "cache",
    "[disabled]",
    "_disabled_archive",
    ".vscode",
    ".idea",
    ".svelte-kit",
    ".next",
    ".nuxt",
    ".turbo",
    ".parcel-cache",
    ".pytest_cache",
    ".ruff_cache",
    ".mypy_cache",
    "coverage",
    "target",
    ".venv",
    "venv",
    // Rift's own FiveM bridge resource — server-side daemon rewrites
    // `snapshot.json` + `devbridge.jsonl` every few seconds as runtime state.
    // Pre-ignore the entire folder: same self-loop pathology as
    // `.rift-trail.jsonl` (pull → watcher event → push → server rewrites →
    // drift → pull forever). User pulled+synced these every 10s in v0.2.35.
    // If the bridge resource itself ever needs an update, do it manually
    // outside the watch pipeline. See v0.2.36 post-mortem.
    "endure_devbridge",
];

/// Suffix-matched folder names — catches the bridge under any prefix
/// (`<server>_devbridge`) without needing per-server entries. Matched against
/// each path segment; `endure_devbridge` and `qbx_devbridge` both hit.
const IGNORE_SEGMENT_SUFFIXES: &[&str] = &[
    "_devbridge",
];

/// Prefix-matched folder names — catches user "park" dirs created out-of-band
/// (`_disabled_extras`, `_disabled_bracket_dupes_<date>`, etc). The exact-
/// match `_disabled_archive` above stays as a stable label; the prefix here
/// covers free-form park-dir names users invent during cleanup sessions.
/// Endure RP 2026-05-12 used `_disabled_extras/bracket_dupes_20260512/` to
/// park bracket dupes inside the resources tree; the live SFTP scan kept
/// surfacing those as ToPull until the prefix rule landed.
const IGNORE_SEGMENT_PREFIXES: &[&str] = &[
    "_disabled_",
];

/// Returns the ignore-rule label that matched, or None if eligible for sync.
/// Stable label so the UI summary buckets by it.
pub fn classify(path: &str) -> Option<&'static str> {
    if path.is_empty() {
        return Some("empty-path");
    }
    let normalized: std::borrow::Cow<str> = if path.contains('\\') {
        std::borrow::Cow::Owned(path.replace('\\', "/"))
    } else {
        std::borrow::Cow::Borrowed(path)
    };
    let trimmed = normalized.trim_end_matches('/');
    let name = trimmed.rsplit('/').next().unwrap_or("");
    if name.is_empty() {
        return Some("empty-name");
    }

    // Editor lock/swap prefixes
    if name.starts_with("~$") {
        return Some("editor-lock(~$)");
    }
    if name.starts_with(".~lock.") {
        return Some("editor-lock(.~lock.)");
    }
    if name.starts_with(".#") {
        return Some("editor-lock(.#)");
    }

    // Exact filenames
    for exact in IGNORE_FILE_EXACT {
        if name.eq_ignore_ascii_case(exact) {
            return Some(match *exact {
                "4913" => "file:4913",
                ".DS_Store" => "file:.DS_Store",
                "Thumbs.db" => "file:Thumbs.db",
                "desktop.ini" => "file:desktop.ini",
                ".rift-trail.jsonl" => "file:.rift-trail.jsonl",
                _ => "file:?",
            });
        }
    }

    // Extension suffix
    for ext in IGNORE_EXTS {
        if name.to_ascii_lowercase().ends_with(ext) {
            return Some(match *ext {
                ".swp" => "ext:.swp",
                ".tmp" => "ext:.tmp",
                ".bak" => "ext:.bak",
                ".backup" => "ext:.backup",
                ".pyc" => "ext:.pyc",
                ".rift-tmp" => "ext:.rift-tmp",
                ".rift-lock" => "ext:.rift-lock",
                _ => "ext:?",
            });
        }
    }

    // <base>.backup.<tail>
    if let Some(idx) = name.to_ascii_lowercase().find(".backup.") {
        if idx > 0 && idx + 8 < name.len() {
            return Some("editor-backup");
        }
    }

    // Rift conflict-rename marker — `<file>.rift-conflict.<user>-<ts>.<ext>`
    // produced by the DriftWatcher when a remote pull would clobber a local
    // file that's still dirty. These files are user-facing artifacts (the
    // ConflictResolver UI surfaces them); they MUST NOT be re-uploaded as
    // their own thing or we'd loop conflict-renames across machines.
    if name.to_ascii_lowercase().contains(".rift-conflict.") {
        return Some("rift-conflict-marker");
    }

    // <base>.tmp.<digits-or-dot> — Svelte/VSCode atomic save pattern
    if let Some(idx) = name.to_ascii_lowercase().find(".tmp.") {
        if idx > 0 && idx + 5 < name.len() {
            let tail = &name[idx + 5..];
            if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return Some("editor-tmp(.tmp.<digits>)");
            }
        }
    }

    // FiveM resource UI bundle: `<...>/web/build/` or `<...>/web/dist/` —
    // bypass the generic /build/ + /dist/ rules only.
    let lower = normalized.to_ascii_lowercase();
    let is_fivem_ui_output =
        lower.contains("/web/build/") || lower.contains("/web/dist/");

    for seg in IGNORE_SEGMENTS {
        // IGNORE_SEGMENTS entries are kept lowercase by convention so we can
        // skip the per-iteration `.to_ascii_lowercase()` allocation on the
        // synthesized needle. Validated by the lowercase-segments unit test.
        let needle = format!("/{seg}/");
        if !lower.contains(&needle) {
            // Also match leading-no-slash form for relative paths starting with the seg.
            let leading = format!("{seg}/");
            if !lower.starts_with(&leading) {
                continue;
            }
        }
        if is_fivem_ui_output && (*seg == "build" || *seg == "dist") {
            continue;
        }
        return Some(match *seg {
            ".git" => "seg:.git",
            "node_modules" => "seg:node_modules",
            "obj" => "seg:obj",
            "bin" => "seg:bin",
            "dist" => "seg:dist",
            "build" => "seg:build",
            "__pycache__" => "seg:__pycache__",
            ".cache" => "seg:.cache",
            "cache" => "seg:cache",
            "[disabled]" => "seg:[disabled]",
            "_disabled_archive" => "seg:_disabled_archive",
            ".vscode" => "seg:.vscode",
            ".idea" => "seg:.idea",
            ".svelte-kit" => "seg:.svelte-kit",
            ".next" => "seg:.next",
            ".nuxt" => "seg:.nuxt",
            ".turbo" => "seg:.turbo",
            ".parcel-cache" => "seg:.parcel-cache",
            ".pytest_cache" => "seg:.pytest_cache",
            ".ruff_cache" => "seg:.ruff_cache",
            ".mypy_cache" => "seg:.mypy_cache",
            "coverage" => "seg:coverage",
            "target" => "seg:target",
            ".venv" => "seg:.venv",
            "venv" => "seg:venv",
            "endure_devbridge" => "seg:endure_devbridge",
            _ => "seg:?",
        });
    }

    // Suffix-matched segments (e.g. `qbx_devbridge`, `world_devbridge` — any
    // bridge variant) — applied to ALL segments incl. filenames since
    // `_devbridge` is always a dir name.
    // Prefix-matched (`_disabled_extras`, `_disabled_dupes`) — applied ONLY
    // to non-terminal segments (directories), so a legitimate file named
    // `_disabled_for_review.lua` is not falsely ignored.
    let segs: Vec<&str> = lower.split('/').filter(|s| !s.is_empty()).collect();
    let last_idx = segs.len().saturating_sub(1);
    for (i, seg) in segs.iter().enumerate() {
        for suffix in IGNORE_SEGMENT_SUFFIXES {
            if seg.ends_with(suffix) && seg.len() > suffix.len() {
                return Some(match *suffix {
                    "_devbridge" => "seg-suffix:_devbridge",
                    _ => "seg-suffix:?",
                });
            }
        }
        // Prefix matching skips the terminal segment — that's typically a
        // file, and we don't want `_disabled_*` prefix on filenames to
        // false-trip. Directory segments at any depth do trip.
        if i < last_idx {
            for prefix in IGNORE_SEGMENT_PREFIXES {
                if seg.starts_with(prefix) && seg.len() > prefix.len() {
                    return Some(match *prefix {
                        "_disabled_" => "seg-prefix:_disabled_",
                        _ => "seg-prefix:?",
                    });
                }
            }
        }
    }

    None
}

pub fn should_ignore(path: &str) -> bool {
    classify(path).is_some()
}

/// Names that are pure dir-names (no brackets) — exposed for remote `find -name`
/// pruning callers. Mirrors WPF `IgnoredDirectoryNames()`.
pub fn ignored_directory_names() -> Vec<&'static str> {
    IGNORE_SEGMENTS
        .iter()
        .copied()
        .filter(|s| !s.starts_with('['))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_lock_variants() {
        assert_eq!(classify("foo/~$file.docx"), Some("editor-lock(~$)"));
        assert_eq!(classify("foo/.~lock.bar.ods#"), Some("editor-lock(.~lock.)"));
        assert_eq!(classify("foo/.#bar"), Some("editor-lock(.#)"));
    }

    #[test]
    fn extensions() {
        assert!(should_ignore("a/b/c.swp"));
        assert!(should_ignore("a/b/c.tmp"));
        assert!(should_ignore("a/b/c.rift-lock"));
        assert!(!should_ignore("a/b/c.lua"));
    }

    #[test]
    fn editor_tmp_digits() {
        assert_eq!(
            classify("foo/main.lua.tmp.12345"),
            Some("editor-tmp(.tmp.<digits>)")
        );
        // Legit user file w/ non-digit tail — must NOT trip
        assert_eq!(classify("foo/report.tmp.draft.md"), None);
    }

    #[test]
    fn dot_backup_pattern() {
        assert_eq!(classify("foo/main.backup.123abc"), Some("editor-backup"));
    }

    #[test]
    fn segments() {
        assert_eq!(classify("a/.git/HEAD"), Some("seg:.git"));
        assert_eq!(classify("a/node_modules/x/index.js"), Some("seg:node_modules"));
        assert_eq!(classify("a/[disabled]/qbx_core/main.lua"), Some("seg:[disabled]"));
        assert_eq!(classify("a/target/debug/foo"), Some("seg:target"));
    }

    #[test]
    fn disabled_prefix_segments() {
        // Park dirs created via `mv` during cleanup sessions.
        assert_eq!(
            classify("resources/_disabled_extras/bracket_dupes_20260512/[ox]/ox_lib/init.lua"),
            Some("seg-prefix:_disabled_"),
        );
        assert_eq!(
            classify("resources/_disabled_dupes/foo.lua"),
            Some("seg-prefix:_disabled_"),
        );
        // Existing exact `_disabled_archive` still matches via the segment list.
        assert_eq!(
            classify("resources/_disabled_archive/qbx_core/main.lua"),
            Some("seg:_disabled_archive"),
        );
        // Doesn't trip on legitimate-name files w/ unrelated prefix.
        assert_eq!(classify("resources/qbx_core/_disabled_for_review.lua"), None);
    }

    #[test]
    fn fivem_web_build_bypassed() {
        // generic /build/ → ignored
        assert_eq!(classify("a/proj/build/out.js"), Some("seg:build"));
        // FiveM /web/build/ → NOT ignored
        assert_eq!(classify("res/qbx_core/web/build/index.html"), None);
        assert_eq!(classify("res/qbx_core/web/dist/style.css"), None);
    }

    #[test]
    fn empty_paths_rejected() {
        assert_eq!(classify(""), Some("empty-path"));
    }

    #[test]
    fn legit_files_pass() {
        assert!(!should_ignore("server/main.lua"));
        assert!(!should_ignore("[qbx]/qbx_core/client/main.lua"));
        assert!(!should_ignore("ui/index.html"));
    }

    #[test]
    fn ignored_dir_names_excludes_brackets() {
        let names = ignored_directory_names();
        assert!(names.contains(&"node_modules"));
        assert!(names.contains(&"target"));
        assert!(!names.iter().any(|n| n.starts_with('[')));
    }

    /// Guards the `pre-lower needle` optimization in `classify` — if anyone
    /// adds an uppercase-bearing segment, the lowercase-input check would
    /// silently miss it.
    #[test]
    fn segments_are_lowercase() {
        for s in IGNORE_SEGMENTS {
            assert_eq!(*s, s.to_ascii_lowercase(), "segment {s:?} must be lowercase");
        }
    }
}
