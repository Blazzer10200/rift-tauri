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
mod mins {
    use super::Version;

    /// Stream-json I/O + the idempotent `initialize` control handshake that
    /// Rift's permission + steer plumbing rides (the `control_request`/
    /// `control_response` round-trip with `pending_permission_requests`). This is
    /// the HARD floor — below it Rift can't drive the CLI as designed. Basic
    /// `--input/--output-format stream-json` is 2.0.x-era, but the FULL handshake
    /// formalized at SDK 0.3.161 = CLI v2.1.161 (agent-sdk-typescript CHANGELOG,
    /// confirmed by scout 2026-06-21). Floor set there so the permission channel
    /// is reliable rather than silently half-working.
    pub const STREAM_JSON_CONTROL: Version = (2, 1, 161); // confirmed: handshake @ SDK 0.3.161 / cli 2.1.161

    /// `--include-partial-messages` (partial assistant deltas on the stream).
    /// SDK 0.2.108 adds `includePartialMessages`; SDK/CLI track NNN → cli 2.1.108.
    pub const INCLUDE_PARTIAL_MESSAGES: Version = (2, 1, 108); // est (SDK 0.2.108 lockstep)

    /// `--exclude-dynamic-system-prompt-sections` (moves per-machine prompt
    /// sections into the first user message; keeps the cached prefix stable).
    /// SDK 0.2.119 adds `excludeDynamicSections` → cli 2.1.119.
    pub const EXCLUDE_DYNAMIC_SECTIONS: Version = (2, 1, 119); // est (SDK 0.2.119 lockstep)

    /// `--permission-prompt-tool stdio` (routes per-action asks over the control
    /// channel). No direct introduction entry found; confirmed PRESENT ≤ v2.1.152
    /// (turn.rs inline note + scout). Use the confirmed-present bound as the floor.
    pub const PERMISSION_PROMPT_TOOL: Version = (2, 1, 152); // confirmed present @ 2.1.152

    /// `--max-budget-usd` (per-turn spend cap). claude-code changelog entry at
    /// v2.0.31 (ToC-confirmed; secondary sources point there).
    pub const MAX_BUDGET_USD: Version = (2, 0, 31); // est (changelog 2.0.31)

    /// `--effort` (low/medium/high/xhigh/max extended-thinking tier). Confirmed
    /// added at v2.1.142 (luongnv89/claude-howto CHANGELOG, v2.1.143 sync entry).
    pub const EFFORT: Version = (2, 1, 142); // confirmed @ 2.1.142

    /// `--settings '{"ultracode":true}'` (additive settings merge; ultracode
    /// workflow key). `--settings` confirmed added at v2.1.142 (same batch as
    /// `--effort`). The ultracode key itself is plan-gated server-side and
    /// harmlessly ignored when unknown, so this gates only the FLAG's existence.
    pub const SETTINGS_FLAG: Version = (2, 1, 142); // confirmed @ 2.1.142
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
        // older flags (max-budget @ 2.0.31, partial @ 2.1.108) "exist" — a turn
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
        assert!(c.exclude_dynamic_sections, "2.1.119 flag present at 2.1.130");
        assert!(c.max_budget_usd, "2.0.31 flag present at 2.1.130");
        assert!(c.include_partial_messages, "2.1.108 flag present at 2.1.130");
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
}
