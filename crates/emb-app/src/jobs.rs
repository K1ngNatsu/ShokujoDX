//! The export queue (P8 Milestone 4): Output design, batch output, Send to
//! USB and the library catalog run as jobs in the background, one after
//! another, so the user can go on with the work. The Exports tab shows the
//! jobs that wait, run and are done, with their files and errors. The
//! history (the jobs that ended) is kept in `exports.json` next to the
//! settings file.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use eframe::egui;
use egui_phosphor::regular as ph;
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;
use crate::output::OutputProfile;

/// The number of ended jobs that the history keeps.
pub const HISTORY: usize = 100;

/// The work of a job. It runs in a thread of its own.
pub type Work = Box<dyn FnOnce() -> Result<JobOutput, String> + Send>;

/// What a job made.
#[derive(Debug, Default)]
pub struct JobOutput {
    pub files: Vec<PathBuf>,
    /// Details for the Exports tab (for example a line for each design of
    /// a batch).
    pub lines: Vec<String>,
    /// The message for the status line.
    pub message: String,
    /// What the program does when the job ends well.
    pub then: Option<Then>,
}

/// A change of the settings after a job ended well.
#[derive(Debug, Clone, PartialEq)]
pub enum Then {
    /// Keep the USB stick and folder of the machine (Send to USB).
    SendTarget(String, crate::send::SendTarget),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    Waiting,
    Running,
    Done,
    Failed(String),
}

/// How to run a job again from the history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Again {
    /// The output files of these project or machine files.
    Batch {
        paths: Vec<PathBuf>,
        folder: PathBuf,
        profile: OutputProfile,
        machine: Option<String>,
    },
    /// The catalog of these library designs.
    Catalog {
        paths: Vec<PathBuf>,
        title: String,
        paper: emb_render::page::Paper,
        path: PathBuf,
    },
}

/// A job in the Exports tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    /// The kind of job: "Output Design", "Send to USB", …
    pub label: String,
    /// What the job is for: a design name, "3 designs", …
    pub what: String,
    /// Where the files go.
    pub folder: Option<PathBuf>,
    pub state: JobState,
    pub files: Vec<PathBuf>,
    pub lines: Vec<String>,
    /// Local time when the job was queued and when it ended.
    pub queued: String,
    pub ended: Option<String>,
    pub again: Option<Again>,
}

impl Job {
    pub fn is_finished(&self) -> bool {
        matches!(self.state, JobState::Done | JobState::Failed(_))
    }
}

/// The queue: the jobs (oldest first) and the work of the jobs that wait.
#[derive(Default)]
pub struct Queue {
    pub jobs: Vec<Job>,
    pending: VecDeque<(u64, Work)>,
    running: Option<(u64, mpsc::Receiver<Result<JobOutput, String>>)>,
    next_id: u64,
    /// The history file (`None`: not kept).
    pub file: Option<PathBuf>,
}

impl Queue {
    /// Reads the history file. Jobs that did not end when the program
    /// stopped show as failed.
    pub fn load(file: Option<PathBuf>) -> Queue {
        let mut jobs: Vec<Job> = file
            .as_deref()
            .and_then(|f| std::fs::read(f).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        for j in &mut jobs {
            if !j.is_finished() {
                j.state = JobState::Failed("The program stopped before the job ended.".into());
            }
        }
        let next_id = jobs.iter().map(|j| j.id + 1).max().unwrap_or(1);
        Queue {
            jobs,
            pending: VecDeque::new(),
            running: None,
            next_id,
            file,
        }
    }

    fn save(&self) {
        let Some(f) = &self.file else { return };
        let ended: Vec<&Job> = self.jobs.iter().filter(|j| j.is_finished()).collect();
        if let Some(dir) = f.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(b) = serde_json::to_vec_pretty(&ended) {
            let _ = std::fs::write(f, b);
        }
    }

    /// `true` while a job runs or waits.
    pub fn busy(&self) -> bool {
        self.running.is_some() || !self.pending.is_empty()
    }

    /// The jobs that wait or run.
    pub fn active(&self) -> usize {
        self.jobs.iter().filter(|j| !j.is_finished()).count()
    }

    /// Takes a waiting job out of the queue.
    pub fn cancel(&mut self, id: u64) {
        if self.pending.iter().any(|(i, _)| *i == id) {
            self.pending.retain(|(i, _)| *i != id);
            self.jobs.retain(|j| j.id != id);
        }
    }

    /// Takes an ended job out of the history.
    pub fn remove(&mut self, id: u64) {
        self.jobs.retain(|j| j.id != id || !j.is_finished());
        self.save();
    }

    /// Takes all ended jobs out of the history.
    pub fn clear_finished(&mut self) {
        self.jobs.retain(|j| !j.is_finished());
        self.save();
    }
}

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

impl AppState {
    /// Puts a job in the queue. It starts when the jobs before it end.
    pub fn queue_job(
        &mut self,
        label: &str,
        what: &str,
        folder: Option<PathBuf>,
        again: Option<Again>,
        work: Work,
    ) -> u64 {
        let q = &mut self.jobs;
        let id = q.next_id.max(1);
        q.next_id = id + 1;
        q.jobs.push(Job {
            id,
            label: label.into(),
            what: what.into(),
            folder,
            state: JobState::Waiting,
            files: Vec::new(),
            lines: Vec::new(),
            queued: now(),
            ended: None,
            again,
        });
        q.pending.push_back((id, work));
        self.poll_jobs();
        id
    }

    /// Takes the result of the running job and starts the next one. `true`
    /// while a job runs or waits (the program paints again soon).
    pub fn poll_jobs(&mut self) -> bool {
        if let Some((id, rx)) = &self.jobs.running {
            let id = *id;
            let result = match rx.try_recv() {
                Ok(r) => Some(r),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err("The job stopped with an error.".into())),
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if let Some(r) = result {
                self.jobs.running = None;
                self.end_job(id, r);
            }
        }
        if self.jobs.running.is_none()
            && let Some((id, work)) = self.jobs.pending.pop_front()
        {
            if let Some(j) = self.jobs.jobs.iter_mut().find(|j| j.id == id) {
                j.state = JobState::Running;
            }
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(work());
            });
            self.jobs.running = Some((id, rx));
        }
        self.jobs.busy()
    }

    fn end_job(&mut self, id: u64, r: Result<JobOutput, String>) {
        let Some(k) = self.jobs.jobs.iter().position(|j| j.id == id) else {
            return;
        };
        let label = self.jobs.jobs[k].label.clone();
        match r {
            Ok(out) => {
                if let Some(Then::SendTarget(key, target)) = out.then {
                    let mut s = self.settings.clone();
                    s.send_targets.insert(key, target);
                    self.set_settings(s);
                }
                self.message = if out.message.is_empty() {
                    format!("{label}: {} files written", out.files.len())
                } else {
                    out.message
                };
                let j = &mut self.jobs.jobs[k];
                j.state = JobState::Done;
                j.files = out.files;
                j.lines = out.lines;
            }
            Err(e) => {
                self.notice = Some(Notice {
                    title: format!("{label}: not done"),
                    lines: vec![e.clone()],
                });
                self.jobs.jobs[k].state = JobState::Failed(e);
            }
        }
        self.jobs.jobs[k].ended = Some(now());
        // The history keeps the last ended jobs.
        let ended = self.jobs.jobs.iter().filter(|j| j.is_finished()).count();
        if ended > HISTORY {
            let mut drop = ended - HISTORY;
            self.jobs.jobs.retain(|j| {
                if drop > 0 && j.is_finished() {
                    drop -= 1;
                    false
                } else {
                    true
                }
            });
        }
        self.jobs.save();
    }

    /// In tests: runs the queue until all jobs ended.
    #[cfg(test)]
    pub fn wait_jobs(&mut self) {
        while self.poll_jobs() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    /// Queues the batch output of `paths` (project or machine files).
    pub fn queue_batch(
        &mut self,
        paths: Vec<PathBuf>,
        folder: PathBuf,
        profile: OutputProfile,
        machine: Option<String>,
    ) {
        let what = match paths.as_slice() {
            [one] => one
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            many => format!("{} designs", many.len()),
        };
        let again = Again::Batch {
            paths: paths.clone(),
            folder: folder.clone(),
            profile: profile.clone(),
            machine: machine.clone(),
        };
        let catalogs = self.owned_catalogs();
        let (f, n) = (folder.clone(), paths.len());
        let work: Work = Box::new(move || {
            let lines = crate::output::batch_files(&paths, &f, &profile, catalogs.view(), machine.as_deref());
            let failed = lines.iter().filter(|l| l.contains(": not written:")).count();
            let files = std::fs::read_dir(&f)
                .map(|r| r.flatten().map(|e| e.path()).collect::<Vec<_>>())
                .unwrap_or_default();
            if failed == n {
                return Err(lines.join("\n"));
            }
            Ok(JobOutput {
                message: format!(
                    "Batch output: {} of {n} designs written to {}{}",
                    n - failed,
                    f.display(),
                    if failed > 0 { " (see Exports)" } else { "" }
                ),
                files,
                lines,
                then: None,
            })
        });
        self.queue_job("Batch Output", &what, Some(folder), Some(again), work);
    }

    /// Queues the catalog of the library designs `paths`.
    pub fn queue_catalog(&mut self, paths: Vec<PathBuf>, title: String, paper: emb_render::page::Paper, path: PathBuf) {
        let entries: Vec<emb_library::Entry> = paths
            .iter()
            .filter_map(|p| self.library.index.entry(p).cloned())
            .collect();
        let again = Again::Catalog {
            paths,
            title: title.clone(),
            paper,
            path: path.clone(),
        };
        let p = path.clone();
        let n = entries.len();
        let work: Work = Box::new(move || {
            crate::library::write_catalog(&entries, &title, paper, &p)?;
            Ok(JobOutput {
                message: format!("Wrote the catalog {} ({n} designs)", p.display()),
                files: vec![p],
                ..Default::default()
            })
        });
        self.queue_job(
            "Catalog",
            &format!("{n} designs"),
            path.parent().map(Path::to_path_buf),
            Some(again),
            work,
        );
    }

    /// Runs an ended job again.
    pub fn run_again(&mut self, id: u64) {
        let Some(again) = self.jobs.jobs.iter().find(|j| j.id == id).and_then(|j| j.again.clone()) else {
            return;
        };
        match again {
            Again::Batch {
                paths,
                folder,
                profile,
                machine,
            } => self.queue_batch(paths, folder, profile, machine),
            Again::Catalog {
                paths,
                title,
                paper,
                path,
            } => self.queue_catalog(paths, title, paper, path),
        }
    }
}

/// The Exports tab.
pub fn panel(ui: &mut egui::Ui, st: &mut AppState) {
    if st.poll_jobs() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
    ui.horizontal(|ui| {
        let active = st.jobs.active();
        let ended = st.jobs.jobs.len() - active;
        ui.weak(match active {
            0 => "No job runs.".to_string(),
            n => format!("{n} job{} in the queue", if n == 1 { "" } else { "s" }),
        });
        if ui
            .add_enabled(ended > 0, egui::Button::new(tr("Clear history")).small())
            .on_hover_text(tr("Take the ended jobs off the list (no file is deleted)."))
            .clicked()
        {
            st.jobs.clear_finished();
        }
    });
    ui.separator();
    if st.jobs.jobs.is_empty() {
        ui.weak(tr(
            "Output design, batch output, Send to USB and the library catalog show here while they run, and after.",
        ));
        return;
    }
    let mut action: Option<(u64, &str)> = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        for j in st.jobs.jobs.iter().rev() {
            ui.push_id(j.id, |ui| {
                ui.horizontal(|ui| {
                    match &j.state {
                        JobState::Waiting => {
                            ui.label(ph::HOURGLASS)
                                .on_hover_text(tr("Waits for the jobs before it"));
                        }
                        JobState::Running => {
                            ui.add(egui::Spinner::new());
                        }
                        JobState::Done => {
                            ui.colored_label(egui::Color32::from_rgb(60, 170, 90), ph::CHECK_CIRCLE);
                        }
                        JobState::Failed(_) => {
                            ui.colored_label(ui.visuals().error_fg_color, ph::X_CIRCLE);
                        }
                    }
                    ui.strong(&j.label);
                    ui.label(&j.what);
                });
                let when = j.ended.as_deref().unwrap_or(&j.queued);
                let line = match &j.state {
                    JobState::Waiting => format!("Queued {when}"),
                    JobState::Running => "Running…".to_string(),
                    JobState::Done => format!("{} files · {when}", j.files.len()),
                    JobState::Failed(e) => format!("{e} · {when}"),
                };
                ui.weak(line);
                if let Some(f) = &j.folder {
                    ui.weak(f.display().to_string());
                }
                if !j.lines.is_empty() || !j.files.is_empty() {
                    egui::CollapsingHeader::new(tr("Details"))
                        .id_salt(("job-details", j.id))
                        .show(ui, |ui| {
                            for l in &j.lines {
                                ui.label(l);
                            }
                            for f in &j.files {
                                ui.weak(f.display().to_string());
                            }
                        });
                }
                ui.horizontal(|ui| {
                    if j.state == JobState::Waiting && ui.small_button(tr("Cancel")).clicked() {
                        action = Some((j.id, "cancel"));
                    }
                    if let Some(f) = j.files.first().or(j.folder.as_ref())
                        && j.is_finished()
                        && ui.small_button(tr("Show folder")).clicked()
                    {
                        crate::library::show_in_file_manager(f);
                    }
                    if j.again.is_some() && j.is_finished() && ui.small_button(tr("Run again")).clicked() {
                        action = Some((j.id, "again"));
                    }
                    if j.is_finished() && ui.small_button(tr("Remove")).clicked() {
                        action = Some((j.id, "remove"));
                    }
                });
                ui.separator();
            });
        }
    });
    match action {
        Some((id, "cancel")) => st.jobs.cancel(id),
        Some((id, "again")) => st.run_again(id),
        Some((id, "remove")) => st.jobs.remove(id),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("emb-jobs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn jobs_run_in_order_and_the_history_is_kept() {
        let dir = temp("order");
        let mut h = Harness::new();
        h.st.jobs = Queue::load(Some(dir.join("exports.json")));
        let (tx, rx) = mpsc::channel::<u32>();
        // The first job waits for a signal, so the second one must wait.
        let gate: Work = Box::new(move || {
            rx.recv().unwrap();
            Ok(JobOutput {
                message: "first".into(),
                ..Default::default()
            })
        });
        let a = h.st.queue_job("Test", "a", None, None, gate);
        let b =
            h.st.queue_job("Test", "b", None, None, Box::new(|| Err("it went wrong".into())));
        let c =
            h.st.queue_job("Test", "c", None, None, Box::new(|| Ok(JobOutput::default())));
        assert!(h.st.poll_jobs());
        let state = |st: &AppState, id: u64| st.jobs.jobs.iter().find(|j| j.id == id).map(|j| j.state.clone());
        assert_eq!(state(&h.st, a), Some(JobState::Running));
        assert_eq!(state(&h.st, b), Some(JobState::Waiting));
        // A waiting job can be cancelled.
        h.st.jobs.cancel(c);
        assert_eq!(state(&h.st, c), None);
        tx.send(1).unwrap();
        h.st.wait_jobs();
        assert_eq!(state(&h.st, a), Some(JobState::Done));
        assert_eq!(state(&h.st, b), Some(JobState::Failed("it went wrong".into())));
        assert!(h.st.notice.as_ref().unwrap().lines[0].contains("went wrong"));
        // The history file has the two ended jobs, newest last.
        let back = Queue::load(Some(dir.join("exports.json")));
        assert_eq!(back.jobs.len(), 2);
        assert_eq!(back.jobs[1].what, "b");
        assert!(back.next_id > b);
        let mut back = back;
        back.remove(a);
        assert_eq!(back.jobs.len(), 1);
        back.clear_finished();
        assert!(Queue::load(Some(dir.join("exports.json"))).jobs.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_batch_job_writes_the_files_and_runs_again() {
        let dir = temp("batch");
        let mut h = Harness::new();
        assert!(h.st.save_to(&dir.join("patch.embproj")));
        let out = dir.join("out");
        let profile = h.st.settings.output.clone();
        h.st.queue_batch(vec![dir.join("patch.embproj")], out.clone(), profile, None);
        h.st.wait_jobs();
        let j = h.st.jobs.jobs.last().unwrap().clone();
        assert_eq!(j.state, JobState::Done, "{:?}", j.state);
        assert!(out.join("patch.dst").is_file() || out.join("patch.vp3").is_file());
        assert!(out.join("patch worksheet.pdf").is_file());
        assert!(h.st.message.starts_with("Batch output: 1 of 1"), "{}", h.st.message);
        std::fs::remove_dir_all(&out).unwrap();
        h.st.run_again(j.id);
        h.st.wait_jobs();
        assert_eq!(h.st.jobs.jobs.len(), 2);
        assert!(
            out.join("patch worksheet.pdf").is_file(),
            "run again writes the files again"
        );
        // A batch where no design can be read fails.
        std::fs::write(dir.join("bad.jef"), [1, 2, 3]).unwrap();
        h.st.queue_batch(vec![dir.join("bad.jef")], out.clone(), OutputProfile::default(), None);
        h.st.wait_jobs();
        assert!(matches!(h.st.jobs.jobs.last().unwrap().state, JobState::Failed(_)));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_job_that_did_not_end_shows_as_failed_after_a_restart() {
        let dir = temp("restart");
        let f = dir.join("exports.json");
        let job = Job {
            id: 7,
            label: "Output Design".into(),
            what: "rose".into(),
            folder: None,
            state: JobState::Running,
            files: Vec::new(),
            lines: Vec::new(),
            queued: "2026-10-23 10:00:00".into(),
            ended: None,
            again: None,
        };
        std::fs::write(&f, serde_json::to_vec(&vec![job]).unwrap()).unwrap();
        let q = Queue::load(Some(f));
        assert!(matches!(q.jobs[0].state, JobState::Failed(_)));
        assert_eq!(q.next_id, 8);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_tab_draws() {
        let mut h = Harness::new();
        h.st.queue_job("Test", "a", None, None, Box::new(|| Ok(JobOutput::default())));
        h.st.wait_jobs();
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| panel(ui, &mut h.st));
        });
        out.textures_delta.clear();
    }
}
