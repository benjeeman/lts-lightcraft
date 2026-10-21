//! Blurs at extreme sigmas. `box_radii` squared the box width in `i32`, which overflows past a
//! sigma of about 23,170 px: debug builds panicked, release builds went on with a wrapped value.
//! A person reaches such sigmas through crop-scaled blurs (heal and remove spots, brushes and
//! Auto Mask strokes in very tight crops). Now the radii are computed in 64-bit and every box pass
//! is capped at what the image can use, so any sigma, infinite or NaN included, blurs in bounded
//! time and gives finite values; every blur whose boxes fit inside the image stays byte-identical
//! to the pre-fix algorithm, kept below as the reference.

use lightcraft_raster::Plane;
use lightcraft_raster::blur::{BAND, box_radii, gaussian};

/// The pre-fix algorithm, sequential: `box_radii` in `i32` (exact below the overflow) and the
/// three box passes each way with the same running sums, band priming included.
mod reference {
    use super::BAND;

    pub fn box_radii(sigma: f32) -> [usize; 3] {
        let n = 3.0f32;
        let w_ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
        let mut wl = w_ideal.floor() as i32;
        if wl % 2 == 0 {
            wl -= 1;
        }
        let wu = wl + 2;
        let m_ideal = (12.0 * sigma * sigma - n * (wl * wl) as f32 - 4.0 * n * wl as f32 - 3.0 * n) / (-4.0 * wl as f32 - 4.0);
        let m = m_ideal.round() as i32;
        std::array::from_fn(|i| (((if (i as i32) < m { wl } else { wu }) - 1) / 2).max(0) as usize)
    }

    fn madd(a: f32, o: f32, w: f32) -> f32 {
        a + o * w
    }

    fn box_row(row: &mut [f32], r: usize) {
        let w = row.len();
        if r == 0 || w == 0 {
            return;
        }
        let s = row.to_vec();
        let last = w - 1;
        let inv = 1.0 / (2 * r + 1) as f32;
        let mut acc = 0.0f32;
        for i in 0..=2 * r {
            acc = madd(acc, s[i.saturating_sub(r).min(last)], 1.0);
        }
        for (x, out) in row.iter_mut().enumerate() {
            *out = madd(0.0, acc, inv);
            acc = madd(madd(acc, s[(x + r + 1).min(last)], 1.0), s[x.saturating_sub(r)], -1.0);
        }
    }

    fn box_v(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
        let inv = 1.0 / (2 * r + 1) as f32;
        let last = h - 1;
        let row = |y: usize| &src[y * w..(y + 1) * w];
        let mut dst = vec![0.0f32; w * h];
        for (band, out) in dst.chunks_mut(w * BAND).enumerate() {
            let y0 = band * BAND;
            let mut acc = vec![0.0f32; w];
            for i in 0..=2 * r {
                let yy = (y0 + i).saturating_sub(r).min(last);
                for (a, v) in acc.iter_mut().zip(row(yy)) {
                    *a = madd(*a, *v, 1.0);
                }
            }
            for (k, o) in out.chunks_mut(w).enumerate() {
                let y = y0 + k;
                for (d, a) in o.iter_mut().zip(&acc) {
                    *d = madd(0.0, *a, inv);
                }
                let (add, sub) = (row((y + r + 1).min(last)), row(y.saturating_sub(r)));
                for ((a, p), m) in acc.iter_mut().zip(add).zip(sub) {
                    *a = madd(madd(*a, *p, 1.0), *m, -1.0);
                }
            }
        }
        dst
    }

    pub fn gaussian(data: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
        if sigma <= 0.3 || data.is_empty() {
            return data.to_vec();
        }
        let radii = box_radii(sigma);
        let mut a = data.to_vec();
        for row in a.chunks_mut(w) {
            for r in radii {
                box_row(row, r);
            }
        }
        for r in radii {
            if r > 0 {
                a = box_v(&a, w, h, r);
            }
        }
        a
    }
}

/// A deterministic plane of values in 0..1 (splitmix64).
fn noise(w: usize, h: usize, seed: u64) -> Plane {
    let mut s = seed;
    let data = (0..w * h)
        .map(|_| {
            s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            ((z ^ (z >> 31)) >> 40) as f32 / (1u64 << 24) as f32
        })
        .collect();
    Plane { width: w, height: h, data }
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn box_radii_are_unchanged_below_the_old_overflow() {
    let mut sigma = 0.0f32;
    while sigma <= 23_170.0 {
        assert_eq!(box_radii(sigma), reference::box_radii(sigma), "sigma {sigma}");
        sigma += if sigma < 64.0 { 0.013 } else { 0.37 };
    }
    for sigma in [0.3, 0.31, 1.0, 2.5, 100.0, 4096.0, 23_170.0, -1.5, -40.0] {
        assert_eq!(box_radii(sigma), reference::box_radii(sigma), "sigma {sigma}");
    }
}

#[test]
fn box_radii_never_overflow() {
    for sigma in [23_171.0f32, 30_000.0, 46_341.0, 100_000.0, 1.0e6] {
        for r in box_radii(sigma) {
            let r = r as f64;
            assert!((r - f64::from(sigma)).abs() <= 0.01 * f64::from(sigma) + 2.0, "sigma {sigma}: radius {r}");
        }
    }
    let widest = box_radii(1.0e6);
    for sigma in [1.0e9, f32::MAX, f32::INFINITY, -f32::INFINITY] {
        let radii = box_radii(sigma);
        assert!(radii.iter().zip(&widest).all(|(a, b)| a >= b), "sigma {sigma}: {radii:?} narrower than {widest:?}");
    }
    assert_eq!(box_radii(f32::NAN), [0; 3]);
}

#[test]
fn blurs_wider_than_the_image_are_bounded_and_finite() {
    let img = noise(64, 48, 7);
    let (lo, hi) = img.data.iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let wide = gaussian(&img, 500.0);
    for sigma in [500.0, 23_171.0, 1.0e6, 1.0e12, f32::MAX, f32::INFINITY] {
        let b = gaussian(&img, sigma);
        assert_eq!((b.width, b.height), (64, 48), "sigma {sigma}");
        assert!(b.data.iter().all(|v| (lo - 1e-3..=hi + 1e-3).contains(v)), "sigma {sigma}: values outside the input's range");
        assert_eq!(bits(&b.data), bits(&wide.data), "sigma {sigma}: every box past the image's sides blurs alike");
    }
    let nan = gaussian(&img, f32::NAN);
    assert_eq!(bits(&nan.data), bits(&img.data), "a NaN sigma leaves the image as it is");
}

#[test]
fn ordinary_blurs_are_byte_identical_to_the_reference() {
    let common = [0.31f32, 0.6, 1.0, 1.4, 2.0, 3.3, 5.0, 8.0, 10.0];
    for (w, h, extra) in [(97usize, 61usize, &[14.0f32, 20.0, 29.5][..]), (61, 97, &[19.0][..]), (33, 33, &[][..]), (150, 40, &[16.0][..])] {
        let img = noise(w, h, (w * 1000 + h) as u64);
        for &sigma in common.iter().chain(extra) {
            let widest = reference::box_radii(sigma).into_iter().max().unwrap_or(0);
            assert!(widest < w.min(h), "fixture: {w}x{h} sigma {sigma} must be an ordinary blur");
            let got = gaussian(&img, sigma);
            let want = reference::gaussian(&img.data, w, h, sigma);
            assert_eq!(bits(&got.data), bits(&want), "{w}x{h} sigma {sigma}");
        }
    }
}
