//! The edits of the open sketch: the edit tools, the transform tools,
//! the constraints and dimensions of the selection, and the changes of the
//! selected parts.

use super::*;

impl AppState {
    /// Changes the open sketch with an edit that can be refused: one undo
    /// step if it is done, none if it is refused.
    pub(super) fn try_edit_sketch<R>(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut Sketch) -> Result<R, EditError>,
    ) -> Option<Result<R, EditError>> {
        let id = self.sketch.as_ref()?.object;
        self.sketch_of(id)?;
        let mut lost = 0;
        let out = self.doc.try_edit(label, |d| {
            let Some(ObjectKind::Sketch(s)) = d.object_mut(id).map(|o| &mut o.kind) else {
                return Err(EditError::Missing);
            };
            let r = s.edit(|sk| {
                let r = f(sk);
                if r.is_ok() {
                    sk.solve();
                }
                r
            });
            if r.is_ok() {
                lost = d.refresh_sketch_links(id);
            }
            r
        });
        self.warn_lost(lost);
        Some(out)
    }

    /// Offset of `curves` at the distance of the panel: one undo step.
    pub(super) fn offset_curves(&mut self, curves: &[CurveId]) {
        let mm = self.sketch.as_ref().map_or(2.0, |e| e.values.offset_mm);
        let r = self.try_edit_sketch(tr("Offset"), |sk| sk.offset(curves, mm));
        self.edit_done(r.map(|r| r.map(|made| fill("Offset: {} new curves.", &[&made.len()]))));
    }

    /// The message of an edit, and the end of its selection.
    fn edit_done(&mut self, r: Option<Result<String, EditError>>) {
        match r {
            Some(Ok(text)) => {
                self.message = text;
                if let Some(e) = &mut self.sketch {
                    e.selection.clear();
                }
            }
            Some(Err(e)) => self.message = edit_message(&e),
            None => {}
        }
    }

    /// A click of an edit tool at sketch position `at`.
    pub(super) fn edit_click(&mut self, tool: SketchTool, at: Point, tol: f64) {
        let Some(obj) = self.sketch_obj() else { return };
        let s = obj.sketch();
        let curve = s.nearest_curve(at, tol, &[]).map(|(c, _)| c);
        let point = infer::snap(s, at, tol, &[]).point;
        let v = self.sketch.as_ref().map(|e| e.values.clone()).unwrap_or_default();
        let source = self.sketch.as_ref().map(|e| e.source.clone()).unwrap_or_default();
        let need_curve = tr("Click a curve.");
        let r: Option<Result<String, EditError>> = match tool {
            SketchTool::Trim => match curve {
                Some(c) => self
                    .try_edit_sketch(tr("Trim"), |sk| sk.trim(c, at))
                    .map(|r| r.map(|_| tr("Trimmed.").to_string())),
                None => Some(Err(EditError::Refused(need_curve))),
            },
            SketchTool::Extend => match curve {
                Some(c) => self
                    .try_edit_sketch(tr("Extend"), |sk| sk.extend(c, at))
                    .map(|r| r.map(|_| tr("Extended.").to_string())),
                None => Some(Err(EditError::Refused(need_curve))),
            },
            SketchTool::Split => match curve {
                Some(c) => self
                    .try_edit_sketch(tr("Split"), |sk| sk.split(c, at))
                    .map(|r| r.map(|_| tr("Split in two.").to_string())),
                None => Some(Err(EditError::Refused(need_curve))),
            },
            SketchTool::Fillet | SketchTool::Chamfer => {
                let Some(p) = point else {
                    self.message = tr("Click the corner of two lines.").into();
                    return;
                };
                let fillet = tool == SketchTool::Fillet;
                let r = if fillet {
                    self.try_edit_sketch(tr("Fillet"), |sk| sk.fillet(p, v.fillet_mm))
                } else {
                    let kind = match v.chamfer_kind {
                        1 => emb_sketch::edit::ChamferKind::Two(v.chamfer_mm, v.chamfer2_mm),
                        2 => emb_sketch::edit::ChamferKind::Angle(v.chamfer_mm, v.chamfer_deg),
                        _ => emb_sketch::edit::ChamferKind::Equal(v.chamfer_mm),
                    };
                    self.try_edit_sketch(tr("Chamfer"), |sk| sk.chamfer_with(p, kind))
                };
                // Type the value of the new dimension at once.
                if let Some(Ok((_, dim))) = &r
                    && let Some(c) = self.sketch_obj().and_then(|o| o.sketch().constraint(*dim).copied())
                {
                    let text = value_text(self.settings.units, &c);
                    if let Some(e) = &mut self.sketch {
                        e.edit_value(*dim, text);
                    }
                }
                r.map(|r| {
                    r.map(|_| {
                        if fillet {
                            tr("Fillet added: type its radius.").to_string()
                        } else {
                            tr("Chamfer added: type its distance.").to_string()
                        }
                    })
                })
            }
            SketchTool::Offset => match curve {
                Some(c) => {
                    let chain = s.connected_curves(c);
                    self.offset_curves(&chain);
                    return;
                }
                None => Some(Err(EditError::Refused(need_curve))),
            },
            SketchTool::Mirror => match curve.filter(|c| is_line(s, *c)) {
                Some(axis) => self
                    .try_edit_sketch(tr("Mirror"), |sk| sk.mirror(&source, axis))
                    .map(|r| r.map(|made| fill("Mirror: {} new curves.", &[&made.len()]))),
                None => Some(Err(EditError::Refused(tr("Click the line to mirror about.")))),
            },
            SketchTool::CircularPattern => {
                let center = point.and_then(|p| s.point(p)).unwrap_or(at);
                let (n, deg) = (v.count as usize, v.angle_deg);
                self.try_edit_sketch(tr("Circular Pattern"), |sk| {
                    sk.circular_pattern(&source, center, n, deg)
                })
                .map(|r| r.map(|made| fill("Pattern: {} new curves.", &[&made.len()])))
            }
            t if t.is_transform() => {
                let at = infer::snap(s, at, tol, &[]).at;
                self.transform_click(t, at);
                return;
            }
            _ => None,
        };
        let done = matches!(r, Some(Ok(_)));
        self.edit_done(r);
        // A mirror or a pattern is done; the other tools stay for more clicks.
        if done
            && matches!(tool, SketchTool::Mirror | SketchTool::CircularPattern)
            && let Some(e) = &mut self.sketch
        {
            e.tool = SketchTool::Select;
            e.source.clear();
        }
    }

    /// A click of Move, Copy, Rotate or Scale at sketch position `at`.
    fn transform_click(&mut self, tool: SketchTool, at: Point) {
        let Some(e) = self.sketch.as_mut() else { return };
        if matches!(tool, SketchTool::Move | SketchTool::Copy) && e.clicks.is_empty() {
            e.clicks.push(Snap::free(at));
            self.message = tr("Click where the base point goes.").into();
            return;
        }
        let base = e.clicks.first().map(|c| c.at);
        e.clicks.clear();
        let v = e.values.clone();
        let f = match (tool, base) {
            (SketchTool::Move | SketchTool::Copy, Some(b)) => Motion::Shift(at - b),
            (SketchTool::Rotate, _) => Motion::Turn(at, v.rotate_deg),
            (SketchTool::Scale, _) => Motion::Grow(at, v.scale),
            _ => return,
        };
        let copy = tool == SketchTool::Copy || (matches!(tool, SketchTool::Rotate | SketchTool::Scale) && v.copy_parts);
        self.apply_motion(tool, f, copy);
    }

    /// Moves, turns or scales the parts of the transform tool (or copies).
    pub fn apply_motion(&mut self, tool: SketchTool, m: Motion, copy: bool) {
        let Some(e) = self.sketch.as_ref() else { return };
        let (curves, points) = (e.source.clone(), e.source_points.clone());
        let label = match tool {
            SketchTool::Stretch => tr("Stretch in Sketch"),
            SketchTool::Rotate => tr("Rotate in Sketch"),
            SketchTool::Scale => tr("Scale in Sketch"),
            _ if copy => tr("Copy in Sketch"),
            _ => tr("Move in Sketch"),
        };
        let (scale, turned) = match m {
            Motion::Grow(_, k) => (k, false),
            Motion::Turn(..) => (1.0, true),
            Motion::Shift(_) => (1.0, false),
        };
        let r = self.try_edit_sketch(label, |sk| {
            sk.transform_parts(&curves, &points, &|p| m.apply(p), scale, turned, copy)
        });
        self.edit_done(r.map(|r| {
            r.map(|made| {
                if copy {
                    fill("{} new curves.", &[&made.len()])
                } else {
                    tr("Moved as near as the constraints let it.").to_string()
                }
            })
        }));
        // The parts stay selected for one more move.
        if let Some(e) = &mut self.sketch {
            e.selection = e
                .source
                .iter()
                .map(|c| Pick::Curve(*c))
                .chain(e.source_points.iter().map(|p| Pick::Point(*p)))
                .collect();
        }
    }

    /// Make Pattern (Rectangular Pattern): the selected curves in columns
    /// and rows.
    pub fn rect_pattern(&mut self) {
        let Some(e) = &self.sketch else { return };
        let curves: Vec<CurveId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .collect();
        if curves.is_empty() {
            self.message = tr("Select the curves to copy first.").into();
            return;
        }
        let v = e.values.clone();
        let r = self.try_edit_sketch(tr("Rectangular Pattern"), |sk| {
            sk.rect_pattern(&curves, v.cols as usize, v.rows as usize, v.dx_mm, v.dy_mm)
        });
        self.edit_done(r.map(|r| r.map(|made| fill("Pattern: {} new curves.", &[&made.len()]))));
    }

    /// Relax: removes the constraints that conflict (dimensions first), in
    /// one undo step, and lists them.
    pub fn relax_sketch(&mut self) {
        let has = self.sketch_analysis().is_some_and(|a| !a.conflicting.is_empty());
        if !has {
            self.message = tr("No constraints conflict.").into();
            return;
        }
        let units = self.settings.units;
        let removed = self.edit_sketch(tr("Relax"), None, |sk| sk.relax()).unwrap_or_default();
        let list: Vec<String> = removed.iter().map(|c| constraint_text(units, c)).collect();
        self.message = fill("Relax removed {} constraints: {}.", &[&list.len(), &list.join(", ")]);
        if let Some(e) = &mut self.sketch {
            e.relaxed = list;
        }
    }

    /// Adds the constraint of `kind` for `picks`. Returns `true` if it was
    /// added (or refused with a message), `false` if it needs more picks.
    pub(super) fn apply_constraint(&mut self, kind: Kind, picks: &[Pick]) -> bool {
        let Some(obj) = self.sketch_obj() else { return true };
        match build(kind, obj.sketch(), picks) {
            Build::Done(c) => {
                let name = c.name();
                let r = self.edit_sketch(&fill("Constraint: {}", &[&tr(name)]), None, |sk| sk.add_constraint(c));
                match r {
                    Some(Ok(_)) => self.message = fill("{} added.", &[&tr(name)]),
                    Some(Err(e)) => self.message = e.to_string(),
                    None => {}
                }
                if let Some(e) = &mut self.sketch {
                    e.selection.clear();
                    e.picks.clear();
                }
                true
            }
            Build::More => false,
            Build::Wrong(why) => {
                self.message = why.into();
                if let Some(e) = &mut self.sketch {
                    e.picks.clear();
                }
                true
            }
        }
    }

    /// Delete in sketch mode: the selected constraints, curves and points
    /// (a point with the curves that use it), with their constraints.
    pub fn delete_sketch_selection(&mut self) {
        let sel = self.sketch.as_ref().map(|e| e.selection.clone()).unwrap_or_default();
        if sel.is_empty() {
            self.message = tr("Select the parts to delete first.").into();
            return;
        }
        self.edit_sketch(tr("Delete in Sketch"), None, |sk| {
            for p in &sel {
                match *p {
                    Pick::Constraint(c) => {
                        sk.remove_constraint(c);
                    }
                    Pick::Curve(c) => {
                        sk.remove_curve(c);
                    }
                    Pick::Point(_) => {}
                }
            }
            for p in &sel {
                // The origin stays (v0.3 plan, Stage 4).
                if let Pick::Point(id) = *p
                    && !sk.is_origin(id)
                {
                    let users: Vec<CurveId> = sk
                        .curves
                        .iter()
                        .filter(|c| c.points().contains(&id))
                        .map(|c| c.id)
                        .collect();
                    for c in users {
                        sk.remove_curve(c);
                    }
                    sk.remove_point(id);
                }
            }
        });
        if let Some(e) = &mut self.sketch {
            e.selection.clear();
        }
        self.message = if sel.len() == 1 {
            tr("Deleted.").to_string()
        } else {
            fill("Deleted {} parts.", &[&sel.len()])
        };
    }

    /// Adds dimension `c` with its label at `at` (else the program places
    /// it), and opens its value box. A dimension that the sketch sets
    /// already is driven: it shows the size only.
    pub fn add_sketch_dimension(&mut self, c: Constraint, at: Option<Point>) {
        let r = self.edit_sketch(&fill("Dimension: {}", &[&tr(c.name())]), None, |sk| {
            let r = sk.add_dimension(c);
            if let (Ok((id, _)), Some(at)) = (&r, at) {
                sk.label_at.insert(*id, at);
            }
            r
        });
        let units = self.settings.units;
        let Some(e) = self.sketch.as_mut() else { return };
        e.picks.clear();
        match r {
            Some(Ok((id, false))) => e.edit_value(id, value_text(units, &c)),
            Some(Ok((_, true))) => {
                self.message = tr(
                    "The sketch sets this size already: the dimension is driven (in parentheses). Right-click it to make it driving.",
                )
                .into();
            }
            Some(Err(err)) => self.message = err.to_string(),
            None => {}
        }
    }

    /// Makes the selected dimension driven, or driving again.
    pub fn toggle_driven(&mut self, id: ConstraintId) {
        let on = !self.sketch_obj().is_some_and(|o| o.sketch().is_driven(id));
        self.edit_sketch(if on { "Driven Dimension" } else { "Driving Dimension" }, None, |sk| {
            sk.set_driven(id, on)
        });
        if !on
            && self
                .sketch_analysis()
                .is_some_and(|a| a.conflicting.contains(&id) || a.redundant.contains(&id))
        {
            self.message = tr("This dimension repeats or conflicts with others (gray or red).").into();
        }
    }

    /// Removes the constraints of the selected points and curves.
    pub fn remove_sketch_constraints_of_selection(&mut self) {
        let sel = self.sketch.as_ref().map(|e| e.selection.clone()).unwrap_or_default();
        let (points, curves): (Vec<PointId>, Vec<CurveId>) = (
            sel.iter()
                .filter_map(|p| match p {
                    Pick::Point(p) => Some(*p),
                    _ => None,
                })
                .collect(),
            sel.iter()
                .filter_map(|p| match p {
                    Pick::Curve(c) => Some(*c),
                    _ => None,
                })
                .collect(),
        );
        let n = self.edit_sketch(tr("Delete Constraints"), None, |sk| {
            let before = sk.constraints.len();
            sk.constraints.retain(|c| {
                !c.constraint.points().iter().any(|p| points.contains(p))
                    && !c.constraint.curves().iter().any(|x| curves.contains(x))
            });
            before - sk.constraints.len()
        });
        self.message = fill("Removed {} constraints.", &[&n.unwrap_or(0)]);
    }

    /// Selects all points, curves and constraints of the open sketch.
    pub fn sketch_select_all(&mut self) {
        let Some(obj) = self.sketch_obj() else { return };
        let s = obj.sketch();
        let all: Vec<Pick> = s
            .curves
            .iter()
            .map(|c| Pick::Curve(c.id))
            .chain(s.constraints.iter().map(|c| Pick::Constraint(c.id)))
            .collect();
        if let Some(e) = &mut self.sketch {
            e.selection = all;
        }
    }

    /// Makes the selected curves construction curves (or not), or turns
    /// construction on for new curves.
    pub fn toggle_construction(&mut self) {
        let Some(e) = &self.sketch else { return };
        let curves: Vec<CurveId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .collect();
        if curves.is_empty() {
            if let Some(e) = &mut self.sketch {
                e.construction = !e.construction;
            }
            return;
        }
        let on = self.sketch_obj().is_some_and(|o| {
            curves
                .iter()
                .any(|c| o.sketch().curve(*c).is_some_and(|c| !c.construction))
        });
        self.edit_sketch(tr("Construction"), None, |sk| {
            for c in &curves {
                sk.set_construction(*c, on);
            }
        });
    }

    /// Makes the selected points center points, or not.
    pub fn toggle_center_point(&mut self) {
        let Some(e) = &self.sketch else { return };
        let pts: Vec<PointId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Point(p) => Some(*p),
                _ => None,
            })
            .collect();
        if pts.is_empty() {
            self.message = tr("Select the points to make center points.").into();
            return;
        }
        let on = self
            .sketch_obj()
            .is_some_and(|o| pts.iter().any(|p| !o.sketch().is_center_point(*p)));
        self.edit_sketch(tr("Center Point"), None, |sk| {
            for p in &pts {
                sk.set_center_point(*p, on);
            }
        });
    }

    /// Automatic Dimensions: constraints and dimensions for the selected
    /// curves (all curves when none is selected), in one undo step.
    pub fn auto_dimension(&mut self) {
        let Some(e) = &self.sketch else { return };
        let curves: Vec<CurveId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .collect();
        let r = self.edit_sketch(tr("Automatic Dimensions"), None, |sk| sk.auto_dimension(&curves));
        let free = self.sketch_analysis().map_or(0, |a| a.free);
        if let Some((c, d)) = r {
            self.message = fill(
                "Added {} constraints and {} dimensions; {} free motions left (a Fix places the sketch).",
                &[&c, &d, &free],
            );
        }
    }

    /// Makes the selected lines centerlines, or not.
    pub fn toggle_centerline(&mut self) {
        let Some(e) = &self.sketch else { return };
        let lines: Vec<CurveId> = e
            .selection
            .iter()
            .filter_map(|p| match p {
                Pick::Curve(c) => Some(*c),
                _ => None,
            })
            .filter(|c| self.sketch_obj().is_some_and(|o| is_line(o.sketch(), *c)))
            .collect();
        if lines.is_empty() {
            self.message = tr("Select the lines to make centerlines.").into();
            return;
        }
        let on = self
            .sketch_obj()
            .is_some_and(|o| lines.iter().any(|l| !o.sketch().is_centerline(*l)));
        self.edit_sketch(tr("Centerline"), None, |sk| {
            for l in &lines {
                sk.set_centerline(*l, on);
            }
        });
    }

    /// Sets the value of dimension `id` from the text the user typed (in
    /// the units of the Options; an angle in degrees).
    pub(super) fn set_dimension_text(&mut self, id: ConstraintId, text: &str) {
        let Some(old) = self.sketch_obj().and_then(|o| o.sketch().constraint(id).copied()) else {
            return;
        };
        // An expression with the names of parameters (ADR 0078).
        if emb_sketch::expr::parse(text).is_ok_and(|e| !e.names().is_empty()) {
            let units = self.settings.units;
            let r = self.edit_params("Dimension", |sk| {
                first_units(sk, units);
                sk.set_dimension_expr(id, text)
            });
            if let Err(e) = r {
                self.message = expr_message(&e);
            }
            return;
        }
        let typed = match old {
            Constraint::Angle { .. } => crate::workspace::parse_number(text),
            _ => crate::workspace::parse_length(text, self.settings.units),
        };
        let Some(typed) = typed else {
            self.message = tr("Type a number, for example 12.5, 2 in or 1 1/2\".").into();
            return;
        };
        let has_expr = self
            .sketch_obj()
            .is_some_and(|o| o.sketch().dimension_expr(id).is_some());
        if old.value() == Some(typed) && !has_expr {
            return;
        }
        let v = match old {
            Constraint::Angle { .. } => typed,
            // A horizontal or vertical distance keeps its direction.
            _ => typed.abs() * old.value().map_or(1.0, |o| if o < 0.0 { -1.0 } else { 1.0 }),
        };
        let r = self.edit_sketch(tr("Dimension"), None, |sk| {
            sk.clear_dimension_expr(id);
            sk.set_value(id, v)
        });
        match r {
            Some(Err(e)) => self.message = e.to_string(),
            _ => {
                if let Some(a) = self.sketch_analysis()
                    && a.conflicting.contains(&id)
                {
                    self.message = tr("This value conflicts with other constraints (red).").into();
                }
            }
        }
    }
}
