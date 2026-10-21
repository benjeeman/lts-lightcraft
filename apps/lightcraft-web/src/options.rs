//! The page's URL options (`?bench&store=idb&workers=2&reset&embedded&demo`) and the wording the page
//! itself shows (notices, the blocking messages, the panic overlay), which follows the
//! presentation the host selected. Pure, so it is tested natively; the wasm start-up (`web.rs`,
//! `safety.rs`) parses `location.search` once and reads the result.

use lightcraft_ui_egui::presentation::{EMBEDDED_NAME, Presentation};

/// URL options (`?bench&store=idb&workers=2&reset&embedded&demo`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub bench: bool,
    /// "opfs" (default: OPFS, falling back to IndexedDB), "idb" or "memory".
    pub store: String,
    pub workers: Option<usize>,
    pub reset: bool,
    /// `embedded`: the LTS host's presentation, decided by the exact name's presence (any value).
    /// It changes no storage, worker or other option, and grants nothing.
    pub embedded: bool,
    /// `demo`: an embedded page seeds the demo photos into a new library too (owner decision OC-5:
    /// otherwise its new library starts empty). Read like `embedded`; it changes nothing else.
    pub demo: bool,
}

impl Options {
    /// Parse `location.search` (the leading `?` is optional); unknown keys are ignored.
    pub fn parse(search: &str) -> Options {
        let params: Vec<(String, String)> = search
            .trim_start_matches('?')
            .split('&')
            .filter(|p| !p.is_empty())
            .map(|p| match p.split_once('=') {
                Some((k, v)) => (k.to_string(), v.to_string()),
                None => (p.to_string(), String::new()),
            })
            .collect();
        let get = |k: &str| params.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        Options {
            bench: get("bench").is_some(),
            store: get("store").filter(|s| s == "idb" || s == "memory").unwrap_or_else(|| "opfs".into()),
            workers: get("workers").and_then(|v| v.parse().ok()),
            reset: get("reset").is_some(),
            embedded: has_name(search, "embedded"),
            demo: has_name(search, "demo"),
        }
    }

    /// Whether a new library starts with the procedural demo photos: always in the standalone
    /// page; embedded only with `demo` (the LTS host's library starts empty: its first import is
    /// photo 1).
    pub fn seed_demo(&self) -> bool {
        !self.embedded || self.demo
    }

    /// The presentation the page was opened with.
    pub fn presentation(&self) -> Presentation {
        if self.embedded { Presentation::LtsEmbedded } else { Presentation::Standalone }
    }
}

/// Whether the query has a parameter named `name`, read exactly as the page's own
/// `new URLSearchParams(location.search).has(name)` reads it (`index.html`), so the page and the
/// canvas always agree: one leading `?` dropped, `&`-separated pairs, the name before the first
/// `=`, decoded (see [`form_decode`]). The value doesn't matter.
fn has_name(search: &str, name: &str) -> bool {
    let query = search.strip_prefix('?').unwrap_or(search);
    query.split('&').map(|pair| pair.split_once('=').map_or(pair, |(n, _)| n)).any(|n| form_decode(n) == name)
}

/// A query-string name as `URLSearchParams` decodes it (application/x-www-form-urlencoded): `+`
/// is a space, `%XX` a byte (a `%` without two hex digits stays as it is), and the bytes are read
/// as UTF-8 with invalid sequences replaced.
fn form_decode(raw: &str) -> String {
    let hex = |b: Option<&u8>| b.and_then(|b| char::from(*b).to_digit(16)).and_then(|d| u8::try_from(d).ok());
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        i += 1;
        match byte {
            b'+' => out.push(b' '),
            b'%' => match (hex(bytes.get(i)), hex(bytes.get(i + 1))) {
                (Some(high), Some(low)) => {
                    out.push((high << 4) | low);
                    i += 2;
                }
                _ => out.push(byte),
            },
            _ => out.push(byte),
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

// The page's own wording. Standalone: exactly what it said before the presentation existed.
// Embedded: the LTS editor's name, and only what is true there (the LTS lab runs `?store=memory`,
// where nothing is kept in the browser and reloading would discard the session).

/// `?reset`'s confirmation (browser storage only).
pub fn reset_confirmation(presentation: Presentation) -> &'static str {
    match presentation {
        Presentation::Standalone => {
            "?reset deletes the LightCraft library stored in this browser, including every imported photo. \
             This can't be undone. Delete it?"
        }
        Presentation::LtsEmbedded => {
            "?reset deletes the Image adjustments library stored in this browser, including every imported photo. \
             This can't be undone. Delete it?"
        }
    }
}

/// Shown when the browser didn't grant persistent storage (browser storage only).
pub fn eviction_notice(presentation: Presentation) -> &'static str {
    match presentation {
        Presentation::Standalone => {
            "This browser may delete LightCraft's library when it runs short of space (persistent storage wasn't granted). \
             Keep your original photos elsewhere and use File ▸ Back Up Library… now and then."
        }
        Presentation::LtsEmbedded => {
            "This browser may delete the Image adjustments library when it runs short of space (persistent storage wasn't \
             granted). Keep your original photos elsewhere and use File ▸ Back Up Library… now and then."
        }
    }
}

/// Shown once at start-up when the library lives in browser storage.
pub fn experimental_notice(presentation: Presentation) -> &'static str {
    match presentation {
        Presentation::Standalone => {
            "LightCraft in the browser is experimental. Your library is kept in this browser's storage: keep your original photos \
             elsewhere and back up with File ▸ Back Up Library…."
        }
        Presentation::LtsEmbedded => {
            "Image adjustments in the browser is experimental. Your library is kept in this browser's storage: keep your \
             original photos elsewhere and back up with File ▸ Back Up Library…."
        }
    }
}

/// Covers the page when another tab already has the library (browser storage only).
pub fn tab_lock_message(presentation: Presentation) -> &'static str {
    match presentation {
        Presentation::Standalone => {
            "LightCraft is already open in another tab or window of this browser.\n\n\
             Switch to that tab, or close it and reload this page. (Two tabs would overwrite each other's changes.)"
        }
        Presentation::LtsEmbedded => {
            "Image adjustments is already open in another tab or window of this browser.\n\n\
             Close it there, then open this editor again. (Two copies would overwrite each other's changes.)"
        }
    }
}

/// The status line when eframe could not start (`detail`: the error).
pub fn start_failure(presentation: Presentation, detail: &str) -> String {
    match presentation {
        Presentation::Standalone => format!("LightCraft failed to start: {detail}"),
        Presentation::LtsEmbedded => format!("{EMBEDDED_NAME} failed to start: {detail}"),
    }
}

/// The library window's error text when browser storage couldn't be opened as a library.
pub fn library_problem_error(presentation: Presentation, error: &str) -> String {
    match presentation {
        Presentation::Standalone => format!("{error}. Reload the page to try again."),
        Presentation::LtsEmbedded => format!("{error}."),
    }
}

/// The overlay after a panic (`panic = "abort"` on wasm ends the module).
pub fn panic_message(presentation: Presentation, msg: &str) -> String {
    match presentation {
        Presentation::Standalone => format!(
            "LightCraft stopped after an internal error:\n{msg}\n\nThe library saved in this browser is kept. Reload the page to continue.\n\
             If it happens again right away, a photo may be the cause: reload with ?workers=0 or report the error."
        ),
        Presentation::LtsEmbedded => format!(
            "{EMBEDDED_NAME} stopped after an internal error:\n{msg}\n\nThis editing session has ended. Changes that exist only in this \
             editor are not saved to LTS; use results you already saved or exported."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PANIC: &str = "index out of bounds";

    /// The pure refactor keeps every existing option's meaning.
    #[test]
    fn storage_worker_and_bench_options_keep_their_meaning() {
        let o = Options::parse("?store=memory&workers=1&embedded");
        assert_eq!((o.store.as_str(), o.workers, o.bench, o.reset), ("memory", Some(1), false, false));
        assert_eq!(Options::parse("").store, "opfs");
        assert_eq!(Options::parse("?store=idb").store, "idb");
        assert_eq!(Options::parse("?store=bogus").store, "opfs");
        assert_eq!(Options::parse("store=memory").store, "memory");
        assert_eq!(Options::parse("?workers=x").workers, None);
        assert_eq!(Options::parse("?workers=0").workers, Some(0));
        assert!(Options::parse("?bench").bench && Options::parse("?reset=1").reset);
        assert!(!Options::parse("?benchmark&x=reset").bench && !Options::parse("?benchmark&x=reset").reset);
    }

    /// `embedded` is the exact key's presence, read as the page's own `URLSearchParams` reads it
    /// (`index.html`): any value, percent-encoded letters; never a substring, a value or another case.
    #[test]
    fn web_options_parse_exact_embedded_flag() {
        for search in [
            "?store=memory&workers=1&embedded",
            "?embedded",
            "?embedded=",
            "?embedded=1",
            "?embedded=false",
            "?x=1&embedded&y",
            "?%65mbedded",
            "embedded",
        ] {
            let o = Options::parse(search);
            assert!(o.embedded, "{search}");
            assert_eq!(o.presentation(), Presentation::LtsEmbedded, "{search}");
        }
        for search in [
            "",
            "?",
            "?embeddedx",
            "?x=embedded",
            "?notembedded",
            "?Embedded",
            "?EMBEDDED",
            "?embedded%3D1",
            "?mode=embedded",
            "?em+bedded",
            "??embedded",
        ] {
            let o = Options::parse(search);
            assert!(!o.embedded, "{search}");
            assert_eq!(o.presentation(), Presentation::Standalone, "{search}");
        }
        // the flag changes nothing else: storage, workers, bench and reset are the same
        let plain = Options::parse("?store=memory&workers=1");
        let embedded = Options::parse("?store=memory&workers=1&embedded");
        assert_eq!(Options { embedded: false, ..embedded }, plain);
    }

    /// `demo` is the exact key's presence, read like `embedded`; a new library is seeded with the
    /// demo photos unless the page is embedded without `demo` (OC-5). The flag changes nothing else.
    #[test]
    fn demo_flag_seeds_the_demo_library_unless_embedded() {
        for (search, demo, seeds) in [
            ("", false, true),
            ("?store=memory&workers=1", false, true),
            ("?demo", true, true),
            ("?embedded", false, false),
            ("?store=memory&workers=1&embedded", false, false),
            ("?embedded&demo", true, true),
            ("?embedded&demo=0", true, true),
            ("?embedded&%64emo", true, true),
            ("?embedded&demox", false, false),
            ("?embedded&nodemo", false, false),
            ("?embedded&x=demo", false, false),
            ("?embedded&Demo", false, false),
            ("?embedded&demo%3D1", false, false),
        ] {
            let o = Options::parse(search);
            assert_eq!((o.demo, o.seed_demo()), (demo, seeds), "{search}");
        }
        let plain = Options::parse("?store=memory&workers=1&embedded");
        let with_demo = Options::parse("?store=memory&workers=1&embedded&demo");
        assert_eq!(Options { demo: false, ..with_demo }, plain);
    }

    /// The standalone page says exactly what it said before the presentation existed.
    #[test]
    fn standalone_page_copy_is_unchanged() {
        let s = Presentation::Standalone;
        assert_eq!(
            panic_message(s, PANIC),
            "LightCraft stopped after an internal error:\nindex out of bounds\n\nThe library saved in this browser is kept. Reload the page to \
             continue.\nIf it happens again right away, a photo may be the cause: reload with ?workers=0 or report the error."
        );
        assert_eq!(
            reset_confirmation(s),
            "?reset deletes the LightCraft library stored in this browser, including every imported photo. This can't be undone. Delete it?"
        );
        assert_eq!(
            eviction_notice(s),
            "This browser may delete LightCraft's library when it runs short of space (persistent storage wasn't granted). Keep your original \
             photos elsewhere and use File ▸ Back Up Library… now and then."
        );
        assert_eq!(
            experimental_notice(s),
            "LightCraft in the browser is experimental. Your library is kept in this browser's storage: keep your original photos elsewhere \
             and back up with File ▸ Back Up Library…."
        );
        assert_eq!(
            tab_lock_message(s),
            "LightCraft is already open in another tab or window of this browser.\n\nSwitch to that tab, or close it and reload this page. \
             (Two tabs would overwrite each other's changes.)"
        );
        assert_eq!(start_failure(s, "JsValue(\"x\")"), "LightCraft failed to start: JsValue(\"x\")");
        assert_eq!(library_problem_error(s, "quota exceeded"), "quota exceeded. Reload the page to try again.");
    }

    /// Embedded (`?embedded`, the LTS lab runs `?store=memory`): the page names the LTS editor, and
    /// what it says after a failure is true of a session kept only in this tab — it doesn't say a
    /// library is kept, doesn't suggest reloading (which would discard the session) and doesn't
    /// send the user to upstream support.
    #[test]
    fn embedded_page_copy_is_neutral_and_truthful_for_a_session_in_memory() {
        let e = Presentation::LtsEmbedded;
        let panic = panic_message(e, PANIC);
        assert!(panic.starts_with("LTS Image adjustments stopped after an internal error:\nindex out of bounds\n\n"), "{panic}");
        assert!(panic.contains("not saved to LTS"), "{panic}");
        assert_eq!(start_failure(e, "JsValue(\"x\")"), "LTS Image adjustments failed to start: JsValue(\"x\")");
        let problem = library_problem_error(e, "quota exceeded");
        assert!(problem.starts_with("quota exceeded"), "{problem}");
        for text in [&panic, &problem] {
            for banned in ["Reload", "reload", "is kept", "kept in this browser", "report", "?workers"] {
                assert!(!text.contains(banned), "{banned:?} in {text:?}");
            }
        }
        let start = start_failure(e, "x");
        for text in
            [panic.as_str(), problem.as_str(), start.as_str(), reset_confirmation(e), eviction_notice(e), experimental_notice(e), tab_lock_message(e)]
        {
            assert!(!text.contains("LightCraft") && !text.contains("ArtCraft"), "{text:?}");
        }
    }
}
