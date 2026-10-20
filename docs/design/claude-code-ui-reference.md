# Claude Code UI reference — for tuning Rift's assistant surface

Written by a Claude Code session (2026-06-30) at the user's request: "make Rift's chat
look and behave like Claude Code's own interface." This is a first-hand behavioral spec
of how the real CC CLI/IDE UI works, mapped onto Rift's existing components under
`src/lib/components/assistant/`. It is NOT a rebuild-from-scratch brief — that dir is
already a mature, close analog (thinking collapse w/ live timer, coalesced tool groups,
turn summaries, unified diffs w/ shiki highlighting, first-class Agent/Todo/AskUser
cards). Treat this as a tuning checklist against the real thing, not a spec for new
components. The earlier `rift-redesign.html` exploration was archived to
`~/.claude/archive/rift-design/` on 2026-09-22; `README.md` is the folder index.

A live visual mockup (dark chat transcript using Rift's own oklch tokens — thinking
block, tool chips, unified diff w/ intraline highlight, live shell chip, task list,
sub-agent card, permission bar, streaming cursor) was rendered in the originating
session. Screenshot/recreate from that if useful; not persisted as a file.

**Status (revalidated 2026-09-22):** largely still accurate as a reference — spot
checks against `EditDiff.svelte` (SMALL_DIFF threshold, compactPairs, isWrite
branch), `StreamThinking.svelte` (label contract), and component presence all
matched current code. One section was stale and is now fixed inline: §8's
permission-prompt color, superseded by the `.tile.is-ask` stream-tile system
(commit 834ca67f). `AgentCard.svelte`/`TodoCard.svelte`/`AskUserCard.svelte` now
live under `src/lib/components/assistant/toolchip/`, not flat under `assistant/`.
Most of the checklist describes behavior that is already implemented, not
outstanding work — treat this as a tuning reference to spot-check against, not a
to-do list.

---

## 1. Macro shape of a turn

One continuous column, no chat "bubbles" in the messaging-app sense for the assistant
side — user turns get a distinct inset/right-leaning treatment, assistant turns are
flush and boxless. Inside one assistant turn, blocks stack in strict chronological
arrival order:

```
[thinking?] → [prose] → [tool call] → [prose] → [tool call] → ... → [final prose]
```

Tool calls are NOT batched to the end — they render inline exactly where they occurred
in the model's actual sequence. This is the single most important trait to preserve:
the transcript IS the reasoning trace, not a summary of it. Rift's `MessageBubble.svelte`
already does this (block-ordered render + `coalesceToolGroups`/`numberActions` from
`bubble/helpers.ts`) — keep it that way; don't let a "clean up" pass re-sort tool calls
by type.

## 2. Streaming behavior

Real CC does not fake a per-character typewriter — text renders as actual token chunks
arrive over the wire, in irregular bursts. The only universal visual constant is a
cursor/caret at the tail of the in-flight text node while a block is still open. What
matters for a convincing recreation:

- Blocks appear/extend, never "pop in" fully-formed then get replaced — no flash of
  unstyled/wrong content.
- A trailing caret (thin bar, not a block character) blinks at ~1s period, `step-end`
  timing (not ease), only on the block currently receiving tokens. Every other block
  in the same turn has no caret, including ones that finished a second ago.
- Tool calls appear the instant they're invoked (status = running/spinner), independent
  of whether their result has come back — the chip exists before its body does.
- Markdown renders progressively — a header, list, or code fence forming mid-stream
  should not visually break (no raw `**` flashing before bolding resolves). Rift's
  `Markdown.svelte` + streaming path should be the single source of truth here; if
  there's any regex-based incremental markdown patch, that's the fragile spot to check
  first when streaming looks janky.
- Respect `prefers-reduced-motion` everywhere motion is decorative (caret blink can
  stay — it's a state indicator, not decoration — but slide/fade transitions on
  expand/collapse should no-op). Rift already threads a `reducedMotion` const through
  several components (`ToolChip.svelte`, `MessageBubble.svelte`, `EditDiff.svelte`) —
  confirm it's actually wired into every `slide`/`fade` call, not just computed and
  unused.

## 3. Thinking blocks

Collapsed by default, always. Header is a single row: small icon + label, nothing else.
Label has exactly two states:

- **Active**: `Thinking…` (ellipsis, not three dots as separate chars) — no elapsed
  timer shown while running, or if shown, ticks live.
- **Done**: `Thought for Ns` — rounds to whole seconds, only appears if duration ≥ 1s
  (a sub-second thinking block just says `Thought`, no number — a "for 0s" reads as a
  bug, not a feature).

Clicking expands to the reasoning text: slightly dimmed relative to prose (`--fg-muted`
tier, not `--fg`), no special font, no card border — it's a paragraph, not a panel.
Never auto-expands, never persists expanded state across a new turn. Rift's
`StreamThinking.svelte` already matches this contract almost exactly — the one gap
worth checking: the label only conditionally renders the chevron `{#if text}`, which
is correct (an empty thinking block shouldn't look clickable), but confirm the *button*
itself is still disabled/unclickable when `text` is empty, not just visually chevron-less.

## 4. Tool call chips (collapsed default)

One line, always, when collapsed: `[icon] [verb] [primary target] ... [status]`. No
tool call ever takes more than one line collapsed, no matter how long the input —
truncate with an ellipsis, put the full value in the expanded body or a tooltip. This
is the part most GUI recreations get wrong by trying to show too much inline (full file
paths, full commands) — real CC is aggressively terse here (e.g. a `Grep` shows the
pattern in quotes + scope, not the regex escaped verbatim). Rift's `ToolChip.svelte`
`summary` derivation already does exactly this per-tool truncation — it's the most
faithful piece of the whole implementation and shouldn't need touching.

Status glyph occupies the same visual slot always (right-aligned or fixed-width left of
icon — pick one and never mix): spinner while running, checkmark on success, an X or
triangle on failure — never a color change on the row background itself for status;
status lives in the glyph only. Reserve row-background tinting for permission-required
states, not routine pass/fail.

Expanding a chip reveals input params (compact, monospace, key: value) then result —
never a raw JSON dump as the primary view; format known shapes (file content, grep
matches, command output) and fall back to JSON only for unrecognized tool shapes.

## 5. First-class cards (Agent / Tasks / AskUser)

Three tool types are NOT chips — they're always-expanded cards because their body IS
the message, not a debug detail an engineer might want to peek at:

- **Agent (sub-agent dispatch)**: name + one-line task description + live status
  (pulsing dot while running, static check when done). Does not show the sub-agent's
  own tool calls inline by default — that would double the transcript length — just
  the dispatch + eventual summary. If Rift's `AgentCard.svelte` currently nests full
  sub-transcripts, that's a legitimate divergence (more info) but check it doesn't make
  the parent turn feel like it's scrolling forever.
- **Tasks (TodoWrite)**: flat checklist, three states only — done (check, strikethrough,
  dimmed to `--fg-faint`), in-progress (spinner or filled dot, full-brightness text,
  exactly one at a time), pending (empty/dashed circle, `--fg-muted`). No nesting, no
  priority levels, no due dates — the whole point is it's scannable in under a second.
- **AskUserQuestion**: renders the question(s) as text + each option as a distinct
  clickable target (not a dropdown, not radio buttons styled as buttons — actual button
  affordance). Once answered, collapses to show only the selected option(s), greyed —
  never leaves a live-looking question sitting in scrollback after it's resolved.

Rift's `ToolChip.svelte` already special-cases exactly these three (`isAgent`,
`isTodoWrite`, `isAskUser`) as default-expanded — this is architecturally correct, keep
building on it rather than introducing a fourth pattern.

## 6. Edit / diff blocks

Unified (not side-by-side) — one column, del lines then add lines for a modified
region, context lines between. Gutter shows line numbers for context/add lines only;
del lines carry no number (they don't exist in the new file). This matches what's
actually in `EditDiff.svelte` already (`UnifiedLine`, del has `num: null`) — the
in-file comment header calling it "side-by-side" is stale relative to the
implementation and worth a one-line comment fix, nothing more.

Details that make a diff read as "the real thing" rather than a generic diff widget:

- Header is a breadcrumb, not a filename dump: last 1-2 dir segments + filename +
  a small language badge from the extension, full path in a tooltip. Already present.
- Small edits (few changed lines) auto-expand; large ones default collapsed behind a
  `+N −M` summary — hiding a one-line fix behind a chevron is worse than just showing
  it, but a 200-line rewrite shouldn't eat the whole viewport uninvited. Already present
  (`SMALL_DIFF = 12` threshold) — this exact heuristic is worth preserving verbatim if
  it's ever "simplified."
- Runs of blank *context* lines collapse to a `···N unchanged lines` elision marker;
  blank lines that are themselves part of an add/del/mod are never elided (they're
  meaningful). Already present (`compactPairs`/`gap`).
- Real syntax highlighting per line (not a flat monochrome diff) — already wired via
  the shared shiki core with graceful plain-text fallback before shiki loads.
- Nice-to-have not yet confirmed present: intraline highlighting on `mod` pairs — when
  a line changes `return null;` → `throw new SessionExpiredError();`, highlighting just
  the changed substring (not the whole line) inside the del/add pair reads noticeably
  more polished. Worth a spike if there's spare time, not a blocker.
  - **DEFERRED (2026-07-01):** assessed and intentionally skipped for now. `del`/`add`
    lines already render as shiki-tokenized HTML (`hl()` at ~L251), so intraline marks
    can't be applied to the raw string — they'd have to be interleaved into shiki's
    `<span>` token stream (split spans at word-diff boundaries), which is fragile and
    risks regressing the working syntax highlighting on a core, well-tested renderer.
    Not worth that risk for a polish gain; revisit only with a proper token-merge
    approach (e.g. highlight, then diff at the token level, not the string level).
- A created file (`Write` with no `old_string`) renders as pure additions, not a diff
  against nothing — already handled explicitly (`isWrite` branch), including the "" →
  `[""]` phantom-blank-row trap noted in the source comment. Don't regress this if the
  diff logic gets touched.

## 7. Shell / bash output

Monospace, no syntax highlighting (it's output, not source), streams live line-by-line
while the process runs. A completed run shows exit-relevant status (pass/fail count if
the tool can parse it, otherwise just the terminal status glyph). Long-running/background
shells get a distinct affordance to detach and check back later rather than blocking the
chip open — matches the existing `BashOutput`/`KillBash` tool coverage already in
`ToolChip.svelte`'s summary function; the chip-level UI just needs to expose "still
running in background" as its own state distinct from "finished."

## 8. Permission prompts

Visually distinct from both tool chips and status — this is the one place a tinted
edge on the row itself is correct, because it's the one moment that's actually asking
the user to act, not just reporting. **Updated 2026-09-22:** this now renders as
`.tile.is-ask` from the shared stream-tile system (accent tint + 2px inset edge, per
`design-system/DESIGN.md` §7), not the amber/warn treatment this section originally
called for — `PermissionBar.svelte` explicitly notes it's an ask/prompt tone, not
DESIGN §8's "costs money" amber reservation. Options are explicit buttons (approve /
deny, or a short enumerated list for multi-option cases), never a modal that steals
focus from the rest of the scrollback — the user should be able to keep reading
above/below the prompt while it's live. Maps to `PermissionBar.svelte`.

## 9. Color & status semantics (already correct in Rift's tokens — just enforce the split)

Two independent systems, never conflated:

- **Activity** (`--status-busy` / `--status-pending` / `--status-warn`) — "is the
  system doing something right now." Fixed hex, does not recolor with the accent hue.
  Used for live dots/spinners only.
- **Outcome** (`--ok` / `--warn` / `--danger` / `--info`) — a completed judgment about
  a result. Used for diff add/del tints, pass/fail glyphs, permission-prompt framing.

A running tool call uses activity color; a finished one uses outcome color. Mixing
these (e.g. a green pulse for "success" and a static green dot for "running") is the
kind of thing that reads as inconsistent even when a user can't articulate why.

## 10. Typography & spacing rhythm

- Monospace (`--font-mono`) reserved for: code, file paths, commands, tool names, diff
  content, raw params. Everything else — prose, labels, captions — stays proportional.
  Don't let a "make it feel like a terminal" impulse push monospace into prose; that's
  what makes AI-chat UIs feel like debug consoles instead of products.
- Five-step text ladder (`--fg` → `--fg-faint`) should track semantic weight, not
  visual variety: primary prose = `--fg`, secondary/long-form = `--fg-2`, chip labels
  and captions = `--fg-muted`, metadata (timestamps, counts) = `--fg-subtle`, structural
  eyebrows (section labels, disabled/dimmed state) = `--fg-faint`. Already documented in
  `design-system/foundations/colors.html` — the spec is fine, it's a matter of every new
  block picking the right rung instead of defaulting to `--fg` everywhere.
- Vertical rhythm between blocks inside one turn should be tighter than rhythm between
  turns — a thinking block sitting directly under its triggering prose reads as "part of
  this turn"; too much gap and it reads as a separate event.

## 11. Motion inventory

Everything that animates in this surface, and why — if a future pass adds a new
animation, it should map to one of these purposes, not be decorative:

| Motion | Purpose | Where |
|---|---|---|
| Caret blink | "still receiving tokens" | tail of in-flight text |
| Chevron rotate (0°→90°) | expand/collapse state | every collapsible header |
| Height slide on expand/collapse | avoid content popping | tool chip / thinking body |
| Pulsing dot | "long-running background activity" | sub-agent card, live tool |
| Spinner rotation | "waiting on this specific thing" | running tool glyph, in-progress task |
| Copy-button fade-in on hover | reduce chrome until wanted | message toolbar |

Nothing should animate just because it "feels alive" — every motion in the list above
is load-bearing state communication. Adding ambient motion (shimmer, glow, idle pulse
on static elements) works against the terse, information-dense feel that makes the real
CC UI trustworthy rather than gimmicky.

## 12. Where this maps in the Rift codebase

High confidence (read this session): `ToolChip.svelte` (per-tool summary + Agent/Todo/
AskUser special-casing), `EditDiff.svelte` (unified diff, shiki highlighting, auto-expand
heuristic, blank-run collapsing, write-as-additions), `StreamThinking.svelte` (collapsed/
active/done label contract), `MessageBubble.svelte` (block-ordered render, live tick
heartbeat, reduced-motion threading), `design-system/foundations/colors.html` (activity
vs outcome token split).

`PermissionBar.svelte` (§8) was checked 2026-09-22 and fixed above. Still not read
against the sections above — worth a pass before assuming alignment:
`AgentCard.svelte` / `TodoCard.svelte` / `AskUserCard.svelte` (§5 detail, now under
`toolchip/`), `bubble/TurnSummary.svelte` / `bubble/BoundaryBlock.svelte` (§1 turn
boundaries), `stream/streamModel.ts` (§2 streaming state machine). `Composer.svelte`
is out of scope — this doc is transcript-rendering only.
