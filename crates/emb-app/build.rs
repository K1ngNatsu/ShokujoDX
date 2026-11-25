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
    help_translations();
    windows_resources();
}

/// The folders of `docs/help` that are not languages: the pictures and the
/// step files of the tutorials (P19 Part 2).
const NOT_LANGUAGES: [&str; 2] = ["pictures", "steps"];

/// The translated help topics (ADR 0084): each folder `docs/help/<code>/`
/// with its Markdown files, built into the program as the list
/// `HELP_TRANSLATIONS` (language code, file name, text). A language can
/// have some of the topics: the others show in English. Also the pictures
/// of the help (P19 Part 2), see [`help_pictures`].
fn help_translations() {
    let help = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/help");
    println!("cargo:rerun-if-changed={}", help.display());
    let mut langs: Vec<_> = std::fs::read_dir(&help)
        .map(|d| {
            d.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .filter(|p| !NOT_LANGUAGES.iter().any(|n| p.file_name().is_some_and(|f| f == *n)))
                .collect()
        })
        .unwrap_or_default();
    langs.sort();
    let mut out = String::from("/// Made by build.rs: (language, file name, text) of each translated help topic.\n");
    out.push_str("pub const HELP_TRANSLATIONS: &[(&str, &str, &str)] = &[\n");
    for dir in langs {
        println!("cargo:rerun-if-changed={}", dir.display());
        let code = dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .map(|d| {
                d.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().is_some_and(|x| x == "md"))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        for f in files {
            println!("cargo:rerun-if-changed={}", f.display());
            let name = f.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            // An absolute path (`include_str!` reads it from `OUT_DIR`).
            let path = f.display().to_string();
            out.push_str(&format!("    ({code:?}, {name:?}, include_str!({path:?})),\n"));
        }
    }
    out.push_str("];\n");
    out.push_str(&help_pictures(&help.join("pictures")));
    let dest = std::path::Path::new(&std::env::var("OUT_DIR").unwrap_or_default()).join("help_translations.rs");
    if let Err(e) = std::fs::write(&dest, out) {
        println!("cargo:warning=The help translations are not built in: {e}");
    }
}

/// The list `HELP_PICTURES`: (language code, name, PNG bytes) for each
/// `<dir>/<code>/<name>.png`, and ("", name, bytes) for each
/// `<dir>/<name>.png`, a picture with no text.
fn help_pictures(dir: &std::path::Path) -> String {
    println!("cargo:rerun-if-changed={}", dir.display());
    let pngs = |d: &std::path::Path| -> Vec<std::path::PathBuf> {
        let mut v: Vec<_> = std::fs::read_dir(d)
            .map(|r| {
                r.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().is_some_and(|x| x == "png"))
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    };
    let mut entries: Vec<(String, std::path::PathBuf)> = pngs(dir).into_iter().map(|p| (String::new(), p)).collect();
    let mut langs: Vec<_> = std::fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    langs.sort();
    for l in langs {
        println!("cargo:rerun-if-changed={}", l.display());
        let code = l.file_name().unwrap_or_default().to_string_lossy().into_owned();
        entries.extend(pngs(&l).into_iter().map(|p| (code.clone(), p)));
    }
    let mut out = String::from("/// Made by build.rs: (language or \"\", name, PNG bytes) of each help picture.\n");
    out.push_str("pub const HELP_PICTURES: &[(&str, &str, &[u8])] = &[\n");
    for (code, p) in entries {
        println!("cargo:rerun-if-changed={}", p.display());
        let name = p.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let path = p.display().to_string();
        out.push_str(&format!("    ({code:?}, {name:?}, include_bytes!({path:?})),\n"));
    }
    out.push_str("];\n");
    out
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
