//! A project file that another program saves while it is open in a tab
//! (P22 Milestone 9, ADR 0176): for example an agent that changes the
//! design through `emb-cli mcp` and saves it. The program asks to open the
//! file again, or to keep the design of the tab.
//!
//! The program knows the modification time of each file that it opens or
//! saves (`file_known`). Each second it compares the time on the disk of
//! the file of the tab in front with it (`poll_file_changes`).

use crate::clock::{Duration, Instant};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use eframe::egui;

use crate::app::AppState;
use crate::dock::FitScreen;
use crate::i18n::{fill, tr};

/// How often the program looks at the file of the tab in front.
pub const CHECK_EVERY: Duration = Duration::from_secs(1);

#[derive(Debug, Default)]
pub struct FileWatch {
    /// The modification time of each file that the program opened or saved.
    known: HashMap<PathBuf, SystemTime>,
    last: Option<Instant>,
    /// The file that another program saved, while the question shows.
    pub changed: Option<PathBuf>,
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl AppState {
    /// The program opened or saved `path`: the time on the disk is its own.
    pub fn file_known(&mut self, path: &Path) {
        if let Some(t) = modified(path) {
            self.file_watch.known.insert(path.to_path_buf(), t);
        }
    }

    /// A job of the program (Output, for example) wrote `files`: the times
    /// on the disk of the files that the program knows are its own (the
    /// audit of 2026-11-26: Output saved the project of the tab two times,
    /// and the second save got the question).
    pub fn files_written(&mut self, files: &[PathBuf]) {
        for f in files {
            if self.file_watch.known.contains_key(f) {
                self.file_known(f);
            }
        }
    }

    /// Looks at the file of the tab in front, at most each [`CHECK_EVERY`]:
    /// a file that another program saved gets the question. Not while a
    /// job of the program runs: it can write the file.
    pub fn poll_file_changes(&mut self, now: Instant) {
        let busy = self.jobs.busy();
        let w = &mut self.file_watch;
        if busy || w.changed.is_some() || w.last.is_some_and(|t| now.duration_since(t) < CHECK_EVERY) {
            return;
        }
        w.last = Some(now);
        let Some(path) = self.file_path.clone() else { return };
        let Some(t) = modified(&path) else { return };
        match self.file_watch.known.get(&path) {
            Some(k) if *k != t => self.file_watch.changed = Some(path),
            Some(_) => {}
            // A file that the program did not open itself (a tab that came
            // back): its time now is the known one.
            None => {
                self.file_watch.known.insert(path, t);
            }
        }
    }

    /// Open Again: the design of the file in place of the design of the
    /// tab (a new history; the view stays).
    pub fn open_changed_file(&mut self) {
        let Some(path) = self.file_watch.changed.take() else {
            return;
        };
        match emb_model::project::load(&path) {
            Ok(d) => {
                self.replace_tab_design(d);
                self.file_known(&path);
                self.message = fill("Opened {} again", &[&path.display()]);
            }
            Err(e) => {
                self.file_known(&path);
                self.notice = Some(crate::file::Notice {
                    title: "Cannot open design".into(),
                    lines: vec![path.display().to_string(), e.to_string()],
                });
            }
        }
    }

    /// Keep This Version: the design of the tab stays, and the file on the
    /// disk gets no second question until it changes again. The design is
    /// not the file now, so it is not saved (a close asks).
    pub fn keep_changed_file(&mut self) {
        if let Some(p) = self.file_watch.changed.take() {
            self.file_known(&p);
            self.doc.mark_unsaved();
        }
    }
}

/// The question about a file that another program saved.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    let Some(path) = st.file_watch.changed.clone() else {
        return;
    };
    // The question is for the tab of the file.
    if st.file_path.as_deref() != Some(path.as_path()) {
        st.file_watch.changed = None;
        return;
    }
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let (mut again, mut keep) = (false, false);
    egui::Window::new(tr("File Changed"))
        .id(egui::Id::new("File Changed"))
        .fit_screen(ctx)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(fill(
                "Another program (an agent, for example) saved {} while it is open here.",
                &[&name],
            ));
            if st.doc.is_modified() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    tr("The changes of this tab that are not saved go if you open the file again."),
                );
            }
            ui.horizontal(|ui| {
                again = ui.button(tr("Open Again")).clicked();
                keep = ui.button(tr("Keep This Version")).clicked();
            });
        });
    if again {
        st.open_changed_file();
    } else if keep {
        st.keep_changed_file();
    }
}

#[cfg(test)]
mod tests {
    use emb_model::project;

    use super::*;
    use crate::test_util::Harness;

    fn file(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("emb-file-watch-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name)
    }

    fn design(objects: usize) -> emb_model::Design {
        let mut d = emb_model::Design::new("Agent");
        let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
        for i in 0..objects {
            d.add_object(
                t,
                emb_model::ObjectKind::Running {
                    path: emb_geom::Path::new()
                        .move_to(emb_geom::Point::new(0.0, i as f64))
                        .line_to(emb_geom::Point::new(20.0, i as f64)),
                    params: Default::default(),
                },
            );
        }
        d
    }

    /// An agent saves the file of the tab: the question comes; Open Again
    /// gives the new design in the same tab; a save of the program itself
    /// asks nothing; Keep This Version keeps the design.
    #[test]
    fn a_file_that_another_program_saves_gets_the_question() {
        let path = file("agent.embproj");
        project::save(&design(1), &path).unwrap();
        let mut h = Harness::empty();
        h.st.open_path(&path);
        let tabs = h.st.tab_count();
        let t0 = Instant::now();
        h.st.poll_file_changes(t0);
        assert!(h.st.file_watch.changed.is_none(), "the program opened it");
        // The program saves it: no question.
        assert!(h.st.save_to(&path));
        h.st.poll_file_changes(t0 + Duration::from_secs(2));
        assert!(h.st.file_watch.changed.is_none(), "the program saved it");
        // Another program saves it (a later time on the disk).
        std::thread::sleep(std::time::Duration::from_millis(20));
        project::save(&design(3), &path).unwrap();
        let later = SystemTime::now() + std::time::Duration::from_secs(5);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(later)
            .unwrap();
        h.st.poll_file_changes(t0 + Duration::from_millis(2500));
        assert!(h.st.file_watch.changed.is_none(), "at most one look each second");
        h.st.poll_file_changes(t0 + Duration::from_secs(4));
        assert_eq!(h.st.file_watch.changed.as_deref(), Some(path.as_path()));
        h.frame(vec![]);
        h.st.open_changed_file();
        assert_eq!(h.st.doc.design().objects().len(), 3, "the design of the file");
        assert_eq!(h.st.tab_count(), tabs, "in the same tab");
        assert_eq!(h.st.file_path.as_deref(), Some(path.as_path()));
        assert!(!h.st.doc.can_undo());
        // Again, and this time Keep This Version.
        project::save(&design(5), &path).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(later + std::time::Duration::from_secs(5))
            .unwrap();
        h.st.poll_file_changes(t0 + Duration::from_secs(6));
        assert!(h.st.file_watch.changed.is_some());
        h.st.keep_changed_file();
        assert_eq!(h.st.doc.design().objects().len(), 3);
        assert!(h.st.doc.is_modified(), "the kept design is not the file");
        h.st.poll_file_changes(t0 + Duration::from_secs(8));
        assert!(h.st.file_watch.changed.is_none(), "no second question");
        std::fs::remove_file(&path).ok();
    }

    /// Output saves the project of the tab in its folder, and its job
    /// writes the project again: no question (the audit of 2026-11-26).
    #[test]
    fn the_output_of_the_program_gets_no_question() {
        let dir = file("output");
        std::fs::create_dir_all(&dir).unwrap();
        let mut h = Harness::empty();
        h.st.start_on_machine("husqvarna_topaz20", None);
        h.st.add_sample_objects();
        let t0 = Instant::now();
        h.st.queue_output_design(&dir, "badge").unwrap();
        let proj = dir.join(format!("badge.{}", project::FILE_EXTENSION));
        assert_eq!(h.st.file_path.as_deref(), Some(proj.as_path()));
        // A look while the job runs asks nothing.
        h.st.poll_file_changes(t0);
        assert!(h.st.file_watch.changed.is_none());
        h.st.wait_jobs();
        for k in 1..4 {
            h.st.poll_file_changes(t0 + Duration::from_secs(2 * k));
            assert!(h.st.file_watch.changed.is_none(), "the program wrote it");
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
