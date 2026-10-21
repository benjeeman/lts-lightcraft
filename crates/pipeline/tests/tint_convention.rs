//! White balance Tint follows the professional convention (Lightroom, Camera Raw, DNG): positive
//! tint renders the photo more magenta, negative more green. The slider's track runs from green
//! (negative) to magenta (positive), the Fluorescent preset (3800 K, +21) takes a fluorescent green
//! cast out, and `crs:Tint` from XMP sidecars and Lightroom settings is read with its sign as is,
//! so all of them need the renderer to agree. Seen from the light: a positive tint names a
//! greener illuminant (above the Planckian locus), which the white balance then neutralises.

use lightcraft_color::cct::{temp_tint_to_xy, xy_to_uv};
use lightcraft_develop::{DevelopSettings, WbMode};
use lightcraft_pipeline::{RenderRequest, SourceInfo, render};
use lightcraft_raster::Rgb32f;

/// Mean 8-bit (R, G, B) of `src` rendered with `s`.
fn mean_rgb(src: &Rgb32f, info: &SourceInfo, s: &DevelopSettings) -> [f64; 3] {
    let img = render(src, info, s, &RenderRequest::fit(src.width, src.height)).image;
    let mut sum = [0.0f64; 3];
    for p in &img.data {
        for (acc, v) in sum.iter_mut().zip(p) {
            *acc += f64::from(*v);
        }
    }
    sum.map(|v| v / img.data.len().max(1) as f64)
}

/// How magenta a colour is: red and blue over green (negative: green).
fn magenta(c: [f64; 3]) -> f64 {
    (c[0] + c[2]) / 2.0 - c[1]
}

fn grey() -> Rgb32f {
    Rgb32f::filled(48, 32, [0.18; 3])
}

#[test]
fn positive_tint_renders_magenta_and_negative_tint_green() {
    let info = SourceInfo::default();
    let at = |tint: f64| {
        let mut s = DevelopSettings::default();
        s.wb.mode = WbMode::Custom;
        s.wb.temp = info.as_shot_temp;
        s.wb.tint = tint;
        mean_rgb(&grey(), &info, &s)
    };
    let neutral = at(0.0);
    assert!(magenta(neutral).abs() < 1.0, "tint 0 keeps grey neutral: {neutral:?}");
    for tint in [20.0, 50.0, 150.0] {
        let c = at(tint);
        assert!(magenta(c) > 1.0, "tint +{tint} must render magenta: {c:?}");
        let c = at(-tint);
        assert!(magenta(c) < -1.0, "tint -{tint} must render green: {c:?}");
    }
    let strong = at(150.0);
    assert!(strong[1] < strong[0] && strong[1] < strong[2], "tint +150: green under both red and blue: {strong:?}");
    assert!(magenta(at(150.0)) > magenta(at(50.0)) && magenta(at(50.0)) > magenta(at(20.0)), "more tint, more magenta");
}

#[test]
fn fluorescent_preset_adds_magenta_to_a_raw() {
    let info = SourceInfo { raw: true, as_shot_temp: 3800.0, as_shot_tint: 0.0, ..SourceInfo::default() };
    let mut s = DevelopSettings::default();
    let shot = mean_rgb(&grey(), &info, &s);
    s.wb.mode = WbMode::Fluorescent;
    let fluorescent = mean_rgb(&grey(), &info, &s);
    assert!(magenta(fluorescent) > magenta(shot) + 0.5, "Fluorescent (3800 K, +21) adds magenta: as shot {shot:?}, preset {fluorescent:?}");
}

#[test]
fn positive_tint_names_a_greener_light() {
    for t in [2850.0, 3800.0, 5500.0, 6500.0, 9000.0] {
        let (_, v0) = xy_to_uv(temp_tint_to_xy(t, 0.0));
        let (_, v1) = xy_to_uv(temp_tint_to_xy(t, 21.0));
        assert!(v1 > v0, "{t} K: the light of tint +21 lies above the locus (greener)");
    }
}

#[test]
fn auto_white_balance_corrects_a_green_cast_with_positive_tint() {
    let cast = Rgb32f::filled(32, 32, [0.16, 0.21, 0.16]);
    let (temp, tint) = lightcraft_pipeline::auto::auto_wb(&cast, &SourceInfo::default());
    assert!(tint > 5.0, "a green cast is taken out with positive (magenta) tint, as in Lightroom: {temp} K, tint {tint}");
    let mut s = DevelopSettings::default();
    s.wb.mode = WbMode::Custom;
    s.wb.temp = temp;
    s.wb.tint = tint;
    let mut out = cast.clone();
    lightcraft_pipeline::local::scene_linear_pre(&mut out, &SourceInfo::default(), &s);
    let c = out.get(0, 0);
    assert!((c[0] - c[1]).abs() < 0.02 && (c[1] - c[2]).abs() < 0.02, "the cast is neutralised: {c:?}");
}
