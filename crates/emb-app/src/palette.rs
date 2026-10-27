//! Find a command (Ctrl+K, P10 Milestone 7): keyboard access to all ribbon
//! commands. Type a part of the name or of the tooltip; the arrow keys
//! select; Enter runs the command; Esc closes. A command that cannot run
//! now shows dimmed and says why in its tooltip.

use eframe::egui::{self, Key, KeyboardShortcut, Modifiers};

use crate::app::AppState;
use crate::i18n::tr;
use crate::ribbon::{Cmd, RibbonTab, SelInfo};

pub const FIND_COMMAND: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::K);
/// Most commands in the list.
const MAX_SHOWN: usize = 14;

/// The state of the open window.
#[derive(Debug, Default, Clone)]
pub struct Palette {
    pub query: String,
    pub selected: usize,
}

/// One command of the list: where it is on the ribbon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    pub cmd: Cmd,
    pub tab: RibbonTab,
    pub panel: &'static str,
}

/// All commands of the full ribbon, each one time, in ribbon order.
pub fn entries() -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for (cmd, tab, panel) in crate::ribbon::all_commands() {
        if !out.iter().any(|e| e.cmd == cmd) {
            out.push(Entry { cmd, tab, panel });
        }
    }
    out
}

/// The entries that match the words of `query` (in the name or the
/// tooltip, in the language of the program), best first: names that start
/// with the query, then names with all words, then tooltips.
pub fn search(all: &[Entry], query: &str) -> Vec<Entry> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return all.to_vec();
    }
    let words: Vec<&str> = q.split_whitespace().collect();
    let mut scored: Vec<(u8, usize, Entry)> = all
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let (_, name, tip) = e.cmd.meta();
            let (name, tip) = (name.to_lowercase(), tip.to_lowercase());
            let score = if name.starts_with(&q) {
                0
            } else if words.iter().all(|w| name.contains(w)) {
                1
            } else if words.iter().all(|w| name.contains(w) || tip.contains(w)) {
                2
            } else {
                return None;
            };
            Some((score, i, *e))
        })
        .collect();
    scored.sort_by_key(|(s, i, _)| (*s, *i));
    scored.into_iter().map(|(_, _, e)| e).collect()
}

impl AppState {
    pub fn open_palette(&mut self) {
        self.palette = Some(Palette::default());
    }
}

/// The Find a command window, if it is open.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut pal) = st.palette.take() else { return };
    let all = entries();
    let found = search(&all, &pal.query);
    let (down, up, enter, esc) = ctx.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::ArrowDown),
            i.consume_key(Modifiers::NONE, Key::ArrowUp),
            i.consume_key(Modifiers::NONE, Key::Enter),
            i.consume_key(Modifiers::NONE, Key::Escape),
        )
    });
    if esc {
        return;
    }
    let shown = found.len().min(MAX_SHOWN);
    if down && shown > 0 {
        pal.selected = (pal.selected + 1).min(shown - 1);
    }
    if up {
        pal.selected = pal.selected.saturating_sub(1);
    }
    pal.selected = pal.selected.min(shown.saturating_sub(1));
    let sel = SelInfo::of(st);
    let mut run: Option<Cmd> = None;
    let mut open = true;
    egui::Window::new(tr("Find a Command"))
        .id(egui::Id::new("Find a Command"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .default_width(420.0)
        .show(ctx, |ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut pal.query)
                    .hint_text(tr("Type a part of a command name"))
                    .desired_width(f32::INFINITY),
            );
            edit.request_focus();
            if edit.changed() {
                pal.selected = 0;
            }
            ui.separator();
            if found.is_empty() {
                ui.weak(tr("No command has these words."));
            }
            for (k, e) in found.iter().take(MAX_SHOWN).enumerate() {
                let (_, name, tip) = e.cmd.meta();
                let can = st.can(e.cmd, &sel);
                let keys = e.cmd.shortcut_text(ctx);
                let place = format!("{} › {}", e.tab.label(), tr(e.panel));
                let text = match &keys {
                    Some(k) => format!("{name}    {k}"),
                    None => name.to_string(),
                };
                let resp = ui
                    .add_enabled(
                        can,
                        egui::Button::selectable(k == pal.selected, text).wrap_mode(egui::TextWrapMode::Truncate),
                    )
                    .on_hover_text(format!("{tip}\n{place}"))
                    .on_disabled_hover_text(crate::i18n::fill("{} cannot run now.", &[&name]));
                ui.weak(place);
                if resp.clicked() {
                    run = Some(e.cmd);
                }
                if enter && k == pal.selected {
                    if can {
                        run = Some(e.cmd);
                    } else {
                        st.message = crate::i18n::fill("{} cannot run now.", &[&name]);
                    }
                }
            }
            if found.len() > MAX_SHOWN {
                ui.weak(crate::i18n::fill(
                    "{} more: type more letters.",
                    &[&(found.len() - MAX_SHOWN)],
                ));
            }
        });
    match run {
        Some(cmd) => {
            // The window closes first, so a command that opens a window gets the focus.
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
            st.run(cmd, ctx);
        }
        None if open => st.palette = Some(pal),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn every_ribbon_command_is_in_the_list_one_time() {
        let all = entries();
        assert!(all.len() > 250, "{}", all.len());
        for (i, e) in all.iter().enumerate() {
            assert!(!all[i + 1..].iter().any(|f| f.cmd == e.cmd), "{:?} two times", e.cmd);
        }
        assert!(all.iter().any(|e| e.cmd == Cmd::OptimizeOrder));
        assert!(all.iter().any(|e| e.cmd == Cmd::FindCommand));
    }

    #[test]
    fn search_finds_names_first_then_tooltips() {
        let all = entries();
        let r = search(&all, "shortest");
        assert_eq!(r[0].cmd, Cmd::OptimizeOrder);
        let r = search(&all, "undo");
        assert_eq!(r[0].cmd, Cmd::Undo);
        // A word of the tooltip only.
        let r = search(&all, "trims");
        assert!(r.iter().any(|e| e.cmd == Cmd::OptimizeOrder));
        assert!(search(&all, "zzqx").is_empty());
    }

    #[test]
    fn ctrl_k_opens_and_enter_runs() {
        let mut h = Harness::empty();
        h.press(Modifiers::COMMAND, Key::K);
        assert!(h.st.palette.is_some());
        h.st.palette.as_mut().unwrap().query = "select all".into();
        h.frame(vec![]);
        h.press(Modifiers::NONE, Key::Enter);
        assert!(h.st.palette.is_none(), "the command ran and the window closed");
        assert!(!h.st.message.contains("cannot run"), "{}", h.st.message);
        // Esc closes it with no command.
        h.press(Modifiers::COMMAND, Key::K);
        h.press(Modifiers::NONE, Key::Escape);
        assert!(h.st.palette.is_none());
    }
}
