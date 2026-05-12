// Shared SSH client `Handler` impl + fingerprint helpers. Used by both the
// SFTP client and the SSH local-port-forward tunnel — same kex, same
// fingerprint pinning rules.
//
// Fingerprint format: OpenSSH-style `SHA256:<base64-no-pad>` of the SSH
// wire-format public key blob — matches `ssh-keygen -lf <pub>` output.
// Profiles may store any of:
//   - bare `SHA256:<b64>`                 (rift-tauri canonical / OpenSSH)
//   - `ssh-ed25519 256 SHA256:<b64>`      (some WinSCP exports)
//   - `ssh-ed25519 255 <b64>`             (WPF Rift `~/.rift/*.json` format —
//                                          NO `SHA256:` prefix)
// Stored and computed values are normalized to the bare b64 hash, then compared
// by exact equality.

use base64::Engine;
use russh::client;
use russh::keys::ssh_key;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

/// Compute OpenSSH-style fingerprint from a public key.
fn pubkey_fingerprint(pk: &ssh_key::PublicKey) -> Option<String> {
    let blob = pk.to_bytes().ok()?;
    let mut h = Sha256::new();
    h.update(&blob);
    let digest = h.finalize();
    let b64 = base64::engine::general_purpose::STANDARD_NO_PAD.encode(digest);
    Some(format!("SHA256:{b64}"))
}

fn bare_fingerprint(fp: &str) -> &str {
    let token = fp.split_whitespace().last().unwrap_or(fp).trim();
    token.strip_prefix("SHA256:").unwrap_or(token)
}

/// SSH client Handler with TOFU pinning. Exact-matches normalized fingerprints
/// while accepting OpenSSH, WinSCP, and legacy WPF storage formats. On accept,
/// captures the live fingerprint into `captured` so first-connect TOFU can
/// persist it to the profile.
pub struct PinningHandler {
    pub trusted: Option<String>,
    pub captured: Arc<Mutex<Option<String>>>,
}

impl client::Handler for PinningHandler {
    type Error = russh::Error;
    async fn check_server_key(&mut self, k: &ssh_key::PublicKey) -> Result<bool, Self::Error> {
        let fp = match pubkey_fingerprint(k) {
            Some(s) => s,
            None => return Ok(false),
        };
        if let Some(trusted) = &self.trusted {
            if bare_fingerprint(trusted) != bare_fingerprint(&fp) {
                return Ok(false);
            }
        }
        if let Ok(mut g) = self.captured.lock() {
            *g = Some(fp);
        }
        Ok(true)
    }
}
