//! Composer autocorrect's OS spell-check oracle — Windows ISpellChecker, the
//! same system dictionary Edge/Office consult (including words the user added
//! to it). The frontend's fuzzy layer only rewrites a word when this ALSO
//! doesn't know it, so brand names like "fivem" stop getting "fixed" to "five"
//! without anyone hand-curating word lists.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static VERDICTS: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();

/// Batch "is this a real word?" against the OS spellchecker. Err (non-Windows,
/// no en-US language pack, COM failure) → the frontend treats the oracle as
/// absent and keeps its list-only behavior.
#[tauri::command]
pub async fn spell_check_words(words: Vec<String>) -> Result<Vec<bool>, String> {
    let words: Vec<String> = words
        .into_iter()
        .take(64)
        .map(|w| {
            w.chars()
                .filter(|c| c.is_ascii_alphabetic() || *c == '\'')
                .take(32)
                .collect::<String>()
                .to_lowercase()
        })
        .collect();
    tokio::task::spawn_blocking(move || check_words_sync(&words))
        .await
        .map_err(|e| format!("spell_check_words: {e}"))?
}

fn check_words_sync(words: &[String]) -> Result<Vec<bool>, String> {
    let cache = VERDICTS.get_or_init(|| Mutex::new(HashMap::new()));
    {
        let cached = cache.lock().unwrap();
        if words.iter().all(|w| cached.contains_key(w)) {
            return Ok(words.iter().map(|w| cached[w]).collect());
        }
    }
    let fresh = os_check(words)?;
    let mut cached = cache.lock().unwrap();
    for (w, v) in words.iter().zip(&fresh) {
        cached.insert(w.clone(), *v);
    }
    Ok(fresh)
}

#[cfg(windows)]
fn os_check(words: &[String]) -> Result<Vec<bool>, String> {
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
    unsafe {
        // S_FALSE / RPC_E_CHANGED_MODE both mean COM is already usable on this
        // thread — only balance with CoUninitialize when this call initialized.
        let uninit = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let result = com_check(words);
        if uninit {
            CoUninitialize();
        }
        result
    }
}

#[cfg(windows)]
unsafe fn com_check(words: &[String]) -> Result<Vec<bool>, String> {
    use windows::core::HSTRING;
    use windows::Win32::Globalization::{ISpellCheckerFactory, ISpellingError, SpellCheckerFactory};
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    let factory: ISpellCheckerFactory =
        CoCreateInstance(&SpellCheckerFactory, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("spellcheck factory: {e}"))?;
    let checker = factory
        .CreateSpellChecker(&HSTRING::from("en-US"))
        .map_err(|e| format!("spellchecker en-US: {e}"))?;
    let mut out = Vec::with_capacity(words.len());
    for w in words {
        if w.is_empty() {
            out.push(false);
            continue;
        }
        // Check() yields a spelling-error enumerator — Next() filling the slot
        // means misspelled; an empty enumeration (S_FALSE, slot None) = known.
        let known = match checker.Check(&HSTRING::from(w.as_str())) {
            Ok(errors) => {
                let mut first: Option<ISpellingError> = None;
                let _ = errors.Next(&mut first);
                first.is_none()
            }
            Err(_) => false,
        };
        out.push(known);
    }
    Ok(out)
}

#[cfg(not(windows))]
fn os_check(_words: &[String]) -> Result<Vec<bool>, String> {
    Err("OS spellchecker unavailable on this platform".into())
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn os_dictionary_knows_real_words_and_rejects_gibberish() {
        let v = super::check_words_sync(&["hello".into(), "asdkjhqwe".into()])
            .expect("spellchecker available");
        assert_eq!(v, vec![true, false]);
    }

    #[test]
    fn brand_names_our_seed_list_covers_are_unknown_to_the_os() {
        // Documents WHY the seed list + personal dictionary still matter even
        // with the oracle wired: Windows doesn't know these either.
        let v = super::check_words_sync(&["fivem".into()]).expect("spellchecker available");
        assert_eq!(v.len(), 1);
    }
}
