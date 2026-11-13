//! Tests of the ribbon.

use eframe::egui::{Event, Modifiers, PointerButton, Pos2, RawInput};
use emb_model::{HoopCatalog, MachineCatalog};

use super::*;

fn all_cmds() -> Vec<Cmd> {
    let mut v = Vec::new();
    for tab in RibbonTab::ALL {
        for p in tab.panels() {
            for item in p.items {
                match item {
                    Large(c) => v.push(*c),
                    Small(cs) | Custom(_, cs) => v.extend_from_slice(cs),
                }
            }
        }
    }
    v.extend(QUICK);
    v
}

struct Rib {
    ctx: egui::Context,
    st: AppState,
    /// A very wide screen (no panel folds), else `width`.
    wide: bool,
    width: f32,
}

impl Rib {
    fn new() -> Self {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        let ctx = egui::Context::default();
        icons::install(&ctx);
        Rib {
            ctx,
            st,
            wide: true,
            width: 1600.0,
        }
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                vec2(if self.wide { 4000.0 } else { self.width }, 900.0),
            )),
            events,
            ..Default::default()
        };
        let st = &mut self.st;
        let mut out = self.ctx.run_ui(input, |ui| {
            egui::Panel::top("ribbon").show(ui, |ui| show(ui, st));
        });
        out.textures_delta.clear();
    }

    fn rect(&self, label: &str) -> Rect {
        RECTS.with(|r| {
            r.borrow()
                .iter()
                .find(|(l, _)| l == label)
                .map(|(_, r)| *r)
                .unwrap_or_else(|| panic!("no button {label}"))
        })
    }

    fn click(&mut self, label: &str) {
        let p = self.rect(label).center();
        let ev = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(p)]);
        self.frame(vec![ev(true)]);
        self.frame(vec![ev(false)]);
    }
}

/// No ribbon tab needs the scroll bar in a window 1366 points wide (a
/// common laptop screen): the panels that do not fit fold into one
/// button each, and a click on it shows the panel.
#[test]
fn simple_mode_has_fewer_tabs_and_large_buttons() {
    let mut r = Rib::new();
    r.st.save_settings = false;
    r.st.settings.ribbon_tab = RibbonTab::Artwork;
    r.frame(vec![]);
    r.click("Help");
    r.frame(vec![]);
    r.click("Simple Mode");
    assert!(r.st.settings.simple_mode);
    // A tab that simple mode does not have: the first tab shows.
    r.st.settings.ribbon_tab = RibbonTab::Artwork;
    r.st.run(Cmd::Toggle(Flag::SimpleMode), &r.ctx.clone());
    r.st.run(Cmd::Toggle(Flag::SimpleMode), &r.ctx.clone());
    assert_eq!(r.st.settings.ribbon_tab, RibbonTab::Digitize);
    r.wide = false;
    r.width = 1366.0;
    for &tab in RibbonTab::visible(true) {
        r.st.settings.ribbon_tab = tab;
        for _ in 0..3 {
            r.frame(vec![]);
        }
        let w = PANELS_W.with(|w| w.get());
        assert!(w <= 1366.0, "the simple {} tab is {w} points wide", tab.label());
        // Only large buttons, and each is on the full ribbon too (for the help).
        for p in tab.panels_in(true) {
            for item in p.items {
                match item {
                    Large(c) => assert!(tab_of(*c).is_some(), "{c:?}"),
                    // The Help tab is the same in both modes.
                    Small(cs) if tab != RibbonTab::Help => panic!("small buttons in simple mode: {cs:?}"),
                    Small(_) => {}
                    Custom(..) => {}
                }
            }
        }
    }
    // The tab row has the simple tabs only.
    r.frame(vec![]);
    assert!(RECTS.with(|x| x.borrow().iter().all(|(l, _)| l != "Artwork")));
    r.click("View");
    r.frame(vec![]);
    r.click("Simple Mode");
    assert!(!r.st.settings.simple_mode);
}

/// The names that the screen reader gets in one frame.
fn reader_names(r: &mut Rib) -> Vec<String> {
    r.ctx.enable_accesskit();
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(4000.0, 900.0))),
        ..Default::default()
    };
    let st = &mut r.st;
    let mut out = r.ctx.run_ui(input, |ui| {
        egui::Panel::top("ribbon").show(ui, |ui| show(ui, st));
        egui::CentralPanel::default().show(ui, |ui| crate::canvas::show(ui, st));
    });
    out.textures_delta.clear();
    let update = out.platform_output.accesskit_update.expect("the screen reader tree");
    update
        .nodes
        .iter()
        .filter_map(|(_, n)| n.label().map(str::to_string))
        .collect()
}

#[test]
fn a_screen_reader_reads_the_ribbon_and_the_canvas() {
    let mut r = Rib::new();
    r.frame(vec![]);
    let names = reader_names(&mut r);
    // The tabs, the quick buttons and the buttons of the Digitize tab,
    // which the program paints itself.
    for want in ["Digitize", "Lettering", "Undo", "Save"] {
        assert!(names.iter().any(|n| n == want), "{want} in {names:?}");
    }
    for (cmd, tab, _) in all_commands() {
        if tab == RibbonTab::Digitize {
            let label = cmd.meta().1;
            assert!(names.iter().any(|n| n == label), "{label}");
        }
    }
    assert!(
        names.iter().any(|n| n.starts_with("Design canvas:")),
        "the canvas has a name"
    );
}

#[test]
fn f6_puts_the_keyboard_focus_on_the_ribbon() {
    let mut r = Rib::new();
    r.frame(vec![]);
    assert!(focus_ribbon(&r.ctx));
    r.frame(vec![]);
    let first = r.ctx.memory(|m| m.focused()).expect("a ribbon button has the focus");
    // Tab goes to the next button.
    let tab = |pressed| Event::Key {
        key: egui::Key::Tab,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    r.frame(vec![tab(true), tab(false)]);
    r.frame(vec![]);
    let next = r.ctx.memory(|m| m.focused()).expect("the focus stays in the ribbon");
    assert_ne!(first, next);
}

#[test]
fn every_tab_fits_a_laptop_screen_in_every_language() {
    // Longer German labels fold more panels, but every tab still fits.
    for code in ["de", "es", "ja"] {
        crate::i18n::set_language(code);
        let mut r = Rib::new();
        r.wide = false;
        r.width = 1366.0;
        for tab in RibbonTab::ALL {
            r.st.settings.ribbon_tab = tab;
            for _ in 0..3 {
                r.frame(vec![]);
            }
            let w = PANELS_W.with(|w| w.get());
            assert!(w <= 1366.0, "{code}: the {} tab is {w} points wide", tab.label());
        }
    }
    crate::i18n::set_language("en");
}

#[test]
fn the_sketch_tab_buttons_take_clicks_at_every_width() {
    for width in [1366.0, 1456.0, 1600.0, 1920.0] {
        let mut r = Rib::new();
        r.st.save_settings = false;
        r.wide = false;
        r.width = width;
        r.st.selection.clear();
        r.st.start_sketch();
        for _ in 0..3 {
            r.frame(vec![]);
        }
        let label = tr("Trim");
        if RECTS.with(|x| x.borrow().iter().any(|(l, _)| l == label)) {
            r.click(label);
            assert_eq!(
                r.st.sketch.as_ref().unwrap().tool,
                crate::sketch::SketchTool::Trim,
                "width {width}"
            );
        }
    }
}

#[test]
fn every_tab_fits_a_laptop_screen() {
    let mut r = Rib::new();
    r.wide = false;
    r.width = 1366.0;
    for tab in RibbonTab::ALL {
        r.st.settings.ribbon_tab = tab;
        // The first frame measures the panels; the next ones fold.
        for _ in 0..3 {
            r.frame(vec![]);
        }
        let w = PANELS_W.with(|w| w.get());
        assert!(w <= 1366.0, "the {} tab is {w} points wide", tab.label());
    }
    // A folded panel opens and its buttons work.
    r.st.settings.ribbon_tab = RibbonTab::Artwork;
    for _ in 0..3 {
        r.frame(vec![]);
    }
    let folded = folded_panels(&r.st, RibbonTab::Artwork, 1366.0);
    let k = folded
        .iter()
        .rposition(|f| *f)
        .expect("a panel of the Artwork tab folds");
    r.click(&folded_label(&ARTWORK[k]));
    let (_, label, _) = match ARTWORK[k].items[0] {
        Large(c) => c.meta(),
        Small(cs) => cs[0].meta(),
        Custom(..) => unreachable!(),
    };
    r.frame(vec![]);
    assert!(
        RECTS.with(|x| x.borrow().iter().any(|(l, _)| l == label)),
        "{label} shows in the open panel"
    );
    // A wide window folds nothing.
    assert!(folded_panels(&r.st, RibbonTab::Artwork, 4000.0).iter().all(|f| !f));
}

/// Two commands of one tab with the same icon look like one command.
/// The tutorials share the icon of a tutorial; the 2× zoom buttons show
/// text, not their icon.
#[test]
fn the_commands_of_a_tab_have_different_icons() {
    let mut same = Vec::new();
    for tab in RibbonTab::ALL {
        let mut seen: Vec<(Icon, Cmd)> = Vec::new();
        for (cmd, t, _) in all_commands() {
            if t != tab
                || seen.iter().any(|(_, c)| *c == cmd)
                || matches!(cmd, Cmd::HelpTopic(id) if id.starts_with("tutorial-"))
                || matches!(cmd, Cmd::ZoomTwice(_))
            {
                continue;
            }
            let icon = cmd.meta().0;
            if let Some((_, other)) = seen.iter().find(|(i, _)| *i == icon) {
                same.push(format!("{}: {:?} and {:?}", tab.label(), other, cmd));
            }
            seen.push((icon, cmd));
        }
    }
    assert!(same.is_empty(), "{}", same.join("\n"));
}

/// The name of a large button (also of a folded panel) has two lines at
/// most in each language, so it does not go over the panel title (owner
/// report of 2026-10-26).
#[test]
fn large_button_names_have_two_lines_at_most() {
    let ctx = egui::Context::default();
    for code in ["en", "de", "es", "ja"] {
        crate::i18n::set_language(code);
        let mut long = Vec::new();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let mut labels: Vec<String> = Vec::new();
            for tab in RibbonTab::ALL {
                for simple in [false, true] {
                    for p in tab.panels_in(simple) {
                        labels.push(folded_label(p));
                        for item in p.items {
                            if let Large(c) = item {
                                labels.push(c.meta().1.to_string());
                            }
                        }
                    }
                }
            }
            for label in labels {
                let (w, size) = large_fit(ui, &label);
                let rows = ui
                    .painter()
                    .layout(label.clone(), FontId::proportional(size), Color32::WHITE, w - 4.0)
                    .rows
                    .len();
                if rows > 2 {
                    long.push(format!("{code}: {label} ({rows} lines at {w})"));
                }
            }
        });
        out.textures_delta.clear();
        assert!(
            long.is_empty(),
            "{}",
            long.join(
                "
"
            )
        );
    }
    crate::i18n::set_language("en");
}

#[test]
fn every_tool_is_on_the_ribbon() {
    let cmds = all_cmds();
    for tool in Tool::ALL {
        assert!(cmds.contains(&Cmd::Tool(tool)), "{tool:?} is not on the ribbon");
    }
}

#[test]
fn every_command_runs_on_a_selection() {
    let mut r = Rib::new();
    let ctx = r.ctx.clone();
    let (start, steps) = (r.st.doc.design().clone(), r.st.doc.undo_len());
    let mut changed = 0;
    for cmd in all_cmds() {
        if cmd.opens_dialog() {
            continue;
        }
        r.st.select_all();
        let s = SelInfo::of(&r.st);
        if r.st.can(cmd, &s) {
            r.st.run(cmd, &ctx);
        }
        r.st.set_tool(Tool::Select);
        // Each change is one undo step, and undo gives the start design again.
        if r.st.doc.undo_len() > steps {
            changed += 1;
            assert_eq!(r.st.doc.undo_len(), steps + 1, "{cmd:?}");
            r.st.undo();
            assert!(r.st.doc.design().same_content(&start), "{cmd:?}");
        }
    }
    assert!(changed > 20, "only {changed} commands changed the design");
}

#[test]
fn every_tab_draws() {
    let mut r = Rib::new();
    r.st.select_all();
    for tab in RibbonTab::ALL {
        // The Sketch tab shows only in sketch mode.
        if tab == RibbonTab::Sketch {
            r.st.start_sketch();
        }
        r.st.settings.ribbon_tab = tab;
        r.frame(vec![]);
        r.frame(vec![]);
        let (_, first, _) = match tab.panels()[0].items[0] {
            Large(c) => c.meta(),
            Small(cs) => cs[0].meta(),
            Custom(..) => continue,
        };
        r.rect(first);
    }
}

#[test]
fn click_a_tab_and_a_tool() {
    let mut r = Rib::new();
    r.frame(vec![]);
    r.click("Arrange");
    assert_eq!(r.st.settings.ribbon_tab, RibbonTab::Arrange);
    r.click("Digitize");
    r.click("Ellipse");
    assert_eq!(r.st.tool, Tool::Ellipse);
    // A disabled command does nothing: no selection, so no Knife.
    r.st.selection.clear();
    r.click("Edit");
    r.click("Knife");
    assert_eq!(r.st.tool, Tool::Ellipse);
}

#[test]
fn view_buttons_toggle() {
    let mut r = Rib::new();
    r.frame(vec![]);
    r.click("View");
    let grid = r.st.settings.show_grid;
    r.click("Grid");
    assert_eq!(r.st.settings.show_grid, !grid);
    r.click("Stitch Cursor");
    assert!(r.st.cursor.is_some());
    r.click("Stitch Cursor");
    assert!(r.st.cursor.is_none());
}

#[test]
fn collapse_and_expand() {
    let mut r = Rib::new();
    r.frame(vec![]);
    r.click("Collapse");
    assert!(r.st.settings.ribbon_collapsed);
    // The panels are gone; a click on a tab brings them back.
    assert!(RECTS.with(|x| x.borrow().iter().all(|(l, _)| l != "Select All")));
    r.click("Edit");
    assert!(!r.st.settings.ribbon_collapsed);
    assert_eq!(r.st.settings.ribbon_tab, RibbonTab::Edit);
    r.frame(vec![]);
    r.rect("Select All");
}

#[test]
fn undo_from_the_quick_access_buttons() {
    let mut r = Rib::new();
    r.frame(vec![]);
    let n = r.st.doc.design().objects().len();
    r.click("Undo");
    assert_eq!(r.st.doc.design().objects().len(), 0, "sample objects are one undo step");
    r.click("Redo");
    assert_eq!(r.st.doc.design().objects().len(), n);
}
