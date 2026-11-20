//! View > Stitch Parts (P18 Milestone 1, ADR 0131): the stitches of the
//! design colored by the kind of their part (`emb_art::parts`): runs,
//! columns, rows and other stitches, the underlay of a part paler in its
//! color. Lines show the objects that the parts become
//! (`emb_art::part_objects`, P18 Milestones 2 and 3, ADR 0132, ADR 0133):
//! the line of a running object, the two rails of a satin column and the
//! outline of a fill. It shows how the
//! program reads the stitches of a machine file before it makes objects of
//! them. A thread finds the parts of each new stitch plan.

use std::sync::{Arc, mpsc};

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Shape, Stroke};
use emb_art::part_objects::objects;
use emb_art::parts::{PartKind, parts, shown_kind};
use emb_geom::Point;
use emb_model::{ObjectKind, StitchKind};
use emb_render::{Segment, SegmentKind};

use crate::app::AppState;
use crate::i18n::tr;

/// The kind of the part of each record of a plan, and whether the part is
/// the underlay of another part.
type RecordParts = Vec<Option<(PartKind, bool)>>;

/// What the thread finds: the parts of the records, and the lines of the
/// objects that the parts become (plan coordinates).
struct Found {
    kinds: RecordParts,
    lines: Vec<Vec<Point>>,
}

/// The stitches in the colors of their parts, and the lines of the objects
/// (canvas coordinates).
#[derive(Clone)]
struct Shown {
    stitches: Arc<Vec<(Segment, Color32)>>,
    lines: Arc<Vec<Vec<Point>>>,
}

/// The view and its parts.
#[derive(Default)]
pub struct PartsView {
    pub on: bool,
    /// The state of the plan, and what shows.
    shown: Option<(u64, Shown)>,
    job: Option<(u64, mpsc::Receiver<Found>)>,
}

/// The color of a kind of part; the underlay is paler.
pub fn color(kind: PartKind, underlay: bool) -> Color32 {
    let c = match kind {
        PartKind::Run => [40, 110, 235],
        PartKind::Column => [240, 140, 20],
        PartKind::Rows => [30, 170, 80],
        PartKind::Other => [210, 50, 200],
    };
    if underlay {
        let pale = |v: u8| ((u16::from(v) + 2 * 235) / 3) as u8;
        Color32::from_rgb(pale(c[0]), pale(c[1]), pale(c[2]))
    } else {
        Color32::from_rgb(c[0], c[1], c[2])
    }
}

/// The parts of the records of `s`, and the lines of the objects that the
/// parts become.
fn find(s: &emb_model::StitchDesign) -> Found {
    let p = parts(s);
    let mut kinds = vec![None; s.records.len()];
    for (i, part) in p.iter().enumerate() {
        let k = shown_kind(&p, i);
        for r in part.records.clone() {
            kinds[r] = Some((k, part.underlay_of.is_some()));
        }
    }
    let mut lines = Vec::new();
    for m in objects(s, &p) {
        let paths: Vec<&emb_geom::Path> = match &m.kind {
            ObjectKind::Running { path, .. } => vec![path],
            ObjectKind::Satin { geometry, .. } => vec![&geometry.rail_a, &geometry.rail_b],
            ObjectKind::Fill { outer, holes, .. } => std::iter::once(outer).chain(holes).collect(),
            _ => Vec::new(),
        };
        for path in paths {
            lines.extend(path.flatten(0.05).into_iter().map(|l| {
                let mut pts = l.points;
                if l.closed
                    && let Some(&first) = pts.first()
                {
                    pts.push(first);
                }
                pts
            }));
        }
    }
    Found { kinds, lines }
}

impl AppState {
    /// The stitches of the plan that shows colored by their parts, when a
    /// thread has found them.
    fn part_segments(&mut self, ctx: &egui::Context) -> Option<Shown> {
        let (state, plan) = {
            let pv = self.shown_plan();
            (pv.state, pv.plan.clone())
        };
        if let Some((s, shown)) = &self.parts_view.shown
            && *s == state
        {
            return Some(shown.clone());
        }
        // A finished job.
        let mut done = None;
        if let Some((s, rx)) = &self.parts_view.job {
            match rx.try_recv() {
                Ok(kinds) => done = Some((*s, kinds)),
                Err(mpsc::TryRecvError::Disconnected) => self.parts_view.job = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some((s, found)) = done {
            self.parts_view.job = None;
            if s == state {
                let back = self.doc.design().from_hoop_transform();
                let recs = &plan.stitches.records;
                let mut segs = Vec::with_capacity(recs.len());
                for (i, r) in recs.iter().enumerate().skip(1) {
                    if r.kind != StitchKind::Stitch {
                        continue;
                    }
                    let Some((k, under)) = found.kinds[i] else { continue };
                    segs.push((
                        Segment {
                            a: back.apply(recs[i - 1].pos),
                            b: back.apply(r.pos),
                            kind: SegmentKind::Stitch,
                            ..Default::default()
                        },
                        color(k, under),
                    ));
                }
                let lines = found
                    .lines
                    .iter()
                    .map(|l| l.iter().map(|p| back.apply(*p)).collect())
                    .collect();
                let shown = Shown {
                    stitches: Arc::new(segs),
                    lines: Arc::new(lines),
                };
                self.parts_view.shown = Some((state, shown.clone()));
                return Some(shown);
            }
        }
        if self.parts_view.job.as_ref().is_none_or(|(s, _)| *s != state) {
            let (tx, rx) = mpsc::channel();
            let repaint = ctx.clone();
            let work = move || {
                let _ = tx.send(find(&plan.stitches));
                repaint.request_repaint();
            };
            if self.stitch_tiles.sync {
                work();
            } else if std::thread::Builder::new()
                .name("stitch-parts".into())
                .stack_size(crate::resources::JOB_STACK)
                .spawn(work)
                .is_err()
            {
                return None;
            }
            self.parts_view.job = Some((state, rx));
            if self.stitch_tiles.sync {
                return self.part_segments(ctx);
            }
        }
        // The parts before show until the new ones are there.
        self.parts_view.shown.as_ref().map(|(_, s)| s.clone())
    }
}

/// The width of the lines of the objects, screen points.
const LINE_PX: f32 = 1.5;

/// Draws the stitches colored by their parts over the canvas, the lines of
/// the objects, and a key.
pub fn draw(painter: &egui::Painter, ctx: &egui::Context, rect: Rect, st: &mut AppState) {
    if !st.parts_view.on {
        return;
    }
    let visuals = ctx.global_style().visuals.clone();
    let Some(shown) = st.part_segments(ctx) else {
        ctx.request_repaint_after(std::time::Duration::from_millis(60));
        return;
    };
    // The canvas a little darker or lighter, so the colors show.
    painter.rect_filled(rect, 0.0, visuals.extreme_bg_color.gamma_multiply(0.75));
    let (mesh, _) = crate::canvas::stitch_mesh(rect, &st.view, &shown.stitches, false);
    painter.add(Shape::mesh(mesh));
    // The lines of the objects that the parts become.
    let line_color = visuals.text_color();
    let near = rect.expand(50.0);
    for line in shown.lines.iter() {
        let pts: Vec<Pos2> = line.iter().map(|p| st.view.to_screen(rect, *p)).collect();
        if pts.iter().any(|p| near.contains(*p)) {
            painter.add(Shape::line(pts, Stroke::new(LINE_PX, line_color)));
        }
    }
    // The key, at the top left of the canvas.
    let left = if st.settings.show_rulers {
        crate::workspace::RULER
    } else {
        0.0
    };
    let mut at = Pos2::new(rect.left() + left + 8.0, rect.top() + left + 8.0);
    let font = FontId::proportional(13.0);
    let mut items: Vec<(Color32, String)> = PartKind::ALL
        .iter()
        .map(|k| (color(*k, false), tr(k.label()).to_string()))
        .collect();
    items.push((color(PartKind::Run, true), tr("Underlay (pale)").to_string()));
    // The last item is the line of the objects.
    items.push((line_color, tr("Lines of the objects").to_string()));
    let lines_item = items.len() - 1;
    let width = items
        .iter()
        .map(|(_, t)| {
            painter
                .layout_no_wrap(t.clone(), font.clone(), visuals.text_color())
                .size()
                .x
        })
        .fold(0.0, f32::max)
        + 28.0;
    let back = Rect::from_min_size(at, egui::vec2(width, 20.0 * items.len() as f32 + 6.0)).expand(4.0);
    painter.rect_filled(back, 4.0, visuals.extreme_bg_color.gamma_multiply(0.92));
    at.y += 3.0;
    for (k, (c, text)) in items.into_iter().enumerate() {
        if k == lines_item {
            painter.line_segment(
                [at + egui::vec2(0.0, 8.0), at + egui::vec2(18.0, 8.0)],
                Stroke::new(LINE_PX, c),
            );
        } else {
            painter.rect_filled(
                Rect::from_min_size(at + egui::vec2(0.0, 3.0), egui::vec2(18.0, 10.0)),
                2.0,
                c,
            );
        }
        painter.text(
            at + egui::vec2(26.0, 8.0),
            egui::Align2::LEFT_CENTER,
            text,
            font.clone(),
            visuals.text_color(),
        );
        at.y += 20.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn the_view_colors_the_stitches_by_their_parts() {
        let mut h = Harness::new();
        h.st.stitch_tiles.sync = true;
        h.st.parts_view.on = true;
        h.frame(Vec::new());
        h.frame(Vec::new());
        let shown =
            h.st.parts_view
                .shown
                .as_ref()
                .map(|(_, s)| s.clone())
                .expect("the parts");
        let segs = &shown.stitches;
        assert!(!segs.is_empty());
        // The sample design has a fill, a satin and a run: their colors show.
        let has = |k: PartKind| segs.iter().any(|(_, c)| *c == color(k, false));
        assert!(has(PartKind::Rows) && has(PartKind::Column) && has(PartKind::Run));
        // The fill, the satin and the run become objects: their lines show
        // (an outline, two rails and a line at least).
        assert!(shown.lines.len() >= 4, "{}", shown.lines.len());
    }
}
