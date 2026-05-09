// Phase 1j — port of Rift Project/Services/Transport/SshKeygen.cs.
//
// WPF version shells out to `ssh-keygen.exe`. Rust version generates the
// keypair in-process via `ssh-key` (re-exported through `russh::keys`) — no
// PATH dependency, no Process spawn, no timeout race. Output is OpenSSH
// format on disk, identical to what `ssh-keygen -t ed25519` writes (minus
// the embedded comment in the binary header, which is set explicitly).

use russh::keys::ssh_key::{Algorithm, LineEnding, PrivateKey};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::state::paths::dirs_home;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPaths {
    pub private_path: String,
    pub public_path: String,
    pub public_key_text: String,
}

pub struct SshKeygen;

impl SshKeygen {
    /// User's default `~/.ssh/id_ed25519` path. Mirrors `SshKeygen.DefaultKeyPath`.
    pub fn default_key_path() -> Option<PathBuf> {
        dirs_home().ok().map(|h| h.join(".ssh").join("id_ed25519"))
    }

    pub fn default_pub_key_path() -> Option<PathBuf> {
        Self::default_key_path().map(|p| p.with_extension("pub"))
    }

    pub fn default_key_exists() -> bool {
        Self::default_key_path()
            .map(|p| p.is_file())
            .unwrap_or(false)
    }

    /// Read the existing default `~/.ssh/id_ed25519.pub` if present. Trimmed.
    pub fn read_default_pub_key() -> Option<String> {
        let p = Self::default_pub_key_path()?;
        std::fs::read_to_string(&p).ok().map(|s| s.trim().to_string())
    }

    /// Generate an ed25519 keypair at `target_dir/<filename>` (private) +
    /// `.pub` (public). `comment` lands in the public key. Refuses to
    /// overwrite an existing private key. Caller is responsible for chmod —
    /// Windows ignores it; on POSIX systems the user can `chmod 600` after.
    pub fn generate(
        target_dir: &Path,
        filename: &str,
        comment: &str,
    ) -> Result<KeyPaths, String> {
        if filename.is_empty() {
            return Err("filename empty".into());
        }
        std::fs::create_dir_all(target_dir)
            .map_err(|e| format!("mkdir {}: {e}", target_dir.display()))?;

        let private_path = target_dir.join(filename);
        let public_path = private_path.with_extension("pub");
        if private_path.exists() {
            return Err(format!("key already exists: {}", private_path.display()));
        }

        let mut rng = rand::rngs::OsRng;
        let mut key = PrivateKey::random(&mut rng, Algorithm::Ed25519)
            .map_err(|e| format!("ed25519 generate: {e}"))?;
        key.set_comment(comment);

        let pem = key
            .to_openssh(LineEnding::LF)
            .map_err(|e| format!("private to_openssh: {e}"))?;
        std::fs::write(&private_path, pem.as_bytes())
            .map_err(|e| format!("write {}: {e}", private_path.display()))?;

        let pub_text = key
            .public_key()
            .to_openssh()
            .map_err(|e| format!("public to_openssh: {e}"))?;
        // Trailing newline matches `ssh-keygen` output.
        let pub_with_newline = format!("{pub_text}\n");
        std::fs::write(&public_path, pub_with_newline.as_bytes())
            .map_err(|e| format!("write {}: {e}", public_path.display()))?;

        Ok(KeyPaths {
            private_path: private_path.to_string_lossy().to_string(),
            public_path: public_path.to_string_lossy().to_string(),
            public_key_text: pub_text,
        })
    }

    /// Generate the user's default `~/.ssh/id_ed25519` keypair w/ a sensible
    /// comment. Mirrors WPF's parameterless `SshKeygen.Generate()`.
    pub fn generate_default(comment: Option<&str>) -> Result<KeyPaths, String> {
        let path = Self::default_key_path().ok_or("no home dir")?;
        let dir = path.parent().ok_or("no parent dir for ssh key")?;
        let fallback = format!(
            "rift-{}@{}",
            std::env::var("USERNAME").unwrap_or_else(|_| "user".into()),
            std::env::var("COMPUTERNAME").unwrap_or_else(|_| "host".into()),
        );
        let comment = comment.unwrap_or(&fallback);
        Self::generate(dir, "id_ed25519", comment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_ed25519_keypair_to_temp_dir() {
        let tmp = std::env::temp_dir().join(format!("rift-keygen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let result = SshKeygen::generate(&tmp, "id_test", "rift-test@host")
            .expect("generate");
        let priv_text =
            std::fs::read_to_string(&result.private_path).expect("read priv");
        let pub_text = std::fs::read_to_string(&result.public_path).expect("read pub");
        assert!(priv_text.contains("OPENSSH PRIVATE KEY"));
        assert!(pub_text.starts_with("ssh-ed25519 "));
        assert!(pub_text.contains("rift-test@host"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn refuses_overwrite() {
        let tmp = std::env::temp_dir().join(format!("rift-keygen-no-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        SshKeygen::generate(&tmp, "id_test", "c").expect("first");
        let err = SshKeygen::generate(&tmp, "id_test", "c").unwrap_err();
        assert!(err.contains("already exists"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
