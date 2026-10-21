//! The LTS hook: the `lts.*` commands the LTS lab's adapter sends through `lightcraft.command`
//! (design Appendix A.2; plan §1 FL1; the newsroom pre-declaration R0 of master plan v2 §3.1; the
//! fork batch R1 of master plan v3 W3).
//!
//! The page's inbox (`web.rs`) hands every id to [`handle`] before `web.stats` and
//! `LightcraftApp::run`. The hook answers every id that starts with `lts.`, so these commands never
//! reach the command registry: they are never journaled and never write the status line. In the
//! LTS presentation (`?embedded`) it answers every other id too, with `command_not_allowed`, and
//! runs nothing (R1, backlog item 16): the LTS editor runs no command outside [`POLICY`], whoever
//! sends it, and its adapter and byte bridge read through the reads of the table. The standalone
//! presentation stays upstream's: there the page runs every other id as before. [`POLICY`] is the
//! whole table. Each command names its photo explicitly (`id`, an integer from 1; only `lts.state`
//! falls back to the active photo, and `lts.controls` describes the controls without one), so none
//! needs an active photo, and its parameters are a closed object (an unknown key is
//! `invalid_params`).
//!
//! - Reads answer `ok` and change nothing but the decoded-source cache (a render may keep the
//!   source it decoded): `lts.state` (the tokens, and with `settings: true` the complete settings
//!   they name), `lts.preview` (the engine's own render as a PNG without metadata, with the
//!   histogram and statistics of its pixels: the photo, a mask's view with the mask's coverage, or
//!   the Remove tool's Visualize Spots) and `lts.export` (a JPEG or PNG under an explicit metadata
//!   policy, written nowhere), and since R1 the reads that replace the engine's own queries, which
//!   the LTS presentation no longer runs: `lts.library` (the photos as `catalog.query` lists them,
//!   or those of one file name), `lts.photo` (`photo.inspect`, with the token of each step of the
//!   photo's history), `lts.controls` (the control catalog: every control, and the states in which
//!   it changes nothing), `lts.presets` (`presets.list`, `profiles.menu` and `curve.presets`),
//!   `lts.metadata` (`photo.allMetadata`) and `lts.journal` (`journal.list`). Each answers what the
//!   engine's query answers: the hook runs the query by its own spec, unjournaled and unsaved.
//! - Mutations answer `applied`: `lts.develop` (control values set exactly as `develop.set` sets
//!   them, complete settings, whole sections of the settings that no control value reaches, or a
//!   preset applied as `preset.apply` applies it), `lts.crop` (the geometry `crop.set` or
//!   `crop.reset` makes), `lts.undo` and `lts.redo` (the top step, when it is that photo's alone)
//!   and `lts.restoreSettings` (an earlier state that the photo's history recorded, put back as
//!   `history.restore` puts it); and `lts.select` (R1: the declared selection, the photo selected
//!   and active as `library.select` leaves it, shown in the detail view as `view.detail` shows it).
//!   Each one is one engine command (`Session::execute_fn`: never journaled), and each but
//!   `lts.select` one undo step. It refuses while a human interaction is in progress and verifies
//!   the caller's `expectHash` and expected head both before and inside that command. The writes
//!   that replace what the caller must have read first require `expectHash` (`invalid_params`
//!   without it): complete settings (a checkout), an undo or redo step and `lts.restoreSettings`.
//! - Refusals answer `not_applied` with a registered `code` and a `message`, before anything
//!   changed; a conflict also answers `actual`, the current token or head to refresh from.
//!
//! Every reply carries `resultVersion` ([`RESULT_VERSION`]) and `command` (the id). [`handle`]
//! answers `Err(text)` only when a mutation's engine command failed after it started: its change
//! may or may not be in effect, which the adapter reports as `unknown`.
//!
//! One revision token, the fork's (R1, backlog item 22): a photo's settings token is `hash64` of
//! its settings, 16 lowercase hex digits, and every reply that names a photo's settings names the
//! token of exactly those settings (`lts.state`, the rows of `lts.library`, `lts.photo` and each
//! step of its history, `lts.controls`, `lts.preview`, `lts.export`, `lts.select` and each
//! mutation's `hashBefore` and `hashAfter`), so that no caller computes a token of its own. The
//! token a read answers is the one `expectHash` takes, and a history step's is the `revision`
//! `lts.restoreSettings` puts back.
//!
//! Every value a change sets is held, before anything changes, to what the editor itself leaves:
//! control values to their ranges (`develop.controls`); and the settings that complete `settings`,
//! a `settings` patch or a `preset` would commit, on every part they change, to the bounds the
//! editor's own commands and tools keep (spot sizes, red-eye radii, a mask's own adjustments, brush
//! sizes, point coordinates, mask ids, the crop's size) and to counts (`out_of_range`).
//!
//! Every change, `values` and `lts.crop` included, is also held at the crop's magnification. A
//! tight crop spreads a small part of the photo over the whole output, so every size the settings
//! give as a fraction of the photo grows in the output. The hook reads the crop the renderer shows
//! (the settings' crop, shrunk by Constrain Crop where a transform leaves it blank) two ways: one
//! over its smaller side, at which sizes and work are held, and the renderer's own magnification
//! (an output's pixels per long-edge unit of the photo over the output's long edge), which follows
//! the crop's longer side in pixels, so that a long thin strip squared up is tighter although its
//! smaller side stays. Where the renderer solves the Upright transform from the pixels under
//! Constrain Crop (settings that name an automatic Upright without its transform, as an XMP sidecar
//! or a preset can), the hook, which decodes nothing, counts the crop shown as anything from 0.02
//! of the settings' crop to all of it. When a change makes the crop tighter by either measure, the
//! crop the renderer shows stays wider and taller than the crop tool keeps one, and the photo's own
//! spots and brushes are judged again. A heal or remove spot and an Auto Mask stroke may be no
//! larger in the output than the editor lets them be in the whole photo; no change may add or widen
//! a blur the renderer derives from the settings (their profile's look included, and since bug-fix
//! batch 2 sharpening's own band) past the widest the editor's own tools lead to, nor widen
//! defringe's filter past 64 pixels at the largest export, the photo's own blurs and filter taken
//! at the renderer's own magnification as the change's are (`out_of_range`). Brush and spot work
//! stays within the hook's budgets, counted in output planes (`budget_exceeded`). The budgets bound
//! an estimate of the pixels the renderer paints, measures and blurs; they are not a time limit,
//! and a render within them can still take a while. A part equal to the photo's own is not judged
//! again unless the change makes the crop tighter, so complete settings that keep what the photo
//! holds (a sync built from `lts.state {settings: true}` of both photos) pass whatever it holds. A
//! part that differs is judged, even one the photo held before: the editor's own tools cap no count
//! of brush strokes or spots, so complete settings that bring back a person's heavier work once the
//! photo has left it (a checkout) can be refused (`out_of_range`, `budget_exceeded`), while
//! `lts.restoreSettings` puts back, unjudged, any state the photo's history records (T-LI-28).
//!
//! A `settings` patch names its sections by the settings' own keys (`develop.get`): `masks`,
//! `spots`, `red_eye`, `curve`, `wb`, `profile` and `treatment`. The copy groups name three of them
//! otherwise (master plan v2 §3.1 writes the group name `redEye`): `redEye` is `red_eye`,
//! `toneCurve` is `curve` and `whiteBalance` is `wb` (`SettingsGroup::keys`); the group names are
//! refused.
//!
//! Presets and sync take no `lts.*` id of their own. A preset is `lts.develop {preset}` (and
//! `lts.preview` previews it), because no query answers a preset's settings (`presets.list`, which
//! `lts.presets` answers, gives ids and names). A sync is `lts.develop {settings}` with complete
//! settings that the adapter builds from the settings of both photos (`lts.state {settings: true}`,
//! which `develop.get` gave), the keys of every copied group (`SettingsGroup::keys`) taken from the
//! source, which is what `develop.sync` and `develop.paste` commit.
//!
//! One file serves both runtimes ([`Runtime`]): the browser hook (wasm32) and a native worker
//! (master plan v2 §3.5 NW-5), which hosts a headless `LightcraftApp` and calls [`handle_in`] with
//! its session. Each entry of [`POLICY`] says where it may run ([`Runtimes`]; none is native-only
//! today, and one that is would be refused in the browser as `native_only`, which [`handle_by`]
//! shows with a table of its own), and each runtime names its own render bounds
//! ([`Runtime::preview_max`], [`Runtime::export_max`]). Only the page's entry, [`handle`], has the
//! editor's view: through [`handle_in`] `lts.select` selects and shows no view (`view`: null).
//!
//! Hosted natively, the hook's edits are saved like every command's, in the session's library, and
//! the hook writes nothing else: no preference, no file and no XMP sidecar (the library's sidecar
//! auto-write is held off around its commands, so nothing is written beside an original, wherever
//! the original lives). The host keeps the library in its workspace by opening it with
//! `Session::open_library_in` over `FsStore`s on a folder of the workspace (or `MemStore`s),
//! `on_disk: false` (no thumbnail cache, no `Originals/` copies, no smart previews) and without the
//! demo photos; the hook runs on no library opened on disk (a desktop library: every command is
//! `command_not_allowed`), which the page never opens either (test T-LI-17).

use std::collections::BTreeSet;
use std::fmt::Arguments;
use std::sync::Arc;

use lightcraft_codecs::{EncodeImage, EncodeMeta, encode_png};
use lightcraft_engine::catalog::{Op, Photo, PhotoId};
use lightcraft_engine::develop::settings::{EmbeddedLens, Treatment, VignetteStyle};
use lightcraft_engine::develop::{
    ColorGrading, DevelopSettings, LocalAdjustments, MAX_POINT_COLORS, Mask, MaskShape, RedEye, Spot, SpotMode, ToneCurve, Upright, WbMode, controls,
};
use lightcraft_engine::export::{ExportFormat, ExportOptions, MetadataPolicy, Resize, prepare_batch};
use lightcraft_engine::pipeline::geometry::Frame;
use lightcraft_engine::pipeline::local::plane_sigmas;
use lightcraft_engine::pipeline::optics::DEFRINGE_REF_LONG;
use lightcraft_engine::pipeline::{MaskView, Overlay, Quality, lut, profiles};
use lightcraft_engine::segment::MAX_CLICKS;
use lightcraft_engine::{EngineError, RenderJob, Selection, Session, UndoEntry, find_command, guard};
use lightcraft_raster::{Histogram, Rgba8};
use lightcraft_ui_egui::LightcraftApp;
use lightcraft_ui_egui::presentation::Presentation;
use lightcraft_ui_egui::state::ViewMode;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};

/// The version of every reply: the adapter accepts exactly this one.
pub const RESULT_VERSION: &str = "lightcraft.lts-view.v1";

/// The smallest long edge of a preview or an export, in pixels.
const MIN_EDGE: u64 = 16;
/// The largest export long edge, in pixels, in both runtimes ([`Runtime::export_max`]): the largest
/// output the hook renders, at which it holds the renderer's blurs ([`SIGMA_MAX`]).
const EXPORT_MAX: u32 = 4096;
/// The most controls one `values` object sets.
const MAX_VALUES: usize = 64;
/// The longest undo label, in characters.
const MAX_LABEL: usize = 200;
/// A crop rectangle's coordinates: normalized, 0 to 1 across the photo (the straightened frame
/// reaches a little past it). Every point a change places is held to the same range.
const RECT_MIN: f64 = -1.0;
const RECT_MAX: f64 = 2.0;
/// The smallest crop, a fraction of the photo each way: the editor's crop tool keeps a crop wider
/// and taller than this (`crates/ui-egui/src/panels/detail.rs:957`; `crop.set` keeps none). The
/// crop the renderer shows is held to it too when a change makes the crop tighter ([`magnified`]),
/// so the hook never makes one over the crop's smaller side 50 or more.
const MIN_CROP: f64 = 0.02;
/// The straighten angle's bound in degrees, the range `crop.set` keeps it in.
const MAX_ANGLE: f64 = 45.0;
/// The colour views draw a mask in the editor's default overlay colour and opacity.
const OVERLAY_COLOR: [u8; 3] = [230, 30, 40];
const OVERLAY_OPACITY: u8 = 50;
/// The deepest nesting the JSON comparison and the undo-step walk follow.
const MAX_DEPTH: usize = 32;
/// The most characters of caller or engine text a message repeats.
const ECHO: usize = 120;
/// The sections a `settings` patch sets, by their keys in the settings' own serialization
/// (`develop.get`, `crates/engine/src/cmd/query.rs:143`; the fields of `DevelopSettings`,
/// `crates/develop/src/settings.rs:11-41`): the ones no control value reaches, each set whole and
/// then held to the bounds the engine's own command for it keeps ([`bounded`]). `masks`
/// (`mask.add`, `mask.update {shape: MaskShape JSON}`, `cmd/masks.rs:156-186, 274`), `spots`
/// (`spot.add`, `spot.update`, `cmd/masks.rs:692-697, 741-746`), `red_eye` (`redeye.add`,
/// `cmd/masks.rs:821-826`; the copy group `redEye` keeps this key, `crates/develop/src/presets.rs:115`),
/// `curve` (`develop.curve {channel, points}` in 0..1 with the region sliders,
/// `cmd/develop.rs:359-378`), `wb` (`develop.wb {mode, temp?, tint?}`, `cmd/develop.rs:258-296`),
/// `profile` (`develop.profile {id, amount?}`, `cmd/develop.rs:332-344`) and `treatment`
/// (`develop.treatment {bw?}`, `cmd/develop.rs:325-331`). Sliders go through `values`, the crop
/// through `lts.crop`, and every other section (geometry with its computed Upright transform,
/// orientation, optics, enhance, lens blur, the section toggles) stays out of a patch's reach.
const PATCH_SECTIONS: [&str; 7] = ["masks", "spots", "red_eye", "curve", "wb", "profile", "treatment"];
/// The largest preset amount, in percent (`preset.apply {id, amount?: 0..200, ids?}`,
/// `cmd/develop.rs:754-766`).
const MAX_PRESET_AMOUNT: f64 = 200.0;
/// The most rows a page of `lts.library` holds, and how many it holds by default: `catalog.query`'s
/// own default (`crates/engine/src/cmd/query.rs:66`).
const MAX_PAGE: u64 = 200;
/// The longest file name `lts.library` looks for, in characters.
const MAX_FILE_NAME: usize = 255;
/// The most entries `lts.journal` answers, the most the engine keeps
/// (`crates/engine/src/lib.rs:315-318`), and how many it answers by default, `journal.list`'s own
/// default (`cmd/query.rs:213-217`).
const MAX_JOURNAL: u64 = 10_000;
const JOURNAL_DEFAULT: u64 = 100;

// ------------------------------------------------------------------------------- the policy table

/// Where a command of [`POLICY`] may run (as Film's `agent_view_policy.rs` marks its entries).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Runtimes {
    /// In the browser hook and in a native worker.
    Both,
    /// In a native worker only: work that would hold the page's only thread for too long (no
    /// command of the hook today).
    NativeOnly,
}

/// Where this hook runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Runtime {
    /// The page's `lightcraft.command` (wasm32).
    Browser,
    /// A native worker that hosts a headless `LightcraftApp` (and the hook's native tests).
    Native,
}

impl Runtime {
    /// The runtime this build of the file serves: the browser hook on wasm32, a native worker on
    /// every other target.
    pub const HERE: Runtime = if cfg!(target_arch = "wasm32") { Runtime::Browser } else { Runtime::Native };

    /// The largest preview long edge here, in pixels: a larger one is `budget_exceeded`, refused
    /// before anything is decoded or allocated.
    pub const fn preview_max(self) -> u32 {
        match self {
            Runtime::Browser => 2048,
            Runtime::Native => 2048,
        }
    }

    /// The largest export long edge here, in pixels (`adjust_export {longEdge ≤ 4096}`, design
    /// §4.5).
    pub const fn export_max(self) -> u32 {
        match self {
            Runtime::Browser => EXPORT_MAX,
            Runtime::Native => EXPORT_MAX,
        }
    }
}

/// One command of the hook.
#[derive(Debug)]
pub struct Entry {
    /// The command id, `lts.` included.
    pub command: &'static str,
    /// Its parameter keys, exactly (case-sensitive): any other key is `invalid_params`.
    pub keys: &'static [&'static str],
    /// Whether it changes the library (answered `applied`) rather than reads it (answered `ok`).
    pub mutation: bool,
    /// Where it may run.
    pub runtimes: Runtimes,
}

impl Entry {
    /// Whether the entry may run in `runtime`.
    pub fn runs_in(&self, runtime: Runtime) -> bool {
        self.runtimes == Runtimes::Both || runtime == Runtime::Native
    }
}

/// A command that reads, in both runtimes.
const fn reading(command: &'static str, keys: &'static [&'static str]) -> Entry {
    Entry { command, keys, mutation: false, runtimes: Runtimes::Both }
}

/// A command that changes the library, in both runtimes.
const fn mutating(command: &'static str, keys: &'static [&'static str]) -> Entry {
    Entry { command, keys, mutation: true, runtimes: Runtimes::Both }
}

/// The hook's commands: nothing else under `lts.` is reachable (`command_not_allowed`), and in the
/// LTS presentation nothing else at all ([`handle`]). The reads come first: the tokens, the render
/// and the export, then the reads that replace the engine's queries (R1); the mutations after them,
/// the declared selection last.
pub static POLICY: &[Entry] = &[
    reading("lts.state", &["id", "settings"]),
    reading("lts.preview", &["id", "longEdge", "variant", "values", "settings", "preset", "crop", "overlay", "pixels"]),
    reading("lts.export", &["id", "format", "longEdge", "quality", "metadata", "removeLocation"]),
    reading("lts.library", &["offset", "limit", "fileName"]),
    reading("lts.photo", &["id"]),
    reading("lts.controls", &["id"]),
    reading("lts.presets", &[]),
    reading("lts.metadata", &["id"]),
    reading("lts.journal", &["limit"]),
    mutating("lts.develop", &["id", "values", "settings", "preset", "expectHash", "expectHistoryHead", "label"]),
    mutating("lts.crop", &["id", "rect", "angle", "reset", "expectHash", "expectHistoryHead"]),
    mutating("lts.undo", &["id", "expectHash", "expectHistoryHead"]),
    mutating("lts.redo", &["id", "expectHash", "expectRedoHead"]),
    mutating("lts.restoreSettings", &["id", "revision", "expectHash", "expectHistoryHead", "label"]),
    mutating("lts.select", &["id", "detail"]),
];

/// The editor's view, which `lts.select {detail: true}` shows: the page's, or none for a session
/// without the editor's UI (a native worker's, and the hook's own entries).
type View<'a> = Option<&'a mut ViewMode>;

/// Runs the command `id` with `params` as the page's inbox hands it over (`web.rs`), on the app's
/// session and with its view, in this build's runtime ([`Runtime::HERE`]).
///
/// For an `lts.` id, `Some(Ok(reply))` for every typed outcome, and `Some(Err(text))` only when a
/// mutation's engine command failed after it started. For any other id, in the LTS presentation
/// (`?embedded`), `Some(Ok(refusal))`, `command_not_allowed` whatever the parameters, with nothing
/// run (R1: the fork's entry point refuses every id outside the hook's table, backlog item 16); in
/// the standalone presentation `None`: the inbox runs it as before, and nothing ran here.
pub fn handle(app: &mut LightcraftApp, id: &str, params: &Value) -> Option<Result<Value, String>> {
    if !id.starts_with("lts.") {
        let embedded = app.presentation() == Presentation::LtsEmbedded;
        return embedded.then(|| Reply::Refused(outside_the_hook()).into_result(id));
    }
    handle_at(POLICY, &mut app.session, Some(&mut app.ui.view), id, params, Runtime::HERE)
}

/// [`handle`]'s `lts.*` commands on session `s` in `runtime`, without the editor's view: what a
/// native worker calls with the session of its headless `LightcraftApp`, and what the tests drive
/// each runtime through. `None` for an id that does not start with `lts.`.
pub fn handle_in(s: &mut Session, id: &str, params: &Value, runtime: Runtime) -> Option<Result<Value, String>> {
    handle_at(POLICY, s, None, id, params, runtime)
}

/// [`handle_in`] by the command table `table` in place of [`POLICY`], its entries run as the hook
/// runs its own: how the tests drive an entry that only a native worker runs (`native_only` in the
/// browser), which [`POLICY`] does not hold today.
pub fn handle_by(table: &[Entry], s: &mut Session, id: &str, params: &Value, runtime: Runtime) -> Option<Result<Value, String>> {
    handle_at(table, s, None, id, params, runtime)
}

/// The `lts.*` command `id` by `table`, on session `s` with the editor's view `view`.
fn handle_at(table: &[Entry], s: &mut Session, view: View<'_>, id: &str, params: &Value, runtime: Runtime) -> Option<Result<Value, String>> {
    if !id.starts_with("lts.") {
        return None;
    }
    let reply = match table.iter().find(|entry| entry.command == id) {
        None => Reply::Refused(not_a_command()),
        Some(entry) if !entry.runs_in(runtime) => {
            Reply::Refused(Refusal::new("native_only", format!("{id} runs in a native worker only, not in the browser")))
        }
        Some(_) if s.library.as_ref().is_some_and(|library| library.on_disk) => Reply::Refused(Refusal::new(
            "command_not_allowed",
            "the hook runs on a library kept in the page's storage or in a host's workspace (on_disk: false), or on none; never on a library on disk",
        )),
        Some(entry) => match Params::of(params, entry.keys) {
            Ok(q) => run(s, view, entry.command, &q, runtime),
            Err(refusal) => Reply::Refused(refusal),
        },
    };
    Some(reply.into_result(id))
}

/// Runs command `command` of [`POLICY`] with its parameters, `view` being the editor's view.
fn run(s: &mut Session, view: View<'_>, command: &str, q: &Params<'_>, runtime: Runtime) -> Reply {
    match command {
        "lts.state" => Reply::answered(state(s, q)),
        "lts.preview" => Reply::answered(preview(s, q, runtime)),
        "lts.export" => Reply::answered(export(s, q, runtime)),
        "lts.library" => Reply::answered(library(s, q)),
        "lts.photo" => Reply::answered(photo_read(s, q)),
        "lts.controls" => Reply::answered(controls_read(s, q)),
        "lts.presets" => Reply::answered(presets_read(s)),
        "lts.metadata" => Reply::answered(metadata_read(s, q)),
        "lts.journal" => Reply::answered(journal_read(s, q)),
        "lts.develop" => develop(s, q),
        "lts.crop" => crop(s, q),
        "lts.undo" => step(s, q, Stack::Undo),
        "lts.redo" => step(s, q, Stack::Redo),
        "lts.restoreSettings" => restore(s, q),
        "lts.select" => select(s, view, q),
        // every entry of POLICY has its arm above (T-LI-12 reaches each one)
        _ => Reply::Refused(not_a_command()),
    }
}

/// The refusal of an `lts.` id that is not in [`POLICY`].
fn not_a_command() -> Refusal {
    let commands: Vec<&str> = POLICY.iter().map(|entry| entry.command).collect();
    Refusal::new("command_not_allowed", format!("not a command of this hook: {}", commands.join(", ")))
}

/// The refusal of an id outside `lts.` in the LTS presentation, which runs none (R1).
fn outside_the_hook() -> Refusal {
    let commands: Vec<&str> = POLICY.iter().map(|entry| entry.command).collect();
    Refusal::new("command_not_allowed", format!("the LTS editor runs only its hook's commands: {}", commands.join(", ")))
}

// ------------------------------------------------------------------------------------- the replies

/// A typed refusal: nothing changed.
struct Refusal {
    code: &'static str,
    message: String,
    /// The current token or head to refresh from (conflicts).
    actual: Option<Value>,
}

impl Refusal {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Refusal { code, message: message.into(), actual: None }
    }

    fn conflict(code: &'static str, message: impl Into<String>, actual: Value) -> Self {
        Refusal { code, message: message.into(), actual: Some(actual) }
    }

    /// The refusal, its message followed by `note`.
    fn noting(mut self, note: &str) -> Self {
        self.message.push_str(note);
        self
    }
}

fn invalid(message: impl Into<String>) -> Refusal {
    Refusal::new("invalid_params", message)
}

fn not_found(id: PhotoId) -> Refusal {
    Refusal::new("target_not_found", format!("there is no photo {} in the library", id.0))
}

/// What a command answers.
enum Reply {
    /// A read answered.
    Read(Map<String, Value>),
    /// A mutation committed.
    Applied(Map<String, Value>),
    Refused(Refusal),
    /// A mutation's engine command failed after it started.
    Failed(String),
}

impl Reply {
    fn answered(read: Result<Map<String, Value>, Refusal>) -> Self {
        match read {
            Ok(fields) => Reply::Read(fields),
            Err(refusal) => Reply::Refused(refusal),
        }
    }

    fn into_result(self, id: &str) -> Result<Value, String> {
        let (outcome, mut fields) = match self {
            Reply::Read(fields) => ("ok", fields),
            Reply::Applied(fields) => ("applied", fields),
            Reply::Refused(Refusal { code, message, actual }) => {
                let mut fields = object(json!({"code": code, "message": message}));
                if let Some(actual) = actual {
                    fields.insert("actual".into(), actual);
                }
                ("not_applied", fields)
            }
            Reply::Failed(error) => return Err(format!("{id}: {error}")),
        };
        fields.insert("resultVersion".into(), json!(RESULT_VERSION));
        fields.insert("command".into(), json!(id));
        fields.insert("outcome".into(), json!(outcome));
        Ok(Value::Object(fields))
    }
}

/// The fields of a JSON object.
fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(fields) => fields,
        _ => Map::new(),
    }
}

/// `text` (caller or engine text a message repeats), cut to [`ECHO`] characters.
fn clip(text: &str) -> String {
    let mut chars = text.chars();
    let mut out: String = chars.by_ref().take(ECHO).collect();
    if chars.next().is_some() {
        out.push('…');
    }
    out
}

/// A settings token: `hash64` as 16 lowercase hex digits.
fn hex(hash: u64) -> String {
    format!("{hash:016x}")
}

/// `{length, label}` of an undo or redo stack (`label`: its top step's, null when it is empty).
fn head_json(stack: &[UndoEntry]) -> Value {
    json!({"length": stack.len(), "label": stack.last().map(|step| step.label.as_str())})
}

/// The settings of photo `id`, a photo of the library (not one in Recently Deleted).
fn settings_of(s: &Session, id: PhotoId) -> Result<Arc<DevelopSettings>, Refusal> {
    s.catalog.photo(id).filter(|p| p.in_library()).map(|p| Arc::clone(&p.develop)).ok_or_else(|| not_found(id))
}

// -------------------------------------------------------------------------------- the parameters

/// A command's parameters: a JSON object with no key but the command's own.
struct Params<'a>(&'a Map<String, Value>);

impl<'a> Params<'a> {
    fn of(params: &'a Value, keys: &[&str]) -> Result<Self, Refusal> {
        let fields = params.as_object().ok_or_else(|| invalid("the parameters are a JSON object"))?;
        match fields.keys().find(|key| !keys.contains(&key.as_str())) {
            Some(key) => Err(invalid(format!("unknown parameter `{}`", clip(key)))),
            None => Ok(Params(fields)),
        }
    }

    fn get(&self, key: &str) -> Option<&'a Value> {
        self.0.get(key)
    }

    /// `id`, when given: an explicit photo id, an integer from 1.
    fn photo(&self) -> Result<Option<PhotoId>, Refusal> {
        match self.get("id") {
            None => Ok(None),
            Some(id) => id.as_u64().filter(|id| *id >= 1).map(|id| Some(PhotoId(id))).ok_or_else(|| invalid("`id` is a photo id: an integer from 1")),
        }
    }

    /// `id`, required: the photo to act on, never the active photo by default.
    fn target(&self) -> Result<PhotoId, Refusal> {
        self.photo()?.ok_or_else(|| invalid("`id` is required: the photo to act on (never the active photo by default)"))
    }

    /// A string that must be one of `choices`, when given.
    fn choice(&self, key: &str, choices: &[&'static str]) -> Result<Option<&'static str>, Refusal> {
        match self.get(key) {
            None => Ok(None),
            Some(value) => value
                .as_str()
                .and_then(|text| choices.iter().copied().find(|choice| *choice == text))
                .map(Some)
                .ok_or_else(|| invalid(format!("`{key}` is one of: {}", choices.join(", ")))),
        }
    }

    /// A boolean, `default` when absent.
    fn flag(&self, key: &str, default: bool) -> Result<bool, Refusal> {
        match self.get(key) {
            None => Ok(default),
            Some(value) => value.as_bool().ok_or_else(|| invalid(format!("`{key}` is true or false"))),
        }
    }

    /// An integer from `min` to `max`, `default` when absent.
    fn integer(&self, key: &str, (min, max): (u64, u64), default: u64) -> Result<u64, Refusal> {
        let refused = || invalid(format!("`{key}` is an integer from {min} to {max}"));
        match self.get(key) {
            None => Ok(default),
            Some(value) => value.as_u64().filter(|n| (min..=max).contains(n)).ok_or_else(refused),
        }
    }

    /// `longEdge`, required: an integer from 16. Its upper bound is checked by [`within`] once every
    /// other parameter is known to be well formed.
    fn long_edge(&self) -> Result<u64, Refusal> {
        self.get("longEdge")
            .and_then(Value::as_u64)
            .filter(|long| *long >= MIN_EDGE)
            .ok_or_else(|| invalid(format!("`longEdge` is required: an integer from {MIN_EDGE} (pixels)")))
    }

    /// `values`, `settings` or `preset`, when one of them is given (never two).
    fn change(&self) -> Result<Option<Change>, Refusal> {
        match (self.get("values"), self.get("settings"), self.get("preset")) {
            (None, None, None) => Ok(None),
            (Some(values), None, None) => Ok(Some(Change::Values(control_values(values)?))),
            (None, Some(settings), None) => Ok(Some(settings_change(settings)?)),
            (None, None, Some(preset)) => Ok(Some(preset_change(preset)?)),
            _ => Err(invalid("give one of `values`, `settings` or `preset`")),
        }
    }

    /// `expectHash` and the expected head `head_key`, each when given.
    fn guards(&self, head_key: &str) -> Result<Guards, Refusal> {
        let hash = self.get("expectHash").map(|token| parse_token(token, "expectHash")).transpose()?;
        let head = self.get(head_key).map(|head| parse_head(head, head_key)).transpose()?;
        Ok(Guards { hash, head })
    }

    /// `label`, when given: the undo step's name, 1 to 200 characters without control characters.
    fn label(&self) -> Result<Option<String>, Refusal> {
        match self.get("label") {
            None => Ok(None),
            Some(label) => label
                .as_str()
                .filter(|text| !text.trim().is_empty() && text.chars().count() <= MAX_LABEL && !text.chars().any(char::is_control))
                .map(|text| Some(text.to_string()))
                .ok_or_else(|| invalid(format!("`label` is 1 to {MAX_LABEL} characters"))),
        }
    }
}

/// `long` when it is at most `max` pixels.
fn within(long: u64, max: u32) -> Result<u32, Refusal> {
    u32::try_from(long).ok().filter(|long| *long <= max).ok_or_else(|| Refusal::new("budget_exceeded", format!("`longEdge` is at most {max} pixels")))
}

/// A settings token parameter: 16 lowercase hex digits.
fn parse_token(value: &Value, key: &str) -> Result<u64, Refusal> {
    value
        .as_str()
        .filter(|text| text.len() == 16 && text.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
        .and_then(|text| u64::from_str_radix(text, 16).ok())
        .ok_or_else(|| invalid(format!("`{key}` is a settings token: 16 lowercase hex digits (lts.state)")))
}

/// The expected top of an undo or redo stack: its length and its top step's label (`None` when it
/// is empty).
struct Head {
    length: u64,
    label: Option<String>,
}

impl Head {
    fn matches(&self, stack: &[UndoEntry]) -> bool {
        usize::try_from(self.length).is_ok_and(|length| length == stack.len())
            && self.label.as_deref() == stack.last().map(|step| step.label.as_str())
    }
}

/// An expected head parameter: `{length, label}`, `label` null for an empty stack.
fn parse_head(value: &Value, key: &str) -> Result<Head, Refusal> {
    let refused = || invalid(format!("`{key}` is {{length, label}}: the stack's length and its top step's label, null when empty (lts.state)"));
    let fields = value.as_object().filter(|fields| fields.len() == 2).ok_or_else(refused)?;
    let length = fields.get("length").and_then(Value::as_u64).ok_or_else(refused)?;
    let label = match fields.get("label") {
        Some(Value::Null) => None,
        Some(Value::String(label)) if label.chars().count() <= MAX_LABEL => Some(label.clone()),
        _ => return Err(refused()),
    };
    Ok(Head { length, label })
}

/// What a mutation's caller expects to find.
struct Guards {
    /// `expectHash`: the photo's settings token.
    hash: Option<u64>,
    /// `expectHistoryHead` (`expectRedoHead` for `lts.redo`).
    head: Option<Head>,
}

/// A develop change: control values set as `develop.set` sets them, complete settings, whole
/// sections of [`PATCH_SECTIONS`] (the patch, read in full by [`with_sections`]), or a preset (its
/// id and amount in percent) applied as `preset.apply` applies it.
enum Change {
    Values(Vec<(String, f64)>),
    Settings(Box<DevelopSettings>),
    Sections(Map<String, Value>),
    Preset { id: String, amount: f64 },
}

/// `values`: 1 to 64 develop controls by id (`develop.controls`), each with a number within the
/// control's range (outside it is `out_of_range`).
fn control_values(value: &Value) -> Result<Vec<(String, f64)>, Refusal> {
    let fields = value
        .as_object()
        .filter(|fields| (1..=MAX_VALUES).contains(&fields.len()))
        .ok_or_else(|| invalid(format!("`values` is an object of 1 to {MAX_VALUES} control ids (develop.controls) and numbers")))?;
    let mut values = Vec::with_capacity(fields.len());
    for (key, value) in fields {
        let spec = controls::find(key).ok_or_else(|| invalid(format!("unknown control `{}` (develop.controls)", clip(key))))?;
        let number = value.as_f64().ok_or_else(|| invalid(format!("`{key}` is a number")))?;
        if !(spec.min..=spec.max).contains(&number) {
            return Err(Refusal::new("out_of_range", format!("`{key}` is {} to {}, not {number}", spec.min, spec.max)));
        }
        values.push((key.clone(), number));
    }
    Ok(values)
}

/// `settings`: complete develop settings JSON, as `develop.get` or a checkpoint gives them. They are
/// read exactly, with the very keys of the settings' own serialization at every level, so that a
/// partial object never resets what it leaves out.
fn complete_settings(value: &Value) -> Result<DevelopSettings, Refusal> {
    let refused = || invalid("`settings` is complete develop settings JSON, as develop.get gives them");
    if !value.is_object() {
        return Err(refused());
    }
    let settings = DevelopSettings::from_json(value).map_err(|e| invalid(format!("`settings`: {}", clip(&e.to_string()))))?;
    if same_json(value, &settings.to_json(), 0) { Ok(settings) } else { Err(refused()) }
}

/// Whether `given` says exactly what `canonical` does: the same keys at every level and equal values,
/// numbers compared by value (`20` is `20.0`, as JavaScript writes it).
fn same_json(given: &Value, canonical: &Value, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    match (given, canonical) {
        (Value::Object(a), Value::Object(b)) => a.len() == b.len() && a.iter().all(|(key, v)| b.get(key).is_some_and(|w| same_json(v, w, depth + 1))),
        (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(v, w)| same_json(v, w, depth + 1)),
        (Value::Number(a), Value::Number(b)) => a == b || a.as_f64() == b.as_f64(),
        _ => given == canonical,
    }
}

/// `settings`: complete develop settings (every key `develop.get` gives: a checkpoint), or a patch
/// of 1 to 7 whole sections of [`PATCH_SECTIONS`], which [`with_sections`] reads once the photo is
/// known.
fn settings_change(value: &Value) -> Result<Change, Refusal> {
    let fields = value.as_object().ok_or_else(|| invalid("`settings` is complete develop settings JSON (develop.get) or a patch of sections"))?;
    let complete = DevelopSettings::default().to_json();
    if complete.as_object().is_some_and(|keys| keys.len() == fields.len() && keys.keys().all(|key| fields.contains_key(key))) {
        return Ok(Change::Settings(Box::new(complete_settings(value)?)));
    }
    if let Some(key) = fields.keys().find(|key| !PATCH_SECTIONS.contains(&key.as_str())) {
        return Err(not_a_section(key));
    }
    if fields.is_empty() {
        return Err(invalid(format!("a `settings` patch sets at least one section of {}", PATCH_SECTIONS.join(", "))));
    }
    Ok(Change::Sections(fields.clone()))
}

/// A `settings` key that a patch does not set.
fn not_a_section(key: &str) -> Refusal {
    let (key, sections) = (clip(key), PATCH_SECTIONS.join(", "));
    let others = "values set the sliders, lts.crop the crop; complete settings have every key develop.get gives";
    invalid(format!("`settings.{key}` is not a section a patch sets: a patch sets whole sections of {sections} ({others})"))
}

/// `before` with the sections of `patch` replaced, each by exactly what it gives: read as the
/// section's own type and written back as the settings' serialization writes it, so that a partial
/// or misspelt section is refused rather than filled with defaults. [`bounded`] then holds the
/// values to the editor's own.
fn with_sections(before: &DevelopSettings, patch: &Map<String, Value>) -> Result<DevelopSettings, Refusal> {
    let mut after = before.clone();
    for (key, value) in patch {
        let written = match key.as_str() {
            "masks" => replace_section(&mut after.masks, key, value)?,
            "spots" => replace_section(&mut after.spots, key, value)?,
            "red_eye" => replace_section(&mut after.red_eye, key, value)?,
            "curve" => replace_section(&mut after.curve, key, value)?,
            "wb" => replace_section(&mut after.wb, key, value)?,
            "profile" => replace_section(&mut after.profile, key, value)?,
            "treatment" => replace_section(&mut after.treatment, key, value)?,
            _ => return Err(not_a_section(key)),
        };
        if !same_json(value, &written, 0) {
            return Err(invalid(format!("`settings.{key}` is the whole section, exactly as develop.get gives it")));
        }
    }
    Ok(after)
}

/// Sets `section` to `value` read as the section's own type, and answers it as the settings'
/// serialization writes it.
fn replace_section<T: Serialize + DeserializeOwned>(section: &mut T, key: &str, value: &Value) -> Result<Value, Refusal> {
    let read: T = serde_json::from_value(value.clone()).map_err(|e| section_error(key, &e))?;
    let written = serde_json::to_value(&read).map_err(|e| section_error(key, &e))?;
    *section = read;
    Ok(written)
}

/// A section that does not read as its type.
fn section_error(key: &str, error: &serde_json::Error) -> Refusal {
    invalid(format!("`settings.{key}`: {}", clip(&error.to_string())))
}

/// Whether the editor has profile `id` (a built-in one, `profiles.list`, or an imported LUT), as
/// `develop.profile` requires.
fn known_profile(s: &Session, id: &str) -> bool {
    lightcraft_engine::presets::profile(id).is_some() || s.lut_profiles.iter().any(|lut| lut.id == id)
}

/// `preset`: `{id, amount?}`, the keys of `preset.apply`: a preset (`presets.list`) and how much of
/// it in percent, 0 to 200 (100 by default; outside is `out_of_range`).
fn preset_change(value: &Value) -> Result<Change, Refusal> {
    let q = Params::of(value, &["id", "amount"]).map_err(|_| invalid("`preset` is {id, amount?}: a preset id (presets.list) and a percentage"))?;
    let id = q.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()).ok_or_else(|| invalid("`preset.id` is a preset id (presets.list)"))?;
    let amount = match q.get("amount") {
        None => 100.0,
        Some(amount) => amount.as_f64().ok_or_else(|| invalid("`preset.amount` is a number of percent"))?,
    };
    if !(0.0..=MAX_PRESET_AMOUNT).contains(&amount) {
        return Err(Refusal::new("out_of_range", format!("`preset.amount` is 0 to {MAX_PRESET_AMOUNT} percent, not {amount}")));
    }
    Ok(Change::Preset { id: id.to_string(), amount })
}

/// Each control whose value `after` changes, with its value before and after.
fn control_delta(before: &DevelopSettings, after: &DevelopSettings) -> Map<String, Value> {
    controls::CONTROLS
        .iter()
        .map(|spec| (spec.id, controls::get(before, spec.id), controls::get(after, spec.id)))
        .filter(|(_, old, new)| old != new)
        .map(|(control, old, new)| (control.to_string(), json!([old, new])))
        .collect()
}

/// `settings` with `values` set as `develop.set` sets them: in As Shot white balance a temperature or
/// tint starts from the photo's as-shot pair, and setting either one switches the mode to Custom.
fn with_values(settings: &DevelopSettings, values: &[(String, f64)], as_shot: (f64, f64)) -> DevelopSettings {
    let mut settings = settings.clone();
    if settings.wb.mode == WbMode::AsShot && values.iter().any(|(key, _)| is_white_balance(key)) {
        (settings.wb.temp, settings.wb.tint) = as_shot;
    }
    for (key, value) in values {
        controls::set(&mut settings, key, *value);
        if is_white_balance(key) {
            settings.wb.mode = WbMode::Custom;
        }
    }
    settings
}

/// The white balance controls, which switch the mode to Custom.
fn is_white_balance(key: &str) -> bool {
    key == "wb.temp" || key == "wb.tint"
}

/// The undo label `develop.set` gives: the control's name for one value, `Edit` for several.
fn values_label(values: &[(String, f64)]) -> String {
    match values {
        [(key, _)] => controls::find(key).map_or("Edit", |spec| spec.label).to_string(),
        _ => "Edit".to_string(),
    }
}

/// What `change` makes of photo `id`'s settings `before`: the settings, the undo label the engine
/// would give the step (`develop.set`'s for values, `Edit` for complete settings and sections as
/// `develop.merge` names them, `Preset: <name>` as `preset.apply` does) and what the reply reports:
/// each value before and after (`delta`), whether every value is as asked, whether the white
/// balance mode changed and, for a patch, the sections it set (`sections`). Complete settings,
/// sections and presets are held to [`bounded`] (values are already within their controls' ranges),
/// and every change to [`magnified`].
fn apply(s: &Session, id: PhotoId, before: &DevelopSettings, change: Change) -> Result<(DevelopSettings, String, Map<String, Value>), Refusal> {
    let mut sections = None;
    let (after, label, delta, satisfied) = match change {
        Change::Values(values) => {
            // a list control (`pointColor.3.hueShift`) exists only on the photos that have the element
            if let Some((key, _)) = values.iter().find(|(key, _)| controls::get(before, key).is_none()) {
                return Err(invalid(format!("photo {} has no `{key}`", id.0)));
            }
            let info = s.source_info(id);
            let after = with_values(before, &values, (info.as_shot_temp, info.as_shot_tint));
            let delta: Map<String, Value> =
                values.iter().map(|(key, _)| (key.clone(), json!([controls::get(before, key), controls::get(&after, key)]))).collect();
            let satisfied = values.iter().all(|(key, value)| controls::get(&after, key) == Some(*value));
            (after, values_label(&values), delta, satisfied)
        }
        Change::Settings(settings) => {
            let after = *settings;
            bounded(s, before, &after)?;
            let delta = control_delta(before, &after);
            (after, "Edit".to_string(), delta, true)
        }
        Change::Sections(patch) => {
            let after = with_sections(before, &patch)?;
            bounded(s, before, &after)?;
            let delta = control_delta(before, &after);
            sections = Some(PATCH_SECTIONS.iter().copied().filter(|key| patch.contains_key(*key)).collect::<Vec<_>>());
            (after, "Edit".to_string(), delta, true)
        }
        Change::Preset { id: preset_id, amount } => {
            let preset = s
                .presets
                .iter()
                .find(|preset| preset.id == preset_id)
                .ok_or_else(|| invalid(format!("there is no preset `{}` (presets.list)", clip(&preset_id))))?;
            // `Preset::apply` numbers the preset's masks after the photo's highest id
            let adds_masks = preset.settings.get("masks").and_then(Value::as_array).is_some_and(|masks| !masks.is_empty());
            if adds_masks && let Some(mask) = before.masks.iter().find(|mask| mask.id > MASK_ID_MAX) {
                let message = format!(
                    "photo {}'s mask {} is past the highest mask id, {MASK_ID_MAX}: the preset's masks would be numbered after it",
                    id.0, mask.id
                );
                return Err(Refusal::new("out_of_range", message));
            }
            let after = preset.apply(before, amount / 100.0);
            bounded(s, before, &after)?;
            let delta = control_delta(before, &after);
            (after, format!("Preset: {}", preset.name), delta, true)
        }
    };
    magnified(s, id, before, &after)?;
    let wb_mode_changed = before.wb.mode != after.wb.mode;
    let mut fields = object(json!({"delta": delta, "intentSatisfied": satisfied, "wbModeChanged": wb_mode_changed}));
    if let Some(sections) = sections {
        fields.insert("sections".into(), json!(sections));
    }
    Ok((after, label, fields))
}

// ----------------------------------------------------------------------- the values a change sets

/// Every point a change places (spot, brush and eye positions, gradient ends, Upright guides):
/// normalized, within the crop rectangle's range.
const POINTS: (f64, f64) = (RECT_MIN, RECT_MAX);
/// A spot's radius as a fraction of the long edge, as `spot.add` and `spot.update` clamp it
/// (`SPOT_SIZE`, `crates/engine/src/cmd/masks.rs:714, 769, 887`), and its source offset: from one
/// point a change may place to another.
const SPOT_SIZE: (f64, f64) = (0.001, 0.25);
const SPOT_OFFSET: (f64, f64) = (-3.0, 3.0);
/// A brush's radius as a fraction of the long edge, as the editor's brush keeps it
/// (`crates/ui-egui/src/state.rs:673`, `menus.rs:215`; `mask.brushStroke` keeps none).
const BRUSH_SIZE: (f64, f64) = (0.002, 0.5);
/// A red-eye correction's radii, as `redeye.add` clamps them, and its catchlight's offset in pupil
/// radii, as `redeye.catchlight` clamps it (`crates/engine/src/cmd/masks.rs:834-835, 866`).
const EYE_RADIUS: (f64, f64) = (1e-4, 0.5);
const CATCHLIGHT: (f64, f64) = (-1.0, 1.0);
/// Feathers, flows, densities, opacities and a mask's edge refinement, in percent (`spot.add` and
/// `mask.refine` clamp theirs, `crates/engine/src/cmd/masks.rs:487, 715-716`; the renderer clamps
/// the others).
const PERCENT: (f64, f64) = (0.0, 100.0);
/// A radial gradient's radii, in long edges (4 reaches every point a change may place), and its
/// rotation in degrees.
const RADIUS: (f64, f64) = (0.0, 4.0);
const TURN: (f64, f64) = (-360.0, 360.0);
/// The bounds and feathers of the range masks (luminance, depth), point-curve points and an
/// object's clicks (`mask.objectPoint` refuses a click outside the photo): 0 to 1.
const UNIT: (f64, f64) = (0.0, 1.0);
/// A colour-range sample: OkLab lightness and its two colour axes; `mask.sampleColor` keeps at most
/// [`MAX_SAMPLES`] samples.
const LIGHTNESS: (f64, f64) = (0.0, 1.0);
const CHROMA: (f64, f64) = (-1.0, 1.0);
const MAX_SAMPLES: usize = 5;
/// An AI mask's edge: -100 (harder) to 100 (softer).
const EDGE: (f64, f64) = (-100.0, 100.0);
/// A mask's own adjustments, as `mask.adjust` clamps them (`clamp_local`,
/// `crates/engine/src/cmd/masks.rs:916-943`): exposure, the other sliders, the colour overlay's hue
/// (its saturation is a percentage) and the amount.
const LOCAL_EXPOSURE: (f64, f64) = (-4.0, 4.0);
const LOCAL_SLIDER: (f64, f64) = (-100.0, 100.0);
const LOCAL_HUE: (f64, f64) = (0.0, 360.0);
const LOCAL_AMOUNT: (f64, f64) = (0.0, 200.0);
/// The highest mask id: the highest the mask overlay addresses (`Overlay::Mask { id: u16 }`). The
/// engine numbers a new mask after the photo's highest (`next_mask_id`, `Preset::apply`), so this
/// also keeps that sum far from the end of `u32` (`crates/develop/src/lib.rs:94-96`,
/// `presets.rs:222-229`).
const MASK_ID_MAX: u32 = u16::MAX as u32;
/// The most Upright guides (`Geometry::guides`: up to 4 segments).
const MAX_GUIDES: usize = 4;
/// Counts. The editor's commands cap none of these, and past them one change could keep the next
/// render busy for minutes or run it out of memory: each red-eye correction and each mask component
/// visits the whole output, each brush stroke paints a plane of it.
const MAX_MASKS: usize = 32;
const MAX_COMPONENTS: usize = 64;
const MAX_STROKES: usize = 256;
const MAX_STROKE_POINTS: usize = 10_000;
const MAX_SPOTS: usize = 1_000;
const MAX_SPOT_POINTS: usize = 10_000;
const MAX_EYES: usize = 64;
/// Brush work: a stroke lays a dab at each point and one every quarter radius between them
/// (`brush_dabs`, `crates/pipeline/src/masks.rs:274-302`), however tight the crop, and each dab
/// paints a box twice its radius wide, at most the output (`rasterize_brush`, `:365-406`). The
/// paint is counted in output planes: each box's area over the square of the output's long edge,
/// its radius magnified by the crop ([`magnification`]). The dabs and the paint are estimates that
/// never fall short of the renderer's.
const MAX_DABS: f64 = 1_000_000.0;
const MAX_PAINT: f64 = 64.0;
/// The paint an Auto Mask stroke adds to its dabs': the guided filter that refines the stroke runs
/// over the whole output once per stroke, with six Gaussian blurs and two more passes (`auto_refine`,
/// `guided_cross`, `crates/pipeline/src/masks.rs:332-360, 395-399`), so each such stroke counts this
/// many output planes, however few dabs it lays.
const AUTO_PLANES: f64 = 8.0;
/// Spot work: a spot measures each pixel of its box (its points' bounds widened by twice its radius
/// on every side, at most the output each way) against each of its points, and blurs and blends
/// the box (`crates/pipeline/src/spots.rs:110-212`): the box's area in output planes (its sides
/// magnified by the crop) times its points and [`SPOT_PASSES`].
const MAX_SPOT_WORK: f64 = 128.0;
const SPOT_PASSES: f64 = 16.0;
/// The blurs the renderer derives from a size given as a fraction of the photo, each a Gaussian
/// sigma per unit of that size, in output pixels per long-edge unit: a heal or remove spot's tone
/// correction (`0.6` of its radius, `crates/pipeline/src/spots.rs:177`); an Auto Mask stroke's
/// refinement (`0.25` of its radius, `masks.rs:334-336`); the saliency heuristic of Subject,
/// Background, People and an Object not yet segmented (`0.03`, `masks.rs:172`; a segmentation's
/// feathered edge takes at most `0.02`, `:163`, so an Object or a Prompt counts `0.03` too); and
/// the Sky heuristic (`0.01`, `:154`). The spatial planes' own come from `plane_sigmas`.
const HEAL_BLUR: f64 = 0.6;
const AUTO_BLUR: f64 = 0.25;
const SALIENCY_BLUR: f64 = 0.03;
const SKY_BLUR: f64 = 0.01;
/// The widest blur a change may add or widen, as a Gaussian sigma in pixels of an output
/// [`EXPORT_MAX`] long: the saliency heuristic's in the tightest crop the crop tool makes
/// (`0.03 × 4096 / 0.02`), the widest the editor's own tools lead to, already half again as wide as
/// that output. It is the hook's bound, not the renderer's limit: since bug-fix batch 1 the box
/// filter takes any sigma (`box_radii` works in `i64` and caps the sigma at `MAX_SIGMA`, and
/// `gaussian` caps each pass at the image's side, `crates/raster/src/blur.rs:6-33, 93-114`), where it
/// overflowed `i32` past about 23,170 pixels before.
const SIGMA_MAX: f64 = 6_144.0;
/// The widest defringe filter a change may add or widen, as a radius in pixels of an output
/// [`EXPORT_MAX`] long. Defringe takes the least and the most lightness within this radius of each
/// pixel, across and down (`min_filter`, `crates/pipeline/src/optics.rs:515-527, 561-585`), so its
/// work grows with the radius, which grows with the amount and the crop's magnification. In the
/// whole photo the editor's largest amount (20) gives about 7 pixels.
const DEFRINGE_MAX: f64 = 64.0;
/// The least of a crop that Constrain Crop keeps, each way (`t.max(0.02)`,
/// `crates/pipeline/src/geometry.rs:95`). Where the renderer solves the Upright transform from the
/// pixels ([`solved_upright`]), the crop it shows is anything from this much of the settings' crop
/// to all of it.
const CONSTRAIN_FLOOR: f64 = 0.02;
/// What a refusal at the crop's magnification adds when the crop shown is counted at
/// [`CONSTRAIN_FLOOR`].
const SOLVED_UPRIGHT: &str = " (Constrain Crop with an Upright solved from the pixels: the crop shown counts as 0.02 of itself)";
/// Visualize Spots' blur: the view compares the render's luminance with a Gaussian blur of it whose
/// sigma is this many times the output's pixels per long-edge unit of the photo (`SPOT_SIGMA`,
/// `crates/pipeline/src/visualize.rs:204, 213-225`), so its width grows with the crop's
/// magnification as the blurs of [`blurs`] do.
const SPOTS_VIEW_BLUR: f64 = 0.006;
/// Visualize Spots' threshold when the overlay names none: the editor's default
/// (`crates/ui-egui/src/state.rs:611`).
const SPOTS_THRESHOLD: u8 = 50;

/// Holds `after`, the settings a change would commit in place of the photo's `before`, to what the
/// editor itself leaves, on every part the change sets (a part equal to `before`'s is the photo's
/// own and is not judged again): each control value within its range (`develop.controls`), the
/// sections no control reaches within the bounds above, and their lists within counts
/// (`out_of_range`); a profile is one the editor has, mask ids are distinct and the schema version
/// is the photo's (`invalid_params`). What the renderer then does for them, at the crop's
/// magnification, is held by [`magnified`].
fn bounded(s: &Session, before: &DevelopSettings, after: &DevelopSettings) -> Result<(), Refusal> {
    if after.version != before.version {
        return Err(invalid(format!("`settings.version` is the photo's schema version, {}", before.version)));
    }
    if after.profile.id != before.profile.id && !known_profile(s, &after.profile.id) {
        return Err(invalid(format!("`settings.profile`: there is no profile `{}` (profiles.list)", clip(&after.profile.id))));
    }
    for spec in controls::CONTROLS {
        if let Some(value) = controls::get(after, spec.id)
            && controls::get(before, spec.id) != Some(value)
        {
            in_range(format_args!("{}", spec.id), value, (spec.min, spec.max))?;
        }
    }
    if after.curve != before.curve {
        let curve = &after.curve;
        for (channel, points) in [("master", &curve.master), ("red", &curve.red), ("green", &curve.green), ("blue", &curve.blue)] {
            points_in(format_args!("settings.curve.{channel}"), points.iter().map(|p| (p.x, p.y)), UNIT)?;
        }
    }
    if after.crop != before.crop {
        let rect = after.crop.geometry.rect;
        crop_extent(format_args!("settings.crop.geometry.rect"), [rect.x0, rect.y0, rect.x1, rect.y1])?;
    }
    if after.geometry.guides != before.geometry.guides {
        let guides = &after.geometry.guides;
        at_most(format_args!("settings.geometry.guides"), guides.len(), MAX_GUIDES)?;
        points_in(format_args!("settings.geometry.guides"), guides.iter().flat_map(|(a, b)| [(a.x, a.y), (b.x, b.y)]), POINTS)?;
    }
    if after.point_colors != before.point_colors {
        at_most(format_args!("settings.point_colors"), after.point_colors.len(), MAX_POINT_COLORS)?;
        indexed_in_range(before, after, "pointColor.")?;
    }
    if after.red_eye != before.red_eye {
        eyes_in_range(&after.red_eye)?;
        indexed_in_range(before, after, "redEye.")?;
    }
    if after.spots != before.spots {
        spots_in_range(&after.spots)?;
    }
    if after.masks != before.masks {
        masks_in_range(&after.masks)?;
    }
    Ok(())
}

/// `value`, which `what` names, within `min..=max` (`out_of_range` otherwise).
fn in_range(what: Arguments<'_>, value: f64, (min, max): (f64, f64)) -> Result<(), Refusal> {
    if (min..=max).contains(&value) { Ok(()) } else { Err(Refusal::new("out_of_range", format!("`{what}` is {min} to {max}, not {value}"))) }
}

/// Both coordinates of each point within `range`.
fn points_in(what: Arguments<'_>, points: impl IntoIterator<Item = (f64, f64)>, range: (f64, f64)) -> Result<(), Refusal> {
    points.into_iter().try_for_each(|(x, y)| {
        in_range(what, x, range)?;
        in_range(what, y, range)
    })
}

/// At most `max` of what `what` names (`out_of_range` otherwise).
fn at_most(what: Arguments<'_>, count: usize, max: usize) -> Result<(), Refusal> {
    if count <= max { Ok(()) } else { Err(Refusal::new("out_of_range", format!("`{what}` holds at most {max}, not {count}"))) }
}

/// Rendering work `work` within `budget` (`budget_exceeded` otherwise).
fn within_budget(what: Arguments<'_>, work: f64, budget: f64) -> Result<(), Refusal> {
    if work <= budget {
        Ok(())
    } else {
        Err(Refusal::new(
            "budget_exceeded",
            format!("`{what}` would take {work:.1} of rendering work, past the hook's budget of {budget}: use fewer, smaller or shorter ones"),
        ))
    }
}

/// A crop rectangle `[x0, y0, x1, y1]` the editor's crop tool could leave: corners within the
/// rectangle's range, wider and taller than [`MIN_CROP`] (`out_of_range` otherwise).
fn crop_extent(what: Arguments<'_>, [x0, y0, x1, y1]: [f64; 4]) -> Result<(), Refusal> {
    points_in(what, [(x0, y0), (x1, y1)], POINTS)?;
    if x1 - x0 > MIN_CROP && y1 - y0 > MIN_CROP {
        Ok(())
    } else {
        let size = format!("{} × {}", x1 - x0, y1 - y0);
        Err(Refusal::new("out_of_range", format!("`{what}` is wider and taller than {MIN_CROP} of the photo (the editor's crop tool), not {size}")))
    }
}

/// The indexed controls of `family` (`pointColor.`, `redEye.`: one set per element of its list)
/// whose value changed, within their ranges.
fn indexed_in_range(before: &DevelopSettings, after: &DevelopSettings, family: &str) -> Result<(), Refusal> {
    for (control, spec) in controls::indexed_instances(after).into_iter().filter(|(control, _)| control.starts_with(family)) {
        if let Some(value) = controls::get(after, &control)
            && controls::get(before, &control) != Some(value)
        {
            in_range(format_args!("{control}"), value, (spec.min, spec.max))?;
        }
    }
    Ok(())
}

/// Red-eye corrections as `redeye.add` and `redeye.catchlight` leave them, at most [`MAX_EYES`] (the
/// pupil size and darkening are indexed controls).
fn eyes_in_range(eyes: &[RedEye]) -> Result<(), Refusal> {
    at_most(format_args!("settings.red_eye"), eyes.len(), MAX_EYES)?;
    for (i, eye) in eyes.iter().enumerate() {
        points_in(format_args!("settings.red_eye[{i}].center"), [(eye.center.x, eye.center.y)], POINTS)?;
        in_range(format_args!("settings.red_eye[{i}].rx"), eye.rx, EYE_RADIUS)?;
        in_range(format_args!("settings.red_eye[{i}].ry"), eye.ry, EYE_RADIUS)?;
        points_in(format_args!("settings.red_eye[{i}].catchlight"), eye.catchlight.map(|c| (c.x, c.y)), CATCHLIGHT)?;
    }
    Ok(())
}

/// Spots as `spot.add` and `spot.update` leave them, at most [`MAX_SPOTS`] with
/// [`MAX_SPOT_POINTS`] points between them (their work is held by [`spots_at`]).
fn spots_in_range(spots: &[Spot]) -> Result<(), Refusal> {
    at_most(format_args!("settings.spots"), spots.len(), MAX_SPOTS)?;
    at_most(format_args!("the points of settings.spots"), spots.iter().map(|spot| spot.points.len()).sum(), MAX_SPOT_POINTS)?;
    for (i, spot) in spots.iter().enumerate() {
        in_range(format_args!("settings.spots[{i}].size"), spot.size, SPOT_SIZE)?;
        in_range(format_args!("settings.spots[{i}].feather"), spot.feather, PERCENT)?;
        in_range(format_args!("settings.spots[{i}].opacity"), spot.opacity, PERCENT)?;
        points_in(format_args!("settings.spots[{i}].points"), spot.points.iter().map(|p| (p.x, p.y)), POINTS)?;
        points_in(format_args!("settings.spots[{i}].source_offset"), spot.source_offset.map(|o| (o.x, o.y)), SPOT_OFFSET)?;
    }
    Ok(())
}

/// The largest of `values` minus the smallest (0 for none).
fn span(values: impl Iterator<Item = f64>) -> f64 {
    let (low, high) = values.fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), value| (low.min(value), high.max(value)));
    (high - low).max(0.0)
}

/// Masks as the editor's commands leave them: at most [`MAX_MASKS`] with [`MAX_COMPONENTS`]
/// components and [`MAX_STROKES`] brush strokes between them, distinct ids up to [`MASK_ID_MAX`],
/// a mask's own adjustments and edge refinement within range, and each shape's values within range
/// (their brushes' work is held by [`brushes_at`]).
fn masks_in_range(masks: &[Mask]) -> Result<(), Refusal> {
    at_most(format_args!("settings.masks"), masks.len(), MAX_MASKS)?;
    at_most(format_args!("the components of settings.masks"), masks.iter().map(|mask| mask.components.len()).sum(), MAX_COMPONENTS)?;
    let mut ids = BTreeSet::new();
    let mut strokes = 0;
    for (i, mask) in masks.iter().enumerate() {
        if mask.id > MASK_ID_MAX {
            return Err(Refusal::new("out_of_range", format!("`settings.masks[{i}].id` is at most {MASK_ID_MAX}, not {}", mask.id)));
        }
        if !ids.insert(mask.id) {
            return Err(invalid(format!("`settings.masks`: mask {} is there twice (mask ids are distinct)", mask.id)));
        }
        in_range(format_args!("settings.masks[{i}].refine"), mask.refine, PERCENT)?;
        local_in_range(i, &mask.adjust)?;
        for (k, component) in mask.components.iter().enumerate() {
            strokes += shape_in_range(i, k, &component.shape)?;
        }
    }
    at_most(format_args!("the brush strokes of settings.masks"), strokes, MAX_STROKES)
}

/// Mask `i`'s own adjustments as `mask.adjust` clamps them.
fn local_in_range(i: usize, a: &LocalAdjustments) -> Result<(), Refusal> {
    let sliders = [
        ("temp", a.temp),
        ("tint", a.tint),
        ("contrast", a.contrast),
        ("highlights", a.highlights),
        ("shadows", a.shadows),
        ("whites", a.whites),
        ("blacks", a.blacks),
        ("texture", a.texture),
        ("clarity", a.clarity),
        ("dehaze", a.dehaze),
        ("hue", a.hue),
        ("saturation", a.saturation),
        ("sharpness", a.sharpness),
        ("noise", a.noise),
        ("moire", a.moire),
        ("defringe", a.defringe),
    ];
    let others = [
        ("exposure", a.exposure, LOCAL_EXPOSURE),
        ("color_hue", a.color_hue, LOCAL_HUE),
        ("color_sat", a.color_sat, PERCENT),
        ("amount", a.amount, LOCAL_AMOUNT),
    ];
    for (field, value, range) in sliders.into_iter().map(|(field, value)| (field, value, LOCAL_SLIDER)).chain(others) {
        in_range(format_args!("settings.masks[{i}].adjust.{field}"), value, range)?;
    }
    Ok(())
}

/// The shape of component `k` of mask `i` as the editor's commands and tools leave it. Answers its
/// number of brush strokes.
fn shape_in_range(i: usize, k: usize, shape: &MaskShape) -> Result<usize, Refusal> {
    let mut brush = 0;
    match shape {
        MaskShape::Brush { strokes } => {
            for (j, stroke) in strokes.iter().enumerate() {
                let points = stroke.points.len();
                at_most(format_args!("the points of settings.masks[{i}].components[{k}].shape.strokes[{j}]"), points, MAX_STROKE_POINTS)?;
                in_range(format_args!("settings.masks[{i}].components[{k}].shape.strokes[{j}].size"), stroke.size, BRUSH_SIZE)?;
                for (field, value) in [("feather", stroke.feather), ("flow", stroke.flow), ("density", stroke.density)] {
                    in_range(format_args!("settings.masks[{i}].components[{k}].shape.strokes[{j}].{field}"), value, PERCENT)?;
                }
                let at = stroke.points.iter().map(|p| (p.x, p.y));
                points_in(format_args!("settings.masks[{i}].components[{k}].shape.strokes[{j}].points"), at, POINTS)?;
            }
            brush = strokes.len();
        }
        MaskShape::Linear { start, end } => {
            points_in(format_args!("settings.masks[{i}].components[{k}].shape"), [(start.x, start.y), (end.x, end.y)], POINTS)?;
        }
        MaskShape::Radial { center, rx, ry, angle, feather, .. } => {
            points_in(format_args!("settings.masks[{i}].components[{k}].shape.center"), [(center.x, center.y)], POINTS)?;
            for (field, value, range) in [("rx", rx, RADIUS), ("ry", ry, RADIUS), ("angle", angle, TURN), ("feather", feather, PERCENT)] {
                in_range(format_args!("settings.masks[{i}].components[{k}].shape.{field}"), *value, range)?;
            }
        }
        MaskShape::ColorRange { samples, refine } => {
            at_most(format_args!("settings.masks[{i}].components[{k}].shape.samples"), samples.len(), MAX_SAMPLES)?;
            for [lightness, a, b] in samples {
                in_range(format_args!("settings.masks[{i}].components[{k}].shape.samples"), *lightness, LIGHTNESS)?;
                points_in(format_args!("settings.masks[{i}].components[{k}].shape.samples"), [(*a, *b)], CHROMA)?;
            }
            in_range(format_args!("settings.masks[{i}].components[{k}].shape.refine"), *refine, PERCENT)?;
        }
        MaskShape::LuminanceRange { lo, hi, lo_feather, hi_feather } => {
            for (field, value) in [("lo", lo), ("hi", hi), ("lo_feather", lo_feather), ("hi_feather", hi_feather)] {
                in_range(format_args!("settings.masks[{i}].components[{k}].shape.{field}"), *value, UNIT)?;
            }
        }
        MaskShape::DepthRange { lo, hi, feather } => {
            for (field, value) in [("lo", lo), ("hi", hi), ("feather", feather)] {
                in_range(format_args!("settings.masks[{i}].components[{k}].shape.{field}"), *value, UNIT)?;
            }
        }
        MaskShape::Object { hint, exclude, edge, .. } => {
            at_most(format_args!("the clicks of settings.masks[{i}].components[{k}]"), hint.len() + exclude.len(), MAX_CLICKS)?;
            points_in(format_args!("the clicks of settings.masks[{i}].components[{k}]"), hint.iter().chain(exclude).map(|p| (p.x, p.y)), UNIT)?;
            in_range(format_args!("settings.masks[{i}].components[{k}].shape.edge"), *edge, EDGE)?;
        }
        MaskShape::Prompt { edge, .. } => in_range(format_args!("settings.masks[{i}].components[{k}].shape.edge"), *edge, EDGE)?,
        MaskShape::Subject | MaskShape::Sky | MaskShape::Background | MaskShape::People { .. } | MaskShape::Landscape { .. } => {}
    }
    Ok(brush)
}

// ------------------------------------------------------------------------ the crop's magnification

/// A crop the renderer may show: its sides, as fractions of the photo's width and height, and the
/// renderer's own magnification of the photo in it ([`real_magnification`]).
#[derive(Clone, Copy)]
struct ShownCrop {
    sides: (f64, f64),
    real: f64,
}

/// The crop the renderer shows, at the least and at the most it may be.
struct Shown {
    least: ShownCrop,
    most: ShownCrop,
}

/// The renderer's frame of photo `id` under settings `d`, its crop `applied` or not
/// (`Frame::with_lens` over the catalog's size, the settings and the source's lens, as a render
/// frames it), and the sides of the crop it shows at the least and at the most, as fractions of the
/// photo's width and height: the settings' crop, shrunk about its centre by Constrain Crop where a
/// transform or a lens correction leaves blank areas in it, as far down as [`CONSTRAIN_FLOOR`] of
/// itself (`crates/pipeline/src/geometry.rs:42-98`). The hook frames the photo as the renderer
/// does, so the least and the most are the same, except where the renderer solves the Upright
/// transform from the pixels ([`solved_upright`]): the crop shown is then anything from
/// [`CONSTRAIN_FLOOR`] of the settings' crop to all of it. Nothing is decoded.
fn framed(s: &Session, id: PhotoId, d: &DevelopSettings, applied: bool) -> (Frame, [(f64, f64); 2]) {
    let (width, height) = s.catalog.photo(id).map_or((3, 2), |p| (p.width.max(1), p.height.max(1)));
    let lens = s.source_info(id).lens;
    let frame = Frame::with_lens(width as usize, height as usize, d, applied, lens.as_ref());
    let solved = applied && solved_upright(d);
    let rect = if solved { d.crop.geometry.rect } else { frame.crop.rect };
    let floor = if solved { CONSTRAIN_FLOOR } else { 1.0 };
    let most = (rect.width(), rect.height());
    (frame, [(most.0 * floor, most.1 * floor), most])
}

/// The crop the renderer shows of photo `id` under settings `d` ([`framed`], the crop applied), with
/// the renderer's own magnification there.
fn shown_crop(s: &Session, id: PhotoId, d: &DevelopSettings) -> Shown {
    let (frame, [least, most]) = framed(s, id, d, true);
    let shown = |sides: (f64, f64)| ShownCrop { sides, real: real_magnification(&frame, sides) };
    Shown { least: shown(least), most: shown(most) }
}

/// The output's pixels per long-edge unit of the photo in a crop of `sides` (fractions of the
/// photo's width and height) of `frame`, for an output that fits `edge` × `edge`, as the renderer
/// sizes and scales a render (`Frame::fit`, `Frame::px_per_long`;
/// `crates/pipeline/src/lib.rs:314-315`).
fn px_per_long_at(frame: &Frame, (width, height): (f64, f64), edge: usize) -> f64 {
    let mut frame = frame.clone();
    let rect = &mut frame.crop.rect;
    (rect.x1, rect.y1) = (rect.x0 + width, rect.y0 + height);
    let (out_width, _) = frame.fit(edge, edge);
    frame.px_per_long(out_width)
}

/// The renderer's own magnification of the photo in a crop of `sides` of `frame`: the pixels per
/// long-edge unit of an output [`EXPORT_MAX`] long ([`px_per_long_at`]) over that long edge.
/// Rounding aside, it is the photo's long edge over the crop's longer side in pixels, so it is at
/// most one over the crop's smaller side ([`magnification`]), and far less for a long thin crop.
fn real_magnification(frame: &Frame, sides: (f64, f64)) -> f64 {
    px_per_long_at(frame, sides, EXPORT_MAX as usize) / f64::from(EXPORT_MAX)
}

/// Whether the renderer solves settings `d`'s Upright transform from the pixels under Constrain
/// Crop: an automatic mode (Auto, Level, Vertical or Full) stored without its transform (from an
/// XMP sidecar, a preset or complete settings), which `upright::resolve` computes before it frames
/// the photo (`crates/pipeline/src/upright.rs:587-596`, `lib.rs:306-313`). The hook decodes
/// nothing, so it cannot tell how far Constrain Crop then shrinks the crop.
fn solved_upright(d: &DevelopSettings) -> bool {
    let g = &d.geometry;
    let automatic = !matches!(g.upright, Upright::Off | Upright::Guided);
    g.constrain_crop && automatic && g.upright_transform.is_none() && d.section_enabled("geometry")
}

/// Whether settings `a` and `b` frame the photo alike: the same orientation, crop, geometry and
/// optics, and the same geometry and optics toggles, which is everything the renderer's frame reads
/// (`Frame::with_lens`, `crates/pipeline/src/geometry.rs:42-70`; `Warp::from_settings`,
/// `optics.rs:72-90`). The renderer then shows the same crop for both, whatever it solves from the
/// pixels.
fn same_frame(a: &DevelopSettings, b: &DevelopSettings) -> bool {
    let toggles = |d: &DevelopSettings| (d.section_enabled("geometry"), d.section_enabled("optics"));
    a.orientation == b.orientation && a.crop == b.crop && a.geometry == b.geometry && a.optics == b.optics && toggles(a) == toggles(b)
}

/// One over the smaller side of a crop of `sides` (fractions of the photo's width and height): the
/// magnification at which sizes and work are held. An output's pixels per long-edge unit of the
/// photo are, rounding aside, at most its long edge times this ([`real_magnification`]), so a size
/// given as a fraction of the photo (a spot's or a brush's radius, a blur's width) is at most this
/// many times that fraction of the output's long edge.
fn magnification((width, height): (f64, f64)) -> f64 {
    1.0 / width.min(height).max(f64::MIN_POSITIVE)
}

/// Holds what the renderer would do for `after`, the settings a change would commit in place of
/// photo `id`'s `before`, at the crop's magnification (any change: values, sections, complete
/// settings, presets and crops), by [`at_magnification`]. A refusal says so when the crop shown was
/// counted at [`CONSTRAIN_FLOOR`] ([`solved_upright`]). Nothing is decoded.
fn magnified(s: &Session, id: PhotoId, before: &DevelopSettings, after: &DevelopSettings) -> Result<(), Refusal> {
    let note = if solved_upright(after) { SOLVED_UPRIGHT } else { "" };
    at_magnification(s, id, before, after).map_err(|refusal| refusal.noting(note))
}

/// [`magnified`]'s checks. A change is judged where it may magnify the photo the most, at the least
/// crop it leaves shown; the photo's own state where it may magnify the photo the least, at the
/// most crop shown before, or at the change's own when the change leaves the frame as it was
/// ([`same_frame`]). The change makes the crop tighter when it raises either measure of the
/// magnification: one over the smaller side ([`magnification`]), or the renderer's own, which
/// follows the crop's longer side, so that a long thin strip squared up is tighter although its
/// smaller side stays. When it does, the crop the renderer shows stays wider and taller than
/// [`MIN_CROP`] of the photo, and the photo's own spots and brushes are judged again. Spots and
/// brushes the change sets, or that are judged again, are held to [`spots_at`] and [`brushes_at`]
/// at one over the smaller side. The blurs the renderer derives from the settings ([`blurs`]) and
/// defringe's filter ([`defringe_radius`]) are taken at the renderer's own magnification on both
/// sides, like with like: no blur may be added or widened past [`SIGMA_MAX`] ([`widened`]), nor
/// defringe's filter grow past [`DEFRINGE_MAX`] (`out_of_range`).
fn at_magnification(s: &Session, id: PhotoId, before: &DevelopSettings, after: &DevelopSettings) -> Result<(), Refusal> {
    let shown = shown_crop(s, id, after).least;
    let own = if same_frame(before, after) { shown } else { shown_crop(s, id, before).most };
    let zoom = magnification(shown.sides);
    let tighter = zoom > magnification(own.sides) || shown.real > own.real;
    if tighter {
        shown_in_range(shown.sides)?;
    }
    if tighter || after.spots != before.spots {
        spots_at(&after.spots, zoom)?;
    }
    if tighter || after.masks != before.masks {
        brushes_at(&after.masks, zoom)?;
    }
    if let Some((blur, sigma)) = widened(&blurs(before, own.real), &blurs(after, shown.real)) {
        let what = blur.name();
        let message = format!("{what} would blur by {sigma:.0} px at {EXPORT_MAX} px, past the hook's {SIGMA_MAX}: crop wider first");
        return Err(Refusal::new("out_of_range", message));
    }
    let radius = defringe_radius(after, shown.real);
    if radius > DEFRINGE_MAX && radius > defringe_radius(before, own.real) {
        let message = format!("defringe would reach {radius:.0} px at {EXPORT_MAX} px, past {DEFRINGE_MAX}: lower it or crop wider first");
        return Err(Refusal::new("out_of_range", message));
    }
    Ok(())
}

/// The crop the renderer shows, held to the crop tool's minimum: wider and taller than
/// [`MIN_CROP`] of the photo (`out_of_range` otherwise).
fn shown_in_range((width, height): (f64, f64)) -> Result<(), Refusal> {
    if width > MIN_CROP && height > MIN_CROP {
        Ok(())
    } else {
        let message = format!("the crop shown (Constrain Crop included) is wider and taller than {MIN_CROP}, not {width:.4} × {height:.4}");
        Err(Refusal::new("out_of_range", message))
    }
}

/// Spots at the crop's magnification `zoom`: a heal or remove spot, which blurs what it covers
/// ([`HEAL_BLUR`]), no larger in the output than `spot.add` lets one be in the whole photo
/// ([`SPOT_SIZE`]; `out_of_range`), and the spots' work within [`MAX_SPOT_WORK`]
/// (`budget_exceeded`).
fn spots_at(spots: &[Spot], zoom: f64) -> Result<(), Refusal> {
    let mut work = 0.0;
    for (i, spot) in spots.iter().enumerate() {
        if spot.mode != SpotMode::Clone && spot.size * zoom > SPOT_SIZE.1 {
            let (size, max) = (spot.size, SPOT_SIZE.1 / zoom);
            let message = format!("`settings.spots[{i}].size` is at most {max:.4} for heal and remove spots in this crop, not {size}");
            return Err(Refusal::new("out_of_range", message));
        }
        let side = |extent: f64| ((extent + 4.0 * spot.size) * zoom).min(1.0);
        let area = side(span(spot.points.iter().map(|p| p.x))) * side(span(spot.points.iter().map(|p| p.y)));
        work += area * (spot.points.len() as f64 + SPOT_PASSES);
    }
    within_budget(format_args!("settings.spots"), work, MAX_SPOT_WORK)
}

/// The brush strokes of `masks` at the crop's magnification `zoom`: an Auto Mask stroke, whose
/// refinement blurs the whole output ([`AUTO_BLUR`]), no wider in the output than the editor's
/// brush is in the whole photo ([`BRUSH_SIZE`]; `out_of_range`); their dabs within [`MAX_DABS`]
/// and their paint within [`MAX_PAINT`], each Auto Mask stroke adding [`AUTO_PLANES`]
/// (`budget_exceeded`).
fn brushes_at(masks: &[Mask], zoom: f64) -> Result<(), Refusal> {
    let (mut dabs, mut paint) = (0.0, 0.0);
    for (i, mask) in masks.iter().enumerate() {
        for (k, component) in mask.components.iter().enumerate() {
            let MaskShape::Brush { strokes } = &component.shape else { continue };
            for (j, stroke) in strokes.iter().enumerate() {
                if stroke.auto_mask && stroke.size * zoom > BRUSH_SIZE.1 {
                    let (size, max) = (stroke.size, BRUSH_SIZE.1 / zoom);
                    let what = format!("settings.masks[{i}].components[{k}].shape.strokes[{j}].size");
                    let message = format!("`{what}` is at most {max:.4} for an Auto Mask stroke in this crop, not {size}");
                    return Err(Refusal::new("out_of_range", message));
                }
                // a dab at the first point, then a dab every quarter radius, at least one per point
                let between: f64 = stroke
                    .points
                    .iter()
                    .zip(stroke.points.iter().skip(1))
                    .map(|(a, b)| (4.0 * (b.x - a.x).hypot(b.y - a.y) / stroke.size).ceil().max(1.0))
                    .sum();
                let count = 1.0 + between;
                dabs += count;
                paint += count * (4.0 * stroke.size * stroke.size * zoom * zoom).min(1.0);
                if stroke.auto_mask {
                    paint += AUTO_PLANES;
                }
            }
        }
    }
    within_budget(format_args!("the brush dabs of settings.masks"), dabs, MAX_DABS)?;
    within_budget(format_args!("the brush paint of settings.masks"), paint, MAX_PAINT)
}

/// The kinds of blur the renderer derives from a size given as a fraction of the photo, which grow
/// with the crop's magnification ([`blurs`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Blur {
    /// The highlights and shadows base (`plane_sigmas`, `crates/pipeline/src/local.rs:255-277`).
    Base,
    /// The clarity band.
    Clarity,
    /// The texture band, which sharpening reads at Radius 1.
    Texture,
    /// Sharpening's own band at any other Radius (bug-fix batch 2): the texture band's sigma times
    /// Radius, 0.5 to 3 (`plane_sigmas`'s `sharpen`, `crates/pipeline/src/local.rs:297-299`).
    Sharpen,
    /// The dehaze dark channel.
    Dehaze,
    /// The chromaticity blur of a mask's moiré and noise.
    Chroma,
    /// The saliency heuristic of a Subject, Background, People, Object or Prompt mask
    /// ([`SALIENCY_BLUR`]).
    Saliency,
    /// The Sky heuristic ([`SKY_BLUR`]).
    Sky,
    /// An Auto Mask stroke's refinement ([`AUTO_BLUR`]).
    Auto,
    /// A heal or remove spot's tone correction ([`HEAL_BLUR`]).
    Heal,
}

impl Blur {
    /// What a refusal calls the blur.
    fn name(self) -> &'static str {
        match self {
            Blur::Base => "the highlights and shadows base",
            Blur::Clarity => "the clarity band",
            Blur::Texture => "the texture band",
            Blur::Sharpen => "the sharpening band",
            Blur::Dehaze => "the dehaze plane",
            Blur::Chroma => "a mask's colour-noise plane",
            Blur::Saliency => "a Subject, Background, People, Object or Prompt mask",
            Blur::Sky => "a Sky mask",
            Blur::Auto => "an Auto Mask stroke",
            Blur::Heal => "a heal or remove spot",
        }
    }
}

/// The renderer's blurs that grow with the crop's magnification, for settings `d` (its profile's
/// look applied, as the renderer applies it: `profiles::effective`, `crates/pipeline/src/lib.rs:307`)
/// at the renderer's own magnification `real` ([`real_magnification`]), each as a Gaussian sigma in
/// pixels of an output [`EXPORT_MAX`] long: the spatial planes the settings need (`plane_sigmas`,
/// sharpening's own band among them since bug-fix batch 2), each mask component's shape
/// ([`shape_blurs`]) and each heal and remove spot ([`HEAL_BLUR`]). A mask's edge refinement
/// (`0.04` of the output's long edge, `crates/pipeline/src/masks.rs:63`) does not grow with the
/// magnification.
fn blurs(d: &DevelopSettings, real: f64) -> Vec<(Blur, f64)> {
    let d = profiles::effective(d);
    let ppl = real * f64::from(EXPORT_MAX);
    let p = plane_sigmas(&d, ppl, Quality::Full);
    let planes = [
        (Blur::Base, p.base),
        (Blur::Clarity, p.clarity),
        (Blur::Texture, p.texture),
        (Blur::Sharpen, p.sharpen),
        (Blur::Dehaze, p.dark),
        (Blur::Chroma, p.chroma),
    ];
    let mut blurs: Vec<(Blur, f64)> = planes.into_iter().filter_map(|(blur, sigma)| sigma.map(|sigma| (blur, f64::from(sigma)))).collect();
    for component in d.masks.iter().flat_map(|mask| &mask.components) {
        blurs.extend(shape_blurs(&component.shape).into_iter().map(|(blur, per)| (blur, per * ppl)));
    }
    blurs.extend(d.spots.iter().filter(|spot| spot.mode != SpotMode::Clone).map(|spot| (Blur::Heal, HEAL_BLUR * spot.size * ppl)));
    blurs
}

/// The blurs a mask's shape takes, each per unit of the output's pixels per long-edge unit: the
/// saliency or Sky heuristic's, or the refinement of each of its Auto Mask strokes.
fn shape_blurs(shape: &MaskShape) -> Vec<(Blur, f64)> {
    match shape {
        MaskShape::Subject | MaskShape::Background => vec![(Blur::Saliency, SALIENCY_BLUR)],
        MaskShape::People { .. } | MaskShape::Object { .. } | MaskShape::Prompt { .. } => vec![(Blur::Saliency, SALIENCY_BLUR)],
        MaskShape::Sky => vec![(Blur::Sky, SKY_BLUR)],
        MaskShape::Brush { strokes } => strokes.iter().filter(|s| s.auto_mask).map(|s| (Blur::Auto, AUTO_BLUR * s.size)).collect(),
        _ => Vec::new(),
    }
}

/// The first blur of `after` past [`SIGMA_MAX`] that `before` has no blur of its kind at least as
/// wide for, each blur of `before` answering for one of `after`'s, the widest for the widest: a
/// change may keep or narrow the photo's own blurs, but may add or widen none past the bound.
fn widened(before: &[(Blur, f64)], after: &[(Blur, f64)]) -> Option<(Blur, f64)> {
    let widest_first = |blurs: &[(Blur, f64)], kind: Blur| {
        let mut sigmas: Vec<f64> = blurs.iter().filter(|(blur, _)| *blur == kind).map(|(_, sigma)| *sigma).collect();
        sigmas.sort_by(f64::total_cmp);
        sigmas.reverse();
        sigmas
    };
    // the kinds past the bound: a kind's neighbouring blurs (all the spots') are checked once
    let mut kinds: Vec<Blur> = after.iter().filter(|(_, sigma)| *sigma > SIGMA_MAX).map(|(kind, _)| *kind).collect();
    kinds.dedup();
    kinds.into_iter().find_map(|kind| {
        let (was, is) = (widest_first(before, kind), widest_first(after, kind));
        is.into_iter().enumerate().find(|&(i, sigma)| sigma > SIGMA_MAX && was.get(i).is_none_or(|w| sigma > *w)).map(|(_, sigma)| (kind, sigma))
    })
}

/// Defringe's filter radius for settings `d` at the renderer's own magnification `real`, in pixels
/// of an output [`EXPORT_MAX`] long: one reference pixel, and 0.3 more for each step of the larger
/// amount, a reference pixel being the output's pixels per long-edge unit over [`DEFRINGE_REF_LONG`]
/// (`optics::defringe`, `crates/pipeline/src/optics.rs:515-523`); none without defringe.
fn defringe_radius(d: &DevelopSettings, real: f64) -> f64 {
    let amount = d.optics.defringe_purple_amount.max(d.optics.defringe_green_amount);
    if d.section_enabled("optics") && amount > 0.0 { (1.0 + 0.3 * amount) * real * f64::from(EXPORT_MAX) / DEFRINGE_REF_LONG } else { 0.0 }
}

// ------------------------------------------------------------------------------------------ reads

/// `lts.state {id?, settings?}`: the tokens. `photoId` (the id, else the active photo, else null)
/// and its settings token `hash`, `catalogRevision`, `activePhoto`, `interaction` (a human drag in
/// progress), `historyHead` and `redoHead`; with `settings: true` (R1) also `settings`, the complete
/// settings `develop.get` gives (`crates/engine/src/cmd/query.rs:143-146`), the very ones `hash`
/// names (null without a photo): a checkpoint and its token in one answer.
fn state(s: &Session, q: &Params<'_>) -> Result<Map<String, Value>, Refusal> {
    let with_settings = q.flag("settings", false)?;
    let photo = match q.photo()? {
        Some(id) => Some((id, settings_of(s, id)?)),
        None => s.active().and_then(|id| s.develop_of(id).map(|settings| (id, settings))),
    };
    let mut fields = object(json!({
        "photoId": photo.as_ref().map(|(id, _)| id.0),
        "hash": photo.as_ref().map(|(_, settings)| hex(settings.hash64())),
        "catalogRevision": s.catalog.revision,
        "activePhoto": s.active().map(|id| id.0),
        "interaction": s.interaction.is_some(),
        "historyHead": head_json(&s.undo),
        "redoHead": head_json(&s.redo),
    }));
    if with_settings {
        fields.insert("settings".into(), photo.map_or(Value::Null, |(_, settings)| settings.to_json()));
    }
    Ok(fields)
}

/// `lts.library {offset?, limit?, fileName?}` (R1): the library's photos as `catalog.query {filter:
/// {}}` lists them (`crates/engine/src/cmd/query.rs:57-69`: the library's own photos, not those in
/// Recently Deleted, in the engine's order), `total` of them and a page of `limit` rows (1 to 200,
/// 200 by default) from row `offset` (0 by default) in `photos`, each the engine's own row with its
/// photo's settings token added (`hash`). With `fileName`, only the photos of exactly that file
/// name, 1 to 255 characters (the byte bridge's import, whose dropped files have unique names).
fn library(s: &mut Session, q: &Params<'_>) -> Result<Map<String, Value>, Refusal> {
    let offset = match q.get("offset") {
        None => 0,
        Some(offset) => offset.as_u64().ok_or_else(|| invalid("`offset` is an integer from 0: the row the page starts at"))?,
    };
    let limit = q.integer("limit", (1, MAX_PAGE), MAX_PAGE)?;
    let name = q.get("fileName").map(file_name).transpose()?;
    // the engine reads the offset as a `usize`: past `u32::MAX`, which no library reaches, the page
    // is empty all the same
    let from = offset.min(u64::from(u32::MAX));
    let named: Option<Vec<u64>> = name.map(|name| s.catalog.photos().filter(|p| p.file_name == name).map(|p| p.id.0).collect());
    let listed = match named {
        // no photo of that name: nothing to list (an empty `only` would list every photo)
        Some(ids) if ids.is_empty() => json!({"total": 0, "photos": []}),
        // the engine's own filter keeps them in its order and leaves out those in Recently Deleted
        Some(ids) => query(s, "catalog.query", &json!({"filter": {"only": ids}, "offset": from, "limit": limit}))?,
        None => query(s, "catalog.query", &json!({"filter": {}, "offset": from, "limit": limit}))?,
    };
    let rows: Vec<Value> = listed["photos"].as_array().into_iter().flatten().map(|row| with_token(s, row)).collect();
    Ok(object(json!({
        "total": listed["total"],
        "offset": offset,
        "limit": limit,
        "photos": rows,
        "catalogRevision": s.catalog.revision,
    })))
}

/// `fileName`: a photo's file name, 1 to [`MAX_FILE_NAME`] characters.
fn file_name(value: &Value) -> Result<&str, Refusal> {
    let refused = || invalid(format!("`fileName` is 1 to {MAX_FILE_NAME} characters: a photo's file name"));
    value.as_str().filter(|name| (1..=MAX_FILE_NAME).contains(&name.chars().count())).ok_or_else(refused)
}

/// A row of `catalog.query` with its photo's settings token added (`hash`).
fn with_token(s: &Session, row: &Value) -> Value {
    let mut row = object(row.clone());
    let settings = row.get("id").and_then(Value::as_u64).and_then(|id| s.develop_of(PhotoId(id)));
    row.insert("hash".into(), json!(settings.map(|settings| hex(settings.hash64()))));
    Value::Object(row)
}

/// `lts.photo {id}` (R1): `photo.inspect` of photo `id` (`photo`: its record, with its albums, its
/// history's labels, its versions and its stack, `crates/engine/src/cmd/query.rs:127-142`), its
/// settings token (`hash`, of the very settings `photo.develop` holds) and the token of each step
/// of its history (`revisions`: `step`, `label` and `hash`, the oldest first), which is the
/// `revision` `lts.restoreSettings` puts back.
fn photo_read(s: &mut Session, q: &Params<'_>) -> Result<Map<String, Value>, Refusal> {
    let id = q.target()?;
    let settings = settings_of(s, id)?;
    let revisions = s.catalog.photo(id).map(|photo| history_tokens(photo)).unwrap_or_default();
    let inspected = query(s, "photo.inspect", &json!({"id": id.0}))?;
    Ok(object(json!({
        "photoId": id.0,
        "photo": inspected,
        "hash": hex(settings.hash64()),
        "revisions": revisions,
        "catalogRevision": s.catalog.revision,
    })))
}

/// Each step of photo `photo`'s history, the oldest first: its index, its label and its settings
/// token.
fn history_tokens(photo: &Photo) -> Vec<Value> {
    let steps = photo.history.iter().enumerate();
    steps.map(|(step, entry)| json!({"step": step, "label": entry.label, "hash": hex(entry.settings.hash64())})).collect()
}

/// `lts.presets {}` (R1): the editor's presets as `presets.list` lists them (`presets`: id, name,
/// group, favourite, built in), its profiles as `profiles.menu` arranges them (`profiles`:
/// favourites, recent and groups) and its point-curve presets as `curve.presets` lists them
/// (`curvePresets`: the curves, and the preset the active photo's point curves match).
fn presets_read(s: &mut Session) -> Result<Map<String, Value>, Refusal> {
    let presets = query(s, "presets.list", &json!({}))?;
    let menu = query(s, "profiles.menu", &json!({}))?;
    let curves = query(s, "curve.presets", &json!({}))?;
    Ok(object(json!({"presets": presets, "profiles": menu, "curvePresets": curves})))
}

/// `lts.metadata {id}` (R1): `photo.allMetadata` of photo `id` (`metadata`: every EXIF, TIFF and
/// GPS tag of its file and its XMP properties, `crates/engine/src/cmd/query.rs:100-122`; a
/// generated demo photo has none). The position is the file's own: the adapter flags it.
fn metadata_read(s: &mut Session, q: &Params<'_>) -> Result<Map<String, Value>, Refusal> {
    let id = q.target()?;
    settings_of(s, id)?;
    let metadata = engine_query(s, "photo.allMetadata", &json!({"id": id.0})).map_err(|error| file_failure(&error))?;
    Ok(object(json!({"photoId": id.0, "metadata": metadata})))
}

/// `lts.journal {limit?}` (R1): the command journal as `journal.list` lists it (`journal`: the
/// latest `limit` entries, 1 to 10,000, 100 by default, each a command and its parameters,
/// `crates/engine/src/cmd/query.rs:213-217`) and how many entries it holds (`length`).
fn journal_read(s: &mut Session, q: &Params<'_>) -> Result<Map<String, Value>, Refusal> {
    let limit = q.integer("limit", (1, MAX_JOURNAL), JOURNAL_DEFAULT)?;
    let journal = query(s, "journal.list", &json!({"limit": limit}))?;
    Ok(object(json!({"journal": journal, "length": s.journal.len()})))
}

/// The engine's own answer to its query `command` with `params`, run by the query's spec here and
/// now (a query of the registry, never journaled): not journaled, nothing saved, no auto version,
/// and a panic in it an error. `Err` carries the engine's text.
fn engine_query(s: &mut Session, command: &str, params: &Value) -> Result<Value, String> {
    let spec = find_command(command).filter(|spec| !spec.journal).ok_or_else(|| format!("the engine has no query `{command}`"))?;
    guard::catch(command, || (spec.run)(s, params))?.map_err(|error| error.to_string())
}

/// [`engine_query`], a failure of which is the hook's: this build's engine does not answer the
/// query as the hook reads it (`hook_unavailable`).
fn query(s: &mut Session, command: &str, params: &Value) -> Result<Value, Refusal> {
    engine_query(s, command, params).map_err(|error| Refusal::new("hook_unavailable", format!("the engine's {command} failed: {}", clip(&error))))
}

/// A read of a photo's file that failed: its original still loading from browser storage
/// (`source_loading`, as [`render_failure`] tells it) or not readable (`decode_failed`).
fn file_failure(error: &str) -> Refusal {
    if error.contains("still loading") {
        Refusal::new("source_loading", format!("the photo's original is still loading: {}", clip(error)))
    } else {
        Refusal::new("decode_failed", format!("the photo's file could not be read: {}", clip(error)))
    }
}

// ------------------------------------------------------------------------- the control catalog

/// The controls that change nothing in any state: none since bug-fix batch 2, which made Sharpening
/// Radius, Sharpening Detail and Noise Reduction Contrast render (`plane_sigmas`'s `sharpen`,
/// `sharpen_detail_weight` and `nr_contrast_keep`, `crates/pipeline/src/local.rs` and `finish.rs`),
/// the three controls the catalog would have listed until then (backlog item 2).
const INERT: [&str; 0] = [];

/// `lts.controls {id?}` (R1, backlog item 2): the hook's control catalog, which labels each control
/// that changes nothing in some state of the settings ([`Gate`]). `controls`: every develop control
/// as `develop.controls` describes it (`id`, `label`, `section`, the range, default, step, decimals
/// and track, `crates/engine/src/cmd/query.rs:147-168`), in the table's order, each with the gates
/// it acts behind (`gates`: none for most, [`gates_of`]); `gates`: each gate in order, with when the
/// controls behind it change nothing and what opens it (`id`, `why`); `inert`: the controls that
/// change nothing in any state ([`INERT`]). With `id`, the catalog of that photo: each control with
/// the photo's value, its list controls after the table's (`pointColor.0.hueShift`,
/// `redEye.0.pupilSize`, as `develop.controls` lists them for the active photo), its settings token
/// (`hash`) and each gate open or closed for it (`open`).
fn controls_read(s: &Session, q: &Params<'_>) -> Result<Map<String, Value>, Refusal> {
    let gates: Vec<Value> = Gate::ALL.iter().map(|gate| json!({"id": gate.id(), "why": gate.why()})).collect();
    let mut fields = object(json!({"gates": gates, "inert": INERT}));
    let Some(id) = q.photo()? else {
        let entries: Vec<Value> = controls::CONTROLS.iter().map(|spec| control_entry(spec, spec.id, None)).collect();
        fields.insert("controls".into(), json!(entries));
        return Ok(fields);
    };
    let settings = settings_of(s, id)?;
    let d: &DevelopSettings = &settings;
    let lens = s.source_info(id).lens;
    let open: Map<String, Value> = Gate::ALL.iter().map(|gate| (gate.id().to_string(), json!(gate.open(d, lens.as_ref())))).collect();
    let listed = controls::CONTROLS.iter().map(|spec| (spec.id.to_string(), spec)).chain(controls::indexed_instances(d));
    let entries: Vec<Value> = listed.map(|(control, spec)| control_entry(spec, &control, Some(d))).collect();
    fields.extend(object(json!({"photoId": id.0, "hash": hex(d.hash64()), "open": open, "controls": entries})));
    Ok(fields)
}

/// Control `control` of spec `spec` (an indexed control under its own id, `pointColor.0.hueShift`)
/// as `develop.controls` describes it, with its value in `settings` when the catalog is a photo's,
/// and the gates it acts behind.
fn control_entry(spec: &controls::ControlSpec, control: &str, settings: Option<&DevelopSettings>) -> Value {
    let mut entry = object(serde_json::to_value(spec).unwrap_or_default());
    entry.insert("id".into(), json!(control));
    if let Some(settings) = settings {
        entry.insert("value".into(), json!(controls::get(settings, control)));
    }
    let gates: Vec<&str> = gates_of(control).into_iter().map(Gate::id).collect();
    entry.insert("gates".into(), json!(gates));
    Value::Object(entry)
}

/// A state of the settings in which some develop controls change no pixel (R1, backlog item 2), as
/// the renderer reads the settings: [`gates_of`] names the controls behind each gate, and
/// [`Gate::open`] reads whether a photo opens it. While a gate is closed its controls change no
/// pixel; once it is open they may (on some photos they still change nothing, and a split moves
/// only the regions beside it). A control behind no gate changes the render in every state of the
/// settings, though not every photo's pixels (Saturation changes nothing on a grey photo). The
/// gates are the section switches the renderer honours (four of those a person can turn off,
/// `crates/pipeline/src/finish.rs:226-253`, `optics.rs:74`, `geometry.rs:271`; it ignores the
/// Light, Color and Detail ones, which gate nothing), the modes (black and white or colour, a
/// profile with a look, the lens corrections a file embeds) and, since the second verifier round of
/// R1, the settings other controls act through: a vignette, grain, sharpening, noise reduction,
/// lens vignetting or defringe amount, a tone curve, a colour-grading wheel, a point colour's shift
/// and, in black and white, a B&W mix.
#[derive(Clone, Copy)]
enum Gate {
    /// The Effects section on: texture, clarity, dehaze, the vignette and grain.
    EffectsOn,
    /// The Optics section on: the lens corrections, chromatic aberration and defringe.
    OpticsOn,
    /// The Geometry section on: the Transform sliders (the straighten angle is the crop's).
    GeometryOn,
    /// The Calibration section on.
    CalibrationOn,
    /// Black and white, by the treatment or a B&W profile: the B&W mixes.
    Bw,
    /// Colour: the point colours.
    Color,
    /// A profile with a look: the profile's amount.
    ProfileLook,
    /// The lens profile on, with a distortion correction in the file: its Distortion.
    LensDistortion,
    /// The lens profile on, with a vignetting correction in the file: its Vignetting.
    LensVignetting,
    /// A negative vignette in Highlight Priority: the vignette's Highlights.
    HighlightPriority,
    /// Colour, or black and white with a B&W mix that weighs the colours (the photo's, or its
    /// profile's look's): Vibrance, Saturation and the Color Mixer's Hue and Saturation.
    ColorOrBwMix,
    /// A tone-curve region slider set (the profile's look's included): the region splits.
    CurveRegions,
    /// A tone curve that changes tones, a region slider set or a point curve that is not straight
    /// (the profile's look's included): Refine Saturation.
    CurveOn,
    /// A Shadows, Midtones or Highlights wheel with saturation or luminance (the profile's look's
    /// included): Blending and Balance.
    GradingZones,
    /// The Shadows wheel's own saturation: its Hue.
    ShadowsTint,
    /// The Midtones wheel's own saturation: its Hue.
    MidtonesTint,
    /// The Highlights wheel's own saturation: its Hue.
    HighlightsTint,
    /// The Global wheel's own saturation: its Hue.
    GlobalTint,
    /// A vignette amount: the vignette's Midpoint, Roundness and Feather.
    VignetteOn,
    /// A grain amount: the grain's Size and Roughness.
    GrainOn,
    /// Sharpening, or a mask that sharpens: Radius, Detail and Masking.
    SharpenOn,
    /// Luminance noise reduction: its Detail and Contrast.
    NrLuminanceOn,
    /// Colour noise reduction: its Detail and Smoothness.
    NrColorOn,
    /// The lens Vignetting set: its Midpoint.
    ManualVignetting,
    /// A purple defringe amount: the purple hue range.
    DefringePurple,
    /// A green defringe amount: the green hue range.
    DefringeGreen,
    /// A point colour that shifts something: the point colours' ranges.
    PointColorShift,
}

impl Gate {
    /// Every gate, in the catalog's order.
    const ALL: [Gate; 27] = [
        Gate::EffectsOn,
        Gate::OpticsOn,
        Gate::GeometryOn,
        Gate::CalibrationOn,
        Gate::Bw,
        Gate::Color,
        Gate::ProfileLook,
        Gate::LensDistortion,
        Gate::LensVignetting,
        Gate::HighlightPriority,
        Gate::ColorOrBwMix,
        Gate::CurveRegions,
        Gate::CurveOn,
        Gate::GradingZones,
        Gate::ShadowsTint,
        Gate::MidtonesTint,
        Gate::HighlightsTint,
        Gate::GlobalTint,
        Gate::VignetteOn,
        Gate::GrainOn,
        Gate::SharpenOn,
        Gate::NrLuminanceOn,
        Gate::NrColorOn,
        Gate::ManualVignetting,
        Gate::DefringePurple,
        Gate::DefringeGreen,
        Gate::PointColorShift,
    ];

    /// The gate's id in the catalog.
    fn id(self) -> &'static str {
        match self {
            Gate::EffectsOn => "effectsOn",
            Gate::OpticsOn => "opticsOn",
            Gate::GeometryOn => "geometryOn",
            Gate::CalibrationOn => "calibrationOn",
            Gate::Bw => "bw",
            Gate::Color => "color",
            Gate::ProfileLook => "profileLook",
            Gate::LensDistortion => "lensDistortion",
            Gate::LensVignetting => "lensVignetting",
            Gate::HighlightPriority => "highlightPriority",
            Gate::ColorOrBwMix => "colorOrBwMix",
            Gate::CurveRegions => "curveRegions",
            Gate::CurveOn => "curveOn",
            Gate::GradingZones => "gradingZones",
            Gate::ShadowsTint => "shadowsTint",
            Gate::MidtonesTint => "midtonesTint",
            Gate::HighlightsTint => "highlightsTint",
            Gate::GlobalTint => "globalTint",
            Gate::VignetteOn => "vignetteOn",
            Gate::GrainOn => "grainOn",
            Gate::SharpenOn => "sharpenOn",
            Gate::NrLuminanceOn => "nrLuminanceOn",
            Gate::NrColorOn => "nrColorOn",
            Gate::ManualVignetting => "manualVignetting",
            Gate::DefringePurple => "defringePurple",
            Gate::DefringeGreen => "defringeGreen",
            Gate::PointColorShift => "pointColorShift",
        }
    }

    /// When the controls behind the gate change nothing, and what opens it.
    fn why(self) -> &'static str {
        match self {
            Gate::EffectsOn => "texture, clarity, dehaze, the vignette and grain render only while the Effects section is switched on",
            Gate::OpticsOn => "the lens corrections, chromatic aberration and defringe render only while the Optics section is switched on",
            Gate::GeometryOn => "the Transform sliders render only while the Geometry section is switched on (the straighten angle is the crop's)",
            Gate::CalibrationOn => "the calibration sliders render only while the Calibration section is switched on",
            Gate::Bw => "the B&W mixes render only in black and white: under the B&W treatment or a B&W profile",
            Gate::Color => "the point colours render only in colour: they change nothing under the B&W treatment or a B&W profile",
            Gate::ProfileLook => "the amount scales the profile's look: under Color, the default profile, there is no look to scale",
            Gate::LensDistortion => "the profile's Distortion renders only with the lens profile on and a distortion correction in the file (DNG)",
            Gate::LensVignetting => "the profile's Vignetting renders only with the lens profile on and a vignetting correction in the file (DNG)",
            Gate::HighlightPriority => "the vignette's Highlights render only on a negative vignette in Highlight Priority, the default style",
            Gate::ColorOrBwMix => "Vibrance, Saturation and the mixer's Hue and Saturation render in colour, and in B&W only through a B&W mix",
            Gate::CurveRegions => "the region splits render only while a tone-curve region slider is set (the profile's look's included)",
            Gate::CurveOn => "Refine Saturation renders only while a tone curve changes tones: a region slider set or a point curve bent",
            Gate::GradingZones => "Blending and Balance render only while the Shadows, Midtones or Highlights wheel (or the look's) is set",
            Gate::ShadowsTint => "the Shadows wheel's Hue renders only while its Saturation is not 0",
            Gate::MidtonesTint => "the Midtones wheel's Hue renders only while its Saturation is not 0",
            Gate::HighlightsTint => "the Highlights wheel's Hue renders only while its Saturation is not 0",
            Gate::GlobalTint => "the Global wheel's Hue renders only while its Saturation is not 0",
            Gate::VignetteOn => "the vignette's Midpoint, Roundness and Feather render only while its amount is not 0",
            Gate::GrainOn => "the grain's Size and Roughness render only while its amount is not 0",
            Gate::SharpenOn => "Radius, Detail and Masking render only while Sharpening is not 0 or a mask sharpens",
            Gate::NrLuminanceOn => "the noise reduction's Detail and Contrast render only while Noise Reduction is not 0",
            Gate::NrColorOn => "the colour noise reduction's Detail and Smoothness render only while Color Noise Reduction is not 0",
            Gate::ManualVignetting => "the lens Vignetting's Midpoint renders only while that Vignetting is not 0",
            Gate::DefringePurple => "the purple Hue Low and High render only while the purple defringe amount is not 0",
            Gate::DefringeGreen => "the green Hue Low and High render only while the green defringe amount is not 0",
            Gate::PointColorShift => "a point colour's ranges render only for a sample that shifts its hue, saturation, luminance or variance",
        }
    }

    /// Whether settings `d` open the gate, for a photo whose file embeds the lens corrections `lens`
    /// (`Session::source_info`).
    fn open(self, d: &DevelopSettings, lens: Option<&EmbeddedLens>) -> bool {
        match self {
            Gate::EffectsOn => d.section_enabled("effects"),
            Gate::OpticsOn => d.section_enabled("optics"),
            Gate::GeometryOn => d.section_enabled("geometry"),
            Gate::CalibrationOn => d.section_enabled("calibration"),
            Gate::Bw => is_bw(d),
            Gate::Color => !is_bw(d),
            Gate::ProfileLook => profiles::LOOK_IDS.contains(&d.profile.id.as_str()) || lut::get(&d.profile.id).is_some(),
            Gate::LensDistortion => d.optics.lens_profile && lens.is_some_and(|lens| lens.warp.is_some()),
            Gate::LensVignetting => d.optics.lens_profile && lens.is_some_and(|lens| lens.vignette.is_some()),
            Gate::HighlightPriority => d.vignette.amount < 0.0 && d.vignette.style == VignetteStyle::HighlightPriority,
            Gate::ColorOrBwMix => !is_bw(d) || profiles::effective(d).bw_mix.bands().iter().any(|weight| *weight != 0.0),
            Gate::CurveRegions => regions_set(&profiles::effective(d).curve),
            Gate::CurveOn => curve_set(&profiles::effective(d).curve),
            Gate::GradingZones => zones_set(&profiles::effective(d).grading),
            Gate::ShadowsTint => d.grading.shadows.sat != 0.0,
            Gate::MidtonesTint => d.grading.midtones.sat != 0.0,
            Gate::HighlightsTint => d.grading.highlights.sat != 0.0,
            Gate::GlobalTint => d.grading.global.sat != 0.0,
            Gate::VignetteOn => d.vignette.amount != 0.0,
            Gate::GrainOn => d.grain.amount > 0.0,
            Gate::SharpenOn => d.detail.sharpen_amount != 0.0 || d.masks.iter().any(|mask| mask.adjust.sharpness != 0.0),
            Gate::NrLuminanceOn => d.detail.nr_luminance > 0.0,
            Gate::NrColorOn => d.detail.nr_color > 0.0,
            Gate::ManualVignetting => d.optics.vignetting != 0.0,
            Gate::DefringePurple => d.optics.defringe_purple_amount > 0.0,
            Gate::DefringeGreen => d.optics.defringe_green_amount > 0.0,
            Gate::PointColorShift => d.point_colors.iter().any(|sample| !sample.is_neutral()),
        }
    }
}

/// The gates control `control` acts behind, in the catalog's order, as the renderer reads the
/// settings: the controls of the effects (with the vignette and grain), optics, geometry and
/// calibration sections behind their switches (`finish.rs:226-253`, `optics.rs:74`, `geometry.rs:271`;
/// the straighten angle, `crop.angle`, is the crop's); the B&W mixes under B&W and the point colours
/// in colour (`crates/pipeline/src/lib.rs:481-483`, `colorops.rs:166-171`); the profile's amount
/// under a look (`profiles.rs:45-50`, `finish.rs:249`); the lens profile sliders behind the
/// correction the file embeds (`optics.rs:82-88, 105, 116`); and the vignette's highlights behind a
/// negative vignette in Highlight Priority (`finish.rs:87-101, 538`). And the controls that act only
/// through another setting: Vibrance, Saturation and the mixer's Hue and Saturation change a colour's
/// chroma and hue, which black and white reads only through the B&W mix (`colorops.rs:241-246`); the
/// region splits place the curve's regions, and Refine Saturation tempers what the curves do
/// (`finish.rs:40-55, 574-579`); Blending and Balance share the tones among the zone wheels, and a
/// wheel's Hue turns its tint, which its Saturation scales (`colorops.rs:149-152, 252-264`); the
/// vignette's shape and the grain's follow their amounts (`finish.rs:87-89, 234`); Radius, Detail and
/// Masking shape the sharpening that the amount or a mask's sharpness turns on (`finish.rs:473-484`,
/// `local.rs:297-299`); each noise reduction's sliders follow its amount (`local.rs:152-161`); the
/// lens Vignetting's Midpoint shapes that correction (`optics.rs:80, 187-190`); each defringe hue range
/// picks what its amount removes (`optics.rs:517, 535-538`); and a point colour's ranges select what
/// its shifts move, a sample that shifts nothing being left out (`colorops.rs:166-169`).
fn gates_of(control: &str) -> Vec<Gate> {
    let family = control.split_once('.').map_or(control, |(family, _)| family);
    let field = control.rsplit_once('.').map_or(control, |(_, field)| field);
    match control {
        "profile.amount" => vec![Gate::ProfileLook],
        "vignette.highlights" => vec![Gate::EffectsOn, Gate::HighlightPriority],
        "vignette.midpoint" | "vignette.roundness" | "vignette.feather" => vec![Gate::EffectsOn, Gate::VignetteOn],
        "grain.size" | "grain.roughness" => vec![Gate::EffectsOn, Gate::GrainOn],
        "optics.profileDistortion" => vec![Gate::OpticsOn, Gate::LensDistortion],
        "optics.profileVignetting" => vec![Gate::OpticsOn, Gate::LensVignetting],
        "optics.vignettingMidpoint" => vec![Gate::OpticsOn, Gate::ManualVignetting],
        "optics.defringePurpleHueLo" | "optics.defringePurpleHueHi" => vec![Gate::OpticsOn, Gate::DefringePurple],
        "optics.defringeGreenHueLo" | "optics.defringeGreenHueHi" => vec![Gate::OpticsOn, Gate::DefringeGreen],
        "color.vibrance" | "color.saturation" => vec![Gate::ColorOrBwMix],
        "curve.splitShadows" | "curve.splitMid" | "curve.splitHighlights" => vec![Gate::CurveRegions],
        "curve.refineSaturation" => vec![Gate::CurveOn],
        "grading.blending" | "grading.balance" => vec![Gate::GradingZones],
        "grading.shadows.hue" => vec![Gate::ShadowsTint],
        "grading.midtones.hue" => vec![Gate::MidtonesTint],
        "grading.highlights.hue" => vec![Gate::HighlightsTint],
        "grading.global.hue" => vec![Gate::GlobalTint],
        "detail.sharpenRadius" | "detail.sharpenDetail" | "detail.sharpenMasking" => vec![Gate::SharpenOn],
        "detail.nrDetail" | "detail.nrContrast" => vec![Gate::NrLuminanceOn],
        "detail.nrColorDetail" | "detail.nrColorSmoothness" => vec![Gate::NrColorOn],
        _ => match (family, field) {
            ("mixer", "hue" | "sat") => vec![Gate::ColorOrBwMix],
            ("effects" | "vignette" | "grain", _) => vec![Gate::EffectsOn],
            ("optics", _) => vec![Gate::OpticsOn],
            ("geometry", _) => vec![Gate::GeometryOn],
            ("calibration", _) => vec![Gate::CalibrationOn],
            ("bw", _) => vec![Gate::Bw],
            ("pointColor", "range" | "hueRange" | "satRange" | "lumRange") => vec![Gate::Color, Gate::PointColorShift],
            ("pointColor", _) => vec![Gate::Color],
            _ => Vec::new(),
        },
    }
}

/// Whether settings `d` render in black and white, as the renderer decides it (`is_bw`,
/// `crates/pipeline/src/lib.rs:481-483`): the B&W treatment, or a B&W profile.
fn is_bw(d: &DevelopSettings) -> bool {
    d.treatment == Treatment::Bw || d.profile.id == "lc.mono" || d.profile.id.starts_with("lc.bw.")
}

/// Whether a region slider of tone curve `c` is set: the regions the splits place (`curve_luts`,
/// `crates/pipeline/src/finish.rs:40-55`; a region at 0 is skipped).
fn regions_set(c: &ToneCurve) -> bool {
    [c.highlights, c.lights, c.darks, c.shadows].iter().any(|amount| *amount != 0.0)
}

/// Whether tone curve `c` changes tones as the renderer builds the curves: a region slider set, or a
/// point curve that is not straight (`curve_luts`, `finish.rs:40-46`), which Refine Saturation then
/// tempers (`:574-579`).
fn curve_set(c: &ToneCurve) -> bool {
    regions_set(c) || [&c.master, &c.red, &c.green, &c.blue].into_iter().any(|points| !ToneCurve::point_curve_is_identity(points))
}

/// Whether a Shadows, Midtones or Highlights wheel of `g` has saturation or luminance: the wheels
/// Blending and Balance share the tones among (`ColorOps::grade`, `crates/pipeline/src/colorops.rs:252-264`;
/// the Global wheel tints every tone alike).
fn zones_set(g: &ColorGrading) -> bool {
    [g.shadows, g.midtones, g.highlights].iter().any(|wheel| wheel.sat != 0.0 || wheel.lum != 0.0)
}

/// `lts.preview {id, longEdge, variant?, values?, settings?, preset?, crop?, overlay?, pixels?}`: the
/// engine's render of photo `id` fitting `longEdge` × `longEdge` (16 to the runtime's
/// [`Runtime::preview_max`]), as the loupe and exports render it: `current` (its settings, the
/// default), `before` (the before/after "before" side) or `settings` (its settings with `values` set
/// as `develop.set` sets them, complete `settings`, a `settings` patch of sections, or a `preset`
/// applied: what `lts.develop` would commit, held to the same bounds); the crop `applied` (the
/// default) or the whole photo (`uncropped`); optionally drawn over as the editor's diagnostic views
/// draw it (`overlay`, [`overlay_param`]): a mask's view (white on black by default) with the mask's
/// coverage, or Visualize Spots, held at the crop's magnification ([`spots_view_in_range`]). Answers
/// the PNG (unless `pixels: false`), the photo's settings token (`hash`) and the rendered one
/// (`renderedHash`), and the histogram and statistics of the very pixels answered.
fn preview(s: &mut Session, q: &Params<'_>, runtime: Runtime) -> Result<Map<String, Value>, Refusal> {
    let id = q.target()?;
    let long = q.long_edge()?;
    let variant = q.choice("variant", &["current", "before", "settings"])?.unwrap_or("current");
    let change = q.change()?;
    if (variant == "settings") != change.is_some() {
        return Err(invalid(
            "variant `settings` takes `values`, `settings` (complete, or a patch of sections) or a `preset`; the other variants take none",
        ));
    }
    let crop = q.choice("crop", &["applied", "uncropped"])?.unwrap_or("applied");
    let overlay = q.get("overlay").map(overlay_param).transpose()?;
    let pixels = q.flag("pixels", true)?;
    let long = within(long, runtime.preview_max())? as usize;
    let current = settings_of(s, id)?;
    let shown = match change {
        Some(change) => Arc::new(apply(s, id, &current, change)?.0),
        None if variant == "before" => {
            let photo = s.catalog.photo(id).ok_or_else(|| not_found(id))?;
            Arc::new(s.before_settings(photo))
        }
        None => Arc::clone(&current),
    };
    let apply_crop = crop == "applied";
    let overlay = overlay.map(|asked| drawn(s, id, &shown, asked, apply_crop, long)).transpose()?;
    let job_of = |s: &mut Session| match variant {
        "before" => s.render_job(id, long, long, true, apply_crop),
        "settings" => s.preview_job(id, long, long, apply_crop, &shown),
        _ => s.render_job(id, long, long, false, apply_crop),
    };
    let job = job_of(s).ok_or_else(|| not_found(id))?;
    let rendered_hash = hex(job.settings.hash64());
    let job = match overlay {
        Some((_, view)) => job.with_overlay(view),
        None => job,
    };
    let image = render(s, job)?;
    let coverage = match overlay {
        Some((Some(mask), Overlay::Mask { view: MaskView::WhiteOnBlack, .. })) => Some(mask_coverage(mask, &image)),
        Some((Some(mask), _)) => {
            // the coverage is the mask's own, whatever the view: read from its white-on-black view
            let job = job_of(s).ok_or_else(|| not_found(id))?;
            Some(mask_coverage(mask, &render(s, job.with_overlay(mask_overlay(mask, MaskView::WhiteOnBlack)))?))
        }
        _ => None,
    };
    let frame = if apply_crop { "cropped-straightened" } else { "uncropped-oriented" };
    let mut fields = object(json!({
        "photoId": id.0,
        "variant": variant,
        "crop": crop,
        "frame": frame,
        "width": image.width,
        "height": image.height,
        "hash": hex(current.hash64()),
        "renderedHash": rendered_hash,
        "catalogRevision": s.catalog.revision,
    }));
    fields.extend(image_stats(&image));
    if let Some(coverage) = coverage {
        fields.insert("maskCoverage".into(), coverage);
    }
    if pixels {
        let png = encode_png(&EncodeImage::rgba8(&image), &EncodeMeta::default()).map_err(|e| render_failure(&e.to_string()))?;
        fields.insert("mime".into(), json!("image/png"));
        fields.insert("pngBase64".into(), json!(base64(&png)));
    }
    Ok(fields)
}

/// What `overlay` asks [`preview`] to draw ([`overlay_param`]).
enum Asked {
    /// Mask `id`, drawn as a view.
    Mask(u64, MaskView),
    /// Visualize Spots at a threshold from 0 to 100.
    Spots(u8),
}

/// `overlay`: `{maskId, view?}`, a mask drawn as `view`: `whiteOnBlack` (the mask itself, the
/// default), `color`, `colorOnBw`, `imageOnBlack` or `imageOnWhite`; or `{spots: true, threshold?}`,
/// the Remove tool's Visualize Spots (`Overlay::Spots`): the render's specks white on black, from
/// `threshold` 0 (the strongest only) to 100 (the faintest), 50 by default as in the editor.
fn overlay_param(value: &Value) -> Result<Asked, Refusal> {
    if value.get("spots").is_some() {
        let q = Params::of(value, &["spots", "threshold"]).map_err(|_| invalid("`overlay` is {spots: true, threshold?} for Visualize Spots"))?;
        if q.get("spots") != Some(&Value::Bool(true)) {
            return Err(invalid("`overlay.spots` is true: Visualize Spots (no overlay shows the photo)"));
        }
        let threshold = match q.get("threshold") {
            None => SPOTS_THRESHOLD,
            Some(threshold) => threshold
                .as_u64()
                .and_then(|threshold| u8::try_from(threshold).ok())
                .filter(|threshold| *threshold <= 100)
                .ok_or_else(|| invalid("`overlay.threshold` is an integer from 0 to 100"))?,
        };
        return Ok(Asked::Spots(threshold));
    }
    let q = Params::of(value, &["maskId", "view"]).map_err(|_| invalid("`overlay` is {maskId, view?} or {spots: true, threshold?}"))?;
    let mask = q.get("maskId").and_then(Value::as_u64).ok_or_else(|| invalid("`overlay.maskId` is a mask id: an integer"))?;
    let view = match q.get("view") {
        None => MaskView::WhiteOnBlack,
        Some(view) => view
            .as_str()
            .and_then(MaskView::parse)
            .ok_or_else(|| invalid("`overlay.view` is whiteOnBlack, color, colorOnBw, imageOnBlack or imageOnWhite"))?,
    };
    Ok(Asked::Mask(mask, view))
}

/// The overlay `asked` over a render of photo `id` with settings `d`, its crop `applied` or not,
/// fitting `long` × `long`, and the mask whose coverage the reply adds: a mask of the settings
/// shown ([`mask_shown`]), or Visualize Spots within its bound ([`spots_view_in_range`]).
fn drawn(s: &Session, id: PhotoId, d: &DevelopSettings, asked: Asked, applied: bool, long: usize) -> Result<(Option<u16>, Overlay), Refusal> {
    match asked {
        Asked::Mask(mask, view) => mask_shown(d, id, mask, view).map(|(mask, overlay)| (Some(mask), overlay)),
        Asked::Spots(threshold) => spots_view_in_range(s, id, d, applied, long).map(|()| (None, Overlay::Spots(threshold))),
    }
}

/// Visualize Spots of photo `id` under settings `d`, its crop `applied` or not, in a preview that
/// fits `long` × `long`, held where the renderer may magnify the photo the most ([`framed`]): the
/// view's blur ([`SPOTS_VIEW_BLUR`]) no wider than [`SIGMA_MAX`] (`out_of_range`). In a preview 2048
/// long it binds only where the crop shown is under 0.2 % of the photo, which the hook never makes.
fn spots_view_in_range(s: &Session, id: PhotoId, d: &DevelopSettings, applied: bool, long: usize) -> Result<(), Refusal> {
    let (frame, [least, _]) = framed(s, id, d, applied);
    let sigma = SPOTS_VIEW_BLUR * px_per_long_at(&frame, least, long);
    if sigma <= SIGMA_MAX {
        return Ok(());
    }
    let note = if applied && solved_upright(d) { SOLVED_UPRIGHT } else { "" };
    let message = format!("Visualize Spots would blur by {sigma:.0} px in this {long}-pixel preview, past {SIGMA_MAX}: ask a smaller one");
    Err(Refusal::new("out_of_range", message).noting(note))
}

/// The engine's view of mask `id`: white on black at full strength (the mask itself), the colour
/// views in the editor's default overlay colour.
fn mask_overlay(id: u16, view: MaskView) -> Overlay {
    match view {
        MaskView::WhiteOnBlack => Overlay::Mask { id, view, color: [255, 255, 255], opacity: 100 },
        _ => Overlay::Mask { id, view, color: OVERLAY_COLOR, opacity: OVERLAY_OPACITY },
    }
}

/// Mask `mask` drawn as `view`, when the settings shown have that mask (with something to show): an
/// unknown mask is refused rather than drawn as the plain photo.
fn mask_shown(shown: &DevelopSettings, id: PhotoId, mask: u64, view: MaskView) -> Result<(u16, Overlay), Refusal> {
    u16::try_from(mask)
        .ok()
        .map(|mask| (mask, mask_overlay(mask, view)))
        .filter(|(_, overlay)| overlay.mask(shown).is_some())
        .ok_or_else(|| Refusal::new("target_not_found", format!("photo {} has no mask {mask} to show", id.0)))
}

/// Runs `job` here and now (a panic in it becomes a failed read, as the engine's guard turns one in a
/// command into its error), keeping the source it decoded: the one cache a read may fill.
fn render(s: &mut Session, job: RenderJob) -> Result<Rgba8, Refusal> {
    let result = guard::catch("lts render", || job.run()).map_err(|e| render_failure(&e))?;
    s.accept(&result);
    result.rendered.map(|rendered| rendered.image).map_err(|e| render_failure(&e))
}

/// A render or an encoding that failed: the original still loading from browser storage (the page's
/// store words a miss that way, see `store.rs`; asking again shortly works) or a source that could
/// not be decoded.
fn render_failure(error: &str) -> Refusal {
    if error.contains("still loading") {
        Refusal::new("source_loading", format!("the photo's original is still loading: {}", clip(error)))
    } else {
        Refusal::new("decode_failed", format!("the photo could not be rendered: {}", clip(error)))
    }
}

/// The engine's histogram of `image` (what the Histogram panel shows) in 64 bins of 4 levels, the
/// pixels it counted and the statistics read from it.
fn image_stats(image: &Rgba8) -> Map<String, Value> {
    let histogram = Histogram::of_srgb8(image);
    let (clip_lo, clip_hi) = histogram.clipping();
    object(json!({
        "histogram": {
            "r": bins(&histogram.r),
            "g": bins(&histogram.g),
            "b": bins(&histogram.b),
            "luma": bins(&histogram.luma),
        },
        "total": histogram.total,
        "stats": {
            "meanLuma": f64::from(histogram.mean_luma()),
            "p01": f64::from(histogram.percentile(0.01)),
            "p50": f64::from(histogram.percentile(0.5)),
            "p99": f64::from(histogram.percentile(0.99)),
            "clipLo": f64::from(clip_lo),
            "clipHi": f64::from(clip_hi),
        },
    }))
}

/// 256 histogram levels as 64 bins of 4.
fn bins(levels: &[u32]) -> Vec<u32> {
    levels.chunks(4).map(|four| four.iter().sum::<u32>()).collect()
}

/// Mask `mask`'s coverage in its white-on-black view `image`, in preview pixels: `area`, the mean
/// grey level (the fraction of the image covered); `bbox`, the box of the pixels at least half covered
/// (128 and up); `centroid`, the grey-weighted centre (pixel centres at +0.5).
fn mask_coverage(mask: u16, image: &Rgba8) -> Value {
    let width = image.width.max(1);
    let (mut sum, mut sum_x, mut sum_y) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut corners: Option<[usize; 4]> = None;
    for (i, pixel) in image.data.iter().enumerate() {
        let (x, y) = (i % width, i / width);
        let level = pixel[0];
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
    let area = sum / (255.0 * image.width.saturating_mul(image.height).max(1) as f64);
    let bbox = corners.map(|[x0, y0, x1, y1]| json!({"x": x0, "y": y0, "width": x1 - x0 + 1, "height": y1 - y0 + 1}));
    let centroid = (sum > 0.0).then(|| json!({"x": sum_x / sum, "y": sum_y / sum}));
    json!({"maskId": mask, "area": area, "bbox": bbox, "centroid": centroid})
}

/// Standard base64 (RFC 4648 section 4: `+`, `/`, `=` padding, no line breaks).
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3).saturating_mul(4));
    for chunk in bytes.chunks(3) {
        let byte = |i: usize| u32::from(chunk.get(i).copied().unwrap_or(0));
        let group = (byte(0) << 16) | (byte(1) << 8) | byte(2);
        for (i, shift) in [18_u32, 12, 6, 0].into_iter().enumerate() {
            let symbol = ALPHABET.get(((group >> shift) & 0x3f) as usize).copied().unwrap_or(b'=');
            out.push(if i <= chunk.len() { char::from(symbol) } else { '=' });
        }
    }
    out
}

/// `lts.export {id, format, longEdge, quality?, metadata?, removeLocation?}`: photo `id` as a JPEG or
/// PNG with a long edge of `longEdge` (16 to the runtime's [`Runtime::export_max`], never enlarged),
/// embedding what the metadata policy names (`copyright`, the default (owner decision OC-13), `none`
/// or `allExceptCamera`; never the camera), without the location unless `removeLocation` is false. A
/// pure output read: nothing is selected, written or remembered (`last_export` stays), and nothing
/// is decoded into the caches.
fn export(s: &mut Session, q: &Params<'_>, runtime: Runtime) -> Result<Map<String, Value>, Refusal> {
    let id = q.target()?;
    let format = q.choice("format", &["jpeg", "png"])?.ok_or_else(|| invalid("`format` is required: jpeg or png"))?;
    let long = q.long_edge()?;
    let quality = match q.get("quality") {
        None => 90,
        Some(quality) => quality
            .as_u64()
            .and_then(|quality| u8::try_from(quality).ok())
            .filter(|quality| (1..=100).contains(quality))
            .ok_or_else(|| invalid("`quality` is an integer from 1 to 100"))?,
    };
    let metadata = q.choice("metadata", &["none", "copyright", "allExceptCamera"])?.unwrap_or("copyright");
    let remove_location = q.flag("removeLocation", true)?;
    let long = within(long, runtime.export_max())?;
    let current = settings_of(s, id)?;
    let options = ExportOptions {
        format: if format == "png" { ExportFormat::Png } else { ExportFormat::Jpeg },
        quality,
        resize: Some(Resize::long_edge(long)),
        metadata: match metadata {
            "none" => MetadataPolicy::None,
            "allExceptCamera" => MetadataPolicy::AllExceptCamera,
            _ => MetadataPolicy::Copyright,
        },
        remove_location,
        ..ExportOptions::default()
    };
    // the engine's export as `app.export` prepares it; the item's own run yields the bytes the batch
    // would hand to a writer, so no destination is formed and no file is touched
    let prepared = prepare_batch(s, &[id], &options).map_err(|e| render_failure(&e))?;
    let item = prepared.into_iter().next().ok_or_else(|| not_found(id))?;
    let exported = guard::catch("lts export", || item.run()).map_err(|e| render_failure(&e))?.map_err(|e| render_failure(&e))?;
    let mime = if format == "png" { "image/png" } else { "image/jpeg" };
    Ok(object(json!({
        "photoId": id.0,
        "format": format,
        "mime": mime,
        "width": exported.width,
        "height": exported.height,
        "byteLength": exported.bytes.len(),
        "metadata": metadata,
        "removeLocation": remove_location,
        "hash": hex(current.hash64()),
        "catalogRevision": s.catalog.revision,
        "bytesBase64": base64(&exported.bytes),
    })))
}

// -------------------------------------------------------------------------------------- mutations

/// A develop edit, decided before anything changes.
struct Edit {
    id: PhotoId,
    /// The undo step's label.
    label: String,
    /// The settings the edit was planned on: it commits only while the photo still has these.
    before: Arc<DevelopSettings>,
    after: DevelopSettings,
    guards: Guards,
    /// Commit the step as `develop.set {ids}` does (a batch of one) rather than as `crop.set` and
    /// `history.restore` do.
    batch: bool,
    /// What the reply adds.
    fields: Map<String, Value>,
}

impl Edit {
    fn verify(&self, s: &Session) -> Result<(), Refusal> {
        verify(s, self.id, Some(&self.before), &self.guards, &s.undo)
    }
}

/// What an edit must find, before it is planned and again inside the engine command that commits
/// it: photo `id` in the library (with exactly the `planned` settings, when given), no human
/// interaction in progress, the settings token `expectHash` names and the head the guards expect
/// on `stack`.
fn verify(s: &Session, id: PhotoId, planned: Option<&Arc<DevelopSettings>>, guards: &Guards, stack: &[UndoEntry]) -> Result<(), Refusal> {
    let current = settings_of(s, id)?;
    if s.interaction.is_some() {
        return Err(busy());
    }
    if let Some(expected) = guards.hash
        && current.hash64() != expected
    {
        let message = format!("photo {}'s settings are not the ones `expectHash` names: refresh (lts.state) and decide again", id.0);
        return Err(Refusal::conflict("settings_hash_conflict", message, json!(hex(current.hash64()))));
    }
    if let Some(head) = &guards.head
        && !head.matches(stack)
    {
        let message = "the head of the history is not the expected one: refresh (lts.state) and decide again";
        return Err(Refusal::conflict("history_head_conflict", message, head_json(stack)));
    }
    if planned.is_some_and(|planned| !Arc::ptr_eq(planned, &current)) {
        let message = format!("photo {}'s settings changed while the edit was prepared", id.0);
        return Err(Refusal::conflict("settings_hash_conflict", message, json!(hex(current.hash64()))));
    }
    Ok(())
}

/// The refusal of a mutation while a person is editing: the hook never ends their interaction.
fn busy() -> Refusal {
    Refusal::new("editor_busy", "a human edit is in progress (a slider or brush drag): try again once it ends")
}

/// Runs `f` as the engine command `command` (`Session::execute_fn`: never journaled) with the
/// library's XMP sidecar auto-write held off, then put back as it was: an `lts.*` edit is saved in
/// the session's library and writes nothing beside the photo's original, wherever it lives.
fn execute_held(s: &mut Session, command: &str, f: impl FnOnce(&mut Session) -> Result<Value, EngineError>) -> Result<Value, EngineError> {
    let auto_write = s.xmp.auto_write;
    s.xmp.auto_write = false;
    let result = s.execute_fn(command, f);
    s.xmp.auto_write = auto_write;
    result
}

/// Commits `edit` in one engine command ([`execute_held`]) that verifies again what the edit was
/// planned on, then answers the tokens after it.
fn commit(s: &mut Session, command: &str, edit: Edit) -> Reply {
    let hash_before = hex(edit.before.hash64());
    let Edit { id, label, before, after, guards, batch, mut fields } = edit;
    let mut refused = None;
    let result = execute_held(s, command, |s| {
        if let Err(refusal) = verify(s, id, Some(&before), &guards, &s.undo) {
            refused = Some(refusal);
            return Ok(Value::Null);
        }
        let op = s.develop_op(id, after, &label).ok_or_else(|| EngineError::Other(format!("photo {} is not in the catalog", id.0)))?;
        s.commit(&label, if batch { Op::Batch { ops: vec![op] } } else { op })?;
        Ok(Value::Null)
    });
    if let Some(refusal) = refused {
        return Reply::Refused(refusal);
    }
    if let Err(error) = result {
        return Reply::Failed(error.to_string());
    }
    fields.extend(object(json!({
        "photoId": id.0,
        "label": label,
        "hashBefore": hash_before,
        "hashAfter": s.develop_of(id).map(|settings| hex(settings.hash64())),
        "catalogRevision": s.catalog.revision,
        "historyHead": head_json(&s.undo),
    })));
    Reply::Applied(fields)
}

/// `lts.develop {id, values | settings | preset, expectHash?, expectHistoryHead?, label?}`: photo
/// `id`'s settings with 1 to 64 control `values` set exactly as `develop.set {ids}` sets them (the
/// as-shot white balance rule included; a value outside its control's range is `out_of_range`),
/// replaced by complete `settings` (a checkpoint, or a sync built from two `develop.get` reads;
/// they require `expectHash`, the token of the settings they replace),
/// with whole sections replaced by a `settings` patch (only [`PATCH_SECTIONS`], each exactly as
/// `develop.get` writes it), or with a `preset` (`{id, amount?}`) applied exactly as `preset.apply
/// {ids}` applies it, the last three held to [`bounded`], as one undo step named `label` (by default
/// the name the engine would give it). Adds each value before and after (`delta`), whether every
/// value is as asked (`intentSatisfied`), whether the white balance mode changed (`wbModeChanged`)
/// and, for a patch, the sections set (`sections`).
fn develop(s: &mut Session, q: &Params<'_>) -> Reply {
    match plan_develop(s, q) {
        Ok(edit) => commit(s, "lts.develop", edit),
        Err(refusal) => Reply::Refused(refusal),
    }
}

fn plan_develop(s: &Session, q: &Params<'_>) -> Result<Edit, Refusal> {
    let id = q.target()?;
    let change =
        q.change()?.ok_or_else(|| invalid("give `values` (control ids and numbers), `settings` (complete, or a patch of sections) or a `preset`"))?;
    let guards = q.guards("expectHistoryHead")?;
    if matches!(change, Change::Settings(_)) && guards.hash.is_none() {
        return Err(invalid("`expectHash` is required with complete settings: the token of the photo's settings they replace (lts.state)"));
    }
    let label = q.label()?;
    let before = settings_of(s, id)?;
    let (after, default_label, fields) = apply(s, id, &before, change)?;
    let edit = Edit { id, label: label.unwrap_or(default_label), before, after, guards, batch: true, fields };
    edit.verify(s)?;
    Ok(edit)
}

/// `lts.crop {id, rect? and/or angle?, reset?, expectHash?, expectHistoryHead?}`: the crop of photo
/// `id` set as `crop.set` sets it (`rect` `[x0, y0, x1, y1]` normalized in the straightened frame,
/// wider and taller than the editor's crop tool keeps a crop, `angle` in degrees within ±45; the
/// crop is kept inside the photo) or reset as `crop.reset` resets it, as one undo step, whichever
/// photo is active. A crop that is tighter than the photo's is held at its magnification
/// ([`magnified`]): the crop the renderer would show, and the photo's own spots and brushes, are
/// judged again. Adds the geometry applied (`crop`) and whether it is the one asked for
/// (`intentSatisfied`).
fn crop(s: &mut Session, q: &Params<'_>) -> Reply {
    match plan_crop(s, q) {
        Ok(edit) => commit(s, "lts.crop", edit),
        Err(refusal) => Reply::Refused(refusal),
    }
}

fn plan_crop(s: &Session, q: &Params<'_>) -> Result<Edit, Refusal> {
    let id = q.target()?;
    let rect = q.get("rect").map(crop_rect).transpose()?;
    let angle = q.get("angle").map(crop_angle).transpose()?;
    let reset = match q.get("reset") {
        None => false,
        Some(Value::Bool(true)) => true,
        Some(_) => return Err(invalid("`reset` is true, alone")),
    };
    if reset == (rect.is_some() || angle.is_some()) {
        return Err(invalid("give `rect` and/or `angle`, or `reset: true` alone"));
    }
    let guards = q.guards("expectHistoryHead")?;
    let before = settings_of(s, id)?;
    let (width, height) = crop_dims(s, id);
    let mut after = (*before).clone();
    let label = if reset {
        after.crop = Default::default();
        "Reset Crop"
    } else {
        if let Some([x0, y0, x1, y1]) = rect {
            let r = &mut after.crop.geometry.rect;
            (r.x0, r.y0, r.x1, r.y1) = (x0, y0, x1, y1);
        }
        if let Some(angle) = angle {
            after.crop.geometry.angle = angle;
        }
        after.crop.geometry = after.crop.geometry.constrained(width, height);
        "Crop"
    };
    magnified(s, id, &before, &after)?;
    let geometry = after.crop.geometry;
    let applied = [geometry.rect.x0, geometry.rect.y0, geometry.rect.x1, geometry.rect.y1];
    let as_asked = reset || (rect.is_none_or(|rect| rect == applied) && angle.is_none_or(|angle| angle == geometry.angle));
    let fields = object(json!({"crop": {"rect": applied, "angle": geometry.angle}, "intentSatisfied": as_asked}));
    let edit = Edit { id, label: label.to_string(), before, after, guards, batch: false, fields };
    edit.verify(s)?;
    Ok(edit)
}

/// `rect`: `[x0, y0, x1, y1]`, four numbers from -1 to 2 (normalized), the corners in either order
/// as `crop.set` takes them, enclosing some area (`invalid_params` otherwise) and wider and taller
/// than [`MIN_CROP`] of the photo (`out_of_range` otherwise).
fn crop_rect(value: &Value) -> Result<[f64; 4], Refusal> {
    let refused = || invalid("`rect` is [x0, y0, x1, y1]: four numbers from -1 to 2 (normalized) enclosing some area");
    let items = value.as_array().filter(|items| items.len() == 4).ok_or_else(refused)?;
    let mut numbers = [0.0_f64; 4];
    for (number, item) in numbers.iter_mut().zip(items) {
        *number = item.as_f64().filter(|n| (RECT_MIN..=RECT_MAX).contains(n)).ok_or_else(refused)?;
    }
    let [a, b, c, d] = numbers;
    let corners = [a.min(c), b.min(d), a.max(c), b.max(d)];
    let [x0, y0, x1, y1] = corners;
    if x0 >= x1 || y0 >= y1 {
        return Err(refused());
    }
    crop_extent(format_args!("rect"), corners)?;
    Ok(corners)
}

/// `angle`: the straighten angle in degrees, -45 to 45 (outside it is `out_of_range`).
fn crop_angle(value: &Value) -> Result<f64, Refusal> {
    let angle = value.as_f64().ok_or_else(|| invalid("`angle` is a number of degrees"))?;
    if (-MAX_ANGLE..=MAX_ANGLE).contains(&angle) {
        Ok(angle)
    } else {
        Err(Refusal::new("out_of_range", format!("`angle` is -{MAX_ANGLE} to {MAX_ANGLE} degrees, not {angle}")))
    }
}

/// Photo `id`'s size in its current orientation, which `crop.set` keeps a crop inside.
fn crop_dims(s: &Session, id: PhotoId) -> (f64, f64) {
    let Some(photo) = s.catalog.photo(id) else { return (3.0, 2.0) };
    let (width, height) = (f64::from(photo.width.max(1)), f64::from(photo.height.max(1)));
    if photo.develop.orientation.swaps_axes() { (height, width) } else { (width, height) }
}

/// `lts.restoreSettings {id, revision, expectHash, expectHistoryHead?, label?}` (master plan v2,
/// critic CC-22): photo `id`'s settings put back to `revision`, the settings token of an earlier
/// state of the photo that its history recorded (a reply's `hashAfter`: Light's revision is its
/// settings digest, design §6.1; the newest history step with that token), exactly as
/// `history.restore {index}` puts a step back, as one undo step named `label` (by default the
/// engine's `History: <step label>`), whichever photo is active. `expectHash` (required) names the
/// settings it replaces; a token the history does not hold is `target_not_found`. The settings put
/// back are the photo's own and are not judged again. Adds the step put back (`restored`:
/// `revision`, `step`, `stepLabel`) and each control value before and after (`delta`).
fn restore(s: &mut Session, q: &Params<'_>) -> Reply {
    match plan_restore(s, q) {
        Ok(edit) => commit(s, "lts.restoreSettings", edit),
        Err(refusal) => Reply::Refused(refusal),
    }
}

fn plan_restore(s: &Session, q: &Params<'_>) -> Result<Edit, Refusal> {
    let id = q.target()?;
    let revision =
        q.get("revision").ok_or_else(|| invalid("`revision` is required: the settings token of the state to put back (a reply's hashAfter)"))?;
    let revision = parse_token(revision, "revision")?;
    if q.get("expectHash").is_none() {
        return Err(invalid("`expectHash` is required: the photo's settings token now (lts.state)"));
    }
    let guards = q.guards("expectHistoryHead")?;
    let label = q.label()?;
    let before = settings_of(s, id)?;
    let photo = s.catalog.photo(id).ok_or_else(|| not_found(id))?;
    let (index, step) = photo.history.iter().enumerate().rev().find(|(_, step)| step.settings.hash64() == revision).ok_or_else(|| {
        Refusal::new("target_not_found", format!("photo {}'s history holds no settings with the token {} (history.list)", id.0, hex(revision)))
    })?;
    let after = (*step.settings).clone();
    let fields = object(json!({
        "restored": {"revision": hex(revision), "step": index, "stepLabel": step.label},
        "delta": control_delta(&before, &after),
        "intentSatisfied": true,
        "wbModeChanged": before.wb.mode != after.wb.mode,
    }));
    let label = label.unwrap_or_else(|| format!("History: {}", step.label));
    let edit = Edit { id, label, before, after, guards, batch: false, fields };
    edit.verify(s)?;
    Ok(edit)
}

/// `lts.select {id, detail?}` (R1): the declared selection the adapter made with `library.select`
/// and `view.detail`, which the LTS presentation no longer runs. Photo `id` becomes the only photo
/// selected and the active one, with no mask or spot picked, exactly as `library.select {ids: [id],
/// active: id}` leaves the library (`crates/engine/src/cmd/library.rs:359-388`; leaving the photo
/// that was active records its automatic version, as around every command); with `detail: true` the
/// editor shows it in the detail view, as `view.detail` does, where the session has the editor's
/// view (the page's; a native worker's has none). One engine command, never journaled, and no undo
/// step (a selection is none). It refuses while a human interaction is in progress, which
/// `library.select` would end (`editor_busy`), and answers the photo, the active photo, the view
/// shown (`view`: `detail`, or null), the photo's token, the catalog revision and the history head.
fn select(s: &mut Session, view: View<'_>, q: &Params<'_>) -> Reply {
    let (id, detail) = match plan_select(s, q) {
        Ok(plan) => plan,
        Err(refusal) => return Reply::Refused(refusal),
    };
    let mut refused = None;
    let result = execute_held(s, "lts.select", |s| {
        if let Err(refusal) = selectable(s, id) {
            refused = Some(refusal);
            return Ok(Value::Null);
        }
        s.selection = Selection::single(id);
        (s.active_mask, s.active_spot) = (None, None);
        Ok(Value::Null)
    });
    if let Some(refusal) = refused {
        return Reply::Refused(refusal);
    }
    if let Err(error) = result {
        return Reply::Failed(error.to_string());
    }
    let shown = view.filter(|_| detail).map(|view| {
        *view = ViewMode::Detail;
        "detail"
    });
    Reply::Applied(object(json!({
        "photoId": id.0,
        "activePhoto": s.active().map(|id| id.0),
        "view": shown,
        "hash": s.develop_of(id).map(|settings| hex(settings.hash64())),
        "catalogRevision": s.catalog.revision,
        "historyHead": head_json(&s.undo),
    })))
}

fn plan_select(s: &Session, q: &Params<'_>) -> Result<(PhotoId, bool), Refusal> {
    let id = q.target()?;
    let detail = q.flag("detail", false)?;
    selectable(s, id)?;
    Ok((id, detail))
}

/// What `lts.select` must find, before and inside its engine command: photo `id` in the library
/// and no human interaction in progress.
fn selectable(s: &Session, id: PhotoId) -> Result<(), Refusal> {
    settings_of(s, id)?;
    if s.interaction.is_some() {
        return Err(busy());
    }
    Ok(())
}

/// The stack `lts.undo` and `lts.redo` move a step from.
#[derive(Clone, Copy)]
enum Stack {
    Undo,
    Redo,
}

impl Stack {
    fn command(self) -> &'static str {
        match self {
            Stack::Undo => "lts.undo",
            Stack::Redo => "lts.redo",
        }
    }

    /// The parameter that names the expected head of this stack.
    fn head_key(self) -> &'static str {
        match self {
            Stack::Undo => "expectHistoryHead",
            Stack::Redo => "expectRedoHead",
        }
    }

    fn entries(self, s: &Session) -> &[UndoEntry] {
        match self {
            Stack::Undo => &s.undo,
            Stack::Redo => &s.redo,
        }
    }
}

/// `lts.undo {id, expectHash, expectHistoryHead}` and `lts.redo {id, expectHash, expectRedoHead}`:
/// the top step of the undo (redo) stack undone (redone), when the stack's head is the one expected,
/// the photo's settings token is `expectHash` and the step changes nothing but that photo's develop
/// settings and history. Answers the step's `label`, the tokens before and after, and both heads.
fn step(s: &mut Session, q: &Params<'_>, stack: Stack) -> Reply {
    let (id, guards) = match plan_step(s, q, stack) {
        Ok(plan) => plan,
        Err(refusal) => return Reply::Refused(refusal),
    };
    let hash_before = s.develop_of(id).map(|settings| hex(settings.hash64()));
    let (mut refused, mut label) = (None, None);
    let result = execute_held(s, stack.command(), |s| {
        if let Err(refusal) = verify_step(s, id, &guards, stack) {
            refused = Some(refusal);
            return Ok(Value::Null);
        }
        label = Some(match stack {
            Stack::Undo => s.undo_step()?,
            Stack::Redo => s.redo_step()?,
        });
        Ok(Value::Null)
    });
    if let Some(refusal) = refused {
        return Reply::Refused(refusal);
    }
    if let Err(error) = result {
        return Reply::Failed(error.to_string());
    }
    Reply::Applied(object(json!({
        "photoId": id.0,
        "label": label,
        "hashBefore": hash_before,
        "hashAfter": s.develop_of(id).map(|settings| hex(settings.hash64())),
        "catalogRevision": s.catalog.revision,
        "historyHead": head_json(&s.undo),
        "redoHead": head_json(&s.redo),
    })))
}

fn plan_step(s: &Session, q: &Params<'_>, stack: Stack) -> Result<(PhotoId, Guards), Refusal> {
    let key = stack.head_key();
    let id = q.target()?;
    let hash = q.get("expectHash").ok_or_else(|| invalid("`expectHash` is required: the photo's settings token (lts.state)"))?;
    let head = q.get(key).ok_or_else(|| invalid(format!("`{key}` is required: {{length, label}} of the stack (lts.state)")))?;
    let guards = Guards { hash: Some(parse_token(hash, "expectHash")?), head: Some(parse_head(head, key)?) };
    verify_step(s, id, &guards, stack)?;
    Ok((id, guards))
}

/// What `lts.undo` or `lts.redo` must find, before and inside its engine command: what an edit must
/// find, and a top step that changes nothing but that photo's develop settings and history.
fn verify_step(s: &Session, id: PhotoId, guards: &Guards, stack: Stack) -> Result<(), Refusal> {
    let entries = stack.entries(s);
    verify(s, id, None, guards, entries)?;
    match entries.last() {
        Some(top) if top.folder.is_none() && only_develop_of(&top.op, id, 0) => Ok(()),
        _ => {
            let message = format!("the step {} would move is not photo {}'s alone", stack.command(), id.0);
            Err(Refusal::conflict("history_head_conflict", message, head_json(entries)))
        }
    }
}

/// Whether `op` changes nothing but photo `id`'s develop settings and history.
fn only_develop_of(op: &Op, id: PhotoId, depth: usize) -> bool {
    match op {
        Op::Batch { ops } => depth < MAX_DEPTH && !ops.is_empty() && ops.iter().all(|op| only_develop_of(op, id, depth + 1)),
        Op::SetDevelop { id: photo, .. } | Op::SetHistory { id: photo, .. } | Op::PushHistory { id: photo, .. } => *photo == id,
        _ => false,
    }
}
