//! Stitch types of the selection: the outline type of lines and the column
//! type of satin columns (P5, ADR 0022). The ribbon (Edit > Stitch types)
//! and Object Properties use these functions.

use emb_model::{ColumnPattern, FillPattern, LineBorder, ObjectId, ObjectKind, SatinParams};

use crate::app::AppState;
use crate::i18n::tr;

/// Width of a new border on a line, in mm.
pub const DEFAULT_BORDER_MM: f64 = 3.0;

/// Sets the outline type of the lines in `ids`: `None` makes them plain
/// running lines, `Some` sews each as a column of that type. A line that
/// has a border keeps its width. One undo step; returns the number of lines changed.
pub fn set_outline(st: &mut AppState, ids: &[ObjectId], pattern: Option<ColumnPattern>) -> usize {
    let lines: Vec<ObjectId> = ids
        .iter()
        .copied()
        .filter(|id| {
            matches!(
                st.doc.design().object(*id).map(|o| &o.kind),
                Some(ObjectKind::Running { params, .. })
                    if params.motif.is_some()
                        || params.ripple.is_some()
                        || params.border.as_ref().map(|b| b.column.pattern) != pattern
            )
        })
        .collect();
    if lines.is_empty() {
        return 0;
    }
    let label = format!("Outline type {}", pattern.map_or("Line", |p| p.name()));
    st.doc.edit(label, |d| {
        for id in &lines {
            if let Some(ObjectKind::Running { params, .. }) = d.object_mut(*id).map(|o| &mut o.kind) {
                let width = params.border.as_ref().map_or(DEFAULT_BORDER_MM, |b| b.width_mm);
                params.border = pattern.map(|p| LineBorder::new(width, p));
                params.motif = None;
                params.ripple = None;
            }
        }
    });
    lines.len()
}

/// The column settings of a satin column, or of a line with a border.
pub fn column_params(k: &mut ObjectKind) -> Option<&mut SatinParams> {
    match k {
        ObjectKind::Satin { params, .. } => Some(params),
        ObjectKind::Running { params, .. } => params.border.as_mut().map(|b| &mut b.column),
        _ => None,
    }
}

/// Sets the column type of the satin columns and bordered lines in `ids`,
/// with the default settings of the type (pull compensation and split
/// length stay). One undo step; returns the number of objects changed.
pub fn set_column(st: &mut AppState, ids: &[ObjectId], pattern: ColumnPattern) -> usize {
    let targets: Vec<ObjectId> = ids
        .iter()
        .copied()
        .filter(|id| {
            let mut kind = st.doc.design().object(*id).map(|o| o.kind.clone());
            kind.as_mut()
                .and_then(column_params)
                .is_some_and(|c| !c.pattern.same_type(&pattern))
        })
        .collect();
    if targets.is_empty() {
        return 0;
    }
    st.doc.edit(format!("Column type {}", pattern.name()), |d| {
        for id in &targets {
            if let Some(c) = d.object_mut(*id).and_then(|o| column_params(&mut o.kind)) {
                *c = SatinParams {
                    pull_compensation_mm: c.pull_compensation_mm,
                    split_mm: c.split_mm,
                    ..SatinParams::with_pattern(pattern)
                };
            }
        }
    });
    targets.len()
}

/// Sets the fill type of the fills in `ids`, with the default settings of
/// the type (the angle and the underlay stay). One undo step; returns the
/// number of fills changed.
pub fn set_fill(st: &mut AppState, ids: &[ObjectId], pattern: FillPattern) -> usize {
    let fills: Vec<ObjectId> = ids
        .iter()
        .copied()
        .filter(|id| {
            matches!(
                st.doc.design().object(*id).map(|o| &o.kind),
                Some(ObjectKind::Fill { params, .. }) if !params.pattern.same_type(&pattern)
            )
        })
        .collect();
    if fills.is_empty() {
        return 0;
    }
    st.doc.edit(format!("Fill type {}", pattern.name()), |d| {
        for id in &fills {
            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(*id).map(|o| &mut o.kind) {
                *params = params.with_pattern(pattern.clone());
            }
        }
    });
    fills.len()
}

/// The selection is one fill and one line: the line becomes the guide of a
/// Florentine fill, and the line is removed. One undo step.
pub fn florentine_guide(st: &mut AppState) -> bool {
    let d = st.doc.design();
    let ids = st.selection.in_sew_order(d);
    let find = |want_fill: bool| {
        ids.iter().copied().find(|id| match d.object(*id).map(|o| &o.kind) {
            Some(ObjectKind::Fill { .. }) => want_fill,
            Some(ObjectKind::Running { .. }) => !want_fill,
            _ => false,
        })
    };
    let (Some(fill), Some(line), 2) = (find(true), find(false), ids.len()) else {
        st.message = tr("Select one fill and one line: the line becomes the guide of the fill rows.").into();
        return false;
    };
    let Some(guide) = d.object(line).and_then(|o| o.kind.paths().next()).and_then(|p| {
        p.flatten(0.1)
            .into_iter()
            .next()
            .map(|l| l.points)
            .filter(|pts| pts.len() >= 2)
    }) else {
        return false;
    };
    st.doc.edit("Guide Line", |d| {
        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(fill).map(|o| &mut o.kind) {
            if !matches!(params.pattern, FillPattern::Florentine { .. }) {
                *params = params.with_pattern(FillPattern::Florentine { guide: Vec::new() });
            }
            params.pattern = FillPattern::Florentine { guide };
        }
        d.remove_object(line);
    });
    st.selection.set(fill);
    st.message = tr("The line is now the guide of the fill rows (Florentine).").into();
    true
}

/// The selection is one fill and one or more lines: the lines become split
/// lines of the fill (user split), and are removed. One undo step.
pub fn split_lines(st: &mut AppState) -> bool {
    let d = st.doc.design();
    let ids = st.selection.in_sew_order(d);
    let fills: Vec<ObjectId> = ids
        .iter()
        .copied()
        .filter(|id| matches!(d.object(*id).map(|o| &o.kind), Some(ObjectKind::Fill { .. })))
        .collect();
    let lines = line_points(st, &ids);
    let ([fill], false) = (fills.as_slice(), lines.is_empty()) else {
        st.message = tr("Select one fill and one or more lines: the lines split the fill rows.").into();
        return false;
    };
    let fill = *fill;
    let line_ids: Vec<ObjectId> = {
        let mut v: Vec<ObjectId> = lines.iter().map(|(id, _)| *id).collect();
        v.dedup();
        v
    };
    st.doc.edit("Split Lines", |d| {
        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(fill).map(|o| &mut o.kind) {
            let mut all = match &params.pattern {
                FillPattern::UserSplit { lines } => lines.clone(),
                _ => {
                    *params = params.with_pattern(FillPattern::UserSplit { lines: Vec::new() });
                    Vec::new()
                }
            };
            all.extend(lines.into_iter().map(|(_, l)| l));
            params.pattern = FillPattern::UserSplit { lines: all };
        }
        for id in &line_ids {
            d.remove_object(*id);
        }
    });
    st.selection.set(fill);
    st.message = tr("The lines now split the fill rows (user split).").into();
    true
}

/// The points of each part of the lines of `ids` (a closed part ends at its start).
fn line_points(st: &AppState, ids: &[ObjectId]) -> Vec<(ObjectId, Vec<emb_geom::Point>)> {
    let d = st.doc.design();
    ids.iter()
        .filter_map(|id| match d.object(*id).map(|o| &o.kind) {
            Some(ObjectKind::Running { path, .. }) => Some(
                path.flatten(0.1)
                    .into_iter()
                    .filter(|l| l.points.len() >= 2)
                    .map(|l| {
                        let mut pts = l.points;
                        if l.closed {
                            pts.push(pts[0]);
                        }
                        (*id, pts)
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The selection is one fill and one or more lines: the lines become the
/// guides of a guided fill (P11 Milestone 4), and are removed. One undo step.
pub fn guide_lines(st: &mut AppState) -> bool {
    let d = st.doc.design();
    let ids = st.selection.in_sew_order(d);
    let fills: Vec<ObjectId> = ids
        .iter()
        .copied()
        .filter(|id| matches!(d.object(*id).map(|o| &o.kind), Some(ObjectKind::Fill { .. })))
        .collect();
    let lines = line_points(st, &ids);
    let ([fill], false) = (fills.as_slice(), lines.is_empty()) else {
        st.message = tr("Select one fill and one or more lines: the fill rows follow the lines.").into();
        return false;
    };
    let fill = *fill;
    let mut line_ids: Vec<ObjectId> = lines.iter().map(|(id, _)| *id).collect();
    line_ids.dedup();
    st.doc.edit("Guide Lines", |d| {
        if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(fill).map(|o| &mut o.kind) {
            let mut all = match &params.pattern {
                FillPattern::Guided { guides } => guides.clone(),
                _ => {
                    *params = params.with_pattern(FillPattern::Guided { guides: Vec::new() });
                    Vec::new()
                }
            };
            all.extend(lines.into_iter().map(|(_, l)| l));
            params.pattern = FillPattern::Guided { guides: all };
        }
        for id in &line_ids {
            d.remove_object(*id);
        }
    });
    st.selection.set(fill);
    st.message = tr("The fill rows now follow the lines (guided fill).").into();
    true
}

/// Sets a ripple on the lines of `ids` (`None`: a plain line again). One
/// undo step; returns the number of lines changed.
pub fn set_ripple(st: &mut AppState, ids: &[ObjectId], ripple: Option<emb_model::Ripple>) -> usize {
    let lines: Vec<ObjectId> = ids
        .iter()
        .copied()
        .filter(|id| {
            matches!(
                st.doc.design().object(*id).map(|o| &o.kind),
                Some(ObjectKind::Running { params, .. }) if params.ripple != ripple
            )
        })
        .collect();
    if lines.is_empty() {
        return 0;
    }
    st.doc.edit(if ripple.is_some() { "Ripple" } else { "No ripple" }, |d| {
        for id in &lines {
            if let Some(ObjectKind::Running { params, .. }) = d.object_mut(*id).map(|o| &mut o.kind) {
                params.ripple = ripple.clone();
                if ripple.is_some() {
                    params.border = None;
                    params.motif = None;
                }
            }
        }
    });
    lines.len()
}

/// The selection is two lines: the later one in the sew order becomes the
/// guide of a ripple of the first one, and is removed. One undo step.
pub fn ripple_guide(st: &mut AppState) -> bool {
    let d = st.doc.design();
    let ids = st.selection.in_sew_order(d);
    let lines = line_points(st, &ids);
    let ([a, b], 2) = (ids.as_slice(), lines.len()) else {
        st.message = tr("Select two lines: the second line becomes the guide of a ripple of the first.").into();
        return false;
    };
    let (line, guide_id) = (*a, *b);
    let Some(guide) = lines.iter().find(|(id, _)| *id == guide_id).map(|(_, p)| p.clone()) else {
        st.message = tr("Select two lines: the second line becomes the guide of a ripple of the first.").into();
        return false;
    };
    st.doc.edit("Ripple Guide", |d| {
        if let Some(ObjectKind::Running { params, .. }) = d.object_mut(line).map(|o| &mut o.kind) {
            let mut r = params.ripple.take().unwrap_or_default();
            if r.guide.is_empty() {
                // Copies along a guide keep their size.
                r.end_percent = 100.0;
            }
            r.guide = guide;
            params.ripple = Some(r);
            params.border = None;
            params.motif = None;
        }
        d.remove_object(guide_id);
    });
    st.selection.set(line);
    st.message = tr("The copies of the line now move along the guide (ripple).").into();
    true
}

/// Puff (3D foam, ADR 0050) on or off for the satin columns, the lines
/// with a border and the lettering of `ids`. On: dense satin
/// (`PUFF_SPACING_MM`) of the satin type. One undo step; returns the number
/// of objects changed.
pub fn set_puff(st: &mut AppState, ids: &[ObjectId], on: bool) -> usize {
    let targets: Vec<ObjectId> = ids
        .iter()
        .copied()
        .filter(|id| {
            let mut kind = st.doc.design().object(*id).map(|o| o.kind.clone());
            kind.as_mut().and_then(puff_params).is_some_and(|c| c.puff != on)
        })
        .collect();
    if targets.is_empty() {
        return 0;
    }
    st.doc.edit(if on { "Puff" } else { "No puff" }, |d| {
        for id in &targets {
            let Some(o) = d.object_mut(*id) else { continue };
            if let Some(c) = puff_params(&mut o.kind) {
                c.puff = on;
                if on {
                    c.pattern = ColumnPattern::Satin;
                    c.spacing_mm = emb_model::object::PUFF_SPACING_MM;
                }
            }
            // The letter parts take the new satin settings.
            if let ObjectKind::Lettering(l) = &mut o.kind {
                l.apply_params();
            }
        }
    });
    targets.len()
}

/// Puff on the objects of the selection that can have it, or off if all
/// of them have it. Returns the number of objects changed.
pub fn toggle_puff(st: &mut AppState) -> usize {
    let ids = st.selection.in_sew_order(st.doc.design());
    let all_on = ids.iter().all(|id| {
        let mut kind = st.doc.design().object(*id).map(|o| o.kind.clone());
        kind.as_mut().and_then(puff_params).is_none_or(|c| c.puff)
    });
    set_puff(st, &ids, !all_on)
}

/// The satin settings that puff changes: of a satin column, a line with a
/// border, or the satin parts of lettering.
fn puff_params(k: &mut ObjectKind) -> Option<&mut SatinParams> {
    match k {
        ObjectKind::Lettering(l) => Some(&mut l.satin),
        k => column_params(k),
    }
}

/// Laydown goes this far past the objects, in mm.
pub const LAYDOWN_MARGIN_MM: f64 = 2.0;

/// Adds a laydown fill under the selected objects (or all objects): their
/// areas joined and moved out by [`LAYDOWN_MARGIN_MM`]. It sews first, in a
/// new thread of the fabric color. One undo step.
pub fn add_laydown(st: &mut AppState) -> Option<ObjectId> {
    let d = st.doc.design();
    let ids: Vec<ObjectId> = if st.selection.is_empty() {
        d.objects().iter().map(|o| o.id).collect()
    } else {
        st.selection.in_sew_order(d)
    };
    let kinds: Vec<&ObjectKind> = ids
        .iter()
        .filter_map(|id| d.object(*id))
        .map(|o| &o.kind)
        .filter(
            |k| !matches!(k, ObjectKind::Fill { params, .. } if matches!(params.pattern, FillPattern::Laydown { .. })),
        )
        .collect();
    let rings = emb_stitch::textures::laydown_area(&kinds, LAYDOWN_MARGIN_MM);
    if rings.is_empty() {
        st.message = tr("Nothing to lay down: select fills, columns, lines or lettering.").into();
        return None;
    }
    let mut outer = emb_geom::Path::new();
    for r in &rings {
        outer.cmds.extend(emb_geom::Path::polygon(r).cmds);
    }
    let color = d.background.unwrap_or(emb_model::Rgb(245, 245, 245));
    let params = emb_model::FillParams::default().with_pattern(FillPattern::Laydown {
        layers: 1,
        angle_deg: 90.0,
    });
    let id = st.doc.edit("Laydown", |d| {
        let t = d.add_thread("Laydown", color);
        let id = d.insert_object(
            0,
            t,
            ObjectKind::Fill {
                outer,
                holes: Vec::new(),
                params,
            },
        );
        if let Some(o) = d.object_mut(id) {
            o.name = Some("Laydown".into());
        }
        id
    });
    st.selection.set(id);
    st.message = tr("Laydown added. It sews first; use a thread of the fabric color.").into();
    Some(id)
}

#[cfg(test)]
mod tests {
    use emb_geom::{Path, Point};
    use emb_model::{RunningParams, SatinGeometry};

    use super::*;
    use crate::test_util::Harness;
    use eframe::egui;

    fn line_and_column(h: &mut Harness) -> (ObjectId, ObjectId) {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            let line = d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new().move_to(Point::ZERO).line_to(Point::new(30.0, 0.0)),
                    params: RunningParams::default(),
                },
            );
            let column = d.add_object(
                t,
                ObjectKind::Satin {
                    geometry: SatinGeometry::from_centerline(
                        &Path::new()
                            .move_to(Point::new(0.0, 10.0))
                            .line_to(Point::new(30.0, 10.0)),
                        4.0,
                    ),
                    params: SatinParams::default(),
                },
            );
            (line, column)
        })
    }

    #[test]
    fn outline_and_column_types_are_one_undo_step_each() {
        let mut h = Harness::empty();
        let (line, column) = line_and_column(&mut h);
        let ids = [line, column];
        assert_eq!(set_outline(&mut h.st, &ids, Some(ColumnPattern::Coil)), 1);
        assert_eq!(set_outline(&mut h.st, &ids, Some(ColumnPattern::Coil)), 0, "no change");
        // The line with a border and the satin column both take a column type.
        assert_eq!(set_column(&mut h.st, &ids, ColumnPattern::EStitch), 2);
        let d = h.st.doc.design();
        let Some(ObjectKind::Running { params, .. }) = d.object(line).map(|o| &o.kind) else {
            panic!()
        };
        let b = params.border.as_ref().unwrap();
        assert_eq!(
            (b.width_mm, b.column.pattern),
            (DEFAULT_BORDER_MM, ColumnPattern::EStitch)
        );
        let Some(ObjectKind::Satin { params, .. }) = d.object(column).map(|o| &o.kind) else {
            panic!()
        };
        assert_eq!(params.pattern, ColumnPattern::EStitch);
        assert_eq!(params.spacing_mm, ColumnPattern::EStitch.default_spacing_mm());
        // The stitches change, and undo brings back the coil, then the plain line.
        let plan = |h: &Harness| emb_stitch::stitch_object(h.st.doc.design().object(line).unwrap(), Point::ZERO);
        assert!(plan(&h).unwrap().runs[0].points.len() > 30);
        h.st.doc.undo();
        h.st.doc.undo();
        assert_eq!(plan(&h).unwrap().runs[0].points.len(), 13, "a 30 mm run at 2.5 mm");
    }

    #[test]
    fn fill_types_and_the_florentine_guide() {
        let mut h = Harness::empty();
        let ids = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            let fill = d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(30.0, 20.0))),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            );
            let line = d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new()
                        .move_to(Point::new(0.0, 10.0))
                        .line_to(Point::new(15.0, 4.0))
                        .line_to(Point::new(30.0, 10.0)),
                    params: RunningParams::default(),
                },
            );
            [fill, line]
        });
        assert_eq!(set_fill(&mut h.st, &ids, FillPattern::Spiral), 1);
        let fill_of = |h: &Harness| match h.st.doc.design().object(ids[0]).map(|o| o.kind.clone()) {
            Some(ObjectKind::Fill { params, .. }) => params,
            _ => panic!(),
        };
        assert_eq!(fill_of(&h).pattern, FillPattern::Spiral);
        assert_eq!(fill_of(&h).stitch_length_mm, FillPattern::Spiral.defaults().1);
        h.st.selection.set_all(ids);
        assert!(florentine_guide(&mut h.st));
        assert_eq!(h.st.doc.design().objects().len(), 1, "the line is used up");
        let FillPattern::Florentine { guide } = fill_of(&h).pattern else {
            panic!()
        };
        assert_eq!(guide.len(), 3);
        // Moving the fill moves its guide.
        h.st.nudge_selection(5.0, 0.0);
        let FillPattern::Florentine { guide } = fill_of(&h).pattern else {
            panic!()
        };
        assert_eq!(guide[0], Point::new(5.0, 10.0));
        h.st.doc.undo();
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), 2);
    }

    #[test]
    fn puff_turns_on_and_off_and_makes_dense_satin() {
        let mut h = Harness::empty();
        let (line, column) = line_and_column(&mut h);
        h.st.selection.set_all(vec![line, column]);
        // The plain line cannot have puff; the column can.
        assert_eq!(toggle_puff(&mut h.st), 1);
        let params = |h: &Harness| match h.st.doc.design().object(column).map(|o| o.kind.clone()) {
            Some(ObjectKind::Satin { params, .. }) => params,
            _ => panic!(),
        };
        assert!(params(&h).puff);
        assert_eq!(params(&h).spacing_mm, emb_model::object::PUFF_SPACING_MM);
        assert!(emb_stitch::is_puff(&h.st.doc.design().object(column).unwrap().kind));
        assert_eq!(toggle_puff(&mut h.st), 1);
        assert!(!params(&h).puff);
        h.st.doc.undo();
        assert!(params(&h).puff);
    }

    #[test]
    fn guide_lines_and_the_ripple_guide() {
        let mut h = Harness::empty();
        let line = |d: &mut emb_model::Design, t, a: Point, b: Point| {
            d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new().move_to(a).line_to(b),
                    params: RunningParams::default(),
                },
            )
        };
        let ids = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            let fill = d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(30.0, 20.0))),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            );
            let a = line(d, t, Point::new(0.0, 2.0), Point::new(30.0, 2.0));
            let b = line(d, t, Point::new(0.0, 18.0), Point::new(30.0, 12.0));
            [fill, a, b]
        });
        h.st.selection.set_all(ids);
        assert!(guide_lines(&mut h.st));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1, "the lines are used up");
        let Some(ObjectKind::Fill { params, .. }) = d.object(ids[0]).map(|o| &o.kind) else {
            panic!()
        };
        let FillPattern::Guided { guides } = &params.pattern else {
            panic!("{:?}", params.pattern)
        };
        assert_eq!(guides.len(), 2);
        assert!(emb_stitch::stitch_object(d.object(ids[0]).unwrap(), Point::ZERO).is_ok());
        h.st.doc.undo();
        assert_eq!(h.st.doc.design().objects().len(), 3);
        // Two lines: the second is the guide of a ripple of the first.
        h.st.selection.set_all(vec![ids[1], ids[2]]);
        assert!(ripple_guide(&mut h.st));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 2);
        let Some(ObjectKind::Running { params, .. }) = d.object(ids[1]).map(|o| &o.kind) else {
            panic!()
        };
        let r = params.ripple.as_ref().expect("a ripple");
        assert_eq!((r.guide.len(), r.end_percent), (2, 100.0));
        // A plain line again.
        assert_eq!(set_ripple(&mut h.st, &[ids[1]], None), 1);
        assert_eq!(set_outline(&mut h.st, &[ids[1]], None), 0, "it is a plain line");
    }

    #[test]
    fn split_lines_and_stamps_change_the_needle_points() {
        let mut h = Harness::empty();
        let ids = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            let fill = d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(30.0, 20.0))),
                    holes: Vec::new(),
                    params: emb_model::FillParams {
                        angle_deg: 0.0,
                        ..Default::default()
                    },
                },
            );
            let a = d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new()
                        .move_to(Point::new(10.0, -1.0))
                        .line_to(Point::new(10.0, 21.0)),
                    params: RunningParams::default(),
                },
            );
            let b = d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new()
                        .move_to(Point::new(20.0, -1.0))
                        .line_to(Point::new(20.0, 21.0)),
                    params: RunningParams::default(),
                },
            );
            [fill, a, b]
        });
        h.st.selection.set_all(ids);
        assert!(split_lines(&mut h.st));
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1, "the lines are used up");
        let o = d.object(ids[0]).unwrap();
        let ObjectKind::Fill { params, .. } = &o.kind else {
            panic!()
        };
        assert!(matches!(&params.pattern, FillPattern::UserSplit { lines } if lines.len() == 2));
        let pts: Vec<Point> = emb_stitch::stitch_object(o, Point::ZERO)
            .unwrap()
            .runs
            .into_iter()
            .flat_map(|r| r.points)
            .collect();
        let on = |x: f64| pts.iter().filter(|p| (p.x - x).abs() < 1e-6).count();
        assert!(on(10.0) > 40 && on(20.0) > 40, "{} {}", on(10.0), on(20.0));
        // Moving the fill moves its split lines.
        h.st.nudge_selection(1.0, 0.0);
        let o = h.st.doc.design().object(ids[0]).unwrap();
        let ObjectKind::Fill { params, .. } = &o.kind else {
            panic!()
        };
        let FillPattern::UserSplit { lines } = &params.pattern else {
            panic!()
        };
        assert_eq!(lines[0][0], Point::new(11.0, -1.0));
        assert_eq!(h.st.doc.undo_len(), 3);
        // A stamp in a tatami fill: needle points along the stamp, and
        // Object Properties draws its settings with no edit.
        set_fill(&mut h.st, &[ids[0]], FillPattern::Tatami);
        h.st.doc.edit("Stamp", |d| {
            if let Some(ObjectKind::Fill { params, .. }) = d.object_mut(ids[0]).map(|o| &mut o.kind) {
                params.stamps.push(emb_model::StampPlacement {
                    stamp: emb_model::Stamp::builtin("Circle").unwrap(),
                    center: Point::new(16.0, 10.0),
                    size_mm: 12.0,
                    angle_deg: 0.0,
                });
            }
        });
        let before = h.st.doc.undo_len();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, crate::test_util::SCREEN)),
            ..Default::default()
        };
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| crate::panels::properties(ui, st));
        });
        out.textures_delta.clear();
        assert_eq!(h.st.doc.undo_len(), before);
        let o = h.st.doc.design().object(ids[0]).unwrap();
        let pts: Vec<Point> = emb_stitch::stitch_object(o, Point::ZERO)
            .unwrap()
            .runs
            .into_iter()
            .flat_map(|r| r.points)
            .collect();
        let on_circle = pts
            .iter()
            .filter(|p| (p.distance(Point::new(16.0, 10.0)) - 6.0).abs() < 0.05)
            .count();
        assert!(on_circle > 40, "{on_circle}");
    }

    #[test]
    fn laydown_goes_under_everything_first() {
        let mut h = Harness::empty();
        let (line, column) = line_and_column(&mut h);
        let id = add_laydown(&mut h.st).unwrap();
        let d = h.st.doc.design();
        assert_eq!(d.objects()[0].id, id, "it sews first");
        assert_eq!(d.objects().len(), 3);
        let o = d.object(id).unwrap();
        assert_eq!(o.name.as_deref(), Some("Laydown"));
        assert_ne!(o.thread, d.object(line).unwrap().thread, "a thread of its own");
        let b = o.control_bbox().unwrap();
        // The line (0..30 at y 0) and the 4 mm column (y 10) with a 2 mm margin.
        assert!(
            b.min.x < -1.9 && b.max.x > 31.9 && b.min.y < -2.0 && b.max.y > 13.9,
            "{b:?}"
        );
        let _ = column;
        // A second laydown does not lay down the first one.
        h.st.selection.clear();
        let again = add_laydown(&mut h.st).unwrap();
        let b2 = h.st.doc.design().object(again).unwrap().control_bbox().unwrap();
        assert!((b2.min.x - b.min.x).abs() < 0.2);
        assert_eq!(h.st.doc.undo_len(), 3);
    }

    #[test]
    fn properties_draw_every_type() {
        use eframe::egui::{self, Pos2, Rect};
        let mut h = Harness::empty();
        let (line, column) = line_and_column(&mut h);
        for pattern in ColumnPattern::ALL {
            set_outline(&mut h.st, &[line], Some(pattern));
            set_column(&mut h.st, &[column], pattern);
            for id in [line, column] {
                h.st.selection.set(id);
                let input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, crate::test_util::SCREEN)),
                    ..Default::default()
                };
                let before = h.st.doc.undo_len();
                let st = &mut h.st;
                let mut out = h.ctx.run_ui(input, |ui| {
                    egui::CentralPanel::default().show(ui, |ui| crate::panels::properties(ui, st));
                });
                out.textures_delta.clear();
                assert_eq!(h.st.doc.undo_len(), before, "{pattern:?}: {:?}", h.st.doc.undo_label());
            }
        }
        // Fills: each fill type draws too, with no edit.
        let fill = h.st.doc.edit("Add", |d| {
            let t = d.threads[0].id;
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(emb_geom::BBox::new(Point::ZERO, Point::new(30.0, 20.0))),
                    holes: Vec::new(),
                    params: Default::default(),
                },
            )
        });
        for pattern in FillPattern::all() {
            set_fill(&mut h.st, &[fill], pattern.clone());
            h.st.selection.set(fill);
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, crate::test_util::SCREEN)),
                ..Default::default()
            };
            let before = h.st.doc.undo_len();
            let st = &mut h.st;
            let mut out = h.ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| crate::panels::properties(ui, st));
            });
            out.textures_delta.clear();
            assert_eq!(h.st.doc.undo_len(), before, "{pattern:?}");
        }
        // Each type change is one undo step: 2 objects, each column type, less
        // the satin that was satin already; then the fill and its type changes.
        // Drawing the panel adds none.
        let fill_types = FillPattern::all().len() - 1;
        let columns = ColumnPattern::ALL.len();
        assert_eq!(h.st.doc.undo_len(), 1 + columns + (columns - 1) + 1 + fill_types);
    }
}
