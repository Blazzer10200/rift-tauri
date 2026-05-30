// Mockable SFTP surface for the sync layer.
//
// `SftpClient` is a live russh/russh-sftp connection — impossible to construct
// in a unit test without a real SSH server. `SftpOps` abstracts the exact set
// of methods the sync layer (`DriftScanner`, and later `flush_batch`) calls
// against it, so tests can inject a fake (see `drift_scanner::tests::MockSftp`)
// and exercise the diff/flush logic offline.
//
// The trait mirrors the inherent `SftpClient` methods 1:1; the impl just
// delegates. Delegation relies on Rust's method-resolution precedence —
// `self.remote_exists(path)` inside the impl binds to the INHERENT method (tried
// before trait methods for `receiver.method()` syntax), so there is no
// recursion back into the trait. Call sites that hold a concrete `SftpClient`
// keep hitting the inherent methods; only code that takes `&dyn SftpOps` routes
// through the trait.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use super::{OpResult, RemoteEntry, RemoteFileInfo, SftpClient};

#[async_trait]
pub trait SftpOps: Send + Sync {
    async fn remote_exists(&self, path: &str) -> bool;
    async fn remote_stat(&self, path: &str) -> RemoteFileInfo;
    async fn delete(&self, path: &str) -> OpResult;
    async fn mkdir_p_strict(&self, path: &str) -> Result<(), String>;
    async fn get_remote_sha1(&self, path: &str) -> Option<String>;
    async fn upload_file_atomic(&self, local_path: &Path, remote_path: &str) -> OpResult;
    async fn download_file_atomic(&self, remote_path: &str, local_path: &Path) -> OpResult;
    async fn list_recursive_batch(
        &self,
        roots: &[String],
        max_depth: usize,
        ext_filter: Option<&[&str]>,
        parallelism: usize,
    ) -> Result<HashMap<String, Vec<RemoteEntry>>, String>;
}

#[async_trait]
impl SftpOps for SftpClient {
    async fn remote_exists(&self, path: &str) -> bool {
        self.remote_exists(path).await
    }
    async fn remote_stat(&self, path: &str) -> RemoteFileInfo {
        self.remote_stat(path).await
    }
    async fn delete(&self, path: &str) -> OpResult {
        self.delete(path).await
    }
    async fn mkdir_p_strict(&self, path: &str) -> Result<(), String> {
        self.mkdir_p_strict(path).await
    }
    async fn get_remote_sha1(&self, path: &str) -> Option<String> {
        self.get_remote_sha1(path).await
    }
    async fn upload_file_atomic(&self, local_path: &Path, remote_path: &str) -> OpResult {
        self.upload_file_atomic(local_path, remote_path).await
    }
    async fn download_file_atomic(&self, remote_path: &str, local_path: &Path) -> OpResult {
        self.download_file_atomic(remote_path, local_path).await
    }
    async fn list_recursive_batch(
        &self,
        roots: &[String],
        max_depth: usize,
        ext_filter: Option<&[&str]>,
        parallelism: usize,
    ) -> Result<HashMap<String, Vec<RemoteEntry>>, String> {
        self.list_recursive_batch(roots, max_depth, ext_filter, parallelism)
            .await
    }
}
