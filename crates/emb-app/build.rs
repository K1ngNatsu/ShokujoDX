//! Puts the build of the app (the git commit and its date) in the program,
//! for the About window and the crash reports of test builds.

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() {
    let hash = git(&["rev-parse", "--short=7", "HEAD"]).unwrap_or_else(|| "unknown".into());
    // Changes to files that git knows, not in a commit yet.
    let changed = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    let date = git(&["log", "-1", "--format=%cs"]).unwrap_or_default();
    println!(
        "cargo:rustc-env=EMB_BUILD={hash}{}",
        if changed { "+changes" } else { "" }
    );
    println!("cargo:rustc-env=EMB_BUILD_DATE={date}");
    for p in ["../../.git/HEAD", "../../.git/refs/heads", "../../.git/index"] {
        println!("cargo:rerun-if-changed={p}");
    }
    windows_resources();
}

/// The icon of the Windows program and the data that Explorer shows in
/// its properties (the version comes from Cargo.toml).
fn windows_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    println!("cargo:rerun-if-changed=../../data/icon/app.ico");
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../data/icon/app.ico")
        .set("FileDescription", "ShokujoDX")
        .set("ProductName", "ShokujoDX")
        .set("OriginalFilename", "ShokujoDX.exe")
        .set("LegalCopyright", "GPL-3.0-or-later");
    // A computer with no Windows SDK still builds the program, with no icon.
    if let Err(e) = res.compile() {
        println!("cargo:warning=The program has no icon: {e}");
    }
}
