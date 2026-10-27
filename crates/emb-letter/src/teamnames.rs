//! Team names (P4 Milestone 6, ADR 0020): the design of each row of a
//! team names list, and all rows in one design.

use emb_geom::{BBox, Point, Transform};
use emb_model::{Design, FontRef, ObjectKind, TeamNames, TeamRow, ThreadId};

use crate::layout::relayout;
use crate::source::AnyFont;

/// The design of `row`: each column object gets the value of the row (an
/// empty value removes the object), with a smaller height where a column
/// has a width limit. `font` gives the font of a lettering object.
pub fn row_design(
    d: &Design,
    tn: &TeamNames,
    row: &TeamRow,
    font: &mut dyn FnMut(&FontRef) -> Option<AnyFont>,
) -> Result<Design, String> {
    let mut out = d.clone();
    out.team_names = None;
    for (i, col) in tn.columns.iter().enumerate() {
        let value = row.values.get(i).map(|v| v.trim()).unwrap_or("");
        let Some(ObjectKind::Lettering(l)) = out.object(col.object).map(|o| o.kind.clone()) else {
            return Err(format!("The column \"{}\" has no lettering object.", col.title));
        };
        if value.is_empty() {
            out.remove_object(col.object);
            continue;
        }
        let f = font(&l.font).ok_or_else(|| format!("{} is not on this computer.", l.font.label()))?;
        let mut n = (*l).clone();
        n.retarget_edits(value);
        n.text = value.to_string();
        relayout(&mut n, f.source());
        if let Some(max) = col.max_width_mm
            && let Some(w) = n
                .parts()
                .filter_map(|p| p.control_bbox())
                .reduce(BBox::union)
                .map(|b| b.width())
            && w > max
            && w > 0.0
        {
            n.height_mm *= max / w;
            relayout(&mut n, f.source());
        }
        if let Some(o) = out.object_mut(col.object) {
            o.kind = ObjectKind::Lettering(Box::new(n));
        }
    }
    Ok(out)
}

/// All rows in one design (each row as often as its quantity), in a grid
/// at most `max_width_mm` wide, `gap_mm` apart. The objects sew color by
/// color over all the copies, so the machine changes color as few times as
/// the design has colors.
pub fn layout_design(
    d: &Design,
    tn: &TeamNames,
    font: &mut dyn FnMut(&FontRef) -> Option<AnyFont>,
    max_width_mm: f64,
    gap_mm: f64,
) -> Result<Design, String> {
    let mut copies = Vec::new();
    for row in &tn.rows {
        let rd = row_design(d, tn, row, font)?;
        for _ in 0..row.quantity {
            copies.push(rd.clone());
        }
    }
    let boxes: Vec<Option<BBox>> = copies
        .iter()
        .map(|c| {
            let ids: Vec<_> = c.objects().iter().map(|o| o.id).collect();
            c.bbox_of(&ids)
        })
        .collect();
    let cell_w = boxes.iter().flatten().map(BBox::width).fold(0.0, f64::max) + gap_mm;
    let cell_h = boxes.iter().flatten().map(BBox::height).fold(0.0, f64::max) + gap_mm;
    let per_row = (((max_width_mm + gap_mm) / cell_w.max(1e-6)).floor() as usize).max(1);
    let rows = copies.len().div_ceil(per_row);
    // The grid is centered on the hoop center.
    let c = d.hoop.center;
    let origin = Point::new(
        c.x - (per_row.min(copies.len()) as f64 * cell_w - gap_mm) / 2.0 + (cell_w - gap_mm) / 2.0,
        c.y - (rows as f64 * cell_h - gap_mm) / 2.0 + (cell_h - gap_mm) / 2.0,
    );
    let moved: Vec<Vec<emb_model::DesignObject>> = copies
        .iter()
        .zip(&boxes)
        .enumerate()
        .map(|(k, (copy, b))| {
            let cell = Point::new(
                origin.x + (k % per_row) as f64 * cell_w,
                origin.y + (k / per_row) as f64 * cell_h,
            );
            let at = b.map_or(Point::ZERO, |b| b.center());
            let t = Transform::translate(cell.x - at.x, cell.y - at.y);
            copy.objects().iter().map(|o| o.transformed(&t)).collect()
        })
        .collect();
    let mut out = d.clone();
    out.team_names = None;
    out.monograms.clear();
    let ids: Vec<_> = out.objects().iter().map(|o| o.id).collect();
    for id in ids {
        out.remove_object(id);
    }
    // Color by color, in the order the colors first sew in the design.
    let mut threads: Vec<ThreadId> = Vec::new();
    for o in d.objects() {
        if !threads.contains(&o.thread) {
            threads.push(o.thread);
        }
    }
    for t in threads {
        for copy in &moved {
            for o in copy.iter().filter(|o| o.thread == t) {
                let id = out.add_object(o.thread, o.kind.clone());
                if let Some(n) = out.object_mut(id) {
                    n.name = o.name.clone();
                    n.functions = o.functions;
                    n.start = o.start;
                    n.end = o.end;
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use emb_model::{Lettering, Rgb, TeamColumn};

    use super::*;
    use crate::bundled;

    fn team() -> (Design, TeamNames) {
        let f = bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap();
        let mut d = Design::new("Team");
        let navy = d.add_thread("Navy", Rgb(25, 35, 90));
        let red = d.add_thread("Red", Rgb(200, 30, 45));
        let mut name = Lettering::new("NAME", crate::LetterSource::font_ref(&f), 12.0, Point::new(0.0, -10.0));
        name.justify = emb_model::Justify::Center;
        relayout(&mut name, &f);
        let mut number = Lettering::new("00", crate::LetterSource::font_ref(&f), 25.0, Point::new(0.0, 25.0));
        number.justify = emb_model::Justify::Center;
        relayout(&mut number, &f);
        let a = d.add_object(navy, ObjectKind::Lettering(Box::new(name)));
        let b = d.add_object(red, ObjectKind::Lettering(Box::new(number)));
        let mut tn = TeamNames {
            columns: vec![
                TeamColumn {
                    object: a,
                    title: "Name".into(),
                    max_width_mm: Some(60.0),
                },
                TeamColumn {
                    object: b,
                    title: "Number".into(),
                    max_width_mm: None,
                },
            ],
            rows: vec![],
        };
        tn.import_text("Anna,7\nMaximiliana Longname,12\nLee,\n", false);
        (d, tn)
    }

    fn fonts() -> impl FnMut(&FontRef) -> Option<AnyFont> {
        let f = Arc::new(bundled::embroidery(bundled::DEFAULT_EMBROIDERY).unwrap());
        move |_| Some(AnyFont::Embroidery(f.clone()))
    }

    #[test]
    fn each_row_gets_its_values() {
        let (d, tn) = team();
        let mut font = fonts();
        let anna = row_design(&d, &tn, &tn.rows[0], &mut font).unwrap();
        let ObjectKind::Lettering(l) = &anna.objects()[0].kind else {
            panic!()
        };
        assert_eq!(l.text, "Anna");
        // A long name gets smaller to fit 60 mm.
        let long = row_design(&d, &tn, &tn.rows[1], &mut font).unwrap();
        let ObjectKind::Lettering(l) = &long.objects()[0].kind else {
            panic!()
        };
        let w = l
            .parts()
            .filter_map(|p| p.control_bbox())
            .reduce(BBox::union)
            .unwrap()
            .width();
        assert!(w <= 60.5 && l.height_mm < 12.0, "{w} {}", l.height_mm);
        // No number: no number object.
        let lee = row_design(&d, &tn, &tn.rows[2], &mut font).unwrap();
        assert_eq!(lee.objects().len(), 1);
    }

    #[test]
    fn all_names_in_one_design_color_by_color() {
        let (d, mut tn) = team();
        tn.rows[0].quantity = 2;
        let mut font = fonts();
        let all = layout_design(&d, &tn, &mut font, 200.0, 10.0).unwrap();
        // 4 copies; Lee has no number: 4 names and 3 numbers.
        assert_eq!(all.objects().len(), 7);
        let threads: Vec<ThreadId> = all.objects().iter().map(|o| o.thread).collect();
        let changes = threads.windows(2).filter(|w| w[0] != w[1]).count();
        assert_eq!(changes, 1, "all names, then all numbers");
        // The copies do not overlap.
        let boxes: Vec<BBox> = all.objects()[..4].iter().filter_map(|o| o.control_bbox()).collect();
        for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                let (a, b) = (boxes[i], boxes[j]);
                let overlap = a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y;
                assert!(!overlap, "{i} {j}");
            }
        }
    }
}
