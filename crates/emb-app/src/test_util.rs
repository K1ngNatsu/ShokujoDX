//! Test harness: runs the canvas headless and sends real pointer and key events.

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use emb_geom::Point;
use emb_model::{HoopCatalog, MachineCatalog, ObjectId};

use crate::app::AppState;
use crate::canvas;
use crate::manipulate::ViewMap;

pub const SCREEN: Vec2 = Vec2::new(1000.0, 700.0);

/// The canvas of the harness: all of the screen.
pub fn screen_rect() -> Rect {
    Rect::from_min_size(Pos2::ZERO, SCREEN)
}

/// A context after one empty frame, so it has its fonts. On macOS,
/// `Context::format_shortcut` reads the fonts (to show ⌘ and ⌥), and a new
/// context has none until its first frame.
pub fn ready_context() -> egui::Context {
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(RawInput::default(), |_| {});
    out.textures_delta.clear();
    ctx
}

pub struct Harness {
    pub ctx: egui::Context,
    pub st: AppState,
}

impl Harness {
    pub fn new() -> Self {
        // The tests of the harness are on the reference machine.
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.start_on_machine("husqvarna_topaz20", None);
        st.add_sample_objects();
        st.selection.clear();
        let mut h = Harness {
            ctx: egui::Context::default(),
            st,
        };
        h.frame(vec![]); // The first frame fits the view.
        h
    }

    pub fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN)),
            events,
            ..Default::default()
        };
        let st = &mut self.st;
        let mut out = self.ctx.run_ui(input, |ui| {
            // As in the app: shortcuts first, then the canvas.
            st.handle_shortcuts(&ui.ctx().clone());
            egui::CentralPanel::no_frame().show(ui, |ui| canvas::show(ui, st));
            crate::palette::window(&ui.ctx().clone(), st);
        });
        out.textures_delta.clear();
    }

    pub fn map(&self) -> ViewMap {
        ViewMap::new(
            Rect::from_min_size(Pos2::ZERO, SCREEN),
            self.st.view,
            self.st.doc.design(),
        )
    }

    pub fn screen(&self, p: Point) -> Pos2 {
        self.map().to_screen(p)
    }

    /// Press at `from`, move in steps to `to`, release.
    pub fn drag(&mut self, button: PointerButton, from: Pos2, to: Pos2, mods: Modifiers) {
        let ev = |pos, pressed| Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: mods,
        };
        self.frame(vec![Event::ModifiersChanged(mods), Event::PointerMoved(from)]);
        self.frame(vec![ev(from, true)]);
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            self.frame(vec![Event::PointerMoved(from + (to - from) * t)]);
        }
        self.frame(vec![ev(to, false)]);
        self.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
    }

    /// Two fingers on a touch screen: from `a.0` and `b.0` to `a.1` and
    /// `b.1` in steps, then up. As in eframe on the web, the first finger
    /// also moves and presses the pointer.
    pub fn pinch(&mut self, a: (Pos2, Pos2), b: (Pos2, Pos2)) {
        use egui::{TouchDeviceId, TouchId, TouchPhase};
        let touch = |id: u64, phase, pos| Event::Touch {
            device_id: TouchDeviceId(1),
            id: TouchId(id),
            phase,
            pos,
            force: None,
        };
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![
            Event::PointerMoved(a.0),
            button(a.0, true),
            touch(0, TouchPhase::Start, a.0),
            touch(1, TouchPhase::Start, b.0),
        ]);
        // egui starts a gesture of two fingers in the frame after they are
        // down; real fingers move in small steps from there.
        self.frame(vec![touch(0, TouchPhase::Move, a.0), touch(1, TouchPhase::Move, b.0)]);
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            let (pa, pb) = (a.0 + (a.1 - a.0) * t, b.0 + (b.1 - b.0) * t);
            self.frame(vec![
                Event::PointerMoved(pa),
                touch(0, TouchPhase::Move, pa),
                touch(1, TouchPhase::Move, pb),
            ]);
        }
        self.frame(vec![touch(1, TouchPhase::End, b.1)]);
        self.frame(vec![
            button(a.1, false),
            touch(0, TouchPhase::End, a.1),
            Event::PointerGone,
        ]);
        self.frame(vec![]);
    }

    pub fn click(&mut self, button: PointerButton, at: Pos2) {
        let ev = |pressed| Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![ev(true)]);
        self.frame(vec![ev(false)]);
    }

    pub fn id(&self, i: usize) -> ObjectId {
        self.st.doc.design().objects()[i].id
    }

    pub fn center_of(&self, id: ObjectId) -> Point {
        self.st.doc.design().bbox_of(&[id]).unwrap().center()
    }
}

impl Harness {
    /// A harness with an empty design (no sample objects).
    pub fn empty() -> Self {
        let mut h = Self::new();
        h.st.new_design();
        h.frame(vec![]);
        h
    }

    /// A key with modifiers, down and up. The Find a command window draws too.
    pub fn press(&mut self, modifiers: Modifiers, key: Key) {
        let ev = |pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        self.frame(vec![Event::ModifiersChanged(modifiers), ev(true)]);
        self.frame(vec![ev(false), Event::ModifiersChanged(Modifiers::NONE)]);
    }

    pub fn key(&mut self, key: Key) {
        let ev = |pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![ev(true)]);
        self.frame(vec![ev(false)]);
    }
}
