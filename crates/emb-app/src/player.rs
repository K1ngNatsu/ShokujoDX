//! The stitch player (roadmap P3, Milestone 4): it moves the stitch cursor
//! through the design at a set speed, so the canvas shows the sew-out.
//!
//! The player bar is under the canvas: to start, back one color, play or
//! pause, forward one color, to end, the speed, and a slider for the
//! position. The stitches after the cursor are dimmed (see `viewing`).

use eframe::egui;
use egui_phosphor::regular as ph;
use emb_model::StitchKind;

use crate::app::AppState;
use crate::i18n::tr;
use crate::viewing::Travel;

/// Speeds in stitches per second.
pub const SPEEDS: [(f64, &str); 6] = [
    (10.0, "10 stitches/s"),
    (50.0, "50 stitches/s"),
    (200.0, "200 stitches/s"),
    (1000.0, "1000 stitches/s"),
    (5000.0, "5000 stitches/s"),
    (20000.0, "20000 stitches/s"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    /// The player bar shows.
    pub shown: bool,
    pub playing: bool,
    /// Records per second.
    pub speed: f64,
    /// The part of a record not yet moved.
    carry: f64,
}

impl Default for Player {
    fn default() -> Self {
        Player {
            shown: false,
            playing: false,
            speed: 200.0,
            carry: 0.0,
        }
    }
}

impl AppState {
    /// Shows or hides the player bar. Showing it puts the cursor at the start.
    pub fn toggle_player(&mut self) {
        self.player.shown = !self.player.shown;
        self.player.playing = false;
        if self.player.shown {
            self.show_plan = true;
            self.cursor.get_or_insert(0);
        }
    }

    /// Starts or pauses. At the end, play starts again from the start.
    pub fn play_pause(&mut self) {
        let last = self.plan_view().plan.stitches.records.len().saturating_sub(1);
        if self.player.playing {
            self.player.playing = false;
            return;
        }
        if self.cursor.is_none_or(|c| c >= last) {
            self.cursor = Some(0);
        }
        self.show_plan = true;
        self.player.playing = true;
        self.player.carry = 0.0;
    }

    /// Moves the cursor for `dt` seconds of play. Returns `true` while playing.
    pub fn player_tick(&mut self, dt: f64) -> bool {
        if !self.player.playing {
            return false;
        }
        let last = self.shown_plan().plan.stitches.records.len().saturating_sub(1);
        let steps = self.player.speed * dt.clamp(0.0, 0.25) + self.player.carry;
        let n = steps.floor();
        self.player.carry = steps - n;
        let next = (self.cursor.unwrap_or(0) + n as usize).min(last);
        self.cursor = Some(next);
        if next >= last {
            self.player.playing = false;
        }
        self.player.playing
    }
}

fn button(ui: &mut egui::Ui, glyph: &str, tip: &str) -> bool {
    ui.add(egui::Button::new(egui::RichText::new(glyph).size(16.0)).min_size(egui::vec2(28.0, 24.0)))
        .on_hover_text(tip)
        .clicked()
}

/// The player bar.
pub fn bar(ui: &mut egui::Ui, st: &mut AppState) {
    let plan = st.shown_plan().plan.clone();
    let recs = &plan.stitches.records;
    let last = recs.len().saturating_sub(1);
    let info = st.design_info();
    ui.horizontal(|ui| {
        if button(ui, ph::SKIP_BACK, tr("To the start")) {
            st.player.playing = false;
            st.cursor = Some(0);
        }
        if button(ui, ph::REWIND, tr("Back One Color")) {
            st.travel(Travel::Color(false));
        }
        let (glyph, tip) = if st.player.playing {
            (ph::PAUSE, tr("Pause"))
        } else {
            (ph::PLAY, tr("Play the sew-out"))
        };
        if button(ui, glyph, tip) {
            st.play_pause();
        }
        if button(ui, ph::FAST_FORWARD, tr("Forward One Color")) {
            st.travel(Travel::Color(true));
        }
        if button(ui, ph::SKIP_FORWARD, tr("To the end")) {
            st.player.playing = false;
            st.cursor = Some(last);
        }
        ui.separator();
        let speed_text = SPEEDS
            .iter()
            .find(|(s, _)| *s == st.player.speed)
            .map_or_else(|| format!("{:.0} stitches/s", st.player.speed), |(_, t)| t.to_string());
        egui::ComboBox::from_id_salt("player_speed")
            .selected_text(speed_text)
            .show_ui(ui, |ui| {
                for (s, t) in SPEEDS {
                    ui.selectable_value(&mut st.player.speed, s, t);
                }
            });
        ui.separator();
        let mut pos = st.cursor.unwrap_or(0).min(last);
        let width = (ui.available_width() - 330.0).max(120.0);
        ui.spacing_mut().slider_width = width;
        if ui
            .add(
                egui::Slider::new(&mut pos, 0..=last)
                    .clamping(egui::SliderClamping::Edits)
                    .show_value(false),
            )
            .on_hover_text(tr("Drag to move through the sew-out"))
            .changed()
        {
            st.player.playing = false;
            st.cursor = Some(pos);
        }
        // Where the cursor is: stitch, color and time.
        let done = recs[..=pos.min(last)]
            .iter()
            .filter(|r| r.kind == StitchKind::Stitch)
            .count();
        let color = recs[..=pos.min(last)]
            .iter()
            .filter(|r| r.kind == StitchKind::ColorChange)
            .count()
            + 1;
        let mut text = format!(
            "Stitch {done} of {} · color {color} of {}",
            info.stitches,
            info.colors.len().max(1)
        );
        if let Some(m) = info.minutes.filter(|_| info.stitches > 0) {
            let at = m * done as f64 / info.stitches as f64;
            text.push_str(&format!(
                " · {} of {}",
                crate::info::time_text(at),
                crate::info::time_text(m)
            ));
        }
        ui.label(text);
        if button(ui, ph::X, tr("Close the player")) {
            st.player.shown = false;
            st.player.playing = false;
        }
    });
}

#[cfg(test)]
mod tests {
    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    fn state() -> AppState {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st
    }

    #[test]
    fn plays_to_the_end_and_again() {
        let mut st = state();
        let last = st.plan_view().plan.stitches.records.len() - 1;
        st.toggle_player();
        assert!(st.player.shown && st.cursor == Some(0));
        st.player.speed = 1000.0;
        st.play_pause();
        assert!(st.player_tick(0.1));
        assert_eq!(st.cursor, Some(100));
        // A fraction of a record waits for the next frame.
        st.player.speed = 15.0;
        st.player_tick(0.1);
        st.player_tick(0.1);
        assert_eq!(st.cursor, Some(103));
        // A long frame is cut to 0.25 s.
        st.player.speed = 1e9;
        assert!(!st.player_tick(10.0), "stops at the end");
        assert_eq!(st.cursor, Some(last));
        // Play at the end starts again from the start.
        st.play_pause();
        assert_eq!(st.cursor, Some(0));
        st.play_pause();
        assert!(!st.player.playing);
    }

    #[test]
    fn bar_draws() {
        let mut st = state();
        st.toggle_player();
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| bar(ui, &mut st));
            });
            out.textures_delta.clear();
        }
    }
}
