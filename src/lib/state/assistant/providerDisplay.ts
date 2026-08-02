/**
 * Human-facing AI naming. Keep provider implementation ids out of product copy:
 * `openai` is the native API route; people choose ChatGPT.
 */
export const CHATGPT = {
  label: "ChatGPT",
  apiAccess: "ChatGPT API access",
  apiKey: "ChatGPT API key",
  apiSetup: "Connect ChatGPT API",
  apiBilling: "ChatGPT API access is billed separately from a ChatGPT subscription.",
} as const;

export function modelProviderLabel(model: string | null | undefined): "Claude" | "ChatGPT" | "Local model" {
  if (!model) return "Local model";
  return /^gpt-/i.test(model) ? CHATGPT.label : /^claude/i.test(model) ? "Claude" : "Local model";
}
