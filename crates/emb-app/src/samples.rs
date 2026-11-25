//! The start designs of the tutorials (P19 Part 2, ADR 0162). They are made
//! in code, as the corpus designs are, so a tutorial opens the same design
//! on each computer, and nothing goes in the folders of the user (ADR
//! 0080). A help topic opens one with a link `[text](sample:<name>)`.

use emb_geom::{Join, Path, Point};
use emb_model::{
    BorderPlace, ConstructionMark, Design, FillParams, MarkShape, ObjectKind, OffsetMake, OffsetSpec, Rgb,
};

use crate::app::AppState;
use crate::i18n::tr;

/// The start designs: the name in a `sample:` link, and the name of the
/// design (shown in its tab, translated).
pub const SAMPLES: &[(&str, &str)] = &[("first-steps", "First design"), ("first-badge", "Badge")];

/// The prefix of a link to a start design in a help topic.
pub const LINK: &str = "sample:";

/// The two corners of the circle of the first design, in mm: the start
/// design marks them with construction points.
pub const FIRST_CORNERS: [Point; 2] = [Point { x: -30.0, y: -30.0 }, Point { x: 30.0, y: 30.0 }];

/// The border that **Add Border** puts round the selection: a line with a
/// satin border, as the Offsets window that `Cmd::AddBorder` opens makes
/// it with the values of a new session. The badge of the second tutorial
/// is the result of the first tutorial (the step files check its size).
pub fn add_border_spec() -> OffsetSpec {
    OffsetSpec {
        distance_mm: 1.0,
        count: 1,
        spacing_mm: OffsetSpec::default().spacing_mm,
        corners: Join::Round,
        make: OffsetMake::Border {
            width_mm: 3.0,
            place: BorderPlace::Outside,
        },
    }
}

/// The start design `name`, or `None` if there is no such design.
pub fn design(name: &str) -> Option<Design> {
    let (_, title) = SAMPLES.iter().find(|(n, _)| *n == name)?;
    let mut d = Design::new(*title);
    let gold = d.add_thread("Gold", Rgb(230, 170, 40));
    let navy = d.add_thread("Navy", Rgb(25, 35, 90));
    match name {
        // An empty design with the two corners of the circle to draw.
        "first-steps" => {
            for at in FIRST_CORNERS {
                d.marks.push(ConstructionMark::new(MarkShape::Point { at }));
            }
        }
        // The badge of the first tutorial: a gold circle with a navy border.
        "first-badge" => {
            let [a, b] = FIRST_CORNERS;
            let fill = d.add_object(
                gold,
                ObjectKind::Fill {
                    outer: Path::ellipse(a.midpoint(b), (b.x - a.x) / 2.0, (b.y - a.y) / 2.0),
                    holes: Vec::new(),
                    params: FillParams::default(),
                },
            );
            let border = d.add_offsets(&[fill], &add_border_spec()).ok()?;
            for id in border {
                if let Some(o) = d.object_mut(id) {
                    o.thread = navy;
                }
            }
        }
        _ => return None,
    }
    Some(d)
}

impl AppState {
    /// Opens the start design `name` in a new tab, on the machine and in
    /// the hoop of the design of now (as a new design).
    pub fn open_sample(&mut self, name: &str) {
        let Some(mut d) = design(name) else {
            self.message = crate::i18n::fill("No start design {}.", &[&name]);
            return;
        };
        d.name = tr(&d.name).to_string();
        d.machine = self.doc.design().machine.clone();
        d.hoop.selection = self.doc.design().hoop.selection.clone();
        let title = d.name.clone();
        self.start_design(d);
        self.message = crate::i18n::fill("Start design \"{}\"", &[&title]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each start design is made, and has what its tutorial needs.
    #[test]
    fn each_start_design_is_made() {
        for (name, _) in SAMPLES {
            let d = design(name).unwrap_or_else(|| panic!("{name}"));
            assert_eq!(d.threads.len(), 2, "{name}");
        }
        assert_eq!(design("first-steps").unwrap().marks.len(), 2);
        let badge = design("first-badge").unwrap();
        let kinds: Vec<&str> = badge
            .objects()
            .iter()
            .map(|o| match o.kind {
                ObjectKind::Fill { .. } => "fill",
                ObjectKind::Running { ref params, .. } if params.border.is_some() => "satin border",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["fill", "satin border"]);
        assert_ne!(badge.objects()[0].thread, badge.objects()[1].thread);
        assert!(design("no such design").is_none());
    }

    /// A start design opens in a new tab, with the machine of the design
    /// before.
    #[test]
    fn a_start_design_opens_in_a_new_tab() {
        let mut h = crate::test_util::Harness::new();
        let machine = h.st.doc.design().machine.clone();
        h.st.open_sample("first-badge");
        assert_eq!(h.st.doc.design().objects().len(), 2);
        assert_eq!(h.st.doc.design().machine, machine);
        assert_eq!(h.st.doc.design().name, "Badge");
    }
}
