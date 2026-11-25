//! Automatic backups and recovery. See `docs/design/p3-plan.md`
//! (Milestone 8).
//!
//! Each running app has a session folder in `backups/` next to the
//! settings file. Every few minutes the app writes each open design with
//! unsaved changes there as a project file, and it writes `session.json`
//! with the time (the heartbeat) about every 30 seconds. A normal exit
//! deletes the session folder. If the app stops by force (a crash, a power
//! cut), the folder stays: at the next start, a session whose heartbeat is
//! old is a crashed session, and the app offers its designs for recovery.
//!
//! The open tabs come back (P22 Milestone 8, ADR 0175): with
//! `Settings::restore_tabs`, a normal quit does not ask about the changes
//! that are not saved. It keeps the session folder with a backup of each
//! tab that has changes (or no project file) and the list of the tabs
//! ([`Closed`]: the file of each, the tab in front, the views). The next
//! start opens the tabs again with no question and deletes the folder. A
//! closed session is not a crashed session.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui;
use emb_model::project;
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::i18n::tr;

/// How often the app writes the heartbeat.
pub const HEARTBEAT: Duration = Duration::from_secs(30);
/// A session whose heartbeat is older than this has stopped.
pub const STALE_S: u64 = 120;

const SESSION_FILE: &str = "session.json";

/// One design in a backup session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupEntry {
    pub id: u64,
    pub name: String,
    /// The project file of the design, if it had one.
    pub file: Option<PathBuf>,
    /// When the backup was written (seconds since 1970).
    pub written: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SessionFile {
    heartbeat: u64,
    docs: Vec<BackupEntry>,
    /// A normal quit that keeps the tabs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    closed: Option<Closed>,
}

/// A tab of a normal quit (ADR 0175).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TabEntry {
    pub name: String,
    /// The file of the design, if it has one.
    pub file: Option<PathBuf>,
    /// The backup project of the tab (`<id>.embproj` in the session
    /// folder): a design with changes that are not saved, or with no
    /// project file. `None`: the tab opens its file again.
    pub backup: Option<u64>,
    /// The design had changes that are not saved.
    pub changed: bool,
    /// The view: the center (mm) and the zoom.
    pub view: (f64, f64, f64),
}

/// The tabs of a normal quit (ADR 0175).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Closed {
    /// When the program quit (seconds since 1970).
    pub at: u64,
    pub tabs: Vec<TabEntry>,
    /// The tab in front (its number in `tabs`).
    pub front: usize,
}

/// The backups of this session.
pub struct Backups {
    /// The `backups` folder. `None`: no backups (no configuration folder).
    pub root: Option<PathBuf>,
    session: String,
    /// The backups that are on disk, with the design state that they have.
    written: BTreeMap<u64, (u64, BackupEntry)>,
    last_write: Option<Instant>,
    last_heartbeat: Option<Instant>,
    /// The last read of the inbox (`one_program`).
    pub(crate) last_inbox: Option<Instant>,
}

/// A design from a crashed session that can be recovered.
#[derive(Debug, Clone, PartialEq)]
pub struct Recovered {
    pub session_dir: PathBuf,
    pub entry: BackupEntry,
    pub path: PathBuf,
}

pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The `backups` folder next to the settings file.
pub fn backups_dir() -> Option<PathBuf> {
    crate::workspace::settings_path()?.parent().map(|d| d.join("backups"))
}

impl Backups {
    pub fn new(root: Option<PathBuf>) -> Backups {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        Backups {
            root,
            session: format!("{}-{millis}", std::process::id()),
            written: BTreeMap::new(),
            last_write: None,
            last_heartbeat: None,
            last_inbox: None,
        }
    }

    pub fn session_dir(&self) -> Option<PathBuf> {
        self.root.as_ref().map(|r| r.join(&self.session))
    }

    fn file_of(&self, id: u64) -> Option<PathBuf> {
        self.session_dir()
            .map(|d| d.join(format!("{id}.{}", project::FILE_EXTENSION)))
    }

    fn write_session(&self, now: u64) -> Result<(), String> {
        let Some(dir) = self.session_dir() else { return Ok(()) };
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        write_session_file(
            &dir,
            &SessionFile {
                heartbeat: now,
                docs: self.written.values().map(|(_, e)| e.clone()).collect(),
                closed: None,
            },
        )
    }
}

/// Writes `s` as the session file of the session folder `dir`.
fn write_session_file(dir: &Path, s: &SessionFile) -> Result<(), String> {
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    let path = dir.join(SESSION_FILE);
    std::fs::write(&path, json).map_err(|e| format!("{}: {e}", path.display()))
}

/// The session folders in `root` (other than `own`) that have a session
/// file that can be read, with it.
fn sessions(root: &Path, own: Option<&Path>) -> impl Iterator<Item = (PathBuf, SessionFile)> {
    let own = own.map(Path::to_path_buf);
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(move |d| d.is_dir() && Some(d) != own.as_ref())
        .filter_map(|d| {
            let s = std::fs::read_to_string(d.join(SESSION_FILE)).ok()?;
            let s = serde_json::from_str::<SessionFile>(&s).ok()?;
            Some((d, s))
        })
}

/// The designs of sessions in `root` (other than `own`) whose heartbeat is
/// older than [`STALE_S`] at `now`.
pub fn find_recoverable(root: &Path, own: Option<&Path>, now: u64) -> Vec<Recovered> {
    let mut out = Vec::new();
    for (dir, s) in sessions(root, own) {
        // A normal quit that kept its tabs is not a crash.
        if s.closed.is_some() || now.saturating_sub(s.heartbeat) <= STALE_S {
            continue;
        }
        for e in s.docs {
            let path = dir.join(format!("{}.{}", e.id, project::FILE_EXTENSION));
            if path.is_file() {
                out.push(Recovered {
                    session_dir: dir.clone(),
                    entry: e,
                    path,
                });
            }
        }
    }
    out.sort_by_key(|r| std::cmp::Reverse(r.entry.written));
    out
}

/// The sessions in `root` (other than `own`) of a normal quit that kept
/// its tabs, the oldest first.
pub fn find_closed(root: &Path, own: Option<&Path>) -> Vec<(PathBuf, Closed)> {
    let mut out: Vec<(PathBuf, Closed)> = sessions(root, own).filter_map(|(d, s)| Some((d, s.closed?))).collect();
    out.sort_by_key(|(_, c)| c.at);
    out
}

/// The session folders in `root` of programs that run: a heartbeat newer
/// than [`STALE_S`] at `now` and no normal quit, the newest heartbeat
/// first (P22 Milestone 8: one program).
pub fn running_sessions(root: &Path, now: u64) -> Vec<PathBuf> {
    let mut out: Vec<(u64, PathBuf)> = sessions(root, None)
        .filter(|(_, s)| s.closed.is_none() && now.saturating_sub(s.heartbeat) <= STALE_S)
        .map(|(d, s)| (s.heartbeat, d))
        .collect();
    out.sort_by_key(|(h, _)| std::cmp::Reverse(*h));
    out.into_iter().map(|(_, d)| d).collect()
}

/// Deletes a crashed session folder when no design in it waits for recovery.
fn remove_if_done(dir: &Path) {
    let waiting = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some(project::FILE_EXTENSION));
    if !waiting {
        std::fs::remove_dir_all(dir).ok();
    }
}

impl AppState {
    /// Writes the backups and the heartbeat when they are due. Call each frame.
    pub fn backups_tick(&mut self, now: Instant) {
        if self.backups.root.is_none() {
            return;
        }
        let minutes = self.settings.backup_minutes;
        let write_due = minutes > 0
            && self
                .backups
                .last_write
                .is_none_or(|t| now.duration_since(t) >= Duration::from_secs(u64::from(minutes) * 60));
        if write_due {
            self.backups.last_write = Some(now);
            if let Err(e) = self.write_backups(unix_now()) {
                self.message = crate::i18n::fill("Backup not written: {}", &[&e]);
            }
        } else if self
            .backups
            .last_heartbeat
            .is_none_or(|t| now.duration_since(t) >= HEARTBEAT)
        {
            self.backups.last_heartbeat = Some(now);
            self.backups.write_session(unix_now()).ok();
        }
    }

    /// Writes a backup of each open design with unsaved changes that
    /// changed since its last backup; removes the backups of saved and
    /// closed designs; writes the heartbeat.
    pub fn write_backups(&mut self, now: u64) -> Result<(), String> {
        let Some(dir) = self.backups.session_dir() else {
            return Ok(());
        };
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let docs: Vec<(u64, u64, bool, String, Option<PathBuf>)> = self
            .open_docs()
            .into_iter()
            .map(|(id, doc, file)| {
                let unsaved = doc.is_modified() && !doc.design().objects().is_empty();
                (
                    id,
                    doc.state_id(),
                    unsaved,
                    doc.design().name.clone(),
                    file.map(Path::to_path_buf),
                )
            })
            .collect();
        let mut keep = Vec::new();
        for (id, state, unsaved, name, file) in docs {
            let Some(path) = self.backups.file_of(id) else { continue };
            if !unsaved {
                continue;
            }
            keep.push(id);
            if self.backups.written.get(&id).is_some_and(|(s, _)| *s == state) {
                continue;
            }
            let design = self
                .open_docs()
                .into_iter()
                .find(|(i, ..)| *i == id)
                .map(|(_, d, _)| d.design().clone());
            if let Some(d) = design {
                project::save(&d, &path).map_err(|e| e.to_string())?;
                let entry = BackupEntry {
                    id,
                    name,
                    file,
                    written: now,
                };
                self.backups.written.insert(id, (state, entry));
            }
        }
        let gone: Vec<u64> = self
            .backups
            .written
            .keys()
            .copied()
            .filter(|id| !keep.contains(id))
            .collect();
        for id in gone {
            self.backups_remove(id);
        }
        self.backups.write_session(now)
    }

    fn backups_remove(&mut self, id: u64) {
        self.backups.written.remove(&id);
        if let Some(p) = self.backups.file_of(id) {
            std::fs::remove_file(p).ok();
        }
    }

    /// The design `id` is closed: its backup goes.
    pub fn backups_forget(&mut self, id: u64) {
        if self.backups.written.contains_key(&id) {
            self.backups_remove(id);
            self.backups.write_session(unix_now()).ok();
        }
    }

    /// A normal quit that keeps the tabs (ADR 0175): a backup of each tab
    /// with changes that are not saved or with no project file, and the
    /// list of the tabs in the session file. Blank tabs are left out.
    pub fn backups_keep_tabs(&mut self) -> Result<(), String> {
        let Some(dir) = self.backups.session_dir() else {
            return Err(tr("no folder for the backups").into());
        };
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut tabs = Vec::new();
        let mut front = 0;
        for i in 0..self.tabs.len() {
            let (id, doc, file, view) = match &self.tabs[i] {
                Some(s) => (s.doc_id, &s.doc, s.file_path.clone(), &s.view),
                None => (self.doc_id, &self.doc, self.file_path.clone(), &self.view),
            };
            let changed = doc.is_modified();
            if doc.design().objects().is_empty() && !changed && file.is_none() {
                continue;
            }
            let backup = if changed || file.is_none() {
                let path = self
                    .backups
                    .file_of(id)
                    .ok_or_else(|| tr("no folder for the backups").to_string())?;
                project::save(doc.design(), &path).map_err(|e| e.to_string())?;
                Some(id)
            } else {
                None
            };
            if i == self.active_tab {
                front = tabs.len();
            }
            tabs.push(TabEntry {
                name: doc.design().name.clone(),
                file,
                backup,
                changed,
                view: (view.center.x, view.center.y, view.zoom),
            });
        }
        // The backups of designs that are not in the list go.
        let kept: Vec<u64> = tabs.iter().filter_map(|t| t.backup).collect();
        for id in self.backups.written.keys().copied().collect::<Vec<_>>() {
            if !kept.contains(&id) {
                self.backups_remove(id);
            }
        }
        self.backups.written.clear();
        if tabs.is_empty() {
            self.backups_clean_exit();
            return Ok(());
        }
        let now = unix_now();
        let s = SessionFile {
            heartbeat: now,
            docs: Vec::new(),
            closed: Some(Closed { at: now, tabs, front }),
        };
        write_session_file(&dir, &s)
    }

    /// Opens the tabs of the normal quits before again (ADR 0175), and
    /// deletes their session folders. Returns the number of tabs.
    pub fn restore_tabs(&mut self) -> usize {
        let Some(root) = self.backups.root.clone() else {
            return 0;
        };
        let own = self.backups.session_dir();
        let mut front = None;
        let mut count = 0;
        let mut missing = Vec::new();
        for (dir, closed) in find_closed(&root, own.as_deref()) {
            for (k, t) in closed.tabs.iter().enumerate() {
                let opened = match (t.backup, &t.file) {
                    (Some(id), file) => match project::load(&dir.join(format!("{id}.{}", project::FILE_EXTENSION))) {
                        Ok(d) => {
                            self.start_design(d);
                            self.file_path = file.clone();
                            if t.changed {
                                self.doc.mark_unsaved();
                            }
                            true
                        }
                        Err(_) => false,
                    },
                    (None, Some(file)) if file.is_file() => {
                        self.open_path(file);
                        self.file_path.as_deref() == Some(file.as_path())
                    }
                    _ => false,
                };
                if !opened {
                    missing.push(t.name.clone());
                    continue;
                }
                count += 1;
                // A view of a damaged session file (a zoom of 0, a center
                // that is not a number or far out) is the view of the design.
                let m = project::MAX_COORD_MM;
                let center = emb_geom::Point::new(t.view.0, t.view.1);
                let good = center.x.abs() <= m && center.y.abs() <= m && t.view.2.is_finite();
                if good {
                    self.view = crate::canvas::CanvasView {
                        center,
                        zoom: t.view.2.clamp(crate::canvas::MIN_ZOOM, crate::canvas::MAX_ZOOM),
                    };
                }
                self.fit_requested = false;
                if k == closed.front {
                    front = Some(self.active_tab);
                }
            }
            std::fs::remove_dir_all(&dir).ok();
        }
        if let Some(i) = front {
            self.switch_tab(i);
        }
        if count > 0 {
            self.message = crate::i18n::fill("The tabs of the last time are open again: {}", &[&count]);
        }
        if !missing.is_empty() {
            self.message = crate::i18n::fill(
                "These designs of the last time could not open again: {}",
                &[&missing.join(", ")],
            );
        }
        count
    }

    /// A normal exit: the session folder goes.
    pub fn backups_clean_exit(&mut self) {
        if let Some(dir) = self.backups.session_dir() {
            std::fs::remove_dir_all(dir).ok();
        }
        self.backups.written.clear();
    }

    /// Opens recovered design `i` in a new tab. It has unsaved changes, and
    /// its file is the file it had (Save writes there).
    pub fn recover(&mut self, i: usize) {
        if i >= self.recovery.len() {
            return;
        }
        let r = self.recovery.remove(i);
        match project::load(&r.path) {
            Ok(d) => {
                self.start_design(d);
                self.file_path = r.entry.file.clone();
                self.doc.mark_unsaved();
                std::fs::remove_file(&r.path).ok();
                self.message = crate::i18n::fill("Recovered \"{}\". Save it to keep it.", &[&r.entry.name]);
            }
            Err(e) => self.message = crate::i18n::fill("Cannot recover \"{}\": {}", &[&r.entry.name, &e]),
        }
        remove_if_done(&r.session_dir);
    }

    /// Deletes recovered design `i` without opening it.
    pub fn discard_recovery(&mut self, i: usize) {
        if i >= self.recovery.len() {
            return;
        }
        let r = self.recovery.remove(i);
        std::fs::remove_file(&r.path).ok();
        remove_if_done(&r.session_dir);
    }
}

/// The Recover designs window, when a crashed session left designs.
pub fn recovery_window(ctx: &egui::Context, st: &mut AppState) {
    if st.recovery.is_empty() {
        return;
    }
    let mut action: Option<(usize, bool)> = None;
    let mut later = false;
    egui::Window::new(tr("Recover Designs"))
        .id(egui::Id::new("Recover Designs"))
        .collapsible(false)
        .resizable(false)
        .default_width(460.0)
        .show(ctx, |ui| {
            ui.set_min_width(440.0);
            ui.label(tr(
                "The app stopped before these designs were saved. Recover them, or discard the backups.",
            ));
            ui.add_space(4.0);
            egui::Grid::new("recovery")
                .num_columns(3)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    for (i, r) in st.recovery.iter().enumerate() {
                        let ago = unix_now().saturating_sub(r.entry.written) / 60;
                        ui.vertical(|ui| {
                            ui.set_min_width(300.0);
                            ui.strong(&r.entry.name);
                            let file = r
                                .entry
                                .file
                                .as_ref()
                                .map_or("not saved before".to_string(), |f| f.display().to_string());
                            ui.weak(crate::i18n::fill("{} · backup from {} min ago", &[&file, &ago]));
                        });
                        if ui.button(tr("Recover")).clicked() {
                            action = Some((i, true));
                        }
                        if ui.button(tr("Discard")).clicked() {
                            action = Some((i, false));
                        }
                        ui.end_row();
                    }
                });
            ui.separator();
            if ui
                .button(tr("Later"))
                .on_hover_text(tr("Keep the backups. The app asks again at the next start."))
                .clicked()
            {
                later = true;
            }
        });
    match action {
        Some((i, true)) => st.recover(i),
        Some((i, false)) => st.discard_recovery(i),
        None if later => st.recovery.clear(),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    fn root(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("emb-backups-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&d).ok();
        d
    }

    /// A harness with one blank tab and backups in `r`.
    fn blank_with_backups(r: &Path) -> Harness {
        let mut h = Harness::empty();
        while h.st.tab_count() > 1 {
            h.st.switch_tab(0);
            h.st.close_tab();
        }
        assert!(h.st.is_blank());
        h.st.backups = Backups::new(Some(r.to_path_buf()));
        h
    }

    /// A design with one run, named `name`.
    fn small(name: &str) -> emb_model::Design {
        let mut d = emb_model::Design::new(name);
        let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
        d.add_object(
            t,
            emb_model::ObjectKind::Running {
                path: emb_geom::Path::new()
                    .move_to(emb_geom::Point::ZERO)
                    .line_to(emb_geom::Point::new(20.0, 0.0)),
                params: Default::default(),
            },
        );
        d
    }

    /// The exit test of P22 Milestone 8 (ADR 0175): a quit with three tabs
    /// (one with changes that are not saved) gives the three tabs, the
    /// changes, the views and the same tab in front at the next start,
    /// with no question; it is not a crash.
    #[test]
    fn damaged_session_files_are_left_out() {
        let r = root("damaged");
        std::fs::create_dir_all(&r).unwrap();
        let file = r.join("one.embproj");
        project::save(&small("One"), &file).unwrap();
        let session = |name: &str, text: String| {
            let d = r.join(name);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(SESSION_FILE), text).unwrap();
        };
        session("empty", String::new());
        session("half", "{\"heartbeat\": 1, \"docs\": [".into());
        session("types", "{\"heartbeat\": \"no\", \"docs\": 5}".into());
        // A quit with tabs: a file that is not there, a backup that is not
        // there, and a view with a zoom of 0 and a center far out.
        let tab = |file: &str, backup: &str, view: &str| {
            format!("{{\"name\": \"T\", \"file\": {file}, \"backup\": {backup}, \"changed\": false, \"view\": {view}}}")
        };
        let file_json = serde_json::to_string(&file).unwrap();
        let tabs = [
            tab("\"Z:/not/there.embproj\"", "null", "[0.0, 0.0, 1.0]"),
            tab("null", "77", "[0.0, 0.0, 1.0]"),
            tab(&file_json, "null", "[1e300, -1e300, 0.0]"),
        ]
        .join(", ");
        session(
            "closed",
            format!("{{\"heartbeat\": 1, \"docs\": [], \"closed\": {{\"at\": 1, \"tabs\": [{tabs}], \"front\": 99}}}}"),
        );
        assert_eq!(find_recoverable(&r, None, unix_now()).len(), 0);
        assert_eq!(running_sessions(&r, unix_now()).len(), 0);
        let mut h = blank_with_backups(&r);
        assert_eq!(h.st.restore_tabs(), 1, "the one tab whose file is there");
        assert_eq!(h.st.file_path.as_deref(), Some(file.as_path()));
        let v = h.st.view;
        assert!(v.zoom >= crate::canvas::MIN_ZOOM && v.center.x.abs() < 1e6, "{v:?}");
        h.frame(vec![]);
        std::fs::remove_dir_all(&r).ok();
    }

    #[test]
    fn a_quit_keeps_the_tabs_and_the_next_start_opens_them() {
        let r = root("tabs");
        std::fs::create_dir_all(&r).unwrap();
        let first = r.join("first.embproj");
        let third = r.join("third.embproj");
        project::save(&small("First"), &first).unwrap();
        project::save(&small("Third"), &third).unwrap();
        let mut a = blank_with_backups(&r);
        a.st.open_path(&first);
        a.st.start_design(small("Second"));
        a.st.doc.edit("Add", |d| {
            let t = d.threads[0].id;
            d.add_object(
                t,
                emb_model::ObjectKind::Running {
                    path: emb_geom::Path::new()
                        .move_to(emb_geom::Point::new(0.0, 5.0))
                        .line_to(emb_geom::Point::new(20.0, 5.0)),
                    params: Default::default(),
                },
            );
        });
        a.st.view = crate::canvas::CanvasView {
            center: emb_geom::Point::new(5.0, 6.0),
            zoom: 2.5,
        };
        a.st.open_path(&third);
        assert_eq!(a.st.tab_count(), 3);
        a.st.switch_tab(1);
        a.st.request(crate::file::Pending::Quit);
        assert!(a.st.close_now, "the quit asks nothing");
        assert!(a.st.confirm.is_none());
        // A normal quit is not a crash.
        assert!(find_recoverable(&r, None, unix_now() + STALE_S + 10).is_empty());
        assert_eq!(find_closed(&r, None).len(), 1);
        // The next start.
        let mut b = blank_with_backups(&r);
        assert_eq!(b.st.restore_tabs(), 3);
        assert_eq!(b.st.tab_count(), 3);
        assert_eq!(b.st.active_tab, 1, "the same tab in front");
        assert_eq!(b.st.doc.design().name, "Second");
        assert_eq!(b.st.doc.design().objects().len(), 2, "the changes");
        assert!(b.st.doc.is_modified());
        assert!(b.st.file_path.is_none());
        assert_eq!(b.st.view.zoom, 2.5);
        assert_eq!(b.st.view.center, emb_geom::Point::new(5.0, 6.0));
        b.st.switch_tab(0);
        assert_eq!(b.st.file_path.as_deref(), Some(first.as_path()));
        assert!(!b.st.doc.is_modified());
        b.st.switch_tab(2);
        assert_eq!(b.st.file_path.as_deref(), Some(third.as_path()));
        // The session folder of the quit is gone.
        assert!(find_closed(&r, None).is_empty());
        std::fs::remove_dir_all(&r).ok();
    }

    /// With the option off, a quit asks about the changes as before.
    #[test]
    fn with_the_option_off_a_quit_asks() {
        let r = root("asks");
        let mut a = blank_with_backups(&r);
        a.st.settings.restore_tabs = false;
        a.st.start_design(small("Changed"));
        a.st.doc.mark_unsaved();
        a.st.request(crate::file::Pending::Quit);
        assert!(!a.st.close_now);
        assert_eq!(a.st.confirm, Some(crate::file::Pending::Quit));
        assert!(find_closed(&r, None).is_empty());
        std::fs::remove_dir_all(&r).ok();
    }

    #[test]
    fn recover_after_the_app_is_closed_by_force() {
        let root = root("crash");
        // The first app: two designs, one with unsaved changes.
        let mut a = Harness::new();
        a.st.backups = Backups::new(Some(root.clone()));
        a.st.new_design();
        a.st.write_backups(1_000).unwrap();
        let dir = a.st.backups.session_dir().unwrap();
        assert!(dir.join(SESSION_FILE).is_file());
        let files = |d: &Path| {
            std::fs::read_dir(d)
                .unwrap()
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "embproj"))
                .count()
        };
        assert_eq!(files(&dir), 1, "only the design with changes");
        let n = a.st.tabs[0].as_ref().unwrap().doc.design().objects().len();
        // The app stops by force: no clean exit.
        drop(a);

        // A new app: the heartbeat is recent, so the session may still run.
        let mut b = Harness::new();
        b.st.backups = Backups::new(Some(root.clone()));
        assert!(find_recoverable(&root, b.st.backups.session_dir().as_deref(), 1_060).is_empty());
        // Later the session is stale: its design can be recovered.
        b.st.recovery = find_recoverable(&root, b.st.backups.session_dir().as_deref(), 1_000 + STALE_S + 1);
        assert_eq!(b.st.recovery.len(), 1);
        b.st.recover(0);
        assert_eq!(b.st.doc.design().objects().len(), n);
        assert!(b.st.doc.is_modified(), "a recovered design is not saved");
        assert!(!dir.exists(), "the crashed session folder goes when it is empty");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn saved_and_closed_designs_lose_their_backup_and_exit_cleans_up() {
        let root = root("clean");
        let mut h = Harness::new();
        h.st.backups = Backups::new(Some(root.clone()));
        h.st.write_backups(10).unwrap();
        let dir = h.st.backups.session_dir().unwrap();
        let backup = dir.join(format!("{}.embproj", h.st.doc_id));
        assert!(backup.is_file());
        // No change: no new write (the file stays as it is).
        h.st.write_backups(20).unwrap();
        assert_eq!(h.st.backups.written[&h.st.doc_id].1.written, 10);
        // Saved: the backup goes.
        let saved = root.join("saved.embproj");
        assert!(h.st.save_to(&saved));
        h.st.write_backups(30).unwrap();
        assert!(!backup.exists());
        // Closed: the backup goes.
        h.st.add_sample_objects();
        h.st.write_backups(40).unwrap();
        assert!(backup.is_file());
        h.st.close_tab();
        assert!(!backup.exists());
        // Discard deletes a recovered backup without opening it.
        h.st.add_sample_objects();
        h.st.write_backups(50).unwrap();
        let mut other = Harness::new();
        other.st.recovery = find_recoverable(&root, None, 50 + STALE_S + 1);
        assert_eq!(other.st.recovery.len(), 1);
        other.st.discard_recovery(0);
        assert!(other.st.recovery.is_empty());
        // A normal exit deletes the session folder.
        h.st.backups_clean_exit();
        assert!(!dir.exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_window_draws() {
        let mut h = Harness::new();
        h.st.recovery = vec![Recovered {
            session_dir: PathBuf::from("x"),
            entry: BackupEntry {
                id: 1,
                name: "Patch".into(),
                file: None,
                written: unix_now(),
            },
            path: PathBuf::from("x/1.embproj"),
        }];
        let st = &mut h.st;
        let mut out = h
            .ctx
            .run_ui(Default::default(), |ui| recovery_window(&ui.ctx().clone(), st));
        out.textures_delta.clear();
        assert_eq!(h.st.recovery.len(), 1);
    }
}
