//! Detail panel: Sharpening Radius and Detail and Noise Reduction Contrast change the render. They
//! were stored, shown (once Sharpening is above 0) and read from Lightroom XMP, but nothing in the
//! pipeline read them (`develop/src/controls.rs:211, 212, 216`: `finish.rs` reads only the
//! sharpening amount and masking, `local::nr_params` no contrast). Their defaults (Radius 1.0,
//! Detail 25, Contrast 0) must keep rendering exactly the pixels they rendered before, so every
//! existing edit keeps its look: `GOLDEN` holds the hashes of those renders, taken from the
//! pipeline before the controls took effect.

use lightcraft_develop::DevelopSettings;
use lightcraft_pipeline::{RenderRequest, SourceInfo, render};
use lightcraft_raster::Rgba8;

fn rendered(s: &DevelopSettings) -> Rgba8 {
    let src = lightcraft_scenes::demo_library()[0].render(480, 320);
    render(&src, &SourceInfo::default(), s, &RenderRequest::fit(480, 480)).image
}

fn sharpened(amount: f64, radius: f64, detail: f64, masking: f64) -> DevelopSettings {
    let mut s = DevelopSettings::default();
    s.detail.sharpen_amount = amount;
    s.detail.sharpen_radius = radius;
    s.detail.sharpen_detail = detail;
    s.detail.sharpen_masking = masking;
    s
}

fn denoised(luminance: f64, detail: f64, contrast: f64) -> DevelopSettings {
    let mut s = DevelopSettings::default();
    s.detail.nr_luminance = luminance;
    s.detail.nr_detail = detail;
    s.detail.nr_contrast = contrast;
    s
}

/// FNV-1a (64-bit) of the pixels.
fn hash(img: &Rgba8) -> u64 {
    img.data.iter().flatten().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3))
}

/// Mean absolute difference of two renders, 8-bit RGB.
fn distance(a: &Rgba8, b: &Rgba8) -> f64 {
    let sum: u64 = a.data.iter().zip(&b.data).map(|(p, q)| (0..3).map(|c| u64::from(p[c].abs_diff(q[c]))).sum::<u64>()).sum();
    sum as f64 / (3 * a.data.len().max(1)) as f64
}

/// The hashes of the renders at the controls' defaults (Radius 1.0, Detail 25, Contrast 0), by the
/// pipeline before Radius, Detail and Contrast took effect (BF2-L-RED-01, H/logs/fork-light/20261017T170255Z-BF2-L-RED-01).
const GOLDEN: [(&str, u64); 4] = [
    ("sharpen 40", 13145252848777823820),
    ("sharpen 120 masking 30", 3657232105353024028),
    ("nr 50", 5406803947728635863),
    ("nr 50 detail 80", 3673571801985009737),
];

#[test]
fn the_defaults_render_exactly_as_before() {
    let cases = [
        ("sharpen 40", sharpened(40.0, 1.0, 25.0, 0.0)),
        ("sharpen 120 masking 30", sharpened(120.0, 1.0, 25.0, 30.0)),
        ("nr 50", denoised(50.0, 50.0, 0.0)),
        ("nr 50 detail 80", denoised(50.0, 80.0, 0.0)),
    ];
    let got: Vec<(&str, u64)> = cases.iter().map(|(name, s)| (*name, hash(&rendered(s)))).collect();
    assert_eq!(got, GOLDEN.to_vec(), "Radius 1.0, Detail 25 and Contrast 0 must render the same pixels as before");
}

#[test]
fn sharpening_radius_widens_the_sharpened_band() {
    let none = rendered(&DevelopSettings::default());
    let at = |radius: f64| rendered(&sharpened(100.0, radius, 25.0, 0.0));
    let (narrow, normal, wide) = (at(0.5), at(1.0), at(3.0));
    assert!(narrow.data != normal.data && wide.data != normal.data, "Radius changes the render");
    let (n, d, w) = (distance(&narrow, &none), distance(&normal, &none), distance(&wide, &none));
    assert!(n < d && d < w, "a wider radius sharpens a wider band: {n} < {d} < {w}");
}

#[test]
fn sharpening_detail_brings_out_more_detail() {
    let none = rendered(&DevelopSettings::default());
    let at = |detail: f64| rendered(&sharpened(100.0, 1.0, detail, 0.0));
    let (low, normal, high) = (at(0.0), at(25.0), at(100.0));
    assert!(low.data != normal.data && high.data != normal.data, "Detail changes the render");
    let (l, d, h) = (distance(&low, &none), distance(&normal, &none), distance(&high, &none));
    assert!(l < d && d < h, "more Detail, more sharpened detail: {l} < {d} < {h}");
}

#[test]
fn noise_reduction_contrast_keeps_more_of_the_contrast() {
    let none = rendered(&DevelopSettings::default());
    let (flat, kept) = (rendered(&denoised(80.0, 50.0, 0.0)), rendered(&denoised(80.0, 50.0, 100.0)));
    assert!(flat.data != kept.data, "Contrast changes the render");
    assert!(distance(&kept, &none) < distance(&flat, &none), "Contrast 100 keeps the denoised render closer to the original's contrast");
}
