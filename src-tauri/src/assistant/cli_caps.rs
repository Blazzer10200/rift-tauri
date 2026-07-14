//! CLI capability gating — make Rift's spawn args adapt to the *installed*
//! Claude Code version instead of assuming the dev's bleeding-edge one.
//!
//! `assistant/turn.rs::assistant_send` builds the `claude` command with a pile
//! of recent/experimental flags (`--effort`, `--permission-prompt-tool stdio`,
//! `--exclude-dynamic-system-prompt-sections`, `--include-partial-messages`,
//! `--max-budget-usd`, `--settings`, the stream-json control handshake). On a
//! CLI older than the dev's, an unknown flag is a hard spawn failure or an
//! empty turn. This module is the single source of truth for "does the active
//! CLI support flag X" — turn.rs gates each flag through a `CliCaps` field and
//! falls back to a still-working (reduced-feature) spawn when it doesn't.
//!
//! Design invariants:
//!  - **Unreadable version ⇒ conservative-old.** `from_version(None)` gates OFF
//!    every optional flag. A CLI whose `--version` we can't read is assumed too
//!    old, never optimistically new — a missing feature degrades, an unknown
//!    flag crashes.
//!  - **One floor constant per flag.** Min-versions live in `mins` as named
//!    consts so confirming/adjusting one (when the npm changelog is checked) is
//!    a one-line edit. Each carries a confidence note.
//!  - **Floor below everything = `MIN_SUPPORTED`.** Below it Rift can't run a
//!    turn at all (the stream-json + `initialize` control handshake predates
//!    nothing we gate, so this is the true hard floor); turn.rs surfaces an
//!    actionable "update Claude Code" banner instead of a dead turn.

/// Semantic version triple `(major, minor, patch)`.
pub type Version = (u64, u64, u64);

/// Minimum CLI versions each gated capability landed in. Sourced from the
/// `@anthropic-ai/claude-code` release history; confidence noted per const.
/// When a value is confirmed against the changelog, drop the `// est` note.
/// All gated flags re-confirmed present in `claude --help` at v2.1.201
/// (2026-07-06). `--permission-mode default` also still parses there — the
/// mode was renamed "Manual" display-side at 2.1.200 with `manual` accepted
/// as an alias, but `default` remains valid (official permission-modes docs).
mod mins {
    use super::Version;

    /// Stream-json I/O + the idempotent `initialize` control handshake that
    /// Rift's permission plumbing rides (the `control_request`/
    /// `control_response` round-trip with `pending_permission_requests`). This is
    /// the HARD floor — below it Rift can't drive the CLI as designed. Basic
    /// `--input/--output-format stream-json` is 2.0.x-era, but the FULL handshake
    /// formalized at SDK 0.3.161 = CLI v2.1.161 (agent-sdk-typescript CHANGELOG,
    /// confirmed by scout 2026-06-21). Floor set there so the permission channel
    /// is reliable rather than silently half-working.
    pub const STREAM_JSON_CONTROL: Version = (2, 1, 161); // confirmed: handshake @ SDK 0.3.161 / cli 2.1.161

    /// `--include-partial-messages` (partial assistant deltas on the stream).
    /// Changelog: "1.0.109 — SDK: Added partial message streaming support via
    /// `--include-partial-messages` CLI flag" (verified vs raw CHANGELOG.md 2026-07-04).
    pub const INCLUDE_PARTIAL_MESSAGES: Version = (1, 0, 109); // confirmed @ 1.0.109

    /// `--exclude-dynamic-system-prompt-sections` (moves per-machine prompt
    /// sections into the first user message; keeps the cached prefix stable).
    /// Changelog: added at v2.1.98 "for improved cross-user prompt caching"
    /// (verified vs raw CHANGELOG.md 2026-07-04).
    pub const EXCLUDE_DYNAMIC_SECTIONS: Version = (2, 1, 98); // confirmed @ 2.1.98

    /// `--permission-prompt-tool stdio` (routes per-action asks over the control
    /// channel). No direct introduction entry found; confirmed PRESENT ≤ v2.1.152
    /// (turn.rs inline note + scout). Use the confirmed-present bound as the floor.
    pub const PERMISSION_PROMPT_TOOL: Version = (2, 1, 152); // confirmed present @ 2.1.152

    /// `--max-budget-usd` (per-turn spend cap). Changelog: "2.0.28 — SDK: added
    /// --max-budget-usd flag" (verified vs raw CHANGELOG.md 2026-07-04).
    pub const MAX_BUDGET_USD: Version = (2, 0, 28); // confirmed @ 2.0.28

    /// `--effort` (low/medium/high/xhigh/max extended-thinking tier). Confirmed
    /// added at v2.1.142 (luongnv89/claude-howto CHANGELOG, v2.1.143 sync entry).
    pub const EFFORT: Version = (2, 1, 142); // confirmed @ 2.1.142

    /// `--settings '{"ultracode":true}'` (additive settings merge; ultracode
    /// workflow key). `--settings` confirmed added at v2.1.142 (same batch as
    /// `--effort`). The ultracode key itself is plan-gated server-side and
    /// harmlessly ignored when unknown, so this gates only the FLAG's existence.
    pub const SETTINGS_FLAG: Version = (2, 1, 142); // confirmed @ 2.1.142

    /// `--strict-mcp-config` (ignore user `~/.claude.json` MCP servers; use only
    /// `--mcp-config`). No direct introduction entry; confirmed present ≤2.1.152.
    /// Fired only in api-key / local-llm modes (the non-piggyback path).
    pub const STRICT_MCP_CONFIG: Version = (2, 1, 152); // confirmed present @ 2.1.152

    /// `--disable-slash-commands`. Confirmed added at v2.1.170 (zebbern CHANGELOG:
    /// "Update to version 2.1.170 for access"). Also api-key / local-llm only.
    pub const DISABLE_SLASH_COMMANDS: Version = (2, 1, 170); // confirmed @ 2.1.170

    /// `--prompt-suggestions` (emit one `prompt_suggestion` message per turn
    /// with a predicted next user prompt; requires `-p` + stream-json output —
    /// exactly Rift's spawn shape). Absent from the changelog; confirmed
    /// present @2.1.201 via `--help` + exe strings. Confirmed-present bound.
    pub const PROMPT_SUGGESTIONS: Version = (2, 1, 201); // confirmed present @ 2.1.201

    /// `--settings {"fastMode":true}` honored in `-p` mode (Opus fast output;
    /// the result frame answers with `fast_mode_state:"on"` + `usage.speed:
    /// "fast"`). The feature dates to ~2.1.34 (third-party changelog), but the
    /// HEADLESS settings-key path is probe-confirmed only @2.1.209 — a silent
    /// no-op key on an older CLI would render Rift's Fast toggle dead, so this
    /// is a confirmed-present bound. Tighten downward on changelog evidence.
    pub const FAST_MODE: Version = (2, 1, 209); // confirmed present @ 2.1.209 (live probe)

    /// Host→CLI `set_permission_mode` / `set_model` control_requests over the
    /// stream-json stdin (live-switch a warm child instead of drain+respawn).
    /// Foundational SDK machinery (predates the tracked changelog), but 2.1.208
    /// fixed "headless stream-json sessions hanging permanently when a
    /// control_request carried a non-string set_model payload" — floor at the
    /// hardened release rather than risk the hang class on older CLIs.
    pub const LIVE_SWITCH: Version = (2, 1, 208); // hardened @ 2.1.208 (changelog)
}

/// The hard minimum CLI version Rift can drive at all. Below this, no turn can
/// run (no stream-json control handshake) — turn.rs shows an update banner.
pub const MIN_SUPPORTED: Version = mins::STREAM_JSON_CONTROL;

/// Resolved per-flag support for the active CLI install. Built once from the
/// probed version (`from_version`) and threaded into the spawn builder; each
/// `cmd.arg(...)` for a gated flag becomes `if caps.<field> { ... }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliCaps {
    /// Parsed active version, or `None` when unreadable (⇒ everything below false).
    pub version: Option<Version>,
    /// CLI is at or above the hard floor — a turn can run at all.
    pub supported: bool,
    pub include_partial_messages: bool,
    pub exclude_dynamic_sections: bool,
    pub permission_prompt_tool: bool,
    pub max_budget_usd: bool,
    pub effort: bool,
    pub settings_flag: bool,
    pub strict_mcp_config: bool,
    pub disable_slash_commands: bool,
    pub prompt_suggestions: bool,
    pub fast_mode: bool,
    pub live_switch: bool,
}

/// `a >= b` on version triples.
fn at_least(a: Version, b: Version) -> bool {
    a >= b
}

impl CliCaps {
    /// Derive capabilities from a probed version. `None` (version unreadable)
    /// yields the all-off conservative baseline — see the module invariants.
    pub fn from_version(v: Option<Version>) -> Self {
        match v {
            None => Self {
                version: None,
                supported: false,
                include_partial_messages: false,
                exclude_dynamic_sections: false,
                permission_prompt_tool: false,
                max_budget_usd: false,
                effort: false,
                settings_flag: false,
                strict_mcp_config: false,
                disable_slash_commands: false,
                prompt_suggestions: false,
                fast_mode: false,
                live_switch: false,
            },
            Some(ver) => Self {
                version: Some(ver),
                supported: at_least(ver, MIN_SUPPORTED),
                include_partial_messages: at_least(ver, mins::INCLUDE_PARTIAL_MESSAGES),
                exclude_dynamic_sections: at_least(ver, mins::EXCLUDE_DYNAMIC_SECTIONS),
                permission_prompt_tool: at_least(ver, mins::PERMISSION_PROMPT_TOOL),
                max_budget_usd: at_least(ver, mins::MAX_BUDGET_USD),
                effort: at_least(ver, mins::EFFORT),
                settings_flag: at_least(ver, mins::SETTINGS_FLAG),
                strict_mcp_config: at_least(ver, mins::STRICT_MCP_CONFIG),
                disable_slash_commands: at_least(ver, mins::DISABLE_SLASH_COMMANDS),
                prompt_suggestions: at_least(ver, mins::PROMPT_SUGGESTIONS),
                fast_mode: at_least(ver, mins::FAST_MODE),
                live_switch: at_least(ver, mins::LIVE_SWITCH),
            },
        }
    }

    /// Capabilities for the install Rift currently spawns. Reads the cached,
    /// 5s-bounded `--version` probe (`cli_install::active_cli_version`). When no
    /// CLI resolves or its version can't be read, returns the conservative-old
    /// baseline rather than assuming the newest flags exist.
    pub fn active() -> Self {
        Self::from_version(super::cli_install::active_cli_version())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreadable_version_gates_everything_off() {
        let c = CliCaps::from_version(None);
        assert!(!c.supported, "unreadable version is treated as below the hard floor");
        assert!(!c.effort);
        assert!(!c.permission_prompt_tool);
        assert!(!c.exclude_dynamic_sections);
        assert!(!c.include_partial_messages);
        assert!(!c.max_budget_usd);
        assert!(!c.settings_flag);
    }

    #[test]
    fn bleeding_edge_enables_everything() {
        let c = CliCaps::from_version(Some((9, 9, 9)));
        assert!(c.supported);
        assert!(c.effort);
        assert!(c.permission_prompt_tool);
        assert!(c.exclude_dynamic_sections);
        assert!(c.include_partial_messages);
        assert!(c.max_budget_usd);
        assert!(c.settings_flag);
    }

    #[test]
    fn below_hard_floor_is_unsupported() {
        // Below the 2.1.161 handshake floor → unsupported, even though some
        // older flags (max-budget @ 2.0.28, partial @ 1.0.109) "exist" — a turn
        // can't run without the control handshake, so turn.rs bails before spawn.
        let c = CliCaps::from_version(Some((2, 1, 160)));
        assert!(!c.supported);
    }

    #[test]
    fn permission_prompt_tool_gate_boundary() {
        // Just below the confirmed 2.1.152 floor → off.
        assert!(!CliCaps::from_version(Some((2, 1, 151))).permission_prompt_tool);
        // Exactly at the floor → on.
        assert!(CliCaps::from_version(Some((2, 1, 152))).permission_prompt_tool);
    }

    #[test]
    fn mid_version_partial_support() {
        // A 2.1.130 CLI: above the handshake floor's prerequisites for the older
        // flags but below the 2.1.142 effort/settings batch.
        let c = CliCaps::from_version(Some((2, 1, 130)));
        assert!(c.exclude_dynamic_sections, "2.1.98 flag present at 2.1.130");
        assert!(c.max_budget_usd, "2.0.28 flag present at 2.1.130");
        assert!(c.include_partial_messages, "1.0.109 flag present at 2.1.130");
        assert!(!c.effort, "2.1.142 flag not yet present at 2.1.130");
        assert!(!c.settings_flag, "2.1.142 flag not yet present at 2.1.130");
        assert!(!c.permission_prompt_tool, "2.1.152 flag not yet present at 2.1.130");
    }

    #[test]
    fn effort_and_settings_share_2_1_142_floor() {
        assert!(!CliCaps::from_version(Some((2, 1, 141))).effort);
        assert!(!CliCaps::from_version(Some((2, 1, 141))).settings_flag);
        let at = CliCaps::from_version(Some((2, 1, 142)));
        assert!(at.effort);
        assert!(at.settings_flag);
    }

    #[test]
    fn prompt_suggestions_gate_boundary() {
        // 2.1.201 confirmed-present bound (--help + exe strings; not in changelog).
        assert!(!CliCaps::from_version(Some((2, 1, 200))).prompt_suggestions);
        assert!(CliCaps::from_version(Some((2, 1, 201))).prompt_suggestions);

        assert!(!CliCaps::from_version(Some((2, 1, 207))).live_switch);
        assert!(CliCaps::from_version(Some((2, 1, 208))).live_switch);
        assert!(!CliCaps::from_version(Some((2, 1, 208))).fast_mode);
        assert!(CliCaps::from_version(Some((2, 1, 209))).fast_mode);
        assert!(!CliCaps::from_version(None).prompt_suggestions);
    }

    #[test]
    fn disable_slash_commands_gate_boundary() {
        // 2.1.170 floor (confirmed). 2.1.169 (e.g. the --bare era) is just below.
        assert!(!CliCaps::from_version(Some((2, 1, 169))).disable_slash_commands);
        assert!(CliCaps::from_version(Some((2, 1, 170))).disable_slash_commands);
        // strict-mcp-config sits at the 2.1.152 confirmed-present bound.
        assert!(!CliCaps::from_version(Some((2, 1, 151))).strict_mcp_config);
        assert!(CliCaps::from_version(Some((2, 1, 152))).strict_mcp_config);
    }
}
