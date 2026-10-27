//! The design catalog: pages with a picture of each design and its
//! numbers, 4 × 5 designs on a page.

use emb_geom::{BBox, Point};
use emb_model::{Rgb, StitchDesign};
use emb_render::page::{Op, Page, Paper, fit_segments, text_width_mm};

use crate::Entry;

const MARGIN: f64 = 12.0;
const HEADER: f64 = 12.0;
const COLUMNS: usize = 4;
const ROWS: usize = 5;
/// The space for the text under each picture, in mm.
const TEXT: f64 = 13.0;
const FRAME: Rgb = Rgb(200, 200, 200);

/// The number of designs on a catalog page.
pub const PER_PAGE: usize = COLUMNS * ROWS;

/// One design of the catalog: its entry and its stitches (`None` if the
/// file cannot be read now).
pub struct Item<'a> {
    pub entry: &'a Entry,
    pub stitches: Option<StitchDesign>,
}

/// The catalog pages. `title` is at the top of each page (for example the
/// search text and the date).
pub fn pages(title: &str, items: &[Item], paper: Paper) -> Vec<Page> {
    let n = items.len().div_ceil(PER_PAGE).max(1);
    let mut out = Vec::with_capacity(n);
    for (k, chunk) in items.chunks(PER_PAGE).enumerate() {
        out.push(page(title, chunk, paper, k + 1, n));
    }
    if out.is_empty() {
        let mut p = page(title, &[], paper, 1, 1);
        p.gray_text(MARGIN, MARGIN + HEADER + 6.0, 10.0, "No designs.");
        out.push(p);
    }
    out
}

fn page(title: &str, items: &[Item], paper: Paper, number: usize, of: usize) -> Page {
    let mut p = Page::new(paper);
    let (w, h) = (p.width_mm, p.height_mm);
    p.text(
        MARGIN,
        MARGIN + 5.0,
        13.0,
        true,
        fit_text(title, 13.0, w - 2.0 * MARGIN - 30.0),
    );
    let pages = format!("Page {number} of {of}");
    p.gray_text(w - MARGIN - text_width_mm(&pages, 8.0), MARGIN + 5.0, 8.0, pages);
    let (cw, ch) = (
        (w - 2.0 * MARGIN) / COLUMNS as f64,
        (h - 2.0 * MARGIN - HEADER) / ROWS as f64,
    );
    for (i, item) in items.iter().enumerate() {
        let (col, row) = (i % COLUMNS, i / COLUMNS);
        let cell = BBox::new(
            Point::new(MARGIN + col as f64 * cw, MARGIN + HEADER + row as f64 * ch),
            Point::new(MARGIN + (col + 1) as f64 * cw, MARGIN + HEADER + (row + 1) as f64 * ch),
        );
        cell_ops(&mut p, item, cell);
    }
    p
}

fn cell_ops(p: &mut Page, item: &Item, cell: BBox) {
    let pad = 2.0;
    p.ops.push(Op::Rect {
        rect: BBox::new(cell.min + Point::new(1.0, 1.0), cell.max - Point::new(1.0, 1.0)),
        fill: None,
        stroke: Some((0.2, FRAME)),
    });
    let picture = BBox::new(
        cell.min + Point::new(pad + 1.0, pad + 1.0),
        Point::new(cell.max.x - pad - 1.0, cell.max.y - TEXT),
    );
    let e = item.entry;
    match &item.stitches {
        Some(s) if s.stitch_count() > 0 => {
            let (segments, _) = fit_segments(&emb_render::segments(s), picture);
            p.ops.push(Op::Stitches {
                segments,
                width_mm: 0.25,
            });
        }
        _ => p.gray_text(picture.min.x, picture.center().y, 7.0, "(no picture)"),
    }
    let x = cell.min.x + pad + 1.0;
    let width = cell.width() - 2.0 * pad - 2.0;
    let y = cell.max.y - TEXT + 3.5;
    p.text(x, y, 8.0, true, fit_text(&e.name, 8.0, width));
    let line2 = match &e.error {
        Some(_) => format!("{} · cannot be read", e.format.to_uppercase()),
        None => format!(
            "{} · {:.0} × {:.0} mm · {} color{}",
            e.format.to_uppercase(),
            e.width_mm,
            e.height_mm,
            e.colors,
            if e.colors == 1 { "" } else { "s" }
        ),
    };
    p.gray_text(x, y + 3.4, 6.5, fit_text(&line2, 6.5, width));
    let line3 = if e.folder.is_empty() {
        format!("{} stitches · {}", e.stitches, e.file)
    } else {
        format!("{} stitches · {}/{}", e.stitches, e.folder, e.file)
    };
    p.gray_text(x, y + 6.6, 6.5, fit_text(&line3, 6.5, width));
}

/// The text, cut with "…" so that it is at most `width` mm wide.
fn fit_text(text: &str, size_pt: f64, width: f64) -> String {
    if text_width_mm(text, size_pt) <= width {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() && text_width_mm(&chars.iter().collect::<String>(), size_pt) + 1.5 > width {
        chars.pop();
    }
    chars.into_iter().collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::tests::entry;

    #[test]
    fn twenty_designs_on_a_page() {
        let entries: Vec<Entry> = (0..45)
            .map(|k| {
                entry(
                    &format!("Design {k} with a very long name to cut"),
                    "caps",
                    "vp3",
                    50.0,
                    40.0,
                    2,
                )
            })
            .collect();
        let items: Vec<Item> = entries
            .iter()
            .enumerate()
            .map(|(k, e)| Item {
                entry: e,
                stitches: (k != 3).then(|| crate::index::tests::square(30.0, 2)),
            })
            .collect();
        let pages = pages("Search: caps", &items, Paper::A4);
        assert_eq!(pages.len(), 3);
        let texts = |p: &Page| -> Vec<String> {
            p.ops
                .iter()
                .filter_map(|o| match o {
                    Op::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        let first = texts(&pages[0]);
        assert!(first.contains(&"Page 1 of 3".to_string()));
        assert!(first.iter().any(|t| t.starts_with("Design 0 with") && t.ends_with('…')));
        assert!(first.contains(&"(no picture)".to_string()));
        assert!(first.iter().any(|t| t == "VP3 · 50 × 40 mm · 2 colors"));
        let pictures = pages[0].ops.iter().filter(|o| matches!(o, Op::Stitches { .. })).count();
        assert_eq!(pictures, 19);
        // All of it on the paper.
        for p in &pages {
            for o in &p.ops {
                if let Op::Text { at, text, size_pt, .. } = o {
                    assert!(
                        at.x + text_width_mm(text, *size_pt) <= p.width_mm - MARGIN + 0.01,
                        "{text}"
                    );
                    assert!(at.y < p.height_mm - MARGIN, "{text}");
                }
            }
        }
        assert_eq!(pages[2].ops.iter().filter(|o| matches!(o, Op::Rect { .. })).count(), 5);
        assert_eq!(super::pages("Nothing", &[], Paper::Letter).len(), 1);
    }
}
