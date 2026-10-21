//! Crop-scaled radii at extreme magnification. A tight crop spreads a small part of the photo over
//! the whole output, so sizes given as fractions of the photo (a spot's radius, defringe's fringe
//! width) grow without bound in output pixels; `crop.set` has no minimum. Defringe's radius
//! overflowed `x + r` in its min/max filter, a heal or remove spot's blur overflowed `box_radii`,
//! and an infinite magnification turned a spot's pixels into NaN or overflowed its sampling. The
//! radii are now capped at what the output can use: no panic, finite pixels, and every
//! magnification past the cap renders alike.

use lightcraft_develop::{DevelopSettings, Spot, SpotMode};
use lightcraft_geom::Point;
use lightcraft_pipeline::geometry::Frame;
use lightcraft_pipeline::{optics, spots};
use lightcraft_raster::Rgb32f;

/// A dark-to-bright edge with a purple fringe on it (linear Rec. 2020).
fn fringed() -> Rgb32f {
    Rgb32f::from_fn(40, 30, |x, _| match x {
        0..=18 => [0.02; 3],
        19 | 20 => [0.3, 0.05, 0.9],
        _ => [0.9; 3],
    })
}

fn finite(img: &Rgb32f) -> bool {
    img.data.iter().flatten().all(|v| v.is_finite())
}

#[test]
fn defringe_at_any_magnification_is_bounded() {
    let mut s = DevelopSettings::default();
    s.optics.defringe_purple_amount = 10.0;
    let at = |scale: f64| {
        let mut img = fringed();
        optics::defringe(&mut img, &s, scale);
        img
    };
    // A radius of 400 px: past both sides of the 40×30 image already.
    let wide = at(100.0);
    assert!(finite(&wide) && wide.data != fringed().data, "fixture: defringe changes the fringe");
    for scale in [100.0, 1.0e6, 1.0e18, 1.0e300, f64::MAX, f64::INFINITY] {
        let img = at(scale);
        assert!(finite(&img), "scale {scale}: non-finite pixels");
        assert_eq!(img.data, wide.data, "scale {scale}: every radius past the image filters alike");
    }
    assert!(finite(&at(f64::NAN)), "a NaN scale");
}

/// A gentle gradient with a dark dot in the middle.
fn spotted() -> Rgb32f {
    let mut img = Rgb32f::from_fn(60, 40, |x, y| [0.1 + x as f32 * 0.005, 0.2 + y as f32 * 0.004, 0.3]);
    for y in 18..22 {
        for x in 28..32 {
            img.set(x, y, [0.0; 3]);
        }
    }
    img
}

#[test]
fn spots_at_any_magnification_are_bounded() {
    let frame = Frame::new(60, 40, &DevelopSettings::default(), true);
    for mode in [SpotMode::Heal, SpotMode::Remove, SpotMode::Clone] {
        let spot = Spot { mode, points: vec![Point::new(0.5, 0.5)], size: 0.05, feather: 50.0, ..Default::default() };
        let at = |ppl: f64| {
            let mut img = spotted();
            spots::apply(&mut img, std::slice::from_ref(&spot), &frame, ppl);
            img
        };
        let ordinary = at(frame.px_per_long(60));
        assert!(finite(&ordinary) && ordinary.data != spotted().data, "{mode:?}: fixture spot changes the image");
        let wide = at(1.0e9);
        for ppl in [1.0e9, 1.0e12, 1.0e30, f64::MAX, f64::INFINITY] {
            let img = at(ppl);
            assert!(finite(&img), "{mode:?} at {ppl} px per long edge: non-finite pixels");
            assert_eq!(img.data, wide.data, "{mode:?} at {ppl}: every spot past the image renders alike");
        }
        assert!(finite(&at(f64::NAN)), "{mode:?}: a NaN magnification");
    }
}
