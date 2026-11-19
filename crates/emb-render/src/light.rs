//! Light as light (P18 view plan, Milestone 2, ADR 0123): two colors mix as
//! the light that they send to the eye, not as the numbers of the screen.
//!
//! The pictures keep 8-bit sRGB colors. A mix changes each color to linear
//! light (a table of the 256 values), mixes, and changes the result back to
//! sRGB (with the 255 midpoints between the sRGB values, so the result is
//! the nearest sRGB value and the same on each system). A pixel that a
//! thread covers fully keeps the color of the thread; the edges of a thread
//! and the shadows change: a dark thread on a light fabric has lighter
//! edges than a mix of the numbers gives.

use std::sync::LazyLock;

use emb_geom::math;

/// The linear value of an sRGB value from 0 to 1.
fn linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        math::pow((c + 0.055) / 1.055, 2.4)
    }
}

/// The linear value of each 8-bit sRGB value.
static TO_LINEAR: LazyLock<[f64; 256]> = LazyLock::new(|| std::array::from_fn(|i| linear(i as f64 / 255.0)));

/// The linear values between two 8-bit sRGB values: `MIDPOINTS[k]` is the
/// value of sRGB `k + 0.5`.
static MIDPOINTS: LazyLock<[f64; 255]> = LazyLock::new(|| std::array::from_fn(|k| linear((k as f64 + 0.5) / 255.0)));

/// The linear light (0 to 1) of an 8-bit sRGB value.
pub fn to_linear(v: u8) -> f64 {
    TO_LINEAR[v as usize]
}

/// The nearest 8-bit sRGB value of linear light `l` (0 to 1).
pub fn to_srgb(l: f64) -> u8 {
    MIDPOINTS.partition_point(|&m| m < l) as u8
}

/// Mixes `top` over `under` with the part `a` (0 to 1) of `top`, in
/// linear light.
pub fn mix(top: u8, under: u8, a: f64) -> u8 {
    let a = a.clamp(0.0, 1.0);
    to_srgb(to_linear(top) * a + to_linear(under) * (1.0 - a))
}

/// Puts `color` (sRGB, 0 to 255) with the coverage `a` over the RGBA pixel
/// `dst` (alpha not multiplied in), in linear light.
pub fn over(dst: &mut [u8], color: [f64; 3], a: f64) {
    let a = a.clamp(0.0, 1.0);
    let color = color.map(|c| c.round().clamp(0.0, 255.0) as u8);
    let da = f64::from(dst[3]) / 255.0;
    let out_a = a + da * (1.0 - a);
    if a >= 1.0 || da <= 0.0 {
        // The color alone: no mix.
        dst[..3].copy_from_slice(&color);
    } else {
        for k in 0..3 {
            let l = (to_linear(color[k]) * a + to_linear(dst[k]) * da * (1.0 - a)) / out_a.max(1e-9);
            dst[k] = to_srgb(l);
        }
    }
    dst[3] = (out_a * 255.0).round() as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_srgb_value_comes_back() {
        for v in 0..=255u8 {
            assert_eq!(to_srgb(to_linear(v)), v);
        }
        assert_eq!(to_srgb(-1.0), 0);
        assert_eq!(to_srgb(2.0), 255);
        // Half the light of white is sRGB 188, not 128.
        assert_eq!(to_srgb(0.5), 188);
    }

    #[test]
    fn a_mix_of_black_and_white_is_half_the_light() {
        assert_eq!(mix(255, 0, 0.5), 188);
        assert_eq!(mix(255, 0, 0.0), 0);
        assert_eq!(mix(255, 0, 1.0), 255);
        assert_eq!(mix(77, 77, 0.3), 77);
    }

    #[test]
    fn over_keeps_full_cover_and_mixes_edges_as_light() {
        // Full cover: the color of the thread, rounded.
        let mut px = [240, 240, 240, 255];
        over(&mut px, [30.4, 60.0, 90.6], 1.0);
        assert_eq!(px, [30, 60, 91, 255]);
        // Half cover of black on white: half the light.
        let mut px = [255, 255, 255, 255];
        over(&mut px, [0.0; 3], 0.5);
        assert_eq!(px, [188, 188, 188, 255]);
        // On a transparent pixel the color stays and the alpha is the cover.
        let mut px = [0, 0, 0, 0];
        over(&mut px, [200.0, 10.0, 10.0], 0.25);
        assert_eq!(px, [200, 10, 10, 64]);
        // Over a half-transparent pixel.
        let mut px = [255, 0, 0, 128];
        over(&mut px, [0.0, 0.0, 255.0], 0.5);
        assert!((191..=192).contains(&px[3]), "{px:?}");
        assert!(px[0] > px[2] / 2 && px[2] > 150, "{px:?}");
    }
}
