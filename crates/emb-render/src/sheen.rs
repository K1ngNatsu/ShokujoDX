//! The light on the surface (P18 view plan, Milestone 4, ADR 0125): the
//! picture of a [`Surface`] in a light. The shadows of a surface are made
//! with its detail for the light of the surface ([`Surface::light`]); a
//! picture in another light makes them from the heights of its zoom
//! level (less exact at a small zoom).
//!
//! For each pixel, in linear light:
//!
//! - **Diffuse light** from the mean direction of the surface (its normal
//!   is shorter than 1 at a small zoom, so the mean light of the pixels
//!   under it stays the same), with the constants of the 3D thread look of
//!   P10.
//! - **Sheen by the direction of the thread**: a highlight on fibers. A
//!   thread shines where it goes across the light, and the twist of its
//!   plies turns its fibers [`TWIST_DEG`] from the thread, so a satin at an
//!   angle and one at the mirror angle are not equally light. The strength
//!   and the width of the highlight come from the kind of the thread
//!   ([`sheen_of`]). Where threads of more directions mix in a pixel (a
//!   small zoom), the highlight goes toward its mean over all directions:
//!   a satin keeps its sheen, a mixed area is matte.
//! - **Shadows from the heights**: a high part puts a soft shadow on a low
//!   part and on the fabric, as long as the light is low.
//! - **Less light in the gaps** (from the surface) and **the plies** (a
//!   pattern along each thread).
//! - **The bright end**: a tone curve takes a strong highlight toward
//!   white slowly, so it keeps the color of the thread.
//! - **What is not thread** (ADR 0128): a rhinestone is glass, so it has
//!   less diffuse light and a sharp highlight on each facet that faces the
//!   light; metallic thread has small glints of its foil at a large zoom;
//!   a hole in the fabric shows what is behind it ([`BEHIND`]); with the
//!   lights off ([`Lighting::lights_off`]) only glow thread shows.
//!
//! Then the threads mix with the fabric in the part of the pixel that they
//! cover (the light of the fabric: [`crate::fabric`], ADR 0127).

use emb_geom::math;
use emb_model::{Rgb, ThreadKind};

use crate::fabric::FabricLook;
use crate::light::{to_linear, to_srgb};
use crate::raster::Picture;
use crate::surface::{Surface, kind_of};
use crate::thread3d::Lighting;

/// The angle between the fibers on top of a thread and the thread (the
/// twist of its plies), degrees.
pub const TWIST_DEG: f64 = 35.0;
/// How much darker a pixel in full shadow is.
pub const SHADOW_DARK: f64 = 0.45;
/// The light of a part of a thread that faces away from the light, and the
/// diffuse light: a flat top in the default light (45° high) has the color
/// of its thread (0.62 + 0.54 × sin 45° = 1).
const AMBIENT: f64 = 0.62;
const DIFFUSE: f64 = 0.537;
/// How much the plies change the brightness of a thread.
const PLY_LIGHT: f64 = 0.07;
/// From this brightness the tone curve makes the light grow more slowly.
const KNEE: f64 = 1.0;
/// With the lights off, the light that is left (a dark room), as a part of
/// the light.
pub const LIGHTS_OFF: f64 = 0.04;
/// The light of glow thread with the lights off (its own light).
const GLOW_LIGHT: f64 = 0.85;
/// What shows through a hole in the fabric: the dark inside of a garment
/// or the room behind it.
pub const BEHIND: Rgb = Rgb(30, 30, 33);
/// The highlight of a facet of a rhinestone: its strength and its
/// sharpness; the diffuse light of the glass is this part.
const STONE_SHINE: f64 = 1.8;
const STONE_SHARP: f64 = 48.0;
const STONE_DIFFUSE: f64 = 0.55;
/// The foil at the back of a rhinestone: the light from a facet comes from
/// the light on the other side (its direction this part of the facet's), and
/// its strength.
const FOIL: f64 = 0.45;
const FOIL_SHINE: f64 = 0.8;
/// The glints of metallic thread: a place for a glint each this many mm,
/// from this many pixels for each mm (the full glints from 10 more), and
/// their strength and sharpness.
const GLINT_MM: f64 = 0.12;
const GLINT_FROM_PX: f64 = 6.0;
const GLINT: f64 = 1.4;
const GLINT_SHARP: f64 = 120.0;

/// A value from 0 to 1 for the cell (`x`, `y`) and `seed`.
fn hash(x: i64, y: i64, seed: u64) -> f64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ seed.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h & 0xFFFF) as f64 / 65535.0
}

/// The diffuse light of a surface with the normal `n` in the light `l`: a
/// factor of the color of its thread.
pub(crate) fn diffuse(n: [f64; 3], l: [f64; 3]) -> f64 {
    AMBIENT + DIFFUSE * (n[0] * l[0] + n[1] * l[1] + n[2] * l[2]).max(0.0)
}

/// The highlight of a kind of thread: its strength and its sharpness (a
/// power of the sine of the angle between the fiber and the light).
pub fn sheen_of(kind: ThreadKind) -> (f64, f64) {
    match kind {
        ThreadKind::Polyester => (0.30, 6.0),
        ThreadKind::Rayon => (0.40, 8.0),
        ThreadKind::Cotton => (0.03, 2.0),
        ThreadKind::Metallic => (1.20, 20.0),
        ThreadKind::Wool => (0.02, 2.0),
        ThreadKind::Glow => (0.18, 4.0),
    }
}

/// The highlight of a fiber in the direction `angle` (radians, picture
/// coordinates) in the light `l`, seen from above, for the sharpness `p`.
fn lobe(angle: f64, l: [f64; 3], p: f64) -> f64 {
    let tl = math::cos(angle) * l[0] + math::sin(angle) * l[1];
    math::pow((1.0 - tl * tl).max(0.0), p / 2.0)
}

/// The mean of [`lobe`] over all directions.
fn mean_lobe(l: [f64; 3], p: f64) -> f64 {
    const N: usize = 90;
    (0..N)
        .map(|k| lobe(std::f64::consts::PI * k as f64 / N as f64, l, p))
        .sum::<f64>()
        / N as f64
}

/// The relative luminance of a color in linear light.
fn luminance(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

/// The tone curve: above [`KNEE`] the luminance goes toward 1 slowly, and
/// the three channels keep their ratio (the color of the thread).
pub(crate) fn tone(c: [f64; 3]) -> [f64; 3] {
    let y = luminance(c);
    if y <= KNEE {
        return c.map(|v| v.clamp(0.0, 1.0));
    }
    let y2 = KNEE + (1.0 - KNEE) * (1.0 - math::exp(-(y - KNEE) / (1.0 - KNEE)));
    let s = y2 / y;
    c.map(|v| (v * s).clamp(0.0, 1.0))
}

/// The shadow at the pixel (`x`, `y`): the shadow of the surface when its
/// light is `l` (made at the detail and halved), else from the heights of
/// this zoom level.
/// `v` is the direction of the light `l` (`Lighting::vector`).
fn shadow(s: &Surface, x: i64, y: i64, l: &Lighting, v: [f64; 3], top: f64) -> f64 {
    if l.azimuth_deg == s.light.azimuth_deg && l.elevation_deg == s.light.elevation_deg {
        let (t, i) = s.at(x as u32, y as u32);
        return f64::from(t.shadow[i]) / 255.0;
    }
    let at = |x: i64, y: i64| f64::from(s.height(x, y));
    crate::surface::shadow_at(
        &at,
        i64::from(s.width),
        i64::from(s.height),
        x,
        y,
        v,
        l.strength,
        s.px_per_mm,
        top,
    )
}

/// The light of a surface: for each pixel the color in linear light (it
/// can be above 1 in a highlight) and the part that is not transparent,
/// row by row, before the tone curve.
#[derive(Debug, Clone, PartialEq)]
pub struct Light {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<[f32; 4]>,
}

impl Light {
    /// The light with `factor` times fewer pixels on each side: each pixel
    /// is the mean of the pixels under it (the last row and column take the
    /// pixels that are left).
    pub fn smaller(&self, factor: u32) -> Light {
        let f = factor.max(1);
        let (w, h) = (self.width.div_ceil(f), self.height.div_ceil(f));
        let mut rgba = vec![[0.0f32; 4]; (w * h) as usize];
        for y in 0..h {
            for x in 0..w {
                let (mut sum, mut n) = ([0.0f64; 4], 0.0);
                for sy in y * f..((y + 1) * f).min(self.height) {
                    for sx in x * f..((x + 1) * f).min(self.width) {
                        let p = self.rgba[(sy * self.width + sx) as usize];
                        for k in 0..4 {
                            sum[k] += f64::from(p[k]);
                        }
                        n += 1.0;
                    }
                }
                rgba[(y * w + x) as usize] = sum.map(|v| (v / n) as f32);
            }
        }
        Light {
            width: w,
            height: h,
            rgba,
        }
    }

    /// The mean luminance (linear light) of the pixels `x0..x1`, `y0..y1`.
    pub fn mean(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> f64 {
        let (mut sum, mut n) = (0.0, 0.0);
        for y in y0..y1.min(self.height) {
            for x in x0..x1.min(self.width) {
                let p = self.rgba[(y * self.width + x) as usize].map(f64::from);
                sum += luminance([p[0], p[1], p[2]]);
                n += 1.0;
            }
        }
        if n == 0.0 { 0.0 } else { sum / n }
    }

    /// The picture: the tone curve on each pixel, then the colors of the
    /// screen.
    pub fn picture(&self) -> Picture {
        let mut rgba = Vec::with_capacity(self.rgba.len() * 4);
        for p in &self.rgba {
            let c = tone([p[0], p[1], p[2]].map(f64::from));
            rgba.extend_from_slice(&[
                to_srgb(c[0]),
                to_srgb(c[1]),
                to_srgb(c[2]),
                (f64::from(p[3]) * 255.0).round() as u8,
            ]);
        }
        Picture {
            width: self.width,
            height: self.height,
            rgba,
        }
    }
}

/// The picture of a surface in the light `lighting` (see the module text).
/// `fabric`: the color and the fabric under the threads, else a transparent
/// background.
pub fn light(s: &Surface, lighting: &Lighting, fabric: Option<(Rgb, FabricLook)>) -> Picture {
    light_values(s, lighting, fabric).picture()
}

/// As [`light`], the light before the tone curve.
pub fn light_values(s: &Surface, lighting: &Lighting, fabric: Option<(Rgb, FabricLook)>) -> Light {
    let (w, h) = (s.width as usize, s.height as usize);
    let lit = Lit::new(s, lighting);
    // The light of the fabric, and its colors in linear light.
    let lin = |c: Rgb| [c.0, c.1, c.2].map(to_linear);
    let ground = fabric.map(|(c, f)| (f.fabric().shade(lighting), lin(c), f.mix.map(lin)));
    let ground = &ground;
    let size = 1.0 / s.px_per_mm;
    let behind = lit.behind;
    let mut rgba = vec![[0.0f32; 4]; w * h];
    let rows_per = h
        .div_ceil(std::thread::available_parallelism().map_or(1, usize::from))
        .max(1);
    std::thread::scope(|scope| {
        for (band, chunk) in rgba.chunks_mut(rows_per * w).enumerate() {
            let lit = &lit;
            scope.spawn(move || {
                for (r, row) in chunk.chunks_mut(w).enumerate() {
                    let y = band * rows_per + r;
                    for (x, px) in row.iter_mut().enumerate() {
                        let p = lit.at(x, y);
                        let under = ground.as_ref().map(|(shade, c1, c2)| {
                            let (mx, my) = (
                                s.area.min.x + (x as f64 + 0.5) * size,
                                s.area.min.y + (y as f64 + 0.5) * size,
                            );
                            shade.color(*c1, *c2, mx, my, size).map(|v| v * p.fabric)
                        });
                        // The part that no thread covers: a hole (what is
                        // behind) and the fabric.
                        let open = 1.0 - p.cover;
                        let (hole, bare) = (open * p.behind, open * (1.0 - p.behind));
                        let c = p.thread.unwrap_or([0.0; 3]);
                        *px = match under {
                            Some(u) => {
                                let m = |j: usize| (c[j] * p.cover + behind[j] * hole + u[j] * bare) as f32;
                                [m(0), m(1), m(2), 1.0]
                            }
                            None if p.cover > 0.0 => [c[0] as f32, c[1] as f32, c[2] as f32, p.cover as f32],
                            None => [0.0; 4],
                        };
                    }
                }
            });
        }
    });
    Light {
        width: s.width,
        height: s.height,
        rgba,
    }
}

/// The picture of a surface to put over a fabric (the tiles of the canvas,
/// ADR 0126), on this processor: the threads in the part of each pixel
/// that they cover, and a dark layer that gives the fabric under them its
/// gaps and its shadows. The colors are those of the screen, with the
/// alpha multiplied in (premultiplied).
pub fn overlay(s: &Surface, lighting: &Lighting) -> Picture {
    picture_over(&Lit::new(s, lighting))
}

/// The picture of [`overlay`] from the light of each pixel.
fn picture_over(lit: &Lit) -> Picture {
    let s = lit.s;
    let (w, h) = (s.width as usize, s.height as usize);
    let mut rgba = Vec::with_capacity(w * h * 4);
    let behind = lit.behind.map(|v| f64::from(to_srgb(v)));
    for y in 0..h {
        for x in 0..w {
            let p = lit.at(x, y);
            // The part that no thread covers: a hole (what is behind, as it
            // is) and the fabric, whose light is a factor of the colors of
            // the screen (the screen mixes in its colors).
            let open = 1.0 - p.cover;
            let (hole, bare) = (open * p.behind, open * (1.0 - p.behind));
            let keep = bare * math::pow(p.fabric.clamp(0.0, 1.0), 1.0 / 2.2);
            let alpha = 1.0 - keep;
            let thread = match p.thread {
                Some(c) => tone(c).map(|v| f64::from(to_srgb(v)) * p.cover),
                None => [0.0; 3],
            };
            let c = [0, 1, 2].map(|k| thread[k] + behind[k] * hole);
            rgba.extend_from_slice(&[
                c[0].round().min(255.0) as u8,
                c[1].round().min(255.0) as u8,
                c[2].round().min(255.0) as u8,
                (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
            ]);
        }
    }
    Picture {
        width: s.width,
        height: s.height,
        rgba,
    }
}

/// The heights round a surface, for its shadows in a light other than its
/// own (see [`overlay_around`]).
pub struct Around<'a> {
    /// The height at the pixel (`x`, `y`) of the surface, also outside it,
    /// mm.
    pub height: &'a (dyn Fn(i64, i64) -> f64 + Sync),
    /// The pixels that it has: `x0`, `y0`, `x1`, `y1` (the last ones not).
    pub pixels: (i64, i64, i64, i64),
    /// The highest point in them, mm.
    pub top: f64,
}

/// As [`overlay`], in a light other than the light of the surface: the
/// shadows from the heights of this zoom level in `around` (the surface
/// and the tiles round it), the diffuse light from its slopes. A quick
/// picture of a tile in a new light, until the tile is made again (ADR
/// 0126); less exact at a small zoom, as [`light`] in another light.
pub fn overlay_around(s: &Surface, lighting: &Lighting, around: &Around) -> Picture {
    let mut lit = Lit::new(s, lighting);
    lit.around = Some(around);
    lit.same_light = false;
    picture_over(&lit)
}

/// The light of one pixel.
struct Pixel {
    /// The color of the threads (linear light, before the tone curve), if
    /// threads or pieces cover a part of it.
    thread: Option<[f64; 3]>,
    /// The part that they cover.
    cover: f64,
    /// The light of the fabric under them: its gaps and its shadow.
    fabric: f64,
    /// The part of the open part that is a hole in the fabric.
    behind: f64,
}

/// The light of the pixels of a surface (see the module text).
struct Lit<'a> {
    s: &'a Surface,
    lighting: &'a Lighting,
    l: [f64; 3],
    k: f64,
    same_light: bool,
    top: f64,
    /// The mean highlight over all directions, for each kind.
    means: Vec<f64>,
    twist: f64,
    /// The heights round the surface for the shadows in another light.
    around: Option<&'a Around<'a>>,
    /// What shows through a hole (linear light).
    behind: [f64; 3],
    /// The half vector of the light and the eye (straight above).
    half: [f64; 3],
}

impl<'a> Lit<'a> {
    fn new(s: &'a Surface, lighting: &'a Lighting) -> Lit<'a> {
        let l = lighting.vector();
        Lit {
            s,
            lighting,
            l,
            k: lighting.strength.clamp(0.0, 1.0),
            same_light: lighting.azimuth_deg == s.light.azimuth_deg && lighting.elevation_deg == s.light.elevation_deg,
            top: f64::from(s.max_height()),
            means: ThreadKind::ALL.iter().map(|&kd| mean_lobe(l, sheen_of(kd).1)).collect(),
            twist: math::to_radians(TWIST_DEG),
            around: None,
            behind: [BEHIND.0, BEHIND.1, BEHIND.2]
                .map(|c| to_linear(c) * if lighting.lights_off { LIGHTS_OFF } else { 1.0 }),
            half: {
                let v = [l[0], l[1], l[2] + 1.0];
                let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
                v.map(|c| c / len)
            },
        }
    }

    /// The light of the pixel (`x`, `y`).
    fn at(&self, x: usize, y: usize) -> Pixel {
        let (s, k, l) = (self.s, self.k, self.l);
        let (t, i) = s.at(x as u32, y as u32);
        let open = f64::from(t.open[i]) / 255.0;
        let dark = match self.around {
            _ if k <= 0.0 => 0.0,
            Some(a) => crate::surface::shadow_with(
                &|x, y| (a.height)(x, y),
                None,
                a.pixels,
                x as i64,
                y as i64,
                l,
                self.lighting.strength,
                s.px_per_mm,
                a.top,
            ),
            None => shadow(s, x as i64, y as i64, self.lighting, l, self.top),
        };
        let shade = 1.0 + (open * (1.0 - SHADOW_DARK * dark) - 1.0) * k;
        let fabric = if self.same_light {
            1.0 + (f64::from(t.fabric_shade[i]) / 255.0 - 1.0) * k
        } else {
            shade
        };
        let cover = f64::from(t.cover[i]) / 255.0;
        let behind = f64::from(t.behind[i]) / 255.0;
        if cover <= 0.0 {
            return Pixel {
                thread: None,
                cover: 0.0,
                fabric,
                behind,
            };
        }
        // Diffuse light from the mean normal, the plies, the gaps and the
        // shadow.
        let ply = f64::from(t.ply[i]) / 127.0;
        let lit = if self.same_light {
            f64::from(t.diffuse[i]) / 255.0 * 2.0
        } else {
            diffuse(t.normal[i].map(|v| f64::from(v) / 127.0), l)
        };
        let factor = lit * (1.0 + PLY_LIGHT * ply);
        let factor = 1.0 + (factor - 1.0) * k;
        let base = t.color[i].map(to_linear);
        let mut c = base.map(|v| v * factor * shade);
        // A rhinestone: glass, with a sharp highlight on each facet that
        // faces the light.
        if t.material[i] == crate::surface::material::STONE {
            let n = t.normal[i].map(|v| f64::from(v) / 127.0);
            let h = self.half;
            let nh = (n[0] * h[0] + n[1] * h[1] + n[2] * h[2]).max(0.0);
            // The light that the foil at the back of the stone sends back
            // through it: the facets on the far side from the light.
            let back = [-n[0] * FOIL, -n[1] * FOIL, n[2]];
            let bl = (back[0] * back[0] + back[1] * back[1] + back[2] * back[2])
                .sqrt()
                .max(1e-9);
            let bh = ((back[0] * h[0] + back[1] * h[1] + back[2] * h[2]) / bl).max(0.0);
            let shine =
                STONE_SHINE * (math::pow(nh, STONE_SHARP) + FOIL_SHINE * math::pow(bh, STONE_SHARP)) * shade * k;
            c = base.map(|v| v * (STONE_DIFFUSE + (1.0 - STONE_DIFFUSE) * factor) * shade + shine);
        }
        // The sheen of the fibers (a piece of fabric has the diffuse light
        // only).
        if let Some(kind) = kind_of(t.material[i]) {
            let (strength, sharp) = sheen_of(kind);
            let a = t.axis[i].map(|v| f64::from(v) / 127.0);
            let one = (a[0] * a[0] + a[1] * a[1]).sqrt().min(1.0);
            let angle = math::atan2(a[1], a[0]) / 2.0 + self.twist;
            let mean = self.means[ThreadKind::ALL.iter().position(|x| *x == kind).unwrap_or(0)];
            let hl = one * lobe(angle, l, sharp) + (1.0 - one) * mean;
            let mut hl = strength * hl * (0.6 + 0.4 * ply) * shade * k;
            // Metallic thread: glints of its foil, each at a place along the
            // thread and turned a little, so they change with the light.
            if kind == ThreadKind::Metallic && s.px_per_mm >= GLINT_FROM_PX {
                let size = 1.0 / s.px_per_mm;
                let (mx, my) = (
                    s.area.min.x + (x as f64 + 0.5) * size,
                    s.area.min.y + (y as f64 + 0.5) * size,
                );
                let (gx, gy) = ((mx / GLINT_MM).floor() as i64, (my / GLINT_MM).floor() as i64);
                if hash(gx, gy, 1) > 0.55 {
                    let turn = (hash(gx, gy, 2) - 0.5) * 1.2;
                    let fade = ((s.px_per_mm - GLINT_FROM_PX) / 10.0).clamp(0.0, 1.0);
                    hl += GLINT * one * lobe(angle + turn, l, GLINT_SHARP) * shade * k * fade;
                }
            }
            // Metallic thread reflects in its own color.
            let tint = if kind == ThreadKind::Metallic {
                base.map(|v| 0.5 + 0.5 * v)
            } else {
                [1.0; 3]
            };
            for (ch, tn) in c.iter_mut().zip(tint) {
                *ch += hl * tn;
            }
        }
        // The lights off: the little light of a dark room, and glow thread
        // in its own light.
        if self.lighting.lights_off {
            c = c.map(|v| v * LIGHTS_OFF);
            if kind_of(t.material[i]) == Some(ThreadKind::Glow) {
                let m = base[0].max(base[1]).max(base[2]).max(1e-6);
                for (ch, b) in c.iter_mut().zip(base) {
                    *ch += GLOW_LIGHT * b / m;
                }
            }
        }
        Pixel {
            thread: Some(c),
            cover,
            fabric,
            behind,
        }
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::{BBox, Point};

    use super::*;
    use crate::strand::Strand;
    use crate::surface::lay;

    fn strand(a: (f64, f64), b: (f64, f64), kind: ThreadKind) -> Strand {
        Strand {
            a: Point::new(a.0, a.1),
            b: Point::new(b.0, b.1),
            color: Rgb(120, 60, 60),
            kind,
            weight: 40,
            lift_mm: 0.0,
            record: 0,
        }
    }

    fn picture(strands: &[Strand], area: BBox, ppm: f64) -> Picture {
        let s = lay(strands, &[]).surface(area, ppm);
        light(
            &s,
            &Lighting::default(),
            Some((Rgb(250, 250, 250), FabricLook::new(crate::fabric::Weave::Leather))),
        )
    }

    fn red(p: &Picture, x: u32, y: u32) -> u8 {
        p.rgba[((y * p.width + x) * 4) as usize]
    }

    #[test]
    fn the_light_comes_from_the_top_left() {
        let area = BBox::new(Point::new(0.0, -1.0), Point::new(10.0, 1.0));
        let p = picture(&[strand((0.0, 0.0), (10.0, 0.0), ThreadKind::Cotton)], area, 40.0);
        // The thread is from y = 32 to 48 (pixels); its upper edge faces the light.
        assert!(
            red(&p, 200, 35) > red(&p, 200, 45) + 20,
            "{} {}",
            red(&p, 200, 35),
            red(&p, 200, 45)
        );
    }

    #[test]
    fn a_thread_across_the_light_shines_and_matte_thread_does_not() {
        // The light comes from the top left (135°): a thread from the bottom
        // left to the top right goes across it.
        let mean = |kind, a: (f64, f64), b: (f64, f64)| {
            let mut s = Vec::new();
            for k in 0..30 {
                let o = 0.4 * k as f64 - 6.0;
                // Parallel stitches, 0.4 mm apart.
                let n = (a.1 - b.1, b.0 - a.0);
                let len = (n.0 * n.0 + n.1 * n.1).sqrt();
                let (ox, oy) = (n.0 / len * o, n.1 / len * o);
                s.push(strand((a.0 + ox, a.1 + oy), (b.0 + ox, b.1 + oy), kind));
            }
            let p = picture(&s, BBox::new(Point::new(-2.0, -2.0), Point::new(2.0, 2.0)), 16.0);
            p.rgba.chunks(4).map(|c| f64::from(c[1])).sum::<f64>() / (p.rgba.len() / 4) as f64
        };
        // The fibers are TWIST_DEG from the thread: this thread has its
        // fibers across the light.
        let (s, c) = math::sin_cos_deg(-45.0 - TWIST_DEG);
        let fibers_across = ((-8.0 * c, -8.0 * s), (8.0 * c, 8.0 * s));
        let across = mean(ThreadKind::Polyester, (-8.0, 8.0), (8.0, -8.0));
        let toward = mean(ThreadKind::Polyester, (-8.0, -8.0), (8.0, 8.0));
        assert!(across > toward + 15.0, "{across} {toward}");
        let (m_across, m_toward) = (
            mean(ThreadKind::Cotton, (-8.0, 8.0), (8.0, -8.0)),
            mean(ThreadKind::Cotton, (-8.0, -8.0), (8.0, 8.0)),
        );
        assert!((m_across - m_toward).abs() < 4.0, "{m_across} {m_toward}");
        let (a, b) = fibers_across;
        assert!(mean(ThreadKind::Polyester, a, b) > across);
        assert!(mean(ThreadKind::Metallic, a, b) > mean(ThreadKind::Polyester, a, b));
    }

    #[test]
    fn a_high_part_puts_a_shadow_on_the_fabric() {
        // A puff stitch: its shadow falls to the bottom right (away from the
        // light at the top left), as long as it is high.
        let s = Strand {
            lift_mm: crate::strand::FOAM_MM,
            record: 0,
            ..strand((0.0, 0.0), (10.0, 0.0), ThreadKind::Polyester)
        };
        let p = picture(&[s], BBox::new(Point::new(-2.0, -4.0), Point::new(12.0, 4.0)), 16.0);
        // 1 mm and 3 mm below the stitch, and 3 mm above it (in the light).
        let at = |y_mm: f64| red(&p, 5 * 16 + 32, ((y_mm + 4.0) * 16.0) as u32);
        assert!(at(1.0) < at(-3.0) - 30, "{} {}", at(1.0), at(-3.0));
        assert!(at(3.5) > at(1.0), "{} {}", at(3.5), at(1.0));
    }

    #[test]
    fn the_tone_curve_keeps_the_color() {
        let c = tone([2.0, 1.0, 0.5]);
        assert!(c[0] <= 1.0 && c[0] > c[1] && c[1] > c[2], "{c:?}");
        assert!(luminance(c) < 1.0);
        assert_eq!(tone([0.2, 0.3, 0.4]), [0.2, 0.3, 0.4]);
    }

    #[test]
    fn a_small_zoom_keeps_the_mean_light() {
        // A satin of 40 stitches at 16 px/mm, and the same at 2 px/mm.
        let s: Vec<Strand> = (0..40)
            .map(|k| {
                let y = 0.2 * k as f64;
                let (a, b) = if k % 2 == 0 {
                    ((0.0, y), (5.0, y + 0.2))
                } else {
                    ((5.0, y), (0.0, y + 0.2))
                };
                strand(a, b, ThreadKind::Polyester)
            })
            .collect();
        let laid = lay(&s, &[]);
        let fine = laid.surface(BBox::new(Point::new(-1.0, -1.0), Point::new(7.0, 9.0)), 16.0);
        let coarse = fine.half().half().half();
        assert_eq!((coarse.width, coarse.px_per_mm), (16, 2.0));
        let lit = |s: &Surface| {
            light(
                s,
                &Lighting::default(),
                Some((Rgb(250, 250, 250), FabricLook::new(crate::fabric::Weave::Leather))),
            )
        };
        let c = crate::compare::compare(&lit(&fine), &lit(&coarse), None, 2, 2);
        assert!(c.largest().unwrap().relative().abs() < 0.05, "{}", c.report());
    }
}
