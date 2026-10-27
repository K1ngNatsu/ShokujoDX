//! The 3D thread look (P10 Milestone 3): light and shade on each stitch, as
//! if the thread were a round cord that lies on the fabric and goes into
//! it at both ends, the shadow of the thread on the fabric, and fabric
//! textures.
//!
//! The model, in picture coordinates (x to the right, y down, z toward the
//! viewer):
//!
//! - The light comes from a direction (`azimuth_deg`, counterclockwise from
//!   the right of the picture, 135°: top left) and a height above the fabric
//!   (`elevation_deg`).
//! - A point across the thread at `s` (−1 and 1 are the edges, 0 the top)
//!   has the normal of a round cord there. Its brightness is an ambient part
//!   plus the diffuse light; a highlight goes to white where the normal is
//!   half way between the light and the viewer.
//! - Along the thread at `t` (0 and 1 are the needle points) the thread
//!   goes down into the fabric, so it is darker near the ends.
//! - `strength` 0 gives flat colors, 1 the full light and shade.
//!
//! [`ShadeTable`] keeps the shading of the mesh points for 64 directions, so
//! the canvas does no trigonometry for each stitch. The pictures use
//! [`Lighting::shade`] for each pixel, with the twist of the thread plies.

use emb_geom::math;
use emb_model::Rgb;
use serde::{Deserialize, Serialize};

use crate::raster::Picture;

/// Light and shade of the 3D thread look.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lighting {
    /// The direction that the light comes from, degrees counterclockwise
    /// from the right of the picture (135: top left).
    pub azimuth_deg: f64,
    /// The height of the light above the fabric, degrees (90: straight above).
    pub elevation_deg: f64,
    /// 0: flat colors; 1: full light and shade.
    pub strength: f64,
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting {
            azimuth_deg: 135.0,
            elevation_deg: 45.0,
            strength: 1.0,
        }
    }
}

/// The ambient part of the brightness.
const AMBIENT: f64 = 0.68;
/// The diffuse part (a thread top that faces the light at 45° is about 1.0).
const DIFFUSE: f64 = 0.62;
/// The sharpness and the strength of the highlight.
const SHINE_POWER: f64 = 18.0;
const SHINE: f64 = 0.38;
/// The brightness at the needle points, where the thread goes into the fabric.
const END_DARK: f64 = 0.82;

impl Lighting {
    /// The unit vector toward the light.
    fn vector(&self) -> [f64; 3] {
        let (sa, ca) = math::sin_cos_deg(self.azimuth_deg);
        let (se, ce) = math::sin_cos_deg(self.elevation_deg.clamp(5.0, 90.0));
        [ca * ce, -sa * ce, se]
    }

    /// The direction (unit, picture coordinates) that shadows fall: away from the light.
    pub fn shadow_dir(&self) -> (f64, f64) {
        let (sa, ca) = math::sin_cos_deg(self.azimuth_deg);
        (-ca, sa)
    }

    /// How far a shadow falls, as a part of the thread width: long when the
    /// light is low.
    pub fn shadow_length(&self) -> f64 {
        let (se, ce) = math::sin_cos_deg(self.elevation_deg.clamp(5.0, 90.0));
        (0.25 + 0.6 * ce / se.max(0.2)).min(1.6) * self.strength.clamp(0.0, 1.0)
    }

    /// The brightness factor and the highlight (0 to 1) at the point `s`
    /// across and `t` along a stitch whose direction is the unit vector `u`
    /// (picture coordinates). `s` is positive on the left of the direction
    /// as the picture shows it (the side of `(u.y, -u.x)`).
    pub fn shade(&self, u: (f64, f64), s: f64, t: f64) -> (f64, f64) {
        let s = s.clamp(-1.0, 1.0);
        let (nx, ny) = (u.1, -u.0);
        let z = (1.0 - s * s).max(0.0).sqrt();
        let n = [nx * s, ny * s, z];
        let l = self.vector();
        let diffuse = (n[0] * l[0] + n[1] * l[1] + n[2] * l[2]).max(0.0);
        let h = {
            let v = [l[0], l[1], l[2] + 1.0];
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
            [v[0] / len, v[1] / len, v[2] / len]
        };
        let nh = (n[0] * h[0] + n[1] * h[1] + n[2] * h[2]).max(0.0);
        let along = END_DARK + (1.0 - END_DARK) * math::sin(std::f64::consts::PI * t.clamp(0.0, 1.0));
        let factor = (AMBIENT + DIFFUSE * diffuse) * along;
        let shine = SHINE * math::pow(nh, SHINE_POWER) * along;
        let k = self.strength.clamp(0.0, 1.0);
        (1.0 + (factor - 1.0) * k, shine * k)
    }
}

/// A color with a brightness factor and a highlight (see [`Lighting::shade`]).
pub fn lit(c: Rgb, (factor, shine): (f64, f64)) -> [u8; 3] {
    [c.0, c.1, c.2].map(|v| {
        let v = f64::from(v) * factor;
        (v + (255.0 - v).max(0.0) * shine).round().clamp(0.0, 255.0) as u8
    })
}

/// The points across a stitch where the mesh has vertices (`s`).
pub const ACROSS: [f64; 5] = [-1.0, -0.55, 0.0, 0.55, 1.0];
/// The points along a stitch where the mesh has vertices (`t`).
pub const ALONG: [f64; 3] = [0.0, 0.5, 1.0];
/// Directions in the table.
const DIRECTIONS: usize = 64;

/// The shading of the mesh points ([`ACROSS`] × [`ALONG`]) for 64 stitch
/// directions, made once for a lighting.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadeTable {
    pub lighting: Lighting,
    data: Vec<[(f32, f32); ACROSS.len() * ALONG.len()]>,
}

impl ShadeTable {
    pub fn new(lighting: Lighting) -> ShadeTable {
        let data = (0..DIRECTIONS)
            .map(|k| {
                let a = 360.0 * k as f64 / DIRECTIONS as f64;
                let (sy, cx) = math::sin_cos_deg(a);
                let mut cell = [(0.0f32, 0.0f32); ACROSS.len() * ALONG.len()];
                for (r, &t) in ALONG.iter().enumerate() {
                    for (c, &s) in ACROSS.iter().enumerate() {
                        let (f, h) = lighting.shade((cx, sy), s, t);
                        cell[r * ACROSS.len() + c] = (f as f32, h as f32);
                    }
                }
                cell
            })
            .collect();
        ShadeTable { lighting, data }
    }

    /// The shading of the mesh points of a stitch in direction (`dx`, `dy`)
    /// (any length), row by row along the stitch.
    pub fn get(&self, dx: f32, dy: f32) -> &[(f32, f32); ACROSS.len() * ALONG.len()] {
        &self.data[self.index(dx, dy)]
    }

    /// The shading of direction number `k` (see [`ShadeTable::index`]).
    pub fn cell(&self, k: usize) -> &[(f32, f32); ACROSS.len() * ALONG.len()] {
        &self.data[k % DIRECTIONS]
    }

    /// The number of directions in the table.
    pub const DIRECTIONS: usize = DIRECTIONS;

    /// The number of the table direction nearest to (`dx`, `dy`).
    pub fn index(&self, dx: f32, dy: f32) -> usize {
        // The octant and the ratio give the angle with no trigonometry.
        let (ax, ay) = (dx.abs(), dy.abs());
        let r = if ax >= ay {
            ay / ax.max(1e-12)
        } else {
            ax / ay.max(1e-12)
        };
        // atan(r) for r in [0, 1], in eighths of a turn (good to 0.3°).
        let e = r + 0.3476 * r * (1.0 - r);
        let oct = if ax >= ay { e } else { 2.0 - e };
        let eighths = match (dx >= 0.0, dy >= 0.0) {
            (true, true) => oct,
            (false, true) => 4.0 - oct,
            (false, false) => 4.0 + oct,
            (true, false) => 8.0 - oct,
        };
        (eighths * DIRECTIONS as f32 / 8.0).round() as usize % DIRECTIONS
    }
}

/// A brightness factor and a highlight as an RGB color with alpha, for a
/// mesh vertex: `color` lit with `shade`.
pub fn vertex_color(color: [u8; 3], (factor, shine): (f32, f32)) -> [u8; 3] {
    color.map(|v| {
        let v = f32::from(v) * factor;
        (v + (255.0 - v).max(0.0) * shine).round().clamp(0.0, 255.0) as u8
    })
}

/// The look of a thread plied of two strands: light and dark bands across
/// the thread that turn along it. `along_mm` is the distance from the start
/// of the stitch. Returns a brightness factor near 1.
pub fn ply(along_mm: f64, s: f64, strength: f64) -> f64 {
    const PITCH_MM: f64 = 0.35;
    1.0 + 0.07 * strength.clamp(0.0, 1.0) * math::sin(2.0 * std::f64::consts::PI * (along_mm / PITCH_MM + 0.6 * s))
}

/// The surface of a fabric for the 3D look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Weave {
    /// Plain weave (cotton, silk, caps).
    #[default]
    Plain,
    /// Diagonal lines (denim, canvas).
    Twill,
    /// Rows of small V shapes (T-shirt knit).
    Knit,
    /// A fine grid of cells (piqué).
    Pique,
    /// Loops (towels).
    Terry,
    /// Soft noise (felt, fleece).
    Felt,
    /// Almost smooth with a fine grain (leather, vinyl).
    Smooth,
}

impl Weave {
    /// The surface for a fabric id of `data/fabrics.toml` (plain weave if unknown).
    pub fn for_fabric(id: Option<&str>) -> Weave {
        match id {
            Some("denim") => Weave::Twill,
            Some("knit") => Weave::Knit,
            Some("pique") => Weave::Pique,
            Some("towel") => Weave::Terry,
            Some("fleece" | "felt") => Weave::Felt,
            Some("leather") => Weave::Smooth,
            _ => Weave::Plain,
        }
    }
}

/// The size of a fabric tile, mm. All patterns repeat in it.
pub const TILE_MM: f64 = 4.0;

/// A value from 0 to 1 for a cell of a grid that repeats every `n` cells.
fn hash(x: i64, y: i64, n: i64) -> f64 {
    let (x, y) = (x.rem_euclid(n), y.rem_euclid(n));
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h & 0xFFFF) as f64 / 65535.0
}

/// Smooth noise with cells of `cell_mm` that repeats every [`TILE_MM`].
fn noise(x: f64, y: f64, cell_mm: f64) -> f64 {
    let n = (TILE_MM / cell_mm).round() as i64;
    let (gx, gy) = (x / cell_mm, y / cell_mm);
    let (ix, iy) = (gx.floor() as i64, gy.floor() as i64);
    let (fx, fy) = (gx - gx.floor(), gy - gy.floor());
    let smooth = |f: f64| f * f * (3.0 - 2.0 * f);
    let (sx, sy) = (smooth(fx), smooth(fy));
    let a = hash(ix, iy, n) + (hash(ix + 1, iy, n) - hash(ix, iy, n)) * sx;
    let b = hash(ix, iy + 1, n) + (hash(ix + 1, iy + 1, n) - hash(ix, iy + 1, n)) * sx;
    a + (b - a) * sy
}

/// The brightness factor of the fabric at (`x`, `y`) mm (near 1).
pub fn fabric_value(weave: Weave, x: f64, y: f64) -> f64 {
    use std::f64::consts::PI;
    let wave = |v: f64, period: f64| math::sin(2.0 * PI * v / period);
    match weave {
        Weave::Plain => {
            // Over and under at each crossing: a checker of small bumps.
            let p = 0.5;
            let bump = wave(x, p) * wave(y, p);
            1.0 + 0.07 * bump + 0.03 * (noise(x, y, 0.5) - 0.5)
        }
        Weave::Twill => 1.0 + 0.09 * wave(x + y, 0.8) + 0.03 * wave(y, 0.25) + 0.03 * (noise(x, y, 0.5) - 0.5),
        Weave::Knit => {
            // Columns of V shapes.
            let col = (x / 1.0).rem_euclid(1.0) - 0.5;
            1.0 + 0.09 * wave(y + col.abs() * 1.2, 1.0) - 0.05 * (col * 2.0).powi(2)
        }
        Weave::Pique => 1.0 + 0.08 * (wave(x, 1.0) * wave(y, 1.0)).abs() - 0.04,
        Weave::Terry => {
            let n = noise(x, y, 0.5);
            let loops = (wave(x, 1.0) * wave(y, 1.0) + 1.0) / 2.0;
            0.92 + 0.12 * loops * n + 0.06 * noise(x, y, 0.25)
        }
        Weave::Felt => 0.94 + 0.08 * noise(x, y, 0.5) + 0.04 * noise(x, y, 0.25),
        Weave::Smooth => 0.98 + 0.03 * noise(x, y, 0.25),
    }
}

/// A tile of the fabric of [`TILE_MM`] × [`TILE_MM`] at `px_per_mm`, in the
/// fabric `color`. It repeats with no seam.
pub fn fabric_tile(weave: Weave, color: Rgb, px_per_mm: f64) -> Picture {
    let n = ((TILE_MM * px_per_mm).round() as u32).max(1);
    let step = TILE_MM / f64::from(n);
    let mut rgba = Vec::with_capacity(n as usize * n as usize * 4);
    for py in 0..n {
        for px in 0..n {
            let v = fabric_value(weave, (f64::from(px) + 0.5) * step, (f64::from(py) + 0.5) * step);
            let c = lit(color, (v, 0.0));
            rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    Picture {
        width: n,
        height: n,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_side_toward_the_light_is_brighter() {
        let l = Lighting::default();
        // A stitch from left to right; the light is at the top left, so the
        // top edge (s = 1, the left of the direction) is lit.
        let (top, _) = l.shade((1.0, 0.0), 0.9, 0.5);
        let (bottom, _) = l.shade((1.0, 0.0), -0.9, 0.5);
        assert!(top > bottom + 0.3, "{top} {bottom}");
        // The top of the thread is a little lighter than the flat color (as
        // the middle of the flat thread look).
        let (mid, _) = l.shade((1.0, 0.0), 0.0, 0.5);
        assert!((1.0..1.2).contains(&mid), "{mid}");
        // The ends are darker than the middle.
        assert!(l.shade((1.0, 0.0), 0.0, 0.0).0 < mid * 0.85);
        // There is a highlight on the thread.
        let shine = ACROSS
            .iter()
            .map(|&s| l.shade((1.0, 0.0), s, 0.5).1)
            .fold(0.0, f64::max);
        assert!(shine > 0.05);
        // Strength 0: flat.
        let flat = Lighting { strength: 0.0, ..l };
        assert_eq!(flat.shade((0.6, 0.8), 0.7, 0.1), (1.0, 0.0));
    }

    #[test]
    fn the_table_matches_the_shading() {
        let l = Lighting::default();
        let t = ShadeTable::new(l);
        for deg in [0.0, 17.0, 90.0, 135.0, 200.0, 271.0, 330.0] {
            let (s, c) = math::sin_cos_deg(deg);
            let cell = t.get(c as f32 * 7.0, s as f32 * 7.0);
            for (r, &tt) in ALONG.iter().enumerate() {
                for (k, &ss) in ACROSS.iter().enumerate() {
                    let (f, h) = l.shade((c, s), ss, tt);
                    let (tf, th) = cell[r * ACROSS.len() + k];
                    // One direction step is 5.6°.
                    assert!(
                        (f64::from(tf) - f).abs() < 0.08 && (f64::from(th) - h).abs() < 0.08,
                        "{deg}"
                    );
                }
            }
        }
    }

    #[test]
    fn shadows_fall_away_from_the_light() {
        let l = Lighting::default();
        let (x, y) = l.shadow_dir();
        // Light from the top left: shadows go to the bottom right.
        assert!(x > 0.5 && y > 0.5);
        let low = Lighting {
            elevation_deg: 15.0,
            ..l
        };
        assert!(low.shadow_length() > l.shadow_length());
    }

    #[test]
    fn fabric_tiles_repeat_with_no_seam() {
        for weave in [
            Weave::Plain,
            Weave::Twill,
            Weave::Knit,
            Weave::Pique,
            Weave::Terry,
            Weave::Felt,
            Weave::Smooth,
        ] {
            for (x, y) in [(0.3, 0.7), (1.1, 2.9), (3.95, 0.05)] {
                let a = fabric_value(weave, x, y);
                let b = fabric_value(weave, x + TILE_MM, y - TILE_MM);
                assert!((a - b).abs() < 1e-9, "{weave:?}");
                assert!((0.8..1.2).contains(&a), "{weave:?} {a}");
            }
            let tile = fabric_tile(weave, Rgb(200, 100, 50), 16.0);
            assert_eq!((tile.width, tile.height), (64, 64));
        }
        assert_eq!(Weave::for_fabric(Some("towel")), Weave::Terry);
        assert_eq!(Weave::for_fabric(None), Weave::Plain);
    }
}
