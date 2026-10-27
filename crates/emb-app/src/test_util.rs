//! Test harness: runs the canvas headless and sends real pointer and key events.

use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
use emb_geom::Point;
use emb_model::{HoopCatalog, MachineCatalog, ObjectId};

use crate::app::AppState;
use crate::canvas;
use crate::manipulate::ViewMap;

pub const SCREEN: Vec2 = Vec2::new(1000.0, 700.0);

pub struct Harness {
    pub ctx: egui::Context,
    pub st: AppState,
}

impl Harness {
    pub fn new() -> Self {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
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
