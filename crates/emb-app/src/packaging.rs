//! Tests of the package files (ADR 0043): the icon, the installer script,
//! the macOS app data and the Linux desktop files agree with the program.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(root().join(path))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .replace("\r\n", "\n")
}

/// The values of `attr="…"` in an XML text.
fn attrs<'a>(text: &'a str, attr: &str) -> Vec<&'a str> {
    let key = format!("{attr}=\"");
    text.match_indices(&key)
        .filter_map(|(i, _)| {
            let rest = &text[i + key.len()..];
            rest.find('"').map(|end| &rest[..end])
        })
        .collect()
}

#[test]
fn the_icons_have_their_sizes() {
    for size in [16u32, 24, 32, 48, 64, 128, 256, 512, 1024] {
        let path = root().join(format!("data/icon/icon-{size}.png"));
        let img = image::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!((img.width(), img.height()), (size, size), "{}", path.display());
    }
    let ico = std::fs::read(root().join("data/icon/app.ico")).unwrap();
    // An icon file: reserved 0, type 1, then the number of pictures.
    assert_eq!(&ico[..4], &[0, 0, 1, 0]);
    assert_eq!(
        u16::from_le_bytes([ico[4], ico[5]]),
        7,
        "16, 24, 32, 48, 64, 128 and 256 pixels"
    );
    let icon = crate::icons::window_icon().expect("the window icon reads");
    assert_eq!((icon.width, icon.height), (256, 256));
    assert_eq!(icon.rgba.len(), 256 * 256 * 4);
}

/// The `.rs` files under a folder, at all depths.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
}

/// A `pub` item with the bytes of a file in it (`include_bytes!`,
/// `include_str!`) is a `static`, not a `const`. Each crate that uses a
/// `const` gets its own copy of the bytes: the bundled fonts were two times
/// in each program (22 MB in the program, 20 MB in `emb-cli` and in the
/// browser version).
#[test]
fn built_in_files_are_statics() {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root().join("crates")).unwrap().flatten() {
        let src = entry.path().join("src");
        if src.is_dir() {
            rust_files(&src, &mut files);
        }
    }
    assert!(files.len() > 100, "the source files are found");
    let mut consts = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        // The name of the `pub const` item that the line is in, if any.
        let mut item: Option<String> = None;
        for line in text.lines() {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("pub const ") {
                item = Some(rest.split(':').next().unwrap_or(rest).to_string());
            } else if t.starts_with("pub static ")
                || t.starts_with("static ")
                || t.starts_with("const ")
                || t.starts_with("fn ")
                || t.starts_with("pub fn ")
                || t.starts_with("pub(crate) ")
            {
                item = None;
            }
            if let Some(name) = &item {
                if t.contains("include_bytes!") || t.contains("include_str!") {
                    let file = path.strip_prefix(root()).unwrap_or(path);
                    consts.push(format!("{} {name}", file.display()));
                }
                if t.ends_with(';') {
                    item = None;
                }
            }
        }
    }
    consts.dedup();
    assert!(consts.is_empty(), "make these `pub static`: {consts:?}");
}

/// The social preview of the repository (`tools/make-social-preview.py`):
/// GitHub shows 1280 x 640 pixels and takes a file smaller than 1 MB.
#[test]
fn the_social_preview_fits_github() {
    let path = root().join("data/icon/social-preview.png");
    let img = image::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert_eq!((img.width(), img.height()), (1280, 640));
    let bytes = std::fs::metadata(&path).unwrap().len();
    assert!(bytes < 1024 * 1024, "{bytes} bytes: GitHub takes less than 1 MB");
}

#[test]
fn the_linux_files_agree() {
    let id = crate::icons::APP_ID;
    let desktop = read(&format!("packaging/linux/{id}.desktop"));
    let value = |key: &str| {
        desktop
            .lines()
            .find_map(|l| l.strip_prefix(key).and_then(|r| r.strip_prefix('=')))
            .unwrap_or_else(|| panic!("no {key} in the .desktop file"))
    };
    assert_eq!(value("Exec"), format!("{id} %f"), "the program of the packages");
    assert_eq!(value("Icon"), id);
    assert_eq!(value("StartupWMClass"), id, "the window has the app id");
    let mime = read(&format!("packaging/linux/{id}.xml"));
    let types = attrs(&mime, "type");
    for t in value("MimeType").split(';').filter(|t| !t.is_empty()) {
        assert!(types.contains(&t), "{t} is not in the MIME file");
    }
    for glob in attrs(&mime, "pattern") {
        let ext = glob.strip_prefix("*.").unwrap();
        assert!(
            ext == emb_model::project::FILE_EXTENSION || crate::file::machine_ext(Path::new(glob)).is_some(),
            "the program does not read {glob}"
        );
    }
    // The thumbnailer runs the program for the same file types.
    let thumbnailer = read(&format!("packaging/linux/{id}.thumbnailer"));
    assert!(thumbnailer.contains(&format!("Exec={id} --thumbnail %i %o %s")));
    let mime_of = |text: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix("MimeType="))
            .unwrap_or_default()
            .to_string()
    };
    assert_eq!(mime_of(&thumbnailer), value("MimeType"));
    let script = read("tools/package-linux.sh");
    assert!(script.contains(&format!("usr/bin/{id}\"")));
    assert!(script.contains(&format!("{id}.thumbnailer")));
}

#[test]
fn the_windows_installer_agrees() {
    let iss = read("packaging/windows/installer.iss");
    assert!(iss.contains("#define AppExe \"ShokujoDX.exe\""));
    assert!(read("tools/package-windows.ps1").contains("'ShokujoDX.exe'"));
    // The command-line tool with the MCP server is in each package (ADR 0176).
    for script in [
        "tools/package-windows.ps1",
        "tools/package-macos.sh",
        "tools/package-linux.sh",
    ] {
        assert!(
            read(script).contains("-p emb-cli") || read(script).contains("'-p', 'emb-cli'"),
            "{script}"
        );
        assert!(
            read(script).contains("release/emb-cli") || read(script).contains("release\\emb-cli.exe"),
            "{script}"
        );
    }
    assert!(iss.contains(&format!("Classes\\.{}\"", emb_model::project::FILE_EXTENSION)));
    let mut machine = 0;
    for line in iss
        .lines()
        .filter(|l| l.contains("\\OpenWithProgids\"") && l.contains("MachineFile"))
    {
        let ext = line
            .split("Classes\\.")
            .nth(1)
            .and_then(|r| r.split('\\').next())
            .unwrap();
        assert!(
            crate::file::machine_ext(Path::new(&format!("a.{ext}"))).is_some(),
            "{ext}"
        );
        assert!(
            iss.contains(&format!("ValueName: \".{ext}\"")),
            ".{ext} is in SupportedTypes"
        );
        machine += 1;
    }
    assert!(machine >= 10);
    // Each language of the installer has each of its own texts.
    let keys: Vec<&str> = iss
        .lines()
        .filter_map(|l| l.strip_prefix("en."))
        .filter_map(|l| l.split('=').next())
        .collect();
    assert_eq!(keys.len(), 12);
    // The check for a new version (ADR 0109): a tick box, and the choice
    // as yes or no in the file that the program reads.
    assert!(iss.contains("Name: \"updatecheck\"; Description: \"{cm:UpdateCheck}\""));
    for (value, tasks) in [("yes", "Tasks: updatecheck"), ("no", "Tasks: not updatecheck")] {
        let line = format!(
            "Filename: \"{{app}}\\{}\"; Section: \"install\"; Key: \"{}\"; String: \"{value}\"; {tasks}",
            crate::update::INSTALL_FILE,
            crate::update::INSTALL_KEY
        );
        assert!(iss.lines().any(|l| l == line), "{line}");
        let text = format!("[install]\r\n{}={value}\r\n", crate::update::INSTALL_KEY);
        assert_eq!(crate::update::parse_install_file(&text), Some(value == "yes"));
    }
    assert!(iss.contains(&format!(
        "Type: files; Name: \"{{app}}\\{}\"",
        crate::update::INSTALL_FILE
    )));
    // The thumbnail handler: its class id, and the file types that it
    // knows from their first bytes (ADR 0046).
    let lib = read("crates/emb-thumbnail/src/lib.rs");
    let clsid = lib
        .split("CLSID_TEXT: &str = \"")
        .nth(1)
        .and_then(|r| r.split('"').next())
        .unwrap();
    assert!(iss.contains(&format!("CLSID\\{{{clsid}\"")), "the class id");
    assert!(iss.contains("{app}\\emb_thumbnail.dll"));
    for ext in emb_library::preview::SNIFFED {
        assert!(
            iss.contains(&format!("Classes\\.{ext}\\ShellEx\\")),
            "thumbnails of .{ext}"
        );
    }
    assert!(read("tools/package-windows.ps1").contains("emb_thumbnail.dll"));
    for lang in ["de", "es", "ja"] {
        for k in &keys {
            assert!(iss.contains(&format!("\n{lang}.{k}=")), "{lang}.{k}");
        }
    }
}

#[test]
fn the_macos_app_data_agrees() {
    let plist = read("packaging/macos/Info.plist");
    for (key, value) in [
        ("CFBundleExecutable", "ShokujoDX"),
        ("CFBundleIconFile", "AppIcon"),
        ("CFBundleShortVersionString", "@VERSION@"),
    ] {
        assert!(
            plist.contains(&format!("<key>{key}</key>\n\t<string>{value}</string>")),
            "{key}"
        );
    }
    let script = read("tools/package-macos.sh");
    assert!(script.contains("AppIcon.icns"));
    // The Quick Look extension: its content types are declared by the app,
    // and its helper has the name that the extension starts (ADR 0046).
    let ext = read("packaging/macos/thumbnails/Info.plist");
    for ty in ["org.shokujodx.project", "org.shokujodx.stitches"] {
        assert!(ext.contains(&format!("<string>{ty}</string>")), "{ty}");
        assert!(
            plist.contains(&format!("<string>{ty}</string>")),
            "the app declares {ty}"
        );
    }
    let swift = read("packaging/macos/thumbnails/ThumbnailProvider.swift");
    assert!(swift.contains("forAuxiliaryExecutable: \"emb-thumbnail-helper\""));
    assert!(script.contains("MacOS/emb-thumbnail-helper"));
    assert!(script.contains("helper.entitlements") && script.contains("extension.entitlements"));
    assert!(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../emb-thumbnail/src/bin/emb-thumbnail-helper.rs")
            .is_file()
    );
}

/// A new user starts with a clean slate (ADR 0080): no package ships
/// sample designs (the test corpus) or a settings file.
#[test]
fn no_package_ships_samples_or_settings() {
    for script in [
        "tools/package-windows.ps1",
        "tools/package-macos.sh",
        "tools/package-linux.sh",
    ] {
        let text = read(script);
        for word in ["samples", "corpus", "settings.json"] {
            assert!(!text.contains(word), "{script} has {word}");
        }
    }
    assert!(!read("packaging/windows/installer.iss").contains("samples"));
}

#[test]
fn the_third_party_licenses_name_each_crate() {
    // `tools/third-party-licenses.py` makes the file again after a change of Cargo.lock.
    let licenses = read("data/licenses/THIRD-PARTY.txt");
    let lock = read("Cargo.lock");
    let mut missing = Vec::new();
    for pkg in lock.split("[[package]]").skip(1) {
        let field = |k: &str| {
            pkg.lines().find_map(|l| {
                l.strip_prefix(k)
                    .map(|v| v.trim_matches(|c| c == ' ' || c == '=' || c == '"'))
            })
        };
        // Crates of the workspace have no source.
        if field("source").is_none() {
            continue;
        }
        let (name, version) = (field("name ").unwrap(), field("version").unwrap());
        if !licenses.contains(&format!("\n{name} {version}  (")) {
            missing.push(format!("{name} {version}"));
        }
    }
    assert!(missing.is_empty(), "run tools/third-party-licenses.py: {missing:?}");
    for script in [
        "tools/package-windows.ps1",
        "tools/package-macos.sh",
        "tools/package-linux.sh",
    ] {
        assert!(read(script).contains("THIRD-PARTY.txt"), "{script}");
    }
}

#[test]
fn the_help_text_names_each_language() {
    for l in crate::i18n::languages() {
        assert!(crate::USAGE.contains(&l.code), "{}", l.code);
    }
}

/// The font packages (ADR 0115) are files of each release: the release
/// workflow takes all of them from `data/font-packages`.
#[test]
fn the_release_has_the_font_packages() {
    let release = read(".github/workflows/release.yml");
    assert!(release.contains("cp data/font-packages/shokujodx-fonts-*.zip dist/"));
    assert!(release.contains("dist/shokujodx-fonts-*.zip"));
    let packages = std::fs::read_dir(root().join("data/font-packages"))
        .expect("data/font-packages")
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.starts_with("shokujodx-fonts-") && name.ends_with(".zip")
        })
        .count();
    // sans, display, serif, script, hand, earlier, and japanese (P20
    // Milestone 6, ADR 0146).
    assert_eq!(packages, 7);
}

/// Each file of letter corrections (ADR 0112) is for a font of the program
/// or of a font package, and the font of a package has its corrections:
/// `tools/make-embroidery-fonts.py` made the packages after the last change
/// of the file. `emb-letter` checks the fonts of the program.
#[test]
fn the_font_packages_have_their_corrections() {
    let mut fonts = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(root().join("data/font-packages"))
        .expect("data/font-packages")
        .flatten()
    {
        let bytes = std::fs::read(entry.path()).expect("a package");
        for (name, data) in crate::share::unzip(&bytes).expect("a zip file") {
            if let Some(slug) = name.strip_prefix("fonts/").and_then(|n| n.strip_suffix(".embfont")) {
                fonts.insert(slug.to_string(), data);
            }
        }
    }
    let Ok(files) = std::fs::read_dir(root().join("data/fonts/embroidery/corrections")) else {
        return;
    };
    for path in files.flatten().map(|e| e.path()) {
        let c = emb_letter::correction::Corrections::load(&path).expect("a file of corrections");
        if emb_letter::bundled::embroidery(&c.family).is_some() {
            continue;
        }
        let slug = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        let data = fonts.get(slug).unwrap_or_else(|| {
            panic!(
                "{}: no font {} in the program or in a package",
                path.display(),
                c.family
            )
        });
        let font = emb_letter::embfont::EmbFont::from_json(std::str::from_utf8(data).expect("text"))
            .unwrap_or_else(|e| panic!("{slug}: {e}"));
        let missing = c.missing_in(&font);
        assert!(missing.is_empty(), "{slug}: {missing:?}; make the fonts again");
    }
}

/// The release checks of the thumbnails use a test design of the
/// repository, not a sample in a package (there are none, ADR 0080), and
/// the program can open it.
#[test]
fn the_release_checks_use_the_test_design() {
    let release = read(".github/workflows/release.yml");
    assert!(!release.contains("samples/"), "a package has no samples folder");
    assert_eq!(release.matches("packaging/test/two-color-patch.embproj").count(), 2);
    let d = emb_model::project::load(&root().join("packaging/test/two-color-patch.embproj"))
        .expect("the test design opens");
    assert!(!d.objects().is_empty());
}

/// The workflows use actions that run on Node.js 24: GitHub retires the
/// earlier Node.js versions of actions.
#[test]
fn the_workflows_use_current_actions() {
    let old = [
        "actions/checkout@v4",
        "actions/upload-artifact@v4",
        "actions/download-artifact@v4",
        "softprops/action-gh-release@v2",
    ];
    for f in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let text = read(f);
        for o in old {
            assert!(!text.contains(o), "{f} uses {o}");
        }
    }
}

/// CI skips only documents that the program does not build in, and checks
/// all ten engine crates for UI and C dependencies (rules 5 and 9).
#[test]
fn the_ci_skips_only_documents_and_checks_each_engine_crate() {
    let ci = read(".github/workflows/ci.yml");
    let ignored: Vec<&str> = ci
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("- '").and_then(|l| l.strip_suffix('\'')))
        .collect();
    assert!(!ignored.is_empty());
    for p in &ignored {
        assert!(p.ends_with(".md") || p.ends_with("/**"), "{p}");
        for built in ["docs/help", "data", "crates", "packaging", "Cargo"] {
            assert!(!p.starts_with(built), "CI must not skip {p}: the program builds it in");
        }
    }
    let engine = [
        "emb-geom",
        "emb-model",
        "emb-stitch",
        "emb-formats",
        "emb-render",
        "emb-letter",
        "emb-art",
        "emb-library",
        "emb-script",
        "emb-sketch",
    ];
    let line = ci
        .lines()
        .find(|l| l.contains("for c in emb-geom"))
        .expect("the engine check");
    for c in engine {
        assert!(
            line.split_whitespace().any(|w| w.trim_end_matches(';') == c),
            "{c} is not in the engine check"
        );
    }
}

/// The numbers in the README are the numbers of the program (the audit of
/// 2026-11-20: the README named thirty fonts, and the program had 65). A
/// new font, machine, thread chart, tutorial or format needs the new
/// number there.
#[test]
fn the_numbers_in_the_readme_are_the_numbers_of_the_program() {
    let readme = read("README.md").split_whitespace().collect::<Vec<_>>().join(" ");
    let has = |text: String| assert!(readme.contains(&text), "the README does not say: {text}");
    has(format!("{} embroidery fonts", emb_letter::bundled::EMBROIDERY.len()));
    has(format!(
        "A machine library of {} profiles",
        emb_model::MachineCatalog::builtin().machines.len()
    ));
    has(format!(
        "Thread charts of {} brands",
        emb_model::ThreadChart::builtin().len()
    ));
    let tutorials = std::fs::read_dir(root().join("docs/help"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("tutorial-"))
        .count();
    assert_eq!(tutorials, 31, "the README says 31 tutorials");
    has("31 step-by-step tutorials".into());
    // The formats table: the formats to write, eight more by name, and the
    // number of the others.
    let (read_n, write) = (emb_formats::READ_EXTENSIONS.len(), emb_formats::WRITE_EXTENSIONS);
    let named = ["SEW", "HUS", "SHV", "DSB", "DSZ", "10O", "100", "DAT"];
    let written: Vec<String> = write.iter().map(|e| e.to_uppercase()).collect();
    has(format!("| **Write** | {} |", written.join(", ")));
    has(format!(
        "plus {} and {} more",
        named.join(", "),
        read_n - write.len() - named.len()
    ));
    for e in named {
        assert!(emb_formats::READ_EXTENSIONS.contains(&e.to_lowercase().as_str()), "{e}");
    }
}
