//! View > Stitch Parts (P18 Milestone 1, ADR 0131): the stitches of the
//! design colored by the kind of their part (`emb_art::parts`): runs,
//! columns, rows and other stitches, the underlay of a part paler in its
//! color. It shows how the program reads the stitches of a machine file
//! before it makes objects of them. A thread finds the parts of each new
//! stitch plan.

use std::sync::{Arc, mpsc};

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Shape};
use emb_art::parts::{PartKind, parts, shown_kind};
use emb_model::StitchKind;
use emb_render::{Segment, SegmentKind};

use crate::app::AppState;
use crate::i18n::tr;

/// The kind of the part of each record of a plan, and whether the part is
/// the underlay of another part.
type RecordParts = Vec<Option<(PartKind, bool)>>;

/// The stitches in the colors of their parts (canvas coordinates).
type Colored = Arc<Vec<(Segment, Color32)>>;

/// The view and its parts.
#[derive(Default)]
pub struct PartsView {
    pub on: bool,
    /// The state of the plan, and the colored stitches (canvas coordinates).
    shown: Option<(u64, Colored)>,
    job: Option<(u64, mpsc::Receiver<RecordParts>)>,
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

/// The parts of the records of `s`.
fn record_parts(s: &emb_model::StitchDesign) -> RecordParts {
    let p = parts(s);
    let mut out = vec![None; s.records.len()];
    for (i, part) in p.iter().enumerate() {
        let k = shown_kind(&p, i);
        for r in part.records.clone() {
            out[r] = Some((k, part.underlay_of.is_some()));
        }
    }
    out
}

impl AppState {
    /// The stitches of the plan that shows colored by their parts, when a
    /// thread has found them.
    fn part_segments(&mut self, ctx: &egui::Context) -> Option<Colored> {
        let (state, plan) = {
            let pv = self.shown_plan();
            (pv.state, pv.plan.clone())
        };
        if let Some((s, segs)) = &self.parts_view.shown
            && *s == state
        {
            return Some(segs.clone());
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
        if let Some((s, kinds)) = done {
            self.parts_view.job = None;
            if s == state {
                let back = self.doc.design().from_hoop_transform();
                let recs = &plan.stitches.records;
                let mut segs = Vec::with_capacity(recs.len());
                for (i, r) in recs.iter().enumerate().skip(1) {
                    if r.kind != StitchKind::Stitch {
                        continue;
                    }
                    let Some((k, under)) = kinds[i] else { continue };
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
                let segs = Arc::new(segs);
                self.parts_view.shown = Some((state, segs.clone()));
                return Some(segs);
            }
        }
        if self.parts_view.job.as_ref().is_none_or(|(s, _)| *s != state) {
            let (tx, rx) = mpsc::channel();
            let repaint = ctx.clone();
            let work = move || {
                let _ = tx.send(record_parts(&plan.stitches));
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

/// Draws the stitches colored by their parts over the canvas, and a key.
pub fn draw(painter: &egui::Painter, ctx: &egui::Context, rect: Rect, st: &mut AppState) {
    if !st.parts_view.on {
        return;
    }
    let visuals = ctx.global_style().visuals.clone();
    let Some(segs) = st.part_segments(ctx) else {
        ctx.request_repaint_after(std::time::Duration::from_millis(60));
        return;
    };
    // The canvas a little darker or lighter, so the colors show.
    painter.rect_filled(rect, 0.0, visuals.extreme_bg_color.gamma_multiply(0.75));
    let (mesh, _) = crate::canvas::stitch_mesh(rect, &st.view, &segs, false);
    painter.add(Shape::mesh(mesh));
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
    for (c, text) in items {
        painter.rect_filled(
            Rect::from_min_size(at + egui::vec2(0.0, 3.0), egui::vec2(18.0, 10.0)),
            2.0,
            c,
        );
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
        let segs =
            h.st.parts_view
                .shown
                .as_ref()
                .map(|(_, s)| s.clone())
                .expect("the parts");
        assert!(!segs.is_empty());
        // The sample design has a fill, a satin and a run: their colors show.
        let has = |k: PartKind| segs.iter().any(|(_, c)| *c == color(k, false));
        assert!(has(PartKind::Rows) && has(PartKind::Column) && has(PartKind::Run));
    }
}
