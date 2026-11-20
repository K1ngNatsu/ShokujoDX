//! The pictures of the program from the surface (P18 view plan,
//! Milestone 9, ADR 0130): the design picture, `emb-cli output`, the
//! approval sheet, the library and the file manager, the kiosk, the wizard
//! and the font previews show the stitches as the canvas does, with their
//! heights, their sheen and their shadows.
//!
//! [`Embroidery`] lays the stitches one time (with the foam, the cut lines
//! and the pieces of a design, or the stitches of a machine file alone).
//! [`Embroidery::picture`] makes a picture of an area in blocks of
//! [`BLOCK`] pixels on all processors: each block is a surface with an
//! apron round it, so a pixel is the same in each block, and a large
//! picture needs little memory. A block with no stitch near it is the
//! fabric alone. The `detail` is the least number of pixels for each mm
//! that the surface is made with before it is halved to the size of the
//! picture: [`FULL_DETAIL`] for the pictures, [`FAST_DETAIL`] for the
//! thumbnails of the file manager.

use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};

use emb_geom::{BBox, Point};
use emb_model::{Design, ObjectId, Rgb, StitchDesign};

use crate::fabric::Ground;
use crate::raster::{Look, Picture};
use crate::strand::Strand;
use crate::surface::{Laid, Piece, Surface};
use crate::thread3d::Lighting;
use crate::{Segment, SegmentKind};

/// The side of a block of a picture, pixels.
pub const BLOCK: u32 = 256;
/// The detail of the pictures: as the canvas.
pub const FULL_DETAIL: f64 = crate::surface::DETAIL_PX_PER_MM;
/// The detail of a quick picture (the thumbnails of the file manager).
pub const FAST_DETAIL: f64 = 4.0;

/// The frames of the sew-out animation (`emb-cli output --sewout-gif`, the
/// Output window), the longest side of its picture in pixels, the time of a
/// frame and of the last frame, ms.
pub const SEWOUT_FRAMES: usize = 36;
pub const SEWOUT_PX: f64 = 400.0;
pub const SEWOUT_FRAME_MS: u32 = 120;
pub const SEWOUT_LAST_MS: u32 = 2500;

/// The stitches of a design laid for pictures.
pub struct Embroidery {
    laid: Laid,
    ground: Option<Ground>,
    /// The number of records of the stitches (a step of a sew-out).
    records: u32,
}

impl Embroidery {
    /// The stitches `stitches` of `design` (its plan: `spans`, the records
    /// of each object) on the fabric `ground`, with the foam of puff, the
    /// cut lines and the pieces of the design (output coordinates).
    pub fn of_design(
        design: &Design,
        stitches: &StitchDesign,
        spans: impl IntoIterator<Item = (ObjectId, Range<usize>)>,
        ground: Option<Ground>,
    ) -> Embroidery {
        let marks = crate::strand::marks(design, spans);
        let strands = crate::strand::strands(stitches, &marks);
        Embroidery {
            laid: crate::surface::lay(&strands, &crate::strand::pieces(design)),
            ground,
            records: stitches.records.len() as u32,
        }
    }

    /// The stitches of a machine file (no objects): each stitch segment is
    /// a thread.
    pub fn of_stitches(stitches: &StitchDesign) -> Embroidery {
        let strands = crate::strand::strands(stitches, &Default::default());
        Embroidery {
            laid: crate::surface::lay(&strands, &[]),
            ground: None,
            records: stitches.records.len() as u32,
        }
    }

    /// The stitch segments `segments` (any coordinates) as threads, on the
    /// `pieces`.
    pub fn of_segments(segments: &[Segment], pieces: &[Piece]) -> Embroidery {
        let strands: Vec<Strand> = segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Stitch)
            .enumerate()
            .map(|(i, s)| Strand {
                a: s.a,
                b: s.b,
                color: s.color,
                kind: s.thread_kind,
                weight: s.thread_weight,
                lift_mm: 0.0,
                record: i as u32,
            })
            .collect();
        Embroidery {
            records: strands.len() as u32,
            laid: crate::surface::lay(&strands, pieces),
            ground: None,
        }
    }

    /// The laid stitches.
    pub fn laid(&self) -> &Laid {
        &self.laid
    }

    /// The area of the stitches and the pieces, with the width of the
    /// thread (`None` with no stitch).
    pub fn bounds(&self) -> Option<BBox> {
        (!self.laid.is_empty()).then(|| self.laid.bounds())
    }

    /// The picture of the area `b` at `px_per_mm` (`ceil(b × px_per_mm)`
    /// pixels, pixel `x` with its center at `b.min.x + (x + 0.5) /
    /// px_per_mm`) in the light of `look` (the default light if it has
    /// none): over `background` with the fabric of `look`, or with a clear
    /// background. The records that `show` keeps (the stitches up to a
    /// step: the picture of a sew-out that goes on).
    pub fn picture(
        &self,
        b: BBox,
        px_per_mm: f64,
        look: &Look,
        background: Option<Rgb>,
        detail: f64,
        show: &(dyn Fn(u32) -> bool + Sync),
    ) -> Picture {
        let width = ((b.width() * px_per_mm).ceil() as u32).clamp(1, 16_384);
        let height = ((b.height() * px_per_mm).ceil() as u32).clamp(1, 16_384);
        let lighting = look.lighting.unwrap_or_default();
        let fabric = background.zip(look.fabric);
        let (bx, by) = (width.div_ceil(BLOCK), height.div_ceil(BLOCK));
        let count = (bx * by) as usize;
        let next = AtomicUsize::new(0);
        let workers = std::thread::available_parallelism()
            .map_or(1, usize::from)
            .min(count)
            .max(1);
        let ground = self.ground.as_ref();
        // Each block, row by row; a block of a worker goes in its place.
        let blocks: Vec<(usize, Picture)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    scope.spawn(|| {
                        let mut out = Vec::new();
                        loop {
                            let k = next.fetch_add(1, Ordering::Relaxed);
                            if k >= count {
                                break out;
                            }
                            let (cx, cy) = (k as u32 % bx, k as u32 / bx);
                            let (x0, y0) = (cx * BLOCK, cy * BLOCK);
                            let (w, h) = (BLOCK.min(width - x0), BLOCK.min(height - y0));
                            let s = self
                                .laid
                                .block_from(
                                    b.min,
                                    px_per_mm,
                                    i64::from(x0),
                                    i64::from(y0),
                                    w,
                                    h,
                                    &lighting,
                                    ground,
                                    show,
                                    detail,
                                )
                                .unwrap_or_else(|| {
                                    let corner = Point::new(
                                        b.min.x + f64::from(x0) / px_per_mm,
                                        b.min.y + f64::from(y0) / px_per_mm,
                                    );
                                    Surface::empty(corner, px_per_mm, w, h, &lighting)
                                });
                            out.push((k, block_picture(&s, &lighting, fabric, background)));
                        }
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|t| t.join().expect("a thread of a picture does not stop with an error"))
                .collect()
        });
        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        for (k, p) in blocks {
            let (x0, y0) = ((k as u32 % bx) * BLOCK, (k as u32 / bx) * BLOCK);
            let row = p.width as usize * 4;
            for y in 0..p.height as usize {
                let at = ((y0 as usize + y) * width as usize + x0 as usize) * 4;
                rgba[at..at + row].copy_from_slice(&p.rgba[y * row..(y + 1) * row]);
            }
        }
        Picture { width, height, rgba }
    }
}

impl Embroidery {
    /// The sewing of the design in `frames` pictures of the area `b`: each
    /// shows the stitches up to its step, the last one all of them (the
    /// sew-out animation).
    pub fn sewing(&self, b: BBox, px_per_mm: f64, look: &Look, background: Option<Rgb>, frames: usize) -> Vec<Picture> {
        let frames = frames.max(1);
        (1..=frames)
            .map(|k| {
                let upto = (u64::from(self.records) * k as u64).div_ceil(frames as u64) as u32;
                let show = move |r: u32| k == frames || r < upto;
                self.picture(b, px_per_mm, look, background, FULL_DETAIL, &show)
            })
            .collect()
    }
}

/// The pictures `frames` as an animated GIF that plays again and again:
/// each `frame_ms`, the last one `last_ms`.
pub fn gif(frames: &[Picture], frame_ms: u32, last_ms: u32) -> Result<Vec<u8>, String> {
    use image::codecs::gif::{GifEncoder, Repeat};
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new_with_speed(&mut out, 10);
        enc.set_repeat(Repeat::Infinite).map_err(|e| e.to_string())?;
        for (i, f) in frames.iter().enumerate() {
            let img = image::RgbaImage::from_raw(f.width, f.height, f.rgba.clone())
                .ok_or_else(|| "a frame of the wrong size".to_string())?;
            let ms = if i + 1 == frames.len() { last_ms } else { frame_ms };
            let frame = image::Frame::from_parts(img, 0, 0, image::Delay::from_numer_denom_ms(ms, 1));
            enc.encode_frame(frame).map_err(|e| e.to_string())?;
        }
    }
    Ok(out)
}

/// The sew-out animation of a design (ADR 0130): the GIF of
/// [`Embroidery::sewing`] with [`SEWOUT_FRAMES`] frames, at most
/// [`SEWOUT_PX`] pixels on its longest side, on the fabric color of the
/// design (white if it has none) with no fabric texture (a GIF of a
/// texture is large; the shadows stay).
pub fn sewout_gif(
    design: &Design,
    stitches: &StitchDesign,
    spans: impl IntoIterator<Item = (ObjectId, Range<usize>)>,
    ground: Option<Ground>,
    look: &Look,
) -> Result<Vec<u8>, String> {
    let segments = crate::segments(stitches);
    let areas = crate::sheet::picture_areas(design);
    let e = Embroidery::of_design(design, stitches, spans, ground);
    let b = area_of(&segments, &areas, 2.0)
        .or_else(|| e.bounds())
        .ok_or_else(|| emb_geom::text::tr("The design has no stitches.").to_string())?;
    let ppm = SEWOUT_PX / b.width().max(b.height()).max(1.0);
    let look = Look {
        lighting: Some(look.lighting.unwrap_or_default()),
        fabric: None,
    };
    let background = Some(design.background.unwrap_or(Rgb(255, 255, 255)));
    let frames = e.sewing(b, ppm, &look, background, SEWOUT_FRAMES);
    gif(&frames, SEWOUT_FRAME_MS, SEWOUT_LAST_MS)
}

/// The picture of the surface of a block: with the fabric, lit; on a color
/// with no fabric texture, the threads and their shadows over it; with no
/// background, the threads and their shadows and gaps as a dark layer on
/// a clear background (so the picture has them on any background).
fn block_picture(
    s: &Surface,
    lighting: &Lighting,
    fabric: Option<(Rgb, crate::fabric::FabricLook)>,
    background: Option<Rgb>,
) -> Picture {
    match (fabric, background) {
        (Some(f), _) => crate::sheen::light_on(s, lighting, Some(f), 1),
        (None, Some(c)) => {
            let mut over = crate::sheen::overlay(s, lighting);
            for px in over.rgba.as_chunks_mut::<4>().0 {
                let a = f64::from(px[3]) / 255.0;
                for (v, bg) in px.iter_mut().zip([c.0, c.1, c.2]) {
                    *v = (f64::from(*v) + f64::from(bg) * (1.0 - a)).round().min(255.0) as u8;
                }
                px[3] = 255;
            }
            over
        }
        (None, None) => {
            // The colors of the overlay have the alpha multiplied in.
            let mut over = crate::sheen::overlay(s, lighting);
            for px in over.rgba.as_chunks_mut::<4>().0 {
                let a = f64::from(px[3]) / 255.0;
                if a > 0.0 {
                    for v in &mut px[..3] {
                        *v = (f64::from(*v) / a).round().min(255.0) as u8;
                    }
                }
            }
            over
        }
    }
}

/// The design picture (the PNG of the program and of `emb-cli output`, the
/// picture to share): with the 3D look the surface of the stitches of
/// `design` (its stitch plan: `stitches` and `spans`) on `ground`, at
/// `px_per_mm` with a margin of `margin_mm`; with the flat look the flat
/// stitches of `raster`. The background is `background`, with the fabric of
/// `look`, or clear.
#[allow(clippy::too_many_arguments)]
pub fn design_picture(
    design: &Design,
    stitches: &StitchDesign,
    spans: impl IntoIterator<Item = (ObjectId, Range<usize>)>,
    ground: Option<Ground>,
    px_per_mm: f64,
    margin_mm: f64,
    background: Option<Rgb>,
    look: &Look,
) -> Picture {
    let segments = crate::segments(stitches);
    let areas = crate::sheet::picture_areas(design);
    if look.lighting.is_none() {
        return crate::raster::picture_look(&segments, &areas, px_per_mm, 0.4, margin_mm, background, look);
    }
    let e = Embroidery::of_design(design, stitches, spans, ground);
    let b = area_of(&segments, &areas, margin_mm + 0.4)
        .or_else(|| e.bounds())
        .unwrap_or(BBox::new(Point::ZERO, Point::new(1.0, 1.0)));
    e.picture(b, px_per_mm, look, background, FULL_DETAIL, &|_| true)
}

/// The area of the stitch segments `segments` and of `areas` with a margin
/// of `margin_mm`, as `raster::picture_look` (the size of a design picture).
pub fn area_of(segments: &[Segment], areas: &crate::raster::Areas, margin_mm: f64) -> Option<BBox> {
    BBox::from_points(
        segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Stitch)
            .flat_map(|s| [s.a, s.b])
            .chain(areas.iter().flat_map(|(rings, _)| rings.iter().flatten().copied())),
    )
    .map(|b| b.expand(margin_mm))
}

#[cfg(test)]
mod tests {
    use emb_model::ThreadKind;

    use super::*;
    use crate::fabric::{FabricLook, Weave};

    fn satin(n: usize) -> Vec<Segment> {
        (0..n)
            .map(|k| {
                let y = 0.2 * k as f64;
                let (a, b) = if k % 2 == 0 { (0.0, 5.0) } else { (5.0, 0.0) };
                Segment {
                    a: Point::new(a, y),
                    b: Point::new(b, y + 0.2),
                    color: Rgb(30, 60, 160),
                    kind: SegmentKind::Stitch,
                    thread_kind: ThreadKind::Polyester,
                    thread_weight: 40,
                }
            })
            .collect()
    }

    #[test]
    fn a_picture_in_blocks_is_the_surface_of_the_area() {
        // A picture larger than a block (so in blocks) and the surface of the
        // same area in one piece have the same pixels.
        let e = Embroidery::of_segments(&satin(150), &[]);
        let b = BBox::new(Point::new(-1.13, -0.71), Point::new(6.0, 31.0));
        let ppm = 12.0;
        let look = Look::three_d(Some(FabricLook::new(Weave::Twill)));
        let bg = Some(Rgb(240, 238, 230));
        let p = e.picture(b, ppm, &look, bg, FULL_DETAIL, &|_| true);
        assert_eq!((p.width, p.height), (86, 381));
        let s = e.laid().surface_on(b, ppm, &Lighting::default(), None);
        let one = crate::sheen::light(&s, &Lighting::default(), bg.zip(look.fabric));
        let c = crate::compare::compare(&p, &one, None, 2, 2);
        assert!(c.largest().unwrap().relative().abs() < 0.02, "{}", c.report());
    }

    #[test]
    fn a_fast_picture_is_near_the_full_one_and_a_sew_out_goes_on() {
        let e = Embroidery::of_segments(&satin(60), &[]);
        let b = BBox::new(Point::new(-1.0, -1.0), Point::new(6.0, 13.0));
        let look = Look::three_d(None);
        let full = e.picture(b, 4.0, &look, Some(Rgb(250, 250, 250)), FULL_DETAIL, &|_| true);
        let fast = e.picture(b, 4.0, &look, Some(Rgb(250, 250, 250)), FAST_DETAIL, &|_| true);
        let c = crate::compare::compare(&full, &fast, None, 2, 2);
        assert!(c.largest().unwrap().relative().abs() < 0.08, "{}", c.report());
        // Half of the stitches: the upper half is fabric, the lower half not.
        let half = e.picture(b, 4.0, &look, Some(Rgb(250, 250, 250)), FULL_DETAIL, &|r| r < 30);
        let px = |p: &Picture, x: u32, y: u32| p.rgba[((y * p.width + x) * 4) as usize];
        assert!(px(&half, 10, 8) < 150, "the first stitches show");
        assert!(px(&half, 10, 48) > 200, "the stitches after the step do not");
        assert!(px(&full, 10, 48) < 150);
    }

    /// The budget of the plan: a design picture of 4000 × 4000 pixels of a
    /// design of 100,000 stitches in 2 s on the development computer. Run
    /// with `cargo test --release -p emb-render -- --ignored picture_budget
    /// --nocapture`.
    #[test]
    #[ignore]
    fn picture_budget() {
        let stitches = crate::stress::make(100_000);
        let t0 = std::time::Instant::now();
        let e = Embroidery::of_stitches(&stitches);
        let lay = t0.elapsed().as_secs_f64();
        let b = e.bounds().unwrap();
        let ppm = 4000.0 / b.width().max(b.height());
        let look = Look::three_d(Some(FabricLook::new(Weave::Twill)));
        let t0 = std::time::Instant::now();
        let p = e.picture(b, ppm, &look, Some(Rgb(235, 235, 230)), FULL_DETAIL, &|_| true);
        let full = t0.elapsed().as_secs_f64();
        let t0 = std::time::Instant::now();
        let small = e.picture(b, 256.0 / b.width().max(b.height()), &look, None, FAST_DETAIL, &|_| {
            true
        });
        let thumb = t0.elapsed().as_secs_f64();
        println!(
            "picture of 100,000 stitches: lay {lay:.2} s; {} x {} px at {ppm:.1} px/mm {full:.2} s; thumbnail {} x {} {:.0} ms",
            p.width,
            p.height,
            small.width,
            small.height,
            thumb * 1000.0
        );
        assert!(lay + full < 2.0, "{lay} {full}");
    }

    /// The audit of the part (Milestone 9, item 6): wrong values make a
    /// picture and no error, in a short time and a bounded memory.
    #[test]
    fn wrong_values_make_a_picture_and_no_error() {
        let seg = |a: (f64, f64), b: (f64, f64), weight: u16| Segment {
            a: Point::new(a.0, a.1),
            b: Point::new(b.0, b.1),
            color: Rgb(200, 30, 30),
            kind: SegmentKind::Stitch,
            thread_kind: ThreadKind::Metallic,
            thread_weight: weight,
        };
        let t0 = std::time::Instant::now();
        // A thread weight of 0 and the largest weight, a stitch of 0 mm and
        // a design 1 km wide (a stitch from one end to the other).
        let segs = vec![
            seg((0.0, 0.0), (5.0, 0.0), 0),
            seg((0.0, 1.0), (5.0, 1.0), u16::MAX),
            seg((2.0, 2.0), (2.0, 2.0), 40),
            seg((0.0, 3.0), (1_000_000.0, 3.0), 40),
            seg((1_000_000.0, 0.0), (1_000_000.0, 1_000_000.0), 40),
        ];
        let e = Embroidery::of_segments(&segs, &[]);
        assert!(e.laid().bytes() < 200 << 20, "{} bytes", e.laid().bytes());
        let b = e.bounds().unwrap();
        assert!(b.width() > 999_000.0);
        for ppm in [0.001, 0.01] {
            let p = e.picture(
                b,
                ppm,
                &Look::three_d(None),
                Some(Rgb(250, 250, 250)),
                FAST_DETAIL,
                &|_| true,
            );
            assert!(p.width <= 16_384 && p.height <= 16_384);
        }
        // A small part of it at a large zoom.
        let near = BBox::new(Point::new(-1.0, -1.0), Point::new(6.0, 4.0));
        let p = e.picture(near, 32.0, &Look::three_d(None), None, FULL_DETAIL, &|_| true);
        assert!(p.rgba.chunks(4).any(|c| c[3] > 0), "the threads show");
        assert!(t0.elapsed().as_secs_f64() < 20.0, "{:?}", t0.elapsed());
    }

    #[test]
    fn with_no_background_the_picture_is_clear_round_the_stitches() {
        let e = Embroidery::of_segments(&satin(20), &[]);
        let b = BBox::new(Point::new(-2.0, -2.0), Point::new(7.0, 6.0));
        let p = e.picture(b, 8.0, &Look::three_d(None), None, FULL_DETAIL, &|_| true);
        assert_eq!(p.rgba[3], 0, "a corner is clear");
        let mid = ((16 * p.width + 36) * 4 + 3) as usize;
        assert_eq!(p.rgba[mid], 255, "a stitch is not");
    }
}
