//! Custom slash-command discovery for the composer's `/` menu — scans the same
//! places the Claude CLI resolves skills + commands from:
//!   project — `<root>/.claude/skills/*/SKILL.md` + `<root>/.claude/commands/**/*.md`
//!   user    — `~/.claude/skills/*/SKILL.md`     + `~/.claude/commands/**/*.md`
//!   plugin  — each install dir in `~/.claude/plugins/installed_plugins.json`
//!             (`<install>/skills/*/SKILL.md` + `<install>/commands/**/*.md`;
//!             the registry, NOT `plugins/marketplaces/` — that catalog mirrors
//!             every AVAILABLE plugin, installed or not)
//! Metadata only (name / frontmatter description / argument-hint): Rift never
//! executes these — an unmatched `/name` rides to the CLI as the prompt, where
//! the CLI's own skill resolution runs it. Collisions dedup project-over-user-
//! over-plugin (CLI precedence), skills-over-commands within a source.

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomCommand {
    pub name: String,
    pub description: String,
    /// "project" | "user"
    pub source: String,
    /// "skill" | "command"
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub argument_hint: Option<String>,
}

/// Per-source entry cap — a runaway skills dir can't flood the menu payload.
const MAX_PER_SOURCE: usize = 200;
/// Only the head of each md file is read — frontmatter lives at the top.
const HEAD_BYTES: usize = 16 * 1024;
const DESC_MAX: usize = 240;

#[tauri::command]
pub async fn assistant_list_custom_commands(
    root: Option<String>,
) -> Result<Vec<CustomCommand>, String> {
    tokio::task::spawn_blocking(move || {
        let mut out = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        // Project first so a name collision resolves project-over-user, matching
        // CLI precedence. Root is trusted here the same way the sibling
        // workspace commands trust it — it comes from the tab's picker state.
        if let Some(r) = root.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let dot_claude = Path::new(r).join(".claude");
            scan_source(&dot_claude, "project", &mut seen, &mut out);
        }
        if let Ok(home) = crate::state::paths::dirs_home() {
            scan_source(&home.join(".claude"), "user", &mut seen, &mut out);
            // Installed plugins last so a same-name user/project entry wins.
            // A plugin install dir lays out skills/ + commands/ directly under
            // itself — the same shape scan_source expects under a `.claude`.
            let registry = home.join(".claude").join("plugins").join("installed_plugins.json");
            if let Ok(txt) = std::fs::read_to_string(&registry) {
                for dir in plugin_install_paths(&txt) {
                    scan_source(&dir, "plugin", &mut seen, &mut out);
                }
            }
        }
        Ok(out)
    })
    .await
    .map_err(|e| format!("skills scan: {e}"))?
}

/// One `.claude` dir → entries appended to `out` (alphabetical, skills first so
/// a skill's richer frontmatter wins a same-name command). `seen` spans sources.
fn scan_source(
    dot_claude: &Path,
    source: &str,
    seen: &mut HashSet<String>,
    out: &mut Vec<CustomCommand>,
) {
    let mut batch = Vec::new();
    scan_skills(&dot_claude.join("skills"), source, &mut batch);
    scan_commands(&dot_claude.join("commands"), source, &mut batch);
    batch.sort_by(|a, b| a.name.cmp(&b.name));
    for c in batch {
        if seen.insert(c.name.clone()) {
            out.push(c);
        }
    }
}

/// `skills/<name>/SKILL.md` — one level deep, dir name = command name unless
/// the frontmatter overrides it with `name:`.
fn scan_skills(dir: &Path, source: &str, out: &mut Vec<CustomCommand>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if out.len() >= MAX_PER_SOURCE {
            return;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        if dir_name.starts_with('.') {
            continue;
        }
        let md = path.join("SKILL.md");
        let Some(head) = read_head(&md) else { continue };
        let fm = parse_frontmatter(&head);
        let name = fm.name.as_deref().unwrap_or(dir_name);
        let Some(name) = valid_name(name) else { continue };
        out.push(CustomCommand {
            name,
            description: fm
                .description
                .unwrap_or_else(|| body_first_line(&head).unwrap_or_default()),
            source: source.into(),
            kind: "skill".into(),
            argument_hint: fm.argument_hint,
        });
    }
}

/// `commands/**/*.md` — filename (minus `.md`) = command name; subdirectories
/// namespace with `:` the way the CLI surfaces them (`frontend/build.md` →
/// `/frontend:build`).
fn scan_commands(dir: &Path, source: &str, out: &mut Vec<CustomCommand>) {
    if !dir.is_dir() {
        return;
    }
    for entry in walkdir::WalkDir::new(dir)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .flatten()
    {
        if out.len() >= MAX_PER_SOURCE {
            return;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(dir) else { continue };
        let mut parts: Vec<String> = rel
            .components()
            .filter_map(|c| c.as_os_str().to_str().map(String::from))
            .collect();
        let Some(last) = parts.last_mut() else { continue };
        *last = last.trim_end_matches(".md").to_string();
        let Some(name) = valid_name(&parts.join(":")) else { continue };
        let Some(head) = read_head(path) else { continue };
        let fm = parse_frontmatter(&head);
        out.push(CustomCommand {
            name,
            description: fm
                .description
                .unwrap_or_else(|| body_first_line(&head).unwrap_or_default()),
            source: source.into(),
            kind: "command".into(),
            argument_hint: fm.argument_hint,
        });
    }
}

/// Install dirs out of `installed_plugins.json` (version-2 shape:
/// `{"plugins": {"name@marketplace": [{"installPath": …}, …]}}`). Forgiving —
/// any parse miss yields an empty list, never an error (no registry file =
/// no plugins installed, the common case).
fn plugin_install_paths(json: &str) -> Vec<std::path::PathBuf> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return Vec::new() };
    let Some(map) = v.get("plugins").and_then(|p| p.as_object()) else { return Vec::new() };
    let mut out: Vec<std::path::PathBuf> = map
        .values()
        .filter_map(|entries| entries.as_array())
        .flatten()
        .filter_map(|e| e.get("installPath").and_then(|s| s.as_str()))
        .filter(|p| !p.trim().is_empty())
        .map(std::path::PathBuf::from)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Slash-safe name: alnum head, then word chars / `:` / `-` / `.`. Anything
/// else (spaces, unicode punctuation) can't be typed as `/name` — drop it.
fn valid_name(raw: &str) -> Option<String> {
    let name = raw.trim();
    let mut chars = name.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphanumeric() {
        return None;
    }
    if name.len() > 64
        || !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ':' | '.'))
    {
        return None;
    }
    Some(name.to_string())
}

fn read_head(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; HEAD_BYTES];
    let mut filled = 0usize;
    // Loop: File::read may return short counts before EOF.
    loop {
        match f.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => {
                filled += n;
                if filled == buf.len() {
                    break;
                }
            }
            Err(_) => return None,
        }
    }
    buf.truncate(filled);
    Some(String::from_utf8_lossy(&buf).into_owned())
}

struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    argument_hint: Option<String>,
}

/// Forgiving YAML-subset frontmatter reader: top-level `key: value` pairs,
/// quoted scalars, and `|`/`>` block or plain indented continuations folded to
/// one space-joined line. Anything fancier degrades to "no value", never errors.
fn parse_frontmatter(text: &str) -> Frontmatter {
    #[derive(Clone, Copy)]
    enum Slot {
        Name,
        Desc,
        Hint,
    }
    fn slot_mut(fm: &mut Frontmatter, s: Slot) -> &mut Option<String> {
        match s {
            Slot::Name => &mut fm.name,
            Slot::Desc => &mut fm.description,
            Slot::Hint => &mut fm.argument_hint,
        }
    }
    let mut fm = Frontmatter { name: None, description: None, argument_hint: None };
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return fm;
    }
    let mut current: Option<Slot> = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "..." {
            break;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if !indented {
            let Some((key, value)) = line.split_once(':') else {
                current = None;
                continue;
            };
            let slot = match key.trim() {
                "name" => Some(Slot::Name),
                "description" => Some(Slot::Desc),
                "argument-hint" | "argument_hint" => Some(Slot::Hint),
                _ => None,
            };
            let value = value.trim();
            let scalar = if value.is_empty() || value.starts_with('|') || value.starts_with('>') {
                String::new() // block scalar / bare key — continuations fill it in
            } else {
                unquote(value)
            };
            if let Some(s) = slot {
                *slot_mut(&mut fm, s) = Some(scalar);
            }
            current = slot;
        } else if let Some(s) = current {
            if let Some(acc) = slot_mut(&mut fm, s).as_mut() {
                if !trimmed.is_empty() {
                    if !acc.is_empty() {
                        acc.push(' ');
                    }
                    acc.push_str(trimmed);
                }
            }
        }
    }
    for v in [&mut fm.name, &mut fm.description, &mut fm.argument_hint] {
        if let Some(s) = v {
            let clipped = clip(s, DESC_MAX);
            if clipped.is_empty() {
                *v = None;
            } else {
                *v = Some(clipped);
            }
        }
    }
    fm
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2
        && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')))
    {
        v[1..v.len() - 1].to_string()
    } else {
        v.to_string()
    }
}

/// First non-empty body line after any frontmatter block — the description
/// fallback for bare command files. Markdown header hashes stripped.
fn body_first_line(text: &str) -> Option<String> {
    let body = if let Some(rest) = text.strip_prefix("---") {
        match rest.find("\n---") {
            Some(i) => &rest[i + 4..],
            None => rest,
        }
    } else {
        text
    };
    body.lines()
        .map(|l| l.trim().trim_start_matches('#').trim())
        .find(|l| !l.is_empty())
        .map(|l| clip(l, DESC_MAX))
}

fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end].trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_basic() {
        let fm = parse_frontmatter("---\nname: deploy\ndescription: Ship it\nargument-hint: <env>\n---\nbody");
        assert_eq!(fm.name.as_deref(), Some("deploy"));
        assert_eq!(fm.description.as_deref(), Some("Ship it"));
        assert_eq!(fm.argument_hint.as_deref(), Some("<env>"));
    }

    #[test]
    fn frontmatter_quoted_and_multiline() {
        let fm = parse_frontmatter(
            "---\ndescription: >\n  Runs the full\n  release chain.\nname: \"ship\"\n---\n",
        );
        assert_eq!(fm.name.as_deref(), Some("ship"));
        assert_eq!(fm.description.as_deref(), Some("Runs the full release chain."));
    }

    #[test]
    fn frontmatter_absent() {
        let fm = parse_frontmatter("# Just a command\nDo the thing.");
        assert!(fm.description.is_none());
        assert_eq!(
            body_first_line("# Just a command\nDo the thing.").as_deref(),
            Some("Just a command"),
        );
    }

    #[test]
    fn body_first_line_skips_frontmatter() {
        assert_eq!(
            body_first_line("---\nfoo: bar\n---\n\n## Run tests\n").as_deref(),
            Some("Run tests"),
        );
    }

    #[test]
    fn name_validation() {
        assert_eq!(valid_name("git-ship"), Some("git-ship".into()));
        assert_eq!(valid_name("frontend:build"), Some("frontend:build".into()));
        assert!(valid_name("-bad").is_none());
        assert!(valid_name("has space").is_none());
        assert!(valid_name("").is_none());
    }

    #[test]
    fn plugin_install_paths_parses_v2_registry() {
        let json = r#"{
          "version": 2,
          "plugins": {
            "rust-analyzer-lsp@official": [
              {"scope": "user", "installPath": "C:\\u\\.claude\\plugins\\cache\\official\\rust-analyzer-lsp\\1.0.0"}
            ],
            "dupe@official": [
              {"installPath": "C:\\u\\.claude\\plugins\\cache\\official\\rust-analyzer-lsp\\1.0.0"},
              {"installPath": ""}
            ]
          }
        }"#;
        let paths = plugin_install_paths(json);
        assert_eq!(paths.len(), 1); // deduped; empty path dropped
        assert!(paths[0].to_string_lossy().ends_with("1.0.0"));
    }

    #[test]
    fn plugin_install_paths_tolerates_garbage() {
        assert!(plugin_install_paths("not json").is_empty());
        assert!(plugin_install_paths("{}").is_empty());
        assert!(plugin_install_paths(r#"{"plugins": 3}"#).is_empty());
    }

    #[test]
    fn clip_respects_char_boundaries() {
        let s = "é".repeat(300);
        let c = clip(&s, DESC_MAX);
        assert!(c.len() <= DESC_MAX + "…".len());
        assert!(c.ends_with('…'));
    }
}
