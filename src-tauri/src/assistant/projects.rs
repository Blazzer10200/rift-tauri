//! User-defined projects — a named alias over a workspace folder plus
//! per-project file-pattern config (include/exclude globs) that scopes the
//! MCP server's `read_file`/`list_dir`/`grep` walk.
//!
//! A `Project` is intentionally lightweight: it evolves the existing
//! `recent_roots` MRU into a first-class, named, configurable entity without
//! touching the global `current_root` resolution path. Conversations still
//! stamp a raw `workspaceRoot`; projects associate by canonical-path match, so
//! the project system is purely additive — a folder opened the old way keeps
//! working, and removing every project leaves the app behaving exactly as
//! before.
//!
//! Storage: `AssistantConfig.projects` (a new `#[serde(default)]` field, so old
//! on-disk configs parse unchanged). The renderer generates the `id` via
//! `crypto.randomUUID()` (matching conversation ids); the backend canonicalizes
//! the root + validates patterns.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{load_config, save_config, AssistantConfig, CONFIG_WRITE_LOCK};

/// Max projects a user can define. Generous — the sidebar list scrolls, but a
/// runaway count would bloat config.json and the per-turn lookup.
pub(super) const PROJECTS_MAX: usize = 64;
/// Per-pattern length cap (a single include/exclude glob). Mirrors the grep
/// glob cap in `mcp_server.rs` so a project pattern can't exceed what the tool
/// would accept anyway.
const PATTERN_MAX_LEN: usize = 512;
/// Max patterns per list. A project with hundreds of globs is misconfigured and
/// would slow every walk; cap to keep the per-file match loop bounded.
const PATTERNS_MAX: usize = 64;
/// Project name length cap — a label, not prose.
const NAME_MAX_LEN: usize = 120;

/// A user-defined project. `root` is canonicalized on write so it lines up with
/// the canonical roots conversations and the MCP server compare against.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    /// Glob include allowlist. EMPTY = include everything (minus exclude +
    /// SKIP_DIRS). Non-empty = only paths matching at least one glob are
    /// visible to the workspace tools.
    #[serde(default)]
    pub include: Vec<String>,
    /// Glob exclude list, applied on top of the always-on `SKIP_DIRS` baseline.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Creation timestamp (epoch ms), set by the renderer. Used only for stable
    /// display ordering; never load-bearing.
    #[serde(default)]
    pub created_at: u64,
}

/// Renderer-facing view — identical to `Project` but with the root rendered as
/// a forward-slash-tolerant string (PathBuf serializes fine, but an explicit
/// DTO keeps the wire shape stable if `Project` grows internal-only fields).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub root: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub created_at: u64,
}

impl From<&Project> for ProjectDto {
    fn from(p: &Project) -> Self {
        ProjectDto {
            id: p.id.clone(),
            name: p.name.clone(),
            root: p.root.to_string_lossy().into_owned(),
            include: p.include.clone(),
            exclude: p.exclude.clone(),
            created_at: p.created_at,
        }
    }
}

/// Reject ids that aren't the hex/dash uuid shape the renderer generates — same
/// guard convo_store applies, so a project id can never escape into a path.
fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Validate + normalize a pattern list: trim, drop blanks, cap count + length,
/// and reject embedded newlines (the env channel is newline-separated, so an
/// embedded `\n` would split one glob into two phantom patterns).
fn sanitize_patterns(patterns: Vec<String>) -> Result<Vec<String>, String> {
    let mut out = Vec::with_capacity(patterns.len().min(PATTERNS_MAX));
    for p in patterns {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        if p.len() > PATTERN_MAX_LEN {
            return Err(format!("pattern too long (max {PATTERN_MAX_LEN} bytes): {p}"));
        }
        if p.contains('\n') || p.contains('\r') {
            return Err("pattern may not contain newlines".into());
        }
        // Compile against the SAME glob→regex the matcher uses → reject at save
        // time, not silently at walk time.
        if let Err(e) = super::mcp_server::glob_to_regex(p) {
            return Err(format!("invalid glob \"{p}\": {e}"));
        }
        out.push(p.to_string());
        if out.len() >= PATTERNS_MAX {
            break;
        }
    }
    Ok(out)
}

fn projects_dto(cfg: &AssistantConfig) -> Vec<ProjectDto> {
    cfg.projects.iter().map(ProjectDto::from).collect()
}

/// The include/exclude patterns for the project whose canonical root matches
/// `root`, if any. Used by `turn.rs` to thread per-project patterns into the
/// MCP child's env. Returns `(include, exclude)`; empty vecs when no project
/// owns this root (→ the MCP child applies only its SKIP_DIRS baseline).
pub(super) fn patterns_for_root(cfg: &AssistantConfig, root: &std::path::Path) -> (Vec<String>, Vec<String>) {
    // Canonical-on-write means a plain equality check suffices, but a moved/
    // deleted root won't canonicalize at lookup time — compare the stored
    // (already-canonical) value directly against the resolved turn root.
    let target = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    for p in &cfg.projects {
        if p.root == target {
            return (p.include.clone(), p.exclude.clone());
        }
    }
    (Vec::new(), Vec::new())
}

#[tauri::command]
pub fn assistant_list_projects() -> Result<Vec<ProjectDto>, String> {
    Ok(projects_dto(&load_config()))
}

/// Create or update a project. The renderer supplies a `crypto.randomUUID()`
/// id; an existing id updates in place, a new id appends. Canonicalizes the
/// root (must be an existing directory) and sanitizes both pattern lists.
#[tauri::command]
pub fn assistant_save_project(
    id: String,
    name: String,
    root: String,
    include: Vec<String>,
    exclude: Vec<String>,
    created_at: Option<u64>,
) -> Result<Vec<ProjectDto>, String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    if !is_valid_id(&id) {
        return Err(format!("invalid project id: {id}"));
    }
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("project name is required".into());
    }
    if name.len() > NAME_MAX_LEN {
        return Err(format!("project name too long (max {NAME_MAX_LEN})"));
    }
    let raw = PathBuf::from(root.trim());
    if !raw.is_dir() {
        return Err(format!("not a directory: {}", raw.display()));
    }
    let canonical = std::fs::canonicalize(&raw).unwrap_or(raw);
    let include = sanitize_patterns(include)?;
    let exclude = sanitize_patterns(exclude)?;

    let mut cfg = load_config();
    if let Some(existing) = cfg.projects.iter_mut().find(|p| p.id == id) {
        existing.name = name;
        existing.root = canonical;
        existing.include = include;
        existing.exclude = exclude;
    } else {
        if cfg.projects.len() >= PROJECTS_MAX {
            return Err(format!("project limit reached (max {PROJECTS_MAX})"));
        }
        cfg.projects.push(Project {
            id,
            name,
            root: canonical,
            include,
            exclude,
            created_at: created_at.unwrap_or(0),
        });
    }
    save_config(&cfg)?;
    Ok(projects_dto(&cfg))
}

#[tauri::command]
pub fn assistant_delete_project(id: String) -> Result<Vec<ProjectDto>, String> {
    let _cfg_guard = CONFIG_WRITE_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    if !is_valid_id(&id) {
        return Err(format!("invalid project id: {id}"));
    }
    let mut cfg = load_config();
    let before = cfg.projects.len();
    cfg.projects.retain(|p| p.id != id);
    if cfg.projects.len() != before {
        save_config(&cfg)?;
    }
    Ok(projects_dto(&cfg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_guard_accepts_uuid_shape_rejects_paths() {
        assert!(is_valid_id("2a3c95b0-1234-4abc-9def-0123456789ab"));
        assert!(is_valid_id("abc_123-XYZ"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("../etc/passwd"));
        assert!(!is_valid_id("a/b"));
        assert!(!is_valid_id(&"x".repeat(65)));
    }

    #[test]
    fn sanitize_trims_drops_blanks_and_caps() {
        let out = sanitize_patterns(vec![
            "  src/**  ".into(),
            "".into(),
            "   ".into(),
            "*.rs".into(),
        ])
        .unwrap();
        assert_eq!(out, vec!["src/**".to_string(), "*.rs".to_string()]);
    }

    #[test]
    fn sanitize_rejects_newlines_and_overlong() {
        assert!(sanitize_patterns(vec!["a\nb".into()]).is_err());
        assert!(sanitize_patterns(vec!["x".repeat(PATTERN_MAX_LEN + 1)]).is_err());
    }

    #[test]
    fn patterns_for_root_returns_empty_when_no_project() {
        let cfg = AssistantConfig::default();
        let (inc, exc) = patterns_for_root(&cfg, std::path::Path::new("/no/such/project"));
        assert!(inc.is_empty() && exc.is_empty());
    }
}
