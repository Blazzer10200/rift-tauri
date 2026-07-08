//! Windows Job Object — structural guarantee that EVERY child process dies when
//! the main Rift process dies, on ANY exit path (graceful, crash, panic, Task
//! Manager kill, `std::process::exit`, WebView2 renderer crash, power-loss reboot
//! reaping).
//!
//! WHY: Rift spawns a tree — WebView2 renderer processes (its own UI), per-turn
//! `claude` CLI children, and a `rift-tauri.exe` MCP server per claude child. The
//! app reaps these manually in `RunEvent::Exit` + the Velopack apply path, which
//! works on a GRACEFUL close. But any exit that skips that handler orphans the
//! whole tree — WebView2 subprocesses in particular linger in Task Manager
//! burning memory/CPU (the "WebView2 Manager still running after I closed the
//! app" complaint). Manual per-path reaping is fragile: every new exit path has
//! to remember to reap, and a crash/kill runs none of it.
//!
//! A Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` flips this from
//! opt-in to structural: when the LAST handle to the job closes (which happens
//! when our process — the only handle holder — terminates for ANY reason), the
//! OS terminates every process still assigned to the job. Children spawned after
//! assignment inherit the job by default. The existing manual reaps become a
//! fast, clean-shutdown optimization rather than the only safety net.
//!
//! No new crate dependency — raw `extern "system"` FFI, matching the style of
//! `center_in_work_area` in lib.rs.
//!
//! NOT called in MCP-server mode (`RIFT_MCP_SERVER=1`): those are the CHILDREN.
//! Putting a child in its own kill-on-close job is harmless but pointless, and
//! we want them governed by the MAIN process's job, which they inherit.

#[cfg(target_os = "windows")]
pub fn assign_current_process_to_kill_on_close_job() {
    use std::ffi::c_void;

    type Handle = *mut c_void;
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: u32 = 9; // JobObjectExtendedLimitInformation
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;

    // Layout must match the Win32 structs exactly (packed as the ABI expects).
    #[repr(C)]
    #[derive(Default)]
    struct IoCounters {
        read_op_count: u64,
        write_op_count: u64,
        other_op_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }
    #[repr(C)]
    #[derive(Default)]
    struct BasicLimitInformation {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct ExtendedLimitInformation {
        basic_limit_information: BasicLimitInformation,
        io_info: IoCounters,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    extern "system" {
        fn CreateJobObjectW(security: *mut c_void, name: *const u16) -> Handle;
        fn SetInformationJobObject(
            job: Handle,
            class: u32,
            info: *mut c_void,
            len: u32,
        ) -> i32;
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> i32;
        fn GetCurrentProcess() -> Handle;
    }

    unsafe {
        let job = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
        if job.is_null() {
            log::warn!("job_object: CreateJobObjectW failed — children may orphan on abnormal exit");
            return;
        }

        let mut info = ExtendedLimitInformation::default();
        info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ok = SetInformationJobObject(
            job,
            JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
            &mut info as *mut _ as *mut c_void,
            std::mem::size_of::<ExtendedLimitInformation>() as u32,
        );
        if ok == 0 {
            log::warn!("job_object: SetInformationJobObject failed — kill-on-close not armed");
            return;
        }

        // Assign OURSELVES. Children spawned afterward inherit the job by default
        // (unless they create a breakaway job — WebView2 and the CLI don't).
        // NOTE: intentionally leak the job handle for the life of the process —
        // the job must stay open exactly as long as we run; the OS closes the
        // last handle (ours) at process death, which triggers the kill. Do NOT
        // CloseHandle(job) here or the guarantee evaporates immediately.
        let assigned = AssignProcessToJobObject(job, GetCurrentProcess());
        if assigned == 0 {
            // Common benign case: the process is ALREADY in a job that disallows
            // nesting/breakaway (e.g. launched under some sandboxes / older
            // Windows). Not fatal — the manual RunEvent::Exit reap still runs.
            log::warn!(
                "job_object: AssignProcessToJobObject failed (already in a restrictive job?) — \
                 falling back to manual exit reap only"
            );
            return;
        }
        log::info!("job_object: armed KILL_ON_JOB_CLOSE — all children reap with the host on any exit");
    }
}

#[cfg(not(target_os = "windows"))]
pub fn assign_current_process_to_kill_on_close_job() {
    // POSIX: children are handled by kill_on_drop + the RunEvent::Exit reap.
    // (A process-group / prctl(PR_SET_PDEATHSIG) equivalent could go here if a
    // Linux/macOS build ever ships; Rift is Windows-only today.)
}
