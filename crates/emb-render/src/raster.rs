//! A picture of the stitches (the design picture for customers and web
//! pages): each stitch is a thread with soft edges, drawn in sew order.
//! With a [`Look`] that has a lighting, each stitch has light and shade,
//! the twist of its plies and a shadow, and the background can have the
//! texture of the fabric (the 3D thread look, see [`crate::thread3d`]).
//! PNG output uses the `image` crate (pure Rust).

use emb_geom::{BBox, Point};
use emb_model::Rgb;

use crate::thread3d::{Lighting, Weave, fabric_value, lit, ply};
use crate::{Segment, SegmentKind};

/// The darkness of the shadow of a thread (0 to 1).
const SHADOW_ALPHA: f64 = 0.12;

/// How a picture draws the threads and the background.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Look {
    /// Light and shade (the 3D thread look); `None`: the flat thread look.
    pub lighting: Option<Lighting>,
    /// The texture of the background color (with a background only).
    pub fabric: Option<Weave>,
}

impl Look {
    /// The 3D thread look with the default light, on `fabric`.
    pub fn three_d(fabric: Option<Weave>) -> Look {
        Look {
            lighting: Some(Lighting::default()),
            fabric,
        }
    }
}

/// RGBA pixels, row by row from the top.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Areas to fill under the stitches: rings (even-odd) and a color.
pub type Areas = [(Vec<Vec<Point>>, Rgb)];

/// Draws the stitches at `px_per_mm` with a margin of `margin_mm`, over the
/// `areas` (appliqué fabric). The background is transparent, or the color given.
pub fn picture(
    segments: &[Segment],
    areas: &Areas,
    px_per_mm: f64,
    thread_mm: f64,
    margin_mm: f64,
    background: Option<Rgb>,
) -> Picture {
    picture_look(
        segments,
        areas,
        px_per_mm,
        thread_mm,
        margin_mm,
        background,
        &Look::default(),
    )
}

/// As [`picture`], with the `look` of the threads and the background.
pub fn picture_look(
    segments: &[Segment],
    areas: &Areas,
    px_per_mm: f64,
    thread_mm: f64,
    margin_mm: f64,
    background: Option<Rgb>,
    look: &Look,
) -> Picture {
    // The stitches and the areas (appliqué fabric, rhinestones).
    let b = BBox::from_points(
        segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Stitch)
            .flat_map(|s| [s.a, s.b])
            .chain(areas.iter().flat_map(|(rings, _)| rings.iter().flatten().copied())),
    )
    .unwrap_or(BBox::new(Point::ZERO, Point::ZERO))
    .expand(margin_mm + thread_mm);
    picture_at_look(segments, areas, b, px_per_mm, thread_mm, background, look)
}

/// As [`picture`], for the area `b` (mm): the picture shows only that area.
pub fn picture_at(
    segments: &[Segment],
    areas: &Areas,
    b: BBox,
    px_per_mm: f64,
    thread_mm: f64,
    background: Option<Rgb>,
) -> Picture {
    picture_at_look(segments, areas, b, px_per_mm, thread_mm, background, &Look::default())
}

/// As [`picture_at`], with the `look` of the threads and the background.
pub fn picture_at_look(
    segments: &[Segment],
    areas: &Areas,
    b: BBox,
    px_per_mm: f64,
    thread_mm: f64,
    background: Option<Rgb>,
    look: &Look,
) -> Picture {
    let width = ((b.width() * px_per_mm).ceil() as u32).clamp(1, 16_384);
    let height = ((b.height() * px_per_mm).ceil() as u32).clamp(1, 16_384);
    let mut rgba = vec![0u8; width as usize * height as usize * 4];
    if let Some(c) = background {
        match look.fabric {
            Some(weave) => {
                for (i, px) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    let (x, y) = ((i % width as usize) as f64 + 0.5, (i / width as usize) as f64 + 0.5);
                    let v = fabric_value(weave, b.min.x + x / px_per_mm, b.min.y + y / px_per_mm);
                    let l = lit(c, (v, 0.0));
                    px.copy_from_slice(&[l[0], l[1], l[2], 255]);
                }
            }
            None => {
                for px in rgba.as_chunks_mut::<4>().0 {
                    px.copy_from_slice(&[c.0, c.1, c.2, 255]);
                }
            }
        }
    }
    let to_px = |p: Point| ((p.x - b.min.x) * px_per_mm, (p.y - b.min.y) * px_per_mm);
    for (rings, color) in areas {
        let rings: Vec<Vec<(f64, f64)>> = rings.iter().map(|r| r.iter().map(|q| to_px(*q)).collect()).collect();
        for py in 0..height {
            let yc = f64::from(py) + 0.5;
            let mut xs: Vec<f64> = Vec::new();
            for r in &rings {
                for i in 0..r.len() {
                    let (a, c) = (r[i], r[(i + 1) % r.len()]);
                    if (a.1 <= yc) != (c.1 <= yc) {
                        xs.push(a.0 + (yc - a.1) / (c.1 - a.1) * (c.0 - a.0));
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for pair in xs.as_chunks::<2>().0 {
                let x0 = pair[0].round().clamp(0.0, f64::from(width)) as u32;
                let x1 = pair[1].round().clamp(0.0, f64::from(width)) as u32;
                for px in x0..x1 {
                    let i = (py as usize * width as usize + px as usize) * 4;
                    rgba[i..i + 4].copy_from_slice(&[color.0, color.1, color.2, 255]);
                }
            }
        }
    }
    let mut pic = Picture { width, height, rgba };
    stitches_onto_look(&mut pic, b, segments, px_per_mm, thread_mm, look.lighting.as_ref());
    pic
}

/// Draws the stitches over the picture `pic`, which shows the area `b` at
/// `px_per_mm`.
pub fn stitches_onto(pic: &mut Picture, b: BBox, segments: &[Segment], px_per_mm: f64, thread_mm: f64) {
    stitches_onto_look(pic, b, segments, px_per_mm, thread_mm, None);
}

/// Blends `color` with coverage `a` over the pixel `dst`.
fn blend(dst: &mut [u8], color: [f64; 3], a: f64) {
    let da = f64::from(dst[3]) / 255.0;
    let out_a = a + da * (1.0 - a);
    for k in 0..3 {
        let v = (color[k] * a + f64::from(dst[k]) * da * (1.0 - a)) / out_a.max(1e-9);
        dst[k] = v.round().clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).round() as u8;
}

/// Calls `paint(pixel, cover, s, t, d)` for each pixel near the thread
/// from `a` to `b` (pixels) of radius `r`: `cover` is the part of the pixel
/// that the thread covers (one pixel of soft edge), `s` the place across
/// the thread (−1 to 1, positive on the left of the direction), `t` the
/// place along it (0 to 1) and `d` the distance from the middle line of
/// the thread as a part of `r`.
fn each_pixel(
    pic: &mut Picture,
    a: (f64, f64),
    b: (f64, f64),
    r: f64,
    mut paint: impl FnMut(&mut [u8], f64, f64, f64, f64),
) {
    let (width, height) = (pic.width, pic.height);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let len = len2.sqrt();
    let (nx, ny) = if len > 0.0 { (dy / len, -dx / len) } else { (0.0, 0.0) };
    let x0 = (a.0.min(b.0) - r - 1.0).floor().max(0.0) as u32;
    let y0 = (a.1.min(b.1) - r - 1.0).floor().max(0.0) as u32;
    let x1 = ((a.0.max(b.0) + r + 1.0).ceil().max(0.0) as u32).min(width);
    let y1 = ((a.1.max(b.1) + r + 1.0).ceil().max(0.0) as u32).min(height);
    for py in y0..y1 {
        for px in x0..x1 {
            let (cx, cy) = (f64::from(px) + 0.5, f64::from(py) + 0.5);
            let t = if len2 > 0.0 {
                (((cx - a.0) * dx + (cy - a.1) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let (qx, qy) = (cx - (a.0 + t * dx), cy - (a.1 + t * dy));
            let d = (qx * qx + qy * qy).sqrt();
            let cover = (r + 0.5 - d).clamp(0.0, 1.0);
            if cover <= 0.0 {
                continue;
            }
            let s = if r > 0.0 { (qx * nx + qy * ny) / r } else { 0.0 };
            let i = (py as usize * width as usize + px as usize) * 4;
            let dr = if r > 0.0 { d / r } else { 0.0 };
            paint(&mut pic.rgba[i..i + 4], cover, s, t, dr);
        }
    }
}

/// As [`stitches_onto`]; with a `lighting`, each stitch has light and
/// shade, the twist of its plies and a shadow (the 3D thread look).
pub fn stitches_onto_look(
    pic: &mut Picture,
    b: BBox,
    segments: &[Segment],
    px_per_mm: f64,
    thread_mm: f64,
    lighting: Option<&Lighting>,
) {
    let to_px = |p: Point| ((p.x - b.min.x) * px_per_mm, (p.y - b.min.y) * px_per_mm);
    let r = thread_mm * px_per_mm / 2.0;
    for s in segments.iter().filter(|s| s.kind == SegmentKind::Stitch) {
        let (a, e) = (to_px(s.a), to_px(s.b));
        match lighting {
            Some(l) if r > 1.0 => {
                let (dx, dy) = (e.0 - a.0, e.1 - a.1);
                let len = (dx * dx + dy * dy).sqrt();
                let u = if len > 0.0 { (dx / len, dy / len) } else { (1.0, 0.0) };
                // The shadow on what is under the thread.
                let (sx, sy) = l.shadow_dir();
                let off = l.shadow_length() * r;
                let (sa, sb) = ((a.0 + sx * off, a.1 + sy * off), (e.0 + sx * off, e.1 + sy * off));
                each_pixel(pic, sa, sb, r, |dst, cover, _, _, _| {
                    blend(dst, [0.0; 3], SHADOW_ALPHA * cover * l.strength.clamp(0.0, 1.0));
                });
                let len_mm = len / px_per_mm;
                each_pixel(pic, a, e, r, |dst, cover, sv, t, _| {
                    let (f, h) = l.shade(u, sv, t);
                    let f = f * ply(t * len_mm, sv, l.strength);
                    let c = lit(s.color, (f, h));
                    blend(dst, c.map(f64::from), cover);
                });
            }
            _ => {
                // The flat thread look: the middle of the thread is lighter.
                each_pixel(pic, a, e, r, |dst, cover, _, _, d| {
                    let shade = if r > 1.5 { 0.75 + 0.45 * (1.0 - d.min(1.0)) } else { 1.0 };
                    let c = [s.color.0, s.color.1, s.color.2].map(|v| (f64::from(v) * shade).min(255.0));
                    blend(dst, c, cover);
                });
            }
        }
    }
}

impl Picture {
    /// The picture as a PNG file.
    pub fn png(&self) -> Vec<u8> {
        use image::ImageEncoder;
        let mut out = Vec::new();
        let enc = image::codecs::png::PngEncoder::new(&mut out);
        enc.write_image(&self.rgba, self.width, self.height, image::ExtendedColorType::Rgba8)
            .expect("an RGBA buffer of the right size encodes");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_a_stitch() {
        let seg = Segment {
            a: Point::new(0.0, 0.0),
            b: Point::new(10.0, 0.0),
            color: Rgb(200, 0, 0),
            kind: SegmentKind::Stitch,
        };
        let jump = Segment {
            kind: SegmentKind::Jump,
            b: Point::new(10.0, 50.0),
            ..seg
        };
        let p = picture(&[seg, jump], &[], 10.0, 0.4, 1.0, None);
        // 10 mm + 2 x (1 + 0.4) mm, at 10 px/mm; the jump is not drawn.
        assert_eq!((p.width, p.height), (128, 28));
        let at = |x: u32, y: u32| &p.rgba[((y * p.width + x) * 4) as usize..][..4];
        assert_eq!(at(64, 14)[3], 255, "on the thread");
        assert!(at(64, 14)[0] > 150);
        assert_eq!(at(64, 2)[3], 0, "transparent off the thread");
        let png = p.png();
        assert_eq!(&png[1..4], b"PNG");
        let back = image::load_from_memory(&png).unwrap();
        assert_eq!((back.width(), back.height()), (128, 28));
    }

    #[test]
    fn background() {
        let p = picture(&[], &[], 4.0, 0.4, 2.0, Some(Rgb(1, 2, 3)));
        assert_eq!(&p.rgba[..4], &[1, 2, 3, 255]);
        // A fabric area under a stitch, with a hole.
        let seg = Segment {
            a: Point::new(0.0, 0.0),
            b: Point::new(20.0, 20.0),
            color: Rgb(0, 0, 0),
            kind: SegmentKind::Stitch,
        };
        let ring = |a: f64, b: f64| vec![Point::new(a, a), Point::new(b, a), Point::new(b, b), Point::new(a, b)];
        let p = picture(
            &[seg],
            &[(vec![ring(0.0, 20.0), ring(8.0, 12.0)], Rgb(9, 8, 7))],
            4.0,
            0.2,
            0.0,
            None,
        );
        let at = |x: u32, y: u32| &p.rgba[((y * p.width + x) * 4) as usize..][..4];
        assert_eq!(at(70, 10), &[9, 8, 7, 255], "fabric");
        assert_eq!(at(45, 38)[3], 0, "the hole is empty");
    }

    #[test]
    fn the_3d_look_has_light_shade_and_a_shadow() {
        // A horizontal stitch, 1 mm wide at 20 px/mm, on white fabric.
        let seg = Segment {
            a: Point::new(0.0, 0.0),
            b: Point::new(10.0, 0.0),
            color: Rgb(120, 120, 120),
            kind: SegmentKind::Stitch,
        };
        let white = Some(Rgb(255, 255, 255));
        let flat = picture(&[seg], &[], 20.0, 1.0, 1.0, white);
        let look = Look::three_d(None);
        let lit = picture_look(&[seg], &[], 20.0, 1.0, 1.0, white, &look);
        assert_eq!((flat.width, flat.height), (lit.width, lit.height));
        let at = |p: &Picture, x: u32, y: u32| p.rgba[((y * p.width + x) * 4) as usize];
        // The picture is 2 mm (40 px) round the stitch: the thread is from
        // y = 30 to 50 (px).
        let (x, top, bottom) = (lit.width / 2, 32, 47);
        // The light is at the top left: the top edge is lighter than the bottom edge.
        assert!(
            at(&lit, x, top) > at(&lit, x, bottom) + 30,
            "{} {}",
            at(&lit, x, top),
            at(&lit, x, bottom)
        );
        // In the flat look both edges are the same.
        assert_eq!(at(&flat, x, top), at(&flat, x, bottom));
        // The shadow falls below the thread (to the bottom right) on the white fabric.
        assert!(at(&lit, x, 54) < 250, "{}", at(&lit, x, 54));
        assert_eq!(at(&flat, x, 54), 255);
        // A fabric texture on the background.
        let fab = picture_look(
            &[],
            &[],
            20.0,
            1.0,
            1.0,
            Some(Rgb(200, 200, 200)),
            &Look::three_d(Some(Weave::Twill)),
        );
        let vals: std::collections::BTreeSet<u8> = fab.rgba.chunks(4).map(|p| p[0]).collect();
        assert!(vals.len() > 5, "the fabric has a texture");
    }
}
