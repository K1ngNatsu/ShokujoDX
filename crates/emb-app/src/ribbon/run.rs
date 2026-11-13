//! What the selection holds (`SelInfo`), and what each command can do
//! and does: `AppState::can`, `is_on` and `run`.

use super::*;

/// What the selection holds. It sets which commands are available.
pub struct SelInfo {
    pub n: usize,
    pub fills: usize,
    pub holes: bool,
    pub lines: usize,
    pub single_line: bool,
    /// One object that is not a line (a fill or a satin).
    pub single_shape: bool,
    pub appliques: usize,
    pub letterings: usize,
    /// Satin columns and lines with a border: objects with a column type.
    pub columns: usize,
    /// Vector objects (artwork).
    pub vectors: usize,
    /// Objects that take part in shaping (objects with an area).
    pub shapes: usize,
    /// Stitch objects (from a machine file or a photo stitch).
    pub stitches: usize,
}

impl SelInfo {
    pub fn of(st: &AppState) -> Self {
        let d = st.doc.design();
        let objs: Vec<&emb_model::DesignObject> = st.selection.ids().iter().filter_map(|id| d.object(*id)).collect();
        let count = |f: fn(&ObjectKind) -> bool| objs.iter().filter(|o| f(&o.kind)).count();
        let fills = count(|k| matches!(k, ObjectKind::Fill { .. }));
        let lines = count(|k| matches!(k, ObjectKind::Running { .. }));
        let vectors = count(|k| matches!(k, ObjectKind::Vector { .. }));
        SelInfo {
            vectors,
            shapes: count(emb_model::shaping::takes_part),
            n: objs.len(),
            fills,
            holes: count(|k| matches!(k, ObjectKind::Fill { holes, .. } if !holes.is_empty())) > 0,
            lines,
            single_line: objs.len() == 1 && lines == 1,
            single_shape: objs.len() == 1 && lines == 0 && vectors == 0,
            appliques: count(|k| matches!(k, ObjectKind::Applique { .. })),
            stitches: count(|k| matches!(k, ObjectKind::Stitches { .. })),
            letterings: count(|k| matches!(k, ObjectKind::Lettering(_))),
            columns: count(|k| match k {
                ObjectKind::Satin { .. } => true,
                ObjectKind::Running { params, .. } => params.border.is_some(),
                _ => false,
            }),
        }
    }
}

impl AppState {
    /// Returns `true` if the command can run now.
    pub fn can(&self, cmd: Cmd, s: &SelInfo) -> bool {
        let d = self.doc.design();
        match cmd {
            Cmd::Tool(Tool::Knife | Tool::MirrorLine | Tool::RefPoints) => s.n > 0,
            Cmd::Tool(Tool::CropRect | Tool::CropEllipse | Tool::CropPolygon | Tool::ClickStitch) => {
                d.backdrop.as_ref().is_some_and(|b| b.visible)
            }
            Cmd::Tool(Tool::AddHole) => s.n == 1 && s.fills == 1,
            Cmd::Tool(Tool::StitchAngle | Tool::DirectionLine) => s.single_shape,
            Cmd::StartSketch => self.sketch.is_none(),
            Cmd::Sketch(_) => self.sketch.is_some(),
            Cmd::Undo => self.doc.can_undo(),
            Cmd::Redo => self.doc.can_redo(),
            Cmd::Paste | Cmd::PasteSpecial => self.clipboard.is_some(),
            Cmd::PasteAfter => self.clipboard.is_some() && s.n > 0,
            Cmd::CloseShape(_) => self.selected_count(crate::editing::has_open_part) > 0,
            Cmd::Reverse => self.selected_count(crate::editing::reversible) > 0,
            Cmd::SwapSides => self.selected_count(|k| matches!(k, ObjectKind::Satin { .. })) > 0,
            Cmd::ToSingleStones => self.selected_count(crate::editing::stone_pattern) > 0,
            Cmd::SelectAtCursor | Cmd::CenterAtCursor => self.cursor.is_some(),
            Cmd::RemoveHoles | Cmd::FillHoles => s.holes,
            Cmd::ContinueLine => s.single_line,
            Cmd::Convert(_) => s.vectors > 0,
            Cmd::Shape(emb_model::ShapeOp::Combine) => s.vectors >= 2 || s.appliques >= 2,
            Cmd::Shape(emb_model::ShapeOp::Flatten) => s.shapes + s.letterings >= 2,
            Cmd::Shape(_) => s.shapes >= 2,
            Cmd::Offsets | Cmd::AddBorder | Cmd::Arrangements | Cmd::Quilting | Cmd::Cutwork => true,
            Cmd::RemoveOverlaps => s.shapes + s.letterings > 0,
            Cmd::MirrorMerge(..) | Cmd::Reflect => s.n > 0,
            Cmd::BorderTrace => true,
            Cmd::PictureTools | Cmd::EditPictureOutside => d.backdrop.is_some(),
            Cmd::SendByEmail => !d.objects().is_empty(),
            Cmd::NewView => self.views.iter().filter(|v| v.is_some()).count() < crate::views::MAX_VIEWS,
            Cmd::OtherDesign => self.tab_count() > 1,
            Cmd::PictureWizard => true,
            Cmd::TracePicture | Cmd::ClickAs(_) => d.backdrop.as_ref().is_some_and(|b| b.visible),
            Cmd::Unlink => {
                let d = self.doc.design();
                let made = |id: &emb_model::ObjectId| d.sketch_links.iter().any(|l| l.sketch == *id);
                self.selection.ids().iter().any(|id| d.is_linked(*id) || made(id))
            }
            Cmd::ToVector => s.n > s.vectors,
            Cmd::VectorToSketch => s.vectors > 0,
            Cmd::RecognizeOutlines => s.stitches > 0,
            Cmd::ExportOutlines => !d.objects().is_empty(),
            Cmd::FlorentineGuide => s.n == 2 && s.fills == 1 && s.lines == 1,
            Cmd::Laydown => !d.objects().is_empty(),
            Cmd::Motifs | Cmd::Borders | Cmd::Buttonholes | Cmd::Blends => true,
            Cmd::SplitLines | Cmd::GuideLines => s.fills == 1 && s.lines > 0 && s.n == 1 + s.lines,
            Cmd::Tartan | Cmd::Gradient => true,
            Cmd::RippleGuide => s.n == 2 && s.lines == 2,
            Cmd::Redwork | Cmd::Branching => s.lines > 0,
            Cmd::LetteringOnLine => {
                (s.n == 2 && s.lines == 1 && s.letterings == 1) || (s.n == 1 && (s.lines == 1 || s.letterings == 1))
            }
            Cmd::QuickSimplify | Cmd::Simplify => s.n > s.letterings,
            Cmd::CloseGaps => s.lines + s.vectors > 0,
            Cmd::CopyProperties => s.n > 0,
            Cmd::Envelope => s.n > s.letterings + s.stitches,
            Cmd::RepeatLast => self.last_cmd.is_some_and(|c| c != Cmd::RepeatLast && self.can(c, s)),
            Cmd::CleanUp => s.lines + s.vectors + s.fills > 0,
            Cmd::Align(_) | Cmd::SameSize(..) | Cmd::Group | Cmd::ClosestJoin | Cmd::ClickOrder | Cmd::Join => s.n > 1,
            Cmd::OptimizeOrder | Cmd::CapOrder => d.objects().len() > 1,
            Cmd::Puff => self.selection.ids().iter().any(|id| {
                matches!(
                    d.object(*id).map(|o| &o.kind),
                    Some(ObjectKind::Satin { .. } | ObjectKind::Lettering(_))
                ) || matches!(d.object(*id).map(|o| &o.kind), Some(ObjectKind::Running { params, .. }) if params.border.is_some())
            }),
            Cmd::Space(_) => s.n > 2,
            Cmd::UnlockAll => d.objects().iter().any(|o| o.locked),
            Cmd::ShowAll => self.view_filter.is_active(),
            Cmd::RemoveGuides => !d.guides.is_empty() || !d.marks.is_empty(),
            Cmd::Cut
            | Cmd::Copy
            | Cmd::Duplicate
            | Cmd::Delete
            | Cmd::BreakApart
            | Cmd::RotateCw
            | Cmd::RotateCcw
            | Cmd::Rotate180
            | Cmd::MirrorH
            | Cmd::MirrorV
            | Cmd::CenterInHoop
            | Cmd::Ungroup
            | Cmd::Lock
            | Cmd::ToStart
            | Cmd::BackOne
            | Cmd::ForwardOne
            | Cmd::ToEnd
            | Cmd::BackColor
            | Cmd::ForwardColor
            | Cmd::ZoomSelection
            | Cmd::ShowSelectedColors
            | Cmd::HideOthers
            | Cmd::HideSelected
            | Cmd::SplitThread => s.n > 0,
            Cmd::DeleteColorway => d.colorways.len() > 1,
            Cmd::Func(_) => s.n > 0,
            Cmd::SplitAtNeedle => self.cursor.is_some(),
            Cmd::CycleColors => d.used_threads().len() > 1,
            Cmd::MatchChart => !d.objects().is_empty(),
            Cmd::OutputDesign | Cmd::PrintPreview | Cmd::SendToUsb | Cmd::SendToUsbChoose => !d.objects().is_empty(),
            Cmd::MakeCurrent | Cmd::ApplyCurrent => s.n > 0,
            Cmd::CornerRadius | Cmd::HoleFromOutline => s.n == 1 && (s.fills == 1 || s.appliques == 1),
            Cmd::PreviousView => !self.view_history.is_empty(),
            Cmd::ConvertToApplique => s.fills + s.vectors > 0,
            Cmd::ConvertToCross => s.shapes > 0,
            Cmd::PictureToCross => true,
            Cmd::ToStonesRun => s.n > 0,
            Cmd::ToStonesFill | Cmd::ToStonesCenter => s.shapes + s.letterings > 0,
            Cmd::StoneTemplate | Cmd::TemplateMarks => {
                d.objects().iter().any(|o| matches!(o.kind, ObjectKind::Bling(_)))
            }
            Cmd::CrossChart => d.objects().iter().any(|o| matches!(o.kind, ObjectKind::CrossStitch(_))),
            Cmd::AutoHoop => !d.objects().is_empty(),
            Cmd::SaveDecoration => d.product.is_some(),
            Cmd::MakeRepeats => d.repeat.is_some() && !d.objects().is_empty(),
            Cmd::AutoLayout => !d.objects().is_empty() && self.layout_hoop().is_some(),
            Cmd::AddHooping => d.hooping.is_some() && self.layout_hoop().is_some_and(|h| h.positions.is_empty()),
            Cmd::DrawSplit | Cmd::ClearHoopings => d.hooping.is_some(),
            Cmd::DeleteHooping => self.hoopings_ui.selected.is_some(),
            Cmd::TurnHooping => matches!(self.hoopings_ui.selected, Some(crate::hooping::HoopingPick::Hooping(_))),
            Cmd::DeleteSplit => d.hooping.as_ref().is_some_and(|l| !l.splits.is_empty()),
            Cmd::PreviewHoopings | Cmd::OutputHoopings => d.hooping.as_ref().is_some_and(|l| !l.hoopings.is_empty()),
            Cmd::TurnHoopLeft | Cmd::TurnHoopRight | Cmd::HoopTemplate => {
                matches!(d.hoop.selection, HoopSelection::Preset { .. })
            }
            Cmd::CombineApplique => s.appliques > 1,
            Cmd::AppliqueFromSelection => s.shapes + s.columns > 0,
            Cmd::BackToOriginal => self.selection.ids().iter().any(|id| d.applique_origin(*id).is_some()),
            Cmd::SaveClipart => s.n > 0,
            Cmd::CutterFile => d
                .objects()
                .iter()
                .any(|o| matches!(o.kind, emb_model::ObjectKind::Applique { .. })),
            _ => true,
        }
    }

    /// Returns `true` if the button of the command shows as on.
    pub fn is_on(&self, cmd: Cmd) -> bool {
        match cmd {
            Cmd::Tool(t) => self.tool == t,
            Cmd::ClickAs(c) => self.tool == Tool::ClickStitch && self.picture.click == c,
            Cmd::Toggle(f) => self.flag(f),
            Cmd::Func(f) => self.func_on(f),
            Cmd::Sketch(SketchCmd::Tool(t)) => self.sketch.as_ref().is_some_and(|e| e.tool == t),
            Cmd::Sketch(SketchCmd::Construction) => self.sketch.as_ref().is_some_and(|e| e.construction),
            Cmd::Sketch(SketchCmd::RelaxMode) => self.sketch.as_ref().is_some_and(|e| e.relax_mode),
            Cmd::Sketch(SketchCmd::Parameters) => self.sketch.as_ref().is_some_and(|e| e.params.open),
            Cmd::DrawSplit => self.hoopings_ui.drawing.is_some(),
            Cmd::PreviewHoopings => self.hoopings_ui.preview,
            _ => false,
        }
    }

    fn flag(&self, f: Flag) -> bool {
        let s = &self.settings;
        match f {
            Flag::Artwork => self.show_artwork,
            Flag::Stitches => self.show_plan,
            Flag::Jumps => self.show_jumps,
            Flag::NeedlePoints => self.show_needle_points,
            Flag::Functions => self.show_functions,
            Flag::Density => self.show_density,
            Flag::StitchCursor => self.cursor.is_some(),
            Flag::Grid => s.show_grid,
            Flag::Rulers => s.show_rulers,
            Flag::SnapGrid => s.snap_grid,
            Flag::SnapGuides => s.snap_guides,
            Flag::SnapObjects => s.snap_objects,
            Flag::ThreadLook => self.show_thread_look,
            Flag::Player => self.player.shown,
            Flag::Overview => self.show_overview,
            Flag::AppliqueFabric => self.show_applique_fabric,
            Flag::Stones => self.show_stones,
            Flag::SimpleMode => s.simple_mode,
            Flag::SplitView => self.show_split,
            Flag::DarkMode => s.dark_mode,
            Flag::Vectors => self.show_vectors,
            Flag::DimVectors => self.dim_vectors,
            Flag::DrawVectors => self.draw_vectors,
            Flag::Hoop => self.show_hoop,
            Flag::HoopingMode => self.hoopings_ui.mode,
            Flag::Product => self.doc.design().product.as_ref().is_some_and(|v| v.visible),
            Flag::Repeats => self.doc.design().repeat.is_some_and(|r| r.show),
        }
    }

    fn toggle(&mut self, f: Flag) {
        let mut s = self.settings.clone();
        match f {
            Flag::Artwork => self.show_artwork = !self.show_artwork,
            Flag::Stitches => self.show_plan = !self.show_plan,
            Flag::Jumps => self.show_jumps = !self.show_jumps,
            Flag::NeedlePoints => self.show_needle_points = !self.show_needle_points,
            Flag::Functions => self.show_functions = !self.show_functions,
            Flag::Density => self.show_density = !self.show_density,
            Flag::StitchCursor if self.cursor.is_some() => self.cursor = None,
            Flag::StitchCursor => self.travel(Travel::Stitches(0)),
            Flag::Grid => s.show_grid = !s.show_grid,
            Flag::Rulers => s.show_rulers = !s.show_rulers,
            Flag::DarkMode => s.dark_mode = !s.dark_mode,
            Flag::SnapGrid => s.snap_grid = !s.snap_grid,
            Flag::SnapGuides => s.snap_guides = !s.snap_guides,
            Flag::SnapObjects => s.snap_objects = !s.snap_objects,
            Flag::ThreadLook => {
                self.show_thread_look = !self.show_thread_look;
                self.show_plan |= self.show_thread_look;
            }
            Flag::Player => self.toggle_player(),
            Flag::Overview => self.show_overview = !self.show_overview,
            Flag::AppliqueFabric => self.show_applique_fabric = !self.show_applique_fabric,
            Flag::Stones => self.show_stones = !self.show_stones,
            Flag::SimpleMode => {
                s.simple_mode = !s.simple_mode;
                self.ribbon_widths.clear();
                if !RibbonTab::visible(s.simple_mode).contains(&s.ribbon_tab) {
                    s.ribbon_tab = RibbonTab::Digitize;
                }
            }
            Flag::Vectors => self.show_vectors = !self.show_vectors,
            Flag::DimVectors => {
                self.dim_vectors = !self.dim_vectors;
                self.show_vectors |= self.dim_vectors;
            }
            Flag::DrawVectors => self.draw_vectors = !self.draw_vectors,
            Flag::Hoop => self.show_hoop = !self.show_hoop,
            Flag::HoopingMode => self.set_hooping_mode(!self.hoopings_ui.mode),
            Flag::Product => match self.doc.design().product.as_ref().map(|v| v.visible) {
                Some(shown) => {
                    self.doc.edit(tr("Show Product"), |d| {
                        if let Some(v) = d.product.as_mut() {
                            v.visible = !shown;
                        }
                    });
                }
                None => self.product_window = true,
            },
            Flag::Repeats => match self.doc.design().repeat.map(|r| r.show) {
                Some(shown) => {
                    self.doc.edit(tr("Show Repeats"), |d| {
                        if let Some(r) = d.repeat.as_mut() {
                            r.show = !shown;
                        }
                    });
                }
                None => self.open_repeats(),
            },
            Flag::SplitView => {
                self.show_split = !self.show_split;
                self.split.fit_requested = true;
            }
        }
        self.set_settings(s);
    }

    /// Runs a ribbon command.
    pub fn run(&mut self, cmd: Cmd, ctx: &egui::Context) {
        let label = cmd.meta().1;
        // Space repeats it (not undo, redo and the view switches).
        if !matches!(cmd, Cmd::Undo | Cmd::Redo | Cmd::Toggle(_) | Cmd::RepeatLast) {
            self.last_cmd = Some(cmd);
        }
        match cmd {
            Cmd::Tool(t) => self.set_tool(t),
            Cmd::Toggle(f) => self.toggle(f),
            Cmd::New => self.request(Pending::New),
            Cmd::Open => self.request(Pending::Open(None)),
            Cmd::Save => {
                self.save();
            }
            Cmd::SaveAs => {
                self.save_as();
            }
            Cmd::Undo => self.undo(),
            Cmd::Redo => self.redo(),
            Cmd::Export => self.open_export(),
            Cmd::ImportSvg => self.import_artwork_dialog(ArtworkImport::Objects),
            Cmd::ImportSvgLines => self.import_artwork_dialog(ArtworkImport::Lines),
            Cmd::ImportSvgVectors => self.import_artwork_dialog(ArtworkImport::Vectors),
            Cmd::Convert(to) => self.convert_vectors(to),
            Cmd::Shape(op) => self.shape_selection(op),
            Cmd::Offsets => self.artwork.offsets = true,
            Cmd::Quilting => self.quilting.open = true,
            Cmd::Cutwork => self.cutwork.open = true,
            Cmd::AddBorder => {
                self.artwork.spec.make = emb_model::OffsetMake::Border {
                    width_mm: self.artwork.border_mm,
                    place: self.artwork.border_place,
                };
                self.artwork.spec.distance_mm = self.artwork.spec.distance_mm.abs().min(1.0);
                self.artwork.spec.count = 1;
                self.artwork.offsets = true;
            }
            Cmd::RemoveOverlaps => self.remove_overlaps(),
            Cmd::Arrangements => self.artwork.arrangements = true,
            Cmd::MirrorMerge(h, v) => self.arrange_selection(&emb_model::Arrangement::MirrorMerge {
                horizontal: h,
                vertical: v,
                gap_mm: self.artwork.mirror_gap_mm,
            }),
            Cmd::Reflect => self.start_reflect(),
            Cmd::BorderTrace => self.artwork.trace = true,
            Cmd::PictureTools => {
                self.selection.clear();
                self.focus_properties = true;
            }
            Cmd::TracePicture => self.trace_backdrop(),
            Cmd::PictureWizard => self.picture_to_design(),
            Cmd::ClickAs(c) => {
                self.picture.click = c;
                self.set_tool(Tool::ClickStitch);
            }
            Cmd::Unlink => self.unlink_selection(),
            Cmd::ToVector => self.convert_to_vectors(),
            Cmd::VectorToSketch => self.vectors_to_sketch(),
            Cmd::RecognizeOutlines => self.recognize_outlines(),
            Cmd::ExportOutlines => self.export_outlines_dialog(),
            Cmd::LoadPicture => self.import_bitmap_dialog(),
            Cmd::Cut => self.cut_selection(ctx),
            Cmd::Copy => self.copy_selection(ctx),
            Cmd::Paste => self.paste(None),
            Cmd::PasteAfter => self.paste_after_selected(),
            Cmd::PasteSpecial => self.open_paste_special(),
            Cmd::CloseShape(curve) => self.close_shapes(curve),
            Cmd::Reverse => self.reverse_selection(),
            Cmd::SwapSides => self.swap_sides(),
            Cmd::ToSingleStones => self.make_single_stones(),
            Cmd::Duplicate => self.duplicate_selection(),
            Cmd::Delete if self.sketch.is_some() => self.delete_sketch_selection(),
            Cmd::Delete => self.delete_selected(),
            Cmd::SelectAll => self.select_all(),
            Cmd::SelectNext => self.select_step(true),
            Cmd::SelectPrev => self.select_step(false),
            Cmd::SelectAtCursor => self.select_at_needle(),
            Cmd::BreakApart => self.break_apart(),
            Cmd::Join => self.join_selection(),
            Cmd::FlorentineGuide => {
                crate::stitch_types::florentine_guide(self);
            }
            Cmd::Laydown => {
                crate::stitch_types::add_laydown(self);
            }
            Cmd::Motifs => self.motifs_window.open = true,
            Cmd::Borders => self.decor.borders = true,
            Cmd::Buttonholes => self.open_buttonholes(),
            Cmd::Blends => self.decor.blends = true,
            Cmd::SplitLines => {
                crate::stitch_types::split_lines(self);
            }
            Cmd::Tartan | Cmd::Gradient => {
                self.color_fills.open = true;
                self.color_fills.gradient = cmd == Cmd::Gradient;
            }
            Cmd::GuideLines => {
                crate::stitch_types::guide_lines(self);
            }
            Cmd::RippleGuide => {
                crate::stitch_types::ripple_guide(self);
            }
            Cmd::ClosestJoin => self.closest_join(),
            Cmd::RemoveHoles => self.remove_holes(),
            Cmd::FillHoles => self.fill_holes(),
            Cmd::ContinueLine => self.continue_line(),
            Cmd::Redwork => self.make_network(RunPass::Redwork),
            Cmd::Branching => self.make_network(RunPass::Branch),
            Cmd::RotateCw => self.rotate_selection(90.0),
            Cmd::RotateCcw => self.rotate_selection(-90.0),
            Cmd::Rotate180 => self.rotate_selection(180.0),
            Cmd::MirrorH => self.mirror_selection(true),
            Cmd::MirrorV => self.mirror_selection(false),
            Cmd::CenterInHoop => self.center_in_hoop(),
            Cmd::Align(a) => self.align(a),
            Cmd::Space(across) => self.space_evenly(across),
            Cmd::SameSize(w, h) => self.make_same_size(w, h),
            Cmd::Group => self.group_selection(),
            Cmd::Ungroup => self.ungroup_selection(),
            Cmd::Lock => self.lock_selection(),
            Cmd::UnlockAll => self.unlock_all(),
            Cmd::ToStart => self.sequence_selected(label, |d, ids| d.move_objects_to_start(ids)),
            Cmd::BackOne => self.sequence_selected(label, |d, ids| d.step_objects(ids, Step::Backward)),
            Cmd::ForwardOne => self.sequence_selected(label, |d, ids| d.step_objects(ids, Step::Forward)),
            Cmd::ToEnd => self.sequence_selected(label, |d, ids| d.move_objects_to_end(ids)),
            Cmd::BackColor => self.step_color(true),
            Cmd::ForwardColor => self.step_color(false),
            Cmd::ClickOrder => {
                let ids = self.selection.ids().to_vec();
                self.resequence(tr("Sequence by Click Order"), |d| d.sequence_in_order(&ids));
            }
            Cmd::OptimizeColors => {
                self.resequence(tr("Optimize color changes"), emb_model::Design::optimize_color_changes)
            }
            Cmd::OptimizeOrder => self.optimize_sew_order(),
            Cmd::CapOrder => self.cap_sew_order(),
            Cmd::Puff => {
                let n = crate::stitch_types::toggle_puff(self);
                self.message = crate::i18n::fill("Puff changed on {} objects.", &[&n]);
            }
            Cmd::FindCommand => self.open_palette(),
            Cmd::GroupByColor => self.resequence(tr("Group by Color"), emb_model::Design::group_by_color),
            Cmd::ZoomExtents => {
                self.push_view();
                self.fit_requested = true;
            }
            Cmd::Zoom100 => self.set_zoom(1.0),
            Cmd::ZoomIn => self.zoom_step(true, self.settings.zoom_step),
            Cmd::ZoomOut => self.zoom_step(false, self.settings.zoom_step),
            Cmd::ZoomTwice(inward) => self.zoom_step(inward, 2.0),
            Cmd::CenterAtCursor => self.center_at_cursor(),
            Cmd::Kiosk => self.open_kiosk(),
            Cmd::EditPictureOutside => self.edit_backdrop_outside(),
            Cmd::Assets => self.assets_window.open = true,
            Cmd::NewView => {
                self.new_view();
            }
            Cmd::OtherDesign => self.show_other_design(),
            Cmd::SendByEmail => self.send_by_email(),
            Cmd::HelpContents => self.open_help("getting-started", None),
            Cmd::HelpTopic(id) => self.open_help(id, None),
            Cmd::KeyboardShortcuts => self.open_help(crate::help::KEYBOARD_ID, None),
            Cmd::StartPage => self.show_start_page(),
            Cmd::Close => self.request(Pending::CloseTab),
            Cmd::CloseAll => self.request(Pending::CloseAll),
            Cmd::ZoomHoop => {
                self.push_view();
                self.zoom_to = self.hoop_box();
            }
            Cmd::ZoomSelection => {
                self.push_view();
                self.zoom_to_selection();
            }
            Cmd::ShowSelectedColors => {
                let d = self.doc.design();
                let threads = self
                    .selection
                    .ids()
                    .iter()
                    .filter_map(|id| d.object(*id))
                    .map(|o| o.thread)
                    .collect();
                self.view_filter.threads = Some(threads);
            }
            Cmd::HideOthers => self.view_filter.objects = Some(self.selection.ids().iter().copied().collect()),
            Cmd::HideSelected => self.hide_selection(),
            Cmd::ShowAll => self.view_filter = ViewFilter::default(),
            Cmd::RemoveGuides => {
                self.doc.edit(tr("Remove Guides"), |d| {
                    d.guides.clear();
                    d.marks.clear();
                });
                self.guide_edit = None;
            }
            Cmd::Options => self.show_options = true,
            Cmd::RotateDesign => {
                self.doc
                    .edit(tr("Rotate Design"), |d| d.rotation = d.rotation.turned_cw());
            }
            Cmd::AutoHoop => self.auto_hoop(),
            Cmd::TurnHoopLeft => self.turn_hoop(-crate::hoops::HOOP_STEP_DEG),
            Cmd::TurnHoopRight => self.turn_hoop(crate::hoops::HOOP_STEP_DEG),
            Cmd::HoopOptions => self.hoop_windows.options = true,
            Cmd::MyHoops => self.hoop_windows.my_hoops = true,
            Cmd::HoopTemplate => self.hoop_template_dialog(),
            Cmd::AutoLayout => self.auto_layout(),
            Cmd::ProductWindow => self.product_window = true,
            Cmd::SaveDecoration => self.save_decoration_dialog(),
            Cmd::RepeatsWindow => self.open_repeats(),
            Cmd::About => self.about_window = true,
            Cmd::Library => self.focus_library = true,
            Cmd::ClipartPanel => self.focus_clipart = true,
            Cmd::CutterFile => self.cutter_file_dialog(),
            Cmd::SaveClipart => self.open_save_clipart(),
            Cmd::SendToUsb => self.send_to_usb(),
            Cmd::SendToUsbChoose => self.open_send_dialog(None),
            Cmd::MakeRepeats => self.make_repeats(),
            Cmd::AddHooping => self.add_hooping(),
            Cmd::DeleteHooping => self.delete_hooping_pick(),
            Cmd::TurnHooping => self.turn_hooping(),
            Cmd::DrawSplit => self.toggle_draw_split(),
            Cmd::DeleteSplit => {
                self.doc.edit(tr("Remove Split Lines"), |d| {
                    if let Some(l) = d.hooping.as_mut() {
                        l.splits.clear();
                    }
                });
                self.hoopings_ui.selected = None;
            }
            Cmd::ClearHoopings => self.clear_hoopings(),
            Cmd::PreviewHoopings => self.hoopings_ui.preview = !self.hoopings_ui.preview,
            Cmd::OutputHoopings => self.output_hoopings_dialog(),
            Cmd::ThreadsPanel => self.focus_threads = true,
            Cmd::MatchChart => self.match_to_chart(),
            Cmd::ImportChart => self.import_chart_dialog(),
            Cmd::RemoveUnused => self.remove_unused_threads(),
            Cmd::CycleColors => self.cycle_colors(),
            Cmd::SplitThread => self.split_thread(),
            Cmd::NewColorway => self.new_colorway(),
            Cmd::DeleteColorway => self.delete_colorway(),
            Cmd::Func(f) => self.toggle_func(f),
            Cmd::StartSketch => self.start_sketch(),
            Cmd::Sketch(SketchCmd::Finish) => self.finish_sketch(),
            Cmd::Sketch(SketchCmd::Construction) => self.toggle_construction(),
            Cmd::Sketch(SketchCmd::Relax) => self.relax_sketch(),
            Cmd::Sketch(SketchCmd::Centerline) => self.toggle_centerline(),
            Cmd::Sketch(SketchCmd::CenterPoint) => self.toggle_center_point(),
            Cmd::Sketch(SketchCmd::AutoDimension) => self.auto_dimension(),
            Cmd::Sketch(SketchCmd::Parameters) => self.toggle_parameters(),
            Cmd::Sketch(SketchCmd::RelaxMode) => {
                if let Some(e) = &mut self.sketch {
                    e.relax_mode = !e.relax_mode;
                }
            }
            Cmd::Sketch(SketchCmd::Tool(t)) => self.set_sketch_tool(t),
            Cmd::SplitAtNeedle => self.split_at_needle(),
            Cmd::StitchListPanel => self.focus_stitch_list = true,
            Cmd::Fabric => self.open_fabric_dialog(),
            Cmd::SewTimes => self.show_sew_times = true,
            Cmd::Needles => self.show_needles = true,
            Cmd::Quote => self.show_quote = true,
            Cmd::OrderDetails => self.show_order = true,
            Cmd::ThreadStock => self.show_stock = true,
            Cmd::Scripts => self.show_scripts = true,
            Cmd::Fonts => self.fonts_window.open = true,
            Cmd::LetteringOnLine => self.lettering_on_line(),
            Cmd::Monogram => self.open_monogram(),
            Cmd::TeamNames => self.open_team_names(),
            Cmd::Alphabets => self.alphabets_window.open = true,
            Cmd::KeyboardDesigns => self.keyboard_window.open = true,
            Cmd::QuickSimplify => self.quick_simplify(),
            Cmd::ExportSvg => self.export_svg_dialog(),
            Cmd::Simplify => self.open_simplify(),
            Cmd::CloseGaps => self.open_close_gaps(),
            Cmd::CopyProperties => self.start_copy_properties(),
            Cmd::Envelope => self.open_envelope(),
            Cmd::RepeatLast => {
                if let Some(c) = self.last_cmd
                    && c != Cmd::RepeatLast
                {
                    self.run(c, ctx);
                }
            }
            Cmd::CleanUp => self.open_cleanup(),
            Cmd::CornerRadius => self.open_corner_dialog(),
            Cmd::HoleFromOutline => self.open_hole_dialog(),
            Cmd::PreviousView => self.previous_view(),
            Cmd::ConvertToApplique => self.convert_to_applique(),
            Cmd::ConvertToCross => self.convert_to_cross(),
            Cmd::CrossChart => self.cross_chart_dialog(),
            Cmd::ToStonesRun => self.convert_to_stones(crate::bling::ToStones::Outline),
            Cmd::ToStonesCenter => self.convert_to_stones(crate::bling::ToStones::CenterLine),
            Cmd::StoneTemplate => self.stone_template_dialog(),
            Cmd::TemplateMarks => self.add_template_marks(),
            Cmd::ToStonesFill => self.convert_to_stones(crate::bling::ToStones::Fill),
            Cmd::PictureToCross => {
                self.picture_to_design();
                if let Some(w) = self.wizard.as_mut() {
                    w.make = crate::wizard::Make::Cross;
                }
            }
            Cmd::CombineApplique => self.combine_applique(),
            Cmd::AppliqueFromSelection => self.applique_from_selection(),
            Cmd::BackToOriginal => self.applique_back_to_original(),
            Cmd::MakeCurrent => self.make_properties_current(),
            Cmd::ApplyCurrent => self.apply_current_properties(),
            Cmd::StylesPanel => self.focus_styles = true,
            Cmd::NewFromTemplate => self.open_template_dialog(false),
            Cmd::SaveAsTemplate => self.open_template_dialog(true),
            Cmd::InsertMachineFile => self.insert_machine_file_dialog(),
            Cmd::OutputDesign => self.open_output_dialog(),
            Cmd::PrintPreview => self.preview = Some(Default::default()),
            Cmd::BatchOutput => self.batch_output_dialog(),
        }
    }
}
