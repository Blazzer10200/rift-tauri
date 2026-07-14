//! Administrator elevation (Windows).
//!
//! Rift normally runs as a standard user. Some workspaces need admin for the
//! tools the assistant runs (service control, HKLM, `%ProgramFiles%` writes),
//! and Windows would otherwise pop a UAC prompt for *every* such action
//! (`Start-Process -Verb RunAs`). Elevation is inherited by child processes, so
//! once **Rift itself** is elevated the whole tree — Rift → the `claude` CLI →
//! its Bash/PowerShell tools — runs elevated with no per-action prompt. This is
//! exactly the "launch VS Code as administrator" experience.
//!
//! Two layers, both opt-in:
//!   1. **Relaunch as administrator** — one UAC prompt, elevated for the session
//!      (`relaunch_as_admin`). The safe default.
//!   2. **Always run elevated (no prompts)** — a per-user on-demand Scheduled
//!      Task with `HighestAvailable` run level. Task Scheduler is a trusted
//!      elevation broker, so `schtasks /Run` launches Rift elevated with NO UAC
//!      prompt. Rift self-triggers it at startup when the preference is on (see
//!      `bootstrap`). It's a legitimate "run my own app as admin without a
//!      prompt" pattern — deliberately opt-in and one-click reversible.
//!
//! Non-Windows targets get honest no-ops (`is_elevated()` → false, task ops
//! error) — elevation is a Windows concept and Rift ships Windows-only today.

/// Scheduled-task name for the always-elevated launcher. Stable — the bootstrap
/// and the enable/disable commands all key off it.
pub const TASK_NAME: &str = "RiftElevatedLauncher";

/// What `bootstrap()` decided the caller (lib.rs `run()`) should do next.
#[derive(Debug, PartialEq, Eq)]
pub enum Boot {
    /// Keep booting this process normally.
    Continue,
    /// This process handed off to an elevated instance (or is redundant) — exit
    /// cleanly WITHOUT building Tauri.
    Exit,
}

/// True when the current process holds an elevated (admin) token.
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::os::raw::c_void;
    #[repr(C)]
    struct TokenElevation {
        token_is_elevated: u32,
    }
    // winnt.h: TOKEN_QUERY = 0x0008; TOKEN_INFORMATION_CLASS::TokenElevation = 20.
    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_ELEVATION: i32 = 20;
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn OpenProcessToken(process: *mut c_void, desired: u32, token: *mut *mut c_void) -> i32;
        fn GetTokenInformation(
            token: *mut c_void,
            class: i32,
            info: *mut c_void,
            len: u32,
            ret_len: *mut u32,
        ) -> i32;
        fn CloseHandle(h: *mut c_void) -> i32;
    }
    unsafe {
        let mut token: *mut c_void = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation = TokenElevation { token_is_elevated: 0 };
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION,
            &mut elevation as *mut _ as *mut c_void,
            std::mem::size_of::<TokenElevation>() as u32,
            &mut ret_len,
        );
        CloseHandle(token);
        ok != 0 && elevation.token_is_elevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

/// Launch a fresh, elevated copy of Rift via the `runas` verb (one UAC prompt)
/// and return `Ok` once it's launched — the caller then exits so only the
/// elevated instance remains. `Err` if the user declines UAC or the spawn fails
/// (caller keeps running non-elevated).
///
/// Uses PowerShell `Start-Process -Verb RunAs` — the same elevation primitive
/// already used elsewhere in Rift — which handles path quoting and returns
/// non-zero when the user cancels the prompt. Window hidden via CREATE_NO_WINDOW.
#[cfg(windows)]
pub fn relaunch_as_admin() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    // Single-quote for PowerShell string; escape embedded single quotes by
    // doubling. The path is our own exe path, never user input, but escape anyway.
    let exe_ps = exe.to_string_lossy().replace('\'', "''");
    let script = format!(
        "try {{ Start-Process -FilePath '{exe_ps}' -Verb RunAs -ErrorAction Stop; exit 0 }} \
         catch {{ exit 1 }}"
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("spawn powershell for elevation: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        // Non-zero = the user clicked "No" on UAC, or elevation is policy-blocked.
        Err("Elevation was cancelled or denied.".to_string())
    }
}

#[cfg(not(windows))]
pub fn relaunch_as_admin() -> Result<(), String> {
    Err("Administrator elevation is only supported on Windows.".to_string())
}

/// XML-escape the five predefined entities so an exe path with `&`/`<`/`"` (rare
/// but possible in a username) can't break the task definition.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Task Scheduler XML for the always-elevated launcher: on-demand only (no
/// triggers), `HighestAvailable` run level, no execution time limit (`PT0S` —
/// otherwise the scheduler would kill Rift after its default 72h). `command`
/// is the fully-resolved exe path.
#[cfg(windows)]
fn task_xml(command: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Launches Rift with administrator privileges without a UAC prompt. Created by Rift; remove it anytime from Rift Settings.</Description>
    <URI>\{task}</URI>
  </RegistrationInfo>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>false</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{command}</Command>
    </Exec>
  </Actions>
</Task>"#,
        task = TASK_NAME,
        command = xml_escape(command),
    )
}

/// Run `schtasks` with the given args, hidden, capturing status + output. Returns
/// (success, combined stdout+stderr) so callers can surface a real error.
#[cfg(windows)]
fn schtasks(args: &[&str]) -> Result<(bool, String), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let out = std::process::Command::new("schtasks")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("spawn schtasks: {e}"))?;
    let mut msg = String::from_utf8_lossy(&out.stdout).into_owned();
    msg.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok((out.status.success(), msg.trim().to_string()))
}

/// True when the always-elevated launcher task is registered.
#[cfg(windows)]
pub fn task_exists() -> bool {
    schtasks(&["/Query", "/TN", TASK_NAME])
        .map(|(ok, _)| ok)
        .unwrap_or(false)
}

#[cfg(not(windows))]
pub fn task_exists() -> bool {
    false
}

/// Register the always-elevated launcher task, pointed at the current exe.
/// Requires elevation on most systems (setting `HighestAvailable` run level) —
/// callers arrange to be elevated first. The XML is written UTF-16LE + BOM,
/// which `schtasks /XML` expects.
#[cfg(windows)]
pub fn create_task() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let xml = task_xml(&exe.to_string_lossy());

    // schtasks reads the XML from a file. UTF-16LE + BOM to match the
    // `encoding="UTF-16"` declaration — the historically safe encoding for
    // `schtasks /XML` across Windows versions.
    let mut bytes: Vec<u8> = vec![0xFF, 0xFE];
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let path = std::env::temp_dir().join(format!("{TASK_NAME}.xml"));
    std::fs::write(&path, &bytes).map_err(|e| format!("write task xml: {e}"))?;

    let path_str = path.to_string_lossy().into_owned();
    let res = schtasks(&["/Create", "/TN", TASK_NAME, "/XML", &path_str, "/F"]);
    let _ = std::fs::remove_file(&path);
    match res? {
        (true, _) => Ok(()),
        (false, msg) => Err(if msg.is_empty() {
            "Could not create the elevated launcher task (administrator rights required).".into()
        } else {
            msg
        }),
    }
}

#[cfg(not(windows))]
pub fn create_task() -> Result<(), String> {
    Err("Administrator elevation is only supported on Windows.".to_string())
}

/// Remove the always-elevated launcher task. Idempotent — a missing task is
/// treated as success so disabling always converges.
#[cfg(windows)]
pub fn delete_task() -> Result<(), String> {
    if !task_exists() {
        return Ok(());
    }
    match schtasks(&["/Delete", "/TN", TASK_NAME, "/F"])? {
        (true, _) => Ok(()),
        (false, msg) => Err(if msg.is_empty() {
            "Could not remove the elevated launcher task.".into()
        } else {
            msg
        }),
    }
}

#[cfg(not(windows))]
pub fn delete_task() -> Result<(), String> {
    Ok(())
}

/// Trigger the always-elevated launcher task — launches an elevated Rift with no
/// UAC prompt. Returns `Ok` once triggered; the caller then exits.
#[cfg(windows)]
fn run_task() -> Result<(), String> {
    match schtasks(&["/Run", "/TN", TASK_NAME])? {
        (true, _) => Ok(()),
        (false, msg) => Err(if msg.is_empty() {
            "Could not launch the elevated launcher task.".into()
        } else {
            msg
        }),
    }
}

/// Startup elevation reconciliation, called once from `run()` after Velopack and
/// after the MCP-server early return. Reads the `always_elevated` preference and
/// converges the process to the desired state:
///
/// | pref | elevated? | task?  | action                                   |
/// |------|-----------|--------|------------------------------------------|
/// | off  | —         | —      | Continue (normal launch)                 |
/// | on   | yes       | no     | create the task, Continue (reconcile)    |
/// | on   | yes       | yes    | Continue (already the elevated instance) |
/// | on   | no        | yes    | run the task → Exit (prompt-free elevate)|
/// | on   | no        | no     | relaunch as admin → Exit (one-time UAC)  |
///
/// Loop-safe: the elevated instance always reads `is_elevated() == true` and so
/// never re-triggers. Any failure to elevate (declined UAC, broken task) falls
/// through to `Continue` so the app still opens — non-elevated, honestly.
#[cfg(windows)]
pub fn bootstrap() -> Boot {
    // Dev builds NEVER self-promote: the launcher task points at the INSTALLED
    // exe, so a debug binary honoring `always_elevated` hands the session to the
    // prod app and kills its own `tauri dev` tree (observed 2026-07-14 — the
    // de-elevated CDP dev launch collapsed into an elevated prod instance).
    // Elevation is an installed-app feature; dev runs stay at the launch IL.
    if cfg!(debug_assertions) {
        log::info!("elevation: dev build — skipping always-elevated bootstrap");
        return Boot::Continue;
    }
    if !crate::assistant::get_always_elevated() {
        return Boot::Continue;
    }
    if is_elevated() {
        // We're the elevated instance. Make sure the prompt-free launcher exists
        // for next time (it may not yet, if the user just enabled the pref and we
        // got here via a one-time UAC relaunch).
        if !task_exists() {
            if let Err(e) = create_task() {
                log::warn!("elevation: could not create launcher task while elevated: {e}");
            } else {
                log::info!("elevation: always-elevated launcher task created");
            }
        }
        return Boot::Continue;
    }
    // Preference is on but we're not elevated — hand off to an elevated instance.
    if task_exists() {
        match run_task() {
            Ok(()) => {
                log::info!("elevation: launched elevated instance via scheduled task");
                return Boot::Exit;
            }
            Err(e) => {
                log::warn!("elevation: scheduled-task launch failed ({e}); continuing non-elevated");
                return Boot::Continue;
            }
        }
    }
    // Pref on, not elevated, no task yet — one-time UAC relaunch so the elevated
    // instance can register the task (see the `is_elevated` branch above).
    match relaunch_as_admin() {
        Ok(()) => {
            log::info!("elevation: relaunching as administrator to finish enabling always-elevated");
            Boot::Exit
        }
        Err(e) => {
            log::warn!("elevation: one-time elevation relaunch failed ({e}); continuing non-elevated");
            Boot::Continue
        }
    }
}

#[cfg(not(windows))]
pub fn bootstrap() -> Boot {
    Boot::Continue
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn xml_escapes_predefined_entities() {
        let out = xml_escape(r#"C:\Users\a&b<c>"d"'\rift.exe"#);
        assert!(out.contains("a&amp;b"));
        assert!(out.contains("&lt;c&gt;"));
        assert!(out.contains("&quot;d&quot;"));
        assert!(out.contains("&apos;"));
        assert!(!out.contains('<') && !out.contains('>'));
    }

    #[test]
    fn task_xml_is_on_demand_highest_and_unbounded() {
        let xml = task_xml(r"C:\Program Files\Rift\rift-tauri.exe");
        // On-demand: no <Triggers> block.
        assert!(!xml.contains("<Triggers>"));
        // Elevated run level.
        assert!(xml.contains("<RunLevel>HighestAvailable</RunLevel>"));
        // No execution time limit — the scheduler must never kill a live Rift.
        assert!(xml.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
        // Command carries the exe path.
        assert!(xml.contains(r"C:\Program Files\Rift\rift-tauri.exe"));
        // Task URI matches the stable name the bootstrap keys off.
        assert!(xml.contains(&format!(">\\{TASK_NAME}<")));
    }
}
