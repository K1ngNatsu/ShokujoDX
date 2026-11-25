//! One program (P22 Milestone 8, ADR 0175), with two processes: a second
//! program that starts with a file gives the file to the program that
//! runs (the inbox of its session) and stops, in less than 2 s, before it
//! opens a window. The first program is a session folder with a new
//! heartbeat, as a program that runs writes it; that it opens the file in
//! a tab is the test `one_program::tests` of the program.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The configuration folder of the processes and the folder of the
/// program in it (as `workspace::settings_path`).
fn config(base: &Path) -> (&'static str, PathBuf, PathBuf) {
    if cfg!(windows) {
        ("APPDATA", base.to_path_buf(), base.join("ShokujoDX"))
    } else if cfg!(target_os = "macos") {
        (
            "HOME",
            base.to_path_buf(),
            base.join("Library/Application Support/ShokujoDX"),
        )
    } else {
        ("XDG_CONFIG_HOME", base.to_path_buf(), base.join("shokujodx"))
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[test]
fn a_second_program_gives_its_file_to_the_first_and_stops() {
    let base = std::env::temp_dir().join(format!("emb-one-program-{}", std::process::id()));
    std::fs::remove_dir_all(&base).ok();
    let (var, value, folder) = config(&base);
    // The first program: its session with a new heartbeat.
    let session = folder.join("backups").join("1-1");
    std::fs::create_dir_all(&session).unwrap();
    std::fs::write(
        session.join("session.json"),
        format!("{{\"heartbeat\": {}, \"docs\": []}}", now()),
    )
    .unwrap();
    let file = base.join("design.embproj");
    std::fs::write(&file, "{}").unwrap();
    // The second program.
    let start = Instant::now();
    let status = Command::new(env!("CARGO_BIN_EXE_emb-app"))
        .arg(&file)
        .env(var, &value)
        .status()
        .expect("the program starts");
    let took = start.elapsed();
    assert!(status.success(), "{status:?}");
    assert!(took < Duration::from_secs(2), "{took:?}");
    // The path of the file is in the inbox of the first program.
    let entries: Vec<PathBuf> = std::fs::read_dir(session.join("inbox"))
        .expect("an inbox")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("path"))
        .collect();
    assert_eq!(entries.len(), 1, "{entries:?}");
    let given = std::fs::read_to_string(&entries[0]).unwrap();
    assert_eq!(PathBuf::from(given.trim()), std::path::absolute(&file).unwrap());
    std::fs::remove_dir_all(&base).ok();
}
