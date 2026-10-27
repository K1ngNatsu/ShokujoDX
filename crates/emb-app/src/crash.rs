//! Crash reports for test builds: when the program stops because of an
//! error, a text file with the error, the place in the code and the build
//! goes to the `crashes` folder next to the settings file. At the next
//! start the program says where the report is, so the user can send it.
//! The automatic backups (`backup`) keep the work.

use std::path::{Path, PathBuf};

/// The folder of the crash reports.
pub fn crashes_dir() -> Option<PathBuf> {
    crate::about::user_folder().map(|d| d.join("crashes"))
}

/// The text of a crash report.
pub fn report_text(message: &str, location: &str, backtrace: &str) -> String {
    format!(
        "ShokujoDX crash report\n\
         Program: {}\n\
         Time: {}\n\
         Error: {message}\n\
         Place: {location}\n\n\
         Please send this file with your feedback form. It has no design data.\n\n\
         Backtrace:\n{backtrace}\n",
        crate::about::build_text(),
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
    )
}

/// Writes a crash report to `dir` as `crash-<time>.txt`.
pub fn write_report(dir: &Path, text: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let mut path = dir.join(format!("crash-{stamp}.txt"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("crash-{stamp}-{n}.txt"));
        n += 1;
    }
    std::fs::write(&path, text)?;
    Ok(path)
}

/// Writes a report when a panic stops the program, then lets the normal
/// panic output follow.
pub fn install() {
    let Some(dir) = crashes_dir() else { return };
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".into());
        let location = info
            .location()
            .map_or_else(|| "(unknown)".into(), |l| format!("{}:{}", l.file(), l.line()));
        let backtrace = std::backtrace::Backtrace::force_capture().to_string();
        let _ = write_report(&dir, &report_text(&message, &location, &backtrace));
        before(info);
    }));
}

/// The reports that the user has not seen yet. Each one is renamed to
/// `…-seen.txt`, so it shows once; the new paths are returned.
pub fn take_unseen(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("crash-") && n.ends_with(".txt") && !n.ends_with("-seen.txt"))
        })
        .filter_map(|p| {
            let stem = p.file_stem()?.to_string_lossy().into_owned();
            let seen = p.with_file_name(format!("{stem}-seen.txt"));
            std::fs::rename(&p, &seen).ok().map(|_| seen)
        })
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_is_written_and_shown_once() {
        let dir = std::env::temp_dir().join(format!("emb-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let text = report_text("index out of bounds", "crates/emb-app/src/x.rs:10", "0: main");
        assert!(text.contains("Error: index out of bounds"));
        assert!(text.contains(&crate::about::build_text()));
        let a = write_report(&dir, &text).unwrap();
        let b = write_report(&dir, &text).unwrap();
        assert_ne!(a, b, "two reports in one second keep both");
        let seen = take_unseen(&dir);
        assert_eq!(seen.len(), 2);
        assert!(seen.iter().all(|p| p.to_string_lossy().ends_with("-seen.txt")));
        assert_eq!(std::fs::read_to_string(&seen[0]).unwrap(), text);
        assert!(take_unseen(&dir).is_empty(), "shown once");
        std::fs::remove_dir_all(&dir).ok();
    }
}
