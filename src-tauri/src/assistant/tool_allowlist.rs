//! `--allowed-tools` string construction for `turn::resolve_spawn`'s
//! `mcp_config_path.is_some()` branch — pure string logic, split out of
//! `turn.rs` so it's unit-testable on its own.

/// Builds the `--allowed-tools` value for a spawn that has an MCP config
/// (i.e. `mcp_config_path` is `Some`). `trust_level` gates the git-write MCP
/// tools, `prompting_mode` selects the narrow safe-only allowlist, and
/// `use_full_config` (when not prompting) widens the CLI-builtin set with
/// `mcp__*` for user-configured MCP servers.
pub(super) fn build_tool_allowlist(
    trust_level: &str,
    prompting_mode: bool,
    use_full_config: bool,
) -> String {
    // S91: full built-in tool set. The CLI's allowlist gate denies any
    // tool name not listed verbatim. S88 added `Skill`; users still hit
    // denials on `Agent` (subagent spawn — used by /plan, /quick-review,
    // /check), `BashOutput`/`KillBash`/`KillShell` (background-bash
    // bookkeeping the CLI auto-invokes after `run_in_background: true`),
    // `MultiEdit`, `NotebookEdit`, `SlashCommand`, `ExitPlanMode`.
    // Wider built-in coverage = fewer denial pop-ups.
    // MCP scope still restricts to rift's tools in the scoped branches.
    //
    // `AskUserQuestion` is INTENTIONALLY omitted: the CLI runs in `-p`
    // (headless) mode with no interactive surface to present the
    // question / capture an answer / inject the tool_result back into
    // the model's stream. When admitted, the model called it and stalled
    // waiting for a tool_result that never arrived, then retried — the
    // user saw two collapsed error bubbles on every question turn.
    // Excluding it makes the model fall back to asking in plain text,
    // which works correctly in `-p` mode.
    // `DesignSync` (claude.ai/design sync, driven by /design-sync) is the
    // built-in for the Claude Design integration; kept out of SAFE_BUILTINS
    // so its cloud writes ride the can_use_tool prompt. OAuth-path only —
    // it has no auth under --bare.
    // Task* are the CLI 2.1.18x+ rename of TodoWrite (the Tasks-dock driver):
    // TaskCreate/TaskUpdate/TaskList/TaskGet/TaskStop. Keep BOTH names — old
    // CLIs emit TodoWrite, new ones emit Task*; the FE (streaming.ts
    // applyTaskCreate/applyTaskUpdate) already renders both into the same Plan
    // card. Omitting Task* silently killed the Tasks panel on current CLI: the
    // model has the tools but the allowlist gated them out, so it fell back to
    // describing the plan in plain text. (Found in the 2026-06-25 stress test.)
    //
    // S107 (2026-07-06, verified vs CLI 2.1.201 sdk-tools.d.ts + exe strings):
    // the 2.1.19x/2.1.20x tool set added EnterPlanMode, ToolSearch (deferred-
    // tool schema fetch), ReportFindings (code-review output), Workflow
    // (multi-agent orchestration), Monitor (background watch), Artifact
    // (claude.ai page publish), REPL (JS scratchpad), SendMessage (agent-to-
    // agent), ScheduleWakeup, Cron*, RemoteTrigger, PushNotification,
    // EnterWorktree/ExitWorktree, and the MCP-resource readers. All listed so
    // a current CLI's tools aren't denial-popped; unknown names are harmless
    // on older CLIs (allowlist entries that never match). SlashCommand/
    // Refreshed 2026-09-18 against CLI 2.1.277 (Rift's hard floor is 2.1.161,
    // past the 2.1.201 removal of MultiEdit/KillBash/KillShell/BashOutput/
    // SlashCommand, so those are dropped). AskUserQuestion stays excluded
    // (see above). S128 (2026-07-08): + PowerShell (the CLI's dedicated
    // Windows shell tool — omitting it denial-gated every PowerShell call on
    // Windows) and LSP (deferred symbol-query tool, loaded via ToolSearch).
    // Skill/plugin discovery (ListSkills/SearchSkills/SuggestSkills/
    // ListPlugins/SearchPlugins/SuggestPluginInstall), ListAgents and the
    // Artifact side tools (ArtifactComments/ArtifactData) are the 2.1.2xx
    // additions.
    const BUILTINS: &str = "Agent,Artifact,ArtifactComments,ArtifactData,Bash,CronCreate,CronDelete,CronList,DesignSync,Edit,EnterPlanMode,EnterWorktree,ExitPlanMode,ExitWorktree,Glob,Grep,ListAgents,ListMcpResources,ListPlugins,ListSkills,LSP,Monitor,NotebookEdit,PowerShell,PushNotification,Read,ReadMcpResource,ReadMcpResourceDir,REPL,RemoteTrigger,ReportFindings,ScheduleWakeup,SearchPlugins,SearchSkills,SendMessage,Skill,SuggestPluginInstall,SuggestSkills,TaskCreate,TaskGet,TaskList,TaskOutput,TaskStop,TaskUpdate,TodoWrite,ToolSearch,WebFetch,WebSearch,Workflow,Write";
    // Read-only / non-mutating subset always auto-approved even in a
    // prompting mode — these shouldn't interrupt the user. Everything
    // omitted (Bash, Edit, Write, Agent, Skill, Workflow, Monitor/REPL
    // (both execute code), Artifact (cloud publish), worktree/Cron/Remote
    // mutations, and the mutating mcp__rift__* tools) falls through to the
    // `can_use_tool` prompt. New no-op additions: ToolSearch (schema fetch),
    // EnterPlanMode (mode flip), ScheduleWakeup (self-timer), ReportFindings
    // (display-only), CronList (read), PushNotification (user-directed toast,
    // parallel to mcp__rift__notify below), LSP (read-only symbol queries).
    // PowerShell executes commands → BUILTINS only, prompts like Bash here.
    // ListAgents + the skill/plugin List/Search/Suggest tools are catalog
    // reads (SuggestPluginInstall only renders a card — the install itself
    // is a separate user click), so they sit here too.
    const SAFE_BUILTINS: &str = "CronList,EnterPlanMode,Glob,Grep,ListAgents,ListPlugins,ListSkills,LSP,PushNotification,Read,ReportFindings,ScheduleWakeup,SearchPlugins,SearchSkills,SuggestPluginInstall,SuggestSkills,TaskCreate,TaskGet,TaskList,TaskOutput,TaskStop,TaskUpdate,TodoWrite,ToolSearch,WebFetch,WebSearch";
    // UI-presentation tools (ask_user / open_browser / notify) are safe to
    // auto-approve: scheme-allowlisted, length-capped, no workspace writes.
    // The browser-dock readers (page text / console) are read-only eyes on
    // the pane the user is already looking at — same no-prompt tier.
    const SAFE_MCP: &str = "mcp__rift__read_file,mcp__rift__list_dir,mcp__rift__grep,mcp__rift__ask_user,mcp__rift__open_browser,mcp__rift__notify,mcp__rift__read_browser_page,mcp__rift__read_browser_console";
    // Local git tools (git_local.rs). Read set is non-mutating → safe to
    // auto-approve even in prompting modes. Write set is admitted in
    // non-prompting variants but deliberately OMITTED from the prompting
    // allowlist so it rides the can_use_tool prompt. RIFT_TRUST_LEVEL is the
    // real authority server-side; these just keep the CLI from rejecting
    // the call before it reaches the server.
    const GIT_READ_MCP: &str = "mcp__rift__git_status,mcp__rift__git_diff,mcp__rift__git_log";
    const GIT_WRITE_MCP: &str = "mcp__rift__git_pull,mcp__rift__git_commit,mcp__rift__git_push";
    // Mirror the server-side gate (mcp_server::trust_at_least("standard")) in
    // the CLI allowlist: only list the git-write tools when trust actually
    // permits them, so the outer allowlist is never wider than the server
    // gate (defense-in-depth — a patched CLI can't call what isn't listed).
    let git_write = if trust_level == "standard" {
        format!(",{GIT_WRITE_MCP}")
    } else {
        String::new()
    };
    if prompting_mode {
        // Narrow allowlist: only the safe set auto-approves; the CLI prompts
        // for the rest via the control channel. Applies across config
        // variants — mutating MCP tools (git write) intentionally prompt here.
        format!("{SAFE_BUILTINS},{SAFE_MCP},{GIT_READ_MCP}")
    } else if use_full_config {
        // `mcp__*` admits any tool from user MCP servers that the CLI
        // merged in (no `--strict-mcp-config`). Rift's tools stay scoped
        // via the explicit-name entries.
        format!("{BUILTINS},mcp__*")
    } else {
        format!("{BUILTINS},{SAFE_MCP},{GIT_READ_MCP}{git_write}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readonly_trust_omits_git_write() {
        let allowed = build_tool_allowlist("readonly", false, false);
        assert!(allowed.contains("mcp__rift__git_status"));
        assert!(!allowed.contains("mcp__rift__git_push"));
    }

    #[test]
    fn standard_trust_admits_git_write() {
        let allowed = build_tool_allowlist("standard", false, false);
        assert!(allowed.contains("mcp__rift__git_push"));
        assert!(allowed.contains("mcp__rift__git_pull"));
    }

    #[test]
    fn prompting_mode_yields_safe_only_regardless_of_trust() {
        let allowed = build_tool_allowlist("standard", true, false);
        assert!(!allowed.contains("Bash"));
        assert!(!allowed.contains("mcp__rift__git_push"));
        assert!(allowed.contains("mcp__rift__read_file"));
    }

    #[test]
    fn full_config_widens_to_mcp_wildcard() {
        let allowed = build_tool_allowlist("readonly", false, true);
        assert!(allowed.contains("mcp__*"));
        assert!(allowed.contains("Bash"));
    }

    #[test]
    fn non_full_config_scopes_to_rift_mcp_only() {
        let allowed = build_tool_allowlist("readonly", false, false);
        assert!(!allowed.contains("mcp__*"));
        assert!(allowed.contains("Bash"));
        assert!(allowed.contains("mcp__rift__read_file"));
    }
}
