//! FL1, R0 and R1: the Light hook's `lts.*` commands (design Appendix A.2; plan §1 FL1: T-LI-02 to
//! T-LI-12 and T-LI-14; master plan v2 §3.1 R0: T-LI-15 to T-LI-20; master plan v3 W3 R1: T-LI-21 to
//! T-LI-28; INV-12, INV-13, INV-14, INV-18, INV-19, INV-22, INV-28, INV-33, INV-34, INV-41, INV-58).
//!
//! The web build routes `lightcraft.command(id, params)` through its inbox, which hands every id to
//! `lightcraft_web::lts_view::handle(&mut app, id, &params)` before `web.stats` and `app.run`. The
//! hook answers an id that starts with `lts.`, and in the LTS presentation (R1) every other id too,
//! with a refusal: such a call never reaches the command registry, so it is never journaled and never
//! writes the status line. These tests drive `handle` natively on the page's own boot (the library
//! opened in an in-memory store, as the LTS lab runs `?store=memory`), rendering on the CPU like the
//! browser build, with no browser, file, network or model.
//!
//! The contract the LTS adapter maps exactly (FL3):
//!
//! - `handle` answers `None` for any other id in the standalone presentation, where the page runs it
//!   as upstream does; in the LTS presentation it refuses it (`command_not_allowed`, R1). For an
//!   `lts.` id it answers `Some(Ok(reply))` for every typed outcome, and `Some(Err(text))` only when a
//!   mutation's engine call failed after it started (the adapter reports that as `unknown`).
//! - Every reply has `resultVersion` (`lightcraft.lts-view.v1`), `command` (the id) and `outcome`:
//!   `ok` (a read answered), `applied` (a mutation committed as one undo step) or `not_applied`
//!   (refused before anything changed, with a registered `code` and a `message`; a conflict also
//!   answers `actual`, the current hash or head to refresh from). An unknown `lts.` id is
//!   `command_not_allowed`. Parameters are closed objects: an unknown key, a wrong type or a photo id
//!   that is not an explicit integer from 1 is `invalid_params` (never "the active photo"); an
//!   unknown photo or mask is `target_not_found`.
//! - Reads leave the catalog, the selection, the undo history, the journal, the preferences
//!   (`last_export` among them), the UI state, the status line and the library store as they were;
//!   at most a decoded source enters the media cache.
//! - `lts.state` takes an optional `id` and answers `photoId` (the id, else the active photo, else
//!   null), `hash` (that photo's settings `hash64` as 16 lowercase hex digits), `catalogRevision`,
//!   `activePhoto`, `interaction`, `historyHead` and `redoHead` (length and label of the undo and redo
//!   stacks); with `settings: true` (R1) also `settings`, the complete settings `develop.get` gives,
//!   which that `hash` names.
//! - `lts.preview` takes `id`, `longEdge` (16 to 2048), `variant` (`current` by default, `before`, or
//!   `settings` with either `values` or a complete `settings`), `crop` (`applied` by default or
//!   `uncropped`), `overlay` (a `maskId` and a `view`, `whiteOnBlack` by default; or `spots: true`
//!   and a `threshold`, 50 by default: Visualize Spots) and `pixels` (true by default). It answers
//!   the engine's own render (`render_job`, `preview_job`, `with_overlay`) as
//!   `pngBase64` (standard base64 of a PNG without metadata) with `mime`, `width`, `height`,
//!   `variant`, `crop`, `frame` (`cropped-straightened` or `uncropped-oriented`), `hash`,
//!   `renderedHash` (the settings rendered), `catalogRevision`, `histogram` (the engine histogram of
//!   the image in 64 bins of 4 levels), `total`, `stats` (`meanLuma`, `p01`, `p50`, `p99`, `clipLo`,
//!   `clipHi`) and, for a mask overlay, `maskCoverage` (`maskId`, `area`, `bbox`, `centroid`).
//! - `lts.export` takes `id`, `format` (`jpeg` or `png`), `longEdge` (16 to 4096), `quality`,
//!   `metadata` (`none`, `copyright` or `allExceptCamera`; `copyright` by default, owner decision
//!   OC-13) and `removeLocation` (true by default). It answers `bytesBase64`, `mime`, `width`,
//!   `height`, `byteLength`, `metadata`, `removeLocation`, `hash` and `catalogRevision`; it never
//!   selects and never writes a file or `last_export`.
//! - Mutations run as one engine command (`execute_fn`, not journaled), refuse `editor_busy` while a
//!   human interaction is in progress and check `expectHash` (`settings_hash_conflict`) and
//!   `expectHistoryHead` (`history_head_conflict`) in that same command; they never select and
//!   answer `photoId`, `label`, `hashBefore`, `hashAfter`, `catalogRevision` and `historyHead`.
//! - `lts.develop` takes `id`, either `values` (1 to 64 controls) or a complete `settings` JSON, and
//!   `expectHash` (optional, but required with complete settings since R1), and optional
//!   `expectHistoryHead` and `label`. It applies values exactly as `develop.set` does (the as-shot
//!   white balance rule included; a value outside its control's range is `out_of_range`) and adds
//!   `delta`, `intentSatisfied` and `wbModeChanged`.
//! - `lts.crop` takes `id`, a `rect` and an `angle` or `reset`, and optional `expectHash` and
//!   `expectHistoryHead`. It applies the geometry `crop.set` or `crop.reset` would and answers it as
//!   `crop` (`rect`, `angle`) with `intentSatisfied`, or answers `hook_unavailable` for every crop.
//! - `lts.undo` (`id`, `expectHash`, `expectHistoryHead`) and `lts.redo` (`id`, `expectHash`,
//!   `expectRedoHead`) move the top step only when it is that photo's alone and both guards hold;
//!   they also answer `redoHead`.
//! - R0 (master plan v2 §3.1, T-LI-15 to T-LI-20): `settings` (in `lts.develop`, and in
//!   `lts.preview` variant `settings`) is also a patch of whole sections, only of `masks`, `spots`,
//!   `red_eye`, `curve`, `wb`, `profile` and `treatment` (the keys of the settings' serialization
//!   that no control value reaches, each section exactly as `develop.get` writes it), and the reply
//!   names them (`sections`); `preset` is `{id, amount?}`, the keys of `preset.apply`. What complete
//!   settings, a patch or a preset would commit is held, on every part it changes, to what the
//!   editor's own commands and tools leave: a value they would clamp, a count past its cap or a
//!   mask id past 65535 is `out_of_range`. Every change, `values` and `lts.crop` included, is also
//!   held at the crop's magnification (one over the smaller side of the crop the renderer shows,
//!   counted as anything from 0.02 of the crop to all of it where the renderer solves an Upright
//!   transform from the pixels under Constrain Crop; the crop is tighter when that or the
//!   renderer's own magnification grows): the crop shown, heal and remove spots, Auto Mask strokes,
//!   each blur the renderer derives from the settings (their profile's look included) and
//!   defringe's filter, the last two compared at the renderer's own magnification (`out_of_range`),
//!   and brush and spot work counted in output planes (`budget_exceeded`). So is Visualize Spots.
//!   `lts.restoreSettings` (`id`, `revision`, `expectHash`, and optional `expectHistoryHead` and
//!   `label`) puts back the settings of an earlier state that the photo's history recorded, as
//!   `history.restore` does. Presets and sync take no `lts.*` id of their own.
//!   The table of commands (`POLICY`) says where each may run (`native_only` in the browser for one
//!   that only a native worker runs; none today), each runtime has its own render bounds, and a
//!   natively hosted editor keeps these edits in the library its host opened in the workspace and
//!   writes no XMP sidecar.
//! - R1 (master plan v3 W3, T-LI-21 to T-LI-28; backlog items 2, 16 and 22): the fork's entry point
//!   refuses every id outside the hook's table. The page hands every id `lightcraft.command` receives
//!   to `handle` first, and in the LTS presentation `handle` answers every id that does not start with
//!   `lts.` with `command_not_allowed` and runs nothing; the standalone presentation stays upstream's.
//!   The certified engine reads the adapter and the byte bridge sent are reads of the table:
//!   `lts.library` (the photos, as `catalog.query` lists them), `lts.photo` (`photo.inspect`, with the
//!   token of each history step), `lts.state {settings: true}` (`develop.get`), `lts.controls` (the
//!   control catalog, its gates labelled), `lts.presets` (`presets.list`, `profiles.menu`,
//!   `curve.presets`), `lts.metadata` (`photo.allMetadata`) and `lts.journal` (`journal.list`); the
//!   declared selection (`library.select` and `view.detail`) is `lts.select`. Every reply that names
//!   a photo's settings names the fork's token of exactly those settings (one revision token), and
//!   the sharpening band that bug-fix batch 2 added is held at the crop's magnification with the
//!   renderer's other blurs. The control catalog also labels the controls that act only through
//!   another setting (T-LI-27). Complete settings need `expectHash`, and a checkout of a person's
//!   state past the hook's bounds is refused while `lts.restoreSettings` puts it back (T-LI-28).

use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use lightcraft_codecs::{DecodeOptions, EncodeImage, EncodeMeta};
use lightcraft_engine::Session;
use lightcraft_engine::catalog::{FsStore, Photo, PhotoId, Source};
use lightcraft_engine::cmd::lut_profiles::LutProfile;
use lightcraft_engine::develop::{CONTROLS, DevelopSettings, Mask, PointColor, Preset, RedEye, SettingsGroup, Spot, Upright, WbMode};
use lightcraft_engine::export::{ExportFormat, ExportOptions, MetadataPolicy, Resize, export_photo};
use lightcraft_engine::library::LibraryStores;
use lightcraft_engine::pipeline::{MaskView, Overlay};
use lightcraft_raster::{Histogram, Rgba8};
use lightcraft_ui_egui::presentation::Presentation;
use lightcraft_ui_egui::{LightcraftApp, Services};
use lightcraft_web::files::Files;
use lightcraft_web::lts_view::{Entry, POLICY, Runtime, Runtimes, handle, handle_by, handle_in};
use lightcraft_web::options::Options;
use lightcraft_web::store::{Originals, content_hash};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

/// The version of every reply; the adapter accepts exactly this one.
const VERSION: &str = "lightcraft.lts-view.v1";
/// Both presentations: the LTS host embeds the editor; the standalone app is upstream's control.
const PRESENTATIONS: [Presentation; 2] = [Presentation::Standalone, Presentation::LtsEmbedded];
/// A photo id the libraries here never allocate.
const UNKNOWN: u64 = 999_999;
/// The status line the setup leaves: an `lts.*` call never rewrites it, not even to refuse.
const STATUS: &str = "status line before the lts calls";
/// The codes a refusal may carry (design Appendix A.2 and C; `native_only`: media-agent contracts
/// §9.9): a closed list.
const CODES: [&str; 12] = [
    "invalid_params",
    "command_not_allowed",
    "target_not_found",
    "settings_hash_conflict",
    "history_head_conflict",
    "editor_busy",
    "out_of_range",
    "budget_exceeded",
    "hook_unavailable",
    "decode_failed",
    "source_loading",
    "native_only",
];
/// The reads (answered `ok`) and the mutations (answered `applied`), in the order of the hook's table:
/// R1 adds the reads that replace the certified engine reads (T-LI-22) and the declared selection.
const READS: [&str; 9] =
    ["lts.state", "lts.preview", "lts.export", "lts.library", "lts.photo", "lts.controls", "lts.presets", "lts.metadata", "lts.journal"];
const MUTATIONS: [&str; 6] = ["lts.develop", "lts.crop", "lts.undo", "lts.redo", "lts.restoreSettings", "lts.select"];
/// The gates of the control catalog (R1, backlog item 2), in the catalog's order: the states in which
/// a control changes nothing. The four section switches the renderer honours, B&W and colour, a
/// profile with a look, the lens corrections a file embeds, and a negative vignette in Highlight
/// Priority; then the settings other controls act through (T-LI-27): colour or a B&W mix, a
/// tone-curve region slider and any curve, the colour-grading zones and each wheel's saturation, the
/// vignette, grain, sharpening, noise reduction, lens vignetting and defringe amounts, and a point
/// colour's shift.
const GATES: [&str; 27] = [
    "effectsOn",
    "opticsOn",
    "geometryOn",
    "calibrationOn",
    "bw",
    "color",
    "profileLook",
    "lensDistortion",
    "lensVignetting",
    "highlightPriority",
    "colorOrBwMix",
    "curveRegions",
    "curveOn",
    "gradingZones",
    "shadowsTint",
    "midtonesTint",
    "highlightsTint",
    "globalTint",
    "vignetteOn",
    "grainOn",
    "sharpenOn",
    "nrLuminanceOn",
    "nrColorOn",
    "manualVignetting",
    "defringePurple",
    "defringeGreen",
    "pointColorShift",
];
/// The gates open on a plain demo photo: colour, every section on, no look, no lens data, and no
/// amount, curve, wheel or point colour that other controls act through.
const PLAIN_OPEN: [&str; 6] = ["effectsOn", "opticsOn", "geometryOn", "calibrationOn", "color", "colorOrBwMix"];
/// EXIF IFD0 tags: the GPS IFD pointer, the camera model and the copyright notice.
const GPS_IFD: u16 = 0x8825;
const MODEL: u16 = 0x0110;
const COPYRIGHT_TAG: u16 = 0x8298;
/// The fixture's notice, caption and position (ASCII, as EXIF text is).
const COPYRIGHT: &str = "Copyright LTS qualification fixture 2026";
const CAPTION: &str = "Caption of the LTS qualification fixture";
const GPS: [f64; 2] = [48.8584, 2.2945];
/// The sections a `settings` patch sets (R0): keys of the settings' own serialization
/// (`develop.get`, `crates/develop/src/settings.rs:11-41`) that no control value reaches.
const SECTIONS: [&str; 7] = ["masks", "spots", "red_eye", "curve", "wb", "profile", "treatment"];
/// A library's own files (`crates/engine/src/library.rs:1-11`): the catalog journal, presets,
/// preferences and view state.
const LIBRARY_FILES: [&str; 5] = ["catalog.snap", "catalog.log", "presets.json", "prefs.json", "view.json"];

// ------------------------------------------------------------------------------------------ the page

/// The page's boot (`web.rs`): a new session with the originals in memory, the library opened in an
/// in-memory store, the editor around the session. `files` sees every write the engine makes to the
/// library (catalog, preferences, presets, view).
struct Lab {
    app: LightcraftApp,
    files: Files,
    originals: Originals,
}

/// Boots like the page, with or without the demo photos, rendering on the CPU as the browser build
/// does (it has no GPU path).
fn boot(presentation: Presentation, seed_demo: bool) -> Lab {
    lightcraft_engine::gpu::set_enabled(false);
    let originals = Originals::default();
    let files = Files::default();
    let mut session = Session::new();
    originals.install(&mut session);
    let stores =
        LibraryStores { dir: "browser:memory/library".into(), catalog: Box::new(files.store()), files: Box::new(files.store()), on_disk: false };
    session.open_library_in(stores, seed_demo).unwrap_or_else(|e| panic!("opening the library: {e}"));
    files.take_dirty();
    let app = LightcraftApp::new(session, Services::default()).with_presentation(presentation);
    Lab { app, files, originals }
}

/// The demo library in the middle of an edit, so that "unchanged" is never vacuous: a mask and a
/// spot selected on the active photo, undo and redo steps, journal entries, a remembered export and
/// a status line, all written to the library store.
fn editing_lab(presentation: Presentation) -> Lab {
    let mut lab = boot(presentation, true);
    for (id, params) in [
        ("mask.add", json!({"kind": "radial"})),
        ("spot.add", json!({"mode": "clone", "points": [[0.7, 0.3]], "source": [0.1, 0.0]})),
        ("develop.set", json!({"control": "light.exposure", "value": 0.5})),
        ("develop.set", json!({"control": "light.contrast", "value": 10})),
        ("edit.undo", json!({})),
    ] {
        lab.app.run(id, params).unwrap_or_else(|e| panic!("{presentation:?} setup {id}: {e}"));
    }
    lab.app.session.last_export = Some(json!({"format": "jpeg", "quality": 90, "metadata": "copyright"}));
    lab.app.ui.status = STATUS.to_string();
    let s = &lab.app.session;
    assert!(s.active().is_some() && s.active_mask.is_some() && s.active_spot.is_some(), "setup selects a photo, a mask and a spot");
    assert!(!s.undo.is_empty() && !s.redo.is_empty() && !s.journal.is_empty(), "setup leaves undo, redo and journal entries");
    settle(&mut lab);
    lab
}

/// Forgets what a setup step queued or wrote, so that the calls under test start from a clean store.
fn settle(lab: &mut Lab) {
    lab.app.session.drain_log();
    lab.files.take_dirty();
}

/// The names of the library files written since the last look.
fn written(lab: &Lab) -> Vec<String> {
    lab.files.take_dirty().iter().map(|op| op.name().to_string()).collect()
}

fn active_photo(app: &LightcraftApp) -> PhotoId {
    app.session.active().expect("the demo library opens with a photo active")
}

/// The photo with the smallest id other than `active`: reads and select-free edits must not need
/// their target active.
fn other_photo(app: &LightcraftApp, active: PhotoId) -> PhotoId {
    app.session.catalog.photos().map(|p| p.id).filter(|id| *id != active).min().expect("the demo library has more than one photo")
}

/// Selects nothing, so that no photo is active (an embedded library before the human picks one).
/// Leaving the edited photo may record an automatic version: that belongs to the deselection.
fn deselect_all(lab: &mut Lab) {
    assert_eq!(lab.app.run("library.select", json!({"ids": []})), Ok(json!({"selected": 0})), "deselect");
    assert!(lab.app.session.active().is_none(), "no photo is active");
    settle(lab);
}

/// Imports `bytes` as the page imports a picked or dropped file: kept with the originals in memory,
/// then `library.import` through the editor.
fn import(lab: &mut Lab, name: &str, bytes: &[u8]) -> PhotoId {
    let path = lab.originals.added(name, &content_hash(bytes), Arc::from(bytes));
    let report = lab.app.run("library.import", json!({"paths": [path]})).unwrap_or_else(|e| panic!("importing {name}: {e}"));
    let id = report["imported"][0].as_u64().unwrap_or_else(|| panic!("{name} was imported: {report}"));
    settle(lab);
    PhotoId(id)
}

/// A small PNG as a person would pick it (a gradient, the engine's own encoder).
fn picked_png() -> Vec<u8> {
    let image = Rgba8::from_fn(48, 32, |x, y| [(x * 5) as u8, (y * 8) as u8, 160, 255]);
    lightcraft_codecs::encode_png(&EncodeImage::rgba8(&image), &EncodeMeta::default()).expect("encoding a PNG")
}

// ---------------------------------------------------------------------------------------- the state

/// `hash64` in 16 lowercase hex digits: the hook's settings token.
fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// A photo's settings token as the hook writes it.
fn hash_of(app: &LightcraftApp, id: PhotoId) -> String {
    hex(app.session.develop_of(id).unwrap_or_else(|| panic!("{id:?} is in the catalog")).hash64())
}

/// Whether a reply value is a settings token: 16 lowercase hex digits.
fn is_hex16(value: &Value) -> bool {
    value.as_str().is_some_and(|s| s.len() == 16 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

/// `{length, label}` of the undo stack, as `historyHead`.
fn history_head(app: &LightcraftApp) -> Value {
    let undo = &app.session.undo;
    json!({"length": undo.len(), "label": undo.last().map(|step| step.label.as_str())})
}

/// `{length, label}` of the redo stack, as `redoHead`.
fn redo_head(app: &LightcraftApp) -> Value {
    let redo = &app.session.redo;
    json!({"length": redo.len(), "label": redo.last().map(|step| step.label.as_str())})
}

/// A copy of a photo of the catalog.
fn photo(app: &LightcraftApp, id: PhotoId) -> Photo {
    app.session.catalog.photo(id).map(|p| (**p).clone()).unwrap_or_else(|| panic!("{id:?} is in the catalog"))
}

/// Every photo of the catalog, in id order.
fn all_photos(app: &LightcraftApp) -> Vec<Photo> {
    let mut photos: Vec<Photo> = app.session.catalog.photos().map(|p| (**p).clone()).collect();
    photos.sort_by_key(|p| p.id);
    photos
}

fn photos_except(app: &LightcraftApp, id: PhotoId) -> Vec<Photo> {
    all_photos(app).into_iter().filter(|p| p.id != id).collect()
}

/// Asserts that two lists of photos are equal, naming the first photo that differs.
fn assert_same_photos(a: &[Photo], b: &[Photo], what: &str) {
    let first = a.iter().zip(b).find(|(x, y)| x != y).map(|(x, _)| x.id);
    assert!(a.len() == b.len() && first.is_none(), "{what}: the photos differ (first: {first:?})");
}

/// Puts `id` in As Shot white balance with a stored temperature and tint that are not its as-shot
/// values (6500 K and 0 for a demo photo), as an ordinary undo step.
fn stale_as_shot(lab: &mut Lab, id: PhotoId) {
    let session = &mut lab.app.session;
    let mut settings = Arc::unwrap_or_clone(session.develop_of(id).expect("the photo"));
    settings.wb.mode = WbMode::AsShot;
    settings.wb.temp = 4000.0;
    settings.wb.tint = 10.0;
    let op = session.develop_op(id, settings, "White Balance").expect("an op for the photo");
    session.commit("White Balance", op).unwrap_or_else(|e| panic!("the fixture: {e}"));
    settle(lab);
}

/// Everything an `lts.*` read, and any refusal, must leave as it was, as named renderings.
struct Witness(Vec<(&'static str, String)>);

impl Witness {
    fn of(app: &LightcraftApp) -> Self {
        let s = &app.session;
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
        let rendered = (s.media.rendered.mem_usage(), s.media.rendered.generation());
        Witness(vec![
            ("status line", app.ui.status.clone()),
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
            ("UI state", format!("{:?}", app.ui)),
            ("render queue", format!("{:?}", (app.renderer.queued(), app.renderer.in_flight()))),
            ("rendered previews", format!("{rendered:?}")),
        ])
    }
}

/// Where two renderings first differ, with some context (renderings can be long).
fn difference(before: &str, after: &str) -> String {
    let at = before.chars().zip(after.chars()).take_while(|(a, b)| a == b).count();
    let around = |s: &str| s.chars().skip(at.saturating_sub(60)).take(200).collect::<String>();
    format!("first difference at character {at}:\n  before: …{}…\n  after:  …{}…", around(before), around(after))
}

/// Asserts that every part of `before` but the `changing` ones is as it was.
fn assert_kept(app: &LightcraftApp, before: &Witness, changing: &[&str], what: &str) {
    let now = Witness::of(app);
    for ((part, was), (_, is)) in before.0.iter().zip(&now.0) {
        assert!(changing.contains(part) || was == is, "{what}: the {part} changed; {}", difference(was, is));
    }
}

/// Asserts that nothing in `before` changed, that nothing was queued for the library and that
/// nothing was written to the library store.
fn assert_unchanged(lab: &mut Lab, before: &Witness, what: &str) {
    assert_kept(&lab.app, before, &[], what);
    assert!(lab.app.session.drain_log().is_empty(), "{what}: an op was queued for the library");
    let files = written(lab);
    assert!(files.is_empty(), "{what}: wrote {files:?} to the library store");
}

// ------------------------------------------------------------------------------------------ the hook

/// Runs an `lts.*` command as the page's inbox does and checks the reply's envelope.
fn lts(app: &mut LightcraftApp, id: &str, params: Value) -> Value {
    let what = format!("{id} {params}");
    let reply = handle(app, id, &params).unwrap_or_else(|| panic!("{what}: lts_view handles every `lts.` id"));
    let reply = reply.unwrap_or_else(|e| panic!("{what}: rejected with untyped text ({e}); every typed outcome resolves"));
    check_envelope(id, &reply, &what);
    reply
}

/// [`lts`] through `handle_in` in `runtime`: the entry a native worker calls, and the browser's
/// runtime driven natively.
fn lts_in(app: &mut LightcraftApp, id: &str, params: Value, runtime: Runtime) -> Value {
    let what = format!("{id} {params} in {runtime:?}");
    let reply = handle_in(&mut app.session, id, &params, runtime).unwrap_or_else(|| panic!("{what}: lts_view handles every `lts.` id"));
    let reply = reply.unwrap_or_else(|e| panic!("{what}: rejected with untyped text ({e}); every typed outcome resolves"));
    check_envelope(id, &reply, &what);
    reply
}

/// [`lts_in`] through `handle_by` with the command table `table` in place of the hook's.
fn lts_by(table: &[Entry], app: &mut LightcraftApp, id: &str, params: Value, runtime: Runtime) -> Value {
    let what = format!("{id} {params} in {runtime:?}, by a table of the test's");
    let reply = handle_by(table, &mut app.session, id, &params, runtime).unwrap_or_else(|| panic!("{what}: the hook handles every `lts.` id"));
    let reply = reply.unwrap_or_else(|e| panic!("{what}: rejected with untyped text ({e}); every typed outcome resolves"));
    check_envelope(id, &reply, &what);
    reply
}

/// The envelope every reply has (see the module documentation).
fn check_envelope(id: &str, reply: &Value, what: &str) {
    assert_eq!(reply["resultVersion"], VERSION, "{what}: {reply}");
    assert_eq!(reply["command"], id, "{what}: the reply names its command: {reply}");
    let no_code = reply.get("code").is_none_or(Value::is_null);
    match reply["outcome"].as_str() {
        Some("ok") => assert!(READS.contains(&id) && no_code, "{what}: `ok` answers a read, without a code: {reply}"),
        Some("applied") => assert!(MUTATIONS.contains(&id) && no_code, "{what}: `applied` answers a mutation, without a code: {reply}"),
        Some("not_applied") => {
            let code = reply["code"].as_str().unwrap_or_default();
            assert!(CODES.contains(&code), "{what}: `{code}` is a registered code: {reply}");
            assert!(reply["message"].as_str().is_some_and(|m| !m.is_empty()), "{what}: a refusal says why: {reply}");
            assert!(reply.get("pngBase64").is_none() && reply.get("bytesBase64").is_none(), "{what}: a refusal carries no bytes");
        }
        _ => panic!("{what}: the outcome is ok, applied or not_applied: {reply}"),
    }
}

/// The code of a typed refusal (and checks that the reply is one).
fn refusal(reply: &Value) -> &str {
    assert_eq!(reply["outcome"], "not_applied", "a typed refusal: {reply}");
    reply["code"].as_str().unwrap_or_default()
}

/// `base` with the keys of `extra` added or replaced.
fn with(base: &Value, extra: Value) -> Value {
    let mut params = base.clone();
    if let (Some(object), Value::Object(extra)) = (params.as_object_mut(), extra) {
        object.extend(extra);
    }
    params
}

/// `lts.preview` of `id` at 256 pixels with `extra` parameters.
fn preview(app: &mut LightcraftApp, id: PhotoId, extra: Value) -> Value {
    lts(app, "lts.preview", with(&json!({"id": id.0, "longEdge": 256}), extra))
}

/// `lts.state` of a photo.
fn state(app: &mut LightcraftApp, id: PhotoId) -> Value {
    lts(app, "lts.state", json!({"id": id.0}))
}

/// A preview's mean luma (0 to 1).
fn luma(reply: &Value) -> f64 {
    reply["stats"]["meanLuma"].as_f64().unwrap_or_else(|| panic!("stats.meanLuma: {reply}"))
}

/// The engine's own render of `id` fitting `long` × `long` with its crop (`render_job`, as the loupe
/// and exports render): what `lts.preview` must return.
fn reference(app: &mut LightcraftApp, id: PhotoId, long: usize, before: bool, overlay: Overlay) -> Rgba8 {
    let job = app.session.render_job(id, long, long, before, true).expect("a render job").with_overlay(overlay);
    job.run().rendered.unwrap_or_else(|e| panic!("the reference render: {e}")).image
}

/// The engine's render of `id` with `settings` in place of its own (`preview_job`).
fn reference_with(app: &mut LightcraftApp, id: PhotoId, long: usize, settings: &DevelopSettings) -> Rgba8 {
    let job = app.session.preview_job(id, long, long, true, settings).expect("a preview job");
    job.run().rendered.unwrap_or_else(|e| panic!("the reference render: {e}")).image
}

// ----------------------------------------------------------------------------------------- the bytes

/// Standard base64 (RFC 4648 section 4: `+`, `/`, `=` padding, no line breaks), read strictly.
fn base64(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    assert!(text.len().is_multiple_of(4), "base64 comes in padded groups of four");
    let groups = text.len() / 4;
    let mut out = Vec::with_capacity(groups * 3);
    for (i, group) in text.as_bytes().chunks(4).enumerate() {
        let pad = group.iter().rev().take_while(|c| **c == b'=').count();
        assert!(pad <= 2 && (pad == 0 || i + 1 == groups), "base64 padding only ends the text");
        let mut n = 0_u32;
        for &c in &group[..4 - pad] {
            let v = ALPHABET.iter().position(|a| *a == c).unwrap_or_else(|| panic!("{:?} is not a base64 character", c as char));
            n = (n << 6) | v as u32;
        }
        n <<= 6 * pad as u32;
        out.extend_from_slice(&n.to_be_bytes()[1..4 - pad]);
    }
    out
}

/// The chunk types of a PNG, in order, after checking its signature and that its chunks fill it.
fn png_chunks(png: &[u8]) -> Vec<[u8; 4]> {
    assert_eq!(png.get(..8), Some(&b"\x89PNG\r\n\x1a\n"[..]), "a PNG signature");
    let mut chunks = Vec::new();
    let mut at = 8;
    while let Some(head) = png.get(at..at + 8) {
        let length = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
        chunks.push([head[4], head[5], head[6], head[7]]);
        at += 12 + length;
    }
    assert_eq!(at, png.len(), "the chunks end with the file");
    chunks
}

/// 8-bit sRGB pixels of encoded image bytes, read by the engine's decoder.
fn decode_srgb8(bytes: &[u8], what: &str) -> Rgba8 {
    lightcraft_codecs::decode(bytes, DecodeOptions::default()).unwrap_or_else(|e| panic!("{what}: decoding the image: {e}")).to_srgb8()
}

/// The pixels of a preview's PNG, after checking its encoding: strict standard base64, a whole PNG
/// without text or EXIF chunks, and the reply's own dimensions.
fn preview_pixels(reply: &Value, what: &str) -> Rgba8 {
    assert_eq!(reply["mime"], "image/png", "{what}: {reply}");
    let png = base64(reply["pngBase64"].as_str().unwrap_or_else(|| panic!("{what}: a preview with pixels carries pngBase64")));
    let chunks = png_chunks(&png);
    for kind in [*b"tEXt", *b"zTXt", *b"iTXt", *b"eXIf"] {
        assert!(!chunks.contains(&kind), "{what}: the preview carries a {} chunk", String::from_utf8_lossy(&kind));
    }
    let image = decode_srgb8(&png, what);
    assert!(reply["width"] == image.width && reply["height"] == image.height, "{what}: the reply's dimensions");
    image
}

/// Checks a preview's histogram and statistics against its own pixels: the engine's histogram of
/// the image (`Histogram::of_srgb8`, what the Histogram panel shows) in 64 bins of 4 levels, the
/// pixels counted, and its mean luma, percentiles and clipping.
fn assert_stats_match(reply: &Value, image: &Rgba8, what: &str) {
    let h = Histogram::of_srgb8(image);
    let bins = |levels: &[u32]| levels.chunks(4).map(|four| four.iter().sum::<u32>()).collect::<Vec<u32>>();
    for (name, levels) in [("r", &h.r[..]), ("g", &h.g[..]), ("b", &h.b[..]), ("luma", &h.luma[..])] {
        assert_eq!(reply["histogram"][name], json!(bins(levels)), "{what}: histogram.{name}");
    }
    assert_eq!(reply["total"], h.total, "{what}: the pixels counted");
    let (clip_lo, clip_hi) = h.clipping();
    for (name, expected) in [
        ("meanLuma", h.mean_luma()),
        ("p01", h.percentile(0.01)),
        ("p50", h.percentile(0.5)),
        ("p99", h.percentile(0.99)),
        ("clipLo", clip_lo),
        ("clipHi", clip_hi),
    ] {
        let got = reply["stats"][name].as_f64().unwrap_or_else(|| panic!("{what}: stats.{name}: {reply}"));
        assert!((got - f64::from(expected)).abs() < 1e-6, "{what}: stats.{name} is {got}, the pixels give {expected}");
    }
}

/// The coverage of a white-on-black mask image: the mean grey level (a fraction of the image), the
/// box `[x, y, width, height]` of the pixels at least half covered (128 and up) and the
/// grey-weighted centroid (pixel centres at +0.5), in preview pixels.
fn coverage(image: &Rgba8) -> (f64, Option<[usize; 4]>, Option<[f64; 2]>) {
    let (mut sum, mut sum_x, mut sum_y) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut corners: Option<[usize; 4]> = None;
    for y in 0..image.height {
        for x in 0..image.width {
            let level = image.get(x, y)[0];
            let v = f64::from(level);
            sum += v;
            sum_x += v * (x as f64 + 0.5);
            sum_y += v * (y as f64 + 0.5);
            if level >= 128 {
                corners = Some(match corners {
                    None => [x, y, x, y],
                    Some([x0, y0, x1, y1]) => [x0.min(x), y0.min(y), x1.max(x), y1.max(y)],
                });
            }
        }
    }
    let area = sum / (255.0 * (image.width * image.height) as f64);
    let bbox = corners.map(|[x0, y0, x1, y1]| [x0, y0, x1 - x0 + 1, y1 - y0 + 1]);
    let centroid = (sum > 0.0).then_some([sum_x / sum, sum_y / sum]);
    (area, bbox, centroid)
}

/// What an encoded image embeds, as the engine's decoder reads it: the tags of its EXIF IFD0
/// (`None` without EXIF), its XMP packet and its dimensions.
struct Embedded {
    exif: Option<Vec<u16>>,
    xmp: Option<String>,
    width: u32,
    height: u32,
}

fn embedded(bytes: &[u8], what: &str) -> Embedded {
    let decoded = lightcraft_codecs::decode(bytes, DecodeOptions::default()).unwrap_or_else(|e| panic!("{what}: decoding the file: {e}"));
    Embedded { exif: decoded.exif.as_deref().map(ifd0_tags), xmp: decoded.xmp, width: decoded.width, height: decoded.height }
}

/// The tags of IFD0 in TIFF-structured EXIF.
fn ifd0_tags(tiff: &[u8]) -> Vec<u16> {
    let little = match tiff.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        other => panic!("EXIF starts with a byte-order mark, not {other:?}"),
    };
    let u16_at = |at: usize| tiff.get(at..at + 2).map(|b| if little { u16::from_le_bytes([b[0], b[1]]) } else { u16::from_be_bytes([b[0], b[1]]) });
    let u32_at = |at: usize| {
        tiff.get(at..at + 4).map(|b| {
            let b = [b[0], b[1], b[2], b[3]];
            if little { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) }
        })
    };
    let ifd = u32_at(4).expect("an IFD0 offset") as usize;
    let count = u16_at(ifd).expect("an IFD0 entry count");
    (0..usize::from(count)).map(|i| u16_at(ifd + 2 + 12 * i).expect("an IFD0 entry")).collect()
}

/// Whether `bytes` contain `text` anywhere.
fn contains(bytes: &[u8], text: &str) -> bool {
    bytes.windows(text.len()).any(|window| window == text.as_bytes())
}

/// A JPEG as a camera or another editor leaves it, with the camera model, creator, copyright,
/// caption and GPS position in its EXIF and XMP: the engine's own export of a tagged demo photo with
/// everything embedded. Returns the bytes and the camera model.
fn tagged_jpeg() -> (Vec<u8>, String) {
    lightcraft_engine::gpu::set_enabled(false);
    let mut session = Session::with_demo();
    let id = session.active().expect("the demo library opens with a photo active");
    session
        .execute("photo.setMeta", &json!({"ids": [id.0], "gps": GPS, "copyright": COPYRIGHT, "caption": CAPTION}))
        .unwrap_or_else(|e| panic!("tagging the fixture: {e}"));
    let camera = session.catalog.photo(id).map(|p| p.meta.camera.clone()).unwrap_or_default();
    assert!(camera.len() >= 6, "the demo photo names its camera: {camera:?}");
    let everything = ExportOptions {
        format: ExportFormat::Jpeg,
        resize: Some(Resize::long_edge(320)),
        metadata: MetadataPolicy::All,
        remove_location: false,
        ..Default::default()
    };
    let jpeg = export_photo(&mut session, id, &everything, 1).unwrap_or_else(|e| panic!("exporting the fixture: {e}")).bytes;
    let tags = embedded(&jpeg, "the fixture").exif.unwrap_or_default();
    assert!(
        tags.contains(&GPS_IFD) && tags.contains(&MODEL) && tags.contains(&COPYRIGHT_TAG),
        "the fixture's EXIF has the position, the camera and the notice: {tags:x?}"
    );
    assert!(contains(&jpeg, &camera) && contains(&jpeg, COPYRIGHT) && contains(&jpeg, CAPTION), "the fixture carries its text");
    (jpeg, camera)
}

/// Exports `id` under every policy and checks what each file embeds, then that nothing else
/// happened: no file written, nothing selected, no preference, nothing in the library store.
fn check_exports(lab: &mut Lab, id: PhotoId, camera: &str, what: &str) {
    let writes = Rc::new(Cell::new(0_usize));
    let counter = Rc::clone(&writes);
    lab.app.services.write = Some(Box::new(move |_path: &str, _bytes: &[u8]| -> Result<(), String> {
        counter.set(counter.get() + 1);
        Ok(())
    }));
    let base = json!({"id": id.0, "format": "jpeg", "longEdge": 200});
    let before = Witness::of(&lab.app);
    // (extra parameters, policy applied, location removed, embeds: position, notice, caption)
    for (extra, policy, removed, position, notice, caption) in [
        (json!({}), "copyright", true, false, true, false),
        (json!({"metadata": "none"}), "none", true, false, false, false),
        (json!({"metadata": "none", "removeLocation": false}), "none", false, false, false, false),
        (json!({"metadata": "copyright", "removeLocation": false}), "copyright", false, false, true, false),
        (json!({"metadata": "allExceptCamera"}), "allExceptCamera", true, false, true, true),
        (json!({"metadata": "allExceptCamera", "removeLocation": false}), "allExceptCamera", false, true, true, true),
    ] {
        let params = with(&base, extra);
        let what = format!("{what}: lts.export {params}");
        let reply = lts(&mut lab.app, "lts.export", params);
        assert_eq!(reply["outcome"], "ok", "{what}: {reply}");
        assert_eq!((reply["metadata"].as_str(), reply["removeLocation"].as_bool()), (Some(policy), Some(removed)), "{what}: the policy applied");
        assert_eq!(
            (reply["photoId"].as_u64(), reply["format"].as_str(), reply["mime"].as_str()),
            (Some(id.0), Some("jpeg"), Some("image/jpeg")),
            "{what}"
        );
        assert_eq!(reply["hash"], hash_of(&lab.app, id), "{what}: the settings exported");
        assert_eq!(reply["catalogRevision"], lab.app.session.catalog.revision, "{what}");
        let bytes = base64(reply["bytesBase64"].as_str().unwrap_or_default());
        assert_eq!(reply["byteLength"], bytes.len(), "{what}");
        let file = embedded(&bytes, &what);
        assert!(reply["width"] == file.width && reply["height"] == file.height, "{what}: the reply's dimensions");
        assert_eq!(file.width.max(file.height), 200, "{what}: the long edge");
        let tags = file.exif.clone().unwrap_or_default();
        assert_eq!(file.exif.is_some(), policy != "none", "{what}: EXIF only under a policy that keeps something: {tags:x?}");
        assert!(policy != "none" || file.xmp.is_none(), "{what}: no XMP under none");
        let xmp = file.xmp.unwrap_or_default();
        assert_eq!(tags.contains(&GPS_IFD) || xmp.contains("GPS"), position, "{what}: the position");
        assert!(!tags.contains(&MODEL) && !xmp.contains("tiff:Model") && !contains(&bytes, camera), "{what}: never the camera");
        assert_eq!(tags.contains(&COPYRIGHT_TAG) && contains(&bytes, COPYRIGHT), notice, "{what}: the copyright notice");
        assert_eq!(contains(&bytes, CAPTION), caption, "{what}: the caption");
    }
    let what_png = format!("{what}: a PNG under none");
    let reply = lts(&mut lab.app, "lts.export", with(&base, json!({"format": "png", "metadata": "none"})));
    assert_eq!((reply["outcome"].as_str(), reply["mime"].as_str()), (Some("ok"), Some("image/png")), "{what_png}: {reply}");
    let png = base64(reply["bytesBase64"].as_str().unwrap_or_default());
    let chunks = png_chunks(&png);
    for kind in [*b"tEXt", *b"zTXt", *b"iTXt", *b"eXIf"] {
        assert!(!chunks.contains(&kind), "{what_png}: a {} chunk", String::from_utf8_lossy(&kind));
    }
    assert!(!contains(&png, COPYRIGHT) && !contains(&png, camera), "{what_png}: no text at all");
    for extra in [
        json!({"metadata": "all"}),
        json!({"metadata": "ALL"}),
        json!({"metadata": "bogus"}),
        json!({"metadata": null}),
        json!({"removeLocation": "yes"}),
        json!({"removeLocation": 0}),
        json!({"format": "tiff"}),
        json!({"format": "original"}),
        json!({"format": "dng"}),
    ] {
        let params = with(&base, extra);
        let what = format!("{what}: lts.export {params}");
        let reply = lts(&mut lab.app, "lts.export", params);
        assert_eq!(refusal(&reply), "invalid_params", "{what}: never more than the policies offered: {reply}");
    }
    assert_unchanged(lab, &before, what);
    assert_eq!(writes.get(), 0, "{what}: no file was written");
}

// ------------------------------------------------------------------------- sections, presets, sync

/// A photo's settings as `develop.get` answers them.
fn settings_json(app: &LightcraftApp, id: PhotoId) -> Value {
    app.session.develop_of(id).unwrap_or_else(|| panic!("{id:?} is in the catalog")).to_json()
}

/// Whether `given` says what `canonical` does: the same keys at every level and equal values,
/// numbers compared by value (`5200` is `5200.0`).
fn same_json(given: &Value, canonical: &Value) -> bool {
    match (given, canonical) {
        (Value::Object(a), Value::Object(b)) => a.len() == b.len() && a.iter().all(|(key, v)| b.get(key).is_some_and(|w| same_json(v, w))),
        (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(v, w)| same_json(v, w)),
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        _ => given == canonical,
    }
}

/// The label of the top undo step.
fn undo_label(lab: &Lab) -> Option<String> {
    lab.app.session.undo.last().map(|step| step.label.clone())
}

/// `value` read as the engine's `T` and written back as the settings' serialization writes it (its
/// defaults filled in): exactly what a section holds.
fn canonical<T: Serialize + DeserializeOwned>(value: Value) -> Value {
    let read: T = serde_json::from_value(value).unwrap_or_else(|e| panic!("test infrastructure: reading the value: {e}"));
    serde_json::to_value(read).unwrap_or_else(|e| panic!("test infrastructure: writing the value: {e}"))
}

/// `base` with the value at JSON pointer `pointer` replaced by `value`.
fn replaced(base: &Value, pointer: &str, value: Value) -> Value {
    let mut out = base.clone();
    *out.pointer_mut(pointer).unwrap_or_else(|| panic!("test infrastructure: {pointer} is in {base}")) = value;
    out
}

/// A normalized point as the settings write one.
fn point(x: f64, y: f64) -> Value {
    json!({"x": x, "y": y})
}

/// The sync an adapter makes with `lts.develop {settings}` alone: the target's settings
/// (`develop.get`) with the keys of every copied group (`SettingsGroup::keys`) taken from the
/// source's, as `develop.sync` and `develop.paste` copy them (`apply_partial` of the groups).
fn synced(source: &Value, target: &Value, groups: &[SettingsGroup]) -> Value {
    let mut settings = target.clone();
    for key in groups.iter().flat_map(|group| group.keys()) {
        if let (Some(fields), Some(value)) = (settings.as_object_mut(), source.get(*key)) {
            fields.insert((*key).to_string(), value.clone());
        }
    }
    settings
}

/// The names in a folder, sorted, each with whether it is a folder.
fn listing(dir: &Path) -> Vec<(String, bool)> {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("listing {}: {e}", dir.display()));
    let mut names: Vec<(String, bool)> = entries
        .map(|entry| entry.unwrap_or_else(|e| panic!("listing {}: {e}", dir.display())))
        .map(|entry| (entry.file_name().to_string_lossy().to_string(), entry.path().is_dir()))
        .collect();
    names.sort();
    names
}

/// The same edits in a library: the agent's develop of `target` through the hook (`LTS develop`),
/// then a human's edit of its detail and vignette through the editor (the photo selected, then
/// `active` again). Answers the agent's reply.
fn agent_then_human(lab: &mut Lab, target: PhotoId, active: PhotoId) -> Value {
    let agent = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "values": {"light.exposure": 0.6}, "label": "LTS develop"}));
    assert_eq!(agent["outcome"], "applied", "{agent}");
    for (command, params) in [
        ("library.select", json!({"ids": [target.0]})),
        ("develop.set", json!({"values": {"detail.sharpenAmount": 60, "vignette.amount": -30}})),
        ("library.select", json!({"ids": [active.0]})),
    ] {
        lab.app.run(command, params).unwrap_or_else(|e| panic!("the human's {command}: {e}"));
    }
    settle(lab);
    agent
}

// ------------------------------------------------------------------------ the crop's magnification

/// The demo's unedited photos (their settings the defaults) other than `taken`, in id order.
fn plain_photos(app: &LightcraftApp, taken: &[PhotoId]) -> Vec<PhotoId> {
    all_photos(app).into_iter().filter(|p| !taken.contains(&p.id) && *p.develop == DevelopSettings::default()).map(|p| p.id).collect()
}

/// Changes photo `id`'s settings with `edit` by another path than the hook (a script, an import:
/// `develop_op` and `commit`), as one undo step, and forgets what that wrote.
fn committed(lab: &mut Lab, id: PhotoId, label: &str, edit: impl FnOnce(&mut DevelopSettings)) {
    let session = &mut lab.app.session;
    let mut settings = Arc::unwrap_or_clone(session.develop_of(id).expect("the photo"));
    edit(&mut settings);
    let op = session.develop_op(id, settings, label).expect("an op for the photo");
    session.commit(label, op).unwrap_or_else(|e| panic!("the fixture: {e}"));
    settle(lab);
}

/// Runs `command` with `params`, which must apply, and forgets what it wrote.
fn applies(lab: &mut Lab, command: &str, params: Value) {
    let what = format!("{command} {params}");
    let reply = lts(&mut lab.app, command, params);
    assert_eq!(reply["outcome"], "applied", "{what}: {reply}");
    settle(lab);
}

/// Runs `command` with `params`, which must be refused with `code` with nothing changed.
fn refuses(lab: &mut Lab, command: &str, params: Value, code: &str) {
    refuses_saying(lab, command, params, code, "");
}

/// [`refuses`], the refusal's message saying `words`.
fn refuses_saying(lab: &mut Lab, command: &str, params: Value, code: &str, words: &str) {
    let what = format!("{command} {params}");
    let before = Witness::of(&lab.app);
    let reply = lts(&mut lab.app, command, params);
    assert_eq!(refusal(&reply), code, "{what}: {reply}");
    assert!(reply["message"].as_str().is_some_and(|message| message.contains(words)), "{what}: the message says {words:?}: {reply}");
    assert_unchanged(lab, &before, &what);
}

// --------------------------------------------------------------------- R1: the reads of the table

/// What the page answers to `lightcraft.command(id, params)` for an id outside the hook's table in
/// the LTS presentation: `handle`, which the inbox asks first for every id (`web.rs`), answers it
/// itself, with a typed reply (T-LI-21); the envelope is checked.
fn at_the_fork(app: &mut LightcraftApp, id: &str, params: Value) -> Value {
    let what = format!("{id:?} {params}");
    let reply = handle(app, id, &params).unwrap_or_else(|| panic!("{what}: the LTS editor answers every id at the fork; the page ran this one"));
    let reply = reply.unwrap_or_else(|e| panic!("{what}: rejected with untyped text ({e}); every typed outcome resolves"));
    check_envelope(id, &reply, &what);
    reply
}

/// The engine's own answer to `id` (`LightcraftApp::run`): what a certified read gave the adapter
/// before the LTS presentation refused it at the fork (T-LI-21).
fn engine_read(app: &mut LightcraftApp, id: &str, params: Value) -> Value {
    app.run(id, params).unwrap_or_else(|e| panic!("the engine's {id}: {e}"))
}

/// `value` without the keys `keys`.
fn without(value: &Value, keys: &[&str]) -> Value {
    let mut out = value.clone();
    if let Some(fields) = out.as_object_mut() {
        for key in keys {
            fields.remove(*key);
        }
    }
    out
}

/// Asserts that `rows` (a page of `lts.library`) are the engine's `want` (`catalog.query`'s rows), in
/// the same order, each with its photo's settings token added (`hash`).
fn assert_rows(app: &LightcraftApp, rows: &Value, want: &Value, what: &str) {
    let (rows, want) = (rows.as_array().cloned().unwrap_or_default(), want.as_array().cloned().unwrap_or_default());
    assert_eq!(rows.len(), want.len(), "{what}: the rows");
    for (row, want) in rows.iter().zip(&want) {
        let id = PhotoId(row["id"].as_u64().unwrap_or_else(|| panic!("{what}: a row names its photo: {row}")));
        assert_eq!(row["hash"], hash_of(app, id), "{what}: photo {}'s token", id.0);
        assert_eq!(&without(row, &["hash"]), want, "{what}: photo {}'s row is catalog.query's", id.0);
    }
}

/// The tokens of photo `id`'s history, as `lts.photo` answers them (`revisions`, the oldest first):
/// each step's index, label and settings token, the `revision` that `lts.restoreSettings` takes.
fn revisions(app: &LightcraftApp, id: PhotoId) -> Value {
    let history = photo(app, id).history;
    let rows = history.iter().enumerate().map(|(step, entry)| json!({"step": step, "label": entry.label, "hash": hex(entry.settings.hash64())}));
    Value::Array(rows.collect())
}

// ------------------------------------------------------------- R1: the control catalog and its gates

/// The gates a develop control acts behind (`lts.controls`; R1, backlog item 2), in the catalog's
/// order: the states in which it changes no pixel. The renderer honours four section switches
/// (`crates/pipeline/src/finish.rs:226-253`, `optics.rs:74`, `geometry.rs:271`; the straighten angle
/// is the crop's, not the geometry section's) and ignores the Light, Color and Detail ones; the B&W
/// mixes act under B&W only and the point colours in colour only (`pipeline/src/lib.rs:481-483`,
/// `colorops.rs:166-171`); the profile's amount scales a look (`profiles.rs:45-50`); the lens profile
/// sliders need the matching correction embedded in the file (`optics.rs:82-88`); and the vignette's
/// highlights need a negative vignette in Highlight Priority (`finish.rs:87-101, 538`). And the
/// settings other controls act through (T-LI-27): in black and white, Vibrance, Saturation and the
/// mixer's Hue and Saturation act only through a B&W mix (`colorops.rs:241-246`); the region splits
/// need a region slider and Refine Saturation a curve (`finish.rs:40-55, 574-579`); Blending and
/// Balance need a zone wheel, a wheel's Hue its Saturation (`colorops.rs:149-152, 252-264`); the
/// vignette's shape and the grain's need their amounts (`finish.rs:87-89, 234`), Radius, Detail and
/// Masking sharpening (`finish.rs:473-484`), each noise reduction's sliders its amount
/// (`local.rs:152-161`), the lens Vignetting's Midpoint that Vignetting (`optics.rs:187-190`), each
/// defringe hue range its amount (`optics.rs:535-538`), and a point colour's ranges a shift of its
/// sample (`colorops.rs:166-169`).
fn gates_of(control: &str) -> Vec<&'static str> {
    let effects = ["effects.", "vignette.", "grain."].into_iter().any(|prefix| control.starts_with(prefix));
    let ranges = [".range", ".hueRange", ".satRange", ".lumRange"].into_iter().any(|field| control.ends_with(field));
    match control {
        "profile.amount" => vec!["profileLook"],
        "vignette.highlights" => vec!["effectsOn", "highlightPriority"],
        "vignette.midpoint" | "vignette.roundness" | "vignette.feather" => vec!["effectsOn", "vignetteOn"],
        "grain.size" | "grain.roughness" => vec!["effectsOn", "grainOn"],
        "optics.profileDistortion" => vec!["opticsOn", "lensDistortion"],
        "optics.profileVignetting" => vec!["opticsOn", "lensVignetting"],
        "optics.vignettingMidpoint" => vec!["opticsOn", "manualVignetting"],
        "optics.defringePurpleHueLo" | "optics.defringePurpleHueHi" => vec!["opticsOn", "defringePurple"],
        "optics.defringeGreenHueLo" | "optics.defringeGreenHueHi" => vec!["opticsOn", "defringeGreen"],
        "color.vibrance" | "color.saturation" => vec!["colorOrBwMix"],
        "curve.splitShadows" | "curve.splitMid" | "curve.splitHighlights" => vec!["curveRegions"],
        "curve.refineSaturation" => vec!["curveOn"],
        "grading.blending" | "grading.balance" => vec!["gradingZones"],
        "grading.shadows.hue" => vec!["shadowsTint"],
        "grading.midtones.hue" => vec!["midtonesTint"],
        "grading.highlights.hue" => vec!["highlightsTint"],
        "grading.global.hue" => vec!["globalTint"],
        "detail.sharpenRadius" | "detail.sharpenDetail" | "detail.sharpenMasking" => vec!["sharpenOn"],
        "detail.nrDetail" | "detail.nrContrast" => vec!["nrLuminanceOn"],
        "detail.nrColorDetail" | "detail.nrColorSmoothness" => vec!["nrColorOn"],
        _ if control.starts_with("mixer.") && !control.ends_with(".lum") => vec!["colorOrBwMix"],
        _ if effects => vec!["effectsOn"],
        _ if control.starts_with("optics.") => vec!["opticsOn"],
        _ if control.starts_with("geometry.") => vec!["geometryOn"],
        _ if control.starts_with("calibration.") => vec!["calibrationOn"],
        _ if control.starts_with("bw.") => vec!["bw"],
        _ if control.starts_with("pointColor.") && ranges => vec!["color", "pointColorShift"],
        _ if control.starts_with("pointColor.") => vec!["color"],
        _ => Vec::new(),
    }
}

/// The gates open for photo `id`, in the catalog's order, as `lts.controls {id}` answers them (`open`:
/// true or false for every gate).
fn open_gates(app: &mut LightcraftApp, id: PhotoId) -> Vec<&'static str> {
    let catalog = lts(app, "lts.controls", json!({"id": id.0}));
    assert!(GATES.iter().all(|gate| catalog["open"][*gate].is_boolean()), "each gate open or closed for photo {}: {catalog}", id.0);
    GATES.into_iter().filter(|gate| catalog["open"][*gate] == true).collect()
}

/// The gates open on a plain demo photo and the gates `more`, in the catalog's order.
fn plain_and(more: &[&str]) -> Vec<&'static str> {
    GATES.into_iter().filter(|gate| PLAIN_OPEN.contains(gate) || more.contains(gate)).collect()
}

/// Asserts that `catalog` (`lts.controls {id}`) gives photo `id`'s values: every develop control, then
/// each list control of the photo (its point colours and red-eye corrections, as `develop.controls`
/// lists them for the active photo), each with the photo's value and its gates.
fn assert_values(app: &LightcraftApp, catalog: &Value, id: PhotoId) {
    let settings = app.session.develop_of(id).unwrap_or_else(|| panic!("{id:?} is in the catalog"));
    let mut ids: Vec<String> = CONTROLS.iter().map(|spec| spec.id.to_string()).collect();
    ids.extend(lightcraft_engine::develop::controls::indexed_instances(&settings).into_iter().map(|(control, _)| control));
    let listed = catalog["controls"].as_array().cloned().unwrap_or_default();
    let got: Vec<&str> = listed.iter().filter_map(|entry| entry["id"].as_str()).collect();
    assert_eq!(got, ids, "photo {}'s controls: the develop controls, then its list controls", id.0);
    for entry in &listed {
        let control = entry["id"].as_str().unwrap_or_default();
        let value = lightcraft_engine::develop::controls::get(&settings, control);
        assert_eq!(entry["value"], json!(value), "{control}: photo {}'s value", id.0);
        assert_eq!(entry["gates"], json!(gates_of(control)), "{control}: its gates");
    }
}

/// Whether `values` change photo `id`'s render: a what-if preview with them (variant `settings`)
/// beside a preview of the photo as it is, both `long` pixels long, compared pixel by pixel.
fn changes_render(app: &mut LightcraftApp, id: PhotoId, values: Value, long: u32) -> bool {
    let own = lts(app, "lts.preview", json!({"id": id.0, "longEdge": long}));
    let what_if = lts(app, "lts.preview", json!({"id": id.0, "longEdge": long, "variant": "settings", "values": values}));
    assert_eq!((own["outcome"].as_str(), what_if["outcome"].as_str()), (Some("ok"), Some("ok")), "previews of photo {}: {what_if}", id.0);
    preview_pixels(&own, "the photo's render") != preview_pixels(&what_if, "the what-if render")
}

/// Asserts that `values`, controls of `section`, change nothing while photo `id`'s section is off
/// (its gate `gate` closed, the photo's other gates as a plain photo's) and change the render once
/// it is on again. The switch is a person's (`develop.sectionEnabled`), made here by another path.
fn assert_section_gate(lab: &mut Lab, id: PhotoId, section: &str, gate: &str, values: Value) {
    committed(lab, id, "Toggle Section", |d| d.set_section_enabled(section, false));
    let open: Vec<&str> = PLAIN_OPEN.into_iter().filter(|other| *other != gate).collect();
    assert_eq!(open_gates(&mut lab.app, id), open, "the {section} section off");
    assert!(!changes_render(&mut lab.app, id, values.clone(), 256), "{values}: nothing while the {section} section is off");
    committed(lab, id, "Toggle Section", |d| d.set_section_enabled(section, true));
    assert_eq!(open_gates(&mut lab.app, id), PLAIN_OPEN, "the {section} section on");
    assert!(changes_render(&mut lab.app, id, values.clone(), 256), "{values}: renders while the {section} section is on");
}

/// A photo with a one-pixel purple fringe between white and black, and a green one likewise, as a
/// lens leaves them along high-contrast edges (64 × 32 pixels): each colour sits in the middle of its
/// defringe hue range by default (OkLab hue about 307° and 152°), where the range picks it.
fn fringed_png() -> Vec<u8> {
    let image = Rgba8::from_fn(64, 32, |x, _| match x {
        15 => [150, 50, 220, 255],
        47 => [0, 160, 80, 255],
        0..=14 | 32..=46 => [255, 255, 255, 255],
        _ => [0, 0, 0, 255],
    });
    lightcraft_codecs::encode_png(&EncodeImage::rgba8(&image), &EncodeMeta::default()).expect("encoding a PNG")
}

/// A ramp of colours that a point colour picked at its centre selects from (red, with green across
/// and blue down; 64 × 64 pixels): hue, chroma and lightness change around the sample on every axis.
fn ramp_png() -> Vec<u8> {
    let image = Rgba8::from_fn(64, 64, |x, y| [220, (30 + 2 * x) as u8, (40 + 2 * y) as u8, 255]);
    lightcraft_codecs::encode_png(&EncodeImage::rgba8(&image), &EncodeMeta::default()).expect("encoding a PNG")
}

// ----------------------------------------------------------------------------------------- the tests

/// T-LI-02: `lts.preview` is a pure read. For the active photo and another one, with a photo active
/// and with none (an embedded library before the human picks one), in both presentations, it
/// answers the engine's render of the current settings (`render_job`, crop applied) as a PNG without
/// metadata, the same answer twice, and leaves the catalog, the selection and the active photo, the
/// active mask and spot, undo and redo, the journal, the preferences and `last_export`, the UI state
/// and the status line, the render queue, the rendered previews and the library store as they were.
/// An unknown photo is refused just as quietly.
#[test]
fn preview_is_pure_no_select_no_prefs_no_last_export_no_journal() {
    for presentation in PRESENTATIONS {
        let mut lab = editing_lab(presentation);
        let active = active_photo(&lab.app);
        let other = other_photo(&lab.app, active);
        for none_active in [false, true] {
            if none_active {
                deselect_all(&mut lab);
            }
            for target in [active, other] {
                let what = format!("{presentation:?}, no photo active: {none_active}, preview of {target:?}");
                let params = json!({"id": target.0, "longEdge": 256});
                let before = Witness::of(&lab.app);
                let reply = lts(&mut lab.app, "lts.preview", params.clone());
                let again = lts(&mut lab.app, "lts.preview", params);
                assert_unchanged(&mut lab, &before, &what);
                assert_eq!(reply["outcome"], "ok", "{what}: {reply}");
                assert_eq!(again, reply, "{what}: the same answer twice");
                assert_eq!(reply["photoId"], target.0, "{what}");
                assert_eq!(
                    (reply["variant"].as_str(), reply["crop"].as_str(), reply["frame"].as_str()),
                    (Some("current"), Some("applied"), Some("cropped-straightened")),
                    "{what}: the defaults"
                );
                assert_eq!(reply["hash"], hash_of(&lab.app, target), "{what}: the photo's settings token");
                assert_eq!(reply["renderedHash"], reply["hash"], "{what}: the current settings are rendered");
                assert_eq!(reply["catalogRevision"], lab.app.session.catalog.revision, "{what}");
                let image = preview_pixels(&reply, &what);
                assert_eq!(image.width.max(image.height), 256, "{what}: the long edge");
                assert!(image == reference(&mut lab.app, target, 256, false, Overlay::None), "{what}: the engine's render of the current settings");
            }
        }
        let what = format!("{presentation:?}: a preview of an unknown photo");
        let before = Witness::of(&lab.app);
        let reply = lts(&mut lab.app, "lts.preview", json!({"id": UNKNOWN, "longEdge": 256}));
        assert_eq!(refusal(&reply), "target_not_found", "{what}: {reply}");
        assert_unchanged(&mut lab, &before, &what);
    }
}

/// T-LI-03: the variants render what they name and stay pure. After an exposure increase, `before`
/// is the engine's before render (darker); `settings` with `values` applies them as `develop.set`
/// would (the current values render the current settings) and with a complete `settings` renders
/// that JSON (darker with a lower exposure); each reply names the settings it rendered
/// (`renderedHash`) and keeps the photo's own token. Malformed variants are refused without effect,
/// and a what-if preview is exactly what the edit then commits.
#[test]
fn before_and_settings_variants_differ_as_expected_and_stay_pure() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let target = other_photo(&lab.app, active_photo(&lab.app));
    lab.app.run("develop.set", json!({"ids": [target.0], "values": {"light.exposure": 1.0}})).unwrap_or_else(|e| panic!("setup: {e}"));
    settle(&mut lab);
    let hash = hash_of(&lab.app, target);
    let current_settings = lab.app.session.develop_of(target).expect("the target");
    let mut darker_json = current_settings.to_json();
    darker_json["light"]["exposure"] = json!(-1.0);
    let darker_settings = DevelopSettings::from_json(&darker_json).expect("settings JSON");
    let before_settings = lab.app.session.before_settings(lab.app.session.catalog.photo(target).expect("the target"));

    let before = Witness::of(&lab.app);
    let current = preview(&mut lab.app, target, json!({}));
    let earlier = preview(&mut lab.app, target, json!({"variant": "before"}));
    let same = preview(&mut lab.app, target, json!({"variant": "settings", "values": {"light.exposure": 1.0}}));
    let darker = preview(&mut lab.app, target, json!({"variant": "settings", "settings": darker_json}));
    let what_if = preview(&mut lab.app, target, json!({"variant": "settings", "values": {"light.exposure": 0.25}}));
    assert_unchanged(&mut lab, &before, "variant previews");
    for (reply, variant, rendered) in [
        (&current, "current", hash.clone()),
        (&earlier, "before", hex(before_settings.hash64())),
        (&same, "settings", hash.clone()),
        (&darker, "settings", hex(darker_settings.hash64())),
        (&what_if, "settings", what_if["renderedHash"].as_str().unwrap_or_default().to_string()),
    ] {
        assert_eq!((reply["outcome"].as_str(), reply["variant"].as_str()), (Some("ok"), Some(variant)), "{reply}");
        assert_eq!(reply["hash"], hash, "{variant}: the photo's own token");
        assert_eq!(reply["renderedHash"], rendered, "{variant}: the settings rendered");
        assert!(is_hex16(&reply["renderedHash"]), "{variant}: a settings token");
    }
    let current_pixels = preview_pixels(&current, "current");
    assert!(current_pixels == reference(&mut lab.app, target, 256, false, Overlay::None), "current: the engine's render");
    assert!(preview_pixels(&earlier, "before") == reference(&mut lab.app, target, 256, true, Overlay::None), "before: the engine's before render");
    assert!(preview_pixels(&same, "the current values") == current_pixels, "the current values render the current settings");
    let darker_pixels = preview_pixels(&darker, "darker settings");
    assert!(darker_pixels == reference_with(&mut lab.app, target, 256, &darker_settings), "settings: the engine's render of that JSON");
    assert!(luma(&earlier) < luma(&current), "before an exposure increase is darker: {} vs {}", luma(&earlier), luma(&current));
    assert!(luma(&darker) < luma(&earlier), "exposure -1 is darker than before: {} vs {}", luma(&darker), luma(&earlier));

    for (extra, code) in [
        (json!({"variant": "settings"}), "invalid_params"),
        (json!({"values": {"light.exposure": 0.5}}), "invalid_params"),
        (json!({"variant": "before", "values": {"light.exposure": 0.5}}), "invalid_params"),
        (json!({"variant": "settings", "values": {"light.exposure": 0.5}, "settings": current_settings.to_json()}), "invalid_params"),
        (json!({"variant": "settings", "values": {"light.bogus": 0.5}}), "invalid_params"),
        (json!({"variant": "settings", "values": {"light.exposure": 9.0}}), "out_of_range"),
        (json!({"variant": "later"}), "invalid_params"),
        (json!({"crop": "sideways"}), "invalid_params"),
    ] {
        let what = format!("lts.preview {extra}");
        let before = Witness::of(&lab.app);
        let reply = preview(&mut lab.app, target, extra);
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
        assert_unchanged(&mut lab, &before, &what);
    }

    let edit = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "values": {"light.exposure": 0.25}, "expectHash": hash}));
    assert_eq!(edit["outcome"], "applied", "{edit}");
    assert_eq!(edit["hashAfter"], what_if["renderedHash"], "the committed settings are the previewed ones");
    let committed = preview(&mut lab.app, target, json!({}));
    assert_eq!(committed["hash"], what_if["renderedHash"], "the photo's token is now the previewed one");
    assert!(preview_pixels(&committed, "committed") == preview_pixels(&what_if, "what-if"), "the committed look is the previewed one");
}

/// T-LI-04: render budgets are refused before anything is decoded or allocated: a preview long edge
/// above 2048 and an export long edge above 4096 are `budget_exceeded` (absurd ones too, which would
/// abort the process if they were rendered), sizes below 16 or not integers are `invalid_params`;
/// nothing changes and no source enters the media cache. The bounds themselves are allowed. Each
/// runtime names its own bounds (R0: the browser hook and a native worker host the same file) and
/// refuses past them.
#[test]
fn render_budgets_refused_before_allocation() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let id = other_photo(&lab.app, active_photo(&lab.app)).0;
    let before = Witness::of(&lab.app);
    let sources = format!("{:?}", lab.app.session.media.usage());
    for (command, params, code) in [
        ("lts.preview", json!({"id": id, "longEdge": 2049}), "budget_exceeded"),
        ("lts.preview", json!({"id": id, "longEdge": 2049, "pixels": false}), "budget_exceeded"),
        ("lts.preview", json!({"id": id, "longEdge": 1_000_000_000}), "budget_exceeded"),
        ("lts.preview", json!({"id": id, "longEdge": u64::MAX}), "budget_exceeded"),
        ("lts.preview", json!({"id": id, "longEdge": 15}), "invalid_params"),
        ("lts.preview", json!({"id": id, "longEdge": 0}), "invalid_params"),
        ("lts.preview", json!({"id": id, "longEdge": -300}), "invalid_params"),
        ("lts.preview", json!({"id": id, "longEdge": 300.5}), "invalid_params"),
        ("lts.preview", json!({"id": id, "longEdge": "300"}), "invalid_params"),
        ("lts.preview", json!({"id": id}), "invalid_params"),
        ("lts.export", json!({"id": id, "format": "jpeg", "longEdge": 4097}), "budget_exceeded"),
        ("lts.export", json!({"id": id, "format": "png", "longEdge": 1_000_000_000}), "budget_exceeded"),
        ("lts.export", json!({"id": id, "format": "jpeg", "longEdge": 15}), "invalid_params"),
        ("lts.export", json!({"id": id, "format": "jpeg"}), "invalid_params"),
        ("lts.export", json!({"id": id, "format": "jpeg", "longEdge": 64, "quality": 0}), "invalid_params"),
        ("lts.export", json!({"id": id, "format": "jpeg", "longEdge": 64, "quality": 101}), "invalid_params"),
    ] {
        let what = format!("{command} {params}");
        let reply = lts(&mut lab.app, command, params);
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
    }
    for runtime in [Runtime::Browser, Runtime::Native] {
        assert_eq!((runtime.preview_max(), runtime.export_max()), (2048, 4096), "{runtime:?}: the render bounds");
        for (command, params) in [
            ("lts.preview", json!({"id": id, "longEdge": runtime.preview_max() + 1})),
            ("lts.export", json!({"id": id, "format": "png", "longEdge": runtime.export_max() + 1})),
        ] {
            let reply = lts_in(&mut lab.app, command, params, runtime);
            assert_eq!(refusal(&reply), "budget_exceeded", "{runtime:?} {command}: {reply}");
        }
    }
    assert_unchanged(&mut lab, &before, "refused renders");
    assert_eq!(format!("{:?}", lab.app.session.media.usage()), sources, "a refused render decodes no source");

    let largest = lts(&mut lab.app, "lts.preview", json!({"id": id, "longEdge": 2048, "pixels": false}));
    assert_eq!(largest["outcome"], "ok", "{largest}");
    assert!(largest.get("pngBase64").is_none(), "pixels:false carries no image: {largest}");
    let long_edge = largest["width"].as_u64().zip(largest["height"].as_u64()).map(|(w, h)| w.max(h));
    assert_eq!(long_edge, Some(2048), "a 2048-pixel preview: {largest}");
    let smallest = lts(&mut lab.app, "lts.preview", json!({"id": id, "longEdge": 16}));
    let image = preview_pixels(&smallest, "a 16-pixel preview");
    assert_eq!(image.width.max(image.height), 16, "a 16-pixel preview");
    let export = lts(&mut lab.app, "lts.export", json!({"id": id, "format": "png", "longEdge": 16, "metadata": "none"}));
    let file = embedded(&base64(export["bytesBase64"].as_str().unwrap_or_default()), "a 16-pixel export");
    assert_eq!(file.width.max(file.height), 16, "a 16-pixel export: {export}");
}

/// T-LI-05: a mask overlay shows the mask itself, white on black (the default view), exactly as the
/// engine's mask view draws it, with its coverage: the mean grey level, the box of the pixels at
/// least half covered and the weighted centroid, which are the statistics of the image returned and
/// are reported whatever the view. A hard radial mask comes out as a disc of its radius (a fraction
/// of the long edge) at its centre. An unknown mask is refused rather than drawn as the plain photo.
/// Visualize Spots (R0: `{spots: true, threshold?}`) is the engine's own view, the specks white on
/// black, at the editor's default threshold (50) or the one asked, with no coverage; `spots: false`,
/// a threshold past 100 and a mask id beside `spots` are refused. Nothing changes, the active mask
/// included.
#[test]
fn mask_overlay_white_on_black_with_coverage() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let target = active_photo(&lab.app);
    lab.app
        .run("mask.add", json!({"kind": "radial", "center": [0.3, 0.5], "rx": 0.12, "ry": 0.12, "feather": 0}))
        .unwrap_or_else(|e| panic!("adding the mask: {e}"));
    let mask = lab.app.session.develop_of(target).and_then(|d| d.masks.last().map(|m| m.id)).expect("the new mask");
    settle(&mut lab);
    let base = json!({"id": target.0, "longEdge": 400});
    let before = Witness::of(&lab.app);
    let white = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"maskId": mask, "view": "whiteOnBlack"}})));
    let default_view = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"maskId": mask}})));
    let tinted = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"maskId": mask, "view": "color"}})));
    let plain = lts(&mut lab.app, "lts.preview", base.clone());
    let unknown = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"maskId": mask + 100}})));
    let sideways = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"maskId": mask, "view": "sideways"}})));
    let beyond = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"maskId": 70_000}})));
    let nameless = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"view": "color"}})));
    let specks = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"spots": true}})));
    let faintest = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"spots": true, "threshold": 100}})));
    let spotless = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"spots": false}})));
    let past = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"spots": true, "threshold": 101}})));
    let both = lts(&mut lab.app, "lts.preview", with(&base, json!({"overlay": {"spots": true, "maskId": mask}})));
    assert_unchanged(&mut lab, &before, "overlays");
    assert_eq!(refusal(&unknown), "target_not_found", "an unknown mask is refused, not drawn as the plain photo: {unknown}");
    assert_eq!(refusal(&sideways), "invalid_params", "{sideways}");
    assert_eq!(refusal(&beyond), "target_not_found", "a mask id the overlay cannot address: {beyond}");
    assert_eq!(refusal(&nameless), "invalid_params", "an overlay names its mask: {nameless}");
    assert!(plain.get("maskCoverage").is_none(), "no overlay, no coverage: {plain}");
    for (reply, what) in [(&spotless, "spots: false"), (&past, "a threshold of 101"), (&both, "spots and a mask id")] {
        assert_eq!(refusal(reply), "invalid_params", "{what}: {reply}");
    }

    let image = preview_pixels(&white, "white on black");
    let view =
        Overlay::Mask { id: u16::try_from(mask).expect("a small mask id"), view: MaskView::WhiteOnBlack, color: [255, 255, 255], opacity: 100 };
    assert!(image == reference(&mut lab.app, target, 400, false, view), "the engine's white-on-black mask view");
    assert!(image.data.iter().all(|p| p[0] == p[1] && p[1] == p[2] && p[3] == 255), "white on black: grey levels only");
    assert!(preview_pixels(&default_view, "the default view") == image, "white on black is the default view");
    assert!(preview_pixels(&tinted, "the colour view") != image, "the colour view shows the photo");
    let speckled = preview_pixels(&specks, "Visualize Spots");
    assert!(speckled == reference(&mut lab.app, target, 400, false, Overlay::Spots(50)), "the engine's Visualize Spots, threshold 50");
    assert!(speckled.data.iter().all(|p| (p[0] == 0 || p[0] == 255) && p[0] == p[1] && p[1] == p[2]), "specks white on black");
    let faint = preview_pixels(&faintest, "the faintest specks");
    assert!(faint == reference(&mut lab.app, target, 400, false, Overlay::Spots(100)), "the engine's Visualize Spots, threshold 100");
    assert!(specks.get("maskCoverage").is_none() && faintest.get("maskCoverage").is_none(), "Visualize Spots shows no mask: {specks}");

    let (area, bbox, centroid) = coverage(&image);
    let reported = &white["maskCoverage"];
    assert_eq!(reported["maskId"], mask, "{reported}");
    let reported_area = reported["area"].as_f64().unwrap_or(f64::NAN);
    assert!((reported_area - area).abs() < 1e-9, "the area is the mean grey level: {reported_area} vs {area}");
    let expected_box = bbox.map_or(Value::Null, |[x, y, width, height]| json!({"x": x, "y": y, "width": width, "height": height}));
    assert_eq!(reported["bbox"], expected_box, "the box of the pixels at least half covered");
    let [cx, cy] = centroid.expect("the mask covers pixels");
    let (rx, ry) = (reported["centroid"]["x"].as_f64().unwrap_or(f64::NAN), reported["centroid"]["y"].as_f64().unwrap_or(f64::NAN));
    assert!((rx - cx).abs() < 1e-6 && (ry - cy).abs() < 1e-6, "the weighted centroid: ({rx}, {ry}) vs ({cx}, {cy})");
    assert_eq!(default_view["maskCoverage"], white["maskCoverage"], "the default view's coverage");
    assert_eq!(tinted["maskCoverage"], white["maskCoverage"], "the colour view reports the mask's own coverage");

    let (width, height) = (image.width as f64, image.height as f64);
    let radius = 0.12 * width.max(height);
    let (ex, ey) = (0.3 * width, 0.5 * height);
    let disc = std::f64::consts::PI * radius * radius / (width * height);
    assert!((area - disc).abs() < 0.05 * disc, "a disc's area: {area} vs {disc}");
    let [left, top, box_width, box_height] = bbox.expect("pixels at least half covered").map(|v| v as f64);
    assert!(
        (box_width - 2.0 * radius).abs() <= 3.0 && (box_height - 2.0 * radius).abs() <= 3.0,
        "a box of the disc's diameter: {box_width} × {box_height}"
    );
    assert!((left + box_width / 2.0 - ex).abs() <= 2.0 && (top + box_height / 2.0 - ey).abs() <= 2.0, "the box is centred on the mask's centre");
    assert!((cx - ex).abs() <= 1.5 && (cy - ey).abs() <= 1.5, "the centroid is the mask's centre: ({cx}, {cy}) vs ({ex}, {ey})");
}

/// T-LI-06: previews are comparable inputs with honest statistics. The histogram (the engine's, in
/// 64 bins), the pixel count, the mean luma, the percentiles and the clipping are those of the very
/// pixels returned; `pixels:false` answers the same numbers without the image; two variants of a
/// photo at the same size and crop share their dimensions, frame and photo token and differ in
/// pixels and look; the same preview twice is byte-identical. (The test's own PNG reading is exact.)
#[test]
fn compare_inputs_and_stats() {
    let levels = Rgba8::from_fn(256, 3, |x, y| {
        let v = x as u8;
        match y {
            0 => [v, 255 - v, v / 2, 255],
            1 => [v / 3, v, 255 - v, 255],
            _ => [255 - v, v / 5, v, 255],
        }
    });
    let png = lightcraft_codecs::encode_png(&EncodeImage::rgba8(&levels), &EncodeMeta::default()).expect("encoding the levels");
    assert!(decode_srgb8(&png, "the levels") == levels, "test infrastructure: PNG bytes read back exactly");

    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let target = other_photo(&lab.app, active_photo(&lab.app));
    lab.app.run("develop.set", json!({"ids": [target.0], "values": {"light.exposure": 1.0}})).unwrap_or_else(|e| panic!("setup: {e}"));
    settle(&mut lab);
    let base = json!({"id": target.0, "longEdge": 300});
    let before = Witness::of(&lab.app);
    let current = lts(&mut lab.app, "lts.preview", base.clone());
    let numbers = lts(&mut lab.app, "lts.preview", with(&base, json!({"pixels": false})));
    let earlier = lts(&mut lab.app, "lts.preview", with(&base, json!({"variant": "before"})));
    let again = lts(&mut lab.app, "lts.preview", base.clone());
    assert_unchanged(&mut lab, &before, "previews for a comparison");

    let current_pixels = preview_pixels(&current, "current");
    let earlier_pixels = preview_pixels(&earlier, "before");
    for (reply, image, name) in [(&current, &current_pixels, "current"), (&earlier, &earlier_pixels, "before")] {
        assert_stats_match(reply, image, name);
        assert_eq!(reply["total"], image.width * image.height, "{name}: every pixel counted");
    }
    assert!(numbers.get("pngBase64").is_none(), "pixels:false carries no image: {numbers}");
    for key in ["photoId", "width", "height", "variant", "crop", "frame", "hash", "renderedHash", "catalogRevision", "histogram", "total", "stats"] {
        assert_eq!(numbers[key], current[key], "pixels:false answers the same {key}");
    }
    for key in ["photoId", "width", "height", "crop", "frame", "hash", "catalogRevision"] {
        assert_eq!(earlier[key], current[key], "before and current share their {key}");
    }
    assert_ne!(earlier["renderedHash"], current["renderedHash"], "they render different settings");
    assert!(earlier_pixels != current_pixels, "and different pixels");
    assert!(luma(&earlier) < luma(&current), "before an exposure increase is darker");
    assert_eq!(again, current, "the same preview twice is byte-identical");
}

/// T-LI-07: `lts.export` is a pure output read under an explicit metadata policy. For a demo photo
/// tagged in the catalog (in both presentations) and for an imported JPEG whose file carries a
/// camera, a GPS position and a copyright notice, every policy embeds exactly what it names: `none`
/// nothing at all (no EXIF, no XMP, no text chunk), the default `copyright` (OC-13) only the notice
/// and creator, `allExceptCamera` also the caption, and the position only when `removeLocation` is
/// false; no policy embeds the camera, and `all`, unknown policies, a non-boolean `removeLocation`
/// and formats that would hand out the original are refused. Nothing is selected, no file is
/// written, `last_export` and the other preferences stay, and nothing reaches the library store.
#[test]
fn export_pure_metadata_none_has_no_exif_gps_and_writes_no_prefs() {
    let (jpeg, camera) = tagged_jpeg();
    for presentation in PRESENTATIONS {
        let mut lab = editing_lab(presentation);
        let target = other_photo(&lab.app, active_photo(&lab.app));
        lab.app
            .run("photo.setMeta", json!({"ids": [target.0], "gps": GPS, "copyright": COPYRIGHT, "caption": CAPTION}))
            .unwrap_or_else(|e| panic!("tagging the photo: {e}"));
        settle(&mut lab);
        let demo_camera = photo(&lab.app, target).meta.camera;
        check_exports(&mut lab, target, &demo_camera, &format!("{presentation:?}, a demo photo"));
    }
    let mut lab = boot(Presentation::LtsEmbedded, false);
    let id = import(&mut lab, "tagged.jpg", &jpeg);
    let meta = photo(&lab.app, id).meta;
    assert!(meta.gps.is_some() && meta.camera == camera && meta.copyright == COPYRIGHT, "the import read the file's metadata: {meta:?}");
    check_exports(&mut lab, id, &camera, "an imported photo");
}

/// T-LI-08: `lts.state` reports the tokens as the engine has them and changes nothing. The catalog
/// revision only grows, whoever edits (a human edit, undo and redo, the guarded `lts.undo` and
/// `lts.redo`); the settings hash follows the content, so an undo brings the earlier hash back (ABA
/// by design: the revision tells the steps apart). `lts.undo` and `lts.redo` move the top step only
/// when the expected head is the stack's, the step is that photo's alone and the expected hash is
/// the photo's, and never during a human interaction; a refusal changes nothing.
#[test]
fn tokens_monotonic_and_hash_changes_on_edit_and_undo() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    let revision = |reply: &Value| reply["catalogRevision"].as_u64().unwrap_or_else(|| panic!("a catalog revision: {reply}"));
    let journal = lab.app.session.journal.len();

    let before = Witness::of(&lab.app);
    let s0 = state(&mut lab.app, target);
    let library = lts(&mut lab.app, "lts.state", json!({}));
    assert_unchanged(&mut lab, &before, "lts.state");
    assert_eq!(s0["outcome"], "ok", "{s0}");
    assert_eq!(s0["photoId"], target.0, "{s0}");
    assert!(is_hex16(&s0["hash"]), "16 lowercase hex digits: {s0}");
    assert_eq!(s0["hash"], hash_of(&lab.app, target), "{s0}");
    assert_eq!(revision(&s0), lab.app.session.catalog.revision, "{s0}");
    assert_eq!((s0["activePhoto"].as_u64(), s0["interaction"].as_bool()), (Some(active.0), Some(false)), "{s0}");
    assert_eq!((&s0["historyHead"], &s0["redoHead"]), (&history_head(&lab.app), &redo_head(&lab.app)), "{s0}");
    let active_hash = hash_of(&lab.app, active);
    assert_eq!(
        (library["photoId"].as_u64(), library["hash"].as_str()),
        (Some(active.0), Some(active_hash.as_str())),
        "without an id: the active photo"
    );

    let length = lab.app.session.undo.len();
    lab.app.run("develop.set", json!({"ids": [target.0], "values": {"light.exposure": 0.8}})).unwrap_or_else(|e| panic!("an edit: {e}"));
    let s1 = state(&mut lab.app, target);
    assert_ne!(s1["hash"], s0["hash"], "an edit changes the hash");
    assert!(revision(&s1) > revision(&s0), "an edit is a new revision");
    assert_eq!(s1["historyHead"], json!({"length": length + 1, "label": "Exposure"}), "{s1}");
    assert_eq!(s1["redoHead"], json!({"length": 0, "label": null}), "{s1}");
    lab.app.run("edit.undo", json!({})).unwrap_or_else(|e| panic!("an undo: {e}"));
    let s2 = state(&mut lab.app, target);
    assert_eq!(s2["hash"], s0["hash"], "an undo brings the earlier content back");
    assert!(revision(&s2) > revision(&s1), "an undo is a new revision");
    assert_eq!(s2["historyHead"], s0["historyHead"], "{s2}");
    assert_eq!(s2["redoHead"], json!({"length": 1, "label": "Exposure"}), "{s2}");
    lab.app.run("edit.redo", json!({})).unwrap_or_else(|e| panic!("a redo: {e}"));
    let s3 = state(&mut lab.app, target);
    assert_eq!(s3["hash"], s1["hash"], "a redo brings the edit back");
    assert!(revision(&s3) > revision(&s2), "a redo is a new revision");
    assert_eq!(s3["historyHead"], s1["historyHead"], "{s3}");
    settle(&mut lab);

    let (h0, h1, head) = (s0["hash"].clone(), s1["hash"].clone(), s3["historyHead"].clone());
    let not_this_photos = json!({"id": active.0, "expectHash": active_hash, "expectHistoryHead": head});
    for (params, code) in [
        (json!({"id": target.0, "expectHash": h1, "expectHistoryHead": s0["historyHead"]}), "history_head_conflict"),
        (json!({"id": target.0, "expectHash": h1, "expectHistoryHead": {"length": length + 1, "label": "Contrast"}}), "history_head_conflict"),
        (json!({"id": target.0, "expectHash": h0, "expectHistoryHead": head}), "settings_hash_conflict"),
        (not_this_photos, "history_head_conflict"),
        (json!({"id": UNKNOWN, "expectHash": h1, "expectHistoryHead": head}), "target_not_found"),
    ] {
        let what = format!("lts.undo {params}");
        let before = Witness::of(&lab.app);
        let reply = lts(&mut lab.app, "lts.undo", params);
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
        assert_unchanged(&mut lab, &before, &what);
    }
    let undo = lts(&mut lab.app, "lts.undo", json!({"id": target.0, "expectHash": h1, "expectHistoryHead": head}));
    assert_eq!(undo["outcome"], "applied", "{undo}");
    assert_eq!((undo["label"].as_str(), &undo["hashBefore"], &undo["hashAfter"]), (Some("Exposure"), &h1, &h0), "{undo}");
    let s4 = state(&mut lab.app, target);
    assert_eq!(s4["hash"], h0, "{s4}");
    assert!(revision(&s4) > revision(&s3), "a guarded undo is a new revision");
    assert_eq!((&s4["historyHead"], &s4["redoHead"]), (&s0["historyHead"], &json!({"length": 1, "label": "Exposure"})), "{s4}");
    assert_eq!((&undo["historyHead"], &undo["redoHead"], revision(&undo)), (&s4["historyHead"], &s4["redoHead"], revision(&s4)), "{undo}");
    assert!(written(&lab).iter().any(|name| name == "catalog.log"), "the undo is written to the library");

    let redo_head_now = s4["redoHead"].clone();
    for (params, code) in [
        (json!({"id": target.0, "expectHash": h0, "expectRedoHead": {"length": 1, "label": "Contrast"}}), "history_head_conflict"),
        (json!({"id": target.0, "expectHash": h1, "expectRedoHead": redo_head_now}), "settings_hash_conflict"),
    ] {
        let what = format!("lts.redo {params}");
        let before = Witness::of(&lab.app);
        let reply = lts(&mut lab.app, "lts.redo", params);
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
        assert_unchanged(&mut lab, &before, &what);
    }
    let redo = lts(&mut lab.app, "lts.redo", json!({"id": target.0, "expectHash": h0, "expectRedoHead": redo_head_now}));
    assert_eq!((redo["outcome"].as_str(), &redo["hashBefore"], &redo["hashAfter"]), (Some("applied"), &h0, &h1), "{redo}");
    let s5 = state(&mut lab.app, target);
    assert_eq!(s5["hash"], h1, "{s5}");
    assert!(revision(&s5) > revision(&s4), "a guarded redo is a new revision");
    assert_eq!(s5["historyHead"], head, "{s5}");
    assert_eq!(lab.app.session.journal.len(), journal + 3, "the human's edit, undo and redo are journaled; the guarded ones are not");
    settle(&mut lab);

    lab.app.run("develop.beginInteraction", json!({"label": "Exposure"})).unwrap_or_else(|e| panic!("an interaction: {e}"));
    settle(&mut lab);
    assert_eq!(state(&mut lab.app, target)["interaction"], true, "an interaction is in progress");
    let what = "lts.undo during a human interaction";
    let before = Witness::of(&lab.app);
    let reply = lts(&mut lab.app, "lts.undo", json!({"id": target.0, "expectHash": h1, "expectHistoryHead": head}));
    assert_eq!(refusal(&reply), "editor_busy", "{what}: {reply}");
    assert_unchanged(&mut lab, &before, what);
    lab.app.run("develop.cancelInteraction", json!({})).unwrap_or_else(|e| panic!("cancelling the interaction: {e}"));
    assert_eq!(state(&mut lab.app, target)["interaction"], false, "the interaction ended");

    lab.app.run("develop.set", json!({"values": {"light.contrast": 15}})).unwrap_or_else(|e| panic!("an edit of the active photo: {e}"));
    settle(&mut lab);
    let what = "lts.undo of a step that is another photo's";
    let params = json!({"id": target.0, "expectHash": hash_of(&lab.app, target), "expectHistoryHead": history_head(&lab.app)});
    let before = Witness::of(&lab.app);
    let reply = lts(&mut lab.app, "lts.undo", params);
    assert_eq!(refusal(&reply), "history_head_conflict", "{what}: {reply}");
    assert_unchanged(&mut lab, &before, what);
}

/// T-LI-09: `Originals::install` gives the engine the originals' bytes (`media.file_bytes`). The web
/// build has no file system, so without it `photo.allMetadata` (and every other reader of a photo's
/// file) fails for an imported photo. The reader serves exactly the stored originals and nothing
/// else; `photo.allMetadata` then lists the file's EXIF (position, notice and camera) and changes
/// nothing.
#[test]
fn file_bytes_installed_all_metadata_works_for_an_imported_photo() {
    let (jpeg, camera) = tagged_jpeg();
    let mut lab = boot(Presentation::LtsEmbedded, false);
    let read = lab.app.session.media.file_bytes.clone().expect("Originals::install installs media.file_bytes");
    let id = import(&mut lab, "tagged.jpg", &jpeg);
    let path = match &photo(&lab.app, id).source {
        Source::File { path } => path.clone(),
        other => panic!("an imported photo has a file: {other:?}"),
    };
    assert!(read(&path).as_deref() == Ok(jpeg.as_slice()), "the reader serves the stored original exactly");
    for elsewhere in ["web/0123456789abcdef0123456789abcdef/other.jpg", "C:/Windows/win.ini", "/etc/hosts", "tagged.jpg", ""] {
        assert!(read(elsewhere).is_err(), "{elsewhere:?}: only stored originals are read, never the file system");
    }
    let meta = photo(&lab.app, id).meta;
    let near = |(lat, lon): (f64, f64)| (lat - GPS[0]).abs() < 1e-4 && (lon - GPS[1]).abs() < 1e-4;
    assert!(meta.gps.is_some_and(near), "the import read the position: {:?}", meta.gps);
    assert_eq!((meta.camera.as_str(), meta.copyright.as_str()), (camera.as_str(), COPYRIGHT), "the import read the camera and the notice");

    let before = Witness::of(&lab.app);
    let all = lab.app.run("photo.allMetadata", json!({"id": id.0})).unwrap_or_else(|e| panic!("photo.allMetadata of an imported photo: {e}"));
    assert_unchanged(&mut lab, &before, "photo.allMetadata");
    let rows = all["exif"].as_array().cloned().unwrap_or_default();
    assert!(rows.iter().any(|row| row["group"] == "GPS" && row["name"] == "GPS Latitude"), "the file's position: {all}");
    assert!(rows.iter().any(|row| row["value"] == COPYRIGHT), "the file's copyright notice: {all}");
    assert!(rows.iter().any(|row| row["value"] == camera.as_str()), "the file's camera model: {all}");
}

/// T-LI-10: `lts.develop` edits the photo it names in one engine command, with no photo active (an
/// embedded library before the human picks one): the values exactly, one undo step and history step
/// with the given label, one catalog step, written to the library, never journaled, nothing
/// selected, the status line untouched. A stale `expectHash` or `expectHistoryHead` (each answering
/// the current one), an unknown photo, values out of range or malformed, and a human interaction in
/// progress are refused with nothing changed. Complete settings restore a checkpoint exactly, and
/// only with `expectHash` (R1): without it they are refused.
#[test]
fn lts_develop_atomic_expect_hash_busy_and_no_active_photo_needed() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    deselect_all(&mut lab);
    let what = "lts.develop with no photo active";
    let old = lab.app.session.develop_of(target).expect("the target");
    let (h0, head0) = (hash_of(&lab.app, target), history_head(&lab.app));
    let (journal, undo, revision) = (lab.app.session.journal.len(), lab.app.session.undo.len(), lab.app.session.catalog.revision);
    let (selection, picked) = (lab.app.session.selection.clone(), (lab.app.session.active_mask, lab.app.session.active_spot));
    let others = photos_except(&lab.app, target);
    let params = json!({
        "id": target.0,
        "values": {"light.exposure": 1.25, "light.contrast": 20},
        "expectHash": h0,
        "expectHistoryHead": head0,
        "label": "LTS develop",
    });
    let reply = lts(&mut lab.app, "lts.develop", params);
    assert_eq!(reply["outcome"], "applied", "{what}: {reply}");
    let new = lab.app.session.develop_of(target).expect("the target");
    assert_eq!((new.light.exposure, new.light.contrast), (1.25, 20.0), "{what}: the values are applied");
    assert_eq!((reply["photoId"].as_u64(), reply["label"].as_str()), (Some(target.0), Some("LTS develop")), "{what}: {reply}");
    let h1 = hash_of(&lab.app, target);
    assert_eq!((&reply["hashBefore"], &reply["hashAfter"]), (&json!(h0), &json!(h1)), "{what}: the tokens");
    assert_ne!(h1, h0, "{what}: the content changed");
    let delta = json!({"light.exposure": [old.light.exposure, 1.25], "light.contrast": [old.light.contrast, 20.0]});
    assert_eq!(reply["delta"], delta, "{what}: each value before and after");
    assert_eq!((reply["intentSatisfied"].as_bool(), reply["wbModeChanged"].as_bool()), (Some(true), Some(false)), "{what}: {reply}");
    let s = &lab.app.session;
    assert_eq!((s.undo.len(), s.undo.last().map(|step| step.label.as_str())), (undo + 1, Some("LTS develop")), "{what}: one undo step, labelled");
    assert!(s.redo.is_empty(), "{what}: a new step clears redo");
    let history = s.catalog.photo(target).and_then(|p| p.history.last()).map(|step| step.label.clone());
    assert_eq!(history.as_deref(), Some("LTS develop"), "{what}: in the photo's history too");
    assert_eq!(s.journal.len(), journal, "{what}: never journaled");
    assert_eq!(s.catalog.revision, revision + 1, "{what}: one catalog step");
    assert_eq!(reply["catalogRevision"], s.catalog.revision, "{what}");
    assert_eq!((&s.selection, (s.active_mask, s.active_spot)), (&selection, picked), "{what}: nothing selected");
    assert_eq!(lab.app.ui.status, STATUS, "{what}: the status line is untouched");
    let head1 = history_head(&lab.app);
    assert_eq!(reply["historyHead"], head1, "{what}");
    assert_same_photos(&photos_except(&lab.app, target), &others, what);
    assert!(written(&lab).iter().any(|name| name == "catalog.log"), "{what}: written to the library like any command");

    let too_many: serde_json::Map<String, Value> = CONTROLS.iter().take(65).map(|c| (c.id.to_string(), json!(c.default))).collect();
    let checkpoint = new.to_json();
    let mut misspelt = checkpoint.clone();
    misspelt["light"]["exposur"] = json!(1.0);
    for (params, code) in [
        (json!({"id": target.0, "values": {"light.exposure": 2.0}, "expectHash": h0}), "settings_hash_conflict"),
        (json!({"id": target.0, "values": {"light.exposure": 2.0}, "expectHistoryHead": head0}), "history_head_conflict"),
        (json!({"id": UNKNOWN, "values": {"light.exposure": 2.0}}), "target_not_found"),
        (json!({"id": target.0, "values": {"light.exposure": 7.5}}), "out_of_range"),
        (json!({"id": target.0, "values": {"wb.temp": 1000}}), "out_of_range"),
        (json!({"id": target.0, "values": {"light.bogus": 1}}), "invalid_params"),
        (json!({"id": target.0, "values": {"light.exposure": "2"}}), "invalid_params"),
        (json!({"id": target.0, "values": {}}), "invalid_params"),
        (json!({"id": target.0, "values": too_many}), "invalid_params"),
        (json!({"id": target.0}), "invalid_params"),
        (json!({"id": target.0, "values": {"light.exposure": 2.0}, "settings": checkpoint}), "invalid_params"),
        (json!({"id": target.0, "settings": {"light": {"exposure": "bright"}}}), "invalid_params"),
        (json!({"id": target.0, "values": {"light.exposure": 2.0}, "expectHash": "not-a-hash"}), "invalid_params"),
        (json!({"id": target.0, "values": {"light.exposure": 2.0}, "label": "x".repeat(201)}), "invalid_params"),
        (json!({"id": target.0, "values": {"light.exposure": 2.0}, "label": "LTS\ndevelop"}), "invalid_params"),
        (json!({"id": target.0, "values": {"light.exposure": 2.0}, "label": "   "}), "invalid_params"),
        (json!({"id": target.0, "values": {"pointColor.0.hueShift": 10}}), "invalid_params"),
        (json!({"id": target.0, "settings": misspelt}), "invalid_params"),
    ] {
        let what = format!("lts.develop {params}");
        let before = Witness::of(&lab.app);
        let reply = lts(&mut lab.app, "lts.develop", params);
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
        if code == "settings_hash_conflict" {
            assert_eq!(reply["actual"], h1, "{what}: names the current token to refresh from");
        }
        if code == "history_head_conflict" {
            assert_eq!(reply["actual"], head1, "{what}: names the current head to refresh from");
        }
        assert_unchanged(&mut lab, &before, &what);
    }
    // complete settings replace what the caller must have read first: they need its token
    let blind = json!({"id": target.0, "settings": checkpoint});
    refuses_saying(&mut lab, "lts.develop", blind, "invalid_params", "`expectHash` is required with complete settings");

    lab.app.run("library.select", json!({"ids": [active.0]})).unwrap_or_else(|e| panic!("selecting a photo: {e}"));
    lab.app.run("develop.beginInteraction", json!({"label": "Exposure"})).unwrap_or_else(|e| panic!("an interaction: {e}"));
    settle(&mut lab);
    let what = "lts.develop during a human interaction";
    let params = json!({"id": target.0, "values": {"light.exposure": 2.0}, "expectHash": h1});
    let before = Witness::of(&lab.app);
    let reply = lts(&mut lab.app, "lts.develop", params);
    assert_eq!(refusal(&reply), "editor_busy", "{what}: {reply}");
    assert_unchanged(&mut lab, &before, what);
    lab.app.run("develop.cancelInteraction", json!({})).unwrap_or_else(|e| panic!("cancelling the interaction: {e}"));
    settle(&mut lab);

    let changed = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "values": {"light.exposure": -0.5}, "expectHash": h1}));
    assert_eq!(changed["outcome"], "applied", "{changed}");
    let params = json!({"id": target.0, "settings": checkpoint, "expectHash": changed["hashAfter"], "label": "Restore checkpoint"});
    let restored = lts(&mut lab.app, "lts.develop", params);
    assert_eq!((restored["outcome"].as_str(), &restored["hashAfter"]), (Some("applied"), &json!(h1)), "a restore brings the token back: {restored}");
    assert_eq!(lab.app.session.develop_of(target).map(|d| d.to_json()), Some(checkpoint), "and the checkpoint's settings");
    assert_eq!(lab.app.session.undo.last().map(|step| step.label.as_str()), Some("Restore checkpoint"), "as one labelled step");
}

/// T-LI-10b: on identical input, `lts.develop` with `values` leaves the catalog exactly as
/// `develop.set` with `ids` does: the same settings (the as-shot rule included: in As Shot mode a
/// white balance value starts from the as-shot temperature and tint and switches to Custom), the same
/// history step and edit time, the same undo label and one catalog step each; undoing either restores
/// the same photo.
#[test]
fn lts_develop_matches_develop_set_on_identical_input() {
    let mut set = editing_lab(Presentation::LtsEmbedded);
    let mut hook = editing_lab(Presentation::LtsEmbedded);
    let target = other_photo(&set.app, active_photo(&set.app));
    assert_same_photos(&all_photos(&set.app), &all_photos(&hook.app), "two boots of the same library");
    let label = |lab: &Lab| lab.app.session.undo.last().map(|step| step.label.clone());
    for (values, stale, white_balance) in [
        (json!({"wb.temp": 5200}), true, Some((5200.0, 0.0))),
        (json!({"wb.tint": 12, "light.contrast": 20}), true, Some((6500.0, 12.0))),
        (json!({"light.exposure": 0.75}), false, None),
    ] {
        let what = format!("{values}");
        if stale {
            stale_as_shot(&mut set, target);
            stale_as_shot(&mut hook, target);
        }
        let mode = set.app.session.develop_of(target).expect("the target").wb.mode;
        let revisions = (set.app.session.catalog.revision, hook.app.session.catalog.revision);
        set.app.run("develop.set", json!({"ids": [target.0], "values": values})).unwrap_or_else(|e| panic!("{what}: develop.set: {e}"));
        let reply = lts(&mut hook.app, "lts.develop", json!({"id": target.0, "values": values}));
        assert_eq!(reply["outcome"], "applied", "{what}: {reply}");
        assert_same_photos(&all_photos(&set.app), &all_photos(&hook.app), &what);
        assert_eq!(label(&hook), label(&set), "{what}: the undo label");
        assert_eq!(reply["label"].as_str(), label(&set).as_deref(), "{what}: the reply names it");
        assert_eq!(
            (set.app.session.undo.len(), set.app.session.redo.len()),
            (hook.app.session.undo.len(), hook.app.session.redo.len()),
            "{what}: the undo and redo stacks"
        );
        let steps = (set.app.session.catalog.revision - revisions.0, hook.app.session.catalog.revision - revisions.1);
        assert_eq!(steps, (1, 1), "{what}: one catalog step each");
        let wb = hook.app.session.develop_of(target).expect("the target").wb;
        if let Some((temp, tint)) = white_balance {
            assert_eq!((wb.temp, wb.tint, wb.mode), (temp, tint, WbMode::Custom), "{what}: the as-shot rule");
        }
        assert_eq!(reply["wbModeChanged"], wb.mode != mode, "{what}: {reply}");
        assert_eq!(reply["intentSatisfied"], true, "{what}: {reply}");
        settle(&mut set);
        settle(&mut hook);
    }
    for lab in [&mut set, &mut hook] {
        lab.app.run("edit.undo", json!({})).unwrap_or_else(|e| panic!("an undo: {e}"));
    }
    assert_same_photos(&all_photos(&set.app), &all_photos(&hook.app), "after undoing either step");
}

/// T-LI-11: the page seeds the demo photos into a new library unless it is embedded (`?embedded`, the
/// LTS host's presentation) without `?demo` (owner decision OC-5). `Options::seed_demo` reads the
/// query as the page's own `URLSearchParams` does (the exact name, any value) and the boot opens the
/// library with it: an embedded library starts empty, with no photo active and no album, and its
/// first import is photo 1. The flag changes no other option.
#[test]
fn embedded_boot_seeds_no_demo_unless_demo_flag() {
    let demo_photos = Session::with_demo().catalog.len();
    assert!(demo_photos > 1, "the demo library has photos");
    for (search, demo, seeds) in [
        ("", false, true),
        ("?store=memory&workers=1", false, true),
        ("?demo", true, true),
        ("?store=memory&workers=1&embedded", false, false),
        ("?embedded", false, false),
        ("?embedded&demo", true, true),
        ("?store=memory&workers=1&embedded&demo=1", true, true),
        ("?embedded&demo=", true, true),
        ("?embedded&%64emo", true, true),
        ("?embedded&nodemo", false, false),
        ("?embedded&demox", false, false),
        ("?embedded&x=demo", false, false),
        ("?embedded&Demo", false, false),
        ("?embedded&demo%3D1", false, false),
    ] {
        let options = Options::parse(search);
        assert_eq!((options.demo, options.seed_demo()), (demo, seeds), "{search}");
        let mut lab = boot(options.presentation(), options.seed_demo());
        let s = &lab.app.session;
        assert_eq!(s.catalog.len(), if seeds { demo_photos } else { 0 }, "{search}: the photos at boot");
        if !seeds {
            assert!(s.active().is_none() && s.catalog.albums().next().is_none(), "{search}: no photo active, no album");
            assert_eq!(import(&mut lab, "first.png", &picked_png()), PhotoId(1), "{search}: the first import is photo 1");
        }
    }
    let plain = Options::parse("?store=memory&workers=1&embedded");
    let with_demo = Options::parse("?store=memory&workers=1&embedded&demo");
    assert_eq!(Options { demo: false, ..with_demo }, plain, "the flag changes no other option");
}

/// T-LI-12: nothing but the fifteen `lts.*` commands of the hook's table (`POLICY`: the nine reads and
/// the six mutations the adapter maps, each in both runtimes, which answer alike) is reachable
/// through the hook, and none passes anything through. Other ids are not the hook's: in the LTS
/// presentation the page refuses them at the fork (`command_not_allowed`, R1: T-LI-21), the hook's
/// own entry (`handle_in`) answers none of them, and the standalone page runs them as upstream does
/// (`None`: the hook ran nothing); unknown and tier-3 ids under `lts.` are `command_not_allowed`, and
/// so are the ids that presets, sync, merges, history steps, the section commands and the engine's
/// own reads would have had (R0: the one new mutation is `lts.restoreSettings`, the others ride
/// `lts.develop`; R1: the reads and the selection are the table's own ids); unknown keys (paths,
/// commands, extra targets, the engine's filters, sorts and modes), formats and policies that would
/// reach files or metadata, photo ids that are not explicit integers from 1, pages, limits and file
/// names out of range, patches of sections a patch does not set (crop, orientation, geometry,
/// sliders) or with keys of their own, presets with keys beyond `preset.apply`'s, the parameters a
/// sync would take and restores without their tokens are `invalid_params`. Nothing changes. An entry
/// that only a native worker runs runs only there: driven through the hook with a table that holds
/// one, the browser refuses it as `native_only` and a native worker answers it as the hook's own
/// entry.
#[test]
fn lts_view_rejects_unknown_and_tier3_ids() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    let (hash, head, redo) = (hash_of(&lab.app, target), history_head(&lab.app), redo_head(&lab.app));
    let current = settings_json(&lab.app, target);
    let before = Witness::of(&lab.app);

    let table: Vec<&str> = POLICY.iter().map(|entry| entry.command).collect();
    let expected: Vec<&str> = READS.iter().chain(MUTATIONS.iter()).copied().collect();
    assert_eq!(table, expected, "the hook's commands, in the table's order");
    for entry in POLICY {
        assert_eq!(entry.mutation, MUTATIONS.contains(&entry.command), "{entry:?}: a mutation answers applied, a read ok");
        assert_eq!(entry.runtimes, Runtimes::Both, "{entry:?}: no command of the hook is native-only today");
        assert!(entry.runs_in(Runtime::Browser) && entry.runs_in(Runtime::Native), "{entry:?}");
    }
    let native = Entry { command: "lts.example", keys: &[], mutation: false, runtimes: Runtimes::NativeOnly };
    assert!(!native.runs_in(Runtime::Browser) && native.runs_in(Runtime::Native), "a native-only entry runs in a native worker only");
    let native_state = [Entry { command: "lts.state", keys: &["id"], mutation: false, runtimes: Runtimes::NativeOnly }];
    let refused = lts_by(&native_state, &mut lab.app, "lts.state", json!({"id": target.0}), Runtime::Browser);
    assert_eq!(refusal(&refused), "native_only", "a native-only entry in the browser: {refused}");
    let answered = lts_by(&native_state, &mut lab.app, "lts.state", json!({"id": target.0}), Runtime::Native);
    assert_eq!(answered, lts(&mut lab.app, "lts.state", json!({"id": target.0})), "a native worker runs the native-only entry");
    assert_eq!(Runtime::HERE, Runtime::Native, "these tests run the native side");
    let in_browser = lts_in(&mut lab.app, "lts.state", json!({"id": target.0}), Runtime::Browser);
    assert_eq!(in_browser, lts(&mut lab.app, "lts.state", json!({"id": target.0})), "both runtimes answer alike");

    let tempting = json!({"id": target.0, "ids": [target.0], "values": {"light.exposure": 3.0}, "path": "C:/Users/Public/out.jpg"});
    let outside = [
        "library.state",
        "develop.set",
        "app.export",
        "web.stats",
        "edit.undo",
        "ui.menu.invoke",
        "",
        "lts",
        "LTS.state",
        "Lts.preview",
        " lts.state",
        "ltsx.state",
        "x.lts.state",
    ];
    for id in outside {
        let reply = at_the_fork(&mut lab.app, id, tempting.clone());
        assert_eq!(refusal(&reply), "command_not_allowed", "{id:?} is not an lts.* id: the LTS editor refuses it at the fork: {reply}");
        let hook = handle_in(&mut lab.app.session, id, &tempting, Runtime::Native);
        assert!(hook.is_none(), "{id:?}: the hook's own entry takes lts.* ids only");
    }
    assert_unchanged(&mut lab, &before, "ids outside lts.");
    let mut upstream = editing_lab(Presentation::Standalone);
    for id in outside {
        assert!(handle(&mut upstream.app, id, &tempting).is_none(), "{id:?}: the standalone page runs it as upstream does");
    }

    for id in [
        "lts.",
        "lts.bogus",
        "lts.State",
        "lts.state ",
        "lts.preview.x",
        "lts.develop.set",
        "lts.app.export",
        "lts.app.quit",
        "lts.ui.menu.invoke",
        "lts.ui.key",
        "lts.prefs.set",
        "lts.script.run",
        "lts.library.import",
        "lts.photo.delete",
        "lts.run",
        "lts.execute",
        "lts.command",
        "lts.preset",
        "lts.preset.apply",
        "lts.presets.list",
        "lts.sync",
        "lts.develop.sync",
        "lts.develop.copy",
        "lts.develop.paste",
        "lts.develop.merge",
        "lts.merge",
        "lts.patch",
        "lts.develop.wb",
        "lts.develop.curve",
        "lts.develop.profile",
        "lts.develop.treatment",
        "lts.mask.add",
        "lts.spot.add",
        "lts.redeye.add",
        "lts.segment.model.download",
        "lts.library.open",
        "lts.restore",
        "lts.restoresettings",
        "lts.RestoreSettings",
        "lts.history.restore",
        "lts.Library",
        "lts.library ",
        "lts.library.query",
        "lts.catalog.query",
        "lts.query",
        "lts.photos",
        "lts.photo.inspect",
        "lts.inspect",
        "lts.settings",
        "lts.develop.get",
        "lts.get",
        "lts.control",
        "lts.develop.controls",
        "lts.controls.list",
        "lts.Presets",
        "lts.profiles",
        "lts.profiles.menu",
        "lts.curve.presets",
        "lts.allMetadata",
        "lts.photo.allMetadata",
        "lts.metadata.set",
        "lts.history",
        "lts.history.list",
        "lts.journal.list",
        "lts.web.stats",
        "lts.stats",
        "lts.Select",
        "lts.select.photo",
        "lts.library.select",
        "lts.view.detail",
        "lts.detail",
    ] {
        let reply = lts(&mut lab.app, id, tempting.clone());
        assert_eq!(refusal(&reply), "command_not_allowed", "{id:?}: {reply}");
    }
    assert_unchanged(&mut lab, &before, "unknown and tier-3 lts.* ids");

    for (id, params) in [
        ("lts.state", json!({"id": target.0, "path": "C:/Users/Public"})),
        ("lts.state", json!([target.0])),
        ("lts.state", json!("lts.state")),
        ("lts.state", Value::Null),
        ("lts.preview", json!({"id": target.0, "longEdge": 64, "path": "C:/Users/Public/out.png"})),
        ("lts.preview", json!({"id": target.0, "longEdge": 64, "command": "app.quit"})),
        ("lts.preview", json!({"id": target.0, "longEdge": 64, "ids": [active.0]})),
        ("lts.export", json!({"id": target.0, "format": "jpeg", "longEdge": 64, "path": "C:/Users/Public/out.jpg"})),
        ("lts.export", json!({"id": target.0, "format": "jpeg", "longEdge": 64, "dir": "C:/Users/Public"})),
        ("lts.export", json!({"id": target.0, "format": "original", "longEdge": 64})),
        ("lts.export", json!({"id": target.0, "format": "dng", "longEdge": 64})),
        ("lts.export", json!({"id": target.0, "format": "jpeg", "longEdge": 64, "metadata": "all"})),
        ("lts.export", json!({"id": target.0, "format": "jpeg", "longEdge": 64, "watermark": "x"})),
        ("lts.develop", json!({"id": target.0, "values": {"light.exposure": 1.0}, "command": "app.quit"})),
        ("lts.develop", json!({"id": target.0, "values": {"app.quit": 1.0}})),
        ("lts.develop", json!({"id": target.0, "values": {"light.exposure": 1.0}, "ids": [active.0, target.0]})),
        ("lts.crop", json!({"id": target.0, "rect": [0.1, 0.1, 0.9, 0.9], "ids": [active.0]})),
        ("lts.undo", json!({"id": target.0, "expectHash": hash, "expectHistoryHead": head, "steps": 2})),
        ("lts.redo", json!({"id": target.0, "expectHash": hash, "expectRedoHead": redo, "force": true})),
        ("lts.develop", json!({"id": target.0, "settings": {"crop": current["crop"]}})),
        ("lts.develop", json!({"id": target.0, "settings": {"orientation": current["orientation"]}})),
        ("lts.develop", json!({"id": target.0, "settings": {"geometry": current["geometry"]}})),
        ("lts.develop", json!({"id": target.0, "settings": {"treatment": "bw", "path": "C:/Users/Public/out.jpg"}})),
        ("lts.develop", json!({"id": target.0, "settings": {"wb": {"mode": "custom", "temp": 5000.0, "tint": 0.0, "command": "app.quit"}}})),
        ("lts.develop", json!({"id": target.0, "preset": {"id": "lc.warm-glow", "path": "C:/Users/Public/look.lcpreset"}})),
        ("lts.develop", json!({"id": target.0, "preset": {"id": "lc.warm-glow", "ids": [active.0, target.0]}})),
        ("lts.develop", json!({"id": target.0, "preset": "lc.warm-glow"})),
        ("lts.develop", json!({"id": target.0, "preset": {"id": "lc.warm-glow"}, "values": {"light.exposure": 1.0}})),
        ("lts.develop", json!({"id": target.0, "sync": {"from": active.0}})),
        ("lts.develop", json!({"id": target.0, "groups": ["light"], "from": active.0})),
        ("lts.preview", json!({"id": target.0, "longEdge": 64, "variant": "settings", "settings": {"light": current["light"]}})),
        ("lts.preview", json!({"id": target.0, "longEdge": 64, "variant": "settings", "preset": {"id": "lc.warm-glow", "file": "C:/x"}})),
        ("lts.restoreSettings", json!({"id": target.0, "revision": hash, "expectHash": hash, "settings": current})),
        ("lts.restoreSettings", json!({"id": target.0, "revision": hash, "expectHash": hash, "index": 0})),
        ("lts.restoreSettings", json!({"id": target.0, "expectHash": hash})),
        ("lts.restoreSettings", json!({"id": target.0, "revision": hash})),
        ("lts.restoreSettings", json!({"id": target.0, "revision": "not-a-token", "expectHash": hash})),
        ("lts.restoreSettings", json!({"id": target.0, "revision": "ABCDEF0123456789", "expectHash": hash})),
        ("lts.state", json!({"id": target.0, "settings": "yes"})),
        ("lts.state", json!({"id": target.0, "settings": {"light": current["light"]}})),
        ("lts.library", json!({"filter": {}})),
        ("lts.library", json!({"sort": {"key": "rating"}})),
        ("lts.library", json!({"id": target.0})),
        ("lts.library", json!({"limit": 0})),
        ("lts.library", json!({"limit": 201})),
        ("lts.library", json!({"limit": "200"})),
        ("lts.library", json!({"offset": -1})),
        ("lts.library", json!({"offset": 1.5})),
        ("lts.library", json!({"fileName": ""})),
        ("lts.library", json!({"fileName": 7})),
        ("lts.library", json!({"fileName": "x".repeat(256)})),
        ("lts.photo", json!({"id": target.0, "include": ["gps"]})),
        ("lts.photo", json!({"id": target.0, "path": "C:/Users/Public"})),
        ("lts.controls", json!({"id": target.0, "section": "light"})),
        ("lts.controls", json!({"values": {"light.exposure": 1.0}})),
        ("lts.presets", json!({"id": target.0})),
        ("lts.presets", json!({"path": "C:/Users/Public/look.lcpreset"})),
        ("lts.metadata", json!({"id": target.0, "path": "C:/Users/Public/out.jpg"})),
        ("lts.metadata", json!({"id": target.0, "include": ["gps"]})),
        ("lts.journal", json!({"limit": 0})),
        ("lts.journal", json!({"limit": 10_001})),
        ("lts.journal", json!({"limit": -5})),
        ("lts.journal", json!({"since": 3})),
        ("lts.select", json!({"id": target.0, "ids": [active.0, target.0]})),
        ("lts.select", json!({"id": target.0, "mode": "add"})),
        ("lts.select", json!({"id": target.0, "active": target.0})),
        ("lts.select", json!({"id": target.0, "detail": "yes"})),
        ("lts.select", json!({"id": target.0, "view": "compare"})),
    ] {
        let what = format!("{id} {params}");
        let reply = lts(&mut lab.app, id, params);
        assert_eq!(refusal(&reply), "invalid_params", "{what}: {reply}");
    }
    assert_unchanged(&mut lab, &before, "unknown keys, formats and policies");

    let commands = [
        ("lts.state", json!({})),
        ("lts.preview", json!({"longEdge": 64})),
        ("lts.export", json!({"format": "png", "longEdge": 64})),
        ("lts.develop", json!({"values": {"light.exposure": 1.0}})),
        ("lts.crop", json!({"rect": [0.1, 0.1, 0.9, 0.9]})),
        ("lts.undo", json!({"expectHash": hash, "expectHistoryHead": head})),
        ("lts.redo", json!({"expectHash": hash, "expectRedoHead": redo})),
        ("lts.restoreSettings", json!({"revision": hash, "expectHash": hash})),
        ("lts.photo", json!({})),
        ("lts.controls", json!({})),
        ("lts.metadata", json!({})),
        ("lts.select", json!({"detail": true})),
    ];
    for (id, rest) in &commands {
        for bad in [json!(target.0.to_string()), json!(-1), json!(1.5), json!(0), Value::Null, json!([target.0]), json!({"id": target.0})] {
            let params = with(rest, json!({"id": bad}));
            let what = format!("{id} {params}");
            let reply = lts(&mut lab.app, id, params);
            assert_eq!(refusal(&reply), "invalid_params", "{what}: a photo id is an explicit integer from 1: {reply}");
        }
        // the tokens and the catalog without a photo are reads of their own; everything else names one
        if !["lts.state", "lts.controls"].contains(id) {
            let reply = lts(&mut lab.app, id, rest.clone());
            assert_eq!(refusal(&reply), "invalid_params", "{id} without an id: never the active photo: {reply}");
        }
    }
    assert_unchanged(&mut lab, &before, "malformed photo ids");
}

/// T-LI-14: `lts.crop` crops the photo it names, whichever photo is active, exactly as the
/// declared-select path would (`crop.set` and `crop.reset` on that photo, in an identical library),
/// as one undo step that selects nothing, answering the geometry applied; previews then show the
/// crop's frame (`cropped-straightened`) or the whole photo (`uncropped-oriented`). A stale token,
/// an unknown photo, malformed geometry, a rectangle the editor's crop tool would not leave (2 % of
/// the photo or less either way: R0), an angle past 45 degrees and a human interaction are refused
/// with nothing changed (a crop that magnifies the photo's spots and brushes too much: T-LI-20).
/// Without public geometry helpers it answers `hook_unavailable` for every crop and changes nothing
/// (the adapter then keeps the declared-select crop).
#[test]
fn crop_explicit_keeps_selection_or_reports_hook_unavailable() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    let rect = [0.1, 0.1, 0.5, 0.9];
    let params = json!({"id": target.0, "rect": rect, "angle": 0.0, "expectHash": hash_of(&lab.app, target)});
    let (undo, revision, others) = (lab.app.session.undo.len(), lab.app.session.catalog.revision, photos_except(&lab.app, target));
    let before = Witness::of(&lab.app);
    let first = lts(&mut lab.app, "lts.crop", params);
    if first["outcome"] == "not_applied" {
        assert_eq!(refusal(&first), "hook_unavailable", "{first}");
        for params in [json!({"id": target.0, "rect": rect}), json!({"id": target.0, "angle": 5.0}), json!({"id": target.0, "reset": true})] {
            let reply = lts(&mut lab.app, "lts.crop", params);
            assert_eq!(refusal(&reply), "hook_unavailable", "all crops or none: {reply}");
        }
        assert_unchanged(&mut lab, &before, "lts.crop without the hook");
        return;
    }

    let what = "lts.crop of a photo that is not active";
    assert_eq!(first["outcome"], "applied", "{what}: {first}");
    let mut oracle = editing_lab(Presentation::LtsEmbedded);
    oracle.app.run("library.select", json!({"ids": [target.0]})).unwrap_or_else(|e| panic!("the oracle's selection: {e}"));
    oracle.app.run("crop.set", json!({"rect": rect, "angle": 0.0})).unwrap_or_else(|e| panic!("the oracle's crop: {e}"));
    let crop = photo(&lab.app, target).develop.crop;
    assert!(photo(&lab.app, target) == photo(&oracle.app, target), "{what}: the photo crop.set leaves ({crop:?})");
    let g = crop.geometry;
    assert_eq!(first["crop"], json!({"rect": [g.rect.x0, g.rect.y0, g.rect.x1, g.rect.y1], "angle": g.angle}), "{what}: the geometry applied");
    assert_eq!(first["intentSatisfied"], true, "{what}: a rectangle inside the photo is applied as asked");
    assert_eq!(first["hashAfter"], hash_of(&lab.app, target), "{what}: {first}");
    assert_ne!(first["hashAfter"], first["hashBefore"], "{what}: {first}");
    assert_eq!((lab.app.session.undo.len(), lab.app.session.catalog.revision), (undo + 1, revision + 1), "{what}: one undo step, one catalog step");
    let label = |lab: &Lab| lab.app.session.undo.last().map(|step| step.label.clone());
    assert_eq!(label(&lab), label(&oracle), "{what}: the undo label");
    assert_kept(&lab.app, &before, &["catalog revision", "catalog", "undo", "redo"], what);
    assert_same_photos(&photos_except(&lab.app, target), &others, what);
    assert!(written(&lab).iter().any(|name| name == "catalog.log"), "{what}: written to the library");

    let (width, height) = {
        let p = photo(&lab.app, target);
        (f64::from(p.width), f64::from(p.height))
    };
    let cropped = preview(&mut lab.app, target, json!({}));
    let whole = preview(&mut lab.app, target, json!({"crop": "uncropped"}));
    assert_eq!((cropped["crop"].as_str(), cropped["frame"].as_str()), (Some("applied"), Some("cropped-straightened")), "{cropped}");
    assert_eq!((whole["crop"].as_str(), whole["frame"].as_str()), (Some("uncropped"), Some("uncropped-oriented")), "{whole}");
    let aspect = |reply: &Value| reply["width"].as_f64().unwrap_or(f64::NAN) / reply["height"].as_f64().unwrap_or(f64::NAN);
    let crop_aspect = (rect[2] - rect[0]) * width / ((rect[3] - rect[1]) * height);
    assert!((aspect(&cropped) / crop_aspect - 1.0).abs() < 0.02, "the cropped preview has the crop's aspect: {} vs {crop_aspect}", aspect(&cropped));
    assert!((aspect(&whole) / (width / height) - 1.0).abs() < 0.02, "the uncropped preview has the photo's aspect: {}", aspect(&whole));

    for (extra, command, engine) in
        [(json!({"rect": rect, "angle": 7.5}), "crop.set", json!({"rect": rect, "angle": 7.5})), (json!({"reset": true}), "crop.reset", json!({}))]
    {
        let what = format!("lts.crop {extra}");
        let before = Witness::of(&lab.app);
        let reply = lts(&mut lab.app, "lts.crop", with(&json!({"id": target.0}), extra));
        oracle.app.run(command, engine).unwrap_or_else(|e| panic!("the oracle's {command}: {e}"));
        assert_eq!(reply["outcome"], "applied", "{what}: {reply}");
        let crop = photo(&lab.app, target).develop.crop;
        assert!(photo(&lab.app, target) == photo(&oracle.app, target), "{what}: the photo {command} leaves ({crop:?})");
        let g = crop.geometry;
        let applied = [g.rect.x0, g.rect.y0, g.rect.x1, g.rect.y1];
        assert_eq!(reply["crop"], json!({"rect": applied, "angle": g.angle}), "{what}: the geometry applied");
        let as_asked = command == "crop.reset" || (applied == rect && g.angle == 7.5);
        assert_eq!(reply["intentSatisfied"], as_asked, "{what}: {reply}");
        assert_eq!(label(&lab), label(&oracle), "{what}: the undo label");
        assert_kept(&lab.app, &before, &["catalog revision", "catalog", "undo", "redo"], &what);
        // (taking this crop's writes here also lets each refusal below start from a clean library store)
        assert!(written(&lab).iter().any(|name| name == "catalog.log"), "{what}: written to the library");
    }

    // (the reset brought the original settings back, so the first crop's result is the stale token)
    for (params, code) in [
        (json!({"id": target.0, "rect": rect, "expectHash": first["hashAfter"]}), "settings_hash_conflict"),
        (json!({"id": UNKNOWN, "rect": rect}), "target_not_found"),
        (json!({"id": target.0, "rect": [0.1, 0.1, 0.5]}), "invalid_params"),
        (json!({"id": target.0, "rect": [0.1, 0.1, 0.5, "0.9"]}), "invalid_params"),
        (json!({"id": target.0, "rect": [0.1, 0.1, 0.11, 0.9]}), "out_of_range"),
        (json!({"id": target.0, "rect": [0.1, 0.5, 0.9, 0.515]}), "out_of_range"),
        (json!({"id": target.0, "rect": [0.1, 0.1, 3.0, 0.9]}), "invalid_params"),
        (json!({"id": target.0, "rect": [0.5, 0.1, 0.5, 0.9]}), "invalid_params"),
        (json!({"id": target.0, "angle": 60.0}), "out_of_range"),
        (json!({"id": target.0, "angle": "5"}), "invalid_params"),
        (json!({"id": target.0, "rect": rect, "reset": true}), "invalid_params"),
        (json!({"id": target.0, "reset": false}), "invalid_params"),
        (json!({"id": target.0}), "invalid_params"),
    ] {
        let what = format!("lts.crop {params}");
        let before = Witness::of(&lab.app);
        let reply = lts(&mut lab.app, "lts.crop", params);
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
        assert_unchanged(&mut lab, &before, &what);
    }
    lab.app.run("develop.beginInteraction", json!({"label": "Exposure"})).unwrap_or_else(|e| panic!("an interaction: {e}"));
    settle(&mut lab);
    let what = "lts.crop during a human interaction";
    let before = Witness::of(&lab.app);
    let reply = lts(&mut lab.app, "lts.crop", json!({"id": target.0, "rect": rect}));
    assert_eq!(refusal(&reply), "editor_busy", "{what}: {reply}");
    assert_unchanged(&mut lab, &before, what);
    lab.app.run("develop.cancelInteraction", json!({})).unwrap_or_else(|e| panic!("cancelling the interaction: {e}"));
}

/// T-LI-15 (R0, master plan v2 §3.1): `lts.develop {settings}` takes, besides complete settings (a
/// checkpoint), a patch of whole sections, and only of the seven that no control value reaches:
/// `masks`, `spots`, `red_eye`, `curve` (the point curves with the region sliders), `wb` (the mode
/// with its temperature and tint), `profile` and `treatment`, the keys of the settings' own
/// serialization (`develop.get`; the copy groups Red Eye, Tone Curve and White Balance keep them as
/// `red_eye`, `curve` and `wb`). Every other key of that serialization (sliders go through `values`,
/// the crop through `lts.crop`, the rest nowhere), the groups' own names, unknown keys and an empty
/// patch are `invalid_params`, alone or beside an allowed section, with nothing changed. Each allowed
/// section is set exactly as given, on a photo that is not active, as one undo step that keeps every
/// other section and every other photo, and the reply names the sections set and the sliders they
/// moved; a patch previews (variant `settings`) exactly as it then commits. A partial or misspelt
/// section, an unknown mode or treatment, a slider outside its range, an unknown profile, a
/// point-curve point outside 0 to 1 (in the master and in a colour channel) and repeated mask ids
/// are refused with nothing changed, and so is every value the editor's own commands and tools
/// would not leave: a spot of size 40, spot feathers, opacities, points and source offsets out of
/// range, red-eye radii, catchlights, centres and darkening out of range, a mask's own adjustments
/// (exposure, the other sliders, the colour overlay's hue and saturation, the amount) and edge
/// refinement out of range, mask ids past 65535, a brush point at 1e9, brush sizes outside the
/// editor's brush, brush feathers, flows and densities out of range, gradient ends, centres, radii,
/// angles and feathers out of range, colour-range samples and refinements, luminance and depth
/// bounds, object clicks outside the photo and AI edges out of range, more colour-range samples or
/// object clicks than the editor keeps, and more masks, mask components, brush strokes, red-eye
/// corrections, spots, spot points or stroke points than the caps (`out_of_range`); brushes or
/// spots past the renderer's budget are `budget_exceeded`. Patches at the bounds apply, and so does
/// a profile imported as a LUT (the editor has it), while a LUT profile it does not have is refused.
#[test]
fn section_patch_whitelist_refuses_other_sections() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    let current = settings_json(&lab.app, target);
    let keys: Vec<&str> = current.as_object().map(|fields| fields.keys().map(String::as_str).collect()).unwrap_or_default();
    assert!(SECTIONS.iter().all(|section| keys.contains(section)), "the sections are keys of the settings: {keys:?}");
    for (group, key) in [
        (SettingsGroup::Masks, "masks"),
        (SettingsGroup::Spots, "spots"),
        (SettingsGroup::RedEye, "red_eye"),
        (SettingsGroup::ToneCurve, "curve"),
        (SettingsGroup::WhiteBalance, "wb"),
        (SettingsGroup::Profile, "profile"),
        (SettingsGroup::Treatment, "treatment"),
    ] {
        assert_eq!(group.keys(), [key], "{group:?}: the settings key of the copy group");
    }

    // every other section, alone or beside an allowed one, and names that are no settings key
    let before = Witness::of(&lab.app);
    for key in keys.iter().filter(|key| !SECTIONS.contains(*key)) {
        let value = &current[*key];
        for settings in [json!({*key: value}), json!({*key: value, "treatment": "bw"})] {
            let reply = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "settings": settings}));
            assert_eq!(refusal(&reply), "invalid_params", "a patch of `{key}` ({settings}): {reply}");
        }
    }
    for settings in [
        json!({}),
        json!({"redEye": []}),
        json!({"toneCurve": current["curve"]}),
        json!({"whiteBalance": current["wb"]}),
        json!({"bogus": 1}),
        json!({"command": "app.quit"}),
        json!({"path": "C:/Users/Public/settings.json"}),
        json!({"treatment": "bw", "redEye": []}),
        json!("bw"),
        json!([{"treatment": "bw"}]),
        Value::Null,
    ] {
        let reply = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "settings": settings}));
        assert_eq!(refusal(&reply), "invalid_params", "settings {settings}: {reply}");
    }
    assert_unchanged(&mut lab, &before, "patches of what a patch does not set");

    // each allowed section, exactly as given, on a photo that is not active
    let source = settings_json(&lab.app, active);
    assert!(
        source["masks"].as_array().is_some_and(|masks| masks.len() == 1) && source["spots"].as_array().is_some_and(|spots| spots.len() == 1),
        "the setup made a mask and a spot on the active photo: {source}"
    );
    let mut curve = current["curve"].clone();
    curve["master"] = json!([{"x": 0.0, "y": 0.0}, {"x": 0.25, "y": 0.2}, {"x": 0.75, "y": 0.82}, {"x": 1.0, "y": 1.0}]);
    curve["shadows"] = json!(12.0);
    let red_eye =
        json!([{"center": {"x": 0.42, "y": 0.4}, "rx": 0.03, "ry": 0.02, "pupil_size": 50.0, "darken": 60.0, "pet": false, "catchlight": null}]);
    for (key, value) in [
        ("wb", json!({"mode": "daylight", "temp": 5200.0, "tint": 6.0})),
        ("treatment", json!("bw")),
        ("profile", json!({"id": "lc.vivid", "amount": 80.0})),
        ("curve", curve),
        ("masks", source["masks"].clone()),
        ("spots", source["spots"].clone()),
        ("red_eye", red_eye),
    ] {
        let what = format!("a patch of {key}");
        let old = lab.app.session.develop_of(target).expect("the target");
        let was = old.to_json();
        let (undo, revision, others) = (lab.app.session.undo.len(), lab.app.session.catalog.revision, photos_except(&lab.app, target));
        let params = json!({"id": target.0, "settings": {key: value}, "expectHash": hash_of(&lab.app, target)});
        let reply = lts(&mut lab.app, "lts.develop", params);
        assert_eq!(reply["outcome"], "applied", "{what}: {reply}");
        assert_eq!(
            (reply["sections"].clone(), reply["label"].as_str(), reply["intentSatisfied"].as_bool()),
            (json!([key]), Some("Edit"), Some(true)),
            "{what}: {reply}"
        );
        let moved = match key {
            "wb" => vec![("wb.temp", old.wb.temp, 5200.0), ("wb.tint", old.wb.tint, 6.0)],
            "profile" => vec![("profile.amount", old.profile.amount, 80.0)],
            "curve" => vec![("curve.shadows", old.curve.shadows, 12.0)],
            _ => Vec::new(),
        };
        let delta: serde_json::Map<String, Value> =
            moved.into_iter().filter(|(_, from, to)| from != to).map(|(control, from, to)| (control.to_string(), json!([from, to]))).collect();
        assert_eq!(reply["delta"], Value::Object(delta), "{what}: the sliders the section moved");
        assert_eq!(reply["wbModeChanged"], key == "wb" && old.wb.mode != WbMode::Daylight, "{what}: {reply}");
        let now = settings_json(&lab.app, target);
        assert!(same_json(&value, &now[key]), "{what}: the section is set as given: {}", now[key]);
        for (other, kept) in was.as_object().into_iter().flatten().filter(|(other, _)| other.as_str() != key) {
            assert_eq!(&now[other], kept, "{what}: `{other}` is kept");
        }
        assert_eq!((lab.app.session.undo.len(), lab.app.session.catalog.revision), (undo + 1, revision + 1), "{what}: one step");
        assert_same_photos(&photos_except(&lab.app, target), &others, &what);
        assert!(written(&lab).iter().any(|name| name == "catalog.log"), "{what}: written to the library");
    }

    // a patch previews as it then commits
    let patch = json!({"treatment": "color", "wb": {"mode": "custom", "temp": 6100.0, "tint": -4.0}});
    let what_if = preview(&mut lab.app, target, json!({"variant": "settings", "settings": patch}));
    assert_eq!(what_if["outcome"], "ok", "a patch's what-if preview: {what_if}");
    let commit = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "settings": patch, "label": "LTS patch"}));
    assert_eq!(
        (commit["outcome"].as_str(), commit["label"].as_str(), commit["sections"].clone()),
        (Some("applied"), Some("LTS patch"), json!(["wb", "treatment"])),
        "{commit}"
    );
    assert_eq!(commit["hashAfter"], what_if["renderedHash"], "the previewed settings are the ones committed");
    let committed = preview(&mut lab.app, target, json!({}));
    assert!(preview_pixels(&committed, "committed") == preview_pixels(&what_if, "what-if"), "the committed look is the previewed one");
    settle(&mut lab);

    // a section that is not whole, not well formed or not in range
    let patched = settings_json(&lab.app, target);
    let mask = patched["masks"][0].clone();
    let mut odd_mask = mask.clone();
    odd_mask["script"] = json!("app.quit");
    let mut steep = patched["curve"].clone();
    steep["master"] = json!([{"x": 0.0, "y": 0.0}, {"x": 1.5, "y": 1.0}]);
    let mut loud = patched["curve"].clone();
    loud["shadows"] = json!(150.0);
    let mut wide_eye = patched["red_eye"].clone();
    wide_eye[0]["pupil_size"] = json!(150.0);
    let mut eye_without_pet = patched["red_eye"][0].clone();
    if let Some(fields) = eye_without_pet.as_object_mut() {
        fields.remove("pet");
    }
    let spot = patched["spots"][0].clone();
    let eye = patched["red_eye"][0].clone();
    let mut refined = mask.clone();
    refined["refine"] = json!(150.0);
    let shaped = |shape: Value| replaced(&mask, "/components/0/shape", shape);
    let brush_defaults = json!({"feather": 50.0, "flow": 100.0, "density": 100.0, "erase": false, "auto_mask": false});
    let stroke = |points: Vec<Value>, size: f64| with(&brush_defaults, json!({"points": points, "size": size}));
    let brush = |points: Vec<Value>, size: f64| shaped(json!({"kind": "brush", "strokes": [stroke(points, size)]}));
    let zigzag = |n: usize, far: f64| (0..n).map(|i| if i.is_multiple_of(2) { point(0.0, 0.0) } else { point(far, far) }).collect::<Vec<_>>();
    let many_masks: Vec<Value> = (1..=33_u32).map(|id| replaced(&mask, "/id", json!(id))).collect();
    let big_spot = replaced(&replaced(&spot, "/size", json!(0.25)), "/mode", json!("heal"));
    let red_channel = replaced(&patched["curve"], "/red", json!([point(0.0, 0.0), point(1.0, 1.5)]));
    let crowded = replaced(&spot, "/points", json!(vec![point(0.5, 0.5); 5_001]));
    let many_components = replaced(&mask, "/components", json!(vec![mask["components"][0].clone(); 65]));
    let many_strokes = shaped(json!({"kind": "brush", "strokes": vec![stroke(vec![point(0.2, 0.2)], 0.03); 257]}));
    let one_stroke = brush(vec![point(0.2, 0.2)], 0.03);
    let stroke_with = |field: &str, value: f64| replaced(&one_stroke, &format!("/components/0/shape/strokes/0/{field}"), json!(value));
    let range = |shape: Value| json!({"masks": [shaped(shape)]});
    for (settings, code) in [
        (json!({"wb": {"mode": "daylight"}}), "invalid_params"),
        (json!({"wb": {"mode": "daylight", "temperature": 5200.0, "tint": 6.0}}), "invalid_params"),
        (json!({"wb": {"mode": "sunny", "temp": 5200.0, "tint": 6.0}}), "invalid_params"),
        (json!({"wb": {"mode": "custom", "temp": 60000.0, "tint": 0.0}}), "out_of_range"),
        (json!({"wb": {"mode": "custom", "temp": 5000.0, "tint": 200.0}}), "out_of_range"),
        (json!({"profile": {"id": "lc.bogus", "amount": 100.0}}), "invalid_params"),
        (json!({"profile": {"id": "lc.vivid", "amount": 250.0}}), "out_of_range"),
        (json!({"profile": {"id": "lc.vivid", "amount": 100.0, "path": "C:/Users/Public/look.cube"}}), "invalid_params"),
        (json!({"treatment": "sepia"}), "invalid_params"),
        (json!({"treatment": 1}), "invalid_params"),
        (json!({"curve": {"master": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}]}}), "invalid_params"),
        (json!({"curve": steep}), "out_of_range"),
        (json!({"curve": loud}), "out_of_range"),
        (json!({"masks": [mask, mask]}), "invalid_params"),
        (json!({"masks": [odd_mask]}), "invalid_params"),
        (json!({"masks": {"id": 1}}), "invalid_params"),
        (json!({"spots": [{"mode": "smudge"}]}), "invalid_params"),
        (json!({"red_eye": wide_eye}), "out_of_range"),
        (json!({"red_eye": [eye_without_pet]}), "invalid_params"),
        (json!({"spots": [replaced(&spot, "/size", json!(40.0))]}), "out_of_range"),
        (json!({"spots": [replaced(&spot, "/feather", json!(150.0))]}), "out_of_range"),
        (json!({"spots": [replaced(&spot, "/opacity", json!(-1.0))]}), "out_of_range"),
        (json!({"spots": [replaced(&spot, "/points", json!([point(3.0, 0.5)]))]}), "out_of_range"),
        (json!({"spots": [replaced(&spot, "/source_offset", point(5.0, 0.0))]}), "out_of_range"),
        (json!({"spots": vec![spot.clone(); 1_001]}), "out_of_range"),
        (json!({"spots": vec![big_spot; 10]}), "budget_exceeded"),
        (json!({"red_eye": vec![eye.clone(); 65]}), "out_of_range"),
        (json!({"red_eye": [replaced(&eye, "/rx", json!(0.9))]}), "out_of_range"),
        (json!({"red_eye": [replaced(&eye, "/catchlight", point(2.0, 0.0))]}), "out_of_range"),
        (json!({"red_eye": [replaced(&eye, "/center", point(-5.0, 0.5))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/id", json!(65_536))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/id", json!(u32::MAX))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/adjust/exposure", json!(9.0))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/adjust/amount", json!(250.0))]}), "out_of_range"),
        (json!({"masks": [refined]}), "out_of_range"),
        (json!({"masks": many_masks}), "out_of_range"),
        (json!({"masks": [brush(vec![point(0.0, 0.0), point(1e9, 0.0)], 0.03)]}), "out_of_range"),
        (json!({"masks": [brush(vec![point(0.2, 0.2)], 0.6)]}), "out_of_range"),
        (json!({"masks": [brush(vec![point(0.2, 0.2)], 0.0001)]}), "out_of_range"),
        (json!({"masks": [brush(zigzag(10_001, 0.5), 0.03)]}), "out_of_range"),
        (json!({"masks": [brush(zigzag(100, 1.0), 0.5)]}), "budget_exceeded"),
        (json!({"masks": [brush(zigzag(4_000, 1.0), 0.002)]}), "budget_exceeded"),
        (json!({"masks": [shaped(json!({"kind": "linear", "start": point(0.5, 0.25), "end": point(5.0, 0.6)}))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/components/0/shape/rx", json!(10.0))]}), "out_of_range"),
        (json!({"masks": [shaped(json!({"kind": "colorRange", "samples": vec![[0.5, 0.0, 0.0]; 6], "refine": 50.0}))]}), "out_of_range"),
        (json!({"masks": [shaped(json!({"kind": "object", "hint": vec![point(0.5, 0.5); 65]}))]}), "out_of_range"),
        (json!({"curve": red_channel}), "out_of_range"),
        (json!({"spots": [crowded, crowded]}), "out_of_range"),
        (json!({"masks": [many_components]}), "out_of_range"),
        (json!({"masks": [many_strokes]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/adjust/contrast", json!(150.0))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/adjust/color_hue", json!(400.0))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/adjust/color_sat", json!(150.0))]}), "out_of_range"),
        (json!({"masks": [stroke_with("feather", 150.0)]}), "out_of_range"),
        (json!({"masks": [stroke_with("flow", -1.0)]}), "out_of_range"),
        (json!({"masks": [stroke_with("density", 150.0)]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/components/0/shape/center", point(5.0, 0.5))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/components/0/shape/ry", json!(10.0))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/components/0/shape/angle", json!(400.0))]}), "out_of_range"),
        (json!({"masks": [replaced(&mask, "/components/0/shape/feather", json!(150.0))]}), "out_of_range"),
        (range(json!({"kind": "colorRange", "samples": [[1.5, 0.0, 0.0]], "refine": 50.0})), "out_of_range"),
        (range(json!({"kind": "colorRange", "samples": [[0.5, 2.0, 0.0]], "refine": 50.0})), "out_of_range"),
        (range(json!({"kind": "colorRange", "samples": [[0.5, 0.0, 0.0]], "refine": 150.0})), "out_of_range"),
        (range(json!({"kind": "luminanceRange", "lo": 1.5, "hi": 0.9, "lo_feather": 0.1, "hi_feather": 0.1})), "out_of_range"),
        (range(json!({"kind": "depthRange", "lo": 0.2, "hi": 0.8, "feather": 2.0})), "out_of_range"),
        (range(json!({"kind": "object", "hint": [point(1.5, 0.5)]})), "out_of_range"),
        (range(json!({"kind": "object", "hint": [point(0.5, 0.5)], "edge": 150.0})), "out_of_range"),
        (range(json!({"kind": "prompt", "text": "the sky", "edge": -150.0})), "out_of_range"),
        (json!({"red_eye": [replaced(&eye, "/ry", json!(0.9))]}), "out_of_range"),
        (json!({"red_eye": [replaced(&eye, "/darken", json!(150.0))]}), "out_of_range"),
    ] {
        let what = format!("lts.develop with settings {settings}");
        let before = Witness::of(&lab.app);
        let reply = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "settings": settings}));
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
        assert_unchanged(&mut lab, &before, &what);
    }

    // the bounds themselves are the editor's own: a patch at them applies
    for settings in [
        json!({"masks": [replaced(&mask, "/id", json!(65_535))]}),
        json!({"spots": [replaced(&spot, "/size", json!(0.25))]}),
        json!({"red_eye": vec![eye; 64]}),
    ] {
        let reply = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "settings": settings}));
        assert_eq!(reply["outcome"], "applied", "{settings}: {reply}");
    }

    // a profile the editor has may be an imported LUT one, as `develop.profile` takes it
    let look = LutProfile { id: "lut:lts-look".into(), name: "LTS look".into(), group: "Imported".into(), file: String::new() };
    lab.app.session.lut_profiles.push(look);
    let reply = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "settings": {"profile": {"id": "lut:lts-look", "amount": 100.0}}}));
    assert_eq!(reply["outcome"], "applied", "a patch that names an imported LUT profile: {reply}");
    let reply = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "settings": {"profile": {"id": "lut:gone", "amount": 100.0}}}));
    assert_eq!(refusal(&reply), "invalid_params", "a LUT profile the editor does not have: {reply}");
}

/// T-LI-16 (R0, master plan v2 §3.1): presets and sync need no new `lts.*` id. A sync is
/// `lts.develop {settings}` with complete settings that an adapter builds from two certified reads
/// (`develop.get` of the source and of the target, the keys of every copied group taken from the
/// source): on identical libraries it leaves the target exactly as `develop.sync` does, with the
/// default groups and then with masks, spots, red eye and crop, one step each under the same label.
/// No query answers a preset's settings (`presets.list` gives ids and names), so a preset is
/// `lts.develop {preset: {id, amount?}}`, the keys of `preset.apply`: it leaves the photo exactly as
/// `preset.apply {id, amount, ids}` does, and `lts.preview` (variant `settings`) previews it as it
/// then commits. An unknown preset, an amount outside 0 to 200 and malformed presets are refused
/// with nothing changed. A preset the human saved or imported is held to the bounds a patch is
/// held to: one whose spot is of size 40 is `out_of_range`, and one with masks on a photo whose
/// mask id is past 65535 is refused before `Preset::apply` numbers its masks after that id (whose
/// sum would overflow); otherwise a user preset with masks applies exactly as `Preset::apply`
/// applies it, its masks numbered after the photo's.
#[test]
fn presets_and_sync_need_no_new_lts_id() {
    let mut hook = editing_lab(Presentation::LtsEmbedded);
    let mut oracle = editing_lab(Presentation::LtsEmbedded);
    let source = active_photo(&hook.app);
    let target = other_photo(&hook.app, source);
    // a source worth copying, edited alike in both libraries through the human's commands
    for lab in [&mut hook, &mut oracle] {
        for (id, params) in [
            ("develop.set", json!({"values": {"light.exposure": 0.8, "color.vibrance": 25, "effects.clarity": 15}})),
            ("develop.wb", json!({"mode": "daylight"})),
            ("develop.profile", json!({"id": "lc.vivid", "amount": 70})),
            ("develop.curve", json!({"channel": "master", "points": [[0.0, 0.0], [0.3, 0.25], [1.0, 1.0]]})),
            ("develop.treatment", json!({"bw": true})),
        ] {
            lab.app.run(id, params).unwrap_or_else(|e| panic!("setup {id}: {e}"));
        }
        settle(lab);
    }
    assert_same_photos(&all_photos(&hook.app), &all_photos(&oracle.app), "two libraries edited alike");

    // sync: complete settings from two reads, as develop.sync leaves the target
    oracle
        .app
        .run("library.select", json!({"ids": [source.0, target.0], "active": source.0}))
        .unwrap_or_else(|e| panic!("the oracle's selection: {e}"));
    for groups in [SettingsGroup::default_copy(), vec![SettingsGroup::Masks, SettingsGroup::Spots, SettingsGroup::RedEye, SettingsGroup::Crop]] {
        let what = format!("a sync of {groups:?}");
        let settings = synced(&settings_json(&hook.app, source), &settings_json(&hook.app, target), &groups);
        let params = json!({"id": target.0, "settings": settings, "expectHash": hash_of(&hook.app, target), "label": "Sync Settings"});
        let reply = lts(&mut hook.app, "lts.develop", params);
        assert_eq!(reply["outcome"], "applied", "{what}: {reply}");
        oracle.app.run("develop.sync", json!({"groups": groups})).unwrap_or_else(|e| panic!("{what}: develop.sync: {e}"));
        assert!(photo(&hook.app, target) == photo(&oracle.app, target), "{what}: the photo develop.sync leaves");
        assert_eq!(undo_label(&hook), undo_label(&oracle), "{what}: the undo label");
        settle(&mut hook);
        settle(&mut oracle);
    }

    // presets: their settings are read nowhere, so the hook applies them
    let listed = hook.app.run("presets.list", json!({})).unwrap_or_else(|e| panic!("presets.list: {e}"));
    let presets = listed.as_array().cloned().unwrap_or_default();
    assert!(presets.iter().any(|preset| preset["id"] == "lc.warm-glow"), "a built-in preset: {listed}");
    assert!(presets.iter().all(|preset| preset.get("settings").is_none()), "presets.list answers no settings: {listed}");
    settle(&mut hook);
    let preset = json!({"id": "lc.warm-glow", "amount": 60});
    let what_if = preview(&mut hook.app, target, json!({"variant": "settings", "preset": preset}));
    assert_eq!(what_if["outcome"], "ok", "a preset's what-if preview: {what_if}");
    let params = json!({"id": target.0, "preset": preset, "expectHash": hash_of(&hook.app, target)});
    let reply = lts(&mut hook.app, "lts.develop", params);
    assert_eq!((reply["outcome"].as_str(), reply["label"].as_str()), (Some("applied"), Some("Preset: Warm Glow")), "{reply}");
    oracle.app.run("preset.apply", json!({"id": "lc.warm-glow", "amount": 60, "ids": [target.0]})).unwrap_or_else(|e| panic!("preset.apply: {e}"));
    assert!(photo(&hook.app, target) == photo(&oracle.app, target), "the photo preset.apply leaves");
    assert_eq!(undo_label(&hook), undo_label(&oracle), "the undo label");
    assert_eq!(reply["hashAfter"], what_if["renderedHash"], "the previewed settings are the ones committed");
    assert_eq!(reply["intentSatisfied"], true, "{reply}");
    settle(&mut hook);
    for (params, code) in [
        (json!({"id": target.0, "preset": {"id": "lc.bogus"}}), "invalid_params"),
        (json!({"id": target.0, "preset": {"id": "lc.warm-glow", "amount": 250}}), "out_of_range"),
        (json!({"id": target.0, "preset": {"id": "lc.warm-glow", "amount": -1}}), "out_of_range"),
        (json!({"id": target.0, "preset": {"id": "lc.warm-glow", "amount": "60"}}), "invalid_params"),
        (json!({"id": target.0, "preset": {"id": 7}}), "invalid_params"),
        (json!({"id": target.0, "preset": {"id": ""}}), "invalid_params"),
        (json!({"id": target.0, "preset": {"amount": 60}}), "invalid_params"),
        (json!({"id": target.0, "preset": {}}), "invalid_params"),
        (json!({"id": UNKNOWN, "preset": {"id": "lc.warm-glow"}}), "target_not_found"),
    ] {
        let what = format!("lts.develop {params}");
        let before = Witness::of(&hook.app);
        let reply = lts(&mut hook.app, "lts.develop", params);
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
        assert_unchanged(&mut hook, &before, &what);
    }

    // presets the human saved or imported: held to the bounds before anything changes
    let user = |id: &str, name: &str, settings: Value| Preset {
        id: id.to_string(),
        name: name.to_string(),
        group: "User".to_string(),
        settings,
        favorite: false,
        builtin: false,
    };
    let huge_spot = canonical::<Spot>(json!({"mode": "heal", "points": [point(0.5, 0.5)], "size": 40.0}));
    let preset_mask = replaced(&settings_json(&hook.app, source)["masks"][0], "/name", json!("Preset mask"));
    hook.app.session.presets.push(user("lts.test.huge-spot", "Huge spot", json!({"spots": [huge_spot]})));
    hook.app.session.presets.push(user("lts.test.mask", "Preset mask", json!({"masks": [preset_mask]})));
    settle(&mut hook);
    let planned = hook.app.session.develop_of(target).expect("the target");
    let reply = lts(&mut hook.app, "lts.develop", json!({"id": target.0, "preset": {"id": "lts.test.mask"}}));
    assert_eq!((reply["outcome"].as_str(), reply["label"].as_str()), (Some("applied"), Some("Preset: Preset mask")), "{reply}");
    let expected = hook.app.session.presets.iter().find(|preset| preset.id == "lts.test.mask").map(|preset| preset.apply(&planned, 1.0));
    let masks = expected.as_ref().map(|settings| settings.masks.iter().map(|mask| mask.id).collect::<Vec<_>>());
    assert_eq!(masks, Some(vec![1, 2]), "the preset's mask is numbered after the photo's");
    assert_eq!(hook.app.session.develop_of(target).map(|settings| (*settings).clone()), expected, "the photo Preset::apply leaves");
    settle(&mut hook);
    let mut held = Arc::unwrap_or_clone(hook.app.session.develop_of(target).expect("the target"));
    held.masks.push(Mask { id: u32::MAX, ..Mask::default() });
    let op = hook.app.session.develop_op(target, held, "Mask").expect("an op for the target");
    hook.app.session.commit("Mask", op).unwrap_or_else(|e| panic!("the fixture: {e}"));
    settle(&mut hook);
    for (command, params) in [
        ("lts.develop", json!({"id": target.0, "preset": {"id": "lts.test.mask"}})),
        ("lts.preview", json!({"id": target.0, "longEdge": 64, "variant": "settings", "preset": {"id": "lts.test.mask"}})),
        ("lts.develop", json!({"id": target.0, "preset": {"id": "lts.test.huge-spot"}})),
        ("lts.preview", json!({"id": target.0, "longEdge": 64, "variant": "settings", "preset": {"id": "lts.test.huge-spot"}})),
    ] {
        let what = format!("{command} {params}: past the bounds");
        let before = Witness::of(&hook.app);
        let reply = lts(&mut hook.app, command, params);
        assert_eq!(refusal(&reply), "out_of_range", "{what}: {reply}");
        assert_unchanged(&mut hook, &before, &what);
    }
}

/// T-LI-17 (R0, master plan v2 §3.1): a natively hosted editor (a worker, plan NW-5) keeps the
/// hook's edits in the library its host opened in the workspace, and the hook writes nothing else.
/// The host builds its session as every native host of the engine does, reading originals from the
/// file system (`Session::with_fs`, as the desktop app, the CLI and the MCP server do), and opens
/// the session's library with `Session::open_library_in` over `FsStore`s on a folder of the
/// workspace, `on_disk: false` and without the demo photos; here it imports an original that lives
/// outside that folder (a worst case: a host stages its inputs in the workspace), and the library
/// asks for XMP sidecars (`xmp.autoWrite` on). The hook's develop, crop, undo, redo and restore are
/// written to the folder's catalog journal and write no sidecar beside the original, where the
/// engine's own `develop.set` then writes one (the preference was on); reads (a preview and an
/// export render the original) write nothing; closing the library writes a snapshot that holds the
/// edits and empties the journal; the workspace holds the library's own files and nothing else, the
/// original's folder the original and the engine's sidecar only; a new session that opens the
/// folder finds the photo with its settings, the library not on disk and no smart-preview folder;
/// and the hook runs on no library opened on disk (a desktop library), in either runtime
/// (`command_not_allowed`).
#[test]
fn workspace_library_keeps_lts_edits_in_the_workspace() {
    lightcraft_engine::gpu::set_enabled(false);
    let root = std::env::temp_dir().join(format!("lc-lts-view-workspace-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (workspace, elsewhere) = (root.join("workspace"), root.join("elsewhere"));
    let project = workspace.join("project");
    std::fs::create_dir_all(&elsewhere).unwrap_or_else(|e| panic!("a folder outside the workspace: {e}"));
    let original = elsewhere.join("outside.png");
    std::fs::write(&original, picked_png()).unwrap_or_else(|e| panic!("the original: {e}"));
    let open = |session: &mut Session| {
        let store = || FsStore::open(&project).unwrap_or_else(|e| panic!("the workspace folder {}: {e}", project.display()));
        let stores = LibraryStores { dir: project.clone(), catalog: Box::new(store()), files: Box::new(store()), on_disk: false };
        if let Err(e) = session.open_library_in(stores, false) {
            panic!("opening the workspace library: {e}");
        }
    };
    let size = |path: &Path| std::fs::metadata(path).map(|meta| meta.len()).ok();
    let (journal, snapshot) = (project.join("catalog.log"), project.join("catalog.snap"));
    let mut session = Session::new().with_fs();
    open(&mut session);
    assert_eq!(session.catalog.len(), 0, "an empty library: no demo photos");
    let created = size(&snapshot).expect("a new library writes its snapshot when it opens");
    let mut app = LightcraftApp::new(session, Services::default()).with_presentation(Presentation::LtsEmbedded);
    let report = app.run("library.import", json!({"paths": [original.to_string_lossy()]})).unwrap_or_else(|e| panic!("importing the original: {e}"));
    let id = PhotoId(report["imported"][0].as_u64().unwrap_or_else(|| panic!("the original was imported: {report}")));
    app.session.xmp.auto_write = true;
    let logged = || size(&journal).unwrap_or(0);
    let imported = logged();
    assert!(imported > 0, "the import is in the workspace's catalog journal");

    let h0 = hash_of(&app, id);
    let develop = lts(&mut app, "lts.develop", json!({"id": id.0, "values": {"light.exposure": 0.6}, "expectHash": h0}));
    let crop = lts(&mut app, "lts.crop", json!({"id": id.0, "rect": [0.1, 0.1, 0.9, 0.8]}));
    let undo = lts(&mut app, "lts.undo", json!({"id": id.0, "expectHash": crop["hashAfter"], "expectHistoryHead": crop["historyHead"]}));
    let redo = lts(&mut app, "lts.redo", json!({"id": id.0, "expectHash": undo["hashAfter"], "expectRedoHead": undo["redoHead"]}));
    let restore = lts(&mut app, "lts.restoreSettings", json!({"id": id.0, "revision": develop["hashAfter"], "expectHash": redo["hashAfter"]}));
    for reply in [&develop, &crop, &undo, &redo, &restore] {
        assert_eq!(reply["outcome"], "applied", "{reply}");
    }
    assert!(app.session.xmp.auto_write, "the library's preference is as it was");
    let edited = logged();
    assert!(edited > imported, "the hook's edits are in the workspace's catalog journal");
    assert_eq!(listing(&elsewhere), [("outside.png".to_string(), false)], "the hook writes no sidecar beside the original");
    for (command, params) in [
        ("lts.state", json!({"id": id.0})),
        ("lts.preview", json!({"id": id.0, "longEdge": 64})),
        ("lts.export", json!({"id": id.0, "format": "png", "longEdge": 64, "metadata": "none"})),
    ] {
        let reply = lts(&mut app, command, params);
        assert_eq!(reply["outcome"], "ok", "{command}: {reply}");
    }
    assert_eq!(logged(), edited, "reads write nothing");

    // the control: with the preference on, the engine's own command writes a sidecar there
    app.run("library.select", json!({"ids": [id.0]})).unwrap_or_else(|e| panic!("selecting the photo: {e}"));
    app.run("develop.set", json!({"values": {"light.contrast": 10}})).unwrap_or_else(|e| panic!("develop.set: {e}"));
    let beside = listing(&elsewhere);
    assert!(beside.len() == 2 && beside.iter().any(|(name, _)| name.ends_with(".xmp")), "develop.set writes the sidecar: {beside:?}");
    let settings = settings_json(&app, id);
    app.session.close_library().unwrap_or_else(|e| panic!("closing the workspace library: {e}"));
    drop(app);
    assert_eq!(size(&journal), Some(0), "closing empties the journal into the snapshot");
    assert!(size(&snapshot).is_some_and(|len| len > created), "the snapshot now holds the photo and its edits");
    assert_eq!(listing(&workspace), [("project".to_string(), true)], "the library stays in its folder");
    let files = listing(&project);
    assert!(files.iter().all(|(name, folder)| !folder && LIBRARY_FILES.contains(&name.as_str())), "only the library's own files: {files:?}");
    let mut again = Session::new().with_fs();
    open(&mut again);
    assert_eq!(again.catalog.len(), 1, "the workspace library has the imported photo and nothing else");
    assert_eq!(again.develop_of(id).map(|d| d.to_json()), Some(settings), "the edits are in the workspace library");
    assert_eq!(again.library.as_ref().map(|lib| (lib.dir.clone(), lib.on_disk)), Some((project.clone(), false)), "not a library on disk");
    assert!(again.media.smart_dir.is_none(), "no smart-preview folder");
    drop(again);

    // the hook runs on no library opened on disk (a desktop library), in either runtime
    let disk = root.join("on-disk");
    let store = || FsStore::open(&disk).unwrap_or_else(|e| panic!("a library on disk {}: {e}", disk.display()));
    let stores = LibraryStores { dir: disk.clone(), catalog: Box::new(store()), files: Box::new(store()), on_disk: true };
    let mut desktop = Session::new().with_fs();
    if let Err(e) = desktop.open_library_in(stores, false) {
        panic!("opening a library on disk: {e}");
    }
    let mut app = LightcraftApp::new(desktop, Services::default());
    for runtime in [Runtime::Browser, Runtime::Native] {
        let reply = lts_in(&mut app, "lts.state", json!({}), runtime);
        assert_eq!(refusal(&reply), "command_not_allowed", "{runtime:?}: never on a library on disk: {reply}");
    }
    drop(app);
    let _ = std::fs::remove_dir_all(&root);
}

/// T-LI-18 (R0, critic CC-22): `lts.restoreSettings` puts back an earlier state that a photo's
/// history recorded, named by that state's settings token (`revision`: a reply's `hashAfter`),
/// while the photo's settings are still the ones `expectHash` names. After the agent's edit and a
/// human's edit of sections no patch reaches (detail and vignette), it leaves the photo, which is not
/// active, exactly as `history.restore {index}` leaves it in an identical library (the settings, the
/// history and the undo label `History: <step label>`), selecting nothing, as one undo step that
/// `lts.undo` takes back. A stale `expectHash` (answering the current token), a token the history
/// does not hold, an unknown photo, malformed or missing tokens, a stale head, unknown keys and a
/// human interaction are refused with nothing changed.
#[test]
fn restore_settings_puts_back_a_recorded_revision() {
    let mut hook = editing_lab(Presentation::LtsEmbedded);
    let mut oracle = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&hook.app);
    let target = other_photo(&hook.app, active);
    let agent = agent_then_human(&mut hook, target, active);
    agent_then_human(&mut oracle, target, active);
    assert_same_photos(&all_photos(&hook.app), &all_photos(&oracle.app), "two libraries edited alike");
    let revision = agent["hashAfter"].clone();
    let current = hash_of(&hook.app, target);
    assert_ne!(json!(current), revision, "the human's edit changed the photo");
    let history = photo(&hook.app, target).history;
    let step = history.iter().rposition(|step| json!(hex(step.settings.hash64())) == revision).expect("the history holds the agent's state");

    let what = "lts.restoreSettings of the agent's revision";
    let (undo, catalog) = (hook.app.session.undo.len(), hook.app.session.catalog.revision);
    let before = Witness::of(&hook.app);
    let reply = lts(&mut hook.app, "lts.restoreSettings", json!({"id": target.0, "revision": revision, "expectHash": current}));
    assert_eq!(reply["outcome"], "applied", "{what}: {reply}");
    for (command, params) in [("library.select", json!({"ids": [target.0]})), ("history.restore", json!({"index": step}))] {
        oracle.app.run(command, params).unwrap_or_else(|e| panic!("the oracle's {command}: {e}"));
    }
    assert!(photo(&hook.app, target) == photo(&oracle.app, target), "{what}: the photo history.restore leaves");
    assert_eq!(undo_label(&hook), undo_label(&oracle), "{what}: the undo label");
    assert_eq!(undo_label(&hook).as_deref(), Some("History: LTS develop"), "{what}");
    assert_eq!((&reply["hashBefore"], &reply["hashAfter"]), (&json!(current), &revision), "{what}: the tokens");
    assert_eq!(reply["restored"], json!({"revision": revision, "step": step, "stepLabel": "LTS develop"}), "{what}: {reply}");
    assert_eq!((hook.app.session.undo.len(), hook.app.session.catalog.revision), (undo + 1, catalog + 1), "{what}: one step");
    assert_kept(&hook.app, &before, &["catalog revision", "catalog", "undo", "redo"], what);
    assert!(written(&hook).iter().any(|name| name == "catalog.log"), "{what}: written to the library");

    let undone = lts(&mut hook.app, "lts.undo", json!({"id": target.0, "expectHash": revision, "expectHistoryHead": reply["historyHead"]}));
    assert_eq!((undone["outcome"].as_str(), &undone["hashAfter"]), (Some("applied"), &json!(current)), "an undo takes it back: {undone}");
    settle(&mut hook);
    let settings = settings_json(&hook.app, target);
    for (params, code) in [
        (json!({"id": target.0, "revision": revision, "expectHash": revision}), "settings_hash_conflict"),
        (json!({"id": target.0, "revision": "0123456789abcdef", "expectHash": current}), "target_not_found"),
        (json!({"id": UNKNOWN, "revision": revision, "expectHash": current}), "target_not_found"),
        (json!({"id": target.0, "revision": "not-a-token", "expectHash": current}), "invalid_params"),
        (json!({"id": target.0, "revision": revision}), "invalid_params"),
        (json!({"id": target.0, "expectHash": current}), "invalid_params"),
        (json!({"id": target.0, "revision": revision, "expectHash": current, "expectHistoryHead": {"length": 1}}), "invalid_params"),
        (json!({"id": target.0, "revision": revision, "expectHash": current, "expectHistoryHead": {"length": 0, "label": 5}}), "invalid_params"),
        (json!({"id": target.0, "revision": revision, "expectHash": current, "label": ""}), "invalid_params"),
        (json!({"id": target.0, "revision": revision, "expectHash": current, "settings": settings}), "invalid_params"),
        (
            json!({"id": target.0, "revision": revision, "expectHash": current, "expectHistoryHead": {"length": 0, "label": null}}),
            "history_head_conflict",
        ),
    ] {
        let what = format!("lts.restoreSettings {params}");
        let before = Witness::of(&hook.app);
        let reply = lts(&mut hook.app, "lts.restoreSettings", params);
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
        if code == "settings_hash_conflict" {
            assert_eq!(reply["actual"], json!(current), "{what}: names the current token to refresh from");
        }
        assert_unchanged(&mut hook, &before, &what);
    }
    hook.app.run("develop.beginInteraction", json!({"label": "Exposure"})).unwrap_or_else(|e| panic!("an interaction: {e}"));
    settle(&mut hook);
    let what = "lts.restoreSettings during a human interaction";
    let before = Witness::of(&hook.app);
    let reply = lts(&mut hook.app, "lts.restoreSettings", json!({"id": target.0, "revision": revision, "expectHash": current}));
    assert_eq!(refusal(&reply), "editor_busy", "{what}: {reply}");
    assert_unchanged(&mut hook, &before, what);
    hook.app.run("develop.cancelInteraction", json!({})).unwrap_or_else(|e| panic!("cancelling the interaction: {e}"));
}

/// T-LI-19 (R0, verifier finding 2): complete settings are held to the bounds a patch is held to,
/// on every part they change. A slider outside its range, a value the editor's commands would clamp
/// (a spot of size 40, a red-eye pupil of 150, a point colour's hue shift of 150), an unknown
/// profile, repeated or too large mask ids, a point-curve point outside 0 to 1, a crop smaller than
/// the crop tool keeps or with a corner outside the rectangle's range, an Upright guide's point out
/// of range, more red-eye corrections, point colours or Upright guides than the caps and a changed
/// schema version are refused with nothing changed, in `lts.develop` and in a `lts.preview` what-if
/// alike. A part equal to the photo's own is not judged again: a photo whose spot holds an opacity
/// the editor would clamp (written by another path) still takes complete settings that keep that
/// spot, while a patch that changes its spots is held to the bounds whole.
#[test]
fn complete_settings_are_held_to_the_patch_bounds() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    let current = settings_json(&lab.app, target);
    let mask = settings_json(&lab.app, active)["masks"][0].clone();
    let spot = |size: f64| canonical::<Spot>(json!({"mode": "heal", "points": [point(0.5, 0.5)], "size": size}));
    let eye = |pupil: f64| {
        canonical::<RedEye>(json!({"center": point(0.4, 0.4), "rx": 0.03, "ry": 0.02, "pupil_size": pupil, "darken": 50.0, "pet": false}))
    };
    let guide = json!([point(0.1, 0.1), point(0.9, 0.9)]);
    let shifted = replaced(&canonical::<PointColor>(json!({})), "/hue_shift", json!(150.0));
    let token = hash_of(&lab.app, target);
    for (pointer, value, code) in [
        ("/light/exposure", json!(9.0), "out_of_range"),
        ("/wb/temp", json!(60_000.0), "out_of_range"),
        ("/detail/sharpen_amount", json!(400.0), "out_of_range"),
        ("/crop/geometry/angle", json!(60.0), "out_of_range"),
        ("/profile/id", json!("lc.bogus"), "invalid_params"),
        ("/red_eye", json!([eye(150.0)]), "out_of_range"),
        ("/red_eye", json!(vec![eye(50.0); 65]), "out_of_range"),
        ("/spots", json!([spot(40.0)]), "out_of_range"),
        ("/masks", json!([mask, mask]), "invalid_params"),
        ("/masks", json!([replaced(&mask, "/id", json!(u32::MAX))]), "out_of_range"),
        ("/curve/master", json!([point(0.0, 0.0), point(1.5, 1.0)]), "out_of_range"),
        ("/crop/geometry/rect", json!({"x0": 0.5, "y0": 0.1, "x1": 0.51, "y1": 0.9}), "out_of_range"),
        ("/point_colors", json!(vec![canonical::<PointColor>(json!({})); 9]), "out_of_range"),
        ("/geometry/guides", json!(vec![guide; 5]), "out_of_range"),
        ("/geometry/guides", json!([[point(0.1, 0.1), point(5.0, 0.9)]]), "out_of_range"),
        ("/point_colors", json!([shifted]), "out_of_range"),
        ("/crop/geometry/rect", json!({"x0": -2.0, "y0": 0.1, "x1": 0.9, "y1": 0.9}), "out_of_range"),
        ("/version", json!(2), "invalid_params"),
    ] {
        let settings = replaced(&current, pointer, value);
        for (command, params) in [
            ("lts.develop", json!({"id": target.0, "settings": settings, "expectHash": token})),
            ("lts.preview", json!({"id": target.0, "longEdge": 64, "variant": "settings", "settings": settings})),
        ] {
            let what = format!("{command} with complete settings, {pointer} changed");
            let before = Witness::of(&lab.app);
            let reply = lts(&mut lab.app, command, params);
            assert_eq!(refusal(&reply), code, "{what}: {reply}");
            assert_unchanged(&mut lab, &before, &what);
        }
    }

    // a part the photo already holds is its own: here a spot with an opacity the editor would clamp
    let mut held = Arc::unwrap_or_clone(lab.app.session.develop_of(target).expect("the target"));
    held.spots.push(serde_json::from_value(replaced(&spot(0.02), "/opacity", json!(150.0))).expect("a spot"));
    let op = lab.app.session.develop_op(target, held, "Remove").expect("an op for the target");
    lab.app.session.commit("Remove", op).unwrap_or_else(|e| panic!("the fixture: {e}"));
    settle(&mut lab);
    let own = settings_json(&lab.app, target);
    let brighter = replaced(&own, "/light/exposure", json!(0.4));
    let params = json!({"id": target.0, "settings": brighter, "expectHash": hash_of(&lab.app, target)});
    let reply = lts(&mut lab.app, "lts.develop", params);
    assert_eq!(reply["outcome"], "applied", "complete settings that keep the photo's own spot: {reply}");
    assert_eq!(settings_json(&lab.app, target), brighter, "the settings given, the photo's own spot with them");
    settle(&mut lab);
    let mut more = own["spots"].clone();
    if let Some(spots) = more.as_array_mut() {
        spots.push(spot(0.02));
    }
    let what = "a patch that changes the photo's spots";
    let before = Witness::of(&lab.app);
    let reply = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "settings": {"spots": more}}));
    assert_eq!(refusal(&reply), "out_of_range", "{what} is held to the bounds whole: {reply}");
    assert_unchanged(&mut lab, &before, what);
}

/// T-LI-20 (R0, the re-verifiers' crop findings; root decision LB1): every change is held at the
/// crop's magnification, one over the smaller side of the crop the renderer shows. In a 2.05 % crop
/// (magnified about 49 times) a heal or remove spot of size 0.25, which `spot.add` allows in the
/// whole photo, is refused (`out_of_range`) in `lts.develop` and in a `lts.preview` what-if alike,
/// and so are an Auto Mask stroke wider there than the editor's brush may be in the whole photo and
/// the editor's largest defringe, whose filter would reach some 350 pixels (`out_of_range`), and a
/// dense brush stroke and eight clone spots, whose work grows with the square of the magnification
/// (`budget_exceeded`; the same spots apply in the whole photo). A slider, a small heal spot, a
/// clone spot of any size, a sparse brush stroke, a narrow Auto Mask stroke and a light defringe
/// still apply there. `lts.crop` judges the photo's own spots and brushes again: a crop that would
/// magnify a heal spot of size 0.25 or a dense brush stroke past these bounds is refused, although
/// the same photos took the spot and the stroke uncropped, and a looser crop the stroke fits
/// applies; a long thin crop that `crop.set` would shrink to 0.68 % tall is refused too. Each Auto
/// Mask stroke counts eight output planes of work: seven apply, nine are `budget_exceeded`. The
/// crop judged is the one the renderer shows: where Constrain Crop would shrink a crop to 0.2 % of
/// the photo (its centre left blank by a transform), `lts.crop` and the transform slider that would
/// do it are refused, while a crop it keeps applies. Where the renderer solves an Upright transform
/// from the pixels under Constrain Crop (an automatic Upright stored without its transform), the
/// crop shown counts as anything from 0.02 of the crop to all of it: a crop (a corner one, and even
/// 40 % of the photo) and a transform slider are refused there, and so are complete settings that
/// would leave a plain photo so, while a slider and a heal spot small enough at 50 times apply. On
/// a photo whose own crop is 1 % (made by another path), a change that would add a blur past the
/// hook's bound (a dehaze; a Subject, People, Object or Prompt mask) is refused, while a Sky mask
/// (whose blur is a third as wide), a slider, a small heal spot and a wider crop apply, after which
/// the dehaze applies too. On one whose own crop is 0.4 %, a Sky mask and a profile whose look needs
/// the highlights and shadows base are refused, whether a patch names the profile or `values` raise
/// its amount, while a profile whose look needs no spatial plane applies; blur by blur, a
/// highlights slider is refused although the photo's own dehaze plane is wider than the base it
/// would add, while a dehaze that keeps that plane applies. A crop is tighter when one over its
/// smaller side grows or the renderer's own magnification does, which follows the crop's longer
/// side: a strip 2.05 % wide and the photo's height, squared up to 2.05 % × 2.05 %, is tighter
/// though its smaller side stays, so the heal spot of size 0.25, the defringe of 20 and the dense
/// brush stroke that the strip held (made by another path) are refused there, by `lts.crop` and by
/// complete settings alike, while the strip still takes a slider and a reset. Blur by blur, the
/// widest answers for the widest: on a photo whose own crop is 1 % with a Subject mask, a second
/// one is refused, the refusal naming the blur, while a People mask in its place applies; a
/// photo's own defringe filter past the bound is its own (a slider and a lighter defringe apply, a
/// stronger one is refused). The section toggles count: with the geometry section off an automatic
/// Upright is not solved (a 40 % crop and a heal spot apply), and complete settings that turn
/// geometry or optics on change the frame and are refused where Constrain Crop would then shrink
/// the crop (the solved Upright's refusal says that the crop counts as 0.02 of itself). Visualize
/// Spots, whose blur grows with the crop's magnification, is refused in a preview 2048 long of a
/// 0.1 % crop and shown in one 256 long or uncropped. Nothing changes on a refusal.
#[test]
fn crop_aware_bounds_hold_spots_brushes_and_blurs_at_the_crops_magnification() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    let plain = plain_photos(&lab.app, &[active, target]);
    assert!(plain.len() >= 17, "the demo library has unedited photos: {plain:?}");
    let (spotted, brushed, warped, cornered, tiny) = (plain[0], plain[1], plain[2], plain[3], plain[4]);
    let (solved, sliver, thin) = (plain[5], plain[6], plain[7]);
    let develop = |id: PhotoId, settings: Value| json!({"id": id.0, "settings": settings});
    let checkout = |id: PhotoId, settings: Value, token: String| json!({"id": id.0, "settings": settings, "expectHash": token});
    let values = |id: PhotoId, controls: Value| json!({"id": id.0, "values": controls});
    let crop = |id: PhotoId, rect: [f64; 4]| json!({"id": id.0, "rect": rect});
    let spot = |mode: &str, at: f64, size: f64| canonical::<Spot>(json!({"mode": mode, "points": [point(at, at)], "size": size}));
    let stroke = |points: Vec<Value>, size: f64, auto: bool| json!({"points": points, "size": size, "auto_mask": auto});
    let brush = |strokes: Vec<Value>| canonical::<Mask>(json!({"id": 1, "components": [{"shape": {"kind": "brush", "strokes": strokes}}]}));
    let masked = |shape: Value| canonical::<Mask>(json!({"id": 1, "components": [{"shape": shape}]}));
    let dense = |at: f64| (0..200_u32).map(|i| point(at + 0.0001 * f64::from(i), at + 0.0001 * f64::from(i))).collect::<Vec<_>>();
    let clones: Vec<Value> = (0..8_u32).map(|i| spot("clone", 0.3 + 0.05 * f64::from(i), 0.02)).collect();

    // a 2.05 % crop and what it still takes
    applies(&mut lab, "lts.crop", crop(target, [0.4, 0.4, 0.4205, 0.4205]));
    applies(&mut lab, "lts.develop", values(target, json!({"light.exposure": 0.5})));
    applies(&mut lab, "lts.develop", develop(target, json!({"spots": [spot("heal", 0.41, 0.004)]})));
    applies(&mut lab, "lts.develop", develop(target, json!({"spots": [spot("heal", 0.41, 0.004), spot("clone", 0.41, 0.25)]})));
    let sparse = brush(vec![stroke(vec![point(0.405, 0.405), point(0.415, 0.415)], 0.01, false)]);
    applies(&mut lab, "lts.develop", develop(target, json!({"masks": [sparse]})));
    let narrow = brush(vec![stroke(vec![point(0.41, 0.41)], 0.01, true)]);
    applies(&mut lab, "lts.develop", develop(target, json!({"masks": [narrow]})));
    applies(&mut lab, "lts.develop", values(target, json!({"optics.defringePurple": 0.5})));

    // and what it refuses
    for mode in ["heal", "remove"] {
        refuses(&mut lab, "lts.develop", develop(target, json!({"spots": [spot(mode, 0.41, 0.25)]})), "out_of_range");
    }
    refuses(&mut lab, "lts.develop", develop(target, json!({"spots": [spot("heal", 0.41, 0.006)]})), "out_of_range");
    let what_if = json!({"id": target.0, "longEdge": 64, "variant": "settings", "settings": {"spots": [spot("heal", 0.41, 0.25)]}});
    refuses(&mut lab, "lts.preview", what_if, "out_of_range");
    let wide = brush(vec![stroke(vec![point(0.41, 0.41)], 0.011, true)]);
    refuses(&mut lab, "lts.develop", develop(target, json!({"masks": [wide]})), "out_of_range");
    refuses(&mut lab, "lts.develop", values(target, json!({"optics.defringePurple": 20})), "out_of_range");
    let thick = brush(vec![stroke(dense(0.401), 0.01, false)]);
    refuses(&mut lab, "lts.develop", develop(target, json!({"masks": [thick]})), "budget_exceeded");
    refuses(&mut lab, "lts.develop", develop(target, json!({"spots": clones})), "budget_exceeded");

    // lts.crop judges the photo's own spots and brushes again
    applies(&mut lab, "lts.develop", develop(spotted, json!({"spots": [spot("heal", 0.5, 0.25)]})));
    refuses(&mut lab, "lts.crop", crop(spotted, [0.49, 0.49, 0.5105, 0.5105]), "out_of_range");
    applies(&mut lab, "lts.develop", develop(spotted, json!({"spots": clones})));
    let thick = brush(vec![stroke(dense(0.45), 0.01, false)]);
    applies(&mut lab, "lts.develop", develop(brushed, json!({"masks": [thick]})));
    refuses(&mut lab, "lts.crop", crop(brushed, [0.45, 0.45, 0.4705, 0.4705]), "budget_exceeded");
    applies(&mut lab, "lts.crop", crop(brushed, [0.3, 0.3, 0.7, 0.7]));
    // crop.set keeps a crop inside the photo: a long thin one shrinks about the centre, here to
    // 0.68 % tall
    refuses(&mut lab, "lts.crop", crop(thin, [-1.0, 0.5, 2.0, 0.5205]), "out_of_range");

    // each Auto Mask stroke counts eight output planes, however few dabs it lays
    let auto = |count: usize| brush(vec![stroke(vec![point(0.2, 0.2)], 0.01, true); count]);
    applies(&mut lab, "lts.develop", develop(spotted, json!({"masks": [auto(7)]})));
    refuses(&mut lab, "lts.develop", develop(spotted, json!({"masks": [auto(9)]})), "budget_exceeded");

    // the crop judged is the one the renderer shows: Constrain Crop shrinks a crop that a transform
    // leaves blank about its centre, here to 0.02 of itself
    committed(&mut lab, warped, "Constrain Crop", |d| {
        d.geometry.scale = 50.0;
        d.geometry.constrain_crop = true;
    });
    refuses(&mut lab, "lts.crop", crop(warped, [0.0, 0.0, 0.1, 0.1]), "out_of_range");
    applies(&mut lab, "lts.crop", crop(warped, [0.3, 0.3, 0.7, 0.7]));
    committed(&mut lab, cornered, "Constrain Crop", |d| d.geometry.constrain_crop = true);
    applies(&mut lab, "lts.crop", crop(cornered, [0.0, 0.0, 0.1, 0.1]));
    refuses(&mut lab, "lts.develop", values(cornered, json!({"geometry.scale": 50})), "out_of_range");
    applies(&mut lab, "lts.develop", values(cornered, json!({"light.exposure": 0.3})));

    // an automatic Upright stored without its transform (as an XMP sidecar or a preset leaves it)
    // is solved from the pixels when the photo renders: under Constrain Crop the crop shown counts
    // as anything from 0.02 of the crop to all of it
    committed(&mut lab, solved, "Upright", |d| {
        d.geometry.upright = Upright::Vertical;
        d.geometry.constrain_crop = true;
    });
    refuses(&mut lab, "lts.crop", crop(solved, [0.0, 0.0, 0.0205, 0.0205]), "out_of_range");
    refuses(&mut lab, "lts.crop", crop(solved, [0.3, 0.3, 0.7, 0.7]), "out_of_range");
    refuses(&mut lab, "lts.develop", values(solved, json!({"geometry.vertical": 10})), "out_of_range");
    applies(&mut lab, "lts.develop", values(solved, json!({"light.exposure": 0.3})));
    applies(&mut lab, "lts.develop", develop(solved, json!({"spots": [spot("heal", 0.5, 0.004)]})));
    refuses(&mut lab, "lts.develop", develop(solved, json!({"spots": [spot("heal", 0.5, 0.006)]})), "out_of_range");
    let mut upright = settings_json(&lab.app, thin);
    upright["geometry"]["upright"] = json!("vertical");
    upright["geometry"]["constrain_crop"] = json!(true);
    let token = hash_of(&lab.app, thin);
    refuses(&mut lab, "lts.develop", checkout(thin, upright, token), "out_of_range");

    // a photo whose own crop is 1 % (made by another path): no change may add a blur past the bound
    committed(&mut lab, tiny, "Crop", |d| {
        let rect = &mut d.crop.geometry.rect;
        (rect.x0, rect.y0, rect.x1, rect.y1) = (0.5, 0.5, 0.51, 0.51);
    });
    applies(&mut lab, "lts.develop", values(tiny, json!({"light.exposure": 0.4})));
    refuses(&mut lab, "lts.develop", values(tiny, json!({"effects.dehaze": 10})), "out_of_range");
    for shape in [
        json!({"kind": "subject"}),
        json!({"kind": "people", "person": 0, "parts": []}),
        json!({"kind": "object", "hint": [point(0.505, 0.505)]}),
        json!({"kind": "prompt", "text": "the sky"}),
    ] {
        refuses(&mut lab, "lts.develop", develop(tiny, json!({"masks": [masked(shape)]})), "out_of_range");
    }
    applies(&mut lab, "lts.develop", develop(tiny, json!({"masks": [masked(json!({"kind": "sky"}))]})));
    applies(&mut lab, "lts.develop", develop(tiny, json!({"spots": [spot("heal", 0.505, 0.002)]})));
    applies(&mut lab, "lts.crop", crop(tiny, [0.4, 0.4, 0.6, 0.6]));
    applies(&mut lab, "lts.develop", values(tiny, json!({"effects.dehaze": 10})));

    // blur by blur, the profile's look included, on a photo whose own crop is 0.4 %
    committed(&mut lab, sliver, "Crop", |d| {
        let rect = &mut d.crop.geometry.rect;
        (rect.x0, rect.y0, rect.x1, rect.y1) = (0.5, 0.5, 0.504, 0.504);
    });
    refuses(&mut lab, "lts.develop", develop(sliver, json!({"masks": [masked(json!({"kind": "sky"}))]})), "out_of_range");
    refuses(&mut lab, "lts.develop", develop(sliver, json!({"profile": {"id": "lc.neutral", "amount": 100.0}})), "out_of_range");
    applies(&mut lab, "lts.develop", develop(sliver, json!({"profile": {"id": "lc.neutral", "amount": 0.0}})));
    refuses(&mut lab, "lts.develop", values(sliver, json!({"profile.amount": 100})), "out_of_range");
    applies(&mut lab, "lts.develop", develop(sliver, json!({"profile": {"id": "lc.vivid", "amount": 100.0}})));
    committed(&mut lab, sliver, "Dehaze", |d| d.effects.dehaze = 10.0);
    refuses(&mut lab, "lts.develop", values(sliver, json!({"light.highlights": -20})), "out_of_range");
    applies(&mut lab, "lts.develop", values(sliver, json!({"effects.dehaze": 20})));

    // a crop is tighter when either measure of its magnification grows: a strip 2.05 % wide and the
    // photo's height (magnified about 1.5 times, or 1 on a portrait photo) squared up to 2.05 % ×
    // 2.05 % (about 49 times) keeps its smaller side, so the heal spot, the defringe and the dense
    // stroke the strip held (made by another path) are judged again there, by lts.crop and complete
    // settings alike; the strip itself still takes a slider and a reset
    let (healed, fringed, painted) = (plain[8], plain[9], plain[10]);
    let strip = |d: &mut DevelopSettings| {
        let rect = &mut d.crop.geometry.rect;
        (rect.x0, rect.y0, rect.x1, rect.y1) = (0.4, 0.0, 0.4205, 1.0);
    };
    let square = [0.4, 0.4, 0.4205, 0.4205];
    committed(&mut lab, healed, "Crop", |d| {
        strip(d);
        d.spots.push(serde_json::from_value(spot("heal", 0.41, 0.25)).expect("a spot"));
    });
    applies(&mut lab, "lts.develop", values(healed, json!({"light.exposure": 0.2})));
    refuses(&mut lab, "lts.crop", crop(healed, square), "out_of_range");
    let mut squared = settings_json(&lab.app, healed);
    squared["crop"]["geometry"]["rect"] = json!({"x0": 0.4, "y0": 0.4, "x1": 0.4205, "y1": 0.4205});
    let token = hash_of(&lab.app, healed);
    refuses(&mut lab, "lts.develop", checkout(healed, squared, token), "out_of_range");
    applies(&mut lab, "lts.crop", json!({"id": healed.0, "reset": true}));
    committed(&mut lab, fringed, "Crop", |d| {
        strip(d);
        d.optics.defringe_purple_amount = 20.0;
    });
    refuses_saying(&mut lab, "lts.crop", crop(fringed, square), "out_of_range", "defringe would reach");
    committed(&mut lab, painted, "Crop", |d| {
        strip(d);
        d.masks.push(serde_json::from_value(brush(vec![stroke(dense(0.41), 0.01, false)])).expect("a mask"));
    });
    refuses(&mut lab, "lts.crop", crop(painted, square), "budget_exceeded");

    // blur by blur, the widest answering for the widest: on a photo whose own crop is 1 % with a
    // Subject mask (made by another path), a second one is refused and the refusal names the blur,
    // while a People mask in its place keeps the width and applies
    let one_percent = |d: &mut DevelopSettings| {
        let rect = &mut d.crop.geometry.rect;
        (rect.x0, rect.y0, rect.x1, rect.y1) = (0.5, 0.5, 0.51, 0.51);
    };
    let (salient, filtered) = (plain[11], plain[12]);
    committed(&mut lab, salient, "Subject", |d| {
        one_percent(d);
        d.masks.push(serde_json::from_value(masked(json!({"kind": "subject"}))).expect("a mask"));
    });
    let second = replaced(&masked(json!({"kind": "subject"})), "/id", json!(2));
    let both = json!({"masks": [masked(json!({"kind": "subject"})), second]});
    let saliency = "a Subject, Background, People, Object or Prompt mask would blur";
    refuses_saying(&mut lab, "lts.develop", develop(salient, both), "out_of_range", saliency);
    let people = masked(json!({"kind": "people", "person": 0, "parts": []}));
    applies(&mut lab, "lts.develop", develop(salient, json!({"masks": [people]})));
    // and the photo's own defringe filter is its own: with defringe 10 there (some 410 pixels), a
    // slider and a lighter defringe apply, a stronger one is refused
    committed(&mut lab, filtered, "Defringe", |d| {
        one_percent(d);
        d.optics.defringe_purple_amount = 10.0;
    });
    applies(&mut lab, "lts.develop", values(filtered, json!({"light.exposure": 0.3})));
    let stronger = values(filtered, json!({"optics.defringePurple": 20}));
    refuses_saying(&mut lab, "lts.develop", stronger, "out_of_range", "defringe would reach");
    applies(&mut lab, "lts.develop", values(filtered, json!({"optics.defringePurple": 5})));

    // the section toggles count: with the geometry section off, an automatic Upright stored without
    // its transform is not solved from the pixels, so a 40 % crop and a heal spot apply; complete
    // settings that turn the section on are judged at 0.02 of the crop, and the refusal says so
    let unsolved = plain[13];
    committed(&mut lab, unsolved, "Upright", |d| {
        d.geometry.upright = Upright::Vertical;
        d.geometry.constrain_crop = true;
        d.set_section_enabled("geometry", false);
    });
    applies(&mut lab, "lts.crop", crop(unsolved, [0.3, 0.3, 0.7, 0.7]));
    applies(&mut lab, "lts.develop", develop(unsolved, json!({"spots": [spot("heal", 0.5, 0.006)]})));
    let mut enabled = settings_json(&lab.app, unsolved);
    enabled["disabled_sections"] = json!([]);
    let token = hash_of(&lab.app, unsolved);
    refuses_saying(&mut lab, "lts.develop", checkout(unsolved, enabled, token), "out_of_range", "counts as 0.02 of itself");
    // with the geometry or the optics section off, a scale of 50 or a distortion of -100 leaves a
    // corner crop as it is; complete settings that turn the section on change the frame, and
    // Constrain Crop then shrinks the crop to 0.2 % (refused), while a slider there applies
    let corner = |d: &mut DevelopSettings| {
        let rect = &mut d.crop.geometry.rect;
        (rect.x0, rect.y0, rect.x1, rect.y1) = (0.0, 0.0, 0.1, 0.1);
        d.geometry.constrain_crop = true;
    };
    let (scaled, distorted) = (plain[14], plain[15]);
    committed(&mut lab, scaled, "Transform", |d| {
        corner(d);
        d.geometry.scale = 50.0;
        d.set_section_enabled("geometry", false);
    });
    committed(&mut lab, distorted, "Distortion", |d| {
        corner(d);
        d.optics.distortion = -100.0;
        d.set_section_enabled("optics", false);
    });
    for id in [scaled, distorted] {
        applies(&mut lab, "lts.develop", values(id, json!({"light.exposure": 0.2})));
        let mut enabled = settings_json(&lab.app, id);
        enabled["disabled_sections"] = json!([]);
        let token = hash_of(&lab.app, id);
        refuses(&mut lab, "lts.develop", checkout(id, enabled, token), "out_of_range");
    }

    // Visualize Spots blurs at the crop's magnification too: on a photo whose own crop is 0.1 %
    // (made by another path) it is refused in a preview 2048 long, and shown in one 256 long and in
    // an uncropped one
    let speckled = plain[16];
    committed(&mut lab, speckled, "Crop", |d| {
        let rect = &mut d.crop.geometry.rect;
        (rect.x0, rect.y0, rect.x1, rect.y1) = (0.5, 0.5, 0.501, 0.501);
    });
    let view = |long: u32, framing: &str| json!({"id": speckled.0, "longEdge": long, "crop": framing, "overlay": {"spots": true}, "pixels": false});
    refuses_saying(&mut lab, "lts.preview", view(2048, "applied"), "out_of_range", "Visualize Spots would blur");
    for (long, framing) in [(256, "applied"), (2048, "uncropped")] {
        let reply = lts(&mut lab.app, "lts.preview", view(long, framing));
        assert_eq!(reply["outcome"], "ok", "Visualize Spots in {long} pixels, {framing}: {reply}");
    }
}

/// T-LI-21 (R1, master plan v3 W3; backlog item 16; ontology decision D13): the fork's entry point
/// refuses every id outside the hook's table. The page hands every id `lightcraft.command` receives
/// to `handle` before `web.stats` and `LightcraftApp::run` (`web.rs`, the inbox). In the LTS
/// presentation (`?embedded`) `handle` answers every id that does not start with `lts.` with a typed
/// refusal, `command_not_allowed`, that names the hook's commands, and runs nothing, whatever the
/// parameters (unreadable ones included): no engine or UI command (an edit, a crop, a selection, a
/// view, an export, an import, a delete, an undo, a history step, a preset, a preference, the GPU and
/// memory switches, a menu, a community link, a quit), no engine read (the reads of the table replace
/// them: T-LI-22) and not `web.stats`. The catalog, the selection, undo and redo, the journal, the
/// preferences, the UI state, the status line and the library store stay as they were, and no file
/// is written. The hook's own ids answer through the page exactly as through the hook's entry, a
/// mutation included. The standalone presentation stays upstream's (the regression control): there
/// `handle` answers no id outside `lts.`, so the page runs it on the editor's own command path, and
/// `lts.*` ids reach the hook alike. The page's inbox itself (`web.rs`, wasm32 only) is read here from
/// its source, the hook asked first; it runs end to end only in the WASM, which qualification
/// probes (master plan v3 QR-1, item 4: a raw engine id and `web.stats` through `lightcraft.command`).
#[test]
fn out_of_table_id_refused_at_fork() {
    let source = include_str!("../src/web.rs");
    let inbox = source.find("fn run_inbox").map(|at| &source[at..]).expect("web.rs has the page's inbox");
    let order = [inbox.find("crate::lts_view::handle(&mut self.app, &id"), inbox.find("\"web.stats\""), inbox.find("self.app.run(&id")];
    let hook_first = matches!(order, [Some(hook), Some(stats), Some(run)] if hook < stats && stats < run);
    assert!(hook_first, "the inbox asks the hook first, for every id, then web.stats, then the editor's own command path");

    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    let writes = Rc::new(Cell::new(0_usize));
    let counter = Rc::clone(&writes);
    lab.app.services.write = Some(Box::new(move |_path: &str, _bytes: &[u8]| -> Result<(), String> {
        counter.set(counter.get() + 1);
        Ok(())
    }));
    let by_id = json!({"id": target.0});
    let before = Witness::of(&lab.app);
    for (id, params) in [
        ("develop.set", json!({"ids": [target.0], "values": {"light.exposure": 3.0}})),
        ("crop.set", json!({"rect": [0.1, 0.1, 0.5, 0.5]})),
        ("library.select", json!({"ids": [target.0], "active": target.0})),
        ("view.detail", json!({})),
        ("app.export", json!({"ids": [target.0], "path": "lts-out.jpg", "format": "jpeg", "longEdge": 64})),
        ("library.import", json!({"paths": ["C:/Users/Public/outside.png"]})),
        ("photo.delete", json!({"ids": [target.0]})),
        ("edit.undo", json!({})),
        ("history.restore", json!({"index": 0})),
        ("preset.apply", json!({"id": "lc.warm-glow", "ids": [target.0]})),
        ("prefs.set", json!({"key": "xmp.autoWrite", "value": true})),
        ("app.gpu", json!({"enabled": true})),
        ("app.memoryBudget", json!({"mb": 64})),
        ("ui.menu.invoke", json!({"id": "app.quit"})),
        ("app.discord", json!({})),
        ("app.quit", json!({})),
        ("library.state", json!({})),
        ("catalog.query", json!({"filter": {}, "limit": 200})),
        ("develop.get", by_id.clone()),
        ("develop.controls", json!({})),
        ("photo.inspect", by_id.clone()),
        ("history.list", by_id.clone()),
        ("journal.list", json!({"limit": 1})),
        ("presets.list", json!({})),
        ("profiles.menu", json!({})),
        ("curve.presets", json!({})),
        ("photo.allMetadata", by_id.clone()),
        ("web.stats", json!({})),
        ("", json!({})),
        ("lts", json!({})),
        ("LTS.state", by_id.clone()),
        (" lts.state", by_id.clone()),
        ("ltsx.state", by_id.clone()),
        ("x.lts.state", by_id.clone()),
    ] {
        // the inbox hands parameters it cannot read to the hook as null
        for given in [params, Value::Null] {
            let what = format!("{id:?} {given}");
            let reply = at_the_fork(&mut lab.app, id, given);
            assert_eq!(refusal(&reply), "command_not_allowed", "{what}: the LTS editor runs no id outside its hook's table: {reply}");
            let named = reply["message"].as_str().is_some_and(|message| message.contains("lts.state"));
            assert!(named, "{what}: the refusal names the hook's commands: {reply}");
        }
    }
    assert_unchanged(&mut lab, &before, "ids outside the hook's table");
    assert_eq!(writes.get(), 0, "no file was written");

    // the hook's own commands answer through the page exactly as through the hook's entry
    for (id, params) in [
        ("lts.state", by_id.clone()),
        ("lts.preview", json!({"id": target.0, "longEdge": 64, "pixels": false})),
        ("lts.export", json!({"id": target.0, "format": "png", "longEdge": 64, "metadata": "none"})),
        ("lts.bogus", by_id.clone()),
    ] {
        let page = lts(&mut lab.app, id, params.clone());
        assert_eq!(page, lts_in(&mut lab.app, id, params, Runtime::HERE), "{id}: the page answers as the hook does");
    }
    let hash = hash_of(&lab.app, target);
    let edit = lts(&mut lab.app, "lts.develop", json!({"id": target.0, "values": {"light.exposure": 0.4}, "expectHash": hash}));
    assert_eq!(edit["outcome"], "applied", "a mutation of the hook's table through the page: {edit}");

    // the standalone page stays upstream's: the hook answers its own ids, the page runs the others
    let mut upstream = editing_lab(Presentation::Standalone);
    for (id, params) in [
        ("library.state", json!({})),
        ("develop.get", by_id.clone()),
        ("develop.set", json!({"values": {"light.exposure": 0.2}})),
        ("web.stats", json!({})),
    ] {
        assert!(handle(&mut upstream.app, id, &params).is_none(), "{id}: the standalone page runs it on the editor's own command path");
    }
    let reply = lts(&mut upstream.app, "lts.state", by_id);
    assert_eq!(reply["outcome"], "ok", "the hook answers in the standalone page too: {reply}");
}

/// T-LI-22 (R1): the reads of the table that replace the certified engine reads the LTS adapter and
/// the byte bridge sent through `lightcraft.command` (F0 T-LI-01: `library.state`, `catalog.query`,
/// `develop.get`, `develop.controls`, `photo.inspect`, `history.list`, `journal.list`,
/// `presets.list`, `profiles.menu` and `curve.presets`; FL1 T-LI-09: `photo.allMetadata`), which the
/// LTS presentation now refuses (T-LI-21). `lts.library {offset?, limit?, fileName?}` answers the
/// library's photos as `catalog.query {filter: {}}` lists them (`total`, and a page of 1 to 200 rows
/// in `photos`, 200 by default), each row with its photo's settings token (`hash`), and only the
/// photos of one exact file name when `fileName` names it (the byte bridge's import); `lts.photo
/// {id}` answers `photo.inspect` of that photo (`photo`: its history's labels and its versions
/// included) with its token and the token of each step of its history (`revisions`: the step, its
/// label and its token, the oldest first); `lts.state {id, settings: true}` adds the complete
/// settings `develop.get` gives (`settings`) to the tokens; `lts.presets` answers `presets.list`
/// (`presets`), `profiles.menu` (`profiles`) and `curve.presets` (`curvePresets`); `lts.metadata
/// {id}` answers `photo.allMetadata` (`metadata`: the file's EXIF and XMP rows, its position
/// included); `lts.journal {limit?}` answers `journal.list` (`journal`, the last 1 to 10,000
/// entries, 100 by default) and the journal's `length`; and `lts.controls` is the catalog of
/// T-LI-24. Each answers for an explicit photo of the library only (never the active one by
/// default): an unknown photo, or one in Recently Deleted, is `target_not_found`, and nothing is
/// written to the status line (where the engine's queries copied their refusals). They change
/// nothing, and both runtimes answer alike.
#[test]
fn typed_reads_answer_what_the_certified_queries_answered() {
    let (jpeg, _) = tagged_jpeg();
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    let imported = import(&mut lab, "tagged.jpg", &jpeg);
    applies(&mut lab, "lts.develop", json!({"id": target.0, "values": {"light.exposure": 0.3}, "label": "LTS develop"}));
    // the engine's own answers, read first (queries change nothing)
    let listing = engine_read(&mut lab.app, "catalog.query", json!({"filter": {}, "offset": 0, "limit": 200}));
    let page = engine_read(&mut lab.app, "catalog.query", json!({"filter": {}, "offset": 2, "limit": 3}));
    let inspected = engine_read(&mut lab.app, "photo.inspect", json!({"id": target.0}));
    let settings = engine_read(&mut lab.app, "develop.get", json!({"id": target.0}));
    let presets = engine_read(&mut lab.app, "presets.list", json!({}));
    let profiles = engine_read(&mut lab.app, "profiles.menu", json!({}));
    let curves = engine_read(&mut lab.app, "curve.presets", json!({}));
    let metadata = engine_read(&mut lab.app, "photo.allMetadata", json!({"id": imported.0}));
    let generated = engine_read(&mut lab.app, "photo.allMetadata", json!({"id": target.0}));
    let journal = engine_read(&mut lab.app, "journal.list", json!({}));
    let latest = engine_read(&mut lab.app, "journal.list", json!({"limit": 2}));
    settle(&mut lab);
    let before = Witness::of(&lab.app);

    // lts.library: catalog.query's rows, each with its photo's token
    let library = lts(&mut lab.app, "lts.library", json!({}));
    let revision = lab.app.session.catalog.revision;
    assert_eq!((library["outcome"].as_str(), &library["total"]), (Some("ok"), &listing["total"]), "{library}");
    assert_eq!(library["catalogRevision"], revision, "{library}");
    assert_rows(&lab.app, &library["photos"], &listing["photos"], "lts.library");
    let paged = lts(&mut lab.app, "lts.library", json!({"offset": 2, "limit": 3}));
    assert_eq!(paged["total"], page["total"], "{paged}");
    assert_rows(&lab.app, &paged["photos"], &page["photos"], "a page of lts.library");
    let rows = listing["photos"].as_array().cloned().unwrap_or_default();
    let row = rows.iter().find(|row| row["id"] == imported.0).cloned().expect("catalog.query lists the import");
    let named = lts(&mut lab.app, "lts.library", json!({"fileName": "tagged.jpg"}));
    assert_eq!(named["total"], 1, "the photo of that file name: {named}");
    assert_rows(&lab.app, &named["photos"], &json!([row]), "lts.library by file name");
    let absent = lts(&mut lab.app, "lts.library", json!({"fileName": "absent.png"}));
    assert_eq!((&absent["total"], &absent["photos"]), (&json!(0), &json!([])), "no photo of that name: {absent}");

    // lts.photo: photo.inspect, the photo's token and the token of each step of its history
    let inspect = lts(&mut lab.app, "lts.photo", json!({"id": target.0}));
    assert_eq!((inspect["outcome"].as_str(), inspect["photoId"].as_u64()), (Some("ok"), Some(target.0)), "{inspect}");
    assert_eq!(inspect["photo"], inspected, "lts.photo answers photo.inspect");
    assert_eq!(inspect["hash"], hash_of(&lab.app, target), "{inspect}");
    assert_eq!(inspect["revisions"], revisions(&lab.app, target), "a token for each step of the history, the oldest first");
    assert!(inspect["revisions"].as_array().is_some_and(|steps| !steps.is_empty()), "the agent's step is in the history: {inspect}");

    // lts.state {settings: true}: the tokens and the complete settings they name
    let state = lts(&mut lab.app, "lts.state", json!({"id": target.0, "settings": true}));
    assert_eq!((state["outcome"].as_str(), &state["settings"]), (Some("ok"), &settings), "lts.state adds develop.get's settings: {state}");
    assert_eq!(state["hash"], hash_of(&lab.app, target), "{state}");
    let tokens = lts(&mut lab.app, "lts.state", json!({"id": target.0}));
    assert!(tokens.get("settings").is_none(), "without `settings`, the tokens alone: {tokens}");
    assert_eq!(lts(&mut lab.app, "lts.state", json!({"id": target.0, "settings": false})), tokens, "`settings: false`: the tokens alone");

    // lts.presets: presets.list, profiles.menu and curve.presets
    let listed = lts(&mut lab.app, "lts.presets", json!({}));
    assert_eq!((&listed["presets"], &listed["profiles"], &listed["curvePresets"]), (&presets, &profiles, &curves), "{listed}");
    // lts.metadata: photo.allMetadata, the file's own rows (its position included: the adapter flags it)
    let file = lts(&mut lab.app, "lts.metadata", json!({"id": imported.0}));
    assert_eq!((file["photoId"].as_u64(), &file["metadata"]), (Some(imported.0), &metadata), "{file}");
    let located = metadata["exif"].as_array().is_some_and(|rows| rows.iter().any(|row| row["group"] == "GPS"));
    assert!(located, "the file's own rows: {metadata}");
    let demo = lts(&mut lab.app, "lts.metadata", json!({"id": target.0}));
    assert_eq!(demo["metadata"], generated, "a generated demo photo has no file: {demo}");
    // lts.journal: journal.list and the journal's length
    let entries = lts(&mut lab.app, "lts.journal", json!({}));
    let length = lab.app.session.journal.len();
    assert_eq!((&entries["journal"], &entries["length"]), (&journal, &json!(length)), "{entries}");
    let last = lts(&mut lab.app, "lts.journal", json!({"limit": 2}));
    assert_eq!(last["journal"], latest, "{last}");
    assert_unchanged(&mut lab, &before, "the reads of the table");

    // both runtimes answer alike
    for (id, params) in [
        ("lts.library", json!({"limit": 5})),
        ("lts.photo", json!({"id": target.0})),
        ("lts.state", json!({"id": target.0, "settings": true})),
        ("lts.controls", json!({"id": target.0})),
        ("lts.presets", json!({})),
        ("lts.metadata", json!({"id": imported.0})),
        ("lts.journal", json!({"limit": 3})),
    ] {
        let browser = lts_in(&mut lab.app, id, params.clone(), Runtime::Browser);
        assert_eq!(browser, lts_in(&mut lab.app, id, params, Runtime::Native), "{id}: both runtimes answer alike");
    }

    // explicit photos of the library only: an unknown photo, or one in Recently Deleted, is refused
    // without a word on the status line
    lab.app.run("photo.delete", json!({"ids": [imported.0]})).unwrap_or_else(|e| panic!("moving the import to Recently Deleted: {e}"));
    settle(&mut lab);
    let before = Witness::of(&lab.app);
    for (id, params) in [
        ("lts.photo", json!({"id": UNKNOWN})),
        ("lts.photo", json!({"id": imported.0})),
        ("lts.metadata", json!({"id": UNKNOWN})),
        ("lts.metadata", json!({"id": imported.0})),
        ("lts.state", json!({"id": UNKNOWN, "settings": true})),
        ("lts.controls", json!({"id": UNKNOWN})),
        ("lts.controls", json!({"id": imported.0})),
    ] {
        let reply = lts(&mut lab.app, id, params);
        assert_eq!(refusal(&reply), "target_not_found", "{id}: {reply}");
    }
    assert_unchanged(&mut lab, &before, "reads of unknown and deleted photos");
    let gone = lts(&mut lab.app, "lts.library", json!({"fileName": "tagged.jpg"}));
    assert_eq!((&gone["total"], &gone["photos"]), (&json!(0), &json!([])), "a photo in Recently Deleted is not the library's: {gone}");
}

/// T-LI-23 (R1, backlog item 22): one revision token, the fork's. Light's settings token is `hash64`
/// of the settings (FNV-1a over their JSON, 16 lowercase hex digits; the adapter writes it `l:<hash>`
/// with the kind `light.settings-hash64`), and the hook answers it beside every settings it names,
/// computed from those very settings, so that no caller computes a second token from a second read:
/// `lts.state {settings: true}` (the settings and their token in one answer), the rows of
/// `lts.library`, `lts.photo` (and each step of the history in `revisions`), `lts.controls {id}`,
/// `lts.preview`, `lts.export` and each mutation's `hashBefore` and `hashAfter`. The token a read
/// answers is the one `expectHash` takes, a history step's token is the `revision`
/// `lts.restoreSettings` puts back, and a mutation's `hashAfter` is the token the next read answers.
#[test]
fn one_revision_token_the_forks() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let target = other_photo(&lab.app, active);
    let agent = agent_then_human(&mut lab, target, active);
    let token = |settings: &Value| json!(hex(DevelopSettings::from_json(settings).unwrap_or_else(|e| panic!("settings JSON: {e}")).hash64()));
    let now = json!(hash_of(&lab.app, target));
    let state = lts(&mut lab.app, "lts.state", json!({"id": target.0, "settings": true}));
    assert_eq!(state["outcome"], "ok", "the tokens with the settings they name: {state}");
    assert_eq!((&state["hash"], token(&state["settings"])), (&now, now.clone()), "the photo's token, of exactly the settings answered: {state}");

    // every reply that names the photo's settings names that one token
    let library = lts(&mut lab.app, "lts.library", json!({}));
    let rows = library["photos"].as_array().cloned().unwrap_or_default();
    let row = rows.iter().find(|row| row["id"] == target.0).cloned().unwrap_or_default();
    let inspect = lts(&mut lab.app, "lts.photo", json!({"id": target.0}));
    let catalog = lts(&mut lab.app, "lts.controls", json!({"id": target.0}));
    let shown = preview(&mut lab.app, target, json!({"pixels": false}));
    let exported = lts(&mut lab.app, "lts.export", json!({"id": target.0, "format": "png", "longEdge": 64, "metadata": "none"}));
    for (what, reply) in
        [("lts.library", &row), ("lts.photo", &inspect), ("lts.controls", &catalog), ("lts.preview", &shown), ("lts.export", &exported)]
    {
        assert_eq!(reply["hash"], now, "{what} names the same token: {reply}");
    }
    assert_eq!(token(&inspect["photo"]["develop"]), now, "lts.photo's token is the token of the settings it answers");

    // each state of the history by its token, which lts.restoreSettings puts back
    assert_eq!(inspect["revisions"], revisions(&lab.app, target), "a token for each step of the history");
    let theirs = agent["hashAfter"].clone();
    let steps = inspect["revisions"].as_array().cloned().unwrap_or_default();
    assert!(steps.iter().any(|step| step["hash"] == theirs), "the agent's state is a step of the history: {inspect}");
    let restore = json!({"id": target.0, "revision": theirs, "expectHash": row["hash"]});
    let restored = lts(&mut lab.app, "lts.restoreSettings", restore);
    let tokens = (restored["outcome"].as_str(), &restored["hashBefore"], &restored["hashAfter"]);
    assert_eq!(tokens, (Some("applied"), &now, &theirs), "the token a read answered is the one expectHash takes: {restored}");
    let after = lts(&mut lab.app, "lts.state", json!({"id": target.0, "settings": true}));
    assert_eq!((&after["hash"], token(&after["settings"])), (&theirs, theirs.clone()), "the state put back, by its token: {after}");

    // a mutation's hashAfter is the token the next read answers
    let expect = json!({"id": target.0, "values": {"light.contrast": 12}, "expectHash": after["hash"]});
    let edit = lts(&mut lab.app, "lts.develop", expect);
    assert_eq!((edit["outcome"].as_str(), &edit["hashBefore"]), (Some("applied"), &theirs), "{edit}");
    let latest = lts(&mut lab.app, "lts.library", json!({}));
    let rows = latest["photos"].as_array().cloned().unwrap_or_default();
    let row = rows.iter().find(|row| row["id"] == target.0).cloned().unwrap_or_default();
    assert_eq!((&edit["hashAfter"], &row["hash"]), (&json!(hash_of(&lab.app, target)), &edit["hashAfter"]), "{edit}");
}

/// T-LI-24 (R1, backlog item 2): the hook's control catalog, `lts.controls {id?}`, labels the
/// controls that do nothing in some state. It lists every develop control as `develop.controls`
/// describes it (`id`, `label`, `section`, the range, default, step, decimals and track), each with
/// the gates it acts behind (`gates`: none for most), names the gates in order (`gates`: `id` and
/// `why`, when its controls change nothing and what opens it) and lists the inert controls (`inert`):
/// none since bug-fix batch 2 made Sharpening Radius, Sharpening Detail and Noise Reduction Contrast
/// render. The gates follow the renderer: the controls of the effects, optics, geometry and
/// calibration sections act only while their section is on (`effectsOn`, `opticsOn`, `geometryOn`,
/// `calibrationOn`), the straighten angle aside; the eight B&W mixes only under B&W, the treatment
/// or a B&W profile (`bw`); the point colours only in colour (`color`); the profile's amount only
/// under a profile with a look (`profileLook`); the lens profile sliders only with the lens profile
/// on and the matching correction embedded in the file (`lensDistortion`, `lensVignetting`: a DNG,
/// never a JPEG or a PNG); and the vignette's highlights only on a negative vignette in Highlight
/// Priority (`highlightPriority`). The Light, Color and Detail switches a person can turn off gate
/// nothing: the renderer ignores them. With `id`, the catalog gives that photo's values (its list
/// controls included), its settings token and each gate open or closed for it (`open`). Each label is
/// checked against the renderer: a gated control changes no pixel while its gate is closed and
/// changes the render once it is open, and the three detail controls change the render. The catalog
/// changes nothing. (The gates of the settings other controls act through are T-LI-27's; colour
/// opens `colorOrBwMix` and a vignette `vignetteOn` here too.)
#[test]
fn controls_catalog_labels_gated_and_inert_controls() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let specs = engine_read(&mut lab.app, "develop.controls", json!({}));
    settle(&mut lab);
    let before = Witness::of(&lab.app);
    let catalog = lts(&mut lab.app, "lts.controls", json!({}));
    assert_unchanged(&mut lab, &before, "lts.controls");
    assert_eq!(catalog["outcome"], "ok", "{catalog}");
    assert_eq!(catalog["inert"], json!([]), "no control is inert since bug-fix batch 2 (Radius, Detail and NR Contrast render): {catalog}");
    let gates = catalog["gates"].as_array().cloned().unwrap_or_default();
    let named: Vec<&str> = gates.iter().filter_map(|gate| gate["id"].as_str()).collect();
    assert_eq!(named, GATES, "the gates, in the catalog's order: {catalog}");
    for gate in &gates {
        assert!(gate["why"].as_str().is_some_and(|why| !why.is_empty()), "a gate says when its controls change nothing: {gate}");
    }
    assert!(catalog.get("open").is_none() && catalog.get("hash").is_none(), "without a photo, the catalog alone: {catalog}");

    // every develop control as develop.controls describes it, with the gates it acts behind
    let listed = catalog["controls"].as_array().cloned().unwrap_or_default();
    let ids: Vec<&str> = listed.iter().filter_map(|entry| entry["id"].as_str()).collect();
    let all: Vec<&str> = CONTROLS.iter().map(|spec| spec.id).collect();
    assert_eq!(ids, all, "the develop controls in the table's order; without a photo, no list controls");
    let engine = specs.as_array().cloned().unwrap_or_default();
    for entry in &listed {
        let id = entry["id"].as_str().unwrap_or_default();
        let spec = engine.iter().find(|spec| spec["id"] == id).unwrap_or_else(|| panic!("develop.controls lists {id}"));
        assert_eq!(without(entry, &["gates"]), without(spec, &["value"]), "{id}: as develop.controls describes it");
        assert_eq!(entry["gates"], json!(gates_of(id)), "{id}: its gates");
    }

    // a photo's catalog: its values, its token and the gates open for it
    let plain = plain_photos(&lab.app, &[active]);
    let (colour, look, lens, vignetted, effects) = (plain[0], plain[1], plain[2], plain[3], plain[4]);
    let (optics, geometry, calibrated, toggled) = (plain[5], plain[6], plain[7], plain[8]);
    let fresh = lts(&mut lab.app, "lts.controls", json!({"id": colour.0}));
    assert_eq!((fresh["photoId"].as_u64(), &fresh["hash"]), (Some(colour.0), &json!(hash_of(&lab.app, colour))), "{fresh}");
    assert_values(&lab.app, &fresh, colour);
    assert_eq!(open_gates(&mut lab.app, colour), PLAIN_OPEN, "a plain photo");

    // the B&W mixes act under B&W only, the point colours in colour only
    let bands = CONTROLS.iter().filter(|spec| spec.id.starts_with("bw."));
    let mixes = Value::Object(bands.map(|spec| (spec.id.to_string(), json!(100))).collect());
    assert!(!changes_render(&mut lab.app, colour, mixes.clone(), 256), "the B&W mixes change nothing in colour");
    applies(&mut lab, "lts.develop", json!({"id": colour.0, "settings": {"treatment": "bw"}}));
    assert_eq!(open_gates(&mut lab.app, colour), ["effectsOn", "opticsOn", "geometryOn", "calibrationOn", "bw"], "under B&W");
    assert!(changes_render(&mut lab.app, colour, mixes.clone(), 256), "the B&W mixes act under B&W");
    committed(&mut lab, colour, "Point Color", |d| d.point_colors.push(PointColor::default()));
    let shifts = json!({"pointColor.0.hueShift": 100, "pointColor.0.satShift": 100, "pointColor.0.lumShift": 100});
    assert!(!changes_render(&mut lab.app, colour, shifts, 256), "a point colour changes nothing under B&W");
    let pointed = lts(&mut lab.app, "lts.controls", json!({"id": colour.0}));
    assert_values(&lab.app, &pointed, colour);

    // the amount scales a profile's look; a B&W profile opens the B&W mixes too
    let amount = json!({"profile.amount": 200});
    assert!(!changes_render(&mut lab.app, look, amount.clone(), 256), "the amount changes nothing under Color, the default profile");
    applies(&mut lab, "lts.develop", json!({"id": look.0, "settings": {"profile": {"id": "lc.vivid", "amount": 100.0}}}));
    assert_eq!(open_gates(&mut lab.app, look), plain_and(&["profileLook"]), "a look");
    assert!(changes_render(&mut lab.app, look, amount, 256), "the amount scales a look");
    applies(&mut lab, "lts.develop", json!({"id": look.0, "settings": {"profile": {"id": "lc.mono", "amount": 100.0}}}));
    assert_eq!(open_gates(&mut lab.app, look), ["effectsOn", "opticsOn", "geometryOn", "calibrationOn", "bw", "profileLook"], "a B&W profile");
    assert!(changes_render(&mut lab.app, look, mixes, 256), "the B&W mixes act under a B&W profile");

    // the lens profile sliders need the correction in the file: a JPEG embeds none
    committed(&mut lab, lens, "Lens Profile", |d| d.optics.lens_profile = true);
    assert_eq!(open_gates(&mut lab.app, lens), PLAIN_OPEN, "the lens profile on, without lens data in the file");
    let profile = json!({"optics.profileDistortion": 200, "optics.profileVignetting": 200});
    assert!(!changes_render(&mut lab.app, lens, profile, 256), "the lens profile sliders change nothing without lens data");

    // the vignette's highlights act on a negative vignette in Highlight Priority only
    let highlights = json!({"vignette.highlights": 100});
    assert!(!changes_render(&mut lab.app, vignetted, highlights.clone(), 256), "the highlights change nothing without a vignette");
    applies(&mut lab, "lts.develop", json!({"id": vignetted.0, "values": {"light.exposure": 2, "vignette.amount": -80}}));
    let priority = plain_and(&["highlightPriority", "vignetteOn"]);
    assert_eq!(open_gates(&mut lab.app, vignetted), priority, "a negative vignette in Highlight Priority, the default style");
    assert!(changes_render(&mut lab.app, vignetted, highlights.clone(), 256), "the highlights act there");
    let mut styled = settings_json(&lab.app, vignetted);
    styled["vignette"]["style"] = json!("colorPriority");
    let token = hash_of(&lab.app, vignetted);
    applies(&mut lab, "lts.develop", json!({"id": vignetted.0, "settings": styled, "expectHash": token}));
    assert_eq!(open_gates(&mut lab.app, vignetted), plain_and(&["vignetteOn"]), "a negative vignette in Color Priority");
    assert!(!changes_render(&mut lab.app, vignetted, highlights, 256), "the highlights change nothing in Color Priority");

    // the four section switches the renderer honours
    let textures = json!({"effects.texture": 100, "effects.clarity": 100, "effects.dehaze": 100, "vignette.amount": -100, "grain.amount": 100});
    assert_section_gate(&mut lab, effects, "effects", "effectsOn", textures);
    let fringes = json!({"optics.distortion": 100, "optics.vignetting": -100, "optics.caRed": 100, "optics.defringePurple": 20});
    assert_section_gate(&mut lab, optics, "optics", "opticsOn", fringes);
    let warps = json!({"geometry.vertical": 50, "geometry.rotate": 10, "geometry.scale": 80});
    assert_section_gate(&mut lab, geometry, "geometry", "geometryOn", warps);
    let primaries = json!({"calibration.redSat": 100, "calibration.greenHue": 100, "calibration.shadowsTint": 100});
    assert_section_gate(&mut lab, calibrated, "calibration", "calibrationOn", primaries);
    // the straighten angle is the crop's: it renders with the geometry section off
    committed(&mut lab, geometry, "Toggle Section", |d| d.set_section_enabled("geometry", false));
    assert!(changes_render(&mut lab.app, geometry, json!({"crop.angle": 10}), 256), "the straighten angle with the geometry section off");

    // the Light, Color and Detail switches a person can turn off gate nothing: the renderer ignores them
    committed(&mut lab, toggled, "Toggle Section", |d| d.disabled_sections = vec!["light".into(), "color".into(), "detail".into()]);
    assert_eq!(open_gates(&mut lab.app, toggled), PLAIN_OPEN, "the Light, Color and Detail switches off");
    for values in [json!({"light.exposure": 1}), json!({"color.saturation": 100}), json!({"detail.sharpenAmount": 150})] {
        assert!(changes_render(&mut lab.app, toggled, values.clone(), 256), "{values}: renders with its section switched off");
    }

    // the three detail controls render since bug-fix batch 2 (on the demo's first scene, as its own
    // tests render it)
    let mut demo = boot(Presentation::LtsEmbedded, true);
    let first = all_photos(&demo.app).into_iter().find(|p| p.source == Source::Demo { scene: 1 }).map(|p| p.id).expect("the first scene");
    applies(&mut demo, "lts.develop", json!({"id": first.0, "values": {"detail.sharpenAmount": 100}}));
    for values in [json!({"detail.sharpenRadius": 3}), json!({"detail.sharpenRadius": 0.5}), json!({"detail.sharpenDetail": 100})] {
        assert!(changes_render(&mut demo.app, first, values.clone(), 480), "{values}: renders while sharpening is on");
    }
    applies(&mut demo, "lts.develop", json!({"id": first.0, "values": {"detail.sharpenAmount": 0, "detail.nrLuminance": 80}}));
    assert!(changes_render(&mut demo.app, first, json!({"detail.nrContrast": 100}), 480), "NR Contrast renders while noise reduction is on");
}

/// T-LI-25 (R1): `lts.select {id, detail?}` is the declared selection the adapter made with
/// `library.select` and `view.detail`, which the LTS presentation now refuses (T-LI-21). It makes
/// photo `id` the only selected photo and the active one, exactly as `library.select {ids: [id],
/// active: id}` leaves an identical library (the automatic version of the photo it leaves included),
/// and with `detail: true` shows it in the detail view, as `view.detail` does; without `detail` the
/// view stays, and a session without the editor's UI (a native worker's) has no view to show (`view`:
/// null). It is never journaled, answers the photo, the active photo, the view shown, the photo's
/// token, the catalog revision and the history head, and refuses a human interaction in progress
/// (`editor_busy`: it never ends a person's drag, which `library.select` does), an unknown photo or
/// one in Recently Deleted (`target_not_found`) and malformed parameters (`invalid_params`), with
/// nothing changed.
#[test]
fn select_is_the_declared_selection_typed() {
    let mut hook = editing_lab(Presentation::LtsEmbedded);
    let mut oracle = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&hook.app);
    let target = other_photo(&hook.app, active);
    let journal = hook.app.session.journal.len();
    // each library the process opens counts its revisions in a range of its own
    // (`crates/engine/src/library.rs:299-304`): the two take the same steps, never the same revision
    let revisions = (hook.app.session.catalog.revision, oracle.app.session.catalog.revision);
    let what = "lts.select with the detail view";
    let reply = lts(&mut hook.app, "lts.select", json!({"id": target.0, "detail": true}));
    assert_eq!(reply["outcome"], "applied", "{what}: {reply}");
    for (command, params) in [("library.select", json!({"ids": [target.0], "active": target.0})), ("view.detail", json!({}))] {
        oracle.app.run(command, params).unwrap_or_else(|e| panic!("the oracle's {command}: {e}"));
    }
    assert_same_photos(&all_photos(&hook.app), &all_photos(&oracle.app), what);
    let (h, o) = (&hook.app.session, &oracle.app.session);
    assert_eq!(format!("{:?}", h.selection), format!("{:?}", o.selection), "{what}: the selection");
    assert_eq!((h.active(), h.previous_active), (o.active(), o.previous_active), "{what}: the active photo");
    assert_eq!(format!("{:?}", (h.active_mask, h.active_spot)), format!("{:?}", (o.active_mask, o.active_spot)), "{what}: no mask or spot picked");
    assert_eq!(format!("{:?}", (&h.undo, &h.redo)), format!("{:?}", (&o.undo, &o.redo)), "{what}: undo and redo");
    let steps = (h.catalog.revision - revisions.0, o.catalog.revision - revisions.1);
    assert_eq!(steps.0, steps.1, "{what}: the catalog's steps");
    assert_eq!(format!("{:?}", hook.app.ui), format!("{:?}", oracle.app.ui), "{what}: the editor's UI, the detail view shown");
    assert_eq!((h.active(), format!("{:?}", hook.app.ui.view)), (Some(target), "Detail".to_string()), "{what}");
    assert_eq!(h.journal.len(), journal, "{what}: never journaled");
    let fields = (reply["photoId"].as_u64(), reply["activePhoto"].as_u64(), reply["view"].as_str(), reply["catalogRevision"].as_u64());
    assert_eq!(fields, (Some(target.0), Some(target.0), Some("detail"), Some(h.catalog.revision)), "{what}: {reply}");
    assert_eq!((&reply["hash"], &reply["historyHead"]), (&json!(hash_of(&hook.app, target)), &history_head(&hook.app)), "{what}: {reply}");

    // without `detail` the view stays; a session without the editor's UI has no view to show
    engine_read(&mut hook.app, "view.photoGrid", json!({}));
    let grid = format!("{:?}", hook.app.ui.view);
    let plain = lts(&mut hook.app, "lts.select", json!({"id": active.0}));
    assert_eq!((plain["outcome"].as_str(), &plain["view"]), (Some("applied"), &Value::Null), "lts.select without the detail view: {plain}");
    assert_eq!((hook.app.session.active(), format!("{:?}", hook.app.ui.view)), (Some(active), grid.clone()), "the view stays");
    let native = lts_in(&mut hook.app, "lts.select", json!({"id": target.0, "detail": true}), Runtime::Native);
    assert_eq!((native["outcome"].as_str(), &native["view"]), (Some("applied"), &Value::Null), "a session without the UI: {native}");
    assert_eq!((hook.app.session.active(), format!("{:?}", hook.app.ui.view)), (Some(target), grid), "a session has no view to change");
    assert_eq!(hook.app.session.journal.len(), journal, "never journaled");
    settle(&mut hook);

    // a person's drag is refused, never ended
    hook.app.run("develop.beginInteraction", json!({"label": "Exposure"})).unwrap_or_else(|e| panic!("an interaction: {e}"));
    settle(&mut hook);
    let before = Witness::of(&hook.app);
    let busy = lts(&mut hook.app, "lts.select", json!({"id": active.0, "detail": true}));
    assert_eq!(refusal(&busy), "editor_busy", "lts.select during a human interaction: {busy}");
    assert_unchanged(&mut hook, &before, "lts.select during a human interaction");
    hook.app.run("develop.cancelInteraction", json!({})).unwrap_or_else(|e| panic!("cancelling the interaction: {e}"));

    // unknown and deleted photos, and malformed parameters
    let gone = plain_photos(&hook.app, &[active, target])[0];
    hook.app.run("photo.delete", json!({"ids": [gone.0]})).unwrap_or_else(|e| panic!("moving a photo to Recently Deleted: {e}"));
    settle(&mut hook);
    let before = Witness::of(&hook.app);
    for (params, code) in [
        (json!({"id": UNKNOWN}), "target_not_found"),
        (json!({"id": gone.0, "detail": true}), "target_not_found"),
        (json!({}), "invalid_params"),
        (json!({"id": target.0, "detail": 1}), "invalid_params"),
        (json!({"id": target.0, "ids": [target.0]}), "invalid_params"),
        (json!({"id": target.0, "mode": "toggle"}), "invalid_params"),
    ] {
        let what = format!("lts.select {params}");
        let reply = lts(&mut hook.app, "lts.select", params);
        assert_eq!(refusal(&reply), code, "{what}: {reply}");
    }
    assert_unchanged(&mut hook, &before, "refused selections");
}

/// T-LI-26 (R1, bug-fix batch 2 notes): the sharpening band that Sharpening Radius widens since bug-fix
/// batch 2 (`plane_sigmas`'s `sharpen`: the texture band's sigma times Radius, 0.5 to 3, while
/// sharpening is on and Radius is not 1) is one of the blurs the renderer derives from the settings,
/// held at the crop's magnification as the others are (T-LI-20). On a photo whose own crop is 0.2 %
/// (made by another path), which the renderer magnifies 500 times (the texture band's sigma some
/// 3,700 pixels at a 4096-pixel export), sharpening at Radius 1 applies; Radius 3 and Radius 2 (some
/// 11,000 and 7,400 pixels) are refused (`out_of_range`, naming the sharpening band), in
/// `lts.develop` and in a `lts.preview` what-if alike; Radius 1.5 (some 5,500 pixels) applies, and
/// then a slider, the amount and a narrower Radius apply while a wider one is refused. A photo's own
/// band is its own: on a photo that holds Radius 3 there (made by another path), a slider and a
/// narrower Radius apply and widening it again is refused; and a mask whose own sharpness turns
/// sharpening on at Radius 3 is refused there too. In the whole photo every Radius applies. Nothing
/// changes on a refusal.
#[test]
fn sharpening_band_held_at_the_crops_magnification() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let plain = plain_photos(&lab.app, &[active]);
    let (sharp, held, masked, whole) = (plain[0], plain[1], plain[2], plain[3]);
    let values = |id: PhotoId, controls: Value| json!({"id": id.0, "values": controls});
    let tiny = |d: &mut DevelopSettings| {
        let rect = &mut d.crop.geometry.rect;
        (rect.x0, rect.y0, rect.x1, rect.y1) = (0.5, 0.5, 0.502, 0.502);
    };
    let band = "the sharpening band";

    // a 0.2 % crop: Radius 1 needs no band of its own; 3 and 2 are past the bound, 1.5 is within it
    committed(&mut lab, sharp, "Crop", tiny);
    applies(&mut lab, "lts.develop", values(sharp, json!({"detail.sharpenAmount": 40})));
    for radius in [3.0, 2.0] {
        refuses_saying(&mut lab, "lts.develop", values(sharp, json!({"detail.sharpenRadius": radius})), "out_of_range", band);
    }
    let what_if = json!({"id": sharp.0, "longEdge": 64, "variant": "settings", "values": {"detail.sharpenRadius": 3}});
    refuses_saying(&mut lab, "lts.preview", what_if, "out_of_range", band);
    applies(&mut lab, "lts.develop", values(sharp, json!({"detail.sharpenRadius": 1.5})));
    applies(&mut lab, "lts.develop", values(sharp, json!({"light.exposure": 0.3})));
    applies(&mut lab, "lts.develop", values(sharp, json!({"detail.sharpenAmount": 120})));
    refuses_saying(&mut lab, "lts.develop", values(sharp, json!({"detail.sharpenRadius": 2})), "out_of_range", band);
    applies(&mut lab, "lts.develop", values(sharp, json!({"detail.sharpenRadius": 0.5})));

    // a photo's own band is its own: a slider and a narrower Radius apply, widening it again does not
    committed(&mut lab, held, "Sharpening", |d| {
        tiny(d);
        d.detail.sharpen_amount = 40.0;
        d.detail.sharpen_radius = 3.0;
    });
    applies(&mut lab, "lts.develop", values(held, json!({"light.exposure": 0.2})));
    applies(&mut lab, "lts.develop", values(held, json!({"detail.sharpenRadius": 2.5})));
    refuses_saying(&mut lab, "lts.develop", values(held, json!({"detail.sharpenRadius": 3})), "out_of_range", band);

    // a mask's own sharpness turns sharpening on at the photo's Radius
    committed(&mut lab, masked, "Radius", |d| {
        tiny(d);
        d.detail.sharpen_radius = 3.0;
    });
    let gradient = json!({"kind": "linear", "start": point(0.5, 0.5), "end": point(0.502, 0.502)});
    let mask = canonical::<Mask>(json!({"id": 1, "components": [{"shape": gradient}], "adjust": {"sharpness": 50}}));
    refuses_saying(&mut lab, "lts.develop", json!({"id": masked.0, "settings": {"masks": [mask]}}), "out_of_range", band);

    // in the whole photo every Radius applies
    applies(&mut lab, "lts.develop", values(whole, json!({"detail.sharpenAmount": 150, "detail.sharpenRadius": 3})));
}

/// T-LI-27 (R1, verifier round 2; backlog item 2): the control catalog also labels the controls that
/// act only through another setting, each gate checked against the renderer: its controls change no
/// pixel while it is closed and change the render once it is open. On a plain photo every such gate
/// is closed but `colorOrBwMix` (colour), and their 27 controls together change nothing. Opened in
/// turn: the vignette's Midpoint, Roundness and Feather by an amount (`vignetteOn`), the grain's Size
/// and Roughness by its amount (`grainOn`); a region split by a region slider (`curveRegions`) and
/// Refine Saturation by any curve (`curveOn`), a point curve alone opening only the second; Blending
/// and Balance by a Shadows, Midtones or Highlights wheel (`gradingZones`), the Global wheel alone
/// leaving them inert, and each wheel's Hue by that wheel's own Saturation (`shadowsTint` to
/// `globalTint`), a profile's look's tint opening Blending but leaving the photo's own Hues inert;
/// Vibrance, Saturation and the mixer's Hue and Saturation in colour, and in black and white only
/// through a B&W mix (`colorOrBwMix`: the B&W treatment with a neutral mix and the `lc.mono` look
/// leave them inert, a mix of the photo's or of the `lc.bw.red-filter` look's opens them); the lens
/// Vignetting's Midpoint by that Vignetting (`manualVignetting`); each defringe hue range by its
/// amount (`defringePurple`, `defringeGreen`), on a photo with a purple and a green fringe along its
/// edges, one amount leaving the other range inert; a point colour's ranges by a shift of its sample
/// (`pointColorShift`), on a ramp of colours, the sample picked in the editor; and, on the demo's
/// first scene, Radius, Detail and Masking by Sharpening or by a mask's sharpness (`sharpenOn`) and
/// each noise reduction's sliders by its amount (`nrLuminanceOn`, `nrColorOn`).
#[test]
fn controls_catalog_labels_the_settings_controls_act_through() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let plain = plain_photos(&lab.app, &[active]);
    let (untouched, vignetted, grained, curved, bent) = (plain[0], plain[1], plain[2], plain[3], plain[4]);
    let (graded, looked, mono, filtered, lensed) = (plain[5], plain[6], plain[7], plain[8], plain[9]);
    let values = |id: PhotoId, controls: Value| json!({"id": id.0, "values": controls});

    // a plain photo: the gates of an amount, a curve, a wheel and a shift closed, their controls inert
    assert_eq!(open_gates(&mut lab.app, untouched), PLAIN_OPEN, "a plain photo");
    let inert = json!({
        "vignette.midpoint": 10, "vignette.roundness": 80, "vignette.feather": 10, "grain.size": 100, "grain.roughness": 100,
        "curve.splitShadows": 60, "curve.splitMid": 30, "curve.splitHighlights": 40, "curve.refineSaturation": 0,
        "grading.blending": 100, "grading.balance": 100,
        "grading.shadows.hue": 200, "grading.midtones.hue": 200, "grading.highlights.hue": 200, "grading.global.hue": 200,
        "detail.sharpenRadius": 3, "detail.sharpenDetail": 100, "detail.sharpenMasking": 100,
        "detail.nrDetail": 100, "detail.nrContrast": 100, "detail.nrColorDetail": 0, "detail.nrColorSmoothness": 100,
        "optics.vignettingMidpoint": 0,
        "optics.defringePurpleHueLo": 0, "optics.defringePurpleHueHi": 10, "optics.defringeGreenHueLo": 0, "optics.defringeGreenHueHi": 10,
    });
    assert!(!changes_render(&mut lab.app, untouched, inert, 256), "the controls behind the closed gates change no pixel");

    // the vignette's shape and the grain's follow their amounts
    applies(&mut lab, "lts.develop", values(vignetted, json!({"vignette.amount": -80})));
    assert_eq!(open_gates(&mut lab.app, vignetted), plain_and(&["highlightPriority", "vignetteOn"]), "a vignette");
    for controls in [json!({"vignette.midpoint": 10}), json!({"vignette.roundness": 80}), json!({"vignette.feather": 10})] {
        assert!(changes_render(&mut lab.app, vignetted, controls.clone(), 256), "{controls}: shapes the vignette");
    }
    applies(&mut lab, "lts.develop", values(grained, json!({"grain.amount": 100})));
    assert_eq!(open_gates(&mut lab.app, grained), plain_and(&["grainOn"]), "grain");
    for controls in [json!({"grain.size": 100}), json!({"grain.roughness": 100})] {
        assert!(changes_render(&mut lab.app, grained, controls.clone(), 256), "{controls}: shapes the grain");
    }

    // a region split follows a region slider, Refine Saturation any curve
    applies(&mut lab, "lts.develop", values(curved, json!({"curve.shadows": -60})));
    assert_eq!(open_gates(&mut lab.app, curved), plain_and(&["curveRegions", "curveOn"]), "a region slider");
    for controls in [json!({"curve.splitShadows": 60}), json!({"curve.refineSaturation": 0})] {
        assert!(changes_render(&mut lab.app, curved, controls.clone(), 256), "{controls}: acts on the curve");
    }
    let mut curve = settings_json(&lab.app, bent)["curve"].clone();
    curve["master"] = json!([point(0.0, 0.0), point(0.5, 0.35), point(1.0, 1.0)]);
    applies(&mut lab, "lts.develop", json!({"id": bent.0, "settings": {"curve": curve}}));
    assert_eq!(open_gates(&mut lab.app, bent), plain_and(&["curveOn"]), "a point curve alone");
    let splits = json!({"curve.splitShadows": 60, "curve.splitMid": 30, "curve.splitHighlights": 40});
    assert!(!changes_render(&mut lab.app, bent, splits, 256), "the splits change nothing without a region slider");
    assert!(changes_render(&mut lab.app, bent, json!({"curve.refineSaturation": 0}), 256), "Refine Saturation tempers a point curve");

    // Blending and Balance follow a zone wheel, a wheel's Hue its own Saturation, under a look's tint too
    applies(&mut lab, "lts.develop", values(graded, json!({"grading.global.sat": 60})));
    assert_eq!(open_gates(&mut lab.app, graded), plain_and(&["globalTint"]), "the Global wheel alone");
    let zones = json!({"grading.blending": 100, "grading.balance": 100, "grading.shadows.hue": 200, "grading.midtones.hue": 200});
    assert!(!changes_render(&mut lab.app, graded, zones, 256), "Blending, Balance and the zones' Hues change nothing under the Global wheel");
    assert!(changes_render(&mut lab.app, graded, json!({"grading.global.hue": 200}), 256), "the Global wheel's Hue turns its tint");
    applies(&mut lab, "lts.develop", values(graded, json!({"grading.shadows.sat": 60})));
    assert_eq!(open_gates(&mut lab.app, graded), plain_and(&["gradingZones", "shadowsTint", "globalTint"]), "a Shadows wheel");
    for controls in [json!({"grading.blending": 100}), json!({"grading.balance": 100}), json!({"grading.shadows.hue": 200})] {
        assert!(changes_render(&mut lab.app, graded, controls.clone(), 256), "{controls}: acts on the Shadows wheel");
    }
    let others = json!({"grading.midtones.hue": 200, "grading.highlights.hue": 200});
    assert!(!changes_render(&mut lab.app, graded, others, 256), "a wheel's Hue changes nothing at its own Saturation 0");
    applies(&mut lab, "lts.develop", json!({"id": looked.0, "settings": {"profile": {"id": "lc.cine.teal-amber", "amount": 100.0}}}));
    assert_eq!(open_gates(&mut lab.app, looked), plain_and(&["profileLook", "gradingZones"]), "a look's wheels");
    assert!(changes_render(&mut lab.app, looked, json!({"grading.blending": 100}), 256), "Blending shares a look's wheels");
    let hues = json!({"grading.shadows.hue": 200, "grading.highlights.hue": 200});
    assert!(!changes_render(&mut lab.app, looked, hues, 256), "the photo's own Hues change nothing under a look's tint");

    // Vibrance, Saturation and the mixer's Hue and Saturation: in colour, and in B&W only through a mix
    let bands = ["red", "orange", "yellow", "green", "aqua", "blue", "purple", "magenta"];
    let mixer = |field: &str| Value::Object(bands.iter().map(|band| (format!("mixer.{band}.{field}"), json!(100))).collect());
    let chroma = with(&with(&json!({"color.vibrance": 100, "color.saturation": 100}), mixer("hue")), mixer("sat"));
    assert!(changes_render(&mut lab.app, mono, chroma.clone(), 256), "they render in colour");
    applies(&mut lab, "lts.develop", json!({"id": mono.0, "settings": {"treatment": "bw"}}));
    assert_eq!(open_gates(&mut lab.app, mono), ["effectsOn", "opticsOn", "geometryOn", "calibrationOn", "bw"], "B&W, a neutral mix");
    assert!(!changes_render(&mut lab.app, mono, chroma.clone(), 256), "they change nothing in black and white with a neutral mix");
    let mix = Value::Object(bands.iter().zip([100, -100].iter().cycle()).map(|(band, weight)| (format!("bw.{band}"), json!(weight))).collect());
    applies(&mut lab, "lts.develop", values(mono, mix));
    let mixed = ["effectsOn", "opticsOn", "geometryOn", "calibrationOn", "bw", "colorOrBwMix"];
    assert_eq!(open_gates(&mut lab.app, mono), mixed, "B&W with a mix");
    for controls in [json!({"color.vibrance": 100}), json!({"color.saturation": 100}), mixer("hue"), mixer("sat")] {
        assert!(changes_render(&mut lab.app, mono, controls.clone(), 256), "{controls}: renders through the B&W mix");
    }
    applies(&mut lab, "lts.develop", json!({"id": filtered.0, "settings": {"profile": {"id": "lc.mono", "amount": 100.0}}}));
    let mono_look = ["effectsOn", "opticsOn", "geometryOn", "calibrationOn", "bw", "profileLook"];
    assert_eq!(open_gates(&mut lab.app, filtered), mono_look, "a B&W look with no mix");
    assert!(!changes_render(&mut lab.app, filtered, chroma, 256), "they change nothing under a B&W look with no mix");
    applies(&mut lab, "lts.develop", json!({"id": filtered.0, "settings": {"profile": {"id": "lc.bw.red-filter", "amount": 100.0}}}));
    let red_filter = ["effectsOn", "opticsOn", "geometryOn", "calibrationOn", "bw", "profileLook", "colorOrBwMix"];
    assert_eq!(open_gates(&mut lab.app, filtered), red_filter, "a B&W look with a mix");
    assert!(changes_render(&mut lab.app, filtered, json!({"color.saturation": 100}), 256), "Saturation renders through the look's mix");

    // the lens Vignetting's Midpoint follows that Vignetting
    applies(&mut lab, "lts.develop", values(lensed, json!({"optics.vignetting": -100})));
    assert_eq!(open_gates(&mut lab.app, lensed), plain_and(&["manualVignetting"]), "the lens Vignetting");
    assert!(changes_render(&mut lab.app, lensed, json!({"optics.vignettingMidpoint": 0}), 256), "its Midpoint shapes it");

    // each defringe hue range follows its amount, on a photo with a purple and a green fringe
    let fringed = import(&mut lab, "fringed.png", &fringed_png());
    assert_eq!(open_gates(&mut lab.app, fringed), PLAIN_OPEN, "no defringe");
    let purple = json!({"optics.defringePurpleHueLo": 0, "optics.defringePurpleHueHi": 10});
    let green = json!({"optics.defringeGreenHueLo": 0, "optics.defringeGreenHueHi": 10});
    assert!(!changes_render(&mut lab.app, fringed, with(&purple, green.clone()), 64), "the hue ranges change nothing without defringe");
    applies(&mut lab, "lts.develop", values(fringed, json!({"optics.defringeGreen": 20})));
    assert_eq!(open_gates(&mut lab.app, fringed), plain_and(&["defringeGreen"]), "green defringe");
    assert!(!changes_render(&mut lab.app, fringed, purple.clone(), 64), "the purple range changes nothing at the purple amount 0");
    assert!(changes_render(&mut lab.app, fringed, green, 64), "the green range picks the green fringe");
    applies(&mut lab, "lts.develop", values(fringed, json!({"optics.defringePurple": 20})));
    assert_eq!(open_gates(&mut lab.app, fringed), plain_and(&["defringePurple", "defringeGreen"]), "purple and green defringe");
    assert!(changes_render(&mut lab.app, fringed, purple, 64), "the purple range picks the purple fringe");

    // a point colour's ranges follow a shift of its sample, picked in the editor on a ramp of colours
    let ramp = import(&mut lab, "ramp.png", &ramp_png());
    lab.app.run("library.select", json!({"ids": [ramp.0]})).unwrap_or_else(|e| panic!("selecting the ramp: {e}"));
    lab.app.run("pointColor.pick", json!({"x": 0.5, "y": 0.5})).unwrap_or_else(|e| panic!("picking a point colour: {e}"));
    settle(&mut lab);
    assert_eq!(open_gates(&mut lab.app, ramp), PLAIN_OPEN, "a sample that shifts nothing");
    let ranges = json!({"pointColor.0.range": 0, "pointColor.0.hueRange": 0, "pointColor.0.satRange": 0, "pointColor.0.lumRange": 0});
    assert!(!changes_render(&mut lab.app, ramp, ranges, 64), "a sample's ranges change nothing while it shifts nothing");
    applies(&mut lab, "lts.develop", values(ramp, json!({"pointColor.0.lumShift": 100})));
    assert_eq!(open_gates(&mut lab.app, ramp), plain_and(&["pointColorShift"]), "a sample that shifts its luminance");
    for field in ["range", "hueRange", "satRange", "lumRange"] {
        let narrowed = json!({format!("pointColor.0.{field}"): 0});
        assert!(changes_render(&mut lab.app, ramp, narrowed.clone(), 64), "{narrowed}: selects what the shift moves");
    }

    // Radius, Detail and Masking follow sharpening or a mask's sharpness, and each noise reduction's
    // sliders its amount (on the demo's first scene, as T-LI-24 renders its detail)
    let mut demo = boot(Presentation::LtsEmbedded, true);
    let first = all_photos(&demo.app).into_iter().find(|p| p.source == Source::Demo { scene: 1 }).map(|p| p.id).expect("the first scene");
    let sharpening = json!({"detail.sharpenRadius": 3, "detail.sharpenDetail": 100, "detail.sharpenMasking": 100});
    let reduction = json!({"detail.nrDetail": 100, "detail.nrContrast": 100, "detail.nrColorDetail": 0, "detail.nrColorSmoothness": 100});
    assert_eq!(open_gates(&mut demo.app, first), PLAIN_OPEN, "the first scene");
    assert!(!changes_render(&mut demo.app, first, with(&sharpening, reduction), 480), "none of them acts at the amounts 0");
    applies(&mut demo, "lts.develop", values(first, json!({"detail.sharpenAmount": 100})));
    assert_eq!(open_gates(&mut demo.app, first), plain_and(&["sharpenOn"]), "sharpening");
    for controls in [json!({"detail.sharpenRadius": 3}), json!({"detail.sharpenDetail": 100}), json!({"detail.sharpenMasking": 100})] {
        assert!(changes_render(&mut demo.app, first, controls.clone(), 480), "{controls}: shapes the sharpening");
    }
    applies(&mut demo, "lts.develop", values(first, json!({"detail.sharpenAmount": 0, "detail.nrLuminance": 80, "detail.nrColor": 80})));
    assert_eq!(open_gates(&mut demo.app, first), plain_and(&["nrLuminanceOn", "nrColorOn"]), "noise reduction");
    assert!(!changes_render(&mut demo.app, first, sharpening, 480), "Radius, Detail and Masking change nothing at Sharpening 0");
    for controls in [
        json!({"detail.nrDetail": 100}),
        json!({"detail.nrContrast": 100}),
        json!({"detail.nrColorDetail": 0}),
        json!({"detail.nrColorSmoothness": 100}),
    ] {
        assert!(changes_render(&mut demo.app, first, controls.clone(), 480), "{controls}: shapes the noise reduction");
    }
    let gradient = json!({"kind": "linear", "start": point(0.5, 0.0), "end": point(0.5, 1.0)});
    let mask = canonical::<Mask>(json!({"id": 1, "components": [{"shape": gradient}], "adjust": {"sharpness": 100}}));
    applies(&mut demo, "lts.develop", json!({"id": first.0, "settings": {"masks": [mask]}}));
    assert_eq!(open_gates(&mut demo.app, first), plain_and(&["sharpenOn", "nrLuminanceOn", "nrColorOn"]), "a mask that sharpens");
    assert!(changes_render(&mut demo.app, first, json!({"detail.sharpenRadius": 3}), 480), "Radius shapes a mask's sharpening");
}

/// T-LI-28 (R1, verifier round 2; master plan v3 W3, the hooks V4 needs): complete settings are held
/// to the hook's bounds on every part that differs from the photo's own, so a checkout
/// (`lts.develop {settings, expectHash}`) of a state a person made past them is refused once the photo
/// has left it, with nothing changed: brush work whose paint is past the hook's budget, which the
/// editor's brush allows (`budget_exceeded`), and more brush strokes than the hook's cap, which the
/// editor does not cap (`out_of_range`). `lts.restoreSettings` puts such a state back by its token,
/// unjudged, while the photo's history records it, exactly as the person left it: the fallback for a
/// version-control checkout.
#[test]
fn checkout_past_the_bounds_falls_back_to_restore_settings() {
    let mut lab = editing_lab(Presentation::LtsEmbedded);
    let active = active_photo(&lab.app);
    let plain = plain_photos(&lab.app, &[active]);
    let brush = |strokes: Vec<Value>| canonical::<Mask>(json!({"id": 1, "components": [{"shape": {"kind": "brush", "strokes": strokes}}]}));
    let zigzag: Vec<Value> = (0..100_u32).map(|i| if i.is_multiple_of(2) { point(0.1, 0.1) } else { point(0.9, 0.9) }).collect();
    let painted = brush(vec![json!({"points": zigzag, "size": 0.5})]);
    let stroked = brush(vec![json!({"points": [point(0.2, 0.2)], "size": 0.03}); 257]);
    for (id, mask, code) in [(plain[0], painted, "budget_exceeded"), (plain[1], stroked, "out_of_range")] {
        // a person's brush work, made in the editor (another path than the hook), then captured
        committed(&mut lab, id, "Brush", |d| d.masks.push(serde_json::from_value(mask).expect("a mask")));
        let captured = lts(&mut lab.app, "lts.state", json!({"id": id.0, "settings": true}));
        let (theirs, settings) = (captured["hash"].clone(), captured["settings"].clone());
        // the photo leaves that state: a checkout of the same settings without the brush work
        let without = replaced(&settings, "/masks", json!([]));
        let left = lts(&mut lab.app, "lts.develop", json!({"id": id.0, "settings": without, "expectHash": theirs}));
        assert_eq!(left["outcome"], "applied", "{code}: leaving the person's state: {left}");
        settle(&mut lab);
        // a checkout back to it is held to the bounds: refused, with nothing changed
        refuses(&mut lab, "lts.develop", json!({"id": id.0, "settings": settings, "expectHash": left["hashAfter"]}), code);
        // lts.restoreSettings puts it back by its token, unjudged: the person's state exactly
        let restore = json!({"id": id.0, "revision": theirs, "expectHash": left["hashAfter"]});
        let restored = lts(&mut lab.app, "lts.restoreSettings", restore);
        assert_eq!((restored["outcome"].as_str(), &restored["hashAfter"]), (Some("applied"), &theirs), "{code}: {restored}");
        assert_eq!(settings_json(&lab.app, id), settings, "{code}: the person's state, exactly");
    }
}
