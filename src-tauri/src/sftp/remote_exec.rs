use super::*;
use std::time::Duration;

/// Captured output of an interactive remote command invocation.
/// `truncated` is true when either stream hit `MAX_EXEC_OUTPUT_BYTES`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub truncated: bool,
}

const MAX_EXEC_OUTPUT_BYTES: usize = 256 * 1024;

impl SftpClient {
    /// Execute an arbitrary command over the live russh session; drain stdout +
    /// stderr to bounded buffers, capture exit status, and bail with a typed
    /// error on timeout. Wraps the same `channel_open_session → exec → drain`
    /// pattern as `get_remote_sha1`, but exposed for remote-Bash use.
    pub async fn exec_bash(
        &self,
        command: &str,
        timeout: Duration,
    ) -> Result<ExecOutput, String> {
        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|e| format!("open channel: {e}"))?;
        channel
            .exec(true, command)
            .await
            .map_err(|e| format!("exec: {e}"))?;
        let mut chan = channel;

        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let mut exit_code: Option<i32> = None;
        let mut truncated = false;

        let drain = async {
            // Drain to channel close, NOT to ExitStatus — SSH does not order
            // ExitStatus after final Data. Breaking early can truncate output.
            while let Some(msg) = chan.wait().await {
                match msg {
                    russh::ChannelMsg::Data { data } => {
                        if out.len() + data.len() > MAX_EXEC_OUTPUT_BYTES {
                            let room = MAX_EXEC_OUTPUT_BYTES.saturating_sub(out.len());
                            out.extend_from_slice(&data[..room]);
                            truncated = true;
                        } else {
                            out.extend_from_slice(&data);
                        }
                    }
                    russh::ChannelMsg::ExtendedData { data, ext: 1 } => {
                        if err.len() + data.len() > MAX_EXEC_OUTPUT_BYTES {
                            let room = MAX_EXEC_OUTPUT_BYTES.saturating_sub(err.len());
                            err.extend_from_slice(&data[..room]);
                            truncated = true;
                        } else {
                            err.extend_from_slice(&data);
                        }
                    }
                    russh::ChannelMsg::ExitStatus { exit_status } => {
                        exit_code = Some(exit_status as i32);
                    }
                    _ => {}
                }
            }
        };

        match tokio::time::timeout(timeout, drain).await {
            Ok(()) => {}
            Err(_) => {
                // #88: send EOF before dropping the channel so the remote
                // shell receives an orderly close signal. Without this, a
                // timed-out `find` / long-running script keeps consuming
                // server CPU until the session itself goes away (could be
                // tens of minutes for long-lived auto-sync sessions).
                let _ = chan.eof().await;
                return Err(format!(
                    "remote command timed out after {}s",
                    timeout.as_secs()
                ));
            }
        }

        Ok(ExecOutput {
            stdout: String::from_utf8_lossy(&out).into_owned(),
            stderr: String::from_utf8_lossy(&err).into_owned(),
            exit_code,
            truncated,
        })
    }
}

impl SftpClient {
    pub async fn heal_owned_dirs(&self, root: &str) {
        let Ok(quoted) = shell_quote(root) else { return };
        // -prune-ignored is too much shell quoting hassle here; the heal only
        // chmods dirs we own, and skipping node_modules/etc is a perf nicety
        // not a correctness need. A typical FXserver resource tree is small.
        let cmd = format!(
            "find {quoted} -type d -user \"$(id -un)\" -exec chmod 2775 {{}} + 2>/dev/null"
        );
        let Ok(channel) = self.handle.channel_open_session().await else { return };
        if channel.exec(true, cmd).await.is_err() {
            return;
        }
        let mut chan = channel;
        // #82: drain to channel close, NOT to ExitStatus. SSH doesn't order
        // ExitStatus after final Data; breaking on ExitStatus can miss trailing
        // frames. Same fix pattern as `list_via_exec` / `get_remote_sha1` /
        // `exec_bash` — heal_owned_dirs was missed in the v0.2.44 sweep. No
        // body is needed: we don't consume stdout, just wait for close.
        while chan.wait().await.is_some() {}
    }


    pub async fn get_remote_sha1(&self, path: &str) -> Option<String> {
        // sha1sum prints "<hex>  <filename>\n". We extract the hex prefix.
        // Quote the path to handle spaces / special chars.
        let quoted = shell_quote(path).ok()?;
        let cmd = format!("sha1sum {quoted}");
        let channel = self.handle.channel_open_session().await.ok()?;
        channel.exec(true, cmd).await.ok()?;
        let mut out = String::new();
        let mut err = String::new();
        let mut chan = channel;
        // Drain to channel close, NOT to ExitStatus — SSH does not order
        // ExitStatus after final Data. Breaking early can truncate the hash.
        // See `list_via_exec` comment for full v0.2.44 post-mortem.
        while let Some(msg) = chan.wait().await {
            match msg {
                russh::ChannelMsg::Data { data } => {
                    out.push_str(&String::from_utf8_lossy(&data));
                }
                russh::ChannelMsg::ExtendedData { data, ext: 1 } => {
                    err.push_str(&String::from_utf8_lossy(&data));
                }
                russh::ChannelMsg::ExitStatus { .. } => {}
                _ => {}
            }
        }
        let hex = out.split_whitespace().next();
        let result = hex.filter(|h| h.len() == 40).map(|h| h.to_uppercase());
        if result.is_none() && !err.trim().is_empty() {
            log::debug!("sha1sum {path}: stderr = {}", err.trim());
        }
        result
    }}




pub(super) fn shell_quote(s: &str) -> Result<String, String> {
    // #90: also reject TAB. `find -printf '%p\t%s\t%T@\n'` uses tab as the
    // field separator; a remote filename containing a literal tab would
    // smuggle extra columns and break the downstream `splitn(3, '\t')`
    // parser → file silently dropped as `skipped_bad_size`.
    if s.contains(['\0', '\n', '\r', '\t']) {
        return Err("path contains command-breaking control character".into());
    }
    let mut out = String::from("'");
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    Ok(out)
}

