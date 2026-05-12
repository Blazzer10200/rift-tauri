use super::*;

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
        while let Some(msg) = chan.wait().await {
            if let russh::ChannelMsg::ExitStatus { .. } = msg {
                break;
            }
        }
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
    if s.contains(['\0', '\n', '\r']) {
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

