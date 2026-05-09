// Phase 1e: rift_bridge HTTP client. Talks to http://<host>:<bridge_port>/<path>
// (typically 127.0.0.1 over a forwarded SSH tunnel; for now we trust the user
// has the tunnel up — Rust SSH-tunnel forwarder lands later as Phase 1g).
//
// Mirrors WPF Services/Transport/BridgeClient.cs (57L). Bearer token via
// `X-Rift-Token` header. 8s timeout. Currently exposes one endpoint —
// `sync_done` — fired by AutoSync after a successful flush batch.

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct BridgeResult {
    pub success: bool,
    pub status: u16,
    pub body: String,
    pub error: String,
}

pub struct BridgeClient {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl BridgeClient {
    pub fn new(host: &str, port: u16, token: &str) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .build()
            .map_err(|e| format!("reqwest build: {e}"))?;
        Ok(Self {
            http,
            base: format!("http://{host}:{port}"),
            token: token.to_string(),
        })
    }

    /// Local-tunnel default (127.0.0.1).
    pub fn local(port: u16, token: &str) -> Result<Self, String> {
        Self::new("127.0.0.1", port, token)
    }

    pub async fn sync_done(&self, resource_name: &str) -> BridgeResult {
        let path = format!(
            "/sync-done?resource={}",
            urlencoding_minimal(resource_name)
        );
        self.send(reqwest::Method::POST, &path).await
    }

    async fn send(&self, method: reqwest::Method, path: &str) -> BridgeResult {
        let url = format!("{}{}", self.base, path);
        let req = self.http.request(method, &url).header("X-Rift-Token", &self.token);
        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let success = resp.status().is_success();
                let body = resp.text().await.unwrap_or_default();
                BridgeResult { success, status, body, error: String::new() }
            }
            Err(e) => BridgeResult {
                success: false,
                status: 0,
                body: String::new(),
                error: format!("HTTP: {e}"),
            },
        }
    }
}

/// Percent-encoder for query values. Allows only RFC 3986 unreserved chars;
/// everything else (incl. `&`, `=`, `+`, brackets, spaces) gets escaped so a
/// resource name like `evil&token=stolen` can't smuggle extra query params
/// into the bridge request URL.
fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_handles_brackets_and_space() {
        assert_eq!(urlencoding_minimal("[qbx]/qbx_core"), "%5Bqbx%5D%2Fqbx_core");
        assert_eq!(urlencoding_minimal("a b"), "a%20b");
        assert_eq!(urlencoding_minimal("plain"), "plain");
    }

    #[test]
    fn local_url_prefix() {
        let c = BridgeClient::local(30120, "tok").unwrap();
        assert!(c.base.starts_with("http://127.0.0.1:30120"));
    }
}
