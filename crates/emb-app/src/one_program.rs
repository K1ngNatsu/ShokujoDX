//! One program (P22 Milestone 8, ADR 0175): a file that the user opens
//! while the program runs (a double-click, Open with, the command line)
//! opens in a tab of that program.
//!
//! The new process finds a session of the backups whose heartbeat is new
//! (`backup::running_sessions`), writes the path of the file in the inbox
//! folder of that session, and stops before it opens a window. The program
//! that runs reads its inbox each second ([`POLL`]), opens each file in a
//! tab and comes to the front. This uses a folder: no network and no
//! sockets (ADR 0109). `--new-window` starts a second program.

use crate::clock::{Duration, Instant};
use std::path::{Path, PathBuf};

use crate::app::AppState;

/// The folder in a session folder for the files of other processes.
pub const INBOX: &str = "inbox";
/// How often the program reads its inbox.
pub const POLL: Duration = Duration::from_secs(1);
/// The argument that starts a second program.
pub const NEW_WINDOW: &str = "--new-window";
/// The extension of a file in the inbox (one path in it).
const ENTRY: &str = "path";
/// How long a start waits for the program that runs to take its file: it
/// reads its inbox each [`POLL`]. The heartbeat of a program that stopped
/// (a crash) is new for up to `STALE_S`, and nothing reads its inbox: the
/// start then opens the file itself (the audit of P22).
pub const TAKE_WAIT: Duration = Duration::from_millis(2_500);

/// The file of the arguments of a start, if the start can give it to a
/// program that runs: the first argument that is not an option. A start
/// with `--new-window` or `--kiosk` keeps its file.
pub fn file_of_args(args: &[String]) -> Option<PathBuf> {
    if args.iter().any(|a| a == NEW_WINDOW || a == "--kiosk") {
        return None;
    }
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--lang" {
            it.next();
            continue;
        }
        if a.starts_with("--") {
            continue;
        }
        return Some(PathBuf::from(a));
    }
    None
}

/// Gives `file` to a program that runs: writes its path (made absolute)
/// in the inbox of a running session in `root` (the newest heartbeat at
/// `now` first), and waits [`TAKE_WAIT`] for that program to take it.
/// `false`: no program runs and takes it, or no inbox can be written.
pub fn hand_over(root: &Path, file: &Path, now: u64) -> bool {
    hand_over_within(root, file, now, TAKE_WAIT)
}

/// As [`hand_over`], with the time to wait for each session.
fn hand_over_within(root: &Path, file: &Path, now: u64, wait: Duration) -> bool {
    let file = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    for dir in crate::backup::running_sessions(root, now) {
        let Some(entry) = give(&dir, &file) else { continue };
        if taken(&entry, wait) {
            return true;
        }
    }
    false
}

/// Writes the path `file` in the inbox of the session folder `dir`: the
/// entry, or `None` if it cannot be written.
fn give(dir: &Path, file: &Path) -> Option<PathBuf> {
    let inbox = dir.join(INBOX);
    std::fs::create_dir_all(&inbox).ok()?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let name = format!("{millis}-{}", std::process::id());
    let tmp = inbox.join(format!("{name}.tmp"));
    let entry = inbox.join(format!("{name}.{ENTRY}"));
    // Written, then renamed: the program never reads half a path.
    if std::fs::write(&tmp, file.to_string_lossy().as_bytes()).is_ok() && std::fs::rename(&tmp, &entry).is_ok() {
        return Some(entry);
    }
    std::fs::remove_file(&tmp).ok();
    None
}

/// The program took the inbox `entry` in `wait`. If not, the entry goes,
/// unless the program takes it just then.
fn taken(entry: &Path, wait: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < wait {
        if !entry.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    matches!(std::fs::remove_file(entry), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
}

/// The paths in the inbox folder `inbox`, the oldest first; the entries
/// are deleted.
pub fn take_inbox(inbox: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(inbox)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some(ENTRY))
        .collect();
    entries.sort();
    entries
        .into_iter()
        .filter_map(|e| {
            let text = std::fs::read_to_string(&e).ok();
            std::fs::remove_file(&e).ok();
            let path = text?.trim().to_string();
            (!path.is_empty()).then(|| PathBuf::from(path))
        })
        .collect()
}

impl AppState {
    /// Reads the inbox of this session when it is due ([`POLL`]) and opens
    /// each file in a tab. `true` if a file came: the window comes to the
    /// front.
    pub fn read_inbox(&mut self, now: Instant) -> bool {
        if self.backups.last_inbox.is_some_and(|t| now.duration_since(t) < POLL) {
            return false;
        }
        self.backups.last_inbox = Some(now);
        let Some(inbox) = self.backups.session_dir().map(|d| d.join(INBOX)) else {
            return false;
        };
        if !inbox.is_dir() {
            return false;
        }
        let files = take_inbox(&inbox);
        for f in &files {
            self.open_any(f);
        }
        !files.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    fn root(name: &str) -> PathBuf {
        let r = std::env::temp_dir().join(format!("emb-one-program-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&r).ok();
        std::fs::create_dir_all(&r).unwrap();
        r
    }

    #[test]
    fn the_file_of_the_arguments() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            file_of_args(&a(&["design.embproj"])),
            Some(PathBuf::from("design.embproj"))
        );
        assert_eq!(
            file_of_args(&a(&["--lang", "de", "--renderer=gl", "x.dst"])),
            Some(PathBuf::from("x.dst"))
        );
        assert_eq!(file_of_args(&a(&["--new-window", "x.dst"])), None);
        assert_eq!(file_of_args(&a(&["--kiosk"])), None);
        assert_eq!(file_of_args(&a(&[])), None);
    }

    /// What another process can put in the inbox: an empty entry, text that
    /// is not a path, a path that is not there, a folder, a very long line,
    /// and entries of other kinds. The program opens nothing and stops at
    /// nothing; the entries go.
    #[test]
    fn the_inbox_takes_junk_and_opens_nothing() {
        let r = root("junk");
        let mut h = Harness::empty();
        h.st.backups = crate::backup::Backups::new(Some(r.clone()));
        h.st.write_backups(crate::backup::unix_now()).unwrap();
        let inbox = h.st.backups.session_dir().unwrap().join(INBOX);
        std::fs::create_dir_all(&inbox).unwrap();
        let entry = |name: &str, text: &[u8]| std::fs::write(inbox.join(name), text).unwrap();
        entry("1.path", b"");
        entry("2.path", &[0xff, 0xfe, 0x00, 0x41]);
        entry("3.path", b"Z:/not/there.embproj");
        entry("4.path", r.to_string_lossy().as_bytes());
        entry("5.path", "x".repeat(5_000_000).as_bytes());
        entry("6.tmp", b"half");
        let tabs = h.st.tab_count();
        let objects = h.st.doc.design().objects().len();
        h.st.read_inbox(Instant::now());
        h.frame(vec![]);
        assert_eq!(h.st.tab_count(), tabs);
        assert_eq!(h.st.doc.design().objects().len(), objects);
        let left: Vec<_> = std::fs::read_dir(&inbox)
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(left, ["6.tmp"], "the entries went; a file that is still written stays");
        std::fs::remove_dir_all(&r).ok();
    }

    /// A file given to the program that runs opens in a tab of it; a
    /// session that stopped or quit gets nothing.
    #[test]
    fn a_file_given_to_the_program_that_runs_opens_in_a_tab() {
        let r = root("open");
        let mut h = Harness::empty();
        h.st.backups = crate::backup::Backups::new(Some(r.clone()));
        // No session yet: nothing runs.
        let file = r.join("given.embproj");
        let mut d = emb_model::Design::new("Given");
        let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
        d.add_object(
            t,
            emb_model::ObjectKind::Running {
                path: emb_geom::Path::new()
                    .move_to(emb_geom::Point::ZERO)
                    .line_to(emb_geom::Point::new(10.0, 0.0)),
                params: Default::default(),
            },
        );
        emb_model::project::save(&d, &file).unwrap();
        let now = crate::backup::unix_now();
        let wait = Duration::from_millis(100);
        assert!(!hand_over_within(&r, &file, now, wait));
        // The program writes its heartbeat. A start that gives it the file
        // and finds it not taken (this program reads its inbox below) takes
        // the entry back: as for a program that stopped.
        h.st.write_backups(now).unwrap();
        assert!(!hand_over_within(&r, &file, now, wait));
        let dir = crate::backup::running_sessions(&r, now)[0].clone();
        assert!(take_inbox(&dir.join(INBOX)).is_empty(), "the entry went");
        // An old heartbeat is a program that stopped.
        assert!(!hand_over_within(&r, &file, now + crate::backup::STALE_S + 1, wait));
        // A second process gives the file (the entry, as `give` writes it).
        assert!(give(&dir, &std::path::absolute(&file).unwrap()).is_some());
        let tabs = h.st.tab_count();
        assert!(h.st.read_inbox(Instant::now()));
        assert_eq!(h.st.file_path.as_deref(), Some(file.as_path()));
        assert!(h.st.tab_count() >= tabs);
        // The entry is gone, and the next read waits for its time.
        assert!(!h.st.read_inbox(Instant::now()));
        h.st.backups.last_inbox = None;
        assert!(!h.st.read_inbox(Instant::now()));
        std::fs::remove_dir_all(&r).ok();
    }
}
