//! FXServer (FiveM) RCON over UDP — legacy Quake-style out-of-band protocol.
//!
//! Request : `\xff\xff\xff\xff` `rcon <password> <command>`
//! Response: one or more datagrams, each `\xff\xff\xff\xff` `print\n<chunk>`.
//! Long output is split across datagrams by the server, so we keep reading
//! until a recv times out, then concatenate. The first recv timing out with no
//! data is a hard error (wrong host/port/password, or the server has no
//! `rcon_password` convar set).
//!
//! Plaintext UDP — that is how FXServer RCON works; the password rides in every
//! packet. Callers MUST source it from the OS keychain, never the IPC boundary.

use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::timeout;

const OOB: &[u8] = b"\xff\xff\xff\xff";
const PRINT: &[u8] = b"print\n";
const FIRST_TIMEOUT: Duration = Duration::from_secs(5);
const TRAIL_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_RESPONSE: usize = 256 * 1024;

/// Send one RCON command to `host:port` and return the server's printed
/// response (OOB + `print\n` headers stripped, trailing whitespace trimmed).
pub async fn send(host: &str, port: u16, password: &str, command: &str) -> Result<String, String> {
    if password.is_empty() {
        return Err("no rcon password set".into());
    }
    let sock = UdpSocket::bind(("0.0.0.0", 0))
        .await
        .map_err(|e| format!("udp bind: {e}"))?;
    sock.connect((host, port))
        .await
        .map_err(|e| format!("connect {host}:{port}: {e}"))?;

    let mut packet = Vec::with_capacity(8 + password.len() + command.len());
    packet.extend_from_slice(OOB);
    packet.extend_from_slice(b"rcon ");
    packet.extend_from_slice(password.as_bytes());
    packet.push(b' ');
    packet.extend_from_slice(command.as_bytes());
    sock.send(&packet).await.map_err(|e| format!("send: {e}"))?;

    let mut out = String::new();
    let mut buf = [0u8; 8192];
    let mut first = true;
    loop {
        let to = if first { FIRST_TIMEOUT } else { TRAIL_TIMEOUT };
        match timeout(to, sock.recv(&mut buf)).await {
            Ok(Ok(n)) => {
                out.push_str(&String::from_utf8_lossy(strip_headers(&buf[..n])));
                first = false;
                if out.len() >= MAX_RESPONSE {
                    break;
                }
            }
            Ok(Err(e)) => return Err(format!("recv: {e}")),
            Err(_) => {
                if first {
                    return Err(
                        "no response — check host/port and that rcon_password is set server-side"
                            .into(),
                    );
                }
                break;
            }
        }
    }
    Ok(out.trim_end().to_string())
}

/// Strip the `\xff\xff\xff\xff` out-of-band marker and any `print\n` prefix
/// from a single response datagram.
fn strip_headers(data: &[u8]) -> &[u8] {
    let d = data.strip_prefix(OOB).unwrap_or(data);
    d.strip_prefix(PRINT).unwrap_or(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_oob_and_print() {
        let mut pkt = Vec::new();
        pkt.extend_from_slice(OOB);
        pkt.extend_from_slice(PRINT);
        pkt.extend_from_slice(b"hello world");
        assert_eq!(strip_headers(&pkt), b"hello world");
    }

    #[test]
    fn strips_oob_without_print() {
        let mut pkt = Vec::new();
        pkt.extend_from_slice(OOB);
        pkt.extend_from_slice(b"raw");
        assert_eq!(strip_headers(&pkt), b"raw");
    }

    #[test]
    fn passthrough_when_no_headers() {
        assert_eq!(strip_headers(b"plain"), b"plain");
    }
}
