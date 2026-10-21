//! Public-boundary embedding tests (L1): the host-selected presentation gates the single command
//! funnel (`LightcraftApp::run`) and the single URL sink (`links::open`). No browser, media,
//! filesystem or model fixture.

use std::sync::{Arc, Mutex};

use lightcraft_engine::Session;
use lightcraft_ui_egui::control::{self, ControlRequest, Outcome};
use lightcraft_ui_egui::state::Dialog;
use lightcraft_ui_egui::{LightcraftApp, Services, UiState};
use lightcraft_ui_egui::{links, presentation::Presentation};
use serde_json::{Value, json};

const COMMUNITY: &str = "Community navigation is unavailable in the LTS editor";
const EXTERNAL: &str = "External links are unavailable in the LTS editor";
/// Every link command but `app.help` (which opens the local Keyboard Shortcuts when embedded).
const COMMUNITY_IDS: [&str; 5] = ["app.discord", "app.feedback", "app.website", "app.github", "app.artcraft"];
/// External pages other than the link table's: the upstream docs behind F1, a contributor's
/// profile, a photo's place on a map, the SAM licence, anything else.
const OTHER_URLS: [&str; 5] = [
    links::HELP,
    "https://github.com/some-contributor",
    "https://www.openstreetmap.org/?mlat=46.000000&mlon=7.000000#map=15/46.000000/7.000000",
    "https://github.com/facebookresearch/sam3/blob/main/LICENSE",
    "https://example.com/",
];

fn app(presentation: Presentation) -> (LightcraftApp, Arc<Mutex<Vec<String>>>) {
    let opened = Arc::new(Mutex::new(Vec::new()));
    let capture = opened.clone();
    let services = Services {
        open_url: Some(Box::new(move |url: &str| {
            capture.lock().unwrap().push(url.to_string());
            Ok(())
        })),
        ..Default::default()
    };
    (LightcraftApp::new(Session::new(), services).with_presentation(presentation), opened)
}

fn call(app: &mut LightcraftApp, method: &str, id: &str) -> Value {
    let (request, _) = ControlRequest::new(method, json!({"command": id, "params": {}}));
    match control::handle(app, &egui::Context::default(), &request) {
        Outcome::Done(value) => value,
        _ => panic!("link invocation is immediate"),
    }
}

#[test]
fn community_ids_are_the_link_commands_except_help() {
    let mut ids: Vec<&str> = links::LINKS.iter().map(|(id, _, _)| *id).filter(|id| *id != "app.help").collect();
    ids.sort_unstable();
    let mut want = COMMUNITY_IDS.to_vec();
    want.sort_unstable();
    assert_eq!(ids, want);
}

#[test]
fn standalone_public_control_preserves_every_link_destination() {
    assert_eq!(LightcraftApp::new(Session::new(), Services::default()).presentation(), Presentation::Standalone);
    for method in ["engine.execute", "ui.menu.invoke", "command"] {
        let (mut app, opened) = app(Presentation::Standalone);
        for (id, _, url) in links::LINKS {
            assert_eq!(call(&mut app, method, id), json!({"ok": true, "result": {"url": url}}), "{method} {id}");
        }
        assert_eq!(*opened.lock().unwrap(), links::LINKS.iter().map(|(_, _, url)| url.to_string()).collect::<Vec<_>>());
    }
}

#[test]
fn standalone_direct_link_sink_opens_every_destination() {
    let (mut app, opened) = app(Presentation::Standalone);
    let urls: Vec<&str> = links::LINKS.iter().map(|(_, _, url)| *url).chain(OTHER_URLS).collect();
    for url in &urls {
        assert_eq!(links::open(&mut app, url), Ok(json!({"url": url})), "{url}");
    }
    assert_eq!(*opened.lock().unwrap(), urls.iter().map(|url| url.to_string()).collect::<Vec<_>>());
}

#[test]
fn embedded_public_control_refuses_community_before_calling_host_service() {
    for method in ["engine.execute", "ui.menu.invoke", "command"] {
        let (mut app, opened) = app(Presentation::LtsEmbedded);
        for id in COMMUNITY_IDS {
            assert_eq!(call(&mut app, method, id), json!({"ok": false, "error": COMMUNITY}), "{method} {id}");
        }
        assert!(opened.lock().unwrap().is_empty());
    }
}

/// Embedded, the sink opens nothing: the five community destinations are refused with the
/// community message, every other external URL (including the upstream docs) with the
/// external-link message, before the host service is called.
#[test]
fn embedded_direct_link_sink_refuses_every_external_url() {
    let (mut app, opened) = app(Presentation::LtsEmbedded);
    for (id, _, url) in links::LINKS {
        let refusal = if *id == "app.help" { EXTERNAL } else { COMMUNITY };
        assert_eq!(links::open(&mut app, url), Err(refusal.to_string()), "{id}");
    }
    for url in OTHER_URLS.into_iter().chain(["mailto:someone@example.com", "http://localhost:8080/"]) {
        assert_eq!(links::open(&mut app, url), Err(EXTERNAL.to_string()), "{url}");
    }
    assert!(opened.lock().unwrap().is_empty(), "{:?}", opened.lock().unwrap());
}

#[test]
fn embedded_help_opens_local_shortcuts_without_external_navigation() {
    for method in ["engine.execute", "ui.menu.invoke", "command"] {
        let (mut app, opened) = app(Presentation::LtsEmbedded);
        assert_eq!(call(&mut app, method, "app.help"), json!({"ok": true, "result": null}), "{method}");
        assert!(matches!(app.ui.dialog, Some(Dialog::Shortcuts)), "{method}");
        assert!(opened.lock().unwrap().is_empty(), "{method}");
    }
}

#[test]
fn restoring_ui_preferences_does_not_replace_instance_embedding_policy() {
    let (mut app, opened) = app(Presentation::LtsEmbedded);
    let restored = serde_json::to_vec(&UiState::default()).unwrap();
    app.ui = serde_json::from_slice(&restored).unwrap();
    assert_eq!(app.presentation(), Presentation::LtsEmbedded);
    assert_eq!(call(&mut app, "engine.execute", "app.discord"), json!({"ok": false, "error": COMMUNITY}));
    assert!(opened.lock().unwrap().is_empty());
    // ui.json never carries the presentation: no key, at any depth, names it
    fn keys(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, v) in map {
                    out.push(key.to_ascii_lowercase());
                    keys(v, out);
                }
            }
            Value::Array(values) => values.iter().for_each(|v| keys(v, out)),
            _ => {}
        }
    }
    let mut saved = Vec::new();
    keys(&serde_json::to_value(&app.ui).unwrap(), &mut saved);
    for forbidden in ["presentation", "embedded", "ltsembedded"] {
        assert!(!saved.iter().any(|key| key == forbidden), "{forbidden} in {saved:?}");
    }
}
