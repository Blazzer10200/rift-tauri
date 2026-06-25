# rift-tauri — Changelog

> Live changelog = current version only. History via `git log -- docs/CHANGELOG.md`.

## Unreleased — Workspace dashboard + "What's new in AI" (cont.203, staged, no version bump yet)

> The Workspace page becomes a real dashboard, and recent Anthropic / Claude Code news lands right on it. Staged in the working tree; not yet shipped.

- **Your activity, on the page — not behind a button.** The usage stats (messages, daily chart, sessions / tool calls / spend / streak, model mix) used to hide behind a small "Activity" chip that popped a modal. They now live inline as a full-width band across the top of the Workspace page — visible at a glance, no click.
- **"What's new in AI" feed.** A new panel surfaces recent Claude Code releases automatically (pulled from the official changelog, free, no AI cost) with version, date, and the highlights. One latest release shows by default; "Show more" reveals the rest so the page stays one screen.
- **"Summarize this week in AI" (optional).** A one-tap button has your own Claude search the web and write a short digest of the latest Anthropic + Claude Code news (model launches, API changes) — the same private, on-your-subscription pattern as AI Health's "Analyze my usage." Strictly opt-in; nothing runs until you ask.
- **Reorganized as a bento dashboard** — a wide Activity band on top, then Projects and What's-new side by side, balanced and fitting one screen with no scroll pile.
- **Richer Projects panel.** The Projects list was reorganized into a clear hierarchy: a count badge + "New" button in the header, the active project rendered as a framed hero card (monogram, live "Active" badge, scope, "Continue" button), the rest as a tidy grid of clickable cards, and a single unified "Add a project" zone that merges the adopt-current-folder prompt with recent folders. On-palette with the app accent — no off-color tints.
- **Removed a duplicate folder switcher** from the sidebar — the project/folder selector that sat under "New chat" duplicated context already shown on the Workspace page.

## Unreleased — Works on other Windows machines (cont.202, staged, no version bump yet)

> A cross-machine compatibility pass so Rift runs for users whose Windows setup differs from the developer's. Driven by two multi-agent audit/research workflows (adversarially verified — 19 false leads discarded). Staged in the working tree; not yet shipped.

- **Safer first-run default.** A brand-new user no longer silently starts in the most permissive "bypass all permissions" mode — fresh installs default to asking, and bypass stays an explicit opt-in.
- **Works behind corporate TLS proxies.** Rift now trusts your machine's own Windows-store root certificates for both its own network calls and the Claude CLI it runs, so HTTPS to Anthropic / model downloads / updates no longer fail on networks that inspect TLS (Zscaler/Palo Alto/Netskope-style). Additive only — nothing changes on a normal home network, and certificate verification is never weakened.
- **Finds the Claude CLI in more setups** — custom npm install locations are detected and updated with the right command; a slow first-launch antivirus scan no longer permanently disables CLI features for the session.
- **Locked-down + domain machines.** Security-sensitive files lock down correctly on domain-joined PCs; the installer now bundles the WebView2 runtime so the app still installs on an offline/restricted machine that lacks it.
- **Smaller screens + laptops.** Minimum window size lowered so the window fits 1366×768 laptops and 125–150% display scaling without going off-screen.
- **Fewer dead-ends.** Downloads no longer hang forever on a silently-blocked network; first-time Git push works on a fresh machine; clearer, honest error messages (names the real update host, points expired logins to Sign In, Windows-correct config path).

## v0.36.2 — Fable 5 is back

> The Fable 5 model returned, so it's available again in the model picker.

- **Fable 5 is selectable again.** Anthropic's limited-run Fable 5 model — temporarily pulled — is back, so it's once more an option in the model picker with its full 1M-token context and effort range. Pick it per chat just like Opus, Sonnet, or Haiku.

## Earlier (full detail via `git log -- docs/CHANGELOG.md`)

- **v0.36.1** — The Claude CLI update actually updates now: clicking "Update" on the CLI bar would spin and then snap right back to the same prompt without moving the version (a running background Claude process held the CLI's file locked on Windows, and the bar read the stale version). Now Rift shuts those processes down before updating and re-checks right after. Plus quieter internals — bounded background-agent history and a handful of small correctness fixes.

- **v0.36.0** — One place for everything that happens: a real **notification center** (a bell in the toolbar with history grouped by when, persisting across restarts), updates moved into an always-visible top bar instead of the corner, redesigned toasts, a calmer chat surface (collapsing long code blocks, stream spacing polish), and a fix for a burst of internal errors on startup.

- **v0.35.0** — A reliability pass on the quiet edges: a stuck turn surfaces instead of hanging forever (dead-pipe detection + retry on a fresh process), the auto-updater recovers from a poisoned internal lock, CLI-update checks read as "Checking…" instead of a stale result, and git-timeout kills are scoped to git.

- **v0.31.2–v0.34.0** — Windows that stay in sync (multi-window, one per monitor; conversation list synced across all), **Use my full Claude Code config** (inherit your global `~/.claude` setup), a ground-up Workspace redesign + adopt-an-existing-folder, and a project editor that tells you what's wrong (live glob validation, plain-English errors).

- **v0.27.0–v0.31.1** — Projects + Workspace era: named folder aliases with include/exclude scoping + a merged **Workspace** home, glob matching that actually works (`**` spans folders), AI Health (your own Claude coaches your plan), a 145-finding hardening pass, accessibility polish, the activity stats panel, and finished split-view.

- **v0.20.7–v0.26.3** — Foundation era (detail via `git log`): full redesign port (all 7 surfaces to spec) + stream design language, the warm-CLI process (first reply ~1400ms → ~5ms after, 30-min idle survival, transparent respawn), honest API-stall watchdog, interactive `ask_user` cards, context ring, sub-agent activity dock, latency auto-scale, per-project chat scoping.
