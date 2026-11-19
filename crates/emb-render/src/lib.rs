//! 2D thread preview data and printed output.
//!
//! This crate converts a [`StitchDesign`] to colored line segments in mm.
//! It has no UI dependency. The app converts the segments to its own draw
//! calls, so a change of UI framework does not change this crate.
//!
//! - [`page`], [`pdf`]: pages as drawing steps, and a PDF writer.
//! - [`sheet`]: the production worksheet, the approval sheet and CSV.
//! - [`raster`]: the design picture (PNG); [`svg`]: the same picture as SVG.
//! - [`product`]: the product under the design and the virtual decoration picture.
//! - [`template`]: the hoop template at full size; [`hooping_sheet`]: the
//!   order and places of the hoopings of a large design.
//! - [`thread3d`]: the 3D thread look (light, shade, shadows, fabric textures).
//! - [`cross_chart`]: the chart of a cross-stitch design; [`stone_sheet`]:
//!   the rhinestone template on paper.
//! - [`density`]: the layers of thread on each small square (density map).
//! - [`light`]: colors mix in linear light (ADR 0123).
//! - [`compare`]: two pictures side by side with their difference, for the
//!   developer; [`stress`]: large synthetic stitch lists to measure speed
//!   (ADR 0122).
#![forbid(unsafe_code)]

pub mod barcode;
pub mod compare;
pub mod cross_chart;
pub mod density;
pub mod hooping_sheet;
pub mod light;
pub mod page;
pub mod pdf;
pub mod photo;
pub mod product;
pub mod raster;
pub mod sheet;
pub mod stone_sheet;
pub mod stress;
pub mod svg;
pub mod template;
pub mod thread3d;
pub mod turntable;

use emb_geom::{BBox, Point};
use emb_model::{Rgb, StitchDesign, StitchKind, ThreadKind};

/// Color for threads that are not in the palette.
pub const FALLBACK_COLOR: Rgb = Rgb(128, 128, 128);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentKind {
    /// Thread on the fabric between two needle positions.
    Stitch,
    /// Frame movement with no stitch. Usually drawn thin or hidden.
    Jump,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub a: Point,
    pub b: Point,
    pub color: Rgb,
    pub kind: SegmentKind,
    /// The kind of the thread (ADR 0123): the 3D look gives it its shine.
    pub thread_kind: ThreadKind,
    /// The weight of the thread: the views give it its width
    /// (`emb_model::thread::width_factor`).
    pub thread_weight: u16,
}

impl Default for Segment {
    /// A stitch of a black 40 weight polyester thread at the origin.
    fn default() -> Self {
        Segment {
            a: Point::ZERO,
            b: Point::ZERO,
            color: Rgb(0, 0, 0),
            kind: SegmentKind::Stitch,
            thread_kind: ThreadKind::default(),
            thread_weight: emb_model::thread::DEFAULT_WEIGHT,
        }
    }
}

impl Segment {
    /// The width of the thread, as a part of the width of a 40 weight thread.
    pub fn width_factor(&self) -> f64 {
        emb_model::thread::width_factor(self.thread_weight)
    }
}

/// Converts a stitch design to line segments in sew order.
///
/// A segment goes from each needle position to the next one. A trim or a
/// color change breaks the thread, so the next movement is a jump.
pub fn segments(design: &StitchDesign) -> Vec<Segment> {
    let mut out = Vec::with_capacity(design.records.len());
    let mut thread_index = 0usize;
    let mut last: Option<Point> = None;
    let mut thread_cut = false;
    let color_of = |i: usize| design.threads.get(i).map(|t| t.color).unwrap_or(FALLBACK_COLOR);
    let look_of = |i: usize| {
        design
            .threads
            .get(i)
            .map_or((ThreadKind::default(), emb_model::thread::DEFAULT_WEIGHT), |t| {
                (t.kind, t.weight)
            })
    };

    for r in &design.records {
        match r.kind {
            StitchKind::Stitch | StitchKind::Jump => {
                if let Some(a) = last
                    && a != r.pos
                {
                    let kind = if r.kind == StitchKind::Jump || thread_cut {
                        SegmentKind::Jump
                    } else {
                        SegmentKind::Stitch
                    };
                    let (thread_kind, thread_weight) = look_of(thread_index);
                    out.push(Segment {
                        a,
                        b: r.pos,
                        color: color_of(thread_index),
                        kind,
                        thread_kind,
                        thread_weight,
                    });
                }
                if r.kind == StitchKind::Stitch {
                    thread_cut = false;
                }
                last = Some(r.pos);
            }
            StitchKind::Trim => thread_cut = true,
            // A speed change does not move the needle.
            StitchKind::Slow | StitchKind::Fast => {}
            StitchKind::ColorChange => {
                thread_index += 1;
                thread_cut = true;
            }
            StitchKind::Stop | StitchKind::End => {}
        }
    }
    out
}

/// Bounding box of all segments.
pub fn bounds(segments: &[Segment]) -> Option<BBox> {
    BBox::from_points(segments.iter().flat_map(|s| [s.a, s.b]))
}

#[cfg(test)]
mod tests {
    use emb_model::{StitchRecord, Thread, ThreadId};

    use super::*;

    fn thread(i: u32, c: Rgb) -> Thread {
        Thread {
            id: ThreadId(i),
            name: format!("t{i}"),
            color: c,
            brand: None,
            code: None,
            ..Default::default()
        }
    }

    #[test]
    fn trims_and_color_changes_make_jumps() {
        let red = Rgb(255, 0, 0);
        let blue = Rgb(0, 0, 255);
        let p = |x| Point::new(x, 0.0);
        let d = StitchDesign {
            threads: vec![thread(0, red), thread(1, blue)],
            records: vec![
                StitchRecord::new(p(0.0), StitchKind::Stitch),
                StitchRecord::new(p(1.0), StitchKind::Stitch),
                StitchRecord::new(p(1.0), StitchKind::Trim),
                StitchRecord::new(p(5.0), StitchKind::Stitch),
                StitchRecord::new(p(5.0), StitchKind::ColorChange),
                StitchRecord::new(p(6.0), StitchKind::Stitch),
                StitchRecord::new(p(7.0), StitchKind::Stitch),
            ],
        };
        let s = segments(&d);
        let kinds: Vec<_> = s.iter().map(|s| (s.kind, s.color)).collect();
        assert_eq!(
            kinds,
            vec![
                (SegmentKind::Stitch, red),
                (SegmentKind::Jump, red),
                (SegmentKind::Jump, blue),
                (SegmentKind::Stitch, blue),
            ]
        );
        assert_eq!(bounds(&s).unwrap().width(), 7.0);
    }

    #[test]
    fn missing_thread_uses_fallback() {
        let d = StitchDesign {
            threads: vec![],
            records: vec![StitchRecord::stitch(0.0, 0.0), StitchRecord::stitch(1.0, 0.0)],
        };
        assert_eq!(segments(&d)[0].color, FALLBACK_COLOR);
    }
}
