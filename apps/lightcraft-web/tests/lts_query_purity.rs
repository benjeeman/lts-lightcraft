//! F0 certification of the Light reads the LTS adapter uses for pre-checks and read-backs
//! (design §6.3; plan §1 F0: T-LI-01, T-LI-01b, T-LI-01c, T-LI-01d; INV-4, INV-14, INV-18, INV-38).
//!
//! The web build routes `lightcraft.command(id, params)` through its inbox to `LightcraftApp::run`,
//! the single funnel of menus, shortcuts, the control channel and JavaScript. These tests drive
//! that funnel natively on the demo library, in both presentations, with no browser, file, network
//! or model. They characterize the current fork: they pass on it, and a failure is a finding that
//! changes the L1 fallback.
//!
//! Declared residuals, which fix the order of the adapter's calls: an unknown id makes
//! `photo.inspect` and `history.list` refuse, and `run` copies the refusal to the status line;
//! `develop.get` answers `null` for an unknown id; an id that is not an unsigned integer is ignored
//! and the read answers for the active photo; while no photo is active, `history.list` and
//! `develop.set` (even with explicit `ids`) are refused; `develop.set {ids}` drops unknown ids
//! without a word and still commits, an empty batch when none is known.

use lightcraft_engine::catalog::{Op, PhotoId};
use lightcraft_engine::{Session, find_command};
use lightcraft_ui_egui::presentation::Presentation;
use lightcraft_ui_egui::{LightcraftApp, Services};
use serde_json::{Value, json};

/// Both presentations: the LTS host embeds the editor; the standalone app is upstream's control.
const PRESENTATIONS: [Presentation; 2] = [Presentation::Standalone, Presentation::LtsEmbedded];
/// A photo id the demo library never allocates (it numbers its photos from 1).
const UNKNOWN: u64 = 999_999;
/// The status line the setup leaves, so that a call which clears or rewrites it is caught.
const STATUS: &str = "status line before the reads";

/// The certified reads of design §6.3 with the parameters the L1 adapter builds (`id`: its target).
fn certified_queries(id: PhotoId) -> [(&'static str, Value); 10] {
    [
        ("library.state", json!({})),
        ("catalog.query", json!({"filter": {}, "limit": 200})),
        ("develop.get", json!({"id": id.0})),
        ("develop.controls", json!({})),
        ("photo.inspect", json!({"id": id.0})),
        ("history.list", json!({"id": id.0})),
        ("journal.list", json!({"limit": 1})),
        ("presets.list", json!({})),
        ("profiles.menu", json!({})),
        ("curve.presets", json!({})),
    ]
}

/// The demo library in the middle of an edit, so that "unchanged" is never vacuous: a mask and a
/// spot selected on the active photo, undo and redo steps, journal entries, a remembered export and
/// a status line. The persistence queue is drained.
fn editing_session(presentation: Presentation) -> LightcraftApp {
    let mut app = LightcraftApp::new(Session::with_demo(), Services::default()).with_presentation(presentation);
    for (id, params) in [
        ("mask.add", json!({"kind": "radial"})),
        ("spot.add", json!({"mode": "clone", "points": [[0.7, 0.3]], "source": [0.1, 0.0]})),
        ("develop.set", json!({"control": "light.exposure", "value": 0.5})),
        ("develop.set", json!({"control": "light.contrast", "value": 10})),
        ("edit.undo", json!({})),
    ] {
        app.run(id, params).unwrap_or_else(|e| panic!("{presentation:?} setup {id}: {e}"));
    }
    app.session.last_export = Some(json!({"format": "jpeg", "quality": 90, "metadata": "copyright"}));
    app.ui.status = STATUS.to_string();
    app.session.drain_log();
    let s = &app.session;
    assert!(s.active().is_some() && s.active_mask.is_some() && s.active_spot.is_some(), "setup selects a photo, a mask and a spot");
    assert!(!s.undo.is_empty() && !s.redo.is_empty() && !s.journal.is_empty(), "setup leaves undo, redo and journal entries");
    app
}

/// A demo photo other than `active`: reads and select-free edits must not need their target active.
fn other_photo(app: &LightcraftApp, active: PhotoId) -> PhotoId {
    app.session.catalog.photos().map(|p| p.id).find(|id| *id != active).expect("the demo library has more than one photo")
}

/// Selects nothing, so that no photo is active (an embedded library before the human picks one).
/// Leaving the edited photo may record an automatic version: that op belongs to the deselection,
/// not to the calls under test, so the persistence queue is drained here.
fn deselect_all(app: &mut LightcraftApp, presentation: Presentation) {
    assert_eq!(app.run("library.select", json!({"ids": []})), Ok(json!({"selected": 0})), "{presentation:?} deselect");
    assert!(app.session.active().is_none(), "{presentation:?}: no photo is active");
    app.session.drain_log();
}

/// The state a certified read must leave as it was, as named renderings: the status line (the
/// declared residual) apart, and everything else, from the catalog and the undo history to the
/// preferences and the UI state.
struct Witness {
    status: String,
    state: Vec<(&'static str, String)>,
}

impl Witness {
    fn of(app: &LightcraftApp) -> Self {
        let s = &app.session;
        let mut ui = app.ui.clone();
        let status = std::mem::take(&mut ui.status);
        let prefs = json!({
            "xmp": s.xmp,
            "exportPresets": s.export_presets,
            "metadataPresets": s.metadata_presets,
            "filterPresets": s.filter_presets,
            "curvePresets": s.curve_presets,
            "labelSets": s.label_sets,
            "lutProfiles": s.lut_profiles,
            "keywordSets": s.keyword_sets,
            "keywordSet": s.keyword_set,
            "recentKeywords": s.recent_keywords,
            "import": s.import_defaults,
            "cacheMb": s.cache_mb,
            "smartPreviewsDir": s.smart_previews_dir,
            "forgetLocalDays": s.forget_local_days,
        });
        let presets = json!({"presets": s.presets, "profileFavorites": s.profile_favorites, "profileRecent": s.profile_recent});
        let clipboards = json!({"settings": s.clipboard, "metadata": s.meta_clipboard, "copyGroups": s.copy_groups});
        let state = vec![
            ("catalog revision", s.catalog.revision.to_string()),
            ("catalog", format!("{:?}", s.catalog)),
            ("selection", format!("{:?}", s.selection)),
            ("active photo", format!("{:?}", s.active())),
            ("previous active photo", format!("{:?}", s.previous_active)),
            ("view", format!("{:?}", (&s.source, &s.filter, &s.sort, &s.browse))),
            ("active mask", format!("{:?}", s.active_mask)),
            ("active spot", format!("{:?}", s.active_spot)),
            ("interaction", format!("{:?}", s.interaction)),
            ("undo", format!("{:?}", s.undo)),
            ("redo", format!("{:?}", s.redo)),
            ("journal", format!("{:?}", s.journal)),
            ("before/after choices", format!("{:?}", s.before)),
            ("clipboards", clipboards.to_string()),
            ("last export", format!("{:?}", s.last_export)),
            ("library preferences", prefs.to_string()),
            ("presets and profiles", presets.to_string()),
            ("session flags", format!("{:?}", (s.auto_sync, s.target_album, s.preview_build.is_some(), s.library.is_some()))),
            ("UI state", format!("{ui:?}")),
        ];
        Self { status, state }
    }

    /// Asserts that nothing but (at most) the status line differs in `after`.
    fn assert_rest_unchanged(&self, after: &Self, what: &str) {
        for ((part, before), (_, now)) in self.state.iter().zip(&after.state) {
            assert!(before == now, "{what}: the {part} changed; {}", difference(before, now));
        }
    }
}

/// Where two renderings first differ, with some context (renderings can be long).
fn difference(before: &str, after: &str) -> String {
    let at = before.chars().zip(after.chars()).take_while(|(a, b)| a == b).count();
    let around = |s: &str| s.chars().skip(at.saturating_sub(60)).take(200).collect::<String>();
    format!("first difference at character {at}:\n  before: …{}…\n  after:  …{}…", around(before), around(after))
}

/// Runs each read twice and asserts that it answers, answers the same, and changes nothing at all:
/// no state, no status line, no op queued for persistence.
fn assert_pure(app: &mut LightcraftApp, context: &str, reads: impl IntoIterator<Item = (&'static str, Value)>) {
    for (id, params) in reads {
        let what = format!("{context}: {id} {params}");
        let before = Witness::of(app);
        let first = app.run(id, params.clone()).unwrap_or_else(|e| panic!("{what}: {e}"));
        let again = app.run(id, params).unwrap_or_else(|e| panic!("{what} (again): {e}"));
        let after = Witness::of(app);
        assert!(!first.is_null(), "{what} answers");
        assert_eq!(again, first, "{what} answers the same twice");
        assert_eq!(after.status, before.status, "{what}: the status line changed");
        before.assert_rest_unchanged(&after, &what);
        assert!(app.session.drain_log().is_empty(), "{what}: an op was queued for persistence");
    }
}

/// T-LI-01: each certified read is a registered, non-journaled query, enabled while a photo is
/// active. For the active photo and for another one, each answers the same twice and leaves the
/// catalog and its revision, the view and selection, the active mask and spot, undo and redo, the
/// journal, the preferences (`last_export` among them), the persistence queue, the UI state and the
/// status line as they were. With no photo active, every read but the gated `history.list`
/// (T-LI-01b) stays as pure.
#[test]
fn certified_queries_leave_catalog_selection_journal_undo_prefs_unchanged() {
    for presentation in PRESENTATIONS {
        let mut app = editing_session(presentation);
        let active = app.session.active().expect("the demo library opens with a photo active");
        let other = other_photo(&app, active);
        for (id, params) in certified_queries(active) {
            let spec = find_command(id).unwrap_or_else(|| panic!("{id} is a registered command"));
            assert!(!spec.journal, "{id} is a non-journaled query");
            assert_eq!((spec.enabled)(&app.session), Ok(()), "{id} {params} is enabled while a photo is active");
        }
        for target in [active, other] {
            assert_pure(&mut app, &format!("{presentation:?}, {active:?} active, target {target:?}"), certified_queries(target));
        }
        // no photo active: an embedded library before the human picks a photo
        deselect_all(&mut app, presentation);
        for target in [active, other] {
            let reads = certified_queries(target).into_iter().filter(|(id, _)| *id != "history.list");
            assert_pure(&mut app, &format!("{presentation:?}, no photo active, target {target:?}"), reads);
        }
    }
}

/// T-LI-01b (declared residual): an unknown photo id makes `photo.inspect` and `history.list`
/// refuse, `LightcraftApp::run` copies the refusal to the status line, and nothing else changes.
/// `develop.get` answers `null` instead, with no refusal. An id that is not an unsigned integer is
/// ignored: the reads answer for the active photo, not for the photo it names. Without an active
/// photo `history.list` refuses even a valid id the same way. Hence the adapter validates ids and
/// checks existence (`catalog.query`) and an active photo (`library.state`) before these reads.
#[test]
fn invalid_photo_id_query_only_sets_the_status_line() {
    for presentation in PRESENTATIONS {
        let mut app = editing_session(presentation);
        let active = app.session.active().expect("the demo library opens with a photo active");
        let other = other_photo(&app, active);
        for id in ["photo.inspect", "history.list"] {
            let params = json!({"id": UNKNOWN});
            let what = format!("{presentation:?} {id} {params}");
            let before = Witness::of(&app);
            let refusal = app.run(id, params).expect_err(&what);
            let after = Witness::of(&app);
            assert_eq!(refusal, format!("invalid parameters for `{id}`: no such photo"), "{what}");
            assert_eq!(after.status, refusal, "{what}: the refusal becomes the status line");
            before.assert_rest_unchanged(&after, &what);
            assert!(app.session.drain_log().is_empty(), "{what}: an op was queued for persistence");
        }

        let params = json!({"id": UNKNOWN});
        let what = format!("{presentation:?} develop.get {params}");
        let before = Witness::of(&app);
        assert_eq!(app.run("develop.get", params), Ok(Value::Null), "{what}: null, not a refusal");
        let after = Witness::of(&app);
        assert_eq!(after.status, before.status, "{what}: the status line changed");
        before.assert_rest_unchanged(&after, &what);

        for id in ["develop.get", "photo.inspect", "history.list"] {
            let for_active = app.run(id, json!({"id": active.0})).unwrap_or_else(|e| panic!("{presentation:?} {id} active: {e}"));
            let for_other = app.run(id, json!({"id": other.0})).unwrap_or_else(|e| panic!("{presentation:?} {id} other: {e}"));
            assert_ne!(for_active, for_other, "{presentation:?} {id}: the two photos answer differently");
            for ignored in [json!(other.0.to_string()), json!(-1), json!(1.5), Value::Null] {
                let params = json!({"id": ignored});
                let what = format!("{presentation:?} {id} {params}");
                let before = Witness::of(&app);
                assert_eq!(app.run(id, params), Ok(for_active.clone()), "{what} answers for the active photo");
                let after = Witness::of(&app);
                assert_eq!(after.status, before.status, "{what}: the status line changed");
                before.assert_rest_unchanged(&after, &what);
            }
        }

        deselect_all(&mut app, presentation);
        let params = json!({"id": active.0});
        let what = format!("{presentation:?} history.list {params} without an active photo");
        let before = Witness::of(&app);
        let refusal = app.run("history.list", params).expect_err(&what);
        let after = Witness::of(&app);
        assert_eq!(refusal, "command `history.list` is not available right now: no photo selected", "{what}");
        assert_eq!(after.status, refusal, "{what}: the refusal becomes the status line");
        before.assert_rest_unchanged(&after, &what);
        assert!(app.session.drain_log().is_empty(), "{what}: an op was queued for persistence");
    }
}

/// T-LI-01c: `develop.set` is gated on some photo being active even when explicit `ids` name its
/// target: with none active it is refused (the refusal becomes the status line; the target, undo
/// and journal are untouched), which is what the `library.state` pre-check (`no_active_photo`)
/// handles. With another photo active the same call edits the target only, without selecting it
/// or switching the selection, the active mask or the active spot.
#[test]
fn develop_set_with_explicit_ids_is_disabled_without_an_active_photo() {
    for presentation in PRESENTATIONS {
        let mut app = editing_session(presentation);
        let active = app.session.active().expect("the demo library opens with a photo active");
        let target = other_photo(&app, active);
        let edit = json!({"ids": [target.0], "values": {"light.exposure": 1.25}});

        deselect_all(&mut app, presentation);
        let state = app.run("library.state", json!({})).unwrap_or_else(|e| panic!("{presentation:?} library.state: {e}"));
        assert!(state["selection"]["active"].is_null(), "{presentation:?}: library.state shows no active photo: {state}");
        let what = format!("{presentation:?} develop.set {edit} without an active photo");
        let before = Witness::of(&app);
        let refusal = app.run("develop.set", edit.clone()).expect_err(&what);
        let after = Witness::of(&app);
        assert_eq!(refusal, "command `develop.set` is not available right now: no photo selected", "{what}");
        assert_eq!(after.status, refusal, "{what}: the refusal becomes the status line");
        before.assert_rest_unchanged(&after, &what);
        assert!(app.session.drain_log().is_empty(), "{what}: an op was queued for persistence");

        let mut app = editing_session(presentation);
        let what = format!("{presentation:?} develop.set {edit} with {active:?} active");
        let selection = app.session.selection.clone();
        let picked = (app.session.active_mask, app.session.active_spot);
        let active_settings = app.session.develop_of(active);
        let (undo, journal) = (app.session.undo.len(), app.session.journal.len());
        assert_eq!(app.run("develop.set", edit.clone()), Ok(Value::Null), "{what}");
        assert_eq!(app.session.develop_of(target).map(|d| d.light.exposure), Some(1.25), "{what}: the target is edited");
        assert_eq!(app.session.develop_of(active), active_settings, "{what}: the active photo is not edited");
        assert_eq!(app.session.selection, selection, "{what}: the selection is kept");
        assert_eq!((app.session.active_mask, app.session.active_spot), picked, "{what}: the active mask and spot are kept");
        assert_eq!((app.session.undo.len(), app.session.journal.len()), (undo + 1, journal + 1), "{what}: one undo step, one journal entry");
        assert_eq!(app.ui.status, STATUS, "{what}: no refusal");
    }
}

/// T-LI-01d: `develop.set {ids}` silently drops the ids of photos that do not exist and still
/// commits: with a known and an unknown id it edits the known photo in one undo step; with only
/// unknown ids it answers success and commits an empty batch (a catalog revision, an undo step, a
/// journal entry and a queued op that change no photo). Neither sets the status line. Hence the
/// `catalog.query` existence pre-check (`target_not_found`) and the `develop.get` read-back.
#[test]
fn develop_set_ids_drops_unknown_ids_and_still_commits() {
    for presentation in PRESENTATIONS {
        let mut app = editing_session(presentation);
        let active = app.session.active().expect("the demo library opens with a photo active");
        let target = other_photo(&app, active);
        let photos = |a: &LightcraftApp| a.session.catalog.photos().cloned().collect::<Vec<_>>();
        let active_settings = app.session.develop_of(active);
        let revision = app.session.catalog.revision;
        let (undo, journal) = (app.session.undo.len(), app.session.journal.len());

        let mixed = json!({"ids": [target.0, UNKNOWN], "values": {"light.exposure": 1.5}});
        let what = format!("{presentation:?} develop.set {mixed}");
        assert_eq!(app.run("develop.set", mixed.clone()), Ok(Value::Null), "{what}");
        assert_eq!(app.session.develop_of(target).map(|d| d.light.exposure), Some(1.5), "{what}: the known photo is edited");
        assert_eq!(app.session.develop_of(active), active_settings, "{what}: the active photo is not edited");
        assert!(app.session.catalog.photo(PhotoId(UNKNOWN)).is_none(), "{what}: the unknown id stays unknown");
        assert_eq!(app.session.catalog.revision, revision + 1, "{what}: one catalog step");
        assert_eq!(app.session.undo.len(), undo + 1, "{what}: one undo step");
        assert_eq!(app.session.undo.last().map(|u| u.label.as_str()), Some("Exposure"), "{what}: undo label");
        assert!(app.session.redo.is_empty(), "{what}: the commit clears redo");
        assert_eq!(app.session.journal.len(), journal + 1, "{what}: one journal entry");
        let (command, params) = app.session.journal.last().expect("a journal entry");
        assert_eq!((command.as_str(), params), ("develop.set", &mixed), "{what}: journaled as called");
        assert_eq!(app.ui.status, STATUS, "{what}: no refusal");

        let unknown = json!({"ids": [UNKNOWN], "values": {"light.exposure": 2.5}});
        let what = format!("{presentation:?} develop.set {unknown}");
        let unchanged = photos(&app);
        assert_eq!(app.run("develop.set", unknown), Ok(Value::Null), "{what}: success, not a refusal");
        assert!(photos(&app) == unchanged, "{what}: no photo changed");
        assert_eq!(app.session.catalog.revision, revision + 2, "{what}: still a catalog step");
        assert_eq!(app.session.undo.len(), undo + 2, "{what}: still an undo step");
        let step = app.session.undo.last().expect("an undo step");
        assert_eq!(step.label, "Exposure", "{what}: undo label");
        assert!(matches!(&step.op, Op::Batch { ops } if ops.is_empty()), "{what}: the undo step is an empty batch: {:?}", step.op);
        assert_eq!(app.session.journal.len(), journal + 2, "{what}: still a journal entry");
        assert_eq!(app.ui.status, STATUS, "{what}: no refusal");
        assert_eq!(app.session.drain_log().len(), 2, "{what}: both commits queued an op for persistence, the second an empty batch");
    }
}
