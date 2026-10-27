//! The hooping sheet (P7 Milestone 3, ADR 0032): a page that shows where
//! each hooping of a large design goes, the order, the files, and how to
//! use the registration marks.

use emb_geom::{BBox, Point};
use emb_model::Rgb;

use crate::page::{Fit, Op, Page, Paper, wrap};
use crate::{Segment, SegmentKind};

const MARGIN_MM: f64 = 15.0;

/// The colors of the hooping frames, in turn.
pub const HOOPING_COLORS: [Rgb; 6] = [
    Rgb(0, 114, 178),
    Rgb(213, 94, 0),
    Rgb(0, 158, 115),
    Rgb(204, 121, 167),
    Rgb(230, 159, 0),
    Rgb(86, 180, 233),
];

/// One hooping in the table of the sheet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HoopingRow {
    /// The machine files of the hooping.
    pub files: String,
    pub stitches: usize,
    pub colors: usize,
    pub turned: bool,
    /// The hooping center from the center of hooping 1, in mm (Y down).
    pub offset: Point,
    /// The hooping sews registration marks first (after the hooping before it).
    pub marks_first: bool,
    /// The hooping sews registration marks last (for a later hooping).
    pub marks_last: bool,
    /// The position of a multi-position hoop.
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct HoopingSheet {
    pub name: String,
    pub hoop_name: String,
    pub date: String,
    /// The stitches of all of the design, in the same coordinates as `fields`.
    pub segments: Vec<Segment>,
    /// The usable field of each hooping.
    pub fields: Vec<BBox>,
    pub rows: Vec<HoopingRow>,
    pub warnings: Vec<String>,
    /// The hoopings are the positions of a multi-position hoop: the fabric
    /// stays in the hoop, and the hoop goes on the machine in each position.
    pub positions: bool,
}

/// The hooping sheet as one page.
pub fn overview(s: &HoopingSheet, paper: Paper) -> Page {
    let mut page = Page::new(paper);
    let w = page.width_mm;
    let mut y = MARGIN_MM + 5.0;
    page.text(MARGIN_MM, y, 16.0, true, format!("Hoopings: {}", s.name));
    y += 6.0;
    page.gray_text(
        MARGIN_MM,
        y,
        9.0,
        format!("{} hoopings of the {}  ·  {}", s.rows.len(), s.hoop_name, s.date),
    );
    y += 6.0;

    // The picture: the design with the frame of each hooping.
    let area = BBox::new(Point::new(MARGIN_MM, y), Point::new(w - MARGIN_MM, y + 110.0));
    let all = s
        .fields
        .iter()
        .copied()
        .chain(BBox::from_points(s.segments.iter().flat_map(|g| [g.a, g.b])))
        .reduce(BBox::union);
    if let Some(b) = all {
        let fit = Fit::of(b, area);
        let segments: Vec<Segment> = s
            .segments
            .iter()
            .filter(|g| g.kind == SegmentKind::Stitch)
            .map(|g| Segment {
                a: fit.apply(g.a),
                b: fit.apply(g.b),
                ..*g
            })
            .collect();
        page.ops.push(Op::Stitches {
            segments,
            width_mm: 0.25,
        });
        for (k, f) in s.fields.iter().enumerate() {
            let color = HOOPING_COLORS[k % HOOPING_COLORS.len()];
            let r = BBox::new(fit.apply(f.min), fit.apply(f.max));
            page.ops.push(Op::Rect {
                rect: r,
                fill: None,
                stroke: Some((0.5, color)),
            });
            page.ops.push(Op::Text {
                at: Point::new(r.min.x + 2.0, r.min.y + 6.0),
                size_pt: 14.0,
                bold: true,
                color,
                text: format!("{}", k + 1),
            });
        }
    }
    y = area.max.y + 8.0;

    // The table.
    page.text(MARGIN_MM, y, 11.0, true, "Sew in this order");
    y += 6.0;
    for (k, r) in s.rows.iter().enumerate() {
        let marks = match (r.marks_first, r.marks_last) {
            (true, true) => "marks first and last",
            (true, false) => "marks first",
            (false, true) => "marks last",
            (false, false) => "no marks",
        };
        page.text(MARGIN_MM, y, 9.0, true, format!("{}.", k + 1));
        page.text(MARGIN_MM + 7.0, y, 9.0, false, r.files.clone());
        y += 4.5;
        let place = match r.position {
            Some(p) => format!("hoop position {}", p + 1),
            None => format!(
                "center {:.1} mm right, {:.1} mm down from hooping 1",
                r.offset.x, r.offset.y
            ),
        };
        page.gray_text(
            MARGIN_MM + 7.0,
            y,
            8.0,
            format!(
                "{} stitches, {} colors · hoop {} · {place} · {marks}",
                r.stitches,
                r.colors,
                if r.turned {
                    "turned (top of the design to the right on the machine)"
                } else {
                    "straight"
                },
            ),
        );
        y += 6.0;
    }

    // What to do.
    y += 2.0;
    page.text(MARGIN_MM, y, 11.0, true, "How to sew");
    y += 5.5;
    let positions = [
        "Hoop the fabric once, in the multi-position hoop, as the picture shows.",
        "Put the hoop on the machine in position 1 and sew file 1.",
        "Move the hoop to the position of the next file (the fabric stays in the hoop) and sew \
         that file. Do the same for each file.",
    ];
    let hoopings = [
        "Hoop the fabric for hooping 1 at the place in the picture, and sew its file.",
        "Take the fabric out. Hoop it again for the next hooping: its registration marks \
         (the crosses) and its part of the design must be inside the field.",
        "Sew the next file. It sews the registration marks first, in their own color, and \
         the machine stops. If the new crosses are not on the old crosses, move the design \
         on the machine (or hoop again) until they are. Then sew the rest.",
        "Do the same for each hooping. At the end, pull out the stitches of the crosses.",
    ];
    let mut steps: Vec<&str> = if s.positions {
        positions.to_vec()
    } else {
        hoopings.to_vec()
    };
    if s.rows.iter().any(|r| r.turned) {
        steps.push(
            "A turned hooping has the top of the design to the right on the machine (the machine screen shows it so). Its worksheet shows the hoop and the design.",
        );
    }
    for (k, t) in steps.iter().enumerate() {
        for (i, line) in wrap(t, 100).into_iter().enumerate() {
            let text = if i == 0 {
                format!("{}. {line}", k + 1)
            } else {
                format!("    {line}")
            };
            page.text(MARGIN_MM, y, 8.5, false, text);
            y += 4.2;
        }
    }
    if !s.warnings.is_empty() {
        y += 3.0;
        page.text(MARGIN_MM, y, 11.0, true, "Warnings");
        y += 5.5;
        for warning in &s.warnings {
            for line in wrap(warning, 105) {
                page.gray_text(MARGIN_MM, y, 8.0, line);
                y += 4.0;
            }
        }
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sheet_has_a_frame_and_a_row_for_each_hooping() {
        let s = HoopingSheet {
            name: "big".into(),
            hoop_name: "Crown".into(),
            fields: vec![
                BBox::from_center_size(Point::new(-90.0, 0.0), 250.0, 190.0),
                BBox::from_center_size(Point::new(90.0, 0.0), 250.0, 190.0),
            ],
            rows: vec![
                HoopingRow {
                    files: "big_1.vp3".into(),
                    marks_last: true,
                    ..Default::default()
                },
                HoopingRow {
                    files: "big_2.vp3".into(),
                    offset: Point::new(180.0, 0.0),
                    marks_first: true,
                    turned: true,
                    ..Default::default()
                },
            ],
            warnings: vec!["A warning".into()],
            ..Default::default()
        };
        let p = overview(&s, Paper::A4);
        let frames = p.ops.iter().filter(|o| matches!(o, Op::Rect { .. })).count();
        assert_eq!(frames, 2);
        let texts: Vec<&str> = p
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"big_2.vp3"));
        assert!(
            texts
                .iter()
                .any(|t| t.contains("180.0 mm right") && t.contains("marks first"))
        );
        assert!(texts.contains(&"A warning"));
        // A turned hooping: a step says where the top of the design is.
        assert!(
            texts
                .iter()
                .any(|t| t.contains("A turned hooping has the top of the design"))
        );
        // All of it is on the page.
        assert!(p.ops.iter().all(|o| match o {
            Op::Text { at, .. } => at.y < p.height_mm,
            _ => true,
        }));
    }
}
