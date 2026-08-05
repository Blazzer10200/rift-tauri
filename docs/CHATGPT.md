# ChatGPT routes, models, effort, and Fast mode

> Runtime contract for Rift's ChatGPT integration. Architecture lives in
> [`ARCHITECTURE.md`](ARCHITECTURE.md); setup and test commands live in
> [`DEVELOPING.md`](DEVELOPING.md).

## Two routes, one visible provider

Rift presents ChatGPT as one provider but keeps its transports and billing
strictly separate.

| Route | Authentication | Billing | History owner | Capability source |
|---|---|---|---|---|
| ChatGPT subscription | Standalone Codex CLI sign-in | ChatGPT plan and credits | Codex thread; Rift stores only its ID | Live App Server `model/list` |
| OpenAI API | Key in Windows Credential Manager | Separately billed API usage | Rift, with `store: false` Responses items | Rift's documented API registry plus `/v1/models` access probe |

The first GPT turn pins one route to the conversation. Model switching cannot
cross that route. If a model or route later becomes unavailable, the turn fails
visibly; Rift never moves subscription work onto API billing or the reverse.

## Subscription model contract

The signed-in App Server catalog is authoritative. Rift reads each model's
exact effort list, default effort, input modalities, upgrade recommendation,
and service tiers. Unknown future models remain selectable but Rift exposes
only capabilities the catalog actually advertises.

Observed catalog on 2026-08-05 (informational, not hard-coded UI policy):

| Model | Best fit | Effort levels | Fast |
|---|---|---|---|
| GPT-5.6 Sol | Hard, open-ended, quality-first work | Low through Max, plus Ultra | About 1.5x speed; 2.5x ChatGPT credits |
| GPT-5.6 Terra | Balanced everyday coding and general work | Low through Max, plus Ultra | About 1.5x speed; 2.5x ChatGPT credits |
| GPT-5.6 Luna | Repeatable, high-volume, cost-sensitive work | Low through Max | About 1.5x speed; 2.5x ChatGPT credits |
| GPT-5.5 | Existing 5.5 workflows | Low through X-High | About 1.5x speed; 2.5x ChatGPT credits |
| GPT-5.4 | Existing 5.4 chats; prefer Terra for new work | Low through X-High | About 1.5x speed; 2x ChatGPT credits |
| GPT-5.4 mini | Existing compact-model chats; prefer Luna for new work | Low through X-High | Not advertised |
| GPT-5.3 Codex Spark | Ultra-fast, focused, text-only coding | Low through X-High | Not advertised; Spark is a separate model |

GPT-5.4 and GPT-5.4 mini are scheduled to leave Codex ChatGPT sign-in on
2026-08-31. Rift displays the live catalog's upgrade target instead of silently
changing an existing conversation.

## API model contract

Rift does not infer capabilities from an arbitrary `gpt-*` name. Known API
models carry reviewed metadata; an account-visible unknown model is shown with
unverified capabilities disabled until Rift catalogs it.

- GPT-5.6 Sol, Terra, Luna, and the 5.6 alias: `none`, `low`, `medium`, `high`,
  `xhigh`, and `max`; 1.05M context; text and image input.
- GPT-5.5: `none`, `low`, `medium`, `high`, and `xhigh`; 1.05M context.
- GPT-5.4: `none`, `low`, `medium`, `high`, and `xhigh`; 1.05M context.
- GPT-5.4 mini: the same effort range with a 400K context window.
- GPT-5.3 Codex: `low`, `medium`, `high`, and `xhigh`; 400K context.

Rift sends the selected value as `reasoning.effort`. Unsupported values fail
before the request rather than being silently downgraded. API Fast is enabled
only for the GPT-5.6 family currently documented for Priority processing.

## Effort semantics

The UI shows provider-native names and never merges distinct levels:

| UI level | Typical use |
|---|---|
| None | Direct transformations and lowest API latency; shown only when the API model supports it |
| Low | Quick lookups, small edits, and minimal reasoning |
| Medium | Balanced default for everyday work |
| High | Complex work where quality matters more than latency |
| X-High | Difficult multi-step work after High is insufficient |
| Max | Hardest single-agent work; avoid as a global default |
| Ultra | Codex multi-agent workflow depth; use only when parallel agent work materially helps |

Turning the lowest rung off maps to the provider's actual floor: `none` on API
models that support it, otherwise `low`. Existing stored Rift values retain
their old meaning, so an old `ultra` preference still means X-High; Codex Ultra
uses a separate stored tier and cannot reinterpret old conversations.

## Fast mode

Fast changes processing priority, not model intelligence or effort. The model
and effort stay independently selectable.

Use Fast for interactive, high-value work where response latency matters:
pairing, short feedback loops, live debugging, or a user waiting on the answer.
Leave it off for long autonomous tasks, background work, batch/ETL jobs, or
when credits/API cost matter more than latency.

- Subscription Fast uses the live model's `priority`/`fast` service tier and
  passes `serviceTier` on thread start/resume and turn start.
- API Fast sends `service_tier: "fast"`. The API can fall back to standard
  processing under capacity pressure.
- The amber control always discloses the route-specific credit or API cost.
- A transcript `fast` chip appears only when the provider response confirms
  `priority`/Fast processing. Requesting Fast alone never earns the chip.
- Models without a Fast service tier do not show the control. Codex Spark's
  speed is intrinsic to that separate model and is not Fast mode.

## Verification boundary

Automated tests cover model parsing, exact effort translation, route pinning,
Fast capability gates, request fields, result confirmation, persistence, and
the settings UI. A real subscription catalog and ephemeral Fast thread start
were also verified on 2026-08-05. The separately billed live API matrix remains
issue `#105` in [`ISSUES.md`](ISSUES.md) and requires a user-owned disposable
key; automated coverage is not presented as a billed-account test.
