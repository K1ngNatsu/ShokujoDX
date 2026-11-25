//! A long press of a pen in the browser trial (the owner, 2026-11-25).
//!
//! The page gives the program the events of a pen (an Apple Pencil) as a
//! mouse with one button (`web::pen_filter`), so egui sees no touch, and its
//! long press for the right-click menu (a touch held still) did not come
//! from a pen. [`LongPress`] finds a pen held still for [`LONG_PRESS_MS`];
//! the program then releases the primary button and clicks the secondary
//! button there ([`menu_events`]), so the menu opens as for a right click.
//! The rest of that stroke of the pen is not a press: when it goes up, no
//! primary button goes up again.

use eframe::egui::{Event, Modifiers, PointerButton, Pos2};

/// A pen that is still for this long (ms) opens the right-click menu. It is
/// longer than the longest click of egui (0.8 s), so the release of the
/// primary button that comes first is not a click.
pub const LONG_PRESS_MS: f64 = 900.0;
/// A pen that moves farther than this (points) from where it went down
/// draws or drags: no menu.
pub const LONG_PRESS_MOVE: f32 = 8.0;

/// The long press of one stroke of the pen.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LongPress {
    /// Where and when (ms) the pen went down, while it can still be a long
    /// press.
    start: Option<(f64, Pos2)>,
    /// The menu opened in this stroke.
    fired: bool,
}

impl LongPress {
    /// No stroke.
    pub const fn new() -> Self {
        LongPress {
            start: None,
            fired: false,
        }
    }

    /// The pen went down at `at` at the time `now` (ms).
    pub fn down(&mut self, now: f64, at: Pos2) {
        self.start = Some((now, at));
        self.fired = false;
    }

    /// The pen moved to `at`: far from where it went down, no long press.
    pub fn moved(&mut self, at: Pos2) {
        if self.start.is_some_and(|(_, s)| s.distance(at) > LONG_PRESS_MOVE) {
            self.start = None;
        }
    }

    /// The pen went up, or the system took it. `true` if the menu opened in
    /// this stroke: the primary button is up already.
    pub fn up(&mut self) -> bool {
        self.start = None;
        std::mem::take(&mut self.fired)
    }

    /// `true` while the menu of this stroke is open: the moves of the pen
    /// are not drags.
    pub fn fired(&self) -> bool {
        self.fired
    }

    /// `true` while the pen is down and still, before its time.
    pub fn waiting(&self) -> bool {
        self.start.is_some()
    }

    /// The place of the long press, one time, when the pen was still for
    /// [`LONG_PRESS_MS`] at `now`.
    pub fn due(&mut self, now: f64) -> Option<Pos2> {
        let (t, at) = self.start?;
        if now - t < LONG_PRESS_MS {
            return None;
        }
        self.start = None;
        self.fired = true;
        Some(at)
    }
}

/// The events of a long press at `at`: the primary button goes up, then a
/// click of the secondary button (the right-click menu).
pub fn menu_events(at: Pos2, modifiers: Modifiers) -> [Event; 3] {
    let button = |button, pressed| Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers,
    };
    [
        button(PointerButton::Primary, false),
        button(PointerButton::Secondary, true),
        button(PointerButton::Secondary, false),
    ]
}

#[cfg(test)]
mod tests {
    use eframe::egui::PointerButton;
    use emb_geom::{Path, Point};
    use emb_model::ObjectKind;

    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn a_still_pen_is_a_long_press_and_a_moved_pen_is_not() {
        let mut p = LongPress::new();
        assert_eq!(p, LongPress::default());
        p.down(0.0, Pos2::new(100.0, 100.0));
        p.moved(Pos2::new(104.0, 103.0));
        assert!(p.waiting());
        assert_eq!(p.due(LONG_PRESS_MS - 1.0), None);
        assert_eq!(p.due(LONG_PRESS_MS), Some(Pos2::new(100.0, 100.0)));
        assert_eq!(p.due(LONG_PRESS_MS + 500.0), None, "one time");
        assert!(p.fired());
        assert!(p.up(), "the primary button is up already");
        assert!(!p.fired());
        // A stroke that moves draws: no menu.
        p.down(0.0, Pos2::new(100.0, 100.0));
        p.moved(Pos2::new(120.0, 100.0));
        assert!(!p.waiting());
        assert_eq!(p.due(5000.0), None);
        assert!(!p.up());
    }

    /// The events of a long press on a line, while the pen is still down,
    /// select the line and open its menu (with Edit Nodes).
    #[test]
    fn the_events_of_a_long_press_open_the_object_menu() {
        let mut h = Harness::empty();
        let id = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new()
                        .move_to(Point::new(-20.0, 0.0))
                        .line_to(Point::new(20.0, 0.0)),
                    params: Default::default(),
                },
            )
        });
        h.st.zoom_to = Some(emb_geom::BBox::new(Point::new(-30.0, -30.0), Point::new(30.0, 30.0)));
        h.frame(vec![]);
        let undo = h.st.doc.undo_len();
        let at = h.screen(Point::new(5.0, 0.0));
        h.frame(vec![Event::PointerMoved(at)]);
        h.frame(vec![Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }]);
        // Still, for longer than a click.
        for _ in 0..60 {
            h.frame(vec![]);
        }
        h.frame(menu_events(at, Modifiers::NONE).to_vec());
        h.frame(vec![]);
        assert_eq!(h.st.selection.single(), Some(id));
        assert!(h.widget("Edit Nodes").is_some(), "the object menu is open");
        assert_eq!(h.st.doc.design().objects().len(), 1);
        assert_eq!(
            h.st.doc.undo_len(),
            undo,
            "the press moved nothing: {:?}",
            h.st.doc.undo_label()
        );
    }
}
