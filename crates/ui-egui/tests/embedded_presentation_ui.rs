//! L2 embedding tests: what the LTS presentation paints, lists and registers, through the public
//! control channel and real egui frames (`run_ui`; no window, GPU or browser). The standalone runs
//! are the regression controls: they pass before and after the presentation work.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use lightcraft_engine::Session;
use lightcraft_ui_egui::control::{self, ControlRequest, Outcome};
use lightcraft_ui_egui::credits::{self, NameMode, SortKey};
use lightcraft_ui_egui::headless::HeadlessView;
use lightcraft_ui_egui::menubar::{self, MenuNode};
use lightcraft_ui_egui::panels::{dialogs, library_problem::LibraryProblem, topbar};
use lightcraft_ui_egui::state::Dialog;
use lightcraft_ui_egui::{LightcraftApp, Services, links, menus, presentation::Presentation, shortcuts, widgets};
use serde_json::{Value, json};

/// The plain-text attribution: the one place the upstream names appear in the LTS presentation.
const ATTRIBUTION: &str = "Modified version based on LightCraft by the ArtCraft team and community.";
/// Upstream promotion and identity the LTS presentation neither paints nor lists.
const UPSTREAM: [&str; 5] = ["Discord", "ArtCraft", "GitHub", "getartcraft", "LightCraft"];
/// Every link command but `app.help`.
const COMMUNITY_IDS: [&str; 5] = ["app.discord", "app.feedback", "app.website", "app.github", "app.artcraft"];

fn assert_no_upstream_branding(text: &str, what: &str) {
    let rest = text.replace(ATTRIBUTION, "");
    for banned in UPSTREAM {
        assert!(!rest.contains(banned), "{what}: {banned:?} in the LTS presentation:\n{text}");
    }
}

fn app(presentation: Presentation) -> (LightcraftApp, Arc<Mutex<Vec<String>>>) {
    app_with(presentation, Session::new())
}

fn app_with(presentation: Presentation, session: Session) -> (LightcraftApp, Arc<Mutex<Vec<String>>>) {
    let opened = Arc::new(Mutex::new(Vec::new()));
    let capture = opened.clone();
    let services = Services {
        open_url: Some(Box::new(move |url: &str| {
            capture.lock().unwrap().push(url.to_string());
            Ok(())
        })),
        ..Default::default()
    };
    (LightcraftApp::new(session, services).with_presentation(presentation), opened)
}

/// The app in real egui frames; keeps the last frame's painted text and cursor.
struct Driver {
    ctx: egui::Context,
    app: LightcraftApp,
    opened: Arc<Mutex<Vec<String>>>,
    time: f64,
    text: String,
    cursor: egui::CursorIcon,
}

impl Driver {
    fn new(presentation: Presentation) -> Self {
        Driver::with_session(presentation, Session::new())
    }

    fn with_session(presentation: Presentation, session: Session) -> Self {
        let (app, opened) = app_with(presentation, session);
        let mut d = Driver { ctx: egui::Context::default(), app, opened, time: 0.0, text: String::new(), cursor: egui::CursorIcon::Default };
        // fonts install on the first frame and are ready on the second
        d.frames(4);
        d
    }

    fn frame(&mut self) {
        fn collect(shape: &egui::epaint::Shape, text: &mut String) {
            match shape {
                egui::epaint::Shape::Text(shape) => {
                    text.push_str(&shape.galley.job.text);
                    text.push('\n');
                }
                egui::epaint::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
                _ => {}
            }
        }
        let mut raw = HeadlessView::raw_input(egui::vec2(1600.0, 1000.0), 1.0, self.time, vec![]);
        self.app.raw_input_hook(&mut raw);
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(raw, |ui| {
            app.logic(ui.ctx());
            app.ui(ui);
        });
        // This inspects shapes without a renderer; discard texture uploads explicitly.
        out.textures_delta.clear();
        self.time += 1.0 / 60.0;
        self.text.clear();
        for clipped in &out.shapes {
            collect(&clipped.shape, &mut self.text);
        }
        self.cursor = out.platform_output.cursor_icon;
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame();
        }
    }

    /// A control request, then frames until its injected input has played out.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let (request, _) = ControlRequest::new(method, params);
        let reply = match control::handle(&mut self.app, &self.ctx, &request) {
            Outcome::Done(value) => value,
            Outcome::Screenshot { .. } => panic!("no screenshots in these tests"),
        };
        for _ in 0..20 {
            if self.app.synthetic.is_empty() {
                break;
            }
            self.frame();
        }
        self.frames(3);
        reply
    }

    fn ok(&mut self, method: &str, params: Value) -> Value {
        let reply = self.request(method, params.clone());
        assert_eq!(reply["ok"], true, "{method} {params}: {reply}");
        reply["result"].clone()
    }

    fn widget_ids(&self) -> BTreeSet<String> {
        self.app.widgets.iter().map(|(id, _)| id.clone()).collect()
    }

    fn opened(&self) -> Vec<String> {
        self.opened.lock().unwrap().clone()
    }
}

/// (id, label) of every node of the menu bar, submenus included (menus and submenus have no id).
fn menu_items(app: &LightcraftApp) -> Vec<(String, String)> {
    fn walk(nodes: &[MenuNode], out: &mut Vec<(String, String)>) {
        for node in nodes {
            match node {
                MenuNode::Item { id, label, .. } => out.push((id.clone(), label.clone())),
                MenuNode::Submenu { label, children } => {
                    out.push((String::new(), label.clone()));
                    walk(children, out);
                }
                MenuNode::Separator => {}
            }
        }
    }
    let mut out = Vec::new();
    for (title, nodes) in menubar::menu_bar(app) {
        out.push((String::new(), title));
        walk(&nodes, &mut out);
    }
    out
}

fn label_of<'a>(items: &'a [(String, String)], id: &str) -> Option<&'a str> {
    items.iter().find(|(i, _)| i == id).map(|(_, label)| label.as_str())
}

/// Every `"label"` string in a JSON tree.
fn labels(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                if key == "label"
                    && let Some(s) = v.as_str()
                {
                    out.push(s.to_string());
                }
                labels(v, out);
            }
        }
        Value::Array(values) => values.iter().for_each(|v| labels(v, out)),
        _ => {}
    }
}

fn catalog(app: &mut LightcraftApp, method: &str) -> Value {
    let (request, _) = ControlRequest::new(method, json!({}));
    match control::handle(app, &egui::Context::default(), &request) {
        Outcome::Done(reply) => {
            assert_eq!(reply["ok"], true, "{method}: {reply}");
            reply["result"].clone()
        }
        Outcome::Screenshot { .. } => panic!("{method} answers at once"),
    }
}

// ------------------------------------------------------------------ menus and catalogs (R2)

#[test]
fn embedded_menu_bar_has_no_upstream_routes() {
    let (app, _) = app(Presentation::LtsEmbedded);
    let items = menu_items(&app);
    for id in COMMUNITY_IDS {
        assert!(label_of(&items, id).is_none(), "{id} in the LTS menus");
    }
    for (id, label) in &items {
        for banned in UPSTREAM {
            assert!(!label.contains(banned), "{id}: {label}");
        }
    }
    assert_eq!(label_of(&items, "app.about"), Some("Editing engine information"));
    assert_eq!(label_of(&items, "app.help"), Some("Editing shortcuts"));
    assert_eq!(label_of(&items, "app.quit"), Some("Quit editor"));
    for kept in ["app.shortcuts", "app.systemInfo", "app.whatsNew", "app.settings", "dialog.export"] {
        assert!(label_of(&items, kept).is_some(), "{kept} stays in the LTS menus");
    }
}

#[test]
fn standalone_menu_bar_keeps_upstream_routes() {
    let (app, _) = app(Presentation::Standalone);
    let items = menu_items(&app);
    for id in COMMUNITY_IDS.into_iter().chain(["app.help", "app.shortcuts", "app.systemInfo", "app.whatsNew"]) {
        assert!(label_of(&items, id).is_some(), "{id}");
    }
    assert_eq!(label_of(&items, "app.about"), Some("About LightCraft"));
    assert_eq!(label_of(&items, "app.help"), Some("LightCraft Help"));
    assert_eq!(label_of(&items, "app.quit"), Some("Quit LightCraft"));
}

#[test]
fn embedded_command_catalogs_hide_promo_ids() {
    for presentation in [Presentation::Standalone, Presentation::LtsEmbedded] {
        let (mut app, opened) = app(presentation);
        for method in ["engine.commands", "ui.menu.list", "ui.menu.tree"] {
            let result = catalog(&mut app, method);
            let text = result.to_string();
            for id in COMMUNITY_IDS {
                let listed = text.contains(format!("\"{id}\"").as_str());
                assert_eq!(listed, presentation == Presentation::Standalone, "{presentation:?} {method}: {id}");
            }
            for kept in ["app.help", "app.about", "app.shortcuts", "app.systemInfo", "app.whatsNew", "app.quit"] {
                assert!(text.contains(format!("\"{kept}\"").as_str()), "{presentation:?} {method}: {kept}");
            }
            if presentation == Presentation::LtsEmbedded {
                let mut all = Vec::new();
                labels(&result, &mut all);
                for label in &all {
                    for banned in UPSTREAM {
                        assert!(!label.contains(banned), "{method}: {label}");
                    }
                }
                for banned in ["Discord", "ArtCraft", "GitHub", "getartcraft"] {
                    assert!(!text.contains(banned), "{method}: {banned}");
                }
                assert!(all.iter().any(|label| label == "Editing engine information"), "{method}");
            }
        }
        assert!(opened.lock().unwrap().is_empty(), "listing opens nothing");
    }
}

/// A Keyboard Shortcuts row: (label, shortcut, command id).
type Row = (&'static str, &'static str, &'static str);

fn row_keys(rows: &[Row]) -> Vec<(&'static str, &'static str)> {
    rows.iter().map(|(_, sc, id)| (*sc, *id)).collect()
}

fn row_label(rows: &[Row], id: &str) -> Option<&'static str> {
    rows.iter().find(|row| row.2 == id).map(|row| row.0)
}

#[test]
fn shortcuts_dialog_rows_follow_presentation() {
    let standalone = dialogs::shortcut_rows(Presentation::Standalone);
    let embedded = dialogs::shortcut_rows(Presentation::LtsEmbedded);
    // the same bindings in the same order: only product-named labels differ
    assert_eq!(row_keys(&standalone), row_keys(&embedded));
    assert_eq!(row_label(&standalone, "app.help"), Some("LightCraft Help"));
    assert_eq!(row_label(&embedded, "app.help"), Some("Editing shortcuts"));
    assert_eq!(row_label(&standalone, "app.quit"), Some("Quit LightCraft"));
    assert_eq!(row_label(&embedded, "app.quit"), Some("Quit editor"));
    for (label, _, id) in &embedded {
        assert!(!COMMUNITY_IDS.contains(id), "{id}");
        for banned in UPSTREAM {
            assert!(!label.contains(banned), "{id}: {label}");
        }
    }
}

// ------------------------------------------------------------------ shortcut precedence pins

/// The hidden commands declare no key (a key added upstream would stay reserved by its command,
/// and the dispatch gate would refuse it).
#[test]
fn hidden_ids_declare_no_shortcuts() {
    for id in COMMUNITY_IDS {
        let command = menus::ui_commands().find(|c| c.0 == id);
        assert!(command.is_some(), "{id} stays a UI command (xtask parity, agents)");
        assert_eq!(command.and_then(|c| c.2), None, "{id}");
        assert!(!shortcuts::ALIASES.iter().any(|(_, target, _)| *target == id), "{id}");
        assert!(lightcraft_engine::find_command(id).is_none(), "{id}");
    }
}

#[test]
fn f1_still_reserved_by_app_help() {
    let f1 = shortcuts::parse("F1");
    assert!(f1.is_some());
    let bound: Vec<&str> = menus::ui_commands().filter(|c| c.2.and_then(shortcuts::parse) == f1).map(|c| c.0).collect();
    assert_eq!(bound, ["app.help"]);
    assert!(!lightcraft_engine::command_specs().iter().any(|c| c.shortcut.and_then(shortcuts::parse) == f1));
    assert!(!shortcuts::ALIASES.iter().any(|(sc, _, _)| shortcuts::parse(sc) == f1));
}

/// (view, dialog, opened URLs) after one key in a fresh app.
fn after_key(presentation: Presentation, key: Value) -> (String, Option<Dialog>, Vec<String>) {
    let mut d = Driver::new(presentation);
    d.ok("ui.key", key);
    (format!("{:?}", d.app.ui.view), d.app.ui.dialog.clone(), d.opened())
}

/// The shortcut tables and their dispatch don't depend on the presentation: every key does the
/// same in both (F1, whose command acts differently, is covered by the two F1 tests below).
#[test]
fn shortcut_dispatch_ignores_presentation() {
    for key in [json!({"key": "/", "cmd": true}), json!({"key": "d"}), json!({"key": "g"}), json!({"key": "y"})] {
        assert_eq!(after_key(Presentation::Standalone, key.clone()), after_key(Presentation::LtsEmbedded, key.clone()), "{key}");
    }
}

#[test]
fn embedded_f1_opens_local_shortcuts() {
    let mut d = Driver::new(Presentation::LtsEmbedded);
    d.ok("ui.key", json!({"key": "F1"}));
    assert_eq!(d.app.ui.dialog, Some(Dialog::Shortcuts));
    assert!(d.opened().is_empty(), "{:?}", d.opened());
    assert!(d.text.contains("Keyboard Shortcuts"), "{}", d.text);
}

#[test]
fn standalone_f1_still_opens_the_documentation() {
    let mut d = Driver::new(Presentation::Standalone);
    d.ok("ui.key", json!({"key": "F1"}));
    assert_eq!(d.opened(), vec![links::HELP.to_string()]);
    assert_eq!(d.app.ui.dialog, None);
}

// ------------------------------------------------------------------ painting and widgets (R3)

#[test]
fn embedded_paints_no_community_or_upstream_branding() {
    let mut d = Driver::new(Presentation::LtsEmbedded);
    assert!(d.text.contains("Help"), "the window painted:\n{}", d.text);
    assert_no_upstream_branding(&d.text, "window");
    d.ok("ui.clickWidget", json!({"id": "menu:Help"}));
    assert!(d.text.contains("Editing engine information") && d.text.contains("Editing shortcuts"), "Help menu:\n{}", d.text);
    assert_no_upstream_branding(&d.text, "Help menu");
    assert!(d.opened().is_empty());
    // About through its command, and through the tool strip's More button (no command involved)
    for (method, params) in [("engine.execute", json!({"command": "app.about"})), ("ui.clickWidget", json!({"id": "icon:more"}))] {
        let mut d = Driver::new(Presentation::LtsEmbedded);
        d.ok(method, params);
        assert_eq!(d.app.ui.dialog, Some(Dialog::About), "{method}");
        for shown in ["Editing engine information", "LTS Image adjustments", ATTRIBUTION, "MIT OR Apache-2.0"] {
            assert!(d.text.contains(shown), "{method}: {shown:?} not in\n{}", d.text);
        }
        assert_no_upstream_branding(&d.text, method);
        assert!(d.opened().is_empty());
    }
}

#[test]
fn standalone_about_keeps_the_upstream_identity() {
    let mut d = Driver::new(Presentation::Standalone);
    d.ok("engine.execute", json!({"command": "app.about"}));
    for kept in ["About LightCraft", "Join the ArtCraft Discord", "LightCraft website", "Source code on GitHub", "MIT OR Apache-2.0"] {
        assert!(d.text.contains(kept), "{kept}:\n{}", d.text);
    }
}

#[test]
fn embedded_widgets_omit_promo_but_keep_useful_controls() {
    let mut d = Driver::new(Presentation::LtsEmbedded);
    let ids = d.widget_ids();
    assert!(!ids.contains("icon:discord"), "{ids:?}");
    for kept in ["icon:cloud", "icon:help", "icon:share", "icon:bell", "icon:more", "field:search", "menu:Help"] {
        assert!(ids.contains(kept), "{kept}: {ids:?}");
    }
    d.ok("engine.execute", json!({"command": "app.about"}));
    let ids = d.widget_ids();
    assert!(!ids.iter().any(|id| id == "button:aboutDiscord" || id.starts_with("link:")), "{ids:?}");
    for tab in ["about", "contributors", "models"] {
        assert!(ids.contains(&format!("button:aboutTab-{tab}")), "{tab}: {ids:?}");
    }
}

#[test]
fn standalone_widgets_keep_the_upstream_routes() {
    let mut d = Driver::new(Presentation::Standalone);
    assert!(d.widget_ids().contains("icon:discord"));
    d.ok("engine.execute", json!({"command": "app.about"}));
    let ids = d.widget_ids();
    for kept in ["button:aboutDiscord", "link:aboutWebsite", "link:aboutGithub", "link:aboutArtcraft"] {
        assert!(ids.contains(kept), "{kept}: {ids:?}");
    }
}

/// About ▸ Contributors: a link to the profile in the standalone app; plain text (with the same
/// hover summary) in the LTS editor, which opens no external page.
#[test]
fn contributor_names_are_plain_text_when_embedded() {
    let first = credits::sorted(credits::CONTRIBUTORS, NameMode::Username, SortKey::FirstCommit, true)
        .first()
        .map(|c| c.login)
        .expect("contributor credits are compiled in");
    for presentation in [Presentation::Standalone, Presentation::LtsEmbedded] {
        let mut d = Driver::new(presentation);
        d.ok("engine.execute", json!({"command": "app.about"}));
        d.ok("ui.clickWidget", json!({"id": "button:aboutTab-contributors"}));
        let id = format!("credit:{first}");
        d.ok("ui.hoverWidget", json!({"id": id}));
        let pointing = d.cursor == egui::CursorIcon::PointingHand;
        d.ok("ui.clickWidget", json!({"id": id}));
        if presentation == Presentation::Standalone {
            assert!(pointing, "a standalone name is a link");
            assert_eq!(d.opened(), vec![format!("https://github.com/{first}")]);
        } else {
            assert!(!pointing, "an LTS name is plain text");
            assert!(d.opened().is_empty(), "{:?}", d.opened());
        }
        assert_eq!(d.app.ui.dialog, Some(Dialog::About), "{presentation:?}");
    }
}

/// Window ▸ Second Window: on the web (as here) egui paints that viewport as a window inside the
/// canvas, with its title, so the LTS editor names it after itself.
#[test]
fn second_window_title_follows_presentation() {
    for presentation in [Presentation::Standalone, Presentation::LtsEmbedded] {
        let mut d = Driver::new(presentation);
        d.ok("engine.execute", json!({"command": "view.secondWindow", "params": {"show": true}}));
        assert!(d.app.ui.second_window, "{presentation:?}");
        let title = match presentation {
            Presentation::Standalone => "LightCraft — Second Window",
            Presentation::LtsEmbedded => "LTS Image adjustments — Second Window",
        };
        assert!(d.text.contains(title), "{presentation:?}: {title:?} not in\n{}", d.text);
        if presentation == Presentation::LtsEmbedded {
            assert_no_upstream_branding(&d.text, "second window");
        }
    }
}

/// Info ▸ Show on Map opens OpenStreetMap: the standalone app keeps the button; the LTS editor
/// opens no external page, so it doesn't paint it (the coordinates stay, and stay editable).
#[test]
fn show_on_map_is_painted_only_in_the_standalone_app() {
    for presentation in [Presentation::Standalone, Presentation::LtsEmbedded] {
        let mut d = Driver::with_session(presentation, Session::with_demo());
        let first = d.app.session.visible_cloned().first().map(|p| p.0).expect("demo photos");
        d.ok("engine.execute", json!({"command": "library.select", "params": {"ids": [first]}}));
        d.ok("engine.execute", json!({"command": "photo.setMeta", "params": {"gps": "46.0, 7.0"}}));
        d.ok("ui.set", json!({"right": "info"}));
        let ids = d.widget_ids();
        assert!(ids.contains("field:gps"), "{presentation:?}: the coordinates stay: {ids:?}");
        assert_eq!(ids.contains("button:showOnMap"), presentation == Presentation::Standalone, "{presentation:?}: {ids:?}");
        assert!(d.opened().is_empty(), "{presentation:?}: {:?}", d.opened());
    }
}

/// About ▸ Third-party notices, in both presentations: selectable plain text, no links.
#[test]
fn third_party_notices_tab_is_plain_text_in_both_presentations() {
    for presentation in [Presentation::Standalone, Presentation::LtsEmbedded] {
        let mut d = Driver::new(presentation);
        d.ok("engine.execute", json!({"command": "app.about"}));
        let r = d.request("ui.clickWidget", json!({"id": "button:aboutTab-notices"}));
        assert_eq!(r["ok"], true, "{presentation:?}: {r}");
        for shown in [ATTRIBUTION, "not a complete list", "Third-party material"] {
            assert!(d.text.contains(shown), "{presentation:?}: {shown:?} not in\n{}", d.text);
        }
        assert!(!d.widget_ids().iter().any(|id| id.starts_with("link:")), "{presentation:?}: {:?}", d.widget_ids());
        d.ok("ui.click", json!({"x": 800.0, "y": 500.0}));
        assert!(d.opened().is_empty(), "{presentation:?}: {:?}", d.opened());
    }
}

// ------------------------------------------------------------------ neutral copy (both presentations)

#[test]
fn neutral_copy_names_no_product() {
    let raw = widgets::preview_only_explanation("Nikon Huffman-compressed NEF (no clean-room decoder yet)");
    assert!(raw.starts_with("This raw variant can't be decoded yet (Nikon Huffman-compressed NEF)."), "{raw}");
    let tip = topbar::unsaved_tooltip(2, "disk full");
    assert!(tip.starts_with("2 changes saved in memory but not written to disk: disk full\n"), "{tip}");
    assert!(tip.contains("would lose them") && !tip.contains("LightCraft"), "{tip}");
}

#[test]
fn settings_and_library_problem_copy_is_neutral_in_both_presentations() {
    for presentation in [Presentation::Standalone, Presentation::LtsEmbedded] {
        // an empty session has no library: Settings ▸ Import says its settings aren't kept
        let mut d = Driver::new(presentation);
        d.ok("engine.execute", json!({"command": "app.settings", "params": {"tab": "import"}}));
        for shown in ["Built-in default", "In-memory session: these settings last until you quit."] {
            assert!(d.text.contains(shown), "{presentation:?}: {shown:?} not in\n{}", d.text);
        }
        assert!(!d.text.contains("LightCraft"), "{presentation:?}:\n{}", d.text);
        let mut d = Driver::new(presentation);
        d.app.library_problem = Some(LibraryProblem::new("/photos/library", "it is open in another program"));
        d.frames(3);
        assert!(d.text.contains("The library couldn't be opened at") && d.text.contains("/photos/library"), "{}", d.text);
        assert!(!d.text.contains("LightCraft"), "{presentation:?}:\n{}", d.text);
        d.app.library_problem = Some(LibraryProblem::new("", "no home folder"));
        d.frames(3);
        assert!(d.text.contains("The library location couldn't be determined."), "{}", d.text);
        assert!(!d.text.contains("LightCraft"), "{presentation:?}:\n{}", d.text);
    }
}
