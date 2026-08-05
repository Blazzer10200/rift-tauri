// Model, effort, and permission-mode policy; see docs/ARCHITECTURE.md#frontend-map.
// option tables + their pure derivation helpers, lifted from Composer.svelte
// 2026-06-10 so the parent's onKey navigation and the SettingsMenu/PermMenu
// children derive from one source and can never disagree.
import { Hand, Code2, ClipboardList, Zap, Infinity as InfinityIcon, Gem, Feather, Sparkles, Rabbit, Orbit } from "lucide-svelte";
import type { ChatGptRoute, CodexModel, ModelSel, OpenAiModel, PermissionMode, ThinkingEffort } from "../../../state/assistant/types";
import { fableAvailable, fastEligible, haikuAvailable, MODEL_MAX_EFFORT, ctxWindowForModelId } from "../../../state/assistant/helpers";
import { CHATGPT } from "../../../state/assistant/providerDisplay";
import type { ModelSel as ModelSelType } from "../../../state/assistant/types";

type EffortOpt = { id: ThinkingEffort; label: string; hint: string };
// Effort ladder mapped to the CLI's `--effort` flag (low/medium/high/xhigh) by
// effortToFlag (helpers.ts): none→low · smart→medium · deep→high · ultra→xhigh.
// "smart" is Rift's default and maps to `medium` — Anthropic's recommended
// responsive default for interactive use (the CLI default is `high`, but their
// guidance is to override it: high makes the model think for a long time before
// any visible text, which reads as a frozen UI). "deep" (high) is explicit
// heavy reasoning; "ultra" rides xhigh + the ultracode workflow key. Haiku
// rejects effort server-side, so the slider hides entirely there. Labels track
// the real CLI `--effort` flag each tier sends, per the user's "keep it
// vanilla" call — no marketing names. (The legacy "quick" tier — a second
// medium rung labeled "Medium" beside smart's "Medium+" — was retired
// 2026-07-03; stored "quick" folds into "smart" at loadEffort.)
const EFFORT_OPTIONS: EffortOpt[] = [
  { id: "none",  label: "Low",    hint: "Low — minimal reasoning. Fastest answers for quick lookups and small edits." },
  { id: "low",   label: "Low",    hint: "Low — light reasoning for quick, straightforward work." },
  { id: "smart", label: "Medium", hint: "Medium — balanced reasoning + fast responses. The recommended default for everyday work." },
  { id: "deep",  label: "High",   hint: "High — heavier reasoning and more thorough tool use, for complex tasks where quality matters more than speed." },
  { id: "ultra", label: "X-High", hint: "X-High — the model's deepest reasoning level. Slowest and most thorough for difficult, multi-step work." },
  { id: "max", label: "Max", hint: "Max — maximum single-agent reasoning for the hardest tasks. Use only when lower levels miss important details." },
  { id: "agentic", label: "Ultra", hint: "Ultra — Codex's multi-agent workflow level. Best for broad work that benefits from parallel agents; unnecessary for most tasks." },
];

export type ModelOpt = {
  id: ModelSel;
  label: string;
  version: string;
  tagline: string;
  blurb: string;     // short second-line description in the model menu row
  ctx: string;
  suffix: string;    // muted inline tag beside the name (e.g. "1M context")
  legacy: boolean;   // previous-generation — moved into the "More models" flyout
  limited?: boolean; // limited-run — accent name + "until" badge (Fable)
  icon: typeof Gem;  // per-model identity glyph in the row's leading tile
  provider: "claude" | "openai";
  // ── Capability matrix (source of truth for what each model can actually do).
  // Drives every affordance gate so the panel never offers a mode the model
  // ignores server-side. Grounded in the model capability docs:
  //   • effort     — accepts the CLI `--effort` flag at all. The API rejects
  //                  effort on Haiku 4.5 wholesale, so this is false for Haiku.
  //   • maxEffort  — highest effort tier the model honors. Opus, Fable, and
  //                  Sonnet 5 reach "ultra" (xhigh + ultracode); Sonnet 5
  //                  honors xhigh + max server-side, where Sonnet 4.6 rejected
  //                  xhigh and stopped at deep. Haiku rejects effort entirely.
  effort: boolean;
  maxEffort: ThinkingEffort;
  /** Exact Rift effort tiers advertised by a live model source. Omitted for
   *  curated Claude/API fallback rows, which continue to use maxEffort as a
   *  contiguous ceiling. */
  supportedEfforts?: readonly ThinkingEffort[];
  defaultReasoningEffort?: string | null;
  fastServiceTier?: { id: string; name: string; description: string } | null;
  upgradeModel?: string | null;
  upgradeCopy?: string | null;
  inputModalities?: readonly string[];
  supportsPersonality?: boolean;
};
// Flat single-column list (Claude-Code-Desktop layout): current models first,
// legacy generations grouped below. `opus` is the alias → newest Opus (5,
// 1M-ctx); `claude-opus-4-8` pins the prior generation. The CLI takes
// the alias / pinned id; name + suffix are display-only.
// Fable 5 row exists while fableAvailable() (gated by FABLE_DISABLED + the
// sunset date in helpers.ts). Owner call 2026-07-01: kept always-visible even
// while the upstream access gate holds — hard-pull only (set FABLE_DISABLED).
export const MODEL_OPTIONS: ModelOpt[] = [
  { id: "gpt-5.6", label: "GPT", version: "5.6", tagline: "Current GPT-5.6 family alias", blurb: "Current general-purpose GPT", ctx: "1.05M ctx", suffix: "1.05M context", legacy: false, effort: true, maxEffort: MODEL_MAX_EFFORT["gpt-5.6"], icon: Orbit, provider: "openai" },
  { id: "gpt-5.6-sol", label: "GPT Sol", version: "5.6", tagline: "Frontier model for complex, open-ended work and maximum polish", blurb: "Highest capability and detail", ctx: "1.05M ctx", suffix: "1.05M context", legacy: false, effort: true, maxEffort: MODEL_MAX_EFFORT["gpt-5.6-sol"], icon: Orbit, provider: "openai" },
  { id: "gpt-5.6-terra", label: "GPT Terra", version: "5.6", tagline: "Balanced everyday workhorse for coding and general tasks", blurb: "Best capability/speed balance", ctx: "1.05M ctx", suffix: "1.05M context", legacy: false, effort: true, maxEffort: MODEL_MAX_EFFORT["gpt-5.6-terra"], icon: Orbit, provider: "openai" },
  { id: "gpt-5.6-luna", label: "GPT Luna", version: "5.6", tagline: "Efficient model for repeatable, high-volume, cost-sensitive work", blurb: "Fastest and most economical 5.6", ctx: "1.05M ctx", suffix: "1.05M context", legacy: false, effort: true, maxEffort: MODEL_MAX_EFFORT["gpt-5.6-luna"], icon: Orbit, provider: "openai" },
  { id: "gpt-5.5", label: "GPT", version: "5.5", tagline: "Previous-generation general reasoning model", blurb: "Use when an existing workflow needs 5.5", ctx: "1.05M ctx", suffix: "1.05M context", legacy: false, effort: true, maxEffort: "ultra", icon: Orbit, provider: "openai" },
  { id: "gpt-5.4", label: "GPT", version: "5.4", tagline: "Retiring ChatGPT model; move new work to Terra", blurb: "Compatibility for existing chats", ctx: "1.05M ctx", suffix: "1.05M context", legacy: false, effort: true, maxEffort: "ultra", icon: Orbit, provider: "openai" },
  { id: "gpt-5.4-mini", label: "GPT Mini", version: "5.4", tagline: "Retiring compact model; move new work to Luna", blurb: "Compatibility for existing chats", ctx: "400K ctx", suffix: "400K context", legacy: false, effort: true, maxEffort: "ultra", icon: Orbit, provider: "openai" },
  { id: "gpt-5.3-codex-spark", label: "GPT Codex Spark", version: "5.3", tagline: "Separate ultra-fast, text-only coding model; not Fast mode", blurb: "Quick focused coding work", ctx: "400K ctx", suffix: "400K context", legacy: false, effort: true, maxEffort: "ultra", icon: Code2, provider: "openai" },
  { id: "gpt-5.3-codex", label: "GPT Codex", version: "5.3", tagline: "ChatGPT's agentic coding model", blurb: "Coding-focused reasoning & tools", ctx: "400K ctx", suffix: "400K context", legacy: false, effort: true, maxEffort: MODEL_MAX_EFFORT["gpt-5.3-codex"], icon: Code2, provider: "openai" },
  ...(fableAvailable() ? [{ id: "claude-fable-5" as ModelSel, label: "Fable", version: "5", tagline: "Anthropic's most capable model — limited run", blurb: "Most capable — limited run", ctx: "1M ctx", suffix: "1M context", legacy: false, limited: true, effort: true, maxEffort: MODEL_MAX_EFFORT["claude-fable-5"], icon: Sparkles, provider: "claude" as const }] : []),
  { id: "opus",            label: "Opus",   version: "5",   tagline: "Newest + most capable — complex reasoning & agentic coding", blurb: "Deep reasoning & agentic coding", ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: MODEL_MAX_EFFORT.opus, icon: Gem, provider: "claude" },
  { id: "sonnet",          label: "Sonnet", version: "5",   tagline: "Best speed + intelligence balance — the default",            blurb: "Everyday default — speed + smarts", ctx: "1M ctx",   suffix: "1M context",   legacy: false, effort: true,  maxEffort: MODEL_MAX_EFFORT.sonnet, icon: Feather, provider: "claude" },
  ...(haikuAvailable() ? [{ id: "haiku" as ModelSel, label: "Haiku", version: "4.5", tagline: "Fastest, near-frontier — quick edits & lookups", blurb: "Fastest — quick edits & lookups", ctx: "200K ctx", suffix: "200K context", legacy: false, effort: false, maxEffort: MODEL_MAX_EFFORT.haiku, icon: Rabbit, provider: "claude" as const }] : []),
  { id: "claude-opus-4-8", label: "Opus",   version: "4.8", tagline: "Previous-generation Opus — proven for complex reasoning",    blurb: "Previous-generation Opus", ctx: "1M ctx",   suffix: "1M context",   legacy: true,  effort: true,  maxEffort: MODEL_MAX_EFFORT["claude-opus-4-8"], icon: Gem, provider: "claude" },
  { id: "claude-opus-4-7", label: "Opus",   version: "4.7", tagline: "Earlier Opus generation — proven for complex reasoning",     blurb: "Earlier Opus generation", ctx: "1M ctx",   suffix: "1M context",   legacy: true,  effort: true,  maxEffort: MODEL_MAX_EFFORT["claude-opus-4-7"], icon: Gem, provider: "claude" },
  { id: "claude-opus-4-6", label: "Opus",   version: "4.6", tagline: "Earlier Opus generation — deep reasoning workhorse",         blurb: "Earlier Opus generation", ctx: "1M ctx",   suffix: "1M context",   legacy: true,  effort: true,  maxEffort: MODEL_MAX_EFFORT["claude-opus-4-6"], icon: Gem, provider: "claude" },
  { id: "claude-opus-4-5", label: "Opus",   version: "4.5", tagline: "Classic Opus — proven reasoning, 200K context",              blurb: "Classic Opus generation", ctx: "200K ctx", suffix: "200K context", legacy: true,  effort: true,  maxEffort: MODEL_MAX_EFFORT["claude-opus-4-5"], icon: Gem, provider: "claude" },
  { id: "claude-sonnet-4-6", label: "Sonnet", version: "4.6", tagline: "Previous-generation Sonnet — balanced speed + smarts",     blurb: "Previous-generation Sonnet", ctx: "1M ctx", suffix: "1M context",   legacy: true,  effort: true,  maxEffort: MODEL_MAX_EFFORT["claude-sonnet-4-6"], icon: Feather, provider: "claude" },
  { id: "claude-sonnet-4-5", label: "Sonnet", version: "4.5", tagline: "Classic Sonnet — everyday workhorse, 200K context",        blurb: "Classic Sonnet generation", ctx: "200K ctx", suffix: "200K context", legacy: true, effort: true,  maxEffort: MODEL_MAX_EFFORT["claude-sonnet-4-5"], icon: Feather, provider: "claude" },
];

const CODEX_TO_RIFT_EFFORT: Readonly<Record<string, ThinkingEffort>> = {
  low: "none",
  medium: "smart",
  high: "deep",
  xhigh: "ultra",
  max: "max",
  ultra: "agentic",
};

const API_TO_RIFT_EFFORT: Readonly<Record<string, ThinkingEffort>> = {
  none: "none",
  low: "low",
  medium: "smart",
  high: "deep",
  xhigh: "ultra",
  max: "max",
};

function mappedEfforts(
  efforts: readonly string[],
  mapping: Readonly<Record<string, ThinkingEffort>>,
): ThinkingEffort[] {
  const mapped = new Set(
    efforts
      .map((effort) => mapping[effort.trim().toLowerCase()])
      .filter((effort): effort is ThinkingEffort => effort !== undefined),
  );
  return EFFORT_OPTIONS.map((effort) => effort.id).filter((effort) => mapped.has(effort));
}

/** Translate App Server effort names into Rift's stored tiers. Unknown future
 *  values stay hidden until Rift knows how to send them; known values are
 *  de-duplicated and returned in low→high order. */
export function codexReasoningEffortsFor(model: Pick<CodexModel, "supportedReasoningEfforts">): ThinkingEffort[] {
  return mappedEfforts(model.supportedReasoningEfforts, CODEX_TO_RIFT_EFFORT);
}

export function openAiReasoningEffortsFor(model: Pick<OpenAiModel, "supportedReasoningEfforts">): ThinkingEffort[] {
  return mappedEfforts(model.supportedReasoningEfforts, API_TO_RIFT_EFFORT);
}

function fallbackCodexLabel(model: CodexModel): string {
  const liveLabel = model.label.trim();
  if (liveLabel && liveLabel.toLowerCase() !== model.id.toLowerCase()) return liveLabel;
  return model.id
    .split("-")
    .map((part) => part.toLowerCase() === "gpt"
      ? "GPT"
      : part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function liveCodexModelOpt(model: CodexModel, curated: readonly ModelOpt[]): ModelOpt {
  const match = curated.find((candidate) => candidate.id === model.id);
  const supportedEfforts = codexReasoningEffortsFor(model);
  const maxEffort = supportedEfforts[supportedEfforts.length - 1] ?? "none";
  if (match) {
    return {
      ...match,
      tagline: model.description.trim() || match.tagline,
      effort: supportedEfforts.length > 0,
      maxEffort,
      supportedEfforts,
      defaultReasoningEffort: model.defaultReasoningEffort,
      fastServiceTier: model.serviceTiers.find((tier) => tier.id === "priority" || tier.id === "fast") ?? null,
      upgradeModel: model.upgradeModel,
      upgradeCopy: model.upgradeCopy,
      inputModalities: model.inputModalities,
      supportsPersonality: model.supportsPersonality,
    };
  }

  const label = fallbackCodexLabel(model);
  return {
    id: model.id,
    label,
    version: "",
    tagline: model.description.trim() || `${label} is available through your ChatGPT account`,
    blurb: model.imageInput ? "Text, vision & tools" : "Text & tools",
    ctx: "Account",
    suffix: "",
    legacy: false,
    icon: model.id.includes("codex") ? Code2 : Orbit,
    provider: "openai",
    effort: supportedEfforts.length > 0,
    maxEffort,
    supportedEfforts,
    defaultReasoningEffort: model.defaultReasoningEffort,
    fastServiceTier: model.serviceTiers.find((tier) => tier.id === "priority" || tier.id === "fast") ?? null,
    upgradeModel: model.upgradeModel,
    upgradeCopy: model.upgradeCopy,
    inputModalities: model.inputModalities,
    supportsPersonality: model.supportsPersonality,
  };
}

/** Picker-ready ChatGPT rows. App Server is the live source when connected;
 *  curated rows supply polished metadata for known ids and remain appended as
 *  the separately billed API fallback. Unknown future live ids still receive
 *  a complete, selectable row. */
export function chatGptModelsFor(
  codexModels: readonly CodexModel[] | null,
  openAiModels: readonly OpenAiModel[] | null = null,
): ModelOpt[] {
  const curated = MODEL_OPTIONS.filter((model) => model.provider === "openai" && !model.legacy);
  const seen = new Set<string>();
  const live = (codexModels ?? []).flatMap((model) => {
    if (seen.has(model.id)) return [];
    seen.add(model.id);
    return [liveCodexModelOpt(model, curated)];
  });
  const fallback = curated.filter((model) => {
    if (seen.has(model.id)) return false;
    seen.add(model.id);
    return true;
  });
  const apiOnly = (openAiModels ?? []).flatMap((model) => {
    if (!model.available || seen.has(model.id)) return [];
    seen.add(model.id);
    const supportedEfforts = openAiReasoningEffortsFor(model);
    const label = model.label.trim() || model.id;
    return [{
      id: model.id as ModelSel,
      label,
      version: "",
      tagline: model.description,
      blurb: model.imageInput ? "Text, vision & tools" : "Text & tools",
      ctx: model.contextWindow ? `${Math.round(model.contextWindow / 1000)}K ctx` : "API account",
      suffix: model.contextWindow ? `${Math.round(model.contextWindow / 1000)}K context` : "",
      legacy: false,
      icon: model.id.includes("codex") ? Code2 : Orbit,
      provider: "openai" as const,
      effort: model.reasoning && supportedEfforts.length > 0,
      maxEffort: supportedEfforts[supportedEfforts.length - 1] ?? "none",
    }];
  });
  return [...live, ...fallback, ...apiOnly];
}

export type ProviderAccessContext = {
  claudeReady: boolean;
  claudeChecking: boolean;
  claudeError: string | null;
  claudeFree: boolean;
  codexReady: boolean;
  codexChecking: boolean;
  codexModels: readonly CodexModel[] | null;
  codexError: string | null;
  openAiConfigured: boolean;
  openAiChecking: boolean;
  openAiModels: readonly OpenAiModel[] | null;
  openAiError: string | null;
};

export type ModelAccessState = "ready" | "checking" | "setup" | "unavailable" | "error";
export type ModelAccess = {
  state: ModelAccessState;
  enabled: boolean;
  tag: string;
  detail: string;
};

/** One honest access decision shared by the composer, model picker, and send
 * gate. A stored key is only configuration; ChatGPT API models become selectable
 * after the account model probe confirms the exact id. Claude requires the
 * CLI because every Claude turn still routes through it. */
export function modelAccessFor(
  m: ModelOpt,
  ctx: ProviderAccessContext,
  pinnedRoute: ChatGptRoute | null = null,
): ModelAccess {
  if (m.provider === "claude") {
    if (ctx.claudeFree && !isFreeClaudeModel(m.id)) {
      return { state: "unavailable", enabled: false, tag: "Paid plan", detail: `${m.label} isn't included in Claude Free.` };
    }
    if (ctx.claudeReady) return { state: "ready", enabled: true, tag: "Connected", detail: m.tagline };
    if (ctx.claudeChecking) return { state: "checking", enabled: false, tag: "Checking", detail: "Checking the Claude Code connection." };
    if (ctx.claudeError) return { state: "error", enabled: false, tag: "Check failed", detail: ctx.claudeError };
    return { state: "setup", enabled: false, tag: "Connect Claude", detail: "Install and sign in to Claude Code in Settings → Providers." };
  }

  const codexModel = ctx.codexModels?.find((model) => model.id === m.id);
  const apiModel = ctx.openAiModels?.find((model) => model.id === m.id);
  if (pinnedRoute === "codex") {
    if (ctx.codexChecking) return { state: "checking", enabled: false, tag: "Checking", detail: "Checking your pinned ChatGPT subscription route." };
    if (ctx.codexReady && codexModel) return {
      state: "ready", enabled: true, tag: "Subscription",
      detail: `${codexModel.description || m.tagline} Available through this chat's pinned ChatGPT subscription route.`,
    };
    return {
      state: ctx.codexError ? "error" : "unavailable",
      enabled: false,
      tag: ctx.codexError ? "Check failed" : "Not in plan",
      detail: ctx.codexError ?? `${m.label} ${m.version} is not available through this chat's pinned ChatGPT subscription route.`,
    };
  }
  if (pinnedRoute === "openai") {
    if (ctx.openAiChecking) return { state: "checking", enabled: false, tag: "Checking", detail: "Checking this chat's pinned OpenAI API route." };
    if (ctx.openAiError || !ctx.openAiModels) return {
      state: "error", enabled: false, tag: "Check failed",
      detail: ctx.openAiError ?? "Rift could not verify this chat's pinned OpenAI API route.",
    };
    if (ctx.openAiConfigured && apiModel?.available) return {
      state: "ready", enabled: true, tag: "API · separately billed",
      detail: `${m.tagline} This chat remains on separately billed OpenAI API access.`,
    };
    return {
      state: "unavailable", enabled: false, tag: "No API access",
      detail: `${m.label} ${m.version} is not available through this chat's pinned OpenAI API route.`,
    };
  }
  if (ctx.codexReady && codexModel) {
    return {
      state: "ready",
      enabled: true,
      tag: "Subscription",
      detail: `${codexModel.description || m.tagline} Available through your signed-in ChatGPT subscription.`,
    };
  }

  if (ctx.codexChecking && !ctx.openAiConfigured) {
    return { state: "checking", enabled: false, tag: "Checking", detail: "Checking your ChatGPT subscription." };
  }

  if (!ctx.openAiConfigured) {
    if (ctx.openAiChecking) return { state: "checking", enabled: false, tag: "Checking", detail: `Checking ${CHATGPT.apiAccess}.` };
    if (ctx.codexError && ctx.openAiError) return { state: "error", enabled: false, tag: "Check failed", detail: ctx.codexError };
    if (ctx.codexReady) return { state: "unavailable", enabled: false, tag: "Not in plan", detail: `${m.label} ${m.version} isn't offered by your ChatGPT account.` };
    return { state: "setup", enabled: false, tag: "Connect ChatGPT", detail: "Sign in with ChatGPT in Settings → Providers. An API key is optional." };
  }
  if (ctx.openAiChecking) return { state: "checking", enabled: false, tag: "Checking", detail: `Checking which models ${CHATGPT.apiAccess} can use.` };
  if (ctx.openAiError || !ctx.openAiModels) {
    return { state: "error", enabled: false, tag: "Check failed", detail: ctx.openAiError ?? "Rift couldn't verify this model. Re-probe AI connections in Settings." };
  }
  if (!apiModel?.available) {
    return { state: "unavailable", enabled: false, tag: "No access", detail: `${m.label} ${m.version} isn't available through ${CHATGPT.apiAccess}.` };
  }
  return {
    state: "ready",
    enabled: true,
    tag: "API · separately billed",
    detail: `${m.tagline} ${CHATGPT.apiBilling}`,
  };
}

export function isFreeClaudeModel(id: ModelSel): boolean {
  return id === "sonnet" || id === "haiku" || id.includes("sonnet");
}

export function providerStatusFor(provider: ModelOpt["provider"], ctx: ProviderAccessContext): ModelAccess {
  const models = provider === "openai"
    ? chatGptModelsFor(ctx.codexModels, ctx.openAiModels)
    : currentModels.filter((m) => m.provider === provider);
  const access = models.map((m) => modelAccessFor(m, ctx));
  return access.find((item) => item.enabled)
    ?? access.find((item) => item.state === "checking")
    ?? access.find((item) => item.state === "error")
    ?? access[0];
}

/** OpenAI's live model metadata can turn reasoning off for a supported id.
 * Honor that instead of rendering an effort dial the API will ignore. */
export function effortCapsFor(
  m: ModelOpt | undefined,
  models: readonly OpenAiModel[] | null,
  route: ChatGptRoute | null = null,
): EffortCaps | undefined {
  if (!m || m.provider !== "openai") return m;
  if (route === "codex") return { ...m, offWireEffort: "low" };
  const live = models?.find((model) => model.id === m.id);
  if (!live || !live.reasoning) {
    return { effort: false, maxEffort: "none", supportedEfforts: [], offWireEffort: "none" };
  }
  const supportedEfforts = openAiReasoningEffortsFor(live);
  return {
    effort: supportedEfforts.length > 0,
    maxEffort: supportedEfforts[supportedEfforts.length - 1] ?? "none",
    supportedEfforts,
    offWireEffort: supportedEfforts.includes("none") ? "none" : "low",
  };
}

export type FastModeInfo = {
  tag: string;
  summary: string;
  avoid: string;
  tooltip: string;
  notice: { key: string; title: string; detail: string };
};

/** Fast is a provider route, not a model-family guess. Subscription support is
 *  taken from the connected App Server catalog; API support is taken from the
 *  native adapter metadata; Claude retains its established Opus gate. */
export function fastModeInfoFor(
  m: ModelOpt | undefined,
  route: ChatGptRoute | null,
  codexModels: readonly CodexModel[] | null,
  openAiModels: readonly OpenAiModel[] | null,
): FastModeInfo | null {
  if (!m) return null;
  if (m.provider === "claude") {
    if (!fastEligible(m.id)) return null;
    return {
      tag: "extra credits",
      summary: "Quicker Opus output using higher-speed processing.",
      avoid: "Skip when latency is not worth extra credits.",
      tooltip: "Fast mode — quicker Opus output, billed from usage credits rather than plan limits. Applies from your next message.",
      notice: {
        key: "claude",
        title: "Fast mode uses extra credits",
        detail: "Fast Opus turns draw from usage credits beyond plan limits. The ⚡ chip marks turns that actually ran fast.",
      },
    };
  }
  if (route === "codex") {
    const model = codexModels?.find((candidate) => candidate.id === m.id);
    const tier = model?.serviceTiers.find((candidate) => candidate.id === "priority" || candidate.id === "fast");
    if (!tier) return null;
    const multiplier = m.id.startsWith("gpt-5.6") || m.id === "gpt-5.5"
      ? "2.5× credits"
      : m.id === "gpt-5.4"
        ? "2× credits"
        : "increased credits";
    return {
      tag: multiplier,
      summary: `${tier.description || "About 1.5× faster output"} · ${multiplier}.`,
      avoid: "Skip long or background work.",
      tooltip: `ChatGPT Fast mode — ${tier.description || "about 1.5× faster"}, using ${multiplier}. Applies from your next message.`,
      notice: {
        key: "chatgpt-subscription",
        title: `ChatGPT Fast uses ${multiplier}`,
        detail: "Fast is best for interactive work where latency matters. Leave it off for long, background, or credit-sensitive tasks. The ⚡ chip confirms turns that ran fast.",
      },
    };
  }
  if (route === "openai") {
    const model = openAiModels?.find((candidate) => candidate.id === m.id);
    if (!model?.fastMode) return null;
    return {
      tag: "premium API",
      summary: "Up to 2.5× faster · premium API rates.",
      avoid: "Skip batch or background work.",
      tooltip: "Fast API processing — up to 2.5× faster at premium token rates. Applies from your next message; capacity can fall back to standard processing.",
      notice: {
        key: "openai-api",
        title: "Fast uses premium API pricing",
        detail: "Use Fast for high-value, latency-sensitive interactive work. Leave it off for batch, background, or cost-sensitive tasks. The ⚡ chip appears only when the API confirms priority processing.",
      },
    };
  }
  return null;
}

/** The context-window tag shown beside a model in the picker, HONEST under the
 *  user's plan: a 1M-native model capped to 200K by a Free plan must read "200K
 *  context", not the static "1M context" baked into MODEL_OPTIONS. Derives from
 *  the SAME `ctxWindowForModelId` that drives the gauge + compaction pill, so all
 *  three agree. `planCap` is the value from `assistant.planCap`. */
export function modelWindowSuffix(
  id: ModelSelType,
  planCap: number,
  reported?: { model: string; window: number } | null,
): string {
  const norm = (value: string) => value.replace(/\[1m\]$/i, "").toLowerCase();
  const w = reported && norm(reported.model) === norm(id)
    ? reported.window
    : ctxWindowForModelId(id, planCap);
  if (w > 1_000_000) return `${(w / 1_000_000).toFixed(2)}M context`;
  return w >= 1_000_000 ? "1M context" : `${Math.round(w / 1000)}K context`;
}

// Current-generation models surface in the main list; previous generations
// (`legacy`) fold into the "More models" flyout — matching the desktop picker.
// Both derive from the one MODEL_OPTIONS table so the number hotkeys (which
// index into MODEL_OPTIONS) and the row split can never disagree.
export const currentModels = MODEL_OPTIONS.filter((m) => !m.legacy);
export const legacyModels = MODEL_OPTIONS.filter((m) => m.legacy);

// ── Reasoning ladder ─────────────────────────────────────────────────────────
// ONE control for reasoning depth. The old separate `Thinking` toggle + effort
// slider rendered two knobs over ONE wire lever: the CLI has no thinking-disable
// flag (`--effort` is the only control), so toggle-OFF sent `--effort low`
// regardless of the slider (the slider was decorative) and toggle-ON sent the
// slider's tier — i.e. the toggle WAS an effort jump, which read as "thinking is
// 5× slower for the same answer". The ladder exposes the real states, one rung
// per distinct CLI `--effort` flag. Rung 0 (`effort: null`) writes
// thinkingEnabled=false — the exact wire state of the old toggle-off (sends
// `--effort low`, rides the no-think shim on the API-key path) — so the fast
// default and the effortToFlag 3-file lockstep are untouched. Rungs 1+ write
// thinkingEnabled=true + their tier atomically (assistant.setThinkingDial).
// (The legacy `quick` tier is coerced to `smart` at loadEffort — a rung here
// never sees it.)
type DialId = "none" | "low" | "medium" | "high" | "xhigh" | "max" | "ultra";
export type DialStop = {
  id: DialId;
  label: string;
  hint: string;
  /** null = fastest rung (extended thinking off → wire `--effort low`).
   *  Otherwise the ThinkingEffort tier this rung selects (thinking on). */
  effort: ThinkingEffort | null;
};
// Labels track the real CLI `--effort` flag each rung sends (vanilla flag
// names, no marketing labels — standing owner call).
const REASONING_STOPS: DialStop[] = [
  { id: "low",    label: "Low",    effort: "low",  hint: "Low — light reasoning for quick, straightforward work." },
  { id: "medium", label: "Medium", effort: "smart", hint: "Medium — balanced reasoning at a responsive pace. The recommended default for everyday work." },
  { id: "high",   label: "High",   effort: "deep",  hint: "High — reasons before replying and works more thoroughly, for complex tasks where quality matters more than speed." },
  { id: "xhigh",  label: "X-High", effort: "ultra", hint: "X-High — the model's deepest reasoning level. Slowest and most thorough for difficult, multi-step work." },
  { id: "max", label: "Max", effort: "max", hint: "Max — maximum single-agent reasoning for the hardest tasks. Use only when X-High is not enough." },
  { id: "ultra", label: "Ultra", effort: "agentic", hint: "Ultra — multi-agent workflow depth for broad tasks that benefit from parallel work. Most tasks do not need it." },
];

/** Minimal capability shape the ladder derives from — a full ModelOpt. */
export type EffortCaps = Pick<ModelOpt, "effort" | "maxEffort" | "supportedEfforts"> & {
  offWireEffort?: "none" | "low";
};

/** The ladder rungs a model supports. Haiku (no effort capability) gets an
 *  empty list — the panel hides the ladder entirely. Otherwise rungs truncate
 *  at the model's effort ceiling; the Low rung is always present. */
export function dialStopsFor(m: EffortCaps | undefined): DialStop[] {
  if (!m?.effort) return [];
  const offWireEffort = m.offWireEffort ?? "low";
  const off: DialStop = offWireEffort === "none"
    ? { id: "none", label: "None", effort: null, hint: "None — no reasoning budget. Best for direct transformations and the lowest latency." }
    : { id: "low", label: "Low", effort: null, hint: "Low — minimal reasoning. Fastest for chat, quick lookups, and small edits." };
  if (m.supportedEfforts !== undefined) {
    const supported = new Set(m.supportedEfforts);
    const enabled = REASONING_STOPS.filter((stop) => supported.has(stop.effort ?? "none"));
    if (offWireEffort === "low") {
      return [off, ...enabled.filter((stop) => stop.effort !== "low")];
    }
    return [off, ...enabled];
  }
  const capIdx = EFFORT_OPTIONS.findIndex((e) => e.id === m.maxEffort);
  return [off, ...REASONING_STOPS.filter(
    (s) => s.effort !== "low"
      && capIdx >= 0
      && EFFORT_OPTIONS.findIndex((e) => e.id === s.effort) <= capIdx,
  )];
}

/** Clamp a stored tier to an exact live capability set. Static Claude rows do
 *  not call this helper; their established max-effort clamp remains unchanged. */
export function clampEffortForCaps(effort: ThinkingEffort, caps: EffortCaps): ThinkingEffort {
  const supported = caps.supportedEfforts;
  if (!caps.effort || supported === undefined || supported.length === 0) return "none";
  if (supported.includes(effort)) return effort;
  const effortIdx = EFFORT_OPTIONS.findIndex((option) => option.id === effort);
  const lower = supported.filter(
    (candidate) => EFFORT_OPTIONS.findIndex((option) => option.id === candidate) <= effortIdx,
  );
  return lower[lower.length - 1] ?? supported[0];
}

/** Project the store pair (thinkingEnabled, thinkingEffort) onto a rung index.
 *  Off → the Low rung whatever tier is parked in storage — that IS the wire
 *  truth (thinking-off sends `--effort low`). On → the tier's rung; a stale
 *  out-of-range tier falls back to Medium. */
export function dialIdxFor(stops: DialStop[], thinkingOn: boolean, effort: ThinkingEffort): number {
  if (!thinkingOn || effort === "none") return 0;
  const i = stops.findIndex((s) => s.effort === effort);
  if (i >= 0) return i;
  const med = stops.findIndex((s) => s.effort === "smart");
  return med >= 0 ? med : 0;
}

/** Clamp an index into a stops array (pure half of setEffortByIdx). Only the
 *  length matters, so it accepts any stop list. */
export function clampEffortIdx(stops: readonly unknown[], i: number): number {
  const max = Math.max(0, stops.length - 1);
  return Math.min(max, Math.max(0, i));
}

// ── Unified settings-panel rows ──────────────────────────────────────────────
// ONE row list built identically by Composer (which owns keyboard nav over it)
// and SettingsMenu (which renders it) so cursor index and rendered order can
// never disagree: Claude model rows · effort.
export type SettingsRow =
  | { kind: "model"; model: ModelOpt }
  | { kind: "effort" };

export function settingsRowsFor(opts: { dialApplies: boolean; models?: readonly ModelOpt[] }): SettingsRow[] {
  const rows: SettingsRow[] = (opts.models ?? MODEL_OPTIONS).map((m) => ({ kind: "model" as const, model: m }));
  if (opts.dialApplies) rows.push({ kind: "effort" });
  return rows;
}

// Permission-mode picker options — order matches the VS Code Claude Code menu:
// ask → auto-edit → plan → auto → bypass. Icons echo that menu.
// `label` = full descriptive label (menu rows). `short` = the compact bar-pill
// label (design-system `PERM_SHORT`) so the composer bar stays terse — e.g.
// "Bypass" not "Bypass permissions". Keep both in sync per mode.
export type ModeOpt = { id: PermissionMode; label: string; short: string; icon: typeof Hand; hint: string };
export const MODE_OPTIONS: ModeOpt[] = [
  { id: "default",           label: "Ask before edits",   short: "Ask first", icon: Hand,          hint: "Ask before edits — approve each change before it's made" },
  { id: "acceptEdits",       label: "Edit automatically", short: "Auto-edit", icon: Code2,         hint: "Edit automatically — apply file edits without asking" },
  { id: "plan",              label: "Plan mode",          short: "Plan",      icon: ClipboardList, hint: "Plan mode — explore and present a plan; you approve it before anything is built" },
  { id: "auto",              label: "Auto mode",          short: "Auto",      icon: Zap,           hint: "Auto mode — pick the best permission mode per task" },
  { id: "bypassPermissions", label: "Bypass permissions", short: "Bypass",    icon: InfinityIcon,  hint: "Bypass permissions — never ask before running anything" },
];

// Per-mode semantic tone for the spec `.pop-item.tone-*` tinting + the composer
// bar's perm-button tone (ok = safe automation, warn = guardrails off, info =
// non-editing). Single source so the menu, the bar pill, and Composer agree.
export type PermTone = "ok" | "warn" | "info" | "";
export function permToneFor(id: PermissionMode): PermTone {
  if (id === "acceptEdits" || id === "auto") return "ok";
  if (id === "bypassPermissions") return "warn";
  if (id === "plan") return "info";
  return "";
}
