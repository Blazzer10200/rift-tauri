// Pure parser for `mcp__rift__ask_user` tool input — shared by AskUserCard,
// StreamAskUser, and the composer answer hand-off. Mirrors the backend's
// normalize_questions (mcp_bridge.rs): the model doesn't always follow the
// schema, and a sloppy shape must degrade to a usable card, not an empty one
// (2026-07-28: a questions-as-JSON-string call with plain-string options
// rendered only "Other" with dead buttons).
export type AskQuestion = {
  question: string;
  header: string;
  multiSelect?: boolean;
  options: Array<{ label: string; description?: string }>;
};

export function parseAskQuestions(input: Record<string, unknown> | null | undefined): AskQuestion[] {
  let raw: unknown = input?.questions;
  if (typeof raw === "string") {
    try {
      raw = JSON.parse(raw);
    } catch {
      raw = null;
    }
  }
  if (raw == null && input && ("question" in input || "options" in input)) raw = [input];
  if (raw && typeof raw === "object" && !Array.isArray(raw)) raw = [raw];
  if (!Array.isArray(raw)) return [];
  const out: AskQuestion[] = [];
  for (const item of raw as unknown[]) {
    if (item == null || typeof item !== "object") continue;
    const q = item as Record<string, unknown>;
    const question = typeof q.question === "string" ? q.question.trim() : "";
    const options: AskQuestion["options"] = [];
    if (Array.isArray(q.options)) {
      for (const o of q.options as unknown[]) {
        if (typeof o === "string") {
          if (o.trim()) options.push({ label: o.trim() });
        } else if (o && typeof o === "object") {
          const oo = o as Record<string, unknown>;
          const label = typeof oo.label === "string" ? oo.label.trim() : "";
          if (!label) continue;
          options.push({
            label,
            description: typeof oo.description === "string" ? oo.description : undefined,
          });
        }
      }
    }
    // A question with no usable options still renders (Other + composer answer
    // paths cover it) — only fully-empty entries drop.
    if (!question && options.length === 0) continue;
    out.push({
      question,
      header: typeof q.header === "string" ? q.header : "",
      multiSelect: q.multiSelect === true,
      options,
    });
  }
  return out;
}
