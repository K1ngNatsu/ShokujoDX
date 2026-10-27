//! The hoop template: the hoop field at full size on paper, to put the
//! design in the correct place on the fabric (P7 Milestone 1). A field
//! that is larger than the paper goes on more sheets; the user cuts them
//! on their frame lines and joins them so the lines meet.

use emb_geom::{BBox, Point};
use emb_model::Rgb;

use crate::page::{Op, Page, Paper, text_width_mm};
use crate::{Segment, SegmentKind};

/// The empty space round the printed area of a sheet, in mm.
const MARGIN_MM: f64 = 10.0;
/// The space for the text at the top of a sheet, in mm.
const HEADER_MM: f64 = 14.0;
/// The length of the scale test line, in mm.
pub const SCALE_MM: f64 = 50.0;

const INK: Rgb = Rgb(0, 0, 0);
const GRAY: Rgb = Rgb(150, 150, 150);

/// What the template shows, in hoop coordinates (mm, origin at the hoop
/// center, Y down, the hoop straight).
#[derive(Debug, Clone, Default)]
pub struct TemplateSpec {
    pub name: String,
    pub hoop_name: String,
    /// The outline of the sewable field.
    pub field: Vec<Vec<Point>>,
    /// The outline of the safety margin.
    pub safe: Vec<Vec<Point>>,
    /// The stitches of the design (they print gray).
    pub segments: Vec<Segment>,
    pub paper: Paper,
}

/// The sheets of the template, at full size (100 %).
pub fn hoop_template(spec: &TemplateSpec) -> Vec<Page> {
    let pts = spec.field.iter().flatten().copied();
    let segs = spec
        .segments
        .iter()
        .filter(|s| s.kind == SegmentKind::Stitch)
        .flat_map(|s| [s.a, s.b]);
    let Some(content) = BBox::from_points(pts.chain(segs).chain([Point::ZERO])) else {
        return Vec::new();
    };
    let content = content.expand(5.0);

    // Portrait or landscape: the one with fewer sheets (portrait on a tie).
    let (pw, ph) = spec.paper.size_mm();
    let tiles = |w: f64, h: f64| {
        let (tw, th) = (w - 2.0 * MARGIN_MM, h - 2.0 * MARGIN_MM - HEADER_MM);
        let cols = (content.width() / tw).ceil().max(1.0) as usize;
        let rows = (content.height() / th).ceil().max(1.0) as usize;
        (cols, rows, tw, th, w, h)
    };
    let a = tiles(pw, ph);
    let b = tiles(ph, pw);
    let (cols, rows, tw, th, w, h) = if b.0 * b.1 < a.0 * a.1 { b } else { a };

    // The content is in the middle of all the sheets together.
    let origin = Point::new(
        content.min.x - (cols as f64 * tw - content.width()) / 2.0,
        content.min.y - (rows as f64 * th - content.height()) / 2.0,
    );
    let many = cols * rows > 1;
    let mut pages = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let tile_min = origin + Point::new(c as f64 * tw, r as f64 * th);
            let tile = BBox::new(tile_min, tile_min + Point::new(tw, th));
            let on_page = Point::new(MARGIN_MM, MARGIN_MM + HEADER_MM) - tile_min;
            let mut page = Page {
                width_mm: w,
                height_mm: h,
                ops: Vec::new(),
            };
            let line = |page: &mut Page, a: Point, b: Point, width: f64, color: Rgb| {
                if let Some((a, b)) = clip(a, b, &tile) {
                    page.line(a + on_page, b + on_page, width, color);
                }
            };

            // The stitches, gray.
            let segments: Vec<Segment> = spec
                .segments
                .iter()
                .filter(|s| s.kind == SegmentKind::Stitch)
                .filter_map(|s| {
                    clip(s.a, s.b, &tile).map(|(a, b)| Segment {
                        a: a + on_page,
                        b: b + on_page,
                        color: Rgb(190, 190, 190),
                        kind: SegmentKind::Stitch,
                    })
                })
                .collect();
            if !segments.is_empty() {
                page.ops.push(Op::Stitches {
                    segments,
                    width_mm: 0.3,
                });
            }
            // The field (solid) and the safety margin (dashed).
            for ring in &spec.field {
                for (a, b) in edges(ring) {
                    line(&mut page, a, b, 0.5, INK);
                }
            }
            for ring in &spec.safe {
                for (a, b) in edges(ring) {
                    for (p, q) in dashes(a, b, 3.0, 2.0) {
                        line(&mut page, p, q, 0.3, INK);
                    }
                }
            }
            // The center lines across the field, with a mark each 10 mm.
            if let Some(f) = BBox::from_points(spec.field.iter().flatten().copied()) {
                line(&mut page, Point::new(f.min.x, 0.0), Point::new(f.max.x, 0.0), 0.2, GRAY);
                line(&mut page, Point::new(0.0, f.min.y), Point::new(0.0, f.max.y), 0.2, GRAY);
                let ticks = |lo: f64, hi: f64| {
                    let first = (lo / 10.0).ceil() as i64;
                    let last = (hi / 10.0).floor() as i64;
                    (first..=last).filter(|k| *k != 0).map(|k| k as f64 * 10.0)
                };
                for x in ticks(f.min.x, f.max.x) {
                    line(&mut page, Point::new(x, -1.5), Point::new(x, 1.5), 0.2, GRAY);
                }
                for y in ticks(f.min.y, f.max.y) {
                    line(&mut page, Point::new(-1.5, y), Point::new(1.5, y), 0.2, GRAY);
                }
            }
            // The center cross.
            line(&mut page, Point::new(-8.0, 0.0), Point::new(8.0, 0.0), 0.5, INK);
            line(&mut page, Point::new(0.0, -8.0), Point::new(0.0, 8.0), 0.5, INK);

            // The frame of the sheet: cut on it to join the sheets.
            if many {
                let a = tile.min + on_page;
                let b = tile.max + on_page;
                page.ops.push(Op::Rect {
                    rect: BBox::new(a, b),
                    fill: None,
                    stroke: Some((0.2, GRAY)),
                });
            }

            // The text at the top.
            let x = MARGIN_MM;
            let y = MARGIN_MM + 4.0;
            let sheet = if many {
                format!(
                    "  ·  Sheet {} of {} (row {}, column {})",
                    r * cols + c + 1,
                    cols * rows,
                    r + 1,
                    c + 1
                )
            } else {
                String::new()
            };
            page.text(x, y, 10.0, true, format!("Hoop template: {}", spec.name));
            page.gray_text(x, y + 5.0, 8.0, format!("{}{sheet}", spec.hoop_name));
            if r == 0 && c == 0 {
                // The scale test: the user measures this line.
                let note = format!("This line is {SCALE_MM:.0} mm. Print at 100 % (no fit to page).");
                let x1 = w - MARGIN_MM;
                let x0 = x1 - SCALE_MM;
                page.line(Point::new(x0, y), Point::new(x1, y), 0.5, INK);
                page.line(Point::new(x0, y - 1.5), Point::new(x0, y + 1.5), 0.5, INK);
                page.line(Point::new(x1, y - 1.5), Point::new(x1, y + 1.5), 0.5, INK);
                page.gray_text(x1 - text_width_mm(&note, 7.0), y + 5.0, 7.0, note);
            }
            if many {
                page.gray_text(
                    x,
                    h - MARGIN_MM + 5.0,
                    7.0,
                    "Cut each sheet on its gray frame and join the sheets so the lines meet.",
                );
            }
            pages.push(page);
        }
    }
    pages
}

/// The edges of a closed ring.
fn edges(ring: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    let n = ring.len();
    (0..n).filter(move |_| n > 1).map(move |i| (ring[i], ring[(i + 1) % n]))
}

/// The dashes of a line: `on` mm drawn, `off` mm not drawn.
fn dashes(a: Point, b: Point, on: f64, off: f64) -> Vec<(Point, Point)> {
    let len = a.distance(b);
    let mut out = Vec::new();
    let mut t = 0.0;
    while t < len {
        let e = (t + on).min(len);
        out.push((a.lerp(b, t / len), a.lerp(b, e / len)));
        t += on + off;
    }
    out
}

/// The part of the line from `a` to `b` inside `r` (Liang and Barsky).
fn clip(a: Point, b: Point, r: &BBox) -> Option<(Point, Point)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-d.x, a.x - r.min.x),
        (d.x, r.max.x - a.x),
        (-d.y, a.y - r.min.y),
        (d.y, r.max.y - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t0 <= t1).then(|| (a + d * t0, a + d * t1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f64, h: f64) -> Vec<Point> {
        let (x, y) = (w / 2.0, h / 2.0);
        vec![
            Point::new(-x, -y),
            Point::new(x, -y),
            Point::new(x, y),
            Point::new(-x, y),
        ]
    }

    fn lines(p: &Page) -> Vec<(Point, Point)> {
        p.ops
            .iter()
            .filter_map(|o| match o {
                Op::Line { a, b, .. } => Some((*a, *b)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_small_hoop_is_one_sheet_at_full_size() {
        let spec = TemplateSpec {
            field: vec![rect(100.0, 100.0)],
            paper: Paper::A4,
            ..Default::default()
        };
        let pages = hoop_template(&spec);
        assert_eq!(pages.len(), 1);
        // The top edge of the field is 100 mm long on paper.
        let top = lines(&pages[0])
            .into_iter()
            .find(|(a, b)| (a.y - b.y).abs() < 1e-9 && (a.distance(*b) - 100.0).abs() < 1e-6);
        assert!(top.is_some());
        // The scale line is there.
        assert!(
            lines(&pages[0])
                .iter()
                .any(|(a, b)| (a.distance(*b) - SCALE_MM).abs() < 1e-9)
        );
    }

    #[test]
    fn the_topaz_hoop_needs_two_sheets_that_join() {
        let spec = TemplateSpec {
            field: vec![rect(260.0, 200.0)],
            paper: Paper::A4,
            ..Default::default()
        };
        let pages = hoop_template(&spec);
        assert_eq!(pages.len(), 2);
        // The top edge is in pieces; the pieces together are 260 mm long.
        let total: f64 = pages
            .iter()
            .flat_map(lines)
            .filter(|(a, b)| (a.y - b.y).abs() < 1e-9 && a.distance(*b) > 20.0)
            .map(|(a, b)| a.distance(b))
            .filter(|l| (*l - SCALE_MM).abs() > 1e-9)
            .sum();
        // Top and bottom edges, plus the center line across the field.
        assert!((total - 3.0 * 260.0).abs() < 1e-6, "{total}");
    }

    #[test]
    fn clip_keeps_the_inside_part() {
        let r = BBox::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let (a, b) = clip(Point::new(-5.0, 5.0), Point::new(15.0, 5.0), &r).unwrap();
        assert_eq!((a, b), (Point::new(0.0, 5.0), Point::new(10.0, 5.0)));
        assert!(clip(Point::new(-5.0, -1.0), Point::new(15.0, -1.0), &r).is_none());
    }
}
