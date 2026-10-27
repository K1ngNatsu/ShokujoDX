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
    assert_eq!(keys.len(), 5);
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
