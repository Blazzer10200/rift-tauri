// Live SFTP integration tests against a real SSH/SFTP server.
//
// These are #[ignore]d — they need a reachable SFTP target and are skipped by a
// plain `cargo test`. Run them explicitly against the Proxmox test LXC (121
// `rift-sftp-test`; see scripts/sftp-test-target.sh + docs/design/
// proxmox-sftp-test-target.md):
//
//   eval $(bash scripts/sftp-test-target.sh env)   # exports RIFT_TEST_SFTP_*
//   cargo test --manifest-path src-tauri/Cargo.toml --ignored sftp_it -- --nocapture
//
// NOTE (Windows): the `env` helper prints a Git-Bash key path (/c/AI Workflow/…)
// which Rust's std::fs CANNOT open. Override with a Windows-form path before
// running, e.g.:
//   export RIFT_TEST_SFTP_KEY="C:/AI Workflow/.secrets/rift-sftp-test"
//
// Env contract (read at test time):
//   RIFT_TEST_SFTP_HOST   required — DHCP IP from `sftp-test-target.sh ip`
//   RIFT_TEST_SFTP_PORT   optional — defaults 22
//   RIFT_TEST_SFTP_USER   optional — defaults "rift"
//   RIFT_TEST_SFTP_KEY    required — path to the OpenSSH private key
//
// If HOST/KEY are unset (or the connect fails), each test prints a SKIP line and
// passes — so `--ignored` on a machine without the target doesn't red-fail.
//
// Safety: every test writes only under a unique per-run scratch dir
// (/home/<user>/.rift-it/<pid>-<nanos>/) and best-effort-deletes it, so the
// seeded `baseline` snapshot is never mutated and concurrent runs don't collide.
// The list test only READS the seeded /home/<user>/fxserver tree.

use super::*;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Read the RIFT_TEST_SFTP_* env contract. `None` → caller should skip.
fn env_target() -> Option<(String, u16, String, PathBuf)> {
    let host = std::env::var("RIFT_TEST_SFTP_HOST").ok().filter(|s| !s.is_empty())?;
    let key = std::env::var("RIFT_TEST_SFTP_KEY").ok().filter(|s| !s.is_empty())?;
    let port = std::env::var("RIFT_TEST_SFTP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(22);
    let user = std::env::var("RIFT_TEST_SFTP_USER")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "rift".to_string());
    Some((host, port, user, PathBuf::from(key)))
}

/// Connect to the test target, or `None` to skip (env unset or connect failed).
/// Returns the live client, the resolved remote home, and a unique scratch dir
/// under it that the caller owns for the duration of the test.
async fn connect_scratch() -> Option<(SftpClient, String, String)> {
    let (host, port, user, key) = match env_target() {
        Some(t) => t,
        None => {
            eprintln!(
                "SKIP: RIFT_TEST_SFTP_HOST/_KEY unset — run via `eval $(bash scripts/sftp-test-target.sh env)`"
            );
            return None;
        }
    };
    let args = ConnectArgs {
        host: &host,
        port,
        user: &user,
        key_path: &key,
        trusted_fingerprint: None,
        write_probe_root: None,
    };
    let client = match SftpClient::connect(args).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("SKIP: connect to {user}@{host}:{port} failed: {e}");
            return None;
        }
    };
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let home = format!("/home/{user}");
    let scratch = format!("{home}/.rift-it/{}-{nanos}", std::process::id());
    Some((client, home, scratch))
}

/// Fresh, unique local temp dir for staging upload sources / download targets.
fn local_tmp_dir() -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let d = std::env::temp_dir().join(format!("rift-it-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(&d).expect("create local tmp dir");
    d
}

/// Upload → stat → download → byte-compare. The size assertion also guards the
/// SETSTAT-0664 path in `upload_atomic_via`: `FileAttributes::default()` (the
/// v0.2.26 data-loss bug) would truncate the file to 0 bytes server-side, which
/// the size + roundtrip-bytes checks here would catch. Binary payload incl. NULs
/// and high bytes proves the transfer is byte-exact, not text-mangled.
#[tokio::test]
#[ignore]
async fn sftp_it_upload_download_roundtrip() {
    let Some((client, _home, scratch)) = connect_scratch().await else {
        return;
    };
    let ld = local_tmp_dir();
    let payload: &[u8] = b"rift integration roundtrip\n\x00\x01\x02 binary-safe \xff\xfe end";
    let src = ld.join("src.bin");
    std::fs::write(&src, payload).unwrap();
    let remote = format!("{scratch}/roundtrip.bin");

    let up = client.upload_file_atomic(&src, &remote).await;
    assert!(up.success, "upload failed: {}", up.error);

    assert!(client.remote_exists(&remote).await, "remote missing after upload");
    let stat = client.remote_stat(&remote).await;
    assert_eq!(stat.size, payload.len() as i64, "remote size mismatch (SETSTAT truncation?)");
    assert!(!stat.is_directory, "uploaded file reported as directory");

    // Atomic upload must leave no `.rift-tmp` sidecar behind.
    assert!(
        !client.remote_exists(&format!("{remote}.rift-tmp")).await,
        "atomic upload left a .rift-tmp sidecar"
    );

    let dst = ld.join("dst.bin");
    let down = client.download_file_atomic(&remote, &dst).await;
    assert!(down.success, "download failed: {}", down.error);
    assert_eq!(std::fs::read(&dst).unwrap(), payload, "roundtrip bytes differ");

    let _ = client.delete(&scratch).await;
    let _ = std::fs::remove_dir_all(&ld);
    client.close().await;
}

/// Re-uploading to an existing path must atomically replace it — no stale tail
/// from the longer prior version, exact new size. v2 is deliberately SHORTER
/// than v1 so a non-truncating overwrite would leave trailing garbage.
#[tokio::test]
#[ignore]
async fn sftp_it_atomic_overwrite_replaces_content() {
    let Some((client, _home, scratch)) = connect_scratch().await else {
        return;
    };
    let ld = local_tmp_dir();
    let remote = format!("{scratch}/overwrite.txt");

    let v1 = ld.join("v1");
    std::fs::write(&v1, b"VERSION-ONE-longer").unwrap(); // 18 bytes
    let v2 = ld.join("v2");
    std::fs::write(&v2, b"V2-short").unwrap(); // 8 bytes

    assert!(client.upload_file_atomic(&v1, &remote).await.success, "v1 upload");
    assert!(client.upload_file_atomic(&v2, &remote).await.success, "v2 upload");

    let dst = ld.join("got");
    assert!(client.download_file_atomic(&remote, &dst).await.success, "download");
    assert_eq!(std::fs::read(&dst).unwrap(), b"V2-short", "atomic replace left stale content");
    assert_eq!(client.remote_stat(&remote).await.size, 8, "size not truncated to v2");

    let _ = client.delete(&scratch).await;
    let _ = std::fs::remove_dir_all(&ld);
    client.close().await;
}

/// `get_remote_sha1` over a known payload — sha1("hello") is a fixed constant,
/// so no hashing dep is needed. Also asserts a missing path returns `None`.
#[tokio::test]
#[ignore]
async fn sftp_it_remote_sha1_matches_known() {
    let Some((client, _home, scratch)) = connect_scratch().await else {
        return;
    };
    let ld = local_tmp_dir();
    let remote = format!("{scratch}/hello.txt");
    let src = ld.join("hello");
    std::fs::write(&src, b"hello").unwrap();
    assert!(client.upload_file_atomic(&src, &remote).await.success, "upload");

    // sha1("hello") = aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d; method upcases it.
    assert_eq!(
        client.get_remote_sha1(&remote).await.as_deref(),
        Some("AAF4C61DDCC5E8A2DABEDE0F3B482CD9AEA9434D"),
        "remote sha1 mismatch"
    );
    assert_eq!(
        client.get_remote_sha1(&format!("{scratch}/missing")).await,
        None,
        "sha1 of missing file should be None"
    );

    let _ = client.delete(&scratch).await;
    let _ = std::fs::remove_dir_all(&ld);
    client.close().await;
}

/// `mkdir_p_strict` creates a nested path; `delete` on a directory removes it
/// recursively (and leaves nothing behind).
#[tokio::test]
#[ignore]
async fn sftp_it_mkdir_strict_and_recursive_delete() {
    let Some((client, _home, scratch)) = connect_scratch().await else {
        return;
    };
    let nested = format!("{scratch}/a/b/c");
    client.mkdir_p_strict(&nested).await.expect("mkdir_p_strict nested");
    assert!(client.remote_exists(&nested).await, "nested dir missing after mkdir");
    assert!(client.remote_stat(&nested).await.is_directory, "leaf not a directory");

    let del = client.delete(&scratch).await; // dir → recursive
    assert!(del.success, "recursive delete failed: {}", del.error);
    assert!(!client.remote_exists(&scratch).await, "scratch survived recursive delete");
    client.close().await;
}

/// Deleting a path that doesn't exist must report success — otherwise the drift
/// reconciler re-queues the delete forever (the `!info.exists → ok()` guard in
/// `ops::delete`).
#[tokio::test]
#[ignore]
async fn sftp_it_delete_missing_is_ok() {
    let Some((client, _home, scratch)) = connect_scratch().await else {
        return;
    };
    let del = client.delete(&format!("{scratch}/never-existed")).await;
    assert!(del.success, "delete of missing path should be ok: {}", del.error);
    client.close().await;
}

/// Worker-pool fan-out, both directions: upload N files via `upload_files_batch`
/// (parallelism 4 → up to 4 independent SSH worker sessions), verify each landed
/// byte-exact, then pull them all back via `download_files_batch`. Exercises
/// `ensure_workers` + the mpsc job-dispatch the serial atomic paths don't —
/// the v13.55.1-class regressions (a worker breaking mid-walk, dropped jobs)
/// live here.
#[tokio::test]
#[ignore]
async fn sftp_it_batch_upload_download_fanout() {
    let Some((client, _home, scratch)) = connect_scratch().await else {
        return;
    };
    let ld = local_tmp_dir();
    const N: usize = 6;

    // Stage N local sources w/ distinct content; build the upload job list.
    let mut up_jobs: Vec<(PathBuf, String)> = Vec::with_capacity(N);
    let mut payloads: Vec<Vec<u8>> = Vec::with_capacity(N);
    for i in 0..N {
        let body = format!("batch-file-{i}-{}", "x".repeat(i * 13)).into_bytes();
        let lp = ld.join(format!("up_{i}.bin"));
        std::fs::write(&lp, &body).unwrap();
        up_jobs.push((lp, format!("{scratch}/sub{i}/file_{i}.bin")));
        payloads.push(body);
    }

    let up_res = client.upload_files_batch(&up_jobs, 4).await;
    assert_eq!(up_res.len(), N, "upload result count");
    assert!(up_res.iter().all(|&ok| ok), "a batch upload failed: {up_res:?}");
    for (i, (_, remote)) in up_jobs.iter().enumerate() {
        let stat = client.remote_stat(remote).await;
        assert!(stat.exists, "batch-uploaded {remote} missing");
        assert_eq!(stat.size, payloads[i].len() as i64, "batch upload {remote} size mismatch");
    }

    // Pull them all back into fresh local paths and byte-compare.
    let ct = tokio_util::sync::CancellationToken::new();
    let dl_jobs: Vec<(String, PathBuf)> = up_jobs
        .iter()
        .enumerate()
        .map(|(i, (_, remote))| (remote.clone(), ld.join(format!("dl_{i}.bin"))))
        .collect();
    let dl_res = client.download_files_batch(&dl_jobs, 4, ct).await;
    assert!(dl_res.iter().all(|&ok| ok), "a batch download failed: {dl_res:?}");
    for (i, (_, local)) in dl_jobs.iter().enumerate() {
        assert_eq!(std::fs::read(local).unwrap(), payloads[i], "batch roundtrip {i} bytes differ");
    }

    let _ = client.delete(&scratch).await;
    let _ = std::fs::remove_dir_all(&ld);
    client.close().await;
}

/// `list_recursive_batch` over the seeded read-only fxserver tree returns a
/// non-empty walk. Reads only — never mutates the seed.
#[tokio::test]
#[ignore]
async fn sftp_it_list_recursive_seeded_tree() {
    let Some((client, home, _scratch)) = connect_scratch().await else {
        return;
    };
    let root = format!("{home}/fxserver");
    if !client.remote_exists(&root).await {
        eprintln!("SKIP: seed tree {root} absent (run `sftp-test-target.sh reset`)");
        client.close().await;
        return;
    }
    let res = client
        .list_recursive_batch(&[root.clone()], 5, None, 2)
        .await
        .expect("list_recursive_batch");
    let total: usize = res.values().map(|v| v.len()).sum();
    assert!(total > 0, "seed tree {root} listed empty");
    eprintln!("listed {total} entries under {root}");
    client.close().await;
}
