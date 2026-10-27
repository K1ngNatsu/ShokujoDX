//! Join (Ctrl+J): puts back together objects that Break apart or the knife
//! separated (owner request, 2026-10-23).
//!
//! The selected objects are joined in groups of the same kind and the same
//! thread. Each group becomes one object in the place of its first object:
//!
//! - Lines become one line. Parts whose ends touch become one continuous
//!   part (a part whose ends then touch is closed).
//! - Fills become one fill: the union of their areas, so parts cut by the
//!   knife are one area again.
//! - Satin columns whose ends touch become one column; columns that do not
//!   touch another column stay apart.
//! - Stitch objects become one object, with a jump between the parts.
//!
//! Appliqué and lettering are not joined. Each join is one undo step.

use eframe::egui::{Key, KeyboardShortcut, Modifiers};
use emb_geom::nodes::{NodePath, SubNodes};
use emb_geom::region::{self, Region};
use emb_geom::{Path, Point};
use emb_model::{DesignObject, ObjectId, ObjectKind, SatinGeometry, ThreadId};

use crate::app::AppState;
use crate::i18n::tr;

pub const JOIN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::J);

/// Ends nearer than this touch, in mm.
const TOUCH_MM: f64 = 0.05;

fn touch(a: Point, b: Point) -> bool {
    a.distance(b) <= TOUCH_MM
}

/// `a` then `b`; the first node of `b` is the last node of `a`.
fn append(a: &SubNodes, b: &SubNodes) -> SubNodes {
    let mut nodes = a.nodes.clone();
    nodes.extend(b.nodes.iter().skip(1));
    let mut segs = a.segs.clone();
    segs.extend(b.segs.iter().copied());
    SubNodes {
        nodes,
        segs,
        closed: false,
    }
}

/// `a` and `b` as one part if an end of `a` touches an end of `b`.
fn chain_two(a: &SubNodes, b: &SubNodes) -> Option<SubNodes> {
    if a.closed || b.closed || a.nodes.len() < 2 || b.nodes.len() < 2 {
        return None;
    }
    let (a0, a1) = (a.nodes[0], a.nodes[a.nodes.len() - 1]);
    let (b0, b1) = (b.nodes[0], b.nodes[b.nodes.len() - 1]);
    if touch(a1, b0) {
        Some(append(a, b))
    } else if touch(a1, b1) {
        Some(append(a, &b.reversed()))
    } else if touch(a0, b1) {
        Some(append(b, a))
    } else if touch(a0, b0) {
        Some(append(&b.reversed(), a))
    } else {
        None
    }
}

/// Joins the parts whose ends touch, and closes a part whose ends touch.
fn chain(mut subs: Vec<SubNodes>) -> Vec<SubNodes> {
    'outer: loop {
        for i in 0..subs.len() {
            for j in i + 1..subs.len() {
                if let Some(joined) = chain_two(&subs[i], &subs[j]) {
                    subs.remove(j);
                    subs[i] = joined;
                    continue 'outer;
                }
            }
        }
        break;
    }
    for s in &mut subs {
        if !s.closed && s.nodes.len() > 2 && touch(s.nodes[0], s.nodes[s.nodes.len() - 1]) {
            s.nodes.pop();
            s.closed = true;
        }
    }
    subs
}

fn join_lines(paths: &[&Path]) -> Path {
    let subs: Vec<SubNodes> = paths.iter().flat_map(|p| NodePath::from_path(p).subs).collect();
    NodePath { subs: chain(subs) }.to_path()
}

fn join_fills(kinds: &[&ObjectKind]) -> Option<ObjectKind> {
    let mut regions: Vec<Region> = Vec::new();
    let mut params = None;
    for k in kinds {
        if let ObjectKind::Fill {
            outer,
            holes,
            params: p,
        } = k
        {
            params.get_or_insert_with(|| p.clone());
            regions = region::union(&regions, &crate::actions::fill_regions(outer, holes));
        }
    }
    let mut outer = Path::new();
    let mut holes = Vec::new();
    for r in &regions {
        outer.cmds.extend(Path::polygon(&r.outer).cmds);
        holes.extend(r.holes.iter().map(|h| Path::polygon(h)));
    }
    Some(ObjectKind::Fill {
        outer,
        holes,
        params: params?,
    })
}

/// The rails of a column as single parts (`None` if a rail has more parts).
fn rails(g: &SatinGeometry) -> Option<(SubNodes, SubNodes)> {
    let one = |p: &Path| {
        let mut n = NodePath::from_path(p);
        (n.subs.len() == 1 && !n.subs[0].closed && n.subs[0].nodes.len() >= 2).then(|| n.subs.remove(0))
    };
    Some((one(&g.rail_a)?, one(&g.rail_b)?))
}

fn last(s: &SubNodes) -> Point {
    s.nodes[s.nodes.len() - 1]
}

/// `a` then `b` as one column if the end of `a` touches an end of `b`
/// (either way round, with either rail on either side), or `b` then `a`.
fn chain_columns(a: &SatinGeometry, b: &SatinGeometry) -> Option<SatinGeometry> {
    let (aa, ab) = rails(a)?;
    let (ba, bb) = rails(b)?;
    for (first, second) in [((&aa, &ab), (&ba, &bb)), ((&ba, &bb), (&aa, &ab))] {
        for flip in [false, true] {
            for swap in [false, true] {
                let (mut x, mut y) = (second.0.clone(), second.1.clone());
                if flip {
                    x = x.reversed();
                    y = y.reversed();
                }
                if swap {
                    std::mem::swap(&mut x, &mut y);
                }
                if touch(last(first.0), x.nodes[0]) && touch(last(first.1), y.nodes[0]) {
                    let mut rungs = a.rungs.clone();
                    rungs.extend(b.rungs.iter().copied());
                    // The joint keeps the stitch direction it had.
                    rungs.push((last(first.0), last(first.1)));
                    let rail = |s: SubNodes| NodePath { subs: vec![s] }.to_path();
                    return Some(SatinGeometry {
                        rail_a: rail(append(first.0, &x)),
                        rail_b: rail(append(first.1, &y)),
                        rungs,
                    });
                }
            }
        }
    }
    None
}

/// Columns whose ends touch become one column. The others stay as they are.
fn join_columns(kinds: &[&ObjectKind]) -> Vec<ObjectKind> {
    let mut cols: Vec<(SatinGeometry, emb_model::SatinParams)> = kinds
        .iter()
        .filter_map(|k| match k {
            ObjectKind::Satin { geometry, params } => Some((geometry.clone(), params.clone())),
            _ => None,
        })
        .collect();
    'outer: loop {
        for i in 0..cols.len() {
            for j in i + 1..cols.len() {
                if let Some(g) = chain_columns(&cols[i].0, &cols[j].0) {
                    cols.remove(j);
                    cols[i].0 = g;
                    continue 'outer;
                }
            }
        }
        break;
    }
    cols.into_iter()
        .map(|(geometry, params)| ObjectKind::Satin { geometry, params })
        .collect()
}

fn join_stitches(kinds: &[&ObjectKind]) -> ObjectKind {
    let mut path = Path::new();
    let mut all_trims = Vec::new();
    let mut parts = 0;
    for k in kinds {
        if let ObjectKind::Stitches { path: p, trims } = k {
            all_trims.extend(trims.iter().map(|t| t + parts));
            parts += emb_stitch::stitch_subpaths(p).len();
            path.cmds.extend(p.cmds.iter().copied());
        }
    }
    ObjectKind::Stitches { path, trims: all_trims }
}

/// The kinds that join, as a key.
fn class(k: &ObjectKind) -> Option<u8> {
    match k {
        ObjectKind::Running { .. } => Some(0),
        ObjectKind::Fill { .. } => Some(1),
        ObjectKind::Satin { .. } => Some(2),
        ObjectKind::Stitches { .. } => Some(3),
        ObjectKind::Applique { .. }
        | ObjectKind::Lettering(_)
        | ObjectKind::Vector { .. }
        | ObjectKind::CrossStitch(_)
        | ObjectKind::Bling(_)
        | ObjectKind::Sketch(_) => None,
    }
}

impl AppState {
    /// Joins the selected objects (see the module comment). One undo step.
    pub fn join_selection(&mut self) {
        if self.refuse_linked(&self.sel()) {
            return;
        }
        let design = self.doc.design();
        let objs: Vec<DesignObject> = self
            .sel()
            .into_iter()
            .filter_map(|id| design.object(id).cloned())
            .collect();
        let mut groups: Vec<((u8, ThreadId), Vec<&DesignObject>)> = Vec::new();
        for o in &objs {
            let Some(c) = class(&o.kind) else { continue };
            match groups.iter_mut().find(|(k, _)| *k == (c, o.thread)) {
                Some((_, v)) => v.push(o),
                None => groups.push(((c, o.thread), vec![o])),
            }
        }
        // Each group: the objects it replaces and the new kinds.
        let mut plan: Vec<(Vec<ObjectId>, Vec<ObjectKind>)> = Vec::new();
        for ((c, _), group) in &groups {
            if group.len() < 2 {
                continue;
            }
            let kinds: Vec<&ObjectKind> = group.iter().map(|o| &o.kind).collect();
            let new = match c {
                0 => {
                    let paths: Vec<&Path> = kinds
                        .iter()
                        .filter_map(|k| match k {
                            ObjectKind::Running { path, .. } => Some(path),
                            _ => None,
                        })
                        .collect();
                    let ObjectKind::Running { params, .. } = kinds[0] else {
                        continue;
                    };
                    vec![ObjectKind::Running {
                        path: join_lines(&paths),
                        params: params.clone(),
                    }]
                }
                1 => join_fills(&kinds).into_iter().collect(),
                2 => join_columns(&kinds),
                _ => vec![join_stitches(&kinds)],
            };
            if !new.is_empty() && new.len() < group.len() {
                plan.push((group.iter().map(|o| o.id).collect(), new));
            }
        }
        if plan.is_empty() {
            self.message = tr(
                "Nothing to join. Select two or more lines, fills, satin columns (with touching \
                            ends) or stitch objects of the same color.",
            )
            .into();
            return;
        }
        let joined: usize = plan.iter().map(|(ids, new)| ids.len() - new.len()).sum();
        let selected = self.doc.edit("Join", |d| {
            let mut selected = Vec::new();
            for (ids, new) in plan {
                for id in &ids[1..] {
                    d.remove_object(*id);
                }
                selected.extend(crate::actions::replace_with(d, ids[0], new));
            }
            selected
        });
        self.selection.set_all(selected);
        self.message = crate::i18n::fill("Joined: {} fewer objects.", &[&joined]);
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::BBox;
    use emb_model::{FillParams, RunningParams, SatinParams};

    use super::*;
    use crate::test_util::Harness;

    fn add(h: &mut Harness, kinds: Vec<ObjectKind>) -> Vec<ObjectId> {
        h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            kinds.into_iter().map(|k| d.add_object(t, k)).collect()
        })
    }

    fn line(pts: &[Point]) -> ObjectKind {
        ObjectKind::Running {
            path: crate::actions::path_of(pts, false),
            params: RunningParams::default(),
        }
    }

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn knife_then_join_gives_back_one_fill() {
        let mut h = Harness::empty();
        let fill = ObjectKind::Fill {
            outer: Path::rect(BBox::new(p(0.0, 0.0), p(40.0, 20.0))),
            holes: Vec::new(),
            params: FillParams::default(),
        };
        add(&mut h, vec![fill]);
        h.st.select_all();
        h.st.knife(p(20.0, -5.0), p(20.0, 25.0));
        assert_eq!(h.st.doc.design().objects().len(), 2);
        h.st.select_all();
        h.st.join_selection();
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let ObjectKind::Fill { outer, holes, .. } = &d.objects()[0].kind else {
            panic!()
        };
        assert!(holes.is_empty());
        let regions = crate::actions::fill_regions(outer, holes);
        assert_eq!(regions.len(), 1, "one area again");
        assert!((regions[0].area() - 800.0).abs() < 1e-6);
        assert_eq!(h.st.doc.undo_label(), Some("Join"));
    }

    #[test]
    fn lines_that_touch_become_one_part() {
        let mut h = Harness::empty();
        // Three pieces of a square: the second is reversed, the last closes it.
        add(
            &mut h,
            vec![
                line(&[p(0.0, 0.0), p(10.0, 0.0)]),
                line(&[p(10.0, 10.0), p(10.0, 0.0)]),
                line(&[p(10.0, 10.0), p(0.0, 10.0), p(0.0, 0.0)]),
                line(&[p(50.0, 0.0), p(60.0, 0.0)]),
            ],
        );
        h.st.select_all();
        h.st.join_selection();
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let ObjectKind::Running { path, .. } = &d.objects()[0].kind else {
            panic!()
        };
        let n = NodePath::from_path(path);
        assert_eq!(n.subs.len(), 2, "the square and the line that does not touch");
        assert!(n.subs[0].closed);
        assert_eq!(n.subs[0].nodes.len(), 4);
    }

    #[test]
    fn knife_then_join_gives_back_one_column() {
        let mut h = Harness::empty();
        let col = ObjectKind::Satin {
            geometry: SatinGeometry::from_centerline(
                &crate::actions::path_of(&[p(0.0, 0.0), p(40.0, 0.0)], false),
                4.0,
            ),
            params: SatinParams::default(),
        };
        add(&mut h, vec![col]);
        h.st.select_all();
        h.st.knife(p(20.0, -5.0), p(20.0, 5.0));
        assert_eq!(h.st.doc.design().objects().len(), 2);
        h.st.select_all();
        h.st.join_selection();
        let d = h.st.doc.design();
        assert_eq!(d.objects().len(), 1);
        let ObjectKind::Satin { geometry, .. } = &d.objects()[0].kind else {
            panic!()
        };
        let a: Vec<Point> = geometry.rail_a.points().collect();
        assert!(touch(a[0], p(0.0, -2.0)) || touch(a[0], p(40.0, -2.0)), "{a:?}");
        let len: f64 = geometry.rail_a.flatten(0.05).iter().map(|l| l.length()).sum();
        assert!((len - 40.0).abs() < 1e-6, "{len}");
    }

    #[test]
    fn different_colors_and_lettering_stay_apart() {
        let mut h = Harness::empty();
        let ids = add(
            &mut h,
            vec![line(&[p(0.0, 0.0), p(10.0, 0.0)]), line(&[p(10.0, 0.0), p(20.0, 0.0)])],
        );
        h.st.doc.edit("Color", |d| {
            let t = d.add_thread("Blue", emb_model::Rgb(0, 0, 200));
            d.object_mut(ids[1]).unwrap().thread = t;
        });
        h.st.select_all();
        let before = h.st.doc.undo_len();
        h.st.join_selection();
        assert_eq!(h.st.doc.undo_len(), before);
        assert!(h.st.message.starts_with("Nothing to join"));
    }
}
