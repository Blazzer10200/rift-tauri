/**
 * Path redaction — replaces `C:\Users\<name>\…` / `/home/<name>/…` / `/Users/<name>/…`
 * with `<user>` so paths can be pasted into support diagnostics, error reports,
 * or shared screenshots without leaking the OS username.
 *
 * #8: lifted out of Settings.svelte 2026-05-26 so any frontend-side log
 * forwarding or path-surfacing surface can reuse the same regex. Rust-side
 * scrubbing already shipped via #238.
 */
export function scrubUser(p: string | null | undefined): string {
  if (!p) return p ?? "";
  return p
    .replace(/([A-Za-z]:[\\/]Users[\\/])[^\\/]+/g, "$1<user>")
    .replace(/(\/home\/)[^/]+/g, "$1<user>")
    .replace(/(\/Users\/)[^/]+/g, "$1<user>");
}
