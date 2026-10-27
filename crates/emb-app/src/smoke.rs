//! Smoke test of the whole user interface: each corpus design, with each
//! object selected in turn, draws all panels and windows. Drawing must not
//! panic, and it must not change the design (a panel that writes a value
//! back each frame makes an undo step with no user action).

use eframe::egui;
use emb_model::ObjectId;

use crate::app::AppState;
use crate::test_util::{Harness, SCREEN};

/// Draws the panels of the app (as `EmbApp::ui`, with plain side panels in
/// place of the dock) and all windows that are open.
fn draw_all(h: &mut Harness) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
        ..Default::default()
    };
    let st: &mut AppState = &mut h.st;
    let mut out = h.ctx.run_ui(input, |ui| {
        let ctx = ui.ctx().clone();
        egui::Panel::top("ribbon").show(ui, |ui| crate::ribbon::show(ui, st));
        egui::Panel::top("property_bar").show(ui, |ui| crate::workspace::property_bar(ui, st));
        egui::Panel::top("design_tabs").show(ui, |ui| crate::tabs::strip(ui, st));
        egui::Panel::bottom("status").show(ui, |ui| st.status_bar(ui));
        egui::Panel::bottom("threads").show(ui, |ui| ui.horizontal(|ui| st.thread_strip(ui)));
        egui::Panel::bottom("player").show(ui, |ui| crate::player::bar(ui, st));
        egui::Panel::left("properties").show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| crate::panels::properties(ui, st))
        });
        egui::Panel::left("sequence").show(ui, |ui| crate::sequence_panel::show(ui, st));
        egui::Panel::right("threads_panel").show(ui, |ui| crate::threads::panel(ui, st));
        egui::Panel::right("stitch_list").show(ui, |ui| crate::functions::stitch_list(ui, st));
        egui::Panel::right("info").show(ui, |ui| crate::info::panel(ui, st));
        egui::Panel::right("styles").show(ui, |ui| crate::styles::panel(ui, st));
        egui::Panel::right("split").show(ui, |ui| crate::tabs::split_canvas(ui, st));
        egui::CentralPanel::no_frame().show(ui, |ui| crate::canvas::show(ui, st));
        crate::workspace::options_window(&ctx, st);
        crate::overview::window(&ctx, st);
        crate::output::windows(&ctx, st);
        crate::styles::windows(&ctx, st);
        crate::setup::sew_time_window(&ctx, st);
        crate::needles::window(&ctx, st);
        crate::quote::window(&ctx, st);
        crate::order::window(&ctx, st);
        crate::stock::window(&ctx, st);
        crate::scripts::window(&ctx, st);
        crate::corners::windows(&ctx, st);
        crate::simplify::window(&ctx, st);
        crate::lettering::windows(&ctx, st);
        crate::fonts::window(&ctx, st);
        crate::monogram::window(&ctx, st);
        crate::motifs::window(&ctx, st);
        crate::decor::windows(&ctx, st);
        crate::artwork::windows(&ctx, st);
        crate::wizard::window(&ctx, st);
        crate::teamnames::window(&ctx, st);
        crate::keyboard::window(&ctx, st);
        crate::alphabets::window(&ctx, st);
        crate::editing::paste_special_window(&ctx, st);
        crate::help::window(&ctx, st);
        crate::assets::window(&ctx, st);
    });
    out.textures_delta.clear();
}

/// Opens the windows that do not need a selection.
fn open_windows(st: &mut AppState) {
    st.show_options = true;
    st.help.open = true;
    st.paste_special = Some(crate::editing::PasteSpecial {
        how: Default::default(),
    });
    st.show_overview = true;
    st.show_sew_times = true;
    st.show_quote = true;
    st.show_order = true;
    st.show_stock = true;
    st.show_scripts = true;
    st.show_split = true;
    st.player.shown = true;
    st.motifs_window.open = true;
    st.team_window.open = true;
    st.keyboard_window.open = true;
    st.decor.blends = true;
    st.decor.buttonholes = true;
    st.decor.borders = true;
    st.artwork.offsets = true;
    st.artwork.arrangements = true;
    st.artwork.trace = true;
    st.open_wizard();
}

/// Prints the lines of the objects that changed, to find the panel that
/// made the change.
fn print_changes(before: &emb_model::Design, after: &emb_model::Design) {
    for o in before.objects() {
        let Some(n) = after.object(o.id) else { continue };
        let (a, b) = (format!("{:#?}", o.kind), format!("{:#?}", n.kind));
        for (x, y) in a.lines().zip(b.lines()) {
            if x != y {
                println!("{:?}: {} -> {}", o.id, x.trim(), y.trim());
            }
        }
    }
}

/// One frame of the bottom bars and the canvas; returns the canvas size.
fn frame_with_bars(h: &mut Harness, events: Vec<egui::Event>) -> egui::Rect {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
        events,
        ..Default::default()
    };
    let st: &mut AppState = &mut h.st;
    let mut out = h.ctx.run_ui(input, |ui| {
        egui::Panel::bottom("status").show(ui, |ui| st.status_bar(ui));
        egui::Panel::bottom("threads").show(ui, |ui| ui.horizontal(|ui| st.thread_strip(ui)));
        egui::Panel::bottom("player").show(ui, |ui| crate::player::bar(ui, st));
        egui::CentralPanel::no_frame().show(ui, |ui| crate::canvas::show(ui, st));
    });
    out.textures_delta.clear();
    h.st.canvas_rect.expect("canvas drawn")
}

#[test]
fn canvas_keeps_its_size_while_an_object_moves_with_the_player_open() {
    use eframe::egui::{Event, Modifiers, PointerButton};
    let mut h = Harness::new();
    h.st.player.shown = true;
    // The player stops at the end of the sew-out.
    let last = h.st.plan_view().plan.stitches.records.len() - 1;
    h.st.cursor = Some(last);
    let fill = h.st.doc.design().objects()[0].id;
    let from = h.screen(h.st.doc.design().bbox_of(&[fill]).unwrap().center());
    let press = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    frame_with_bars(&mut h, vec![Event::PointerMoved(from)]);
    let size = frame_with_bars(&mut h, vec![press(from, true)]);
    for i in 1..=12 {
        let p = from + egui::vec2(3.0 * i as f32, 2.0 * i as f32);
        let r = frame_with_bars(&mut h, vec![Event::PointerMoved(p)]);
        assert_eq!(r, size, "step {i}");
        // The cursor stays at the end of the plan, which changes length.
        let last = h.st.plan_view().plan.stitches.records.len() - 1;
        assert_eq!(h.st.cursor, Some(last), "step {i}");
    }
}

#[test]
fn every_panel_draws_each_corpus_design_with_no_edit() {
    let mut h = Harness::empty();
    open_windows(&mut h.st);
    for name in emb_cli::corpus::NAMES {
        let design = emb_cli::corpus::build(name).expect("corpus design");
        h.st.doc = emb_model::Document::new(design);
        h.st.selection.clear();
        let ids: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        // Nothing selected, then each object alone (Object Properties shows
        // its settings), then all objects (the shared settings).
        let mut selections: Vec<Vec<ObjectId>> = vec![Vec::new()];
        selections.extend(ids.iter().map(|id| vec![*id]));
        selections.push(ids.clone());
        for sel in selections {
            h.st.selection.set_all(sel.iter().copied());
            // Right-click > Edit opens the Lettering or Monogram window.
            h.st.edit_selection();
            let before = h.st.doc.design().clone();
            let undo = h.st.doc.undo_len();
            draw_all(&mut h);
            draw_all(&mut h);
            if h.st.doc.undo_len() != undo {
                print_changes(&before, h.st.doc.design());
            }
            assert_eq!(
                h.st.doc.undo_len(),
                undo,
                "{name}, selection {sel:?}: drawing made the step {:?}",
                h.st.doc.undo_label()
            );
            assert!(
                h.st.doc.design().same_content(&before),
                "{name}, selection {sel:?}: drawing changed the design"
            );
            h.st.lettering_dialog = None;
            h.st.monogram_window.open = false;
        }
    }
}

#[test]
fn every_panel_draws_in_every_language() {
    // One design with each kind of object, all panels and windows, in each
    // built-in language: no panic, no edit.
    let mut h = Harness::new();
    open_windows(&mut h.st);
    let codes: Vec<String> = crate::i18n::languages().iter().map(|l| l.code.clone()).collect();
    assert_eq!(codes, ["de", "es", "ja"]);
    for code in codes {
        crate::i18n::set_language(&code);
        let ids: Vec<ObjectId> = h.st.doc.design().objects().iter().map(|o| o.id).collect();
        for id in ids {
            h.st.selection.set(id);
            let undo = h.st.doc.undo_len();
            draw_all(&mut h);
            draw_all(&mut h);
            assert_eq!(h.st.doc.undo_len(), undo, "{code}: drawing made an undo step");
        }
    }
    crate::i18n::set_language("en");
}

#[test]
fn sketch_mode_works_with_the_mouse_and_drawing_changes_nothing() {
    use crate::sketch::{Pick, SketchTool};
    use eframe::egui::{Modifiers, PointerButton};
    use emb_geom::Point;
    let mut h = Harness::empty();
    h.st.start_sketch();
    let curves = |h: &Harness| h.st.sketch_obj().unwrap().sketch().curves.len();
    // A rectangle from two clicks.
    h.st.set_sketch_tool(SketchTool::Rectangle);
    let (a, b) = (h.screen(Point::new(-30.0, -20.0)), h.screen(Point::new(30.0, 20.0)));
    h.click(PointerButton::Primary, a);
    h.click(PointerButton::Primary, b);
    assert_eq!(curves(&h), 4);
    // A closed triangle from a chain of lines: the last click on the first point.
    h.st.set_sketch_tool(SketchTool::Line);
    let pts = [Point::new(50.0, 0.0), Point::new(80.0, 0.0), Point::new(65.0, 25.0)];
    for p in pts.iter().chain([&pts[0]]) {
        let at = h.screen(*p);
        h.click(PointerButton::Primary, at);
    }
    let s = h.st.sketch_obj().unwrap().sketch().clone();
    assert_eq!(s.curves.len(), 7);
    assert_eq!(s.points.len(), 7, "the triangle shares its corners");
    assert!(h.st.sketch.as_ref().unwrap().clicks.is_empty(), "the chain is closed");
    // The Dimension tool on the bottom side of the rectangle, then a click
    // on an empty place: a length to type.
    h.st.set_sketch_tool(SketchTool::Dimension);
    let mid = h.screen(Point::new(0.0, -20.0));
    h.click(PointerButton::Primary, mid);
    assert!(h.st.sketch.as_ref().unwrap().value_edit.is_none(), "a line waits");
    let empty = h.screen(Point::new(0.0, -32.0));
    h.click(PointerButton::Primary, empty);
    assert!(h.st.sketch.as_ref().unwrap().value_edit.is_some());
    h.st.sketch.as_mut().unwrap().value_edit = None;
    // Drag the top right corner with the Select tool.
    h.st.set_sketch_tool(SketchTool::Select);
    let corner = h.screen(Point::new(30.0, 20.0));
    let to = h.screen(Point::new(40.0, 30.0));
    h.drag(PointerButton::Primary, corner, to, Modifiers::NONE);
    let s = h.st.sketch_obj().unwrap().sketch().clone();
    assert!(
        s.points.iter().any(|p| p.pos.distance(Point::new(40.0, 30.0)) < 0.5),
        "the corner follows"
    );
    // The width dimension (60 mm) keeps the width: the left side moved.
    let xs: Vec<f64> = s.points.iter().map(|p| p.pos.x).collect();
    assert!(xs.iter().any(|x| (x + 20.0).abs() < 1e-3), "{xs:?}");
    // A click selects a part.
    let edge = h.screen(Point::new(10.0, -20.0));
    h.click(PointerButton::Primary, edge);
    assert!(matches!(
        h.st.sketch.as_ref().unwrap().selection.as_slice(),
        [Pick::Curve(_)]
    ));
    // All panels and windows draw in sketch mode and change nothing.
    open_windows(&mut h.st);
    let before = h.st.doc.state_id();
    draw_all(&mut h);
    draw_all(&mut h);
    assert_eq!(h.st.doc.state_id(), before);
    h.st.finish_sketch();
    assert!(h.st.sketch.is_none());
}
