//! The start page (P10 Milestone 4): a dock tab next to the design, at the
//! start of the program (a setting) and from Help > Start page.
//!
//! - Start: New design, New from template, Open, Picture to design, the
//!   design library.
//! - Learn: the tutorials, the help, the keyboard shortcuts, and a box for
//!   a question (the help search answers it).
//! - The recent designs of the user, with a picture of each. A thread makes
//!   the pictures, so the page shows at once. A new user starts with a
//!   clean slate: the program has no sample designs (ADR 0080).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use eframe::egui::{self, RichText, Vec2};

use crate::app::AppState;
use crate::file::Pending;
use crate::i18n::tr;
use crate::ribbon::Cmd;

/// The size of a design picture on the start page, points.
const THUMB: f32 = 96.0;

/// The state of the start page.
#[derive(Default)]
pub struct StartState {
    /// The pictures of the designs (`None`: the design cannot be read).
    thumbs: HashMap<PathBuf, Option<egui::TextureHandle>>,
    /// Designs whose pictures are asked for.
    asked: HashSet<PathBuf>,
    /// The pictures that the thread makes.
    incoming: Option<Receiver<(PathBuf, Option<egui::ColorImage>)>>,
    /// The question box.
    pub question: String,
}

/// The picture of a design file, for the start page.
pub fn thumbnail(path: &Path) -> Option<egui::ColorImage> {
    let s = emb_library::index::read_stitches(path).ok()?;
    let png = emb_library::index::thumbnail(&s);
    let img = image::load_from_memory(&png).ok()?.to_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        img.as_raw(),
    ))
}

impl AppState {
    /// Shows the start page in front.
    pub fn show_start_page(&mut self) {
        self.show_start = true;
        self.focus_start = true;
    }

    /// Asks for the pictures of `paths` that are not asked for yet (a thread makes them).
    fn ask_thumbnails(&mut self, paths: &[PathBuf]) {
        let new: Vec<PathBuf> = paths
            .iter()
            .filter(|p| !self.start.asked.contains(*p))
            .cloned()
            .collect();
        if new.is_empty() || self.start.incoming.is_some() {
            return;
        }
        self.start.asked.extend(new.iter().cloned());
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for p in new {
                let img = thumbnail(&p);
                if tx.send((p, img)).is_err() {
                    return;
                }
            }
        });
        self.start.incoming = Some(rx);
    }

    /// Takes the pictures that are ready. Returns `true` while more come.
    fn take_thumbnails(&mut self, ctx: &egui::Context) -> bool {
        let Some(rx) = &self.start.incoming else { return false };
        loop {
            match rx.try_recv() {
                Ok((p, img)) => {
                    let tex = img.map(|i| ctx.load_texture(format!("start {}", p.display()), i, Default::default()));
                    self.start.thumbs.insert(p, tex);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return true,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.start.incoming = None;
                    return false;
                }
            }
        }
    }
}

/// A card of a design: its picture and its name. Returns `true` on a click.
fn card(ui: &mut egui::Ui, st: &AppState, p: &Path) -> bool {
    let name = p
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = p.parent().map(|d| d.display().to_string()).unwrap_or_default();
    let resp = ui
        .vertical(|ui| {
            ui.set_width(THUMB + 8.0);
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(THUMB), egui::Sense::click());
            let painter = ui.painter();
            painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
            if resp.hovered() {
                painter.rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(1.5, ui.visuals().selection.stroke.color),
                    egui::StrokeKind::Inside,
                );
            }
            match st.start.thumbs.get(p) {
                Some(Some(tex)) => {
                    let size = tex.size_vec2();
                    let k = (THUMB - 8.0) / size.x.max(size.y).max(1.0);
                    let r = egui::Rect::from_center_size(rect.center(), size * k);
                    painter.image(
                        tex.id(),
                        r,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                Some(None) => {
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "?",
                        egui::FontId::proportional(20.0),
                        ui.visuals().weak_text_color(),
                    );
                }
                None => {
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "…",
                        egui::FontId::proportional(20.0),
                        ui.visuals().weak_text_color(),
                    );
                }
            }
            ui.add(egui::Label::new(RichText::new(&name).small()).truncate());
            resp
        })
        .inner;
    resp.on_hover_text(format!("{name}\n{folder}")).clicked()
}

/// Design cards in rows that fit the width. Returns the design that was clicked.
fn cards(ui: &mut egui::Ui, st: &AppState, paths: &[PathBuf]) -> Option<PathBuf> {
    let mut open = None;
    let per_row = ((ui.available_width() / (THUMB + 16.0)).floor() as usize).max(1);
    for row in paths.chunks(per_row) {
        ui.horizontal(|ui| {
            for p in row {
                if card(ui, st, p) {
                    open = Some(p.clone());
                }
            }
        });
    }
    open
}

/// The start page.
pub fn page(ui: &mut egui::Ui, st: &mut AppState) {
    let ctx = ui.ctx().clone();
    let recent: Vec<PathBuf> = st.settings.recent.iter().filter(|p| p.is_file()).cloned().collect();
    st.ask_thumbnails(&recent);
    if st.take_thumbnails(&ctx) {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    let mut run: Option<Cmd> = None;
    let mut open: Option<PathBuf> = None;
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(tr("ShokujoDX")).size(24.0).strong());
                ui.weak(crate::i18n::fill("Version {}", &[&(crate::about::VERSION)]));
                ui.add_space(12.0);
                ui.columns(2, |cols| {
                    let ui = &mut cols[0];
                    ui.label(RichText::new(tr("Start")).size(16.0).strong());
                    for cmd in [
                        Cmd::New,
                        Cmd::NewFromTemplate,
                        Cmd::Open,
                        Cmd::PictureWizard,
                        Cmd::Library,
                    ] {
                        let (_, label, tip) = cmd.meta();
                        let text = match cmd {
                            Cmd::New => "New design",
                            Cmd::Open => "Open a design…",
                            Cmd::Library => "Design Library",
                            _ => label,
                        };
                        if ui
                            .add(egui::Button::new(text).min_size(Vec2::new(220.0, 28.0)))
                            .on_hover_text(tip)
                            .clicked()
                        {
                            run = Some(cmd);
                        }
                    }
                    ui.add_space(12.0);
                    ui.label(RichText::new(tr("Learn")).size(16.0).strong());
                    for id in [
                        "getting-started",
                        "tutorial-patch",
                        "tutorial-lettering",
                        "tutorial-output",
                    ] {
                        let title = match id {
                            "getting-started" => tr("Getting started"),
                            "tutorial-patch" => tr("Tutorial: a patch from a picture"),
                            "tutorial-lettering" => tr("Tutorial: a name on an arc"),
                            _ => tr("Tutorial: production output"),
                        };
                        if ui.link(title).clicked() {
                            st.open_help(id, None);
                        }
                    }
                    if ui.link(tr("Keyboard Shortcuts")).clicked() {
                        st.open_help(crate::help::KEYBOARD_ID, None);
                    }
                    ui.add_space(8.0);
                    let mut s = st.settings.clone();
                    if ui
                        .checkbox(&mut s.simple_mode, tr("Simple mode: fewer tools, larger buttons"))
                        .on_hover_text(tr("The tools to draw a design, letter it and send it to the machine. Help > Simple Mode changes it again."))
                        .changed()
                    {
                        run = Some(Cmd::Toggle(crate::ribbon::Flag::SimpleMode));
                    }
                    if ui.link(tr("Lettering kiosk (for shops)")).clicked() {
                        run = Some(Cmd::Kiosk);
                    }
                    ui.add_space(8.0);
                    ui.label(tr("Ask a question:"));
                    ui.horizontal(|ui| {
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut st.start.question)
                                .hint_text(tr("for example: how do I change the hoop"))
                                .desired_width(220.0),
                        );
                        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if (ui.button(tr("Ask")).clicked() || enter) && !st.start.question.trim().is_empty() {
                            let q = st.start.question.clone();
                            st.ask_help(&q);
                        }
                    });
                    let ui = &mut cols[1];
                    ui.label(RichText::new(tr("Recent designs")).size(16.0).strong());
                    if recent.is_empty() {
                        ui.weak(tr("The designs that you open and save show here."));
                    } else if let Some(p) = cards(ui, st, &recent) {
                        open = Some(p);
                    }
                });
                ui.add_space(16.0);
                let mut s = st.settings.clone();
                if ui
                    .checkbox(&mut s.start_page, tr("Show at start"))
                    .on_hover_text(tr("Show this page when the program starts. Help > Start Page shows it at any time."))
                    .changed()
                {
                    st.set_settings(s);
                }
            });
        });
    });
    if let Some(cmd) = run {
        st.run(cmd, &ctx);
        // A new or opened design shows; the mode switch stays on this page.
        if !matches!(cmd, Cmd::Toggle(_)) {
            st.focus_canvas = true;
        }
    }
    if let Some(p) = open {
        st.request(Pending::Open(Some(p)));
        st.focus_canvas = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pictures_of_designs() {
        let dir = std::env::temp_dir().join(format!("emb-start-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let d = emb_cli::corpus::build("two-color-patch").unwrap();
        let p = dir.join("patch.embproj");
        emb_model::project::save(&d, &p).unwrap();
        std::fs::write(dir.join("notes.txt"), "x").unwrap();
        let img = thumbnail(&p).expect("a picture");
        assert!(img.size[0] > 10 && img.size[1] > 10);
        assert!(thumbnail(&dir.join("notes.txt")).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_page_draws_and_makes_the_pictures() {
        let mut h = crate::test_util::Harness::empty();
        let dir = std::env::temp_dir().join(format!("emb-start-recent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("recent.embproj");
        let d = emb_cli::corpus::build("satin-column").unwrap();
        emb_model::project::save(&d, &p).unwrap();
        h.st.save_settings = false;
        h.st.settings.recent = vec![p.clone()];
        let ctx = egui::Context::default();
        // A time limit, not a number of frames, for slow computers.
        let end = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while std::time::Instant::now() < end {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| page(ui, &mut h.st));
            });
            out.textures_delta.clear();
            if h.st.start.thumbs.contains_key(&p) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            matches!(h.st.start.thumbs.get(&p), Some(Some(_))),
            "the picture of the recent design"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
