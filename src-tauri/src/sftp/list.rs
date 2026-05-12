use super::*;
use std::sync::Arc;
use std::time::Duration;

// v0.2.50: list-op ceiling. Large remote trees over a wedged-but-not-dead
// link could hang the worker indefinitely (russh keepalive detects truly-
// dead sessions but not stalled ones). 120 s gives generous headroom for
// honest big-tree scans over WAN while still surfacing a stall as a
// recognizable error the engine can fail+reconnect on. Server-side
// `find -printf` listings are sub-second on healthy links (v0.2.28), so
// 120 s is ~100× the healthy baseline.
const LIST_T: u64 = 120;

impl SftpClient {
    pub async fn list_recursive(
        &self,
        root: &str,
        max_depth: usize,
        ext_filter: Option<&[&str]>,
    ) -> Result<Vec<RemoteEntry>, String> {
        let owned_filter: Option<Vec<String>> =
            ext_filter.map(|f| f.iter().map(|s| s.to_string()).collect());
        list_recursive_via(&self.sftp, root, max_depth, owned_filter.as_deref()).await
    }


    pub async fn list_recursive_batch(
        &self,
        roots: &[String],
        max_depth: usize,
        ext_filter: Option<&[&str]>,
        parallelism: usize,
    ) -> Result<std::collections::HashMap<String, Vec<RemoteEntry>>, String> {
        let owned_filter: Option<Vec<String>> =
            ext_filter.map(|f| f.iter().map(|s| s.to_string()).collect());

        if roots.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        // FAST PATH: server-side `find` over an SSH exec channel.
        //
        // Recursive SFTP listing is O(N) round-trips per directory (open-dir,
        // read-dir, close-dir, plus a stat per entry). On high-latency links
        // (Tailscale-tunneled WAN, ~30-80ms RTT) a moderate tree takes 30-60s.
        // `find <root> -printf '%p\t%s\t%T@\n'` returns the whole tree in a
        // single round-trip with streamed output — sub-second on the same link.
        //
        // We dispatch one exec channel per root in parallel (cheap — exec
        // channels are lightweight compared to full SFTP sessions). Any root
        // whose exec fails (no `find` on PATH, non-POSIX shell, perms) falls
        // through to the SFTP worker path below, so behavior degrades safely.
        let mut out: std::collections::HashMap<String, Vec<RemoteEntry>> =
            std::collections::HashMap::new();
        {
            // russh's `Handle` is not Clone (it's an internal channel-sender
            // wrapper held by the connection task), so we can't `tokio::spawn`
            // tasks that own a copy. `join_all` over async blocks that *borrow*
            // `&self.handle` gives us parallelism w/o the Clone constraint —
            // each block opens its own exec channel via the shared Handle,
            // which is the supported concurrency model in russh.
            let futs = roots.iter().map(|r| {
                let root = r.clone();
                let filter = owned_filter.clone();
                let handle_ref = &self.handle;
                async move {
                    let res = match tokio::time::timeout(
                        Duration::from_secs(LIST_T),
                        list_via_exec(handle_ref, &root, max_depth, filter.as_deref()),
                    ).await {
                        Ok(r) => r,
                        Err(_) => Err(format!(
                            "list-exec {root}: timeout after {LIST_T}s — connection wedged"
                        )),
                    };
                    (root, res)
                }
            });
            let results = futures::future::join_all(futs).await;
            for (root, res) in results {
                if let Ok(v) = res {
                    out.insert(root, v);
                }
            }
            // If every root succeeded via exec, skip the SFTP worker spin-up.
            if out.len() == roots.len() {
                return Ok(out);
            }
        }

        self.ensure_workers(parallelism).await;
        let live = self.live_worker_count().await;

        // Only process roots the exec fast path didn't already cover.
        let missing: Vec<String> = roots
            .iter()
            .filter(|r| !out.contains_key(*r))
            .cloned()
            .collect();

        if live == 0 {
            // Fallback: serial listing on the main session.
            for r in &missing {
                let v = match tokio::time::timeout(
                    Duration::from_secs(LIST_T),
                    list_recursive_via(&self.sftp, r, max_depth, owned_filter.as_deref()),
                ).await {
                    Ok(Ok(v)) => v,
                    _ => Vec::new(),
                };
                out.insert(r.clone(), v);
            }
        } else {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            for r in &missing {
                let _ = tx.send(r.clone());
            }
            drop(tx);
            let rx = Arc::new(tokio::sync::Mutex::new(rx));

            let workers: Vec<Arc<Worker>> = {
                let g = self.workers.lock().await;
                g.iter().take(live).cloned().collect()
            };
            let filter_arc = Arc::new(owned_filter.clone());
            let results_arc =
                Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::<
                    String,
                    Vec<RemoteEntry>,
                >::new()));

            let mut tasks = Vec::with_capacity(workers.len());
            for worker in workers {
                let rx = rx.clone();
                let results_arc = results_arc.clone();
                let filter = filter_arc.clone();
                tasks.push(tokio::spawn(async move {
                    loop {
                        let job = {
                            let mut rxg = rx.lock().await;
                            rxg.recv().await
                        };
                        let Some(root) = job else { break };
                        let _g = worker.gate.lock().await;
                        let res = tokio::time::timeout(
                            Duration::from_secs(LIST_T),
                            list_recursive_via(
                                &worker.sftp,
                                &root,
                                max_depth,
                                (*filter).as_deref(),
                            ),
                        ).await;
                        match res {
                            Ok(Ok(v)) => {
                                let mut r = results_arc.lock().await;
                                r.insert(root, v);
                            }
                            Ok(Err(e)) => log::warn!("list_recursive worker failed for {root}: {e}"),
                            Err(_) => log::warn!("list_recursive worker timeout ({LIST_T}s) for {root}"),
                        }
                    }
                }));
            }
            let _ = futures::future::join_all(tasks).await;
            // Merge worker results into out (preserving exec-fast-path entries).
            let worker_results = Arc::try_unwrap(results_arc)
                .map(|m| m.into_inner())
                .unwrap_or_default();
            for (k, v) in worker_results {
                out.insert(k, v);
            }

            // Belt-and-braces: retry only roots the worker pool failed to
            // populate (vec absent). Worker can return 0 entries silently if
            // its session breaks mid-walk; main retry confirms before
            // propagating. Genuinely-empty roots (worker returned `Some(vec![])`)
            // do NOT retry — that's a serial round-trip per empty folder.
            for r in &missing {
                if !out.contains_key(r) {
                    if let Ok(retry) = list_recursive_via(
                        &self.sftp,
                        r,
                        max_depth,
                        owned_filter.as_deref(),
                    )
                    .await
                    {
                        out.insert(r.clone(), retry);
                    } else {
                        out.entry(r.clone()).or_default();
                    }
                }
            }
        }

        Ok(out)
    }


    pub async fn list_directory(&self, path: &str) -> Result<Vec<RemoteEntry>, String> {
        let entries = self
            .sftp
            .read_dir(path)
            .await
            .map_err(|e| format!("readdir {path}: {e}"))?;
        let trimmed = path.trim_end_matches('/');
        let mut out: Vec<RemoteEntry> = Vec::new();
        for entry in entries {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let meta = entry.metadata();
            let is_dir = matches!(meta.file_type(), FileType::Dir);
            out.push(RemoteEntry {
                full_path: format!("{}/{}", trimmed, name),
                name,
                is_dir,
                size: meta.size.unwrap_or(0),
                last_modified: mtime_to_utc(meta.mtime),
            });
        }
        out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });
        Ok(out)
    }}




async fn list_via_exec(
    handle: &Handle<PinningHandler>,
    root: &str,
    max_depth: usize,
    ext_filter: Option<&[String]>,
) -> Result<Vec<RemoteEntry>, String> {
    let trimmed_root = trim_slash(root);
    let quoted_root = super::remote_exec::shell_quote(&trimmed_root)?;

    // Prune dirs that match our ignore segments — saves the server from
    // walking node_modules etc., which on busy resource trees can be larger
    // than the actual code. Keep in sync w/ sync::ignore::ignored_directory_names.
    let prune_names = crate::sync::ignore::ignored_directory_names();
    let prune_clause = if prune_names.is_empty() {
        String::new()
    } else {
        let parts: Vec<String> = prune_names
            .iter()
            .filter_map(|n| super::remote_exec::shell_quote(n).ok())
            .map(|q| format!("-name {q}"))
            .collect();
        format!("-type d \\( {} \\) -prune -o", parts.join(" -o "))
    };

    // `%T@` = mtime as seconds-since-epoch (fractional). `%p` = full path.
    // Suppress stderr so transient ENOENT on vanished files doesn't poison
    // stdout parsing.
    let cmd = format!(
        "find {quoted_root} -maxdepth {max_depth} {prune_clause} -type f -printf '%p\\t%s\\t%T@\\n' 2>/dev/null"
    );

    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| format!("open exec channel: {e}"))?;
    channel
        .exec(true, cmd)
        .await
        .map_err(|e| format!("exec find: {e}"))?;
    let mut stdout = String::new();
    let mut exit: Option<u32> = None;
    let mut chan = channel;
    // CRITICAL: do NOT break on ExitStatus. SSH does not guarantee that
    // ExitStatus arrives after all Data messages — for fast `find` runs over
    // deep trees the server can send ExitStatus while trailing Data chunks
    // are still in flight on the channel. Breaking early discards those
    // chunks, producing a non-deterministic short listing that classifies
    // legitimate remote files as ToDelete (the v0.2.44 phantom-delete bug).
    // Drain the channel until it closes (`wait()` returns None) — guaranteed
    // to follow Eof + Close per the SSH-CONNECTION spec.
    while let Some(msg) = chan.wait().await {
        match msg {
            russh::ChannelMsg::Data { data } => {
                stdout.push_str(&String::from_utf8_lossy(&data));
            }
            russh::ChannelMsg::ExitStatus { exit_status } => {
                exit = Some(exit_status);
            }
            _ => {}
        }
    }
    // find returns exit 1 for "some files failed" but still streams the rest;
    // accept 0 and 1, fail anything else so we fall back.
    match exit {
        Some(0) | Some(1) => {}
        Some(c) => return Err(format!("find exit {c}")),
        None => return Err("find: no exit status".into()),
    }

    let filter_lower: Option<Vec<String>> = ext_filter.map(|f| {
        f.iter()
            .map(|e| {
                let s = e.to_ascii_lowercase();
                if s.starts_with('.') { s } else { format!(".{s}") }
            })
            .collect()
    });

    let mut out = Vec::new();
    // v0.2.49 listing-accuracy instrumentation: count raw lines vs emitted
    // entries + capture skip reasons. Item 2 of the foundation pass — the
    // Endure RP 2026-05-12 audit caught 5 files vanishing in this loop on a
    // 61-file remote tree. Buckets help name the dropped lines.
    let mut raw_lines: u32 = 0;
    let mut skipped_short: u32 = 0;
    let mut skipped_bad_size: u32 = 0;
    let mut skipped_by_ext: u32 = 0;
    let mut samples_skipped: Vec<String> = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        raw_lines += 1;
        let mut parts = line.splitn(3, '\t');
        let (Some(path), Some(size_s), Some(mtime_s)) =
            (parts.next(), parts.next(), parts.next())
        else {
            skipped_short += 1;
            if samples_skipped.len() < 5 {
                samples_skipped.push(format!("short:{line}"));
            }
            continue;
        };
        let size: u64 = match size_s.parse() {
            Ok(n) => n,
            Err(_) => {
                skipped_bad_size += 1;
                if samples_skipped.len() < 5 {
                    samples_skipped.push(format!("bad_size:{line}"));
                }
                continue;
            }
        };
        // mtime is fractional seconds. Floor to whole seconds for chrono.
        let mtime_secs: i64 = mtime_s
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if let Some(ref exts) = filter_lower {
            let lower = path.to_ascii_lowercase();
            if !exts.iter().any(|e| lower.ends_with(e)) {
                skipped_by_ext += 1;
                continue;
            }
        }
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        out.push(RemoteEntry {
            full_path: path.to_string(),
            name,
            is_dir: false,
            size,
            last_modified: Utc
                .timestamp_opt(mtime_secs, 0)
                .single()
                .unwrap_or_else(Utc::now),
        });
    }
    let total_skipped = skipped_short + skipped_bad_size + skipped_by_ext;
    if total_skipped > 0 || raw_lines != out.len() as u32 {
        eprintln!(
            "[rift] list_via_exec {root}: raw_lines={raw_lines} emitted={} skipped_short={skipped_short} skipped_bad_size={skipped_bad_size} skipped_by_ext={skipped_by_ext} samples={samples_skipped:?}",
            out.len(),
        );
        crate::diagnostics::emit_with_fields(
            crate::diagnostics::DiagStage::RemoteScanResult,
            if total_skipped > 0 { crate::diagnostics::DiagLevel::Warn } else { crate::diagnostics::DiagLevel::Debug },
            None,
            Some(root),
            format!(
                "list_via_exec parse: raw={raw_lines} emitted={} skipped={total_skipped}",
                out.len(),
            ),
            serde_json::json!({
                "root": root,
                "raw_lines": raw_lines,
                "emitted": out.len(),
                "skipped_short": skipped_short,
                "skipped_bad_size": skipped_bad_size,
                "skipped_by_ext": skipped_by_ext,
                "samples_skipped": samples_skipped,
            }),
        );
    }

    Ok(out)
}


async fn list_recursive_via(
    sftp: &SftpSession,
    root: &str,
    max_depth: usize,
    ext_filter: Option<&[String]>,
) -> Result<Vec<RemoteEntry>, String> {
    let root = trim_slash(root);
    let mut out = Vec::new();
    let mut stack: Vec<(String, usize)> = vec![(root.clone(), 0)];

    while let Some((dir, depth)) = stack.pop() {
        let entries = match sftp.read_dir(&dir).await {
            Ok(e) => e,
            Err(e) => return Err(format!("readdir {dir}: {e}")),
        };
        for entry in entries {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let meta = entry.metadata();
            let full = format!("{}/{}", dir, name);
            let is_dir = matches!(meta.file_type(), FileType::Dir);
            if is_dir {
                if depth < max_depth {
                    stack.push((full, depth + 1));
                }
                continue;
            }
            if let Some(filter) = ext_filter {
                let ok = name
                    .rsplit_once('.')
                    .map(|(_, ext)| filter.iter().any(|f| f.eq_ignore_ascii_case(ext)))
                    .unwrap_or(false);
                if !ok {
                    continue;
                }
            }
            out.push(RemoteEntry {
                full_path: full,
                name,
                is_dir: false,
                size: meta.size.unwrap_or(0),
                last_modified: mtime_to_utc(meta.mtime),
            });
        }
    }
    Ok(out)
}


