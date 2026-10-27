//! The picture bench (P12 Milestone 1): the steps of the Picture to design
//! wizard on test pictures, with numbers that say how good the result is.
//!
//! The test pictures are made by the program ([`pictures`]), so they have
//! no license question and are the same on each computer: a JPEG-like logo
//! (blocks of 8 × 8 pixels coarsely quantized, as a JPEG of low quality),
//! a small icon, a large picture, a picture with soft clear edges, line
//! art, a badge with small details on a large area and a busy
//! portrait-like picture. [`pictures_in`] adds the pictures
//! of a folder.
//!
//! For each picture [`run`] does what the wizard does with its defaults:
//! the work picture (at most `WORK_PX` pixels), the background step that
//! the picture asks for, Simplify, Stitches (each color kept, Auto, in the
//! order of the list) and the stitch plan. Then it compares a picture of
//! the stitches with the picture: how much of the picture the stitches
//! cover, the stitches outside it, and the mean color difference (ΔE, CIE
//! 1976) where both have color.

use std::path::Path;
use std::time::{Duration, Instant};

use emb_art::smart::{self, ColorChoice, SimplifySpec, SmartSpec};
use emb_art::{Rgba, RgbaImage, background};
use emb_geom::{BBox, Point, Transform, math};
use emb_model::{CurrentProperties, Design, Lab, ObjectKind, Rgb, StitchKind};

/// The stitch picture has this many pixels in a mm.
const PX_PER_MM: f64 = 4.0;
/// The smoothing of Simplify at the start of the wizard.
const SMOOTH: u32 = 1;
/// The width of a thread on the fabric in the stitch picture, mm (rows of
/// a fill 0.4 mm apart cover the fabric).
const THREAD_MM: f64 = 0.45;

/// What the background step does with a picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Background {
    /// Nothing (the picture has no background, or it is clear).
    Keep,
    /// Remove the color at the edges.
    EdgeColor,
    /// Cut out the subject in a box `[x0, y0, x1, y1]` of the work picture.
    CutOut([f64; 4]),
}

/// A test picture and the settings that the bench uses for it.
#[derive(Debug, Clone)]
pub struct BenchPicture {
    pub name: String,
    pub image: RgbaImage,
    /// The width of the picture (as on the backdrop), in mm.
    pub width_mm: f64,
    pub background: Background,
    pub simplify: SimplifySpec,
}

/// The numbers of one picture.
#[derive(Debug, Clone)]
pub struct Metrics {
    pub name: String,
    /// The picture, and the simplified picture, in pixels.
    pub size_px: (u32, u32),
    pub detail_px: (u32, u32),
    /// The design size, mm.
    pub size_mm: (f64, f64),
    pub colors: usize,
    pub objects: usize,
    pub satins: usize,
    pub stitches: usize,
    pub trims: usize,
    /// The part of the picture (its pixels with color) that the stitches
    /// cover, in percent.
    pub covered: f64,
    /// The stitches where the picture is clear, in percent of the pixels
    /// with color.
    pub outside: f64,
    /// The mean color difference between the stitches and the picture.
    pub delta_e: f64,
    /// The time of each step.
    pub times: Vec<(&'static str, Duration)>,
    /// The picture of the stitches (clear where there are none), at
    /// `PX_PER_MM`.
    pub stitch_picture: RgbaImage,
}

/// A number from 0 to 1 for a pixel and a seed, the same each time.
fn noise(x: u32, y: u32, seed: u32) -> f64 {
    let mut h = x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    f64::from(h & 0xffff) / 65535.0
}

fn rgba(c: [f64; 3]) -> Rgba<u8> {
    let b = |v: f64| v.round().clamp(0.0, 255.0) as u8;
    Rgba([b(c[0]), b(c[1]), b(c[2]), 255])
}

/// Each 8 × 8 block of each channel through a DCT, a coarse quantization
/// (steps of `q`, larger for the fine waves) and back: the blocks and the
/// rings at the edges of a JPEG of low quality.
fn jpeg_like(img: &RgbaImage, q: f64) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut out = img.clone();
    let c = |k: usize| if k == 0 { std::f64::consts::FRAC_1_SQRT_2 } else { 1.0 };
    let mut cos = [[0.0; 8]; 8];
    for (x, row) in cos.iter_mut().enumerate() {
        for (u, v) in row.iter_mut().enumerate() {
            *v = math::cos((2.0 * x as f64 + 1.0) * u as f64 * std::f64::consts::PI / 16.0);
        }
    }
    for by in (0..h).step_by(8) {
        for bx in (0..w).step_by(8) {
            for ch in 0..3 {
                let at = |x: usize, y: usize| {
                    let (px, py) = ((bx + x as u32).min(w - 1), (by + y as u32).min(h - 1));
                    f64::from(img.get_pixel(px, py)[ch]) - 128.0
                };
                let mut f = [[0.0; 8]; 8];
                for (u, fu) in f.iter_mut().enumerate() {
                    for (v, fv) in fu.iter_mut().enumerate() {
                        let mut s = 0.0;
                        for x in 0..8 {
                            for y in 0..8 {
                                s += at(x, y) * cos[x][u] * cos[y][v];
                            }
                        }
                        let step = q * (1.0 + (u + v) as f64);
                        *fv = (0.25 * c(u) * c(v) * s / step).round() * step;
                    }
                }
                for x in 0..8u32 {
                    for y in 0..8u32 {
                        let (px, py) = (bx + x, by + y);
                        if px >= w || py >= h {
                            continue;
                        }
                        let mut s = 0.0;
                        for (u, fu) in f.iter().enumerate() {
                            for (v, fv) in fu.iter().enumerate() {
                                s += c(u) * c(v) * fv * cos[x as usize][u] * cos[y as usize][v];
                            }
                        }
                        out.get_pixel_mut(px, py)[ch] = (0.25 * s + 128.0).round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
    out
}

/// A logo on white: a navy ring, a red disk in it, a gold star and three
/// green bars, through [`jpeg_like`].
fn logo() -> RgbaImage {
    let star: Vec<Point> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 70.0 } else { 30.0 };
            let (s, c) = math::sin_cos_deg(f64::from(i) * 36.0 - 90.0);
            Point::new(420.0 + r * c, 150.0 + r * s)
        })
        .collect();
    let img = RgbaImage::from_fn(600, 360, |x, y| {
        let p = Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
        let r = p.distance(Point::new(150.0, 170.0));
        if r < 45.0 {
            rgba([200.0, 30.0, 40.0])
        } else if (65.0..100.0).contains(&r) {
            rgba([25.0, 40.0, 110.0])
        } else if emb_geom::region::ring_contains(&star, p) {
            rgba([235.0, 175.0, 30.0])
        } else if (330..560).contains(&x) && [250..262, 280..292, 310..322].iter().any(|b| b.contains(&y)) {
            rgba([30.0, 140.0, 60.0])
        } else {
            rgba([255.0, 255.0, 255.0])
        }
    });
    jpeg_like(&img, 6.0)
}

/// A house icon of 64 × 64 pixels on a clear background.
fn icon() -> RgbaImage {
    RgbaImage::from_fn(64, 64, |x, y| {
        let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        let roof = fy > 8.0 && fy < 28.0 && (fx - 32.0).abs() < (fy - 8.0) * 1.2;
        if roof {
            rgba([190.0, 40.0, 30.0])
        } else if (27..37).contains(&x) && (42..58).contains(&y) {
            rgba([110.0, 70.0, 30.0])
        } else if (12..52).contains(&x) && (28..58).contains(&y) {
            rgba([240.0, 210.0, 90.0])
        } else {
            Rgba([0, 0, 0, 0])
        }
    })
}

/// A large picture (2400 × 1600 pixels): three shapes on a background
/// that shades from left to right, with fine grain.
fn large() -> RgbaImage {
    RgbaImage::from_fn(2400, 1600, |x, y| {
        let (fx, fy) = (f64::from(x), f64::from(y));
        let n = noise(x, y, 3) * 16.0 - 8.0;
        let c = if (fx - 700.0).hypot(fy - 800.0) < 450.0 {
            [40.0, 90.0, 170.0]
        } else if (1300.0..2100.0).contains(&fx) && (300.0..700.0).contains(&fy) {
            [220.0, 120.0, 30.0]
        } else if (1300.0..2100.0).contains(&fx) && (1000.0..1080.0).contains(&fy) {
            [30.0, 30.0, 30.0]
        } else {
            let g = 200.0 + fx * 0.02;
            [g, g, g - 8.0]
        };
        rgba([c[0] + n, c[1] + n, c[2] + n])
    })
}

/// A leaf with soft edges (the alpha goes from 0 to 255 over 3 pixels) on
/// a clear background whose hidden color is magenta, with a dark vein.
fn soft_edges() -> RgbaImage {
    RgbaImage::from_fn(400, 400, |x, y| {
        let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        // Two circles that overlap: the leaf is where both are.
        let d = (150.0 - (fx - 130.0).hypot(fy - 200.0)).min(150.0 - (fx - 270.0).hypot(fy - 200.0));
        let alpha = ((d + 1.5) / 3.0).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return Rgba([255, 0, 255, 0]);
        }
        let c = if (fy - 200.0).abs() < 4.0 && (80.0..320.0).contains(&fx) {
            [30.0, 70.0, 30.0]
        } else {
            [70.0, 170.0, 60.0]
        };
        let mut p = rgba(c);
        p[3] = (alpha * 255.0).round() as u8;
        p
    })
}

/// Line art: a face and a box with black outlines 5 pixels wide, filled
/// with yellow and light blue, on white.
fn line_art() -> RgbaImage {
    RgbaImage::from_fn(500, 400, |x, y| {
        let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        let r = (fx - 160.0).hypot(fy - 200.0);
        let eye = (fx - 130.0).hypot(fy - 170.0) < 12.0 || (fx - 190.0).hypot(fy - 170.0) < 12.0;
        let mouth = ((fx - 160.0).hypot(fy - 200.0) - 70.0).abs() < 2.5 && fy > 230.0;
        let in_box = (310.0..460.0).contains(&fx) && (120.0..300.0).contains(&fy);
        let box_edge = in_box && !(315.0..455.0).contains(&fx) || in_box && !(125.0..295.0).contains(&fy);
        if (r - 110.0).abs() < 2.5 || eye || mouth || box_edge {
            rgba([15.0, 15.0, 15.0])
        } else if r < 110.0 {
            rgba([250.0, 215.0, 60.0])
        } else if in_box {
            rgba([140.0, 190.0, 240.0])
        } else {
            rgba([255.0, 255.0, 255.0])
        }
    })
}

/// A portrait-like picture of 800 × 1000 pixels: a face with hair, eyes
/// and a mouth, shaded and with grain, on a busy background of stripes
/// and noise.
fn portrait() -> RgbaImage {
    RgbaImage::from_fn(800, 1000, |x, y| {
        let (fx, fy) = (f64::from(x), f64::from(y));
        let n = noise(x, y, 7) * 36.0 - 18.0;
        let face = ((fx - 400.0) / 230.0).powi(2) + ((fy - 520.0) / 300.0).powi(2) < 1.0;
        let hair = ((fx - 400.0) / 260.0).powi(2) + ((fy - 420.0) / 280.0).powi(2) < 1.0 && fy < 380.0;
        let eye = (fx - 320.0).hypot(fy - 480.0) < 28.0 || (fx - 480.0).hypot(fy - 480.0) < 28.0;
        let mouth = ((fx - 400.0) / 80.0).powi(2) + ((fy - 680.0) / 22.0).powi(2) < 1.0;
        let c = if eye {
            [45.0, 35.0, 30.0]
        } else if mouth {
            [170.0, 60.0, 70.0]
        } else if hair {
            [70.0, 45.0, 25.0]
        } else if face {
            [225.0 - fy * 0.05, 175.0 - fy * 0.03, 145.0]
        } else if (x / 40 + y / 90) % 2 == 0 {
            [90.0, 130.0 + noise(x / 9, y / 9, 1) * 60.0, 90.0]
        } else {
            [150.0, 150.0, 190.0 + noise(x / 7, y / 7, 2) * 50.0]
        };
        rgba([c[0] + n, c[1] + n, c[2] + n])
    })
}

/// A badge on white: a navy disk with twelve white dots and six gold
/// squares on it (small details) and a red disk in the middle (a large
/// one).
fn badge() -> RgbaImage {
    RgbaImage::from_fn(500, 500, |x, y| {
        let p = Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
        let c = Point::new(250.0, 250.0);
        let r = p.distance(c);
        let at = |k: u32, n: u32, radius: f64| {
            let (s, co) = math::sin_cos_deg(f64::from(k) * 360.0 / f64::from(n));
            c + Point::new(radius * co, radius * s)
        };
        let dot = (0..12).any(|k| p.distance(at(k, 12, 190.0)) < 10.0);
        let square = (0..6).any(|k| {
            let q = at(k, 6, 120.0);
            (p.x - q.x).abs() < 15.0 && (p.y - q.y).abs() < 15.0
        });
        if r >= 230.0 {
            rgba([255.0, 255.0, 255.0])
        } else if r < 60.0 {
            rgba([200.0, 30.0, 40.0])
        } else if dot {
            rgba([250.0, 250.0, 250.0])
        } else if square {
            rgba([235.0, 175.0, 30.0])
        } else {
            rgba([25.0, 40.0, 110.0])
        }
    })
}

/// The Simplify settings of the wizard at its start, with `colors`.
fn simplify(colors: usize, pixel_mm: f64) -> SimplifySpec {
    SimplifySpec {
        pixel_mm,
        colors,
        speck_mm2: 2.0,
        clean_edges: true,
        denoise: 0,
        smooth: SMOOTH,
    }
}

/// The test pictures that the program makes.
pub fn pictures() -> Vec<BenchPicture> {
    let p =
        |name: &str, image: RgbaImage, width_mm: f64, background: Background, simplify: SimplifySpec| BenchPicture {
            name: name.into(),
            image,
            width_mm,
            background,
            simplify,
        };
    vec![
        p("jpeg-logo", logo(), 90.0, Background::EdgeColor, simplify(5, 0.3)),
        p("small-icon", icon(), 40.0, Background::Keep, simplify(4, 0.4)),
        p("large-picture", large(), 120.0, Background::EdgeColor, simplify(5, 0.4)),
        p("soft-edges", soft_edges(), 60.0, Background::Keep, simplify(2, 0.3)),
        p("line-art", line_art(), 80.0, Background::EdgeColor, simplify(4, 0.25)),
        p("badge", badge(), 60.0, Background::EdgeColor, simplify(4, 0.2)),
        p(
            "busy-portrait",
            portrait(),
            80.0,
            Background::CutOut([140.0, 120.0, 680.0, 840.0]),
            SimplifySpec {
                denoise: 1,
                ..simplify(6, 0.4)
            },
        ),
    ]
}

/// The settings of the pictures of a folder.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FolderSettings {
    pub width_mm: f64,
    pub background: Background,
    pub simplify: SimplifySpec,
}

impl Default for FolderSettings {
    /// 100 mm wide, 6 colors, the other settings as the wizard has them at
    /// its start (with noise smoothing 1).
    fn default() -> Self {
        FolderSettings {
            width_mm: 100.0,
            background: Background::EdgeColor,
            simplify: SimplifySpec {
                denoise: 1,
                ..simplify(6, 0.4)
            },
        }
    }
}

/// The pictures of a folder (PNG, JPEG, BMP, TIFF, GIF), with the
/// `settings`; a picture that cannot be read is an
/// error line.
pub fn pictures_in(dir: &Path, settings: &FolderSettings) -> Result<Vec<Result<BenchPicture, String>>, String> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                ["png", "jpg", "jpeg", "bmp", "tif", "tiff", "gif"].contains(&e.to_lowercase().as_str())
            })
        })
        .collect();
    files.sort();
    Ok(files
        .iter()
        .map(|f| {
            let name = f
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().into_owned());
            let bytes = std::fs::read(f).map_err(|e| format!("{name}: {e}"))?;
            let image = emb_art::decode(&bytes).map_err(|e| format!("{name}: {e}"))?;
            Ok(BenchPicture {
                name,
                image,
                width_mm: settings.width_mm,
                background: settings.background,
                simplify: settings.simplify,
            })
        })
        .collect())
}

/// Runs the wizard steps on `p` and measures the result.
pub fn run(p: &BenchPicture) -> Metrics {
    let mut times = Vec::new();
    let mut clock = Instant::now();
    let mut lap = |name: &'static str, times: &mut Vec<(&'static str, Duration)>| {
        times.push((name, clock.elapsed()));
        clock = Instant::now();
    };
    // Background (the wizard keeps the trim on).
    let mut cut = smart::work_picture(&p.image);
    let mm_per_px = p.width_mm / f64::from(cut.width().max(1));
    match p.background {
        Background::Keep => {}
        Background::EdgeColor => {
            background::remove_edges(&mut cut, background::DEFAULT_TOLERANCE);
        }
        Background::CutOut(rect) => {
            if let Some(mask) = emb_art::cutout::cutout(&cut, Some(rect), &[]) {
                for (k, px) in cut.pixels_mut().enumerate() {
                    if !mask[k] {
                        px[3] = 0;
                    }
                }
            }
        }
    }
    let content = background::trim(&cut).map_or(cut, |(c, _)| c);
    lap("background", &mut times);
    // Simplify.
    let width_mm = smart::fit_width(
        f64::from(content.width()) * mm_per_px,
        content.width(),
        content.height(),
    );
    let r = smart::simplify(&content, width_mm, &p.simplify, None);
    lap("simplify", &mut times);
    // Stitches.
    let choices = vec![ColorChoice::default(); r.palette.len()];
    let order: Vec<u16> = (0..r.palette.len() as u16).collect();
    let k = width_mm / f64::from(r.width.max(1));
    let objs = smart::digitize(
        &r,
        &Transform::scale(k, k),
        &choices,
        &order,
        &SmartSpec::default(),
        &CurrentProperties::default(),
    );
    lap("stitches", &mut times);
    let mut d = Design::new("");
    let threads: Vec<_> = r.palette.iter().map(|c| d.add_thread(c.to_hex(), *c)).collect();
    let satins = objs
        .iter()
        .filter(|o| matches!(o.kind, ObjectKind::Satin { .. }))
        .count();
    let objects = objs.len();
    let mut colors: Vec<u16> = objs.iter().map(|o| o.color).collect();
    colors.sort_unstable();
    colors.dedup();
    for o in objs {
        d.add_object(threads[o.color as usize], o.kind);
    }
    let plan = emb_stitch::build_plan(&d, &Default::default(), &mut Default::default());
    lap("plan", &mut times);
    let height_mm = width_mm * f64::from(content.height()) / f64::from(content.width().max(1));
    let (covered, outside, delta_e, stitch_picture) = compare(&content, &plan.stitches, width_mm, height_mm);
    let s = &plan.stitches;
    Metrics {
        name: p.name.clone(),
        size_px: p.image.dimensions(),
        detail_px: (r.width, r.height),
        size_mm: (width_mm, height_mm),
        colors: colors.len(),
        objects,
        satins,
        stitches: s.records.iter().filter(|r| r.kind == StitchKind::Stitch).count(),
        trims: s.trim_count(),
        covered,
        outside,
        delta_e,
        times,
        stitch_picture,
    }
}

/// Compares a picture of the stitches with the picture (`width_mm` ×
/// `height_mm`, its top-left corner at 0, 0): the part covered, the part
/// outside (percent of the pixels with color), the mean ΔE, and the
/// picture of the stitches.
fn compare(
    content: &RgbaImage,
    s: &emb_model::StitchDesign,
    width_mm: f64,
    height_mm: f64,
) -> (f64, f64, f64, RgbaImage) {
    let segs = emb_render::segments(s);
    let b = BBox::new(Point::ZERO, Point::new(width_mm, height_mm));
    let pic = emb_render::raster::picture_at(&segs, &[], b, PX_PER_MM, THREAD_MM, None);
    let reference = smart::resample(content, pic.width);
    let (w, h) = (pic.width.min(reference.width()), pic.height.min(reference.height()));
    let (mut opaque, mut covered, mut outside, mut diff) = (0usize, 0usize, 0usize, 0.0);
    for y in 0..h {
        for x in 0..w {
            let k = (y * pic.width + x) as usize * 4;
            let stitched = pic.rgba[k + 3] >= 128;
            let r = reference.get_pixel(x, y);
            if r[3] < 128 {
                outside += usize::from(stitched);
                continue;
            }
            opaque += 1;
            if stitched {
                covered += 1;
                let thread = Rgb(pic.rgba[k], pic.rgba[k + 1], pic.rgba[k + 2]);
                diff += Lab::of(thread).distance(Lab::of(Rgb(r[0], r[1], r[2])));
            }
        }
    }
    let n = opaque.max(1) as f64;
    let picture = RgbaImage::from_raw(pic.width, pic.height, pic.rgba).unwrap_or_default();
    (
        100.0 * covered as f64 / n,
        100.0 * outside as f64 / n,
        diff / covered.max(1) as f64,
        picture,
    )
}

/// The numbers of the pictures as text; with `times`, also the time of
/// each step (the golden snapshot has no times: they change).
pub fn report(results: &[Metrics], times: bool) -> String {
    let mut out = String::new();
    for m in results {
        out.push_str(&format!(
            "{}: {} × {} px, simplified {} × {} px, {:.1} × {:.1} mm\n",
            m.name, m.size_px.0, m.size_px.1, m.detail_px.0, m.detail_px.1, m.size_mm.0, m.size_mm.1
        ));
        out.push_str(&format!(
            "  colors {}, objects {} (satins {}), stitches {}, trims {}\n",
            m.colors, m.objects, m.satins, m.stitches, m.trims
        ));
        out.push_str(&format!(
            "  covered {:.1} %, outside {:.1} %, color difference {:.1}\n",
            m.covered, m.outside, m.delta_e
        ));
        if times {
            let t: Vec<String> = m
                .times
                .iter()
                .map(|(n, d)| format!("{n} {} ms", d.as_millis()))
                .collect();
            out.push_str(&format!("  times: {}\n", t.join(", ")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stitches_cover_the_picture_where_it_has_color() {
        // A red square in the top-left quarter of a clear picture: the
        // stitches are there (the picture and the stitches line up).
        let img = RgbaImage::from_fn(80, 80, |x, y| {
            if x < 40 && y < 40 {
                Rgba([200, 30, 30, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        });
        let p = BenchPicture {
            name: "square".into(),
            image: img,
            width_mm: 40.0,
            background: Background::Keep,
            simplify: simplify(2, 0.5),
        };
        let m = run(&p);
        assert_eq!((m.colors, m.objects), (1, 1));
        assert!(m.covered > 95.0 && m.outside < 5.0, "{m:?}");
        assert!(m.delta_e < 12.0, "{m:?}");
        assert_eq!(m.times.len(), 4);
    }

    #[test]
    fn jpeg_like_blocks_keep_flat_areas_and_ring_at_edges() {
        let flat = RgbaImage::from_pixel(16, 16, Rgba([120, 120, 120, 255]));
        assert_eq!(jpeg_like(&flat, 6.0), flat);
        let edge = RgbaImage::from_fn(16, 16, |x, _| {
            if x < 5 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        });
        let j = jpeg_like(&edge, 6.0);
        assert!(j.pixels().zip(edge.pixels()).any(|(a, b)| a != b));
    }
}
