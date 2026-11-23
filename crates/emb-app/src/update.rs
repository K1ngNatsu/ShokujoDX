//! The check for a new version (P17 Milestone 1, ADR 0109).
//!
//! The program asks GitHub for the newest release: at the start (not more
//! than one time each day, and only if the user permits it), or when the
//! user clicks Help > Check for Updates. It shows the version, the notes
//! and a link to the download page. It does not download and does not
//! install a file.
//!
//! The `curl` program of the system makes the request, in a thread
//! (Windows 10 and later, macOS and most Linux systems have it). So the
//! program has no network code of its own. The request has no data of the
//! user and no data of a design.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The address that gives the newest release of the program (JSON).
pub const LATEST_API: &str = "https://api.github.com/repos/K1ngNatsu/ShokujoDX/releases/latest";
/// The page with the releases of the program.
pub const RELEASES_PAGE: &str = "https://github.com/K1ngNatsu/ShokujoDX/releases";
/// The file next to the program in which the Windows installer puts the
/// choices of the user (`packaging/windows/installer.iss`).
pub const INSTALL_FILE: &str = "install.ini";
/// The key of the choice "check for a new version at the start" in
/// [`INSTALL_FILE`]. Its value is `yes` or `no`.
pub const INSTALL_KEY: &str = "update_check";
/// The time between two automatic checks, in seconds.
const DAY_S: u64 = 24 * 60 * 60;
/// The longest notes of a release that the window shows, in characters.
const MAX_NOTES: usize = 4000;
/// The largest answer that the program reads, in bytes.
const MAX_REPLY: usize = 1_000_000;
/// The longest time of one request, in seconds.
const MAX_TIME_S: u32 = 15;

/// The settings of the check. They are a part of the user settings.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateSettings {
    /// Check at the start. `None`: the user did not answer yet, and the
    /// program does not check.
    pub auto: Option<bool>,
    /// The time of the last check that got an answer (seconds from 1970).
    pub last_check: u64,
    /// A version that the user does not want a notice of.
    pub skip: String,
    /// The time of the install file whose choice was applied (seconds from
    /// 1970), so each install applies its choice one time.
    pub installer_stamp: u64,
    /// The version of the program at the last start, for "what is new".
    pub seen_version: String,
}

/// A release of the program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The version, for example `0.4.0`.
    pub version: String,
    /// The page of the release.
    pub url: String,
    /// The notes of the release, as plain text.
    pub notes: String,
}

/// Why a check gave no release. The window makes the text, because the
/// check runs in a thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The system has no `curl` program.
    NoCurl,
    /// The request failed; the text is the message of `curl`.
    Request(String),
    /// The answer was not the description of a release.
    Reply,
}

/// The numbers of a version text: `v0.3.0`, `0.3`, `1.2.3-beta`.
pub fn version(text: &str) -> Option<[u32; 3]> {
    let t = text.trim();
    let t = t.strip_prefix(['v', 'V']).unwrap_or(t);
    let t = t.split(['-', '+']).next().unwrap_or(t);
    let parts: Vec<&str> = t.split('.').collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    let mut v = [0; 3];
    for (i, p) in parts.iter().enumerate() {
        if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        v[i] = p.parse().ok()?;
    }
    Some(v)
}

/// True if `release` is a later version than `current`. Text that is not
/// a version is never newer.
pub fn newer(release: &str, current: &str) -> bool {
    match (version(release), version(current)) {
        (Some(r), Some(c)) => r > c,
        _ => false,
    }
}

/// The page of a release, if it is a page of our releases and has only
/// plain characters. The system opens the address, so no other address
/// and no other character can come from the answer.
fn safe_url(url: &str) -> Option<&str> {
    let rest = url.strip_prefix(RELEASES_PAGE)?;
    let plain = rest
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'-' | b'_'));
    (rest.starts_with('/') && plain && url.len() <= 200).then_some(url)
}

/// The notes of a release as plain text of a limited length.
fn clean_notes(body: &str) -> String {
    let mut out: String = body
        .chars()
        .filter(|c| *c == '\n' || !c.is_control())
        .take(MAX_NOTES)
        .collect();
    if body.chars().filter(|c| *c == '\n' || !c.is_control()).count() > MAX_NOTES {
        out.push('…');
    }
    out.trim().to_string()
}

/// Reads the answer of GitHub (the JSON of one release).
pub fn parse_release(json: &str) -> Result<Release, Problem> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|_| Problem::Reply)?;
    let tag = v.get("tag_name").and_then(|t| t.as_str()).ok_or(Problem::Reply)?;
    let numbers = version(tag).ok_or(Problem::Reply)?;
    let flag = |key: &str| v.get(key).and_then(serde_json::Value::as_bool).unwrap_or(false);
    if flag("draft") || flag("prerelease") {
        return Err(Problem::Reply);
    }
    let url = v
        .get("html_url")
        .and_then(|u| u.as_str())
        .and_then(safe_url)
        .unwrap_or(RELEASES_PAGE);
    let notes = v.get("body").and_then(|b| b.as_str()).unwrap_or_default();
    Ok(Release {
        version: format!("{}.{}.{}", numbers[0], numbers[1], numbers[2]),
        url: url.to_string(),
        notes: clean_notes(notes),
    })
}

/// True if an automatic check is to be made now.
pub fn due(s: &UpdateSettings, now: u64) -> bool {
    // A clock that went back does not stop the checks.
    s.auto == Some(true) && (s.last_check > now || now - s.last_check >= DAY_S)
}

/// The choice in the text of an install file: `update_check=yes` or `no`.
pub fn parse_install_file(text: &str) -> Option<bool> {
    text.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        if !key.trim().eq_ignore_ascii_case(INSTALL_KEY) {
            return None;
        }
        match value.trim().to_ascii_lowercase().as_str() {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        }
    })
}

/// The choice of the install file next to the program `exe`, and the time
/// of the file. `None` for a package with no installer.
fn installer_choice_at(exe: &Path) -> Option<(bool, u64)> {
    let file = exe.parent()?.join(INSTALL_FILE);
    let meta = std::fs::metadata(&file).ok()?;
    if meta.len() > 4096 {
        return None;
    }
    let choice = parse_install_file(&String::from_utf8_lossy(&std::fs::read(&file).ok()?))?;
    let stamp = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(1, |d| d.as_secs().max(1));
    Some((choice, stamp))
}

/// The choice that the user made in the installer, if this program came
/// from an installer.
pub fn installer_choice() -> Option<(bool, u64)> {
    installer_choice_at(&std::env::current_exe().ok()?)
}

/// Applies the choice of an install one time. "No" turns the checks off.
/// "Yes" turns them on only if the user did not answer before: a user who
/// turned the checks off in Options keeps them off after an upgrade.
/// Returns true if the settings changed.
pub fn apply_installer(s: &mut UpdateSettings, choice: bool, stamp: u64) -> bool {
    if s.installer_stamp == stamp {
        return false;
    }
    s.installer_stamp = stamp;
    if !choice {
        s.auto = Some(false);
    } else if s.auto.is_none() {
        s.auto = Some(true);
    }
    true
}

/// The `curl` program of the system, by its full path: a program of the
/// same name in another folder is never started.
fn curl_path() -> Option<PathBuf> {
    let places: Vec<PathBuf> = if cfg!(windows) {
        let root = std::env::var_os("SystemRoot").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        vec![root.join("System32").join("curl.exe")]
    } else {
        vec![PathBuf::from("/usr/bin/curl"), PathBuf::from("/bin/curl")]
    };
    places.into_iter().find(|p| p.is_file())
}

/// Asks GitHub for the newest release. It can take [`MAX_TIME_S`]: call it
/// in a thread.
pub fn fetch() -> Result<Release, Problem> {
    let curl = curl_path().ok_or(Problem::NoCurl)?;
    let mut c = std::process::Command::new(curl);
    c.args(["--silent", "--show-error", "--fail", "--location"])
        .args(["--proto", "=https", "--proto-redir", "=https"])
        .args(["--max-time", &MAX_TIME_S.to_string()])
        .args(["--max-filesize", &MAX_REPLY.to_string()])
        .args(["--user-agent", "ShokujoDX"])
        .args(["--header", "Accept: application/vnd.github+json"])
        .arg(LATEST_API)
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        // No console window shows for a moment.
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    let out = c.output().map_err(|e| Problem::Request(e.to_string()))?;
    if !out.status.success() {
        let text = String::from_utf8_lossy(&out.stderr);
        let line: String = text.lines().next().unwrap_or_default().chars().take(200).collect();
        return Err(Problem::Request(line));
    }
    if out.stdout.len() > MAX_REPLY {
        return Err(Problem::Reply);
    }
    parse_release(&String::from_utf8_lossy(&out.stdout))
}

/// The state of a check.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Status {
    /// No check was made in this session.
    #[default]
    Idle,
    Checking,
    /// The program is the newest version.
    Newest,
    /// A newer version exists.
    Found(Release),
    Failed(Problem),
}

/// The check and its windows.
#[derive(Default)]
pub struct Updates {
    /// The Check for Updates window is open.
    pub window: bool,
    pub status: Status,
    /// The user asked for the check that runs (so each result shows).
    manual: bool,
    rx: Option<Receiver<Result<Release, Problem>>>,
    /// The question "check at the start?" is open (for a user who did not
    /// answer it in an installer or at the first start).
    pub ask: bool,
    /// This is the first start of a new version: the start page has a link
    /// to "What is new".
    pub whats_new: bool,
}

impl Updates {
    /// Starts a check in a thread, if none runs. `fetch` makes the request.
    pub fn check<F>(&mut self, manual: bool, ctx: &egui::Context, fetch: F)
    where
        F: FnOnce() -> Result<Release, Problem> + Send + 'static,
    {
        self.manual |= manual;
        if self.rx.is_some() {
            return;
        }
        self.manual = manual;
        self.status = Status::Checking;
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let ctx = ctx.clone();
        crate::background::spawn(move || {
            let _ = tx.send(fetch());
            ctx.request_repaint();
        });
    }

    /// True while a check runs.
    pub fn checking(&self) -> bool {
        self.rx.is_some()
    }
}

/// The program checks for new versions at the start and asks about it.
/// The browser version (P15 trial) does not: it is always the newest build
/// of its page, and it cannot start `curl`.
pub const CHECK_AT_START: bool = !cfg!(target_arch = "wasm32");

/// The check of the browser version: the build of the page against the
/// newest build on the server (`web::check_for_update`).
#[cfg(target_arch = "wasm32")]
fn page_check(ctx: &egui::Context) {
    crate::web::check_for_update(ctx);
}

#[cfg(not(target_arch = "wasm32"))]
fn page_check(_ctx: &egui::Context) {}

impl AppState {
    /// The work of the start: the choice of the installer, the first start
    /// of a new version, the question for a user who did not answer yet,
    /// and the automatic check if it is on and a day went by.
    pub fn start_updates(&mut self, ctx: &egui::Context) {
        if !CHECK_AT_START {
            return;
        }
        self.start_updates_with(ctx, installer_choice(), crate::backup::unix_now(), fetch);
    }

    pub(crate) fn start_updates_with<F>(
        &mut self,
        ctx: &egui::Context,
        installer: Option<(bool, u64)>,
        now: u64,
        fetch: F,
    ) where
        F: FnOnce() -> Result<Release, Problem> + Send + 'static,
    {
        let mut s = self.settings.clone();
        if let Some((choice, stamp)) = installer {
            apply_installer(&mut s.updates, choice, stamp);
        }
        if s.updates.seen_version != crate::about::VERSION {
            // A user who has a machine used an earlier version.
            self.updates.whats_new = s.machine.is_some();
            s.updates.seen_version = crate::about::VERSION.into();
        }
        self.set_settings(s);
        // The first start asks in its own window; the kiosk does not ask.
        self.updates.ask = self.settings.updates.auto.is_none() && self.ask_machine.is_none() && self.kiosk.is_none();
        if due(&self.settings.updates, now) {
            self.updates.check(false, ctx, fetch);
        }
    }

    /// Help > Check for Updates: opens the window and checks now.
    pub fn check_updates_now(&mut self, ctx: &egui::Context) {
        if !CHECK_AT_START {
            page_check(ctx);
            return;
        }
        self.updates.window = true;
        self.updates.check(true, ctx, fetch);
    }

    /// Takes the result of a check that ended. A newer version opens the
    /// window, unless the user skipped that version; an automatic check
    /// with no news or with an error shows nothing.
    pub fn poll_updates(&mut self, now: u64) {
        let Some(rx) = &self.updates.rx else { return };
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(Problem::Request(String::new())),
        };
        self.updates.rx = None;
        match result {
            Ok(release) => {
                let mut s = self.settings.clone();
                s.updates.last_check = now;
                self.set_settings(s);
                if newer(&release.version, crate::about::VERSION) {
                    if self.updates.manual || release.version != self.settings.updates.skip {
                        self.updates.window = true;
                    }
                    self.updates.status = Status::Found(release);
                } else {
                    self.updates.status = Status::Newest;
                }
            }
            Err(p) => self.updates.status = Status::Failed(p),
        }
    }

    /// Sets "check at the start" (Options, the windows of this file).
    pub fn set_update_check(&mut self, on: bool) {
        let mut s = self.settings.clone();
        s.updates.auto = Some(on);
        self.set_settings(s);
    }
}

/// What the check does, for each place that asks the user.
pub fn what_it_does() -> &'static str {
    tr(
        "The check asks GitHub for the newest version of ShokujoDX. It sends no data about you or your designs, and it does not download or install anything.",
    )
}

/// The text of a problem.
fn problem_text(p: &Problem) -> String {
    match p {
        Problem::NoCurl => tr("This computer has no curl program, which the check uses.").into(),
        Problem::Request(detail) if detail.is_empty() => tr("The request got no answer.").into(),
        Problem::Request(detail) => fill("The request failed: {}", &[detail]),
        Problem::Reply => tr("The answer was not the description of a version.").into(),
    }
}

/// Opens the page of the releases in the browser of the system: the font
/// packages are there (ADR 0115). The program downloads nothing.
pub fn open_releases_page(st: &mut AppState) {
    open_page(st, RELEASES_PAGE);
}

/// Opens a page of the releases in the browser of the system.
fn open_page(st: &mut AppState, url: &str) {
    let url = safe_url(url).unwrap_or(RELEASES_PAGE);
    if let Err(e) = crate::share::open_with_system(Path::new(url)) {
        st.message = fill("The page did not open: {}", &[&e]);
    }
}

/// The windows of the check: the question, and Check for Updates.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    st.poll_updates(crate::backup::unix_now());
    ask_window(ctx, st);
    if !st.updates.window {
        return;
    }
    let mut open = true;
    let mut page: Option<String> = None;
    let mut skip: Option<String> = None;
    let mut again = false;
    let mut auto = st.settings.updates.auto == Some(true);
    let was_auto = auto;
    egui::Window::new(tr("Check for Updates"))
        .id(egui::Id::new("Check for Updates"))
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.set_max_width(440.0);
            let current = crate::about::VERSION;
            match &st.updates.status {
                Status::Idle => {
                    ui.label(fill("You have version {}.", &[&current]));
                }
                Status::Checking => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr("Asking GitHub for the newest version…"));
                    });
                }
                Status::Newest => {
                    ui.heading(tr("You have the newest version"));
                    ui.label(fill("You have version {}.", &[&current]));
                }
                Status::Found(release) => {
                    ui.heading(fill("Version {} is available", &[&release.version]));
                    ui.label(fill("You have version {}.", &[&current]));
                    if !release.notes.is_empty() {
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                            ui.label(&release.notes);
                        });
                    }
                    ui.add_space(4.0);
                    ui.weak(tr(
                        "ShokujoDX does not download or install the new version. Get it from the download page and install it as before. Your settings and your designs stay.",
                    ));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(tr("Open Download Page"))
                            .on_hover_text(tr("Open the page of this version in your browser."))
                            .clicked()
                        {
                            page = Some(release.url.clone());
                        }
                        if ui
                            .button(tr("Skip This Version"))
                            .on_hover_text(tr("Show no notice of this version at the start."))
                            .clicked()
                        {
                            skip = Some(release.version.clone());
                        }
                    });
                }
                Status::Failed(p) => {
                    ui.heading(tr("The check did not work"));
                    ui.label(problem_text(p));
                    ui.label(tr("You can look at the download page for a new version."));
                    if ui.button(tr("Open Download Page")).clicked() {
                        page = Some(RELEASES_PAGE.into());
                    }
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!st.updates.checking(), egui::Button::new(tr("Check Now")))
                    .on_hover_text(what_it_does())
                    .clicked()
                {
                    again = true;
                }
                ui.checkbox(&mut auto, tr("Check at the start"))
                    .on_hover_text(tr(
                        "At the start, and not more than one time each day, look for a new version.",
                    ));
            });
        });
    if let Some(url) = page {
        open_page(st, &url);
    }
    if let Some(v) = skip {
        let mut s = st.settings.clone();
        s.updates.skip = v;
        st.set_settings(s);
        open = false;
    }
    if auto != was_auto {
        st.set_update_check(auto);
    }
    if again {
        st.check_updates_now(ctx);
    }
    st.updates.window = open && st.updates.window;
}

/// The question for a user who did not answer in an installer or at the
/// first start (a user of an earlier version, or a package with no
/// installer). It shows one time.
fn ask_window(ctx: &egui::Context, st: &mut AppState) {
    if !st.updates.ask || st.ask_machine.is_some() {
        return;
    }
    let mut answer: Option<bool> = None;
    egui::Window::new(tr("New Versions"))
        .id(egui::Id::new("New Versions"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.heading(tr("Look for new versions at the start?"));
            ui.label(what_it_does());
            ui.weak(tr("You can change this later in Options."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(tr("Check at the Start")).clicked() {
                    answer = Some(true);
                }
                if ui.button(tr("Do Not Check")).clicked() {
                    answer = Some(false);
                }
            });
        });
    if let Some(on) = answer {
        st.set_update_check(on);
        st.updates.ask = false;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use emb_model::{HoopCatalog, MachineCatalog};

    use super::*;

    fn state() -> AppState {
        AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new())
    }

    fn release(version: &str) -> Release {
        Release {
            version: version.into(),
            url: format!("{RELEASES_PAGE}/tag/v{version}"),
            notes: "Better fonts.".into(),
        }
    }

    /// Waits for the thread of a check and takes its result.
    fn finish(st: &mut AppState, now: u64) {
        for _ in 0..500 {
            st.poll_updates(now);
            if !st.updates.checking() {
                return;
            }
            std::thread::sleep(crate::clock::Duration::from_millis(2));
        }
        panic!("the check did not end");
    }

    #[test]
    fn versions_compare_by_their_numbers() {
        assert_eq!(version("v0.3.0"), Some([0, 3, 0]));
        assert_eq!(version("0.10"), Some([0, 10, 0]));
        assert_eq!(version(" V1.2.3-beta.1 "), Some([1, 2, 3]));
        for bad in ["", "v", "1..2", "1.2.3.4", "one", "1.-2", "1.2x", "99999999999"] {
            assert_eq!(version(bad), None, "{bad:?}");
        }
        assert!(newer("v0.10.0", "0.9.9"));
        assert!(newer("0.3.1", "0.3.0"));
        assert!(!newer("0.3.0", "0.3.0"));
        assert!(!newer("0.2.9", "0.3.0"));
        assert!(!newer("newest", "0.3.0"));
    }

    #[test]
    fn the_answer_of_github_gives_a_release() {
        let json = r#"{"tag_name":"v0.4.0","draft":false,"prerelease":false,
            "html_url":"https://github.com/K1ngNatsu/ShokujoDX/releases/tag/v0.4.0",
            "body":"New:\r\n- Tuned fonts\u0007"}"#;
        let r = parse_release(json).unwrap();
        assert_eq!(r.version, "0.4.0");
        assert_eq!(r.url, format!("{RELEASES_PAGE}/tag/v0.4.0"));
        assert_eq!(r.notes, "New:\n- Tuned fonts");
        for bad in [
            "",
            "not json",
            "[]",
            r#"{"tag_name":7}"#,
            r#"{"tag_name":"latest"}"#,
            r#"{"tag_name":"v0.4.0","draft":true}"#,
            r#"{"tag_name":"v0.4.0","prerelease":true}"#,
            r#"{"message":"API rate limit exceeded"}"#,
        ] {
            assert_eq!(parse_release(bad), Err(Problem::Reply), "{bad:?}");
        }
    }

    /// The system opens the address of a release, so an answer cannot give
    /// another site, a file or a command.
    #[test]
    fn only_a_page_of_our_releases_opens() {
        for bad in [
            "https://example.com/setup.exe",
            "https://github.com/K1ngNatsu/ShokujoDX/releases-evil/x",
            "https://github.com/K1ngNatsu/ShokujoDX/releases/tag/v1\" & calc & \"",
            "https://github.com/K1ngNatsu/ShokujoDX/releases/tag/v1?x=%26calc",
            "file:///C:/Windows/System32/calc.exe",
            "",
        ] {
            let json = serde_json::json!({"tag_name": "v9.0.0", "html_url": bad}).to_string();
            assert_eq!(parse_release(&json).unwrap().url, RELEASES_PAGE, "{bad:?}");
        }
        let long = format!("{RELEASES_PAGE}/tag/{}", "a".repeat(300));
        assert_eq!(safe_url(&long), None);
        assert!(safe_url(&format!("{RELEASES_PAGE}/tag/v0.4.0")).is_some());
    }

    #[test]
    fn long_notes_are_cut() {
        let json = serde_json::json!({"tag_name": "v9.0.0", "body": "é".repeat(MAX_NOTES + 50)}).to_string();
        let notes = parse_release(&json).unwrap().notes;
        assert_eq!(notes.chars().count(), MAX_NOTES + 1);
        assert!(notes.ends_with('…'));
    }

    #[test]
    fn a_check_is_due_one_time_each_day_and_only_if_it_is_on() {
        let mut s = UpdateSettings::default();
        assert!(!due(&s, 1_000_000), "no answer of the user: no check");
        s.auto = Some(false);
        assert!(!due(&s, 1_000_000));
        s.auto = Some(true);
        assert!(due(&s, 1_000_000));
        s.last_check = 1_000_000;
        assert!(!due(&s, 1_000_000 + DAY_S - 1));
        assert!(due(&s, 1_000_000 + DAY_S));
        assert!(due(&s, 500), "the clock went back");
    }

    #[test]
    fn the_install_file_gives_the_choice() {
        assert_eq!(parse_install_file("[install]\r\nupdate_check=yes\r\n"), Some(true));
        assert_eq!(parse_install_file("[install]\nUpdate_Check = No\n"), Some(false));
        assert_eq!(parse_install_file("[install]\nupdate_check=maybe\n"), None);
        assert_eq!(parse_install_file("[install]\nother=yes\n"), None);
        assert_eq!(parse_install_file(""), None);

        let dir = std::env::temp_dir().join(format!("shokujodx-install-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("ShokujoDX.exe");
        assert_eq!(installer_choice_at(&exe), None, "a package with no installer");
        std::fs::write(dir.join(INSTALL_FILE), "[install]\nupdate_check=no\n").unwrap();
        let (choice, stamp) = installer_choice_at(&exe).unwrap();
        assert!(!choice);
        assert!(stamp > 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn each_install_applies_its_choice_one_time_and_off_stays_off() {
        // A new user who left the box ticked.
        let mut s = UpdateSettings::default();
        assert!(apply_installer(&mut s, true, 10));
        assert_eq!(s.auto, Some(true));
        // The user turns the checks off in Options; the same install does
        // not turn them on again, and an upgrade with the box ticked does
        // not turn them on.
        s.auto = Some(false);
        assert!(!apply_installer(&mut s, true, 10));
        assert_eq!(s.auto, Some(false));
        assert!(apply_installer(&mut s, true, 20));
        assert_eq!(s.auto, Some(false));
        // An install with the box clear turns the checks off.
        s.auto = Some(true);
        assert!(apply_installer(&mut s, false, 30));
        assert_eq!(s.auto, Some(false));
        // The user turns them on in Options; they stay on.
        s.auto = Some(true);
        assert!(!apply_installer(&mut s, false, 30));
        assert_eq!(s.auto, Some(true));
    }

    /// With the checks off, or with no answer of the user, the start makes
    /// no request.
    #[test]
    fn the_start_makes_no_request_unless_the_user_permits_it() {
        let ctx = egui::Context::default();
        for auto in [None, Some(false)] {
            let mut st = state();
            st.settings.updates.auto = auto;
            let calls = Arc::new(AtomicUsize::new(0));
            let seen = calls.clone();
            st.start_updates_with(&ctx, None, 1_000_000, move || {
                seen.fetch_add(1, Ordering::SeqCst);
                Ok(release("9.0.0"))
            });
            assert!(!st.updates.checking(), "{auto:?}");
            assert_eq!(st.updates.status, Status::Idle);
            std::thread::sleep(crate::clock::Duration::from_millis(20));
            assert_eq!(calls.load(Ordering::SeqCst), 0, "{auto:?}");
        }
        // The installer said no: no request, and no question.
        let mut st = state();
        st.settings.machine = Some("generic".into());
        st.ask_machine = None;
        st.start_updates_with(&ctx, Some((false, 5)), 1_000_000, || panic!("a request"));
        assert_eq!(st.settings.updates.auto, Some(false));
        assert!(!st.updates.ask && !st.updates.checking());
    }

    #[test]
    fn an_automatic_check_shows_only_a_newer_version() {
        let ctx = egui::Context::default();
        let now = 2_000_000;
        // A newer version: the window opens, and the time is kept.
        let mut st = state();
        st.settings.updates.auto = Some(true);
        st.start_updates_with(&ctx, None, now, || Ok(release("99.0.0")));
        assert!(st.updates.checking());
        finish(&mut st, now);
        assert_eq!(st.updates.status, Status::Found(release("99.0.0")));
        assert!(st.updates.window);
        assert_eq!(st.settings.updates.last_check, now);
        assert!(!due(&st.settings.updates, now + 60));
        // The same version: nothing shows.
        let mut st = state();
        st.settings.updates.auto = Some(true);
        st.start_updates_with(&ctx, None, now, || Ok(release(crate::about::VERSION)));
        finish(&mut st, now);
        assert_eq!(st.updates.status, Status::Newest);
        assert!(!st.updates.window);
        // An error: nothing shows, and the next start tries again.
        let mut st = state();
        st.settings.updates.auto = Some(true);
        st.start_updates_with(&ctx, None, now, || Err(Problem::Request("no network".into())));
        finish(&mut st, now);
        assert!(matches!(st.updates.status, Status::Failed(_)));
        assert!(!st.updates.window);
        assert!(due(&st.settings.updates, now + 60));
        // A version that the user skipped: no window at the start, but the
        // check of the Help tab shows it.
        let mut st = state();
        st.settings.updates.auto = Some(true);
        st.settings.updates.skip = "99.0.0".into();
        st.start_updates_with(&ctx, None, now, || Ok(release("99.0.0")));
        finish(&mut st, now);
        assert!(!st.updates.window);
        st.updates.check(true, &ctx, || Ok(release("99.0.0")));
        finish(&mut st, now);
        assert!(st.updates.window);
    }

    #[test]
    fn the_question_shows_to_a_user_who_did_not_answer() {
        let ctx = egui::Context::default();
        // The first start asks in its own window.
        let mut st = state();
        st.ask_machine = Some(crate::first_start::MachineStep::new(&st));
        st.start_updates_with(&ctx, None, 1, || panic!("a request"));
        assert!(!st.updates.ask);
        assert!(!st.updates.whats_new, "a new user sees no list of changes");
        // A user of an earlier version: the question, and what is new.
        let mut st = state();
        st.settings.machine = Some("generic".into());
        st.start_updates_with(&ctx, None, 1, || panic!("a request"));
        assert!(st.updates.ask && st.updates.whats_new);
        assert_eq!(st.settings.updates.seen_version, crate::about::VERSION);
        st.set_update_check(false);
        // The next start of the same version: no question, nothing new.
        let mut next = state();
        next.settings = st.settings.clone();
        next.start_updates_with(&ctx, None, 1, || panic!("a request"));
        assert!(!next.updates.ask && !next.updates.whats_new);
    }

    #[test]
    fn the_windows_draw() {
        let ctx = crate::test_util::ready_context();
        let mut st = state();
        st.settings.machine = Some("generic".into());
        st.updates.ask = true;
        st.updates.window = true;
        for status in [
            Status::Idle,
            Status::Checking,
            Status::Newest,
            Status::Found(release("99.0.0")),
            Status::Failed(Problem::NoCurl),
            Status::Failed(Problem::Request("curl: (6) Could not resolve host".into())),
            Status::Failed(Problem::Reply),
        ] {
            st.updates.status = status;
            let mut out = ctx.run_ui(Default::default(), |ui| window(ui.ctx(), &mut st));
            out.textures_delta.clear();
        }
        assert!(st.updates.window && st.updates.ask);
    }

    /// The real request. Run it by hand:
    /// `cargo test -p emb-app -- --ignored the_real_check --nocapture`.
    #[test]
    #[ignore = "uses the network"]
    fn the_real_check_gives_a_release() {
        let r = fetch().unwrap();
        println!("{r:?}");
        assert!(version(&r.version).is_some());
        assert!(r.url.starts_with(RELEASES_PAGE));
    }
}
