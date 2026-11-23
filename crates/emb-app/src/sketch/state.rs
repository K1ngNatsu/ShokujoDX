//! The start and the end of sketch mode, the change of the open sketch
//! with its undo step (`AppState::edit_sketch`), the tools, sketches from
//! vectors and from drawings, and the status line.

use super::*;

impl AppState {
    /// The sketch object that is open in sketch mode.
    pub fn sketch_obj(&self) -> Option<&SketchObject> {
        let e = self.sketch.as_ref()?;
        match &self.doc.design().object(e.object)?.kind {
            ObjectKind::Sketch(s) => Some(s),
            _ => None,
        }
    }

    /// Start Sketch: opens the selected sketch, else makes a new one.
    pub fn start_sketch(&mut self) {
        if self.sketch.is_some() {
            return;
        }
        let d = self.doc.design();
        let selected = self
            .selection
            .single()
            .filter(|id| matches!(d.object(*id).map(|o| &o.kind), Some(ObjectKind::Sketch(_))));
        let id = match selected {
            Some(id) => id,
            None => {
                let thread = self.current_thread_id();
                self.doc.edit(tr("Start Sketch"), |d| {
                    let t = thread.unwrap_or_else(|| d.add_thread("Sketch", emb_model::Rgb(90, 90, 90)));
                    let id = d.add_object(
                        t,
                        ObjectKind::Sketch(Box::new(SketchObject::new(Sketch::with_origin()))),
                    );
                    if let Some(o) = d.object_mut(id) {
                        o.name = Some("Sketch".into());
                    }
                    id
                })
            }
        };
        self.open_sketch(id);
    }

    /// Opens sketch object `id` in sketch mode.
    pub fn open_sketch(&mut self, id: ObjectId) {
        let tab = self.settings.ribbon_tab;
        let mut e = SketchEdit::new(
            id,
            if tab == RibbonTab::Sketch {
                RibbonTab::Digitize
            } else {
                tab
            },
        );
        e.values = EditValues::for_units(self.settings.units);
        self.sketch = Some(e);
        self.set_tool(crate::draw::Tool::Select);
        self.selection.set(id);
        let mut s = self.settings.clone();
        s.ribbon_tab = RibbonTab::Sketch;
        s.ribbon_collapsed = false;
        self.set_settings(s);
        self.message = tr("Sketch mode: draw, then add constraints and dimensions. Finish Sketch goes back.").into();
    }

    /// Finish Sketch: back to the other tabs. An empty sketch goes away.
    pub fn finish_sketch(&mut self) {
        let Some(e) = self.sketch.take() else { return };
        self.doc.end_merge();
        let empty = self.sketch_of(e.object).is_some_and(|s| s.sketch().is_empty());
        if empty {
            self.doc.edit(tr("Remove Empty Sketch"), |d| {
                d.remove_object(e.object);
            });
            self.selection.clear();
        } else {
            self.selection.set(e.object);
        }
        let mut s = self.settings.clone();
        s.ribbon_tab = e.tab_before;
        self.set_settings(s);
    }

    pub(super) fn sketch_of(&self, id: ObjectId) -> Option<&SketchObject> {
        match &self.doc.design().object(id)?.kind {
            ObjectKind::Sketch(s) => Some(s),
            _ => None,
        }
    }

    /// Changes the open sketch and solves it: one undo step (with `merge`,
    /// the changes of one drag are one step).
    pub fn edit_sketch<R>(
        &mut self,
        label: &str,
        merge: Option<String>,
        f: impl FnOnce(&mut Sketch) -> R,
    ) -> Option<R> {
        let id = self.sketch.as_ref()?.object;
        self.sketch_of(id)?;
        let mut out = None;
        let mut lost = 0;
        let run = |d: &mut emb_model::Design| {
            if let Some(o) = d.object_mut(id)
                && let ObjectKind::Sketch(s) = &mut o.kind
            {
                out = Some(s.edit(|sk| {
                    let r = f(sk);
                    sk.solve();
                    r
                }));
            }
            // The objects made from the sketch follow it (ADR 0076).
            lost = d.refresh_sketch_links(id);
        };
        match merge {
            Some(k) => self.doc.edit_merged(label, emb_model::MergeKey::new(k), run),
            None => self.doc.edit(label, run),
        }
        self.warn_lost(lost);
        out
    }

    /// The message after a change of the sketch that took the region or
    /// the curve of `n` made objects away.
    pub(super) fn warn_lost(&mut self, n: usize) {
        if n > 0 {
            self.message = fill(
                "The region or curve of {} made objects is gone. They stay as they are; Unlink ends their link.",
                &[&n],
            );
        }
    }

    /// The analysis of the open sketch (made again when it changes).
    pub fn sketch_analysis(&mut self) -> Option<Analysis> {
        let state = self.doc.state_id();
        if let Some((s, a)) = self.sketch.as_ref().and_then(|e| e.analysis.clone())
            && s == state
        {
            return Some(a);
        }
        let a = self.sketch_obj()?.sketch().analyze();
        if let Some(e) = &mut self.sketch {
            e.analysis = Some((state, a.clone()));
        }
        Some(a)
    }

    /// Leaves sketch mode if the open sketch is gone (for example after an
    /// undo of Start Sketch).
    pub fn check_sketch(&mut self) {
        if let Some(e) = &self.sketch
            && self.sketch_of(e.object).is_none()
        {
            let tab = e.tab_before;
            self.sketch = None;
            let mut s = self.settings.clone();
            s.ribbon_tab = tab;
            self.set_settings(s);
        }
    }

    /// Chooses a tool of sketch mode.
    pub fn set_sketch_tool(&mut self, tool: SketchTool) {
        let sel = self.sketch.as_ref().map(|e| e.selection.clone()).unwrap_or_default();
        if let SketchTool::Constrain(kind) = tool
            && !sel.is_empty()
            && self.apply_constraint(kind, &sel)
        {
            return;
        }
        let curves: Vec<CurveId> = sel
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .collect();
        match tool {
            // Offset of the selected curves at once.
            SketchTool::Offset if !curves.is_empty() => {
                self.offset_curves(&curves);
                return;
            }
            SketchTool::Mirror | SketchTool::CircularPattern if curves.is_empty() => {
                self.message = tr("Select the curves to copy first.").into();
                return;
            }
            t if t.is_transform() && sel.iter().all(|p| matches!(p, Pick::Constraint(_))) => {
                self.message = tr("Select the parts first.").into();
                return;
            }
            _ => {}
        }
        if let Some(e) = &mut self.sketch {
            e.tool = tool;
            e.clicks.clear();
            e.picks.clear();
            if matches!(tool, SketchTool::Mirror | SketchTool::CircularPattern) || tool.is_transform() {
                e.source = curves;
                e.source_points = sel
                    .iter()
                    .filter_map(|p| match p {
                        Pick::Point(p) => Some(*p),
                        _ => None,
                    })
                    .collect();
            }
        }
    }

    /// Esc in sketch mode: ends the command and goes back to Select; in
    /// Select, clears the selection.
    pub fn sketch_escape(&mut self) {
        self.sketch_clear_work();
        let Some(e) = &mut self.sketch else { return };
        if e.tool != SketchTool::Select {
            e.tool = SketchTool::Select;
        } else {
            e.selection.clear();
            e.picked.clear();
        }
    }

    /// Stops the work in progress of the tool (clicks, picks, a chain, a
    /// box, a dimension that waits for its place); the tool stays.
    pub fn sketch_clear_work(&mut self) {
        let Some(e) = &mut self.sketch else { return };
        e.clicks.clear();
        e.picks.clear();
        e.chain_start = None;
        e.source.clear();
        e.boxsel = None;
        e.pending_dim = None;
        e.pending_locked = false;
        e.arc_drag = None;
        e.source_points.clear();
    }

    /// Vector to Sketch (ADR 0079): the selected vector objects become one
    /// sketch in their place, with the constraints that the program finds.
    /// One undo step.
    pub fn vectors_to_sketch(&mut self) {
        let d = self.doc.design();
        let vectors: Vec<(ObjectId, emb_geom::Path)> = self
            .selection
            .ids()
            .iter()
            .filter_map(|id| match &d.object(*id)?.kind {
                ObjectKind::Vector { path, .. } => Some((*id, path.clone())),
                _ => None,
            })
            .collect();
        if vectors.is_empty() {
            self.message = tr("Select vector objects first.").into();
            return;
        }
        let paths: Vec<emb_geom::Path> = vectors.iter().map(|(_, p)| p.clone()).collect();
        let sketch = emb_sketch::import::from_paths(&paths, emb_sketch::import::TOL);
        let (curves, constraints) = (sketch.curves.len(), sketch.constraints.len());
        let ids: Vec<ObjectId> = vectors.iter().map(|(id, _)| *id).collect();
        let id = self.doc.edit(tr("Vector to Sketch"), |d| {
            let at = ids.iter().filter_map(|id| d.index_of(*id)).min().unwrap_or(0);
            let thread = d.object(ids[0]).map(|o| o.thread);
            for id in &ids {
                d.remove_object(*id);
            }
            let thread = thread.unwrap_or_else(|| d.add_thread("Sketch", emb_model::Rgb(90, 90, 90)));
            let new = d.add_object(thread, ObjectKind::Sketch(Box::new(SketchObject::new(sketch))));
            d.move_object(new, at);
            new
        });
        self.selection.set(id);
        self.message = fill(
            "A sketch with {} curves and {} constraints that the program found.",
            &[&curves, &constraints],
        );
    }

    /// Opens a DXF drawing as a new sketch, centered in the hoop (ADR 0079).
    pub fn import_dxf_bytes(&mut self, bytes: &[u8], source: &str) {
        let text = String::from_utf8_lossy(bytes);
        let sketch = match emb_sketch::import::from_dxf(&text) {
            Ok(s) => s,
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "Cannot import the drawing".into(),
                    lines: vec![source.to_string(), e.to_string()],
                });
                return;
            }
        };
        let mut obj = SketchObject::new(sketch);
        let target = self.doc.design().hoop.center;
        let bbox = emb_geom::BBox::from_points(obj.paths().flat_map(|p| p.flatten(0.05)).flat_map(|pl| pl.points));
        if let Some(b) = bbox {
            let shift = target - b.center();
            obj.transform(&emb_geom::Transform::translate(shift.x, shift.y));
        }
        let (curves, constraints) = (obj.sketch().curves.len(), obj.sketch().constraints.len());
        let thread = self.current_thread_id();
        let id = self.doc.edit(tr("Import DXF"), |d| {
            let t = thread.unwrap_or_else(|| d.add_thread("Sketch", emb_model::Rgb(90, 90, 90)));
            d.add_object(t, ObjectKind::Sketch(Box::new(obj)))
        });
        self.selection.set(id);
        self.fit_requested = true;
        self.message = fill(
            "A sketch with {} curves and {} constraints that the program found.",
            &[&curves, &constraints],
        );
    }

    /// The text for the status line in sketch mode.
    pub fn sketch_status(&mut self) -> Option<(String, String)> {
        let tool = self.sketch.as_ref()?.tool;
        let clicks = self.sketch.as_ref()?.clicks.len();
        let picks_line = {
            let e = self.sketch.as_ref()?;
            matches!(e.picks.as_slice(), [Pick::Curve(c)] if self.sketch_obj().is_some_and(|o| is_line(o.sketch(), *c)))
        };
        let prompt = match tool {
            SketchTool::Select => {
                tr("Click a part to select it, drag it to move it, double-click a dimension to change it.")
            }
            SketchTool::Spline if clicks == 0 => tr("Click the first point of the spline."),
            SketchTool::Spline => tr(
                "Click the next point. Enter, a right click or a click on the last point ends the spline; click the first point to close it.",
            ),
            SketchTool::Line if clicks == 0 => tr("Click the start of the line."),
            SketchTool::Line => tr(
                "Click the next point (drag from the end for a tangent arc). Click the first point to close; Enter or Esc to end.",
            ),
            SketchTool::Rectangle if clicks == 0 => tr("Click a corner."),
            SketchTool::Rectangle => tr("Click the opposite corner."),
            SketchTool::RectCenter if clicks == 0 => tr("Click the center of the rectangle."),
            SketchTool::RectCenter => tr("Click a corner."),
            SketchTool::Rect3 => match clicks {
                0 => tr("Click the first corner of a side."),
                1 => tr("Click the second corner of the side."),
                _ => tr("Click to set the height."),
            },
            SketchTool::Slot => {
                let kind = self.sketch.as_ref().map_or(SlotKind::CenterToCenter, |e| e.slot_kind);
                match (kind, clicks) {
                    (SlotKind::Overall, 0) => tr("Click the first end of the slot."),
                    (SlotKind::Overall, 1) => tr("Click the second end."),
                    (SlotKind::CenterPoint, 0) => tr("Click the middle of the slot."),
                    (SlotKind::CenterPoint, 1) => tr("Click the center of an end."),
                    (SlotKind::Arc3, 0) => tr("Click the first center of the slot."),
                    (SlotKind::Arc3, 1) => tr("Click the second center."),
                    (SlotKind::Arc3, 2) => tr("Click a point on the arc of the slot."),
                    (SlotKind::ArcCenter, 0) => tr("Click the center of the arc of the slot."),
                    (SlotKind::ArcCenter, 1) => tr("Click the first center of the slot."),
                    (SlotKind::ArcCenter, 2) => tr("Click the second center (counterclockwise)."),
                    (_, 0) => tr("Click the first center of the slot."),
                    (_, 1) => tr("Click the second center."),
                    _ => tr("Click to set the width."),
                }
            }
            SketchTool::RectCenter3 => match clicks {
                0 => tr("Click the center of the rectangle."),
                1 => tr("Click the middle of a side."),
                _ => tr("Click a corner."),
            },
            SketchTool::TangentCircle => tr("Click three lines."),
            SketchTool::Regions => tr(
                "Click inside closed regions to pick them (again to unpick), then make embroidery in the Properties panel or with a right click.",
            ),
            SketchTool::Stretch => {
                tr("Drag a box round the points to stretch, then click a base point and where it goes.")
            }
            SketchTool::Circle if clicks == 0 => tr("Click the center."),
            SketchTool::Circle => tr("Click a point on the circle."),
            SketchTool::Arc3 => match clicks {
                0 => tr("Click the start of the arc."),
                1 => tr("Click the end of the arc."),
                _ => tr("Click a point on the arc."),
            },
            SketchTool::ArcCenter => match clicks {
                0 => tr("Click the center."),
                1 => tr("Click the start (the arc goes counterclockwise)."),
                _ => tr("Click the end."),
            },
            SketchTool::ArcTangent if clicks == 0 => tr("Click the end of a line."),
            SketchTool::ArcTangent => tr("Click the end of the arc."),
            SketchTool::Polygon if clicks == 0 => tr("Click the center."),
            SketchTool::Polygon => tr("Click a corner."),
            SketchTool::Point => tr("Click to add a point."),
            SketchTool::Dimension if self.sketch.as_ref().is_some_and(|e| e.pending_dim.is_some()) => {
                tr("Click to place the dimension (Esc cancels).")
            }
            SketchTool::Dimension if picks_line => tr(
                "Click another line (angle), a point (distance to the line), or an empty place (length of the line).",
            ),
            SketchTool::Dimension => tr(
                "Click a line, a circle or an arc (size), two points (distance), or a point and a line (distance to the line).",
            ),
            SketchTool::Constrain(k) => k.prompt(),
            SketchTool::Trim => tr("Click the piece of a curve to remove."),
            SketchTool::Extend => tr("Click near the end of a line or an arc to extend it."),
            SketchTool::Split => tr("Click a line or an arc where to cut it in two."),
            SketchTool::Fillet => {
                tr("Click the corner of two lines to round it (the radius is in the Properties panel).")
            }
            SketchTool::Chamfer => {
                tr("Click the corner of two lines to cut it (the distance is in the Properties panel).")
            }
            SketchTool::Offset => {
                tr("Click a curve: it and the curves joined to it get a copy at the distance of the Properties panel.")
            }
            SketchTool::Mirror => tr("Click the line to mirror about."),
            SketchTool::RectPattern => tr("Select the curves, set the columns and rows, then click Make Pattern."),
            SketchTool::CircularPattern => tr("Click the center of the pattern."),
            SketchTool::Move | SketchTool::Copy if clicks == 0 => tr("Click the base point."),
            SketchTool::Move | SketchTool::Copy => tr("Click where the base point goes."),
            SketchTool::Rotate => tr("Click the center of the turn."),
            SketchTool::Scale => tr("Click the base point of the scale."),
        };
        let prompt = if matches!(
            tool,
            SketchTool::Line
                | SketchTool::Rectangle
                | SketchTool::RectCenter
                | SketchTool::Rect3
                | SketchTool::RectCenter3
                | SketchTool::Slot
                | SketchTool::Circle
                | SketchTool::Arc3
                | SketchTool::ArcCenter
                | SketchTool::ArcTangent
                | SketchTool::Polygon
                | SketchTool::Point
        ) && self.sketch.as_ref().is_some_and(|e| e.auto)
        {
            format!("{} {}", prompt, tr("(Hold Alt: no constraints.)"))
        } else {
            prompt.to_string()
        };
        let a = self.sketch_analysis()?;
        let empty = self.sketch_obj()?.sketch().is_empty();
        let state = if empty {
            tr("empty").to_string()
        } else if !a.conflicting.is_empty() {
            fill("{} constraints in conflict (red)", &[&a.conflicting.len()])
        } else if a.free == 0 {
            tr("fully constrained").to_string()
        } else {
            fill("{} free motions", &[&a.free])
        };
        Some((prompt, state))
    }
}
