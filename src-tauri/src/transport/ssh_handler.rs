// Shared SSH client `Handler` impl + fingerprint helpers. Used by both the
// SFTP client and the SSH local-port-forward tunnel — same kex, same
// fingerprint pinning rules. Lives here to avoid duplicating the substring-
// match TOFU dance in two modules.
//
// Fingerprint format: OpenSSH-style `SHA256:<base64-no-pad>` of the SSH
// wire-format public key blob — matches `ssh-keygen -lf <pub>` output.
// Profiles may store any of:
//   - bare `SHA256:<b64>`                 (rift-tauri canonical / OpenSSH)
//   - `ssh-ed25519 256 SHA256:<b64>`      (some WinSCP exports)
//   - `ssh-ed25519 255 <b64>`             (WPF Rift `~/.rift/*.json` format —
//                                          NO `SHA256:` prefix)
// The substring match strips `SHA256:` from the computed fingerprint and
// looks for the bare b64 hash, accepting all three forms transparently.

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

/// SSH client Handler with TOFU pinning. Substring-matches `trusted` against
/// the computed fingerprint so callers can store either form. On accept,
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
            let bare = fp.strip_prefix("SHA256:").unwrap_or(&fp);
            if !trusted.contains(bare) {
                return Ok(false);
            }
        }
        if let Ok(mut g) = self.captured.lock() {
            *g = Some(fp);
        }
        Ok(true)
    }
}
