//! The cross-stitch chart (P9 Milestone 2): pages for hand cross stitch
//! (or to check a machine design cell by cell).
//!
//! - Page 1: the title, the size in cells, on the machine and on Aida
//!   fabric, the key (a symbol for each color, its thread, the number of
//!   stitches) and a small picture of the whole chart.
//! - Then the chart: a symbol on a light tint of the thread color in each
//!   cell, a darker line each 10 cells with its number, arrows at the
//!   center, and the backstitch lines. A large chart goes on more pages;
//!   each page says which columns and rows it has.

use emb_geom::{BBox, Point};
use emb_model::cross::{Cell, Corner, CrossStitch, Line, Part};
use emb_model::{Design, ObjectKind, Rgb};

use crate::page::{Op, Page, Paper, text_width_mm};

const MARGIN_MM: f64 = 12.0;
/// The size of a cell on the chart pages.
const CELL_MM: f64 = 3.0;
/// The space for the numbers at the top and left of the chart.
const LABEL_MM: f64 = 5.0;
/// The space for the page title.
const HEADER_MM: f64 = 12.0;
/// Aida fabric counts in the size table (cells to the inch).
pub const AIDA_COUNTS: [u32; 4] = [11, 14, 16, 18];
/// The number of different symbols; more colors use them again.
pub const SYMBOLS: usize = 32;

/// The cells and lines of one thread.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartColor {
    pub name: String,
    pub color: Rgb,
    pub cells: Vec<Cell>,
    pub lines: Vec<Line>,
}

/// Settings of [`chart`].
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSpec {
    pub title: String,
    pub paper: Paper,
    /// The cell size of the design (on the machine), mm.
    pub cell_mm: f64,
}

/// The columns and rows of the chart: from the first cell or line to the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    pub col0: i32,
    pub row0: i32,
    pub cols: i32,
    pub rows: i32,
}

impl Extent {
    pub fn of(colors: &[ChartColor]) -> Option<Extent> {
        let mut b: Option<(i32, i32, i32, i32)> = None;
        let mut add = |c0: i32, r0: i32, c1: i32, r1: i32| {
            b = Some(match b {
                None => (c0, r0, c1, r1),
                Some((a, b2, c, d)) => (a.min(c0), b2.min(r0), c.max(c1), d.max(r1)),
            });
        };
        for k in colors {
            for c in &k.cells {
                add(c.col, c.row, c.col + 1, c.row + 1);
            }
            for l in &k.lines {
                for h in l {
                    add(
                        h[0].div_euclid(2),
                        h[1].div_euclid(2),
                        (h[0] + 1).div_euclid(2),
                        (h[1] + 1).div_euclid(2),
                    );
                }
            }
        }
        b.map(|(c0, r0, c1, r1)| Extent {
            col0: c0,
            row0: r0,
            cols: (c1 - c0).max(1),
            rows: (r1 - r0).max(1),
        })
    }
}

/// The number of full, half, quarter and three-quarter crosses.
pub fn counts(cells: &[Cell]) -> [usize; 4] {
    let mut n = [0; 4];
    for c in cells {
        n[match c.part {
            Part::Full => 0,
            Part::HalfUp | Part::HalfDown => 1,
            Part::Quarter(_) => 2,
            Part::ThreeQuarter(_) => 3,
        }] += 1;
    }
    n
}

/// A light tint of a thread color, so a black symbol shows on it.
fn tint(c: Rgb, amount: f64) -> Rgb {
    let t = |v: u8| (f64::from(v) + (255.0 - f64::from(v)) * amount).round() as u8;
    Rgb(t(c.0), t(c.1), t(c.2))
}

/// A darker version of a thread color, for lines on the paper.
fn ink(c: Rgb) -> Rgb {
    let t = |v: u8| (f64::from(v) * 0.75).round() as u8;
    Rgb(t(c.0), t(c.1), t(c.2))
}

fn regular(center: Point, r: f64, n: usize, start_deg: f64) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let a = emb_geom::math::to_radians(start_deg + 360.0 * i as f64 / n as f64);
            let (s, c) = (emb_geom::math::sin(a), emb_geom::math::cos(a));
            Point::new(center.x + r * c, center.y + r * s)
        })
        .collect()
}

/// Draws symbol `k` (of [`SYMBOLS`]) in a box of `size` mm at `center`.
pub fn symbol(page: &mut Page, k: usize, center: Point, size: f64) {
    let black = Rgb(0, 0, 0);
    let w = (size * 0.09).max(0.12);
    let r = size * 0.36;
    let p = |x: f64, y: f64| Point::new(center.x + x * r, center.y + y * r);
    let fill = |page: &mut Page, ring: Vec<Point>| {
        page.ops.push(Op::Area {
            rings: vec![ring],
            fill: black,
        })
    };
    let outline = |page: &mut Page, ring: Vec<Point>| {
        for i in 0..ring.len() {
            page.line(ring[i], ring[(i + 1) % ring.len()], w, black);
        }
    };
    let square = |s: f64| vec![p(-s, -s), p(s, -s), p(s, s), p(-s, s)];
    let circle = |s: f64| regular(center, r * s, 16, 0.0);
    let tri = |dir: usize| -> Vec<Point> {
        match dir {
            0 => vec![p(0.0, -1.0), p(1.0, 0.8), p(-1.0, 0.8)],
            1 => vec![p(0.0, 1.0), p(-1.0, -0.8), p(1.0, -0.8)],
            2 => vec![p(-1.0, 0.0), p(0.8, -1.0), p(0.8, 1.0)],
            _ => vec![p(1.0, 0.0), p(-0.8, 1.0), p(-0.8, -1.0)],
        }
    };
    let diamond = || vec![p(0.0, -1.0), p(1.0, 0.0), p(0.0, 1.0), p(-1.0, 0.0)];
    let dot = |page: &mut Page| {
        page.ops.push(Op::Area {
            rings: vec![regular(center, r * 0.28, 12, 0.0)],
            fill: black,
        })
    };
    let line = |page: &mut Page, a: Point, b: Point| page.line(a, b, w, black);
    match k % SYMBOLS {
        0 => fill(page, square(0.85)),
        1 => outline(page, square(0.85)),
        2 => fill(page, circle(1.0)),
        3 => outline(page, circle(1.0)),
        4 => fill(page, tri(0)),
        5 => outline(page, tri(0)),
        6 => fill(page, tri(1)),
        7 => outline(page, tri(1)),
        8 => fill(page, diamond()),
        9 => outline(page, diamond()),
        10 => {
            line(page, p(-0.9, -0.9), p(0.9, 0.9));
            line(page, p(-0.9, 0.9), p(0.9, -0.9));
        }
        11 => {
            line(page, p(0.0, -1.0), p(0.0, 1.0));
            line(page, p(-1.0, 0.0), p(1.0, 0.0));
        }
        12 => fill(page, tri(2)),
        13 => outline(page, tri(2)),
        14 => fill(page, tri(3)),
        15 => outline(page, tri(3)),
        16 => line(page, p(-0.9, 0.9), p(0.9, -0.9)),
        17 => line(page, p(-0.9, -0.9), p(0.9, 0.9)),
        18 => fill(page, vec![p(-1.0, -0.3), p(1.0, -0.3), p(1.0, 0.3), p(-1.0, 0.3)]),
        19 => fill(page, vec![p(-0.3, -1.0), p(0.3, -1.0), p(0.3, 1.0), p(-0.3, 1.0)]),
        20 => {
            outline(page, square(0.85));
            dot(page);
        }
        21 => {
            outline(page, circle(1.0));
            dot(page);
        }
        22 => {
            outline(page, circle(1.0));
            line(page, p(-0.7, -0.7), p(0.7, 0.7));
            line(page, p(-0.7, 0.7), p(0.7, -0.7));
        }
        23 => {
            outline(page, square(0.85));
            line(page, p(-0.85, -0.85), p(0.85, 0.85));
            line(page, p(-0.85, 0.85), p(0.85, -0.85));
        }
        24 => fill(page, regular(center, r, 6, 0.0)),
        25 => outline(page, regular(center, r, 6, 0.0)),
        26 => fill(page, square(0.45)),
        27 => dot(page),
        28 => {
            outline(page, diamond());
            dot(page);
        }
        29 => {
            outline(page, tri(0));
            page.ops.push(Op::Area {
                rings: vec![regular(Point::new(center.x, center.y + r * 0.25), r * 0.22, 12, 0.0)],
                fill: black,
            });
        }
        30 => {
            outline(page, tri(0));
            outline(page, tri(1));
        }
        _ => {
            fill(page, vec![p(-1.0, -0.55), p(1.0, -0.55), p(1.0, -0.15), p(-1.0, -0.15)]);
            fill(page, vec![p(-1.0, 0.15), p(1.0, 0.15), p(1.0, 0.55), p(-1.0, 0.55)]);
        }
    }
}

/// The two corners next to `c`: with `c` they make the half of the cell
/// that a three-quarter cross toward `c` covers.
fn side_corners(c: Corner) -> (Corner, Corner) {
    match c {
        Corner::TopLeft | Corner::BottomRight => (Corner::TopRight, Corner::BottomLeft),
        Corner::TopRight | Corner::BottomLeft => (Corner::TopLeft, Corner::BottomRight),
    }
}

/// Draws one cell at `at` (its top-left corner on the page) of `size` mm.
fn draw_cell(page: &mut Page, part: Part, k: usize, color: Rgb, at: Point, size: f64) {
    let corner = |c: Corner| {
        let (x, y) = c.offset();
        Point::new(at.x + x * size, at.y + y * size)
    };
    let center = Point::new(at.x + size / 2.0, at.y + size / 2.0);
    let rect = BBox::new(at, Point::new(at.x + size, at.y + size));
    match part {
        Part::Full => {
            page.ops.push(Op::Rect {
                rect,
                fill: Some(tint(color, 0.55)),
                stroke: None,
            });
            symbol(page, k, center, size);
        }
        Part::HalfUp | Part::HalfDown => {
            page.ops.push(Op::Rect {
                rect,
                fill: Some(tint(color, 0.8)),
                stroke: None,
            });
            let (a, b) = if part == Part::HalfUp {
                (Corner::BottomLeft, Corner::TopRight)
            } else {
                (Corner::TopLeft, Corner::BottomRight)
            };
            page.line(corner(a), corner(b), size * 0.12, ink(color));
            symbol(page, k, center, size * 0.5);
        }
        Part::Quarter(c) => {
            let q = corner(c).midpoint(center);
            let h = size / 4.0;
            page.ops.push(Op::Rect {
                rect: BBox::new(Point::new(q.x - h, q.y - h), Point::new(q.x + h, q.y + h)),
                fill: Some(tint(color, 0.55)),
                stroke: None,
            });
            symbol(page, k, q, size * 0.45);
        }
        Part::ThreeQuarter(c) => {
            let (a, b) = side_corners(c);
            let tri = vec![corner(c), corner(a), corner(b)];
            let g = Point::new(
                (tri[0].x + tri[1].x + tri[2].x) / 3.0,
                (tri[0].y + tri[1].y + tri[2].y) / 3.0,
            );
            page.ops.push(Op::Area {
                rings: vec![tri],
                fill: tint(color, 0.55),
            });
            symbol(page, k, g, size * 0.55);
        }
    }
}

/// The part of the segment `a`–`b` inside `r`, if any (Liang–Barsky).
fn clip(a: Point, b: Point, r: BBox) -> Option<(Point, Point)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-d.x, a.x - r.min.x),
        (d.x, r.max.x - a.x),
        (-d.y, a.y - r.min.y),
        (d.y, r.max.y - a.y),
    ] {
        if p.abs() < 1e-12 {
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

/// The chart part of one page: columns `c0..c1`, rows `r0..r1` of the
/// extent (counted from 0), with the top-left cell at `at`.
fn draw_chart(page: &mut Page, colors: &[ChartColor], e: Extent, cols: (i32, i32), rows: (i32, i32), at: Point) {
    let ((c0, c1), (r0, r1)) = (cols, rows);
    let cell_at = |c: i32, r: i32| Point::new(at.x + f64::from(c - c0) * CELL_MM, at.y + f64::from(r - r0) * CELL_MM);
    for (k, color) in colors.iter().enumerate() {
        for cell in &color.cells {
            let (c, r) = (cell.col - e.col0, cell.row - e.row0);
            if (c0..c1).contains(&c) && (r0..r1).contains(&r) {
                draw_cell(page, cell.part, k, color.color, cell_at(c, r), CELL_MM);
            }
        }
    }
    // The grid: thin lines, and a darker line each 10 cells with its number.
    let (x0, y0) = (at.x, at.y);
    let (x1, y1) = (cell_at(c1, r1).x, cell_at(c1, r1).y);
    for c in c0..=c1 {
        let x = cell_at(c, r0).x;
        let major = c % 10 == 0;
        let (w, g) = if major {
            (0.3, Rgb(0, 0, 0))
        } else {
            (0.08, Rgb(150, 150, 150))
        };
        page.line(Point::new(x, y0), Point::new(x, y1), w, g);
        if major && c > 0 {
            let t = c.to_string();
            page.gray_text(x - text_width_mm(&t, 6.0) / 2.0, y0 - 1.2, 6.0, t);
        }
    }
    for r in r0..=r1 {
        let y = cell_at(c0, r).y;
        let major = r % 10 == 0;
        let (w, g) = if major {
            (0.3, Rgb(0, 0, 0))
        } else {
            (0.08, Rgb(150, 150, 150))
        };
        page.line(Point::new(x0, y), Point::new(x1, y), w, g);
        if major && r > 0 {
            let t = r.to_string();
            page.gray_text(x0 - 1.0 - text_width_mm(&t, 6.0), y + 0.8, 6.0, t);
        }
    }
    // Arrows at the center column and row.
    let arrow = Rgb(200, 30, 30);
    let (mc, mr) = (e.cols as f64 / 2.0, e.rows as f64 / 2.0);
    if f64::from(c0) <= mc && mc <= f64::from(c1) {
        let x = at.x + (mc - f64::from(c0)) * CELL_MM;
        page.ops.push(Op::Area {
            rings: vec![vec![
                Point::new(x, y0 - 0.3),
                Point::new(x - 1.2, y0 - 3.5),
                Point::new(x + 1.2, y0 - 3.5),
            ]],
            fill: arrow,
        });
    }
    if f64::from(r0) <= mr && mr <= f64::from(r1) {
        let y = at.y + (mr - f64::from(r0)) * CELL_MM;
        page.ops.push(Op::Area {
            rings: vec![vec![
                Point::new(x0 - 0.3, y),
                Point::new(x0 - 3.5, y - 1.2),
                Point::new(x0 - 3.5, y + 1.2),
            ]],
            fill: arrow,
        });
    }
    // The backstitch lines, on top.
    let area = BBox::new(Point::new(x0, y0), Point::new(x1, y1));
    let half = |h: [i32; 2]| {
        Point::new(
            at.x + (f64::from(h[0]) / 2.0 - f64::from(e.col0 + c0)) * CELL_MM,
            at.y + (f64::from(h[1]) / 2.0 - f64::from(e.row0 + r0)) * CELL_MM,
        )
    };
    for color in colors {
        for l in &color.lines {
            if let Some((a, b)) = clip(half(l[0]), half(l[1]), area) {
                page.line(a, b, 0.6, ink(color.color));
            }
        }
    }
}

/// The chart pages (see the module notes). No pages for no cells.
pub fn chart(colors: &[ChartColor], spec: &ChartSpec) -> Vec<Page> {
    let Some(e) = Extent::of(colors) else { return Vec::new() };
    let mut first = Page::new(spec.paper);
    let (pw, ph) = (first.width_mm, first.height_mm);
    let x = MARGIN_MM;
    let mut y = MARGIN_MM + 6.0;
    first.text(x, y, 16.0, true, format!("{} — cross-stitch chart", spec.title));
    y += 8.0;
    let all: Vec<[usize; 4]> = colors.iter().map(|c| counts(&c.cells)).collect();
    let stitches: usize = all.iter().flatten().sum();
    first.text(
        x,
        y,
        10.0,
        false,
        format!(
            "{} × {} cells, {} stitches, {} color{}",
            e.cols,
            e.rows,
            stitches,
            colors.len(),
            if colors.len() == 1 { "" } else { "s" }
        ),
    );
    y += 7.0;
    // The size on the machine and on Aida fabric.
    let size = |cell: f64| {
        format!(
            "{:.0} × {:.0} mm ({:.1} × {:.1} in)",
            f64::from(e.cols) * cell,
            f64::from(e.rows) * cell,
            f64::from(e.cols) * cell / 25.4,
            f64::from(e.rows) * cell / 25.4
        )
    };
    first.text(x, y, 9.0, true, "Size");
    y += 5.0;
    first.text(
        x,
        y,
        9.0,
        false,
        format!("Machine ({:.2} mm cells): {}", spec.cell_mm, size(spec.cell_mm)),
    );
    for n in AIDA_COUNTS {
        y += 4.5;
        first.text(x, y, 9.0, false, format!("Aida {n}: {}", size(25.4 / f64::from(n))));
    }
    y += 9.0;
    // The key.
    first.text(x, y, 9.0, true, "Key");
    let cols_x = [x, x + 9.0, x + 17.0, x + 95.0, x + 115.0, x + 135.0];
    y += 5.0;
    for (i, h) in ["", "", "Thread", "Full", "Partial", "Lines"].iter().enumerate() {
        first.gray_text(cols_x[i], y, 8.0, *h);
    }
    for (k, (c, n)) in colors.iter().zip(&all).enumerate() {
        if y > ph - MARGIN_MM - 60.0 {
            first.gray_text(x, y + 5.0, 8.0, "More colors: see the chart pages.");
            break;
        }
        y += 5.5;
        let top = Point::new(cols_x[0], y - 3.8);
        draw_cell(&mut first, Part::Full, k, c.color, top, 4.5);
        first.ops.push(Op::Rect {
            rect: BBox::new(top, Point::new(top.x + 4.5, top.y + 4.5)),
            fill: None,
            stroke: Some((0.1, Rgb(120, 120, 120))),
        });
        first.ops.push(Op::Rect {
            rect: BBox::new(Point::new(cols_x[1], y - 3.8), Point::new(cols_x[1] + 6.0, y + 0.7)),
            fill: Some(c.color),
            stroke: Some((0.1, Rgb(120, 120, 120))),
        });
        first.text(cols_x[2], y, 8.5, false, c.name.clone());
        first.text(cols_x[3], y, 8.5, false, n[0].to_string());
        first.text(cols_x[4], y, 8.5, false, (n[1] + n[2] + n[3]).to_string());
        first.text(cols_x[5], y, 8.5, false, c.lines.len().to_string());
    }
    y += 10.0;
    // The whole chart, small.
    let area = BBox::new(Point::new(x, y), Point::new(pw - MARGIN_MM, ph - MARGIN_MM));
    if area.height() > 20.0 {
        let s = (area.width() / f64::from(e.cols))
            .min(area.height() / f64::from(e.rows))
            .min(CELL_MM);
        let at = Point::new(area.min.x, area.min.y);
        for c in colors {
            for cell in &c.cells {
                let p = Point::new(
                    at.x + f64::from(cell.col - e.col0) * s,
                    at.y + f64::from(cell.row - e.row0) * s,
                );
                let corner = |k: Corner| {
                    let (ox, oy) = k.offset();
                    Point::new(p.x + ox * s, p.y + oy * s)
                };
                match cell.part {
                    Part::Full => first.ops.push(Op::Rect {
                        rect: BBox::new(p, Point::new(p.x + s, p.y + s)),
                        fill: Some(c.color),
                        stroke: None,
                    }),
                    Part::HalfUp | Part::HalfDown => first.ops.push(Op::Rect {
                        rect: BBox::new(p, Point::new(p.x + s, p.y + s)),
                        fill: Some(tint(c.color, 0.5)),
                        stroke: None,
                    }),
                    Part::Quarter(q) => {
                        let (ox, oy) = q.offset();
                        let o = Point::new(p.x + ox * s / 2.0, p.y + oy * s / 2.0);
                        first.ops.push(Op::Rect {
                            rect: BBox::new(o, Point::new(o.x + s / 2.0, o.y + s / 2.0)),
                            fill: Some(c.color),
                            stroke: None,
                        });
                    }
                    Part::ThreeQuarter(q) => {
                        let (a, b) = side_corners(q);
                        first.ops.push(Op::Area {
                            rings: vec![vec![corner(q), corner(a), corner(b)]],
                            fill: c.color,
                        });
                    }
                }
            }
            let half = |h: [i32; 2]| {
                Point::new(
                    at.x + (f64::from(h[0]) / 2.0 - f64::from(e.col0)) * s,
                    at.y + (f64::from(h[1]) / 2.0 - f64::from(e.row0)) * s,
                )
            };
            for l in &c.lines {
                first.line(half(l[0]), half(l[1]), (s * 0.2).max(0.2), ink(c.color));
            }
        }
        first.ops.push(Op::Rect {
            rect: BBox::new(
                at,
                Point::new(at.x + f64::from(e.cols) * s, at.y + f64::from(e.rows) * s),
            ),
            fill: None,
            stroke: Some((0.2, Rgb(120, 120, 120))),
        });
    }
    let mut pages = vec![first];
    // The chart pages.
    let per_col = ((pw - 2.0 * MARGIN_MM - LABEL_MM) / CELL_MM).floor().max(1.0) as i32;
    let per_row = ((ph - 2.0 * MARGIN_MM - HEADER_MM - LABEL_MM) / CELL_MM)
        .floor()
        .max(1.0) as i32;
    let (across, down) = ((e.cols + per_col - 1) / per_col, (e.rows + per_row - 1) / per_row);
    let n = across * down;
    for j in 0..down {
        for i in 0..across {
            let mut p = Page::new(spec.paper);
            let (c0, r0) = (i * per_col, j * per_row);
            let (c1, r1) = ((c0 + per_col).min(e.cols), (r0 + per_row).min(e.rows));
            p.text(
                MARGIN_MM,
                MARGIN_MM + 5.0,
                10.0,
                true,
                format!(
                    "{} — page {} of {}: columns {} to {}, rows {} to {}",
                    spec.title,
                    j * across + i + 1,
                    n,
                    c0 + 1,
                    c1,
                    r0 + 1,
                    r1
                ),
            );
            let at = Point::new(MARGIN_MM + LABEL_MM, MARGIN_MM + HEADER_MM + LABEL_MM);
            draw_chart(&mut p, colors, e, (c0, c1), (r0, r1), at);
            pages.push(p);
        }
    }
    pages
}

/// The chart colors of the cross-stitch objects of `design` on the grid of
/// `grid` (or on the grid of its first cross-stitch object): one for each
/// thread, in sew order. Also the cell size of the grid. `None` if the
/// design has no cross-stitch objects.
pub fn colors_of(design: &Design, grid: Option<&CrossStitch>) -> Option<(Vec<ChartColor>, f64)> {
    let crosses = || {
        design.objects().iter().filter_map(|o| match &o.kind {
            ObjectKind::CrossStitch(x) => Some((o.thread, x.as_ref())),
            _ => None,
        })
    };
    let grid = grid.or_else(|| crosses().next().map(|(_, x)| x))?;
    let mut out: Vec<(emb_model::ThreadId, ChartColor)> = Vec::new();
    for (t, x) in crosses().filter(|(_, x)| x.same_grid(grid)) {
        let i = match out.iter().position(|(id, _)| *id == t) {
            Some(i) => i,
            None => {
                let (name, color) = design.thread(t).map_or_else(
                    || ("Thread".to_string(), crate::FALLBACK_COLOR),
                    |th| match &th.code {
                        Some(code) if !code.is_empty() => (format!("{code} {}", th.name), th.color),
                        _ => (th.name.clone(), th.color),
                    },
                );
                out.push((
                    t,
                    ChartColor {
                        name,
                        color,
                        cells: Vec::new(),
                        lines: Vec::new(),
                    },
                ));
                out.len() - 1
            }
        };
        out[i].1.cells.extend(x.cells.iter().copied());
        out[i].1.lines.extend(x.lines.iter().copied());
    }
    Some((out.into_iter().map(|(_, c)| c).collect(), grid.cell_mm()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(name: &str, c: Rgb, cells: &[(i32, i32, Part)]) -> ChartColor {
        ChartColor {
            name: name.into(),
            color: c,
            cells: cells.iter().map(|&(col, row, part)| Cell { row, col, part }).collect(),
            lines: Vec::new(),
        }
    }

    fn spec() -> ChartSpec {
        ChartSpec {
            title: "Heart".into(),
            paper: Paper::A4,
            cell_mm: 2.5,
        }
    }

    #[test]
    fn a_small_chart_has_a_key_page_and_one_chart_page() {
        let mut red = color("Red 1", Rgb(200, 0, 0), &[(5, 5, Part::Full), (6, 5, Part::HalfUp)]);
        red.lines.push([[10, 10], [14, 10]]);
        let blue = color("Blue 2", Rgb(0, 0, 200), &[(5, 6, Part::ThreeQuarter(Corner::TopLeft))]);
        let colors = [red, blue];
        let e = Extent::of(&colors).unwrap();
        assert_eq!(
            e,
            Extent {
                col0: 5,
                row0: 5,
                cols: 2,
                rows: 2
            }
        );
        let pages = chart(&colors, &spec());
        assert_eq!(pages.len(), 2);
        let text: Vec<String> = pages[0]
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(text.iter().any(|t| t == "Red 1"));
        assert!(text.iter().any(|t| t.starts_with("2 × 2 cells, 3 stitches, 2 colors")));
        assert!(text.iter().any(|t| t.starts_with("Aida 14: 4 × 4 mm")), "{text:?}");
        // The backstitch line is on the chart page, 2 cells long.
        let lines: Vec<f64> = pages[1]
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Line { a, b, width_mm, .. } if (*width_mm - 0.6).abs() < 1e-9 => Some(a.distance(*b)),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 1);
        assert!((lines[0] - 2.0 * CELL_MM).abs() < 1e-9);
        // The PDF writer takes the pages.
        let pdf = crate::pdf::write(&pages, "Heart");
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn a_large_chart_goes_on_more_pages() {
        let cells: Vec<(i32, i32, Part)> = (0..150)
            .flat_map(|c| (0..120).map(move |r| (c, r, Part::Full)))
            .collect();
        let pages = chart(&[color("Black", Rgb(0, 0, 0), &cells)], &spec());
        // A4: 60 columns and 85 rows on a page: 3 × 2 chart pages.
        assert_eq!(pages.len(), 1 + 3 * 2);
        // All ops stay on the paper.
        for p in &pages {
            for o in &p.ops {
                if let Op::Line { a, b, .. } = o {
                    for q in [a, b] {
                        assert!(q.x >= 0.0 && q.x <= p.width_mm && q.y >= 0.0 && q.y <= p.height_mm);
                    }
                }
            }
        }
    }

    #[test]
    fn every_symbol_draws_something() {
        for k in 0..SYMBOLS {
            let mut p = Page::new(Paper::A4);
            symbol(&mut p, k, Point::new(50.0, 50.0), 3.0);
            assert!(!p.ops.is_empty(), "symbol {k}");
        }
    }

    #[test]
    fn clip_keeps_the_inside_part() {
        let r = BBox::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let (a, b) = clip(Point::new(-5.0, 5.0), Point::new(5.0, 5.0), r).unwrap();
        assert_eq!((a, b), (Point::new(0.0, 5.0), Point::new(5.0, 5.0)));
        assert!(clip(Point::new(-5.0, 15.0), Point::new(5.0, 15.0), r).is_none());
    }
}
