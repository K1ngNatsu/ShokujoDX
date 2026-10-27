//! Command windows dock at the corners of the canvas (owner request of
//! 2026-10-26, ADR 0057), as the command windows of CAD programs do.
//!
//! - A command window opens docked at the top left corner of the canvas.
//! - Drag it by its title. Release it near a corner of the canvas: it docks
//!   there. Release it at another place: it stays where it is (free).
//! - A docked window follows its corner when the canvas changes size (a
//!   panel opens, the ribbon folds, the window of the program changes).
//! - Two windows docked at one corner go one under the other (one over the
//!   other at a bottom corner), in the order that they draw.
//!
//! The dock of each window is kept in the memory of egui, by the id of the
//! window, so it stays while the program runs. A command window moves when
//! you drag its title or an empty place in it (egui `WindowDrag::Anywhere`):
//! in the title-bar mode, egui 0.36 keeps the position of the last frame
//! and a docked window could not follow its corner.

use eframe::egui::{self, Align2, Id, Pos2, Rect, Vec2};

/// The space between a docked window and the edge of the canvas, points.
pub const MARGIN: f32 = 8.0;
/// A window that is released this near a corner of the canvas docks there,
/// points (from the corner of the window to the corner of the canvas).
pub const SNAP: f32 = 60.0;
/// The space between two windows docked at one corner, points.
const GAP: f32 = 6.0;

/// Where a command window is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dock {
    /// Docked at a corner of the canvas.
    Corner(Align2),
    /// Where the user put it.
    Free,
}

/// The corners that a window can dock at.
pub const CORNERS: [Align2; 4] = [
    Align2::LEFT_TOP,
    Align2::RIGHT_TOP,
    Align2::LEFT_BOTTOM,
    Align2::RIGHT_BOTTOM,
];

/// The dock of a window that the user released with its rect `window` on
/// the canvas `canvas`: the corner that is near enough, else free.
pub fn snap(window: Rect, canvas: Rect) -> Dock {
    let inner = canvas.shrink(MARGIN);
    CORNERS
        .into_iter()
        .map(|c| (c, c.pos_in_rect(&window).distance(c.pos_in_rect(&inner))))
        .filter(|(_, d)| *d <= SNAP)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(Dock::Free, |(c, _)| Dock::Corner(c))
}

/// The state of the docks in the memory of egui.
#[derive(Debug, Clone, Default)]
struct Docks {
    /// The dock of each window; a window with none is docked at the top
    /// left.
    dock: Vec<(Id, Dock)>,
    /// The windows that a drag moved in the last frame.
    dragged: Vec<Id>,
    /// The frame of `placed`.
    frame: u64,
    /// The windows docked in this frame: their corner and size.
    placed: Vec<(Align2, Vec2)>,
}

fn docks_id() -> Id {
    Id::new("command window docks")
}

/// The dock of the window `id` now.
#[cfg(test)]
pub fn dock_of(ctx: &egui::Context, id: Id) -> Dock {
    ctx.data(|d| d.get_temp::<Docks>(docks_id()))
        .and_then(|s| s.dock.iter().find(|(i, _)| *i == id).map(|(_, d)| *d))
        .unwrap_or(Dock::Corner(Align2::LEFT_TOP))
}

/// Sets where `window` (with the id `id`) goes in this frame: at its
/// corner of `canvas` if it is docked, else where it is. Call it on each
/// command window before `show`, in the same order in each frame.
pub fn docked<'a>(window: egui::Window<'a>, ctx: &egui::Context, id: Id, canvas: Option<Rect>) -> egui::Window<'a> {
    let window = window.drag_area(egui::WindowDrag::Anywhere);
    let Some(canvas) = canvas else {
        return window;
    };
    let mut s = ctx.data(|d| d.get_temp::<Docks>(docks_id())).unwrap_or_default();
    let frame = ctx.cumulative_frame_nr();
    if s.frame != frame {
        s.frame = frame;
        s.placed.clear();
    }
    // A drag of the title moves the window; at its end the window docks or
    // is free.
    let dragging = ctx.is_being_dragged(id.with("move"));
    let was_dragging = s.dragged.contains(&id);
    s.dragged.retain(|i| *i != id);
    if dragging {
        s.dragged.push(id);
    }
    let rect = ctx.memory(|m| m.area_rect(id));
    if was_dragging
        && !dragging
        && let Some(r) = rect
    {
        let dock = snap(r, canvas);
        s.dock.retain(|(i, _)| *i != id);
        s.dock.push((id, dock));
    }
    let dock = s
        .dock
        .iter()
        .find(|(i, _)| *i == id)
        .map_or(Dock::Corner(Align2::LEFT_TOP), |(_, d)| *d);
    let window = match dock {
        Dock::Corner(c) if !dragging => {
            // Under (or over) the windows docked at this corner before it.
            // A window that did not show in the last frame (closed, or just
            // opened) takes no place in the stack yet.
            let shown = ctx.memory(|m| {
                m.areas()
                    .visible_last_frame(&egui::LayerId::new(egui::Order::Middle, id))
            });
            let size = rect.filter(|_| shown).map(|r| r.size());
            let before: f32 = s.placed.iter().filter(|(k, _)| *k == c).map(|(_, v)| v.y + GAP).sum();
            if let Some(size) = size {
                s.placed.push((c, size));
            }
            let corner = c.pos_in_rect(&canvas.shrink(MARGIN));
            let down = if c.y() == egui::Align::Max { -before } else { before };
            window.pivot(c).current_pos(Pos2::new(corner.x, corner.y + down))
        }
        _ => window,
    };
    ctx.data_mut(|d| d.insert_temp(docks_id(), s));
    window
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Event, Modifiers, PointerButton, RawInput, pos2, vec2};

    use super::*;

    const CANVAS: Rect = Rect {
        min: Pos2::new(100.0, 100.0),
        max: Pos2::new(900.0, 700.0),
    };

    #[test]
    fn a_window_near_a_corner_docks_there() {
        let at = |x: f32, y: f32| Rect::from_min_size(pos2(x, y), vec2(200.0, 150.0));
        assert_eq!(snap(at(108.0, 108.0), CANVAS), Dock::Corner(Align2::LEFT_TOP));
        assert_eq!(snap(at(680.0, 120.0), CANVAS), Dock::Corner(Align2::RIGHT_TOP));
        assert_eq!(snap(at(130.0, 520.0), CANVAS), Dock::Corner(Align2::LEFT_BOTTOM));
        assert_eq!(snap(at(692.0, 542.0), CANVAS), Dock::Corner(Align2::RIGHT_BOTTOM));
        assert_eq!(snap(at(400.0, 300.0), CANVAS), Dock::Free, "the middle");
    }

    struct Run {
        ctx: egui::Context,
        events: Vec<Event>,
    }

    impl Run {
        /// One frame with the windows `ids` (each 200 points wide), docked.
        fn frame(&mut self, ids: &[&str]) {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 800.0))),
                events: std::mem::take(&mut self.events),
                ..Default::default()
            };
            let mut out = self.ctx.run_ui(input, |ui| {
                let ctx = ui.ctx().clone();
                for name in ids {
                    let id = Id::new(*name);
                    let w = egui::Window::new(*name).id(id).resizable(false);
                    docked(w, &ctx, id, Some(CANVAS)).show(&ctx, |ui| {
                        ui.set_min_size(vec2(200.0, 100.0));
                    });
                }
            });
            out.textures_delta.clear();
        }

        fn rect(&self, name: &str) -> Rect {
            self.ctx.memory(|m| m.area_rect(Id::new(name))).unwrap()
        }

        /// Drags the title of window `name` by `by`, in a few frames.
        fn drag(&mut self, name: &str, by: Vec2, ids: &[&str]) {
            let from = self.rect(name).left_top() + vec2(30.0, 8.0);
            let button = |pos, pressed| Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            self.events = vec![Event::PointerMoved(from)];
            self.frame(ids);
            self.events = vec![button(from, true)];
            self.frame(ids);
            for k in 1..=4 {
                self.events = vec![Event::PointerMoved(from + by * (k as f32 / 4.0))];
                self.frame(ids);
            }
            self.events = vec![button(from + by, false)];
            self.frame(ids);
            self.frame(ids);
        }
    }

    #[test]
    fn command_windows_dock_stack_and_float() {
        let mut r = Run {
            ctx: egui::Context::default(),
            events: Vec::new(),
        };
        let ids = ["One", "Two"];
        for _ in 0..3 {
            r.frame(&ids);
        }
        // Both open at the top left corner, one under the other.
        let (one, two) = (r.rect("One"), r.rect("Two"));
        assert_eq!(one.left_top(), CANVAS.left_top() + vec2(MARGIN, MARGIN));
        assert_eq!(two.left(), one.left());
        assert!((two.top() - (one.bottom() + GAP)).abs() < 0.5, "{one:?} {two:?}");
        // Drag the second one near the bottom right corner: it docks there.
        let target = CANVAS.right_bottom() - vec2(MARGIN + 20.0, MARGIN + 20.0);
        let by = target - r.rect("Two").right_bottom();
        r.drag("Two", by, &ids);
        assert_eq!(dock_of(&r.ctx, Id::new("Two")), Dock::Corner(Align2::RIGHT_BOTTOM));
        let two = r.rect("Two");
        assert!(
            (two.right_bottom() - (CANVAS.right_bottom() - vec2(MARGIN, MARGIN))).length() < 0.5,
            "{two:?}"
        );
        // The first one is alone at the top left now.
        assert_eq!(r.rect("One").left_top(), CANVAS.left_top() + vec2(MARGIN, MARGIN));
        // Drag it to the middle: it is free and stays there.
        r.drag("One", vec2(250.0, 200.0), &ids);
        assert_eq!(dock_of(&r.ctx, Id::new("One")), Dock::Free);
        let free = r.rect("One");
        assert!((free.left_top() - (CANVAS.left_top() + vec2(MARGIN + 250.0, MARGIN + 200.0))).length() < 1.0);
        r.frame(&ids);
        assert_eq!(r.rect("One"), free);
    }
}
