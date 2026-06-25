//! Corporate TLS root extraction for Windows proxy environments.
//! Reads the Windows Trusted Root + CA stores once at startup via
//! `rustls-native-certs`, writes a concatenated PEM to
//! `%LOCALAPPDATA%\Rift\certs\corporate-roots.pem`, and exposes
//! two shared `reqwest::Client` singletons (usage + download timeouts)
//! plus the PEM path for NODE_EXTRA_CA_CERTS injection.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

// --- PEM path singleton ---

static CORP_PEM: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Returns the path to the written corporate-roots PEM, or `None` when the
/// Windows cert store contained zero certs (stripped VM) or write failed.
/// Calling this multiple times is free — extraction runs exactly once.
pub fn corp_pem_path() -> Option<&'static PathBuf> {
    CORP_PEM.get_or_init(extract_corporate_roots).as_ref()
}

fn extract_corporate_roots() -> Option<PathBuf> {
    let result = rustls_native_certs::load_native_certs();
    for err in &result.errors {
        log::warn!("corp-certs: skipped one cert: {err}");
    }
    if result.certs.is_empty() {
        log::info!("corp-certs: no certs from Windows store — NODE_EXTRA_CA_CERTS will not be set");
        return None;
    }
    // Encode each DER cert as PEM.
    use base64::Engine as _;
    let mut pem = String::with_capacity(result.certs.len() * 1024);
    for cert in &result.certs {
        pem.push_str("-----BEGIN CERTIFICATE-----\n");
        pem.push_str(&base64::engine::general_purpose::STANDARD.encode(cert.as_ref()));
        pem.push_str("\n-----END CERTIFICATE-----\n");
    }
    // Write to %LOCALAPPDATA%\Rift\certs\corporate-roots.pem.
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)?
        .join("Rift")
        .join("certs");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("corporate-roots.pem");
    std::fs::write(&path, pem.as_bytes()).ok()?;
    log::info!(
        "corp-certs: wrote {} root(s) to {}",
        result.certs.len(),
        path.display()
    );
    Some(path)
}

// --- Shared reqwest clients ---

static USAGE_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static DOWNLOAD_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Shared client for usage/limits.rs (15 s timeout).
pub fn usage_client() -> &'static reqwest::Client {
    USAGE_CLIENT.get_or_init(|| build_client(Duration::from_secs(15), None))
}

/// Shared client for stt/model_manager.rs (30 s connect, 600 s body).
pub fn download_client() -> &'static reqwest::Client {
    DOWNLOAD_CLIENT.get_or_init(|| {
        build_client(Duration::from_secs(600), Some(Duration::from_secs(30)))
    })
}

fn build_client(timeout: Duration, connect_timeout: Option<Duration>) -> reqwest::Client {
    let result = rustls_native_certs::load_native_certs();
    // Ignore partial errors — already logged by extract_corporate_roots if that
    // ran first; this call is independent (reqwest DER path, no PEM needed).
    let mut b = reqwest::Client::builder().timeout(timeout);
    if let Some(ct) = connect_timeout {
        b = b.connect_timeout(ct);
    }
    for cert in result.certs {
        // from_der accepts the raw DER bytes directly — no PEM round-trip.
        if let Ok(c) = reqwest::tls::Certificate::from_der(cert.as_ref()) {
            b = b.add_root_certificate(c);
        }
    }
    b.build().expect("certs::build_client: reqwest ClientBuilder failed")
}
