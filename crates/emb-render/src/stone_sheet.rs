//! The rhinestone template on paper (P9 Milestone 4).
//!
//! - Page 1: the key (each kind of stone: its color, size, hole and the
//!   number of stones) and how to use the template.
//! - Then the template at full size: a circle for each hole in the stone
//!   color, the registration marks, and a 50 mm line to check the print
//!   scale. A template that is larger than the paper goes on more sheets,
//!   as the hoop template; the user cuts them on their frame lines and
//!   joins them.

use emb_geom::{BBox, Point};
use emb_model::Rgb;
use emb_model::bling::{MARK_ARM_MM, StoneTemplate};

use crate::page::{Op, Page, Paper, text_width_mm};

const MARGIN_MM: f64 = 10.0;
const HEADER_MM: f64 = 14.0;
/// The length of the scale test line, in mm.
pub const SCALE_MM: f64 = 50.0;
const INK: Rgb = Rgb(0, 0, 0);
const GRAY: Rgb = Rgb(150, 150, 150);

/// Settings of [`pages`].
#[derive(Debug, Clone, PartialEq)]
pub struct StoneSheetSpec {
    pub name: String,
    pub paper: Paper,
    /// How much larger the holes are than the stones, mm.
    pub clearance_mm: f64,
}

/// A light tint of a color, for the inside of a hole on paper.
fn tint(c: Rgb) -> Rgb {
    let t = |v: u8| (f64::from(v) + (255.0 - f64::from(v)) * 0.5).round() as u8;
    Rgb(t(c.0), t(c.1), t(c.2))
}

fn circle(center: Point, r: f64) -> Vec<Point> {
    (0..24)
        .map(|i| {
            let a = emb_geom::math::to_radians(15.0 * f64::from(i));
            Point::new(
                center.x + r * emb_geom::math::cos(a),
                center.y + r * emb_geom::math::sin(a),
            )
        })
        .collect()
}

/// Draws a hole: the tint of the stone color and a thin black ring.
fn hole(page: &mut Page, center: Point, d: f64, color: Rgb) {
    let ring = circle(center, d / 2.0);
    page.ops.push(Op::Area {
        rings: vec![ring.clone()],
        fill: tint(color),
    });
    for i in 0..ring.len() {
        page.line(ring[i], ring[(i + 1) % ring.len()], 0.12, INK);
    }
}

/// The key page.
fn key(t: &StoneTemplate, spec: &StoneSheetSpec) -> Page {
    let mut p = Page::new(spec.paper);
    let x = MARGIN_MM + 2.0;
    let mut y = MARGIN_MM + 8.0;
    p.text(x, y, 16.0, true, format!("{} — rhinestone template", spec.name));
    y += 7.0;
    let counts = t.counts();
    let total: usize = counts.iter().map(|(_, n)| n).sum();
    let size = t.bbox().map_or(String::new(), |b| {
        format!(", {:.0} × {:.0} mm with the marks", b.width(), b.height())
    });
    p.text(x, y, 10.0, false, format!("{total} stones{size}"));
    y += 10.0;
    let cols = [x, x + 10.0, x + 70.0, x + 100.0, x + 130.0];
    for (i, h) in ["", "Stone", "Stone size", "Hole", "Stones"].iter().enumerate() {
        p.gray_text(cols[i], y, 8.0, *h);
    }
    for (stone, n) in &counts {
        y += 7.0;
        hole(
            &mut p,
            Point::new(cols[0] + 3.0, y - 1.3),
            stone.diameter_mm(),
            stone.color,
        );
        p.text(cols[1], y, 9.5, false, stone.label());
        p.text(cols[2], y, 9.5, false, format!("{:.1} mm", stone.diameter_mm()));
        p.text(
            cols[3],
            y,
            9.5,
            false,
            format!("{:.2} mm", stone.diameter_mm() + spec.clearance_mm),
        );
        p.text(cols[4], y, 9.5, false, n.to_string());
    }
    y += 12.0;
    p.text(x, y, 10.0, true, "How to use the template");
    let steps = [
        "1. Cut the template file (SVG or DXF) from template material on the cutter. Each size of stone is a group of its own.",
        "2. Or print the next pages at 100 % (no fit to page) to check the places of the stones.",
        "3. Put the template on a backing board. Brush the stones into the holes, the largest size first.",
        "4. Pick up the stones with transfer tape. Put the tape on the garment, with the marks where they must be.",
        "5. Press with a heat press (the time and temperature that the maker of the stones gives).",
    ];
    for s in steps {
        for line in crate::page::wrap(s, 95) {
            y += 5.0;
            p.text(x, y, 9.0, false, line);
        }
    }
    p
}

/// The template pages: the key and the template at full size.
pub fn pages(t: &StoneTemplate, spec: &StoneSheetSpec) -> Vec<Page> {
    let Some(content) = t.bbox().map(|b| b.expand(3.0)) else {
        return Vec::new();
    };
    let mut pages = vec![key(t, spec)];
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
    let origin = Point::new(
        content.min.x - (cols as f64 * tw - content.width()) / 2.0,
        content.min.y - (rows as f64 * th - content.height()) / 2.0,
    );
    let many = cols * rows > 1;
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
            // A hole goes on each sheet that has its center.
            for hl in t.holes.iter().filter(|hl| tile.contains(hl.at)) {
                hole(&mut page, hl.at + on_page, hl.diameter_mm, hl.stone.color);
            }
            for m in t.marks.iter().filter(|m| tile.contains(**m)) {
                let q = *m + on_page;
                let a = MARK_ARM_MM;
                page.line(Point::new(q.x - a, q.y), Point::new(q.x + a, q.y), 0.3, INK);
                page.line(Point::new(q.x, q.y - a), Point::new(q.x, q.y + a), 0.3, INK);
            }
            if many {
                page.ops.push(Op::Rect {
                    rect: BBox::new(tile.min + on_page, tile.max + on_page),
                    fill: None,
                    stroke: Some((0.2, GRAY)),
                });
            }
            let (x, y) = (MARGIN_MM, MARGIN_MM + 4.0);
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
            page.text(x, y, 10.0, true, format!("Rhinestone template: {}", spec.name));
            page.gray_text(
                x,
                y + 5.0,
                8.0,
                format!(
                    "Full size; holes {:.2} mm larger than the stones{sheet}",
                    spec.clearance_mm
                ),
            );
            if r == 0 && c == 0 {
                let note = format!("This line is {SCALE_MM:.0} mm. Print at 100 % (no fit to page).");
                let x1 = w - MARGIN_MM;
                let x0 = x1 - SCALE_MM;
                page.line(Point::new(x0, y), Point::new(x1, y), 0.5, INK);
                page.line(Point::new(x0, y - 1.5), Point::new(x0, y + 1.5), 0.5, INK);
                page.line(Point::new(x1, y - 1.5), Point::new(x1, y + 1.5), 0.5, INK);
                page.gray_text(x1 - text_width_mm(&note, 7.0), y + 5.0, 7.0, note);
            }
            pages.push(page);
        }
    }
    pages
}

#[cfg(test)]
mod tests {
    use emb_model::bling::{Bling, BlingLayout, Stone};
    use emb_model::{Design, ObjectKind};

    use super::*;

    fn spec() -> StoneSheetSpec {
        StoneSheetSpec {
            name: "Stars".into(),
            paper: Paper::Letter,
            clearance_mm: 0.3,
        }
    }

    fn design(width: f64) -> Design {
        let mut d = Design::new("t");
        let th = d.add_thread("Stones", Rgb(0, 0, 0));
        let path = emb_geom::Path::new()
            .move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(width, 0.0));
        d.add_object(
            th,
            ObjectKind::Bling(Box::new(Bling::new(Stone::default(), BlingLayout::Run { path }))),
        );
        d
    }

    fn circles(p: &Page) -> Vec<f64> {
        p.ops
            .iter()
            .filter_map(|o| match o {
                Op::Area { rings, .. } => {
                    let b = BBox::from_points(rings[0].iter().copied()).unwrap();
                    Some(b.width())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_small_template_is_one_sheet_at_full_size() {
        let t = design(60.0).stone_template(0.3).unwrap();
        let pages = pages(&t, &spec());
        assert_eq!(pages.len(), 2, "the key and one sheet");
        // 60 mm with a step of 3.3 mm: 19 stones, each a 3.1 mm hole.
        let c = circles(&pages[1]);
        assert_eq!(c.len(), 19);
        assert!(c.iter().all(|w| (w - 3.1).abs() < 0.01), "{c:?}");
        let pdf = crate::pdf::write(&pages, "Stars");
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn a_long_template_goes_on_more_sheets_with_each_hole_once() {
        let t = design(500.0).stone_template(0.3).unwrap();
        let pages = pages(&t, &spec());
        assert!(pages.len() >= 3, "{}", pages.len());
        let n: usize = pages[1..].iter().map(|p| circles(p).len()).sum();
        assert_eq!(n, t.holes.len());
    }
}
