/**
 * Turn a raw caught error (Tauri `invoke` rejection, Rust error chain, JS
 * Error) into a short, user-readable line for a toast `detail` or an inline
 * error slot. Strips developer noise (ANSI, "Error: " / "tauri::" wrappers),
 * scrubs the OS username from any leaked path, and maps a few common failure
 * shapes to plain-English guidance. Falls back to the cleaned raw text so we
 * never swallow a genuinely-novel error — honesty over a generic "Something
 * went wrong".
 */
import { scrubUser } from "./redact";

// Ordered: first matching pattern wins. Keep these conservative — only map a
// shape we're confident about; everything else passes through cleaned.
const KNOWN: Array<{ re: RegExp; msg: string }> = [
  { re: /\b(timed?\s*out|timeout|deadline\s*exceeded)\b/i,
    msg: "The request timed out — check your connection and try again." },
  { re: /\b(certificate|tls|ssl|self.signed|cert)\b/i,
    msg: "A network security check failed — a corporate proxy or firewall may be intercepting the connection." },
  { re: /\b(dns|getaddrinfo|name.not.resolved|failed to lookup)\b/i,
    msg: "Couldn't reach the server — check your internet connection." },
  { re: /\b(connection (refused|reset)|econnrefused|econnreset|network is unreachable|offline)\b/i,
    msg: "Couldn't connect — the network may be down or blocked." },
  { re: /\b(403|forbidden|401|unauthorized)\b/i,
    msg: "Access was denied — you may need to sign in again." },
  { re: /\b(permission denied|access is denied|eacces|os error 5)\b/i,
    msg: "Access was denied — a file or folder couldn't be written." },
  { re: /\b(being used by another process|locked|sharing violation|os error 32)\b/i,
    msg: "A file is in use by another program — close it and try again." },
  { re: /\b(no space|enospc|disk full)\b/i,
    msg: "Your disk is out of space." },
];

export function humanizeError(e: unknown): string {
  const raw = stringifyErr(e);
  if (!raw) return "An unexpected error occurred.";
  // strip ANSI color codes + common Rust/Tauri/JS wrapper prefixes
  let s = raw
    .replace(/\x1b\[[0-9;]*m/g, "")
    .replace(/^\s*(error|tauri|invoke|caused by)\s*[:>-]+\s*/i, "")
    .trim();
  s = scrubUser(s);
  for (const { re, msg } of KNOWN) if (re.test(s)) return msg;
  // No known shape — return the cleaned raw text, bounded so a giant chain
  // doesn't blow out the toast.
  return s.length > 240 ? s.slice(0, 240) + "…" : s;
}

function stringifyErr(e: unknown): string {
  if (e == null) return "";
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message || String(e);
  if (typeof e === "object") {
    const o = e as Record<string, unknown>;
    if (typeof o.message === "string") return o.message;
    try { return JSON.stringify(e); } catch { return String(e); }
  }
  return String(e);
}
