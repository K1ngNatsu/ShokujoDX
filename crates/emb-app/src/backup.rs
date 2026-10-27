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
        let s = SessionFile {
            heartbeat: now,
            docs: self.written.values().map(|(_, e)| e.clone()).collect(),
        };
        let json = serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?;
        let path = dir.join(SESSION_FILE);
        std::fs::write(&path, json).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// The designs of sessions in `root` (other than `own`) whose heartbeat is
/// older than [`STALE_S`] at `now`.
pub fn find_recoverable(root: &Path, own: Option<&Path>, now: u64) -> Vec<Recovered> {
    let mut out = Vec::new();
    for dir in std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
    {
        if !dir.is_dir() || Some(dir.as_path()) == own {
            continue;
        }
        let Some(s) = std::fs::read_to_string(dir.join(SESSION_FILE))
            .ok()
            .and_then(|s| serde_json::from_str::<SessionFile>(&s).ok())
        else {
            continue;
        };
        if now.saturating_sub(s.heartbeat) <= STALE_S {
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
