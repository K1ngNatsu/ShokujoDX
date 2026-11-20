//! The fabric under the stitches (P18 view plan, Milestone 6, ADR 0127).
//!
//! A fabric is a surface, not only a brightness. [`Fabric`] keeps one
//! repeat of a fabric (an area of about [`REPEAT_MM`] that repeats with no
//! seam) as a relief: for each texel the height, the mean direction of the
//! surface, the part of the second color and the light that its low places
//! let in. A pattern for each [`Weave`] makes it at the real size of the
//! weave (`emb_model::Fabric::weave_size`):
//!
//! - **Plain weave and twill**: the threads of the warp (down) and of the
//!   weft (across) go over and under each other. Each thread is round
//!   across and goes down where it goes under. The weft has the second
//!   color (a denim: a dark warp and a light weft).
//! - **Knit**: columns of V-shaped loops. **Piqué**: a knit with small
//!   raised cells.
//! - **Terry**: loops that stand up. **Fleece**: soft hair that stands up.
//!   **Felt**: pressed fibers.
//! - **Leather**: a grain of small flat areas between fine creases.
//! - **Aida**: blocks of threads with a hole at each corner (for cross
//!   stitch, ADR 0128).
//! - On a fabric that is not woven, the second color is a part of the
//!   fibers (a heather).
//!
//! [`Fabric::shade`] lights the relief ([`Shade`]): the diffuse light of the
//! surface (as on a thread, ADR 0125), less light in the low places, and a
//! soft gloss on leather. In the default light the mean color of the fabric
//! is its color. A [`Shade`] gives the mean light of an area of any size
//! (zoom levels of the texels): the texture of the canvas, the pictures and
//! the product view use it, so the fabric looks the same in each.
//!
//! A pile also has a height in mm ([`Fabric::pile_mm`]): thin stitches sink
//! into it, and a topping keeps them on top ([`Ground`], used by
//! [`crate::surface`]). The needle holes are larger on leather
//! ([`Fabric::hole`]).
//!
//! The patterns use no function of the math library, so a fabric has the
//! same texels on each system.

use std::sync::{Arc, Mutex, OnceLock};

use emb_model::{Design, Rgb};

pub use emb_model::Weave;

use crate::light::{to_linear, to_srgb};
use crate::raster::Picture;
use crate::thread3d::Lighting;

/// The size of a repeat of a fabric, about, mm (the patterns of loops and
/// fibers are random in it).
pub const REPEAT_MM: f64 = 16.0;
/// The texels of a repeat on each side (about 32 for each mm).
pub const TEXELS: usize = 512;
/// Each texel is the mean of `SUB` × `SUB` samples of the pattern.
const SUB: usize = 2;

/// Stitches press a pile down: where threads cover all of the area in
/// [`PILE_RADIUS_MM`] round a place, the pile there is this part lower.
pub const PILE_PRESS: f64 = 0.9;
/// Threads that cover less of that area than this do not press the pile
/// (from it to all of the area, more and more).
pub const PILE_PRESS_FROM: f64 = 0.35;
/// The radius of the area that presses the pile at a place, mm (about the
/// size of a loop: a thread narrower than it holds down only a part of the
/// loops round it).
pub const PILE_RADIUS_MM: f64 = 1.0;
/// The pile hides all of a thread where it is this much higher than the
/// thread, mm (a part of it for less).
pub const PILE_SOFT_MM: f64 = 0.25;
/// The most of a thread that the pile hides (the pile has gaps).
pub const PILE_HIDE: f64 = 0.85;
/// A needle hole with no fabric: its radius, mm, and how much darker its
/// middle is.
pub const HOLE: (f64, f64) = (0.12, 0.3);

/// The values of a weave.
struct Params {
    /// The height of a unit of the pattern as a part of its width.
    aspect: f64,
    /// The height of the relief as a part of the size of the weave.
    depth: f64,
    /// How much less light the lowest places get.
    occlusion: f64,
    /// The strength of the gloss.
    gloss: f64,
}

fn params(weave: Weave) -> Params {
    let p = |aspect, depth, occlusion, gloss| Params {
        aspect,
        depth,
        occlusion,
        gloss,
    };
    match weave {
        Weave::Plain => p(1.0, 0.35, 0.35, 0.0),
        Weave::Twill => p(1.0, 0.4, 0.35, 0.0),
        Weave::Knit => p(0.75, 0.4, 0.4, 0.0),
        Weave::Pique => p(1.0, 0.35, 0.35, 0.0),
        Weave::Terry => p(1.0, 0.6, 0.55, 0.0),
        Weave::Fleece => p(1.0, 0.4, 0.3, 0.0),
        Weave::Felt => p(1.0, 0.12, 0.15, 0.0),
        Weave::Leather => p(1.0, 0.25, 0.2, 0.12),
        Weave::Aida => p(1.0, 0.22, 0.45, 0.0),
    }
}

/// The fabric of a picture: its weave, the size of the weave, mm, and its
/// second color (the color of the fabric is the background).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FabricLook {
    pub weave: Weave,
    pub size_mm: f64,
    pub mix: Option<Rgb>,
}

impl FabricLook {
    /// The weave at its usual size, in one color.
    pub fn new(weave: Weave) -> FabricLook {
        FabricLook {
            weave,
            size_mm: weave.default_mm(),
            mix: None,
        }
    }

    /// The fabric of `design` (its preset `fabric`).
    pub fn of(fabric: &emb_model::Fabric, design: &Design) -> FabricLook {
        FabricLook {
            weave: fabric.weave,
            size_mm: fabric.weave_size(),
            mix: design.fabric_mix,
        }
    }

    /// The relief of this fabric.
    pub fn fabric(&self) -> Arc<Fabric> {
        Fabric::shared(self.weave, self.size_mm)
    }
}

/// The fabric under the stitches as the surface needs it: the pile, the
/// needle holes, a fabric that is not there (lace) and the batting of a
/// quilt.
#[derive(Debug, Clone)]
pub struct Ground {
    pub fabric: Arc<Fabric>,
    /// The sew-out has a topping: the stitches stay on top of a pile.
    pub topping: bool,
    /// Free-standing lace: the stabilizer washes away, so no fabric shows
    /// and nothing puts a shadow on it (ADR 0128).
    pub lace: bool,
    /// The loft of the batting under the fabric (a quilt), mm: the fabric
    /// is this much higher far from the stitches (ADR 0128).
    pub loft_mm: f64,
}

impl Ground {
    pub fn new(weave: Weave, size_mm: f64, topping: bool) -> Ground {
        Ground {
            fabric: Fabric::shared(weave, size_mm),
            topping,
            lace: false,
            loft_mm: 0.0,
        }
    }

    /// The fabric of `design` (its preset `fabric`): with a topping if the
    /// fabric names one and the design does not say that it has none.
    pub fn of(fabric: &emb_model::Fabric, design: &Design) -> Ground {
        Ground {
            lace: fabric.is_lace(),
            loft_mm: fabric.loft_mm.unwrap_or(0.0),
            ..Ground::new(fabric.weave, fabric.weave_size(), fabric.topping_for(design).is_some())
        }
    }

    /// The same number for the same ground.
    pub fn key(&self) -> u64 {
        let w = Weave::ALL.iter().position(|w| *w == self.fabric.weave).unwrap_or(0) as u64;
        self.fabric.size_mm.to_bits().rotate_left(7)
            ^ (w << 1)
            ^ u64::from(self.topping)
            ^ (u64::from(self.lace) << 5)
            ^ self.loft_mm.to_bits().rotate_left(23)
    }

    /// Thin stitches sink into a pile: the fabric has one and no topping.
    pub fn sinks(&self) -> bool {
        self.fabric.pile_mm() > 0.0 && !self.topping
    }

    /// The needle holes.
    pub fn hole(ground: Option<&Ground>) -> (f64, f64) {
        ground.map_or(HOLE, |g| g.fabric.hole())
    }
}

/// One texel of the relief: the mean height (0 to 1 of the depth), the
/// mean direction of the surface (shorter than 1 where it changes), the
/// part of the second color and the light that the low places let in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Texel {
    height: f32,
    normal: [f32; 3],
    mix: f32,
    open: f32,
}

/// One repeat of a fabric as a relief (see the module text).
pub struct Fabric {
    pub weave: Weave,
    pub size_mm: f64,
    /// The size of the repeat, mm.
    pub repeat: (f64, f64),
    texels: Vec<Texel>,
    /// The mean light in the default light (the light is divided by it).
    norm: f64,
}

impl std::fmt::Debug for Fabric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fabric")
            .field("weave", &self.weave)
            .field("size_mm", &self.size_mm)
            .field("repeat", &self.repeat)
            .finish()
    }
}

/// The repeat of a pattern in units: `nu` × `nv` units, and the cells of the
/// fibers in a unit.
#[derive(Clone, Copy)]
struct Repeat {
    nu: i64,
    nv: i64,
    fiber: i64,
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// A value from 0 to 1 for the cell (`x`, `y`) of a grid that repeats after
/// `px` × `py` cells.
fn hash(x: i64, y: i64, px: i64, py: i64, seed: u64) -> f64 {
    let (x, y) = (x.rem_euclid(px.max(1)) as u64, y.rem_euclid(py.max(1)) as u64);
    let mut h = x.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ y.wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ seed.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h & 0xFFFF) as f64 / 65535.0
}

/// Smooth noise from 0 to 1 with `k` cells in a unit, that repeats after
/// the units of `r`.
fn noise(u: f64, v: f64, k: i64, r: &Repeat, seed: u64) -> f64 {
    noise2(u, v, (k, k), r, seed)
}

/// As [`noise`], with `k.0` cells across a unit and `k.1` cells down.
fn noise2(u: f64, v: f64, k: (i64, i64), r: &Repeat, seed: u64) -> f64 {
    let (gx, gy) = (u * k.0 as f64, v * k.1 as f64);
    let (fx, fy) = (gx.floor(), gy.floor());
    let (sx, sy) = (smooth(gx - fx), smooth(gy - fy));
    let (ix, iy) = (fx as i64, fy as i64);
    let (px, py) = (r.nu * k.0, r.nv * k.1);
    let c = |x: i64, y: i64| hash(x, y, px, py, seed);
    let a = c(ix, iy) + (c(ix + 1, iy) - c(ix, iy)) * sx;
    let b = c(ix, iy + 1) + (c(ix + 1, iy + 1) - c(ix, iy + 1)) * sx;
    a + (b - a) * sy
}

/// The part of the second color in a heather: about a third of the fibers,
/// in thin streaks.
fn heather(u: f64, v: f64, r: &Repeat) -> f64 {
    ((noise2(u, v, (2 * r.fiber, (r.fiber / 2).max(1)), r, 7) - 0.56) / 0.08).clamp(0.0, 1.0)
}

/// A triangle wave from 0 to 1 with one turn for each 1 of `t`.
fn tri(t: f64) -> f64 {
    let f = t - t.floor();
    if f < 0.5 { 2.0 * f } else { 2.0 - 2.0 * f }
}

/// The threads of a woven fabric: half the width of a warp thread and of a
/// weft thread (units), the top of the weft (a part of the top of the
/// warp), and the strength of the twist of the fibers on the warp.
struct Threads {
    warp: f64,
    weft: f64,
    weft_top: f64,
    twist: f64,
}

/// A woven fabric: the warp thread `i` at `u = i` and the weft thread `j` at
/// `v = j`; at the crossing (`i`, `j`) the warp is on top if `over(i, j)`.
fn woven(u: f64, v: f64, r: &Repeat, t: &Threads, over: impl Fn(i64, i64) -> bool) -> (f64, f64) {
    // The height of a thread where it goes under, as a part of its top.
    const LOW: f64 = 0.3;
    let across = |d: f64, half: f64| {
        let q = d / half;
        (1.0 - q * q).max(0.0).sqrt()
    };
    let lift = |a: bool, b: bool, t: f64| {
        let (a, b) = (f64::from(u8::from(a)), f64::from(u8::from(b)));
        a + (b - a) * smooth(t)
    };
    let (i, j) = (u.round(), v.round());
    let (i0, j0) = (u.floor(), v.floor());
    let (ii, jj, ii0, jj0) = (i as i64, j as i64, i0 as i64, j0 as i64);
    // The fibers of the warp turn round it (the twist of the yarn): fine
    // ridges at a steep slant.
    let twist = 1.0 - t.twist * tri(3.0 * u + v);
    let warp = across(u - i, t.warp) * (LOW + (1.0 - LOW) * lift(over(ii, jj0), over(ii, jj0 + 1), v - j0)) * twist;
    let weft =
        t.weft_top * across(v - j, t.weft) * (LOW + (1.0 - LOW) * lift(!over(ii0, jj), !over(ii0 + 1, jj), u - i0));
    let grain = 0.06 * (noise(u, v, r.fiber, r, 3) - 0.5);
    if warp >= weft {
        ((warp + grain).clamp(0.0, 1.0), 0.0)
    } else {
        ((weft + grain).clamp(0.0, 1.0), 1.0)
    }
}

/// A knit: in each column (a unit wide) a V-shaped loop in each row (a unit
/// high); the two legs meet at the bottom of the loop.
fn knit(u: f64, v: f64) -> f64 {
    let fu = u - u.floor() - 0.5;
    let fv = v - v.floor();
    let spread = 0.06 + 0.26 * (1.0 - fv);
    let leg = |c: f64| {
        let q = (fu - c) / 0.16;
        (1.0 - q * q).max(0.0).sqrt()
    };
    leg(-spread).max(leg(spread)) * (0.45 + 0.55 * 4.0 * fv * (1.0 - fv))
}

/// Piqué: rows of round cells (each row a half cell to the side) with
/// raised edges.
fn pique(u: f64, v: f64) -> f64 {
    let row = v.floor();
    let uu = u + 0.5 * row.rem_euclid(2.0);
    let (fu, fv) = (uu - uu.floor() - 0.5, v - row - 0.5);
    let d = 2.0 * (fu * fu + fv * fv).sqrt();
    0.25 + 0.75 * smooth(((d - 0.3) / 0.6).clamp(0.0, 1.0))
}

/// Terry: three loops in each cell (a unit) at random places. Seen from above
/// a loop is a ring that is open on one side, on a mound of its fibers.
fn loops(u: f64, v: f64, r: &Repeat) -> f64 {
    let (ci, cj) = (u.floor() as i64, v.floor() as i64);
    let mut best = 0.0f64;
    for dj in -1..=1 {
        for di in -1..=1 {
            for s in 0..3u64 {
                let (i, j) = (ci + di, cj + dj);
                let rnd = |q: u64| hash(i, j, r.nu, r.nv, 10 * s + q);
                let (cx, cy) = (i as f64 + rnd(1), j as f64 + rnd(2));
                let rad = 0.45 + 0.15 * rnd(3);
                let (dx, dy) = (u - cx, v - cy);
                let q = (dx * dx + dy * dy) / (rad * rad);
                if q >= 1.0 {
                    continue;
                }
                let d = q.sqrt();
                let e = (d - 0.55) / 0.4;
                let ring = (1.0 - e * e).max(0.0);
                // The side where the loop is open.
                let (ox, oy) = (rnd(5) - 0.5, rnd(6) - 0.5);
                let olen = (ox * ox + oy * oy).sqrt().max(1e-6);
                let dlen = (dx * dx + dy * dy).sqrt().max(1e-6);
                let facing = (dx * ox + dy * oy) / (olen * dlen);
                let arc = smooth(((0.5 - facing) / 0.6).clamp(0.0, 1.0));
                let mound = 0.55 * (1.0 - q);
                best = best.max((0.7 + 0.3 * rnd(4)) * (ring * arc).max(mound));
            }
        }
    }
    best
}

/// Aida: a block of threads in each cell (a unit), a groove between two
/// blocks and a hole where four blocks meet, and the fine threads of each
/// block.
fn aida(u: f64, v: f64) -> f64 {
    let (fu, fv) = (u - u.floor() - 0.5, v - v.floor() - 0.5);
    let block = |d: f64| {
        let q = (2.0 * d.abs()).min(1.0);
        let q2 = q * q;
        1.0 - q2 * q2 * q2
    };
    let (a, b) = (block(fu), block(fv));
    // Three threads each way in a block, over and under.
    let fine = tri(3.0 * u) * tri(3.0 * v);
    0.33 * a + 0.33 * b + 0.26 * a * b + 0.08 * fine
}

/// Leather: a point in each cell; a fine crease where two points are at the
/// same distance, and round pebbles between the creases.
fn grain(u: f64, v: f64, r: &Repeat) -> f64 {
    let (ci, cj) = (u.floor() as i64, v.floor() as i64);
    let (mut f1, mut f2) = (9.0f64, 9.0f64);
    for dj in -1..=1 {
        for di in -1..=1 {
            let (i, j) = (ci + di, cj + dj);
            let (px, py) = (
                i as f64 + hash(i, j, r.nu, r.nv, 1),
                j as f64 + hash(i, j, r.nu, r.nv, 2),
            );
            let d = ((u - px) * (u - px) + (v - py) * (v - py)).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    let crease = smooth(((f2 - f1) / 0.18).clamp(0.0, 1.0));
    let pebble = 1.0 - 0.8 * (f1 * f1).min(1.0);
    0.88 * crease * pebble + 0.12 * noise(u, v, 4, r, 5)
}

/// The height (0 to 1) and the part of the second color of the pattern of
/// `weave` at (`u`, `v`) units.
fn sample(weave: Weave, u: f64, v: f64, r: &Repeat) -> (f64, f64) {
    let n = |k: i64, seed: u64| noise(u, v, k, r, seed);
    match weave {
        Weave::Plain => {
            let t = Threads {
                warp: 0.46,
                weft: 0.46,
                weft_top: 1.0,
                twist: 0.05,
            };
            woven(u, v, r, &t, |i, j| (i + j).rem_euclid(2) == 0)
        }
        Weave::Twill => {
            // A dense warp that covers most of the weft.
            let t = Threads {
                warp: 0.53,
                weft: 0.38,
                weft_top: 0.8,
                twist: 0.15,
            };
            // The diagonal goes up to the right, with the twist of the
            // fibers (most denim).
            woven(u, v, r, &t, |i, j| (j + i).rem_euclid(4) != 3)
        }
        Weave::Knit => (
            (knit(u, v) + 0.06 * (n(r.fiber, 3) - 0.5)).clamp(0.0, 1.0),
            heather(u, v, r),
        ),
        Weave::Pique => (
            (pique(u, v) + 0.12 * (n(r.fiber, 3) - 0.5)).clamp(0.0, 1.0),
            heather(u, v, r),
        ),
        Weave::Terry => (
            (loops(u, v, r) + 0.1 * (n(r.fiber, 3) - 0.5)).clamp(0.0, 1.0),
            heather(u, v, r),
        ),
        Weave::Fleece => (0.5 * n(1, 11) + 0.3 * n(3, 12) + 0.2 * n(9, 13), heather(u, v, r)),
        Weave::Felt => (0.4 * n(2, 11) + 0.35 * n(5, 12) + 0.25 * n(12, 13), heather(u, v, r)),
        Weave::Leather => (grain(u, v, r), 0.0),
        Weave::Aida => (
            (aida(u, v) + 0.06 * (n(r.fiber, 3) - 0.5)).clamp(0.0, 1.0),
            heather(u, v, r),
        ),
    }
}

/// `f(row)` for each row `0..rows`, on all processors, in order.
fn rows_in_parallel<T: Send>(rows: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(rows)
        .max(1);
    let per = rows.div_ceil(workers);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|k| {
                let f = &f;
                scope.spawn(move || (k * per..((k + 1) * per).min(rows)).map(f).collect::<Vec<T>>())
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("a thread of the fabric does not stop with an error"))
            .collect()
    })
}

/// `x` to the power 16.
fn pow16(x: f64) -> f64 {
    let x2 = x * x;
    let x4 = x2 * x2;
    let x8 = x4 * x4;
    x8 * x8
}

/// The size of a weave in its limits.
fn size_of(weave: Weave, size_mm: f64) -> f64 {
    if size_mm.is_finite() {
        size_mm.clamp(0.05, 5.0)
    } else {
        weave.default_mm()
    }
}

/// The repeat of the pattern of `weave` at `size` mm: its units, and its
/// size in mm.
fn repeat_of(weave: Weave, size: f64) -> (Repeat, (f64, f64)) {
    let p = params(weave);
    let (ux, uy) = (size, size * p.aspect);
    // A whole number of units, a multiple of 4 (the repeat of a twill).
    let units = |unit: f64| ((REPEAT_MM / (4.0 * unit)).round().max(1.0) * 4.0) as i64;
    let (nu, nv) = (units(ux), units(uy));
    let r = Repeat {
        nu,
        nv,
        fiber: ((size / 0.08).round() as i64).clamp(2, 64),
    };
    (r, (nu as f64 * ux, nv as f64 * uy))
}

impl Fabric {
    /// The relief of `weave` with the size `size_mm` (see the module text).
    pub fn new(weave: Weave, size_mm: f64) -> Fabric {
        let size = size_of(weave, size_mm);
        let p = params(weave);
        let (r, repeat) = repeat_of(weave, size);
        let (nu, nv) = (r.nu, r.nv);
        let f = TEXELS * SUB;
        // The fine samples: height and second color.
        let fine: Vec<Vec<(f32, f32)>> = rows_in_parallel(f, |j| {
            let v = (j as f64 + 0.5) / f as f64 * nv as f64;
            (0..f)
                .map(|i| {
                    let u = (i as f64 + 0.5) / f as f64 * nu as f64;
                    let (h, m) = sample(weave, u, v, &r);
                    (h as f32, m as f32)
                })
                .collect()
        });
        let depth = p.depth * size;
        let (dx, dy) = (repeat.0 / f as f64, repeat.1 / f as f64);
        let at = |i: usize, j: usize| f64::from(fine[j % f][i % f].0);
        let texels: Vec<Vec<Texel>> = rows_in_parallel(TEXELS, |ty| {
            (0..TEXELS)
                .map(|tx| {
                    let mut t = [0.0f64; 6];
                    for sj in 0..SUB {
                        for si in 0..SUB {
                            let (i, j) = (tx * SUB + si, ty * SUB + sj);
                            let h = at(i, j);
                            let gx = (at(i + 1, j) - at(i + f - 1, j)) * depth / (2.0 * dx);
                            let gy = (at(i, j + 1) - at(i, j + f - 1)) * depth / (2.0 * dy);
                            let len = (gx * gx + gy * gy + 1.0).sqrt();
                            t[0] += h;
                            t[1] -= gx / len;
                            t[2] -= gy / len;
                            t[3] += 1.0 / len;
                            t[4] += f64::from(fine[j][i].1);
                            t[5] += 1.0 - p.occlusion * (1.0 - h);
                        }
                    }
                    let t = t.map(|v| (v / (SUB * SUB) as f64) as f32);
                    Texel {
                        height: t[0],
                        normal: [t[1], t[2], t[3]],
                        mix: t[4],
                        open: t[5],
                    }
                })
                .collect()
        });
        let mut fabric = Fabric {
            weave,
            size_mm: size,
            repeat,
            texels: texels.into_iter().flatten().collect(),
            norm: 1.0,
        };
        let l = Lighting::default();
        let sum: f64 = fabric.texels.iter().map(|t| fabric.light(t, &l)).sum();
        fabric.norm = (sum / fabric.texels.len() as f64).max(1e-6);
        fabric
    }

    /// The relief of `weave` at `size_mm`, from a cache of the last ones
    /// (a relief takes some milliseconds and 8 MB).
    pub fn shared(weave: Weave, size_mm: f64) -> Arc<Fabric> {
        static CACHE: OnceLock<Mutex<Vec<Arc<Fabric>>>> = OnceLock::new();
        let mut cache = CACHE
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let size = size_of(weave, size_mm);
        if let Some(f) = cache.iter().find(|f| f.weave == weave && f.size_mm == size) {
            return f.clone();
        }
        let f = Arc::new(Fabric::new(weave, size));
        cache.push(f.clone());
        if cache.len() > 4 {
            cache.remove(0);
        }
        f
    }

    /// The light of a texel in the light `l`, before it is divided by the
    /// mean (`norm`).
    fn light(&self, t: &Texel, l: &Lighting) -> f64 {
        let lights = l.lights();
        let n = t.normal.map(f64::from);
        let mut f = f64::from(t.open) * crate::sheen::diffuse_in(n, &lights);
        let gloss = params(self.weave).gloss;
        if gloss > 0.0 {
            for (v, w) in lights {
                if w <= 0.0 {
                    continue;
                }
                // The half vector of the light and the eye (straight above).
                let hv = crate::sheen::half_vector(v);
                let nh = (n[0] * hv[0] + n[1] * hv[1] + n[2] * hv[2]).max(0.0);
                f += w * gloss * pow16(nh);
            }
        }
        f
    }

    /// The height of the pile at (`x`, `y`) mm, mm (0 with no pile).
    pub fn pile_at(&self, x: f64, y: f64) -> f64 {
        let pile = self.pile_mm();
        if pile <= 0.0 {
            return 0.0;
        }
        pile * self.height_at(x, y)
    }

    /// The height of the relief at (`x`, `y`) mm, 0 to 1 (between the
    /// texels: the mean of the four nearest ones).
    pub fn height_at(&self, x: f64, y: f64) -> f64 {
        let n = TEXELS;
        let fx = x / self.repeat.0 * n as f64 - 0.5;
        let fy = y / self.repeat.1 * n as f64 - 0.5;
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (ix, iy) = (x0 as i64, y0 as i64);
        let h = |i: i64, j: i64| {
            let k = j.rem_euclid(n as i64) as usize * n + i.rem_euclid(n as i64) as usize;
            f64::from(self.texels[k].height)
        };
        let a = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * tx;
        let b = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * tx;
        a + (b - a) * ty
    }

    /// The height of the relief from its lowest to its highest place, mm.
    pub fn relief_mm(&self) -> f64 {
        relief_of(self.weave, self.size_mm)
    }

    /// The height of the pile of this fabric, mm (0: no pile).
    pub fn pile_mm(&self) -> f64 {
        let base = match self.weave {
            Weave::Terry => 1.6,
            Weave::Fleece => 0.9,
            _ => return 0.0,
        };
        base * (self.size_mm / self.weave.default_mm()).clamp(0.5, 2.0)
    }

    /// A needle hole: its radius, mm, and how much darker its middle is.
    /// The hole of a needle stays open in leather and vinyl.
    pub fn hole(&self) -> (f64, f64) {
        match self.weave {
            Weave::Leather => (0.4, 0.75),
            Weave::Felt => (0.2, 0.4),
            _ => HOLE,
        }
    }

    /// The light of the fabric in `lighting` (see [`Shade`]); with the
    /// lights off, the little light of a dark room
    /// ([`crate::sheen::LIGHTS_OFF`]).
    pub fn shade(&self, lighting: &Lighting) -> Shade {
        let k = lighting.strength.clamp(0.0, 1.0);
        let dark = if lighting.lights_off {
            crate::sheen::LIGHTS_OFF
        } else {
            1.0
        };
        let n = TEXELS;
        let mut a = Vec::with_capacity(n * n);
        let mut b = Vec::with_capacity(n * n);
        for t in &self.texels {
            let f = (1.0 + (self.light(t, lighting) / self.norm - 1.0) * k) * dark;
            let m = f64::from(t.mix);
            a.push((f * (1.0 - m)) as f32);
            b.push((f * m) as f32);
        }
        let mut levels = vec![Level { n, a, b }];
        while levels.last().is_some_and(|l| l.n > 1) {
            let next = levels.last().expect("a level").half();
            levels.push(next);
        }
        Shade {
            repeat: self.repeat,
            levels,
        }
    }
}

/// The zoom level of the texels of a fabric whose repeat is `repeat_mm`
/// wide, for a view at `px_per_mm`: its texels are as small as the pixels
/// or smaller (at most two times smaller). Level `k` has `TEXELS >> k`
/// texels on each side.
pub fn level_for(repeat_mm: f64, px_per_mm: f64) -> usize {
    let mut texel_ppm = TEXELS as f64 / repeat_mm;
    let mut level = 0;
    while texel_ppm / 2.0 >= px_per_mm && (TEXELS >> (level + 1)) >= 1 {
        texel_ppm /= 2.0;
        level += 1;
    }
    level
}

/// The height of the relief of `weave` at `size_mm` from its lowest to its
/// highest place, mm (with no relief made).
pub fn relief_of(weave: Weave, size_mm: f64) -> f64 {
    params(weave).depth * size_of(weave, size_mm)
}

/// One zoom level of a [`Shade`]: `n` × `n` texels.
struct Level {
    n: usize,
    /// The light of the first color and of the second color (the light
    /// times the part of each color).
    a: Vec<f32>,
    b: Vec<f32>,
}

impl Level {
    /// The level with half the texels on each side (the means of four).
    fn half(&self) -> Level {
        let m = self.n / 2;
        let mean = |v: &[f32], x: usize, y: usize| {
            let at = |i: usize, j: usize| f64::from(v[j * self.n + i]);
            ((at(2 * x, 2 * y) + at(2 * x + 1, 2 * y) + at(2 * x, 2 * y + 1) + at(2 * x + 1, 2 * y + 1)) / 4.0) as f32
        };
        let mut a = Vec::with_capacity(m * m);
        let mut b = Vec::with_capacity(m * m);
        for y in 0..m {
            for x in 0..m {
                a.push(mean(&self.a, x, y));
                b.push(mean(&self.b, x, y));
            }
        }
        Level { n: m, a, b }
    }

    /// The values at (`fx`, `fy`) texels (between the texels: the mean of the
    /// four nearest ones).
    fn at(&self, fx: f64, fy: f64) -> (f64, f64) {
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let n = self.n as i64;
        let (ix, iy) = (x0 as i64, y0 as i64);
        let k = |i: i64, j: i64| j.rem_euclid(n) as usize * self.n + i.rem_euclid(n) as usize;
        let bilinear = |v: &[f32]| {
            let g = |i, j| f64::from(v[k(i, j)]);
            let a = g(ix, iy) + (g(ix + 1, iy) - g(ix, iy)) * tx;
            let b = g(ix, iy + 1) + (g(ix + 1, iy + 1) - g(ix, iy + 1)) * tx;
            a + (b - a) * ty
        };
        (bilinear(&self.a), bilinear(&self.b))
    }
}

/// The light of a fabric in one light: for each texel the light of its
/// first color and of its second color (factors of the colors in linear
/// light; their mean over the repeat is 1 in the default light), at zoom
/// levels for the mean of a larger area.
pub struct Shade {
    /// The size of the repeat, mm.
    pub repeat: (f64, f64),
    levels: Vec<Level>,
}

impl Shade {
    /// The mean light of the first and of the second color in the square
    /// of `size` mm at (`x`, `y`) mm.
    pub fn at(&self, x: f64, y: f64, size: f64) -> (f64, f64) {
        let texel = self.repeat.0 / TEXELS as f64;
        let (mut level, mut t) = (0usize, texel);
        while 2.0 * t <= size && level + 1 < self.levels.len() {
            t *= 2.0;
            level += 1;
        }
        let sample = |k: usize| {
            let l = &self.levels[k];
            let fx = x / self.repeat.0 * l.n as f64 - 0.5;
            let fy = y / self.repeat.1 * l.n as f64 - 0.5;
            l.at(fx, fy)
        };
        let a = sample(level);
        if level + 1 >= self.levels.len() || size <= t {
            return a;
        }
        let b = sample(level + 1);
        let r = ((size - t) / t).clamp(0.0, 1.0);
        (a.0 + (b.0 - a.0) * r, a.1 + (b.1 - a.1) * r)
    }

    /// The mean color (linear light) of the square of `size` mm at (`x`,
    /// `y`) mm of a fabric of the color `color` (linear light) and the second
    /// color `mix` (`None`: one color).
    pub fn color(&self, color: [f64; 3], mix: Option<[f64; 3]>, x: f64, y: f64, size: f64) -> [f64; 3] {
        let (a, b) = self.at(x, y, size);
        let m = mix.unwrap_or(color);
        [0, 1, 2].map(|k| color[k] * a + m[k] * b)
    }

    /// The number of zoom levels (the last one has one texel).
    pub fn levels(&self) -> usize {
        self.levels.len()
    }

    /// The zoom level for a view at `px_per_mm` ([`level_for`]).
    pub fn level_for(&self, px_per_mm: f64) -> usize {
        level_for(self.repeat.0, px_per_mm).min(self.levels.len() - 1)
    }

    /// The texels of the zoom level `level` as a picture of a fabric of the
    /// color `color` and the second color `mix`, in the colors of the
    /// screen (one repeat: it repeats with no seam).
    pub fn picture(&self, level: usize, color: Rgb, mix: Option<Rgb>) -> Picture {
        let l = &self.levels[level.min(self.levels.len() - 1)];
        let lin = |c: Rgb| [c.0, c.1, c.2].map(to_linear);
        let (c1, c2) = (lin(color), lin(mix.unwrap_or(color)));
        let mut rgba = Vec::with_capacity(l.n * l.n * 4);
        for (a, b) in l.a.iter().zip(&l.b) {
            let (a, b) = (f64::from(*a), f64::from(*b));
            let c = crate::sheen::tone([0, 1, 2].map(|k| c1[k] * a + c2[k] * b));
            rgba.extend_from_slice(&[to_srgb(c[0]), to_srgb(c[1]), to_srgb(c[2]), 255]);
        }
        Picture {
            width: l.n as u32,
            height: l.n as u32,
            rgba,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fabric_repeats_with_no_seam() {
        for weave in Weave::ALL {
            let f = Fabric::shared(weave, weave.default_mm());
            let (rx, ry) = f.repeat;
            assert!(
                (12.0..=20.0).contains(&rx) && (12.0..=20.0).contains(&ry),
                "{weave:?} {rx} {ry}"
            );
            for (x, y) in [(0.3, 0.7), (5.1, 2.9), (11.95, 0.05)] {
                let a = f.height_at(x, y);
                let b = f.height_at(x + rx, y - 2.0 * ry);
                assert!((a - b).abs() < 1e-9, "{weave:?}");
            }
            // The pattern itself repeats, so the edges of the repeat join.
            let (r, _) = repeat_of(weave, weave.default_mm());
            for k in 0..200 {
                let (u, v) = (0.173 * f64::from(k), 0.091 * f64::from(k));
                let a = sample(weave, u, v, &r);
                let b = sample(weave, u + r.nu as f64, v - 2.0 * r.nv as f64, &r);
                assert!(
                    (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9,
                    "{weave:?} {u} {v}"
                );
            }
        }
    }

    #[test]
    fn the_mean_light_is_the_color_of_the_fabric() {
        for weave in Weave::ALL {
            let f = Fabric::shared(weave, weave.default_mm());
            let s = f.shade(&Lighting::default());
            let last = &s.levels[s.levels.len() - 1];
            let mean = f64::from(last.a[0]) + f64::from(last.b[0]);
            assert!((mean - 1.0).abs() < 1e-3, "{weave:?} {mean}");
            // A large area has the mean; a small area has the pattern.
            let (a, b) = s.at(3.3, 7.7, 8.0);
            assert!((a + b - 1.0).abs() < 0.08, "{weave:?} {}", a + b);
            let values: Vec<f64> = (0..50)
                .map(|k| {
                    let (a, b) = s.at(0.037 * f64::from(k), 1.0, 0.0);
                    a + b
                })
                .collect();
            let spread = values.iter().copied().fold(0.0f64, f64::max) - values.iter().copied().fold(9.0, f64::min);
            assert!(spread > 0.05, "{weave:?} has a texture: {spread}");
        }
    }

    #[test]
    fn the_light_changes_the_fabric() {
        // A low light from the side shows the relief more.
        let f = Fabric::shared(Weave::Terry, 1.0);
        let spread = |l: &Lighting| {
            let s = f.shade(l);
            let v: Vec<f64> = (0..400)
                .map(|k| {
                    let (a, b) = s.at(0.041 * f64::from(k), 0.013 * f64::from(k), 0.0);
                    a + b
                })
                .collect();
            let mean = v.iter().sum::<f64>() / v.len() as f64;
            // As a part of the mean (a low light is darker).
            v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / v.len() as f64 / (mean * mean)
        };
        let low = Lighting {
            elevation_deg: 15.0,
            ..Lighting::default()
        };
        assert!(spread(&low) > spread(&Lighting::default()));
        let flat = Lighting {
            strength: 0.0,
            ..Lighting::default()
        };
        assert!(spread(&flat) < 1e-12);
    }

    #[test]
    fn the_second_color_is_the_weft_or_a_part_of_the_fibers() {
        let part = |weave: Weave| {
            let f = Fabric::shared(weave, weave.default_mm());
            let s = f.shade(&Lighting::default());
            let last = &s.levels[s.levels.len() - 1];
            f64::from(last.b[0]) / (f64::from(last.a[0]) + f64::from(last.b[0]))
        };
        // Plain weave: half warp, half weft; a twill shows more warp.
        assert!((part(Weave::Plain) - 0.5).abs() < 0.1, "{}", part(Weave::Plain));
        assert!(part(Weave::Twill) < 0.4, "{}", part(Weave::Twill));
        // A heather: a part of the fibers.
        for w in [Weave::Knit, Weave::Fleece, Weave::Felt] {
            assert!((0.1..0.5).contains(&part(w)), "{w:?} {}", part(w));
        }
        assert!(part(Weave::Leather) < 1e-6);
    }

    #[test]
    fn a_pile_has_a_height() {
        let terry = Fabric::shared(Weave::Terry, 1.0);
        assert!(terry.pile_mm() > 1.0);
        let max = (0..200)
            .map(|k| terry.pile_at(0.05 * f64::from(k), 0.03 * f64::from(k)))
            .fold(0.0, f64::max);
        assert!(max > 0.8 * terry.pile_mm() * 0.75, "{max}");
        assert_eq!(Fabric::shared(Weave::Plain, 0.25).pile_at(1.0, 1.0), 0.0);
        let leather = Fabric::shared(Weave::Leather, 0.7);
        assert!(leather.hole().0 > HOLE.0);
    }

    /// Writes a picture of each fabric (one repeat, at about 32 pixels for
    /// each mm, and a denim and a heather) to `target/tmp/fabrics`. Run with
    /// `cargo test --release -p emb-render -- --ignored write_fabric_pictures`.
    #[test]
    #[ignore]
    fn write_fabric_pictures() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/fabrics");
        std::fs::create_dir_all(&dir).expect("a folder");
        let cases = [
            (Weave::Plain, Rgb(238, 234, 226), None, "plain"),
            (Weave::Twill, Rgb(40, 60, 110), Some(Rgb(225, 225, 215)), "denim"),
            (Weave::Twill, Rgb(150, 40, 40), None, "twill"),
            (Weave::Knit, Rgb(60, 110, 60), None, "knit"),
            (Weave::Knit, Rgb(140, 140, 140), Some(Rgb(60, 60, 64)), "heather"),
            (Weave::Pique, Rgb(30, 50, 120), None, "pique"),
            (Weave::Terry, Rgb(200, 216, 232), None, "terry"),
            (Weave::Fleece, Rgb(120, 120, 124), Some(Rgb(70, 70, 74)), "fleece"),
            (Weave::Felt, Rgb(180, 40, 40), None, "felt"),
            (Weave::Leather, Rgb(110, 70, 40), None, "leather"),
        ];
        // The time to make each relief and its light.
        for weave in Weave::ALL {
            let t = std::time::Instant::now();
            let f = Fabric::new(weave, weave.default_mm());
            let made = t.elapsed().as_secs_f64() * 1000.0;
            let t = std::time::Instant::now();
            let s = f.shade(&Lighting::default());
            let lit = t.elapsed().as_secs_f64() * 1000.0;
            let t = std::time::Instant::now();
            let p = s.picture(0, Rgb(200, 100, 50), None);
            println!(
                "{weave:?}: relief {made:.1} ms, light {lit:.1} ms, texture of {} pixels {:.1} ms",
                p.width,
                t.elapsed().as_secs_f64() * 1000.0
            );
        }
        for (weave, color, mix, name) in cases {
            let s = Fabric::shared(weave, weave.default_mm()).shade(&Lighting::default());
            for level in [0, 2] {
                let p = s.picture(level, color, mix);
                std::fs::write(dir.join(format!("{name}-{level}.png")), p.png()).expect("a picture");
            }
        }
    }

    #[test]
    fn the_levels_for_the_canvas() {
        let s = Fabric::shared(Weave::Plain, 0.25).shade(&Lighting::default());
        assert_eq!(s.level_for(64.0), 0);
        assert_eq!(s.level_for(32.0), 0);
        assert_eq!(s.level_for(8.0), 2);
        let p = s.picture(2, Rgb(200, 100, 50), None);
        assert_eq!((p.width, p.height), (128, 128));
        assert_eq!(s.level_for(0.001), s.levels() - 1);
    }
}
