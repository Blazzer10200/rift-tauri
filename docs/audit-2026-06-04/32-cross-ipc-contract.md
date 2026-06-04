# Cross — IPC Contract

_2 confirmed findings._ [← back to index](README.md)

## Cross — IPC Contract

| Severity | Title | Location | Fix-gist |
|---|---|---|---|
| Medium | Settings commands dead on wire — dual-storage split | `src/lib/state/assistant/helpers.ts:13`; `src-tauri/src/lib.rs:147-150`; `src-tauri/src/assistant/mod.rs:1096-1136` | Unify on one layer: route reads/writes through the four IPC commands, or drop them and keep localStorage-only |
| Low | `stt_clean_transcript` registered but never invoked | `src-tauri/src/lib.rs:193`; `src-tauri/src/stt/mod.rs:625` | Wire after `stt_stop_recording` resolves, or remove command + `cleanup` module if feature is abandoned |

### [Medium] Settings commands dead on wire — dual-storage split

`assistant_set_thinking_effort`, `assistant_get_thinking_effort`, `assistant_set_permission_mode`, and `assistant_get_permission_mode` are all registered in `lib.rs:147-150` and fully implemented in `assistant/mod.rs:1096-1136`, persisting to the Rust config file. The frontend never calls any of them. `helpers.ts:13` defines `EFFORT_KEY`/`PERMISSION_KEY` and all reads/writes go through `localStorage.setItem`/`getItem` exclusively (`saveEffort`, `loadEffort`, `savePermissionMode`, `loadPermissionMode`). Per-turn values are forwarded as direct params to `assistant_send`. The two storage systems are completely disconnected: a backend config-file change has zero UI effect, and a localStorage reset (WebView2 profile wipe or reinstall) silently drops user settings with no backend fallback. The four backend commands are dead code on the wire. Recommended fix: on settings load, call `assistant_get_thinking_effort` and `assistant_get_permission_mode` to seed state instead of localStorage reads; on change, call the IPC setters. This makes the Rust config file authoritative and removes the localStorage fallback. Alternatively, remove the four backend commands and stay localStorage-only — simpler, but loses config-file persistence for these two fields.

### [Low] `stt_clean_transcript` registered but never invoked

`stt::stt_clean_transcript` is registered in `lib.rs:193` and implemented at `stt/mod.rs:625`, dispatching to `cleanup::polish`. A full grep of `src/**/*.ts` and `src/**/*.svelte` finds zero `invoke("stt_clean_transcript")` calls, and no aliased wrapper (`cleanTranscript`, `polish`, `clean_transcript`) appears either. The transcript polish path is completely unreachable from the UI; the STT core (start/stop/record) is unaffected. Fix: if the feature is intended, pipe the raw transcript through `invoke<string>("stt_clean_transcript", { text: rawTranscript })` after `stt_stop_recording` resolves before inserting into the composer. If abandoned, remove the command and the `cleanup` module.
