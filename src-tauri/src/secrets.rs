//! OS-keychain wrapper for at-rest secrets.
//!
//! Stores the Anthropic `api_key` outside `~/.rift/*.json`. Backends per the
//! `keyring` crate: Windows Credential Manager, macOS Keychain, Linux Secret
//! Service.

use keyring::Entry;

const SERVICE: &str = "rift";

/// Read a secret. Returns None if the entry doesn't exist, is empty, or the
/// backend is unavailable (e.g. headless Linux with no Secret Service). Errors
/// are logged at debug — callers treat absence as "not configured".
pub fn get(key: &str) -> Option<String> {
    match Entry::new(SERVICE, key) {
        Ok(entry) => match entry.get_password() {
            Ok(v) if !v.is_empty() => Some(v),
            Ok(_) => None,
            Err(keyring::Error::NoEntry) => None,
            Err(e) => {
                log::debug!("secrets::get({key}) backend error: {e}");
                None
            }
        },
        Err(e) => {
            log::debug!("secrets::get({key}) entry-new error: {e}");
            None
        }
    }
}

pub fn set(key: &str, value: &str) -> Result<(), String> {
    // `get()` treats an empty stored value as absent; storing one would be a
    // write that silently reads back as None. Reject it — callers that mean
    // "clear this secret" must call `delete()`.
    if value.is_empty() {
        return Err(format!("refusing to store empty value for {key} (use delete to clear)"));
    }
    let entry = Entry::new(SERVICE, key).map_err(|e| format!("keyring entry {key}: {e}"))?;
    entry
        .set_password(value)
        .map_err(|e| format!("keyring set {key}: {e}"))
}

pub fn delete(key: &str) -> Result<(), String> {
    let entry = match Entry::new(SERVICE, key) {
        Ok(e) => e,
        Err(e) => return Err(format!("keyring entry {key}: {e}")),
    };
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("keyring delete {key}: {e}")),
    }
}

/// Single-tenant key for the Anthropic API key.
pub const ASSISTANT_API_KEY: &str = "assistant.api_key";

/// Key for the experimental local-LLM endpoint (LiteLLM/Ollama). Most local
/// proxies accept any non-empty value; stored separately so it never shadows
/// the cloud `ASSISTANT_API_KEY`.
pub const LOCAL_LLM_API_KEY: &str = "local_llm.api_key";

#[cfg(test)]
mod tests {
    use super::*;

    // The empty-value guard returns before touching the OS keychain, so this is
    // hermetic — no platform credential backend required.
    #[test]
    fn set_rejects_empty_value() {
        let err = set("some.key", "").unwrap_err();
        assert!(err.contains("refusing to store empty value"), "got: {err}");
        assert!(err.contains("delete"), "should point callers at delete(): {err}");
    }
}
