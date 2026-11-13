//! Translations (P10 Milestone 7, ADR 0042).
//!
//! The English text is the key: `tr("Open")` gives the text in the language
//! of the settings, or the English text if the language has no text for it.
//! Each language is one file, `data/lang/<code>.toml`, built into the
//! program:
//!
//! ```toml
//! name = "Deutsch"
//! [text]
//! "Open" = "Öffnen"
//! ```
//!
//! `data/lang/template.toml` has all texts of the program with empty
//! translations; a test makes it again from the source (`tr("…")` calls and
//! the ribbon panel titles) and fails if it is not up to date. A text that
//! has `{}` in it gets its values with [`fill`], in the same order.
//!
//! The language is set for each thread (the program draws on one thread),
//! so tests in parallel do not change the language of each other.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The language files that are built into the program: code and text, in
/// the order of the Options list (after English).
const FILES: &[(&str, &str)] = &[
    ("de", include_str!("../../../data/lang/de.toml")),
    ("es", include_str!("../../../data/lang/es.toml")),
    ("ja", include_str!("../../../data/lang/ja.toml")),
];

/// A language: its code (the file name), its own name, and its texts.
pub struct Language {
    pub code: String,
    pub name: String,
    texts: HashMap<&'static str, &'static str>,
}

impl Language {
    /// Reads a language file. The texts live as long as the program (a
    /// language is read one time).
    pub fn parse(code: &str, text: &str) -> Result<Language, String> {
        let v: toml::Table = toml::from_str(text).map_err(|e| format!("{code}.toml: {e}"))?;
        let name = v.get("name").and_then(|n| n.as_str()).unwrap_or(code).to_string();
        let mut texts = HashMap::new();
        if let Some(t) = v.get("text").and_then(|t| t.as_table()) {
            for (k, v) in t {
                if let Some(v) = v.as_str().filter(|v| !v.is_empty()) {
                    let k: &'static str = Box::leak(k.clone().into_boxed_str());
                    let v: &'static str = Box::leak(v.to_string().into_boxed_str());
                    texts.insert(k, v);
                }
            }
        }
        Ok(Language {
            code: code.to_string(),
            name,
            texts,
        })
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    pub fn get(&self, english: &str) -> Option<&'static str> {
        self.texts.get(english).copied()
    }

    /// The English texts that the language has.
    #[cfg(test)]
    pub fn keys(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.texts.keys().copied()
    }
}

/// The built-in languages (not English, which needs no file).
pub fn languages() -> &'static [Language] {
    static ALL: OnceLock<Vec<Language>> = OnceLock::new();
    ALL.get_or_init(|| {
        FILES
            .iter()
            .filter_map(|(code, text)| Language::parse(code, text).ok())
            .collect()
    })
}

thread_local! {
    static ACTIVE: Cell<Option<&'static Language>> = const { Cell::new(None) };
}

/// Sets the language of this thread by its code; "en" or an unknown code
/// is English.
pub fn set_language(code: &str) {
    let lang = languages().iter().find(|l| l.code == code);
    set_active(lang);
}

/// Sets the language of this thread (tests use their own languages). The
/// engine crates take their texts from the same language (ADR 0083).
pub fn set_active(lang: Option<&'static Language>) {
    emb_geom::text::set_translator(|english| active().and_then(|l| l.get(english)));
    ACTIVE.with(|a| a.set(lang));
}

/// A translated word (a name or a label) for the middle of a sentence:
/// lower case, but German nouns keep their capital letter.
pub fn in_sentence(word: &str) -> String {
    if current() == "de" {
        word.to_string()
    } else {
        word.to_lowercase()
    }
}

/// The language of this thread (`None`: English). A thread that makes
/// texts for the screen takes the language of the program with it.
pub fn active() -> Option<&'static Language> {
    ACTIVE.with(Cell::get)
}

/// The code of the language of this thread.
pub fn current() -> &'static str {
    ACTIVE.with(|a| a.get().map_or("en", |l| l.code.as_str()))
}

/// The text in the language of this thread, else the English text.
pub fn tr(english: &str) -> &str {
    ACTIVE.with(|a| match a.get() {
        Some(l) => l.get(english).unwrap_or(english),
        None => english,
    })
}

/// The translated `template` with each `{}` in turn replaced by a value.
/// A translation can move the `{}` (the values stay in their order), and it
/// can leave out the last ones. `{_}` in a translation takes a value and
/// shows nothing: for an English plural "s" in the middle of a text
/// ("{} piece{} to {}").
pub fn fill(template: &'static str, values: &[&dyn std::fmt::Display]) -> String {
    emb_geom::text::place(tr(template), values)
}

/// The number of values that a text takes: its `{}` and `{_}`.
#[cfg(test)]
fn slots(s: &str) -> usize {
    s.matches("{}").count() + s.matches("{_}").count()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// A text as a TOML basic string on one line (for keys and values).
    pub(crate) fn quoted(s: &str) -> String {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    /// A test language with a few texts.
    pub(crate) fn test_language(pairs: &[(&str, &str)]) -> &'static Language {
        let mut text = String::from("name = \"Test\"\n[text]\n");
        for (k, v) in pairs {
            text.push_str(&format!("{} = {}\n", quoted(k), quoted(v)));
        }
        Box::leak(Box::new(Language::parse("test", &text).unwrap()))
    }

    #[test]
    fn texts_fall_back_to_english() {
        assert_eq!(tr("Open"), "Open");
        set_active(Some(test_language(&[
            ("Open", "Öffnen"),
            ("Joined {} objects.", "{} Objekte verbunden."),
        ])));
        assert_eq!(tr("Open"), "Öffnen");
        assert_eq!(tr("Save"), "Save");
        assert_eq!(fill("Joined {} objects.", &[&3]), "3 Objekte verbunden.");
        // {_} takes a value and shows nothing (an English plural "s").
        set_active(Some(test_language(&[(
            "Cut {} piece{} to {}",
            "{} Teil(e){_} nach {}",
        )])));
        assert_eq!(
            fill("Cut {} piece{} to {}", &[&2, &"s", &"a.svg"]),
            "2 Teil(e) nach a.svg"
        );
        assert_eq!(current(), "test");
        set_language("en");
        assert_eq!(tr("Open"), "Open");
        assert_eq!(fill("Joined {} objects.", &[&3]), "Joined 3 objects.");
    }

    /// The Rust string literal that starts at `s` (after the quote), as text.
    fn literal(s: &str) -> Option<(String, usize)> {
        let mut out = String::new();
        let mut it = s.char_indices().peekable();
        while let Some((i, c)) = it.next() {
            match c {
                '"' => return Some((out, i + 1)),
                '\\' => match it.next()?.1 {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    '\\' => out.push('\\'),
                    '"' => out.push('"'),
                    '\'' => out.push('\''),
                    '0' => out.push('\0'),
                    // A line break in the literal: it and the spaces after it go.
                    '\n' | '\r' => {
                        while it.peek().is_some_and(|(_, c)| c.is_whitespace()) {
                            it.next();
                        }
                    }
                    'u' => {
                        let mut hex = String::new();
                        for (_, c) in it.by_ref() {
                            match c {
                                '{' => {}
                                '}' => break,
                                c => hex.push(c),
                            }
                        }
                        out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                    }
                    _ => return None,
                },
                c => out.push(c),
            }
        }
        None
    }

    /// All texts of the program: the literals in `tr("…")` and `fill("…"`
    /// calls of the source files, and the panel titles of the ribbon.
    /// The `.rs` files of `dir` and of its module folders, not `tests.rs`
    /// files (only tests) and not `skip`.
    fn rs_files(dir: &std::path::Path, skip: &str) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        let mut dirs = vec![dir.to_path_buf()];
        while let Some(d) = dirs.pop() {
            for p in std::fs::read_dir(&d).unwrap().filter_map(|e| e.ok().map(|e| e.path())) {
                if p.is_dir() {
                    dirs.push(p);
                } else if p.extension().is_some_and(|x| x == "rs")
                    && p.file_name().is_some_and(|n| n != skip && n != "tests.rs")
                {
                    files.push(p);
                }
            }
        }
        files.sort();
        files
    }

    /// The code of a source file, without its tests (a `mod tests;` line is
    /// not the tests).
    fn code_of(f: &std::path::Path) -> String {
        // (A checkout on Windows can have CRLF line ends.)
        let s = std::fs::read_to_string(f).unwrap().replace("\r\n", "\n");
        s.split("#[cfg(test)]\nmod tests {").next().unwrap_or(&s).to_string()
    }

    /// The string literals after each of `markers` in `s`.
    fn literals_after(s: &str, markers: &[&str], out: &mut BTreeSet<String>) {
        for marker in markers {
            let mut rest = s;
            while let Some(at) = rest.find(marker) {
                let before = rest[..at].chars().last();
                rest = &rest[at + marker.len()..];
                // `tr(` and not `ptr(` or `attr(`.
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                // The literal can be on the next line (rustfmt).
                let Some(lit) = rest.trim_start().strip_prefix('"') else {
                    continue;
                };
                rest = lit;
                if let Some((text, used)) = literal(rest) {
                    if !text.is_empty() {
                        out.insert(text);
                    }
                    rest = &rest[used..];
                }
            }
        }
    }

    /// All texts of the program: the literals in `tr("…")` and `fill("…"`
    /// calls of the source files, and the panel titles of the ribbon.
    pub(crate) fn source_texts() -> BTreeSet<String> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut out = BTreeSet::new();
        // This file has no texts to show, only the code of the scan.
        for f in rs_files(&dir, "i18n.rs") {
            let s = code_of(&f);
            // Help texts in constants (the program shows them with `tr(X_HELP)`).
            let mut rest = s.as_str();
            while let Some(at) = rest.find("_HELP: &str =") {
                rest = rest[at + "_HELP: &str =".len()..].trim_start();
                if let Some(lit) = rest.strip_prefix('"')
                    && let Some((text, used)) = literal(lit)
                {
                    out.insert(text);
                    rest = &lit[used..];
                }
            }
            literals_after(&s, &["tr(", "fill(", "title:"], &mut out);
        }
        out
    }

    /// The engine crates, whose messages the program shows (ADR 0083).
    pub(crate) const ENGINE_CRATES: [&str; 10] = [
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

    /// The messages of the engine crates: the literals in `text::tr("…")`
    /// and `text::fill("…"` (ADR 0083).
    pub(crate) fn engine_source_texts() -> BTreeSet<String> {
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut out = BTreeSet::new();
        for c in ENGINE_CRATES {
            // The module of the hook has its doc examples only.
            for f in rs_files(&crates.join(c).join("src"), "text.rs") {
                literals_after(&code_of(&f), &["text::tr(", "text::fill("], &mut out);
            }
        }
        out
    }

    /// The names of the engine (stitch types, shaping, products, …) that the
    /// program shows with `tr(x.name())`.
    pub(crate) fn engine_texts() -> BTreeSet<String> {
        let mut out: Vec<&'static str> = Vec::new();
        out.extend(emb_model::ObjectKind::TYPE_NAMES);
        out.extend(emb_model::FillPattern::all().iter().map(|p| p.name()));
        out.extend(emb_model::ColumnPattern::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::MeanderTile::ALL.iter().map(|t| t.name()));
        out.extend(emb_model::tartan::CODES.iter().map(|(_, name, _)| *name));
        out.extend(crate::color_fills::SETTS.iter().map(|(name, _, _)| *name));
        out.extend(emb_model::lace::TEMPLATES.iter().map(|(name, _)| *name));
        out.extend(emb_model::ShapeOp::ALL.iter().map(|p| p.name()));
        out.extend(crate::manipulate::ScaleFrom::ALL.iter().map(|(_, name)| *name));
        out.extend(emb_model::ConvertTo::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::BorderPlace::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::ProductKind::ALL.iter().map(|p| p.name()));
        out.extend(
            emb_model::ProductKind::ALL
                .iter()
                .flat_map(|p| p.placements().iter().map(|pl| pl.name)),
        );
        out.extend(emb_library::SortKey::ALL.iter().map(|k| k.label()));
        out.extend(
            emb_model::chart::harmonies(emb_model::Rgb(200, 40, 40))
                .into_iter()
                .map(|(n, _)| n),
        );
        out.extend(emb_library::GroupKey::ALL.iter().map(|k| k.label()));
        out.extend(emb_model::TraceStitch::all().iter().map(|p| p.name()));
        out.extend(emb_art::click::ClickStitch::ALL.iter().map(|p| p.name()));
        out.extend(emb_art::smart::SmartStitch::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::quote::LineKind::ALL.iter().map(|k| k.label()));
        out.extend(emb_model::order::ROW_LABELS);
        out.extend(emb_model::stock::BuyReason::ALL.iter().map(|k| k.label()));
        out.extend(emb_sketch::Constraint::NAMES);
        out.extend(emb_sketch::edit::MESSAGES);
        out.extend(emb_sketch::ExprError::MESSAGES);
        out.extend(emb_model::sketch::RegionTo::ALL.map(|r| r.name()));
        out.extend(emb_model::cross::Part::all().into_iter().map(|p| p.name()));
        out.extend(emb_model::EnvelopeShape::ALL.iter().map(|s| s.name()));
        out.extend(emb_model::MonogramStyle::ALL.iter().map(|s| s.name()));
        out.extend(emb_model::BorderShape::ALL.iter().map(|s| s.name()));
        out.extend(emb_model::BorderStitch::ALL.iter().map(|s| s.name()));
        out.extend(emb_model::OrnamentShape::ALL.iter().map(|s| s.name()));
        out.extend(emb_model::OrnamentPlace::ALL.iter().map(|s| s.name()));
        out.extend(emb_art::photo::PhotoStyle::ALL.iter().map(|s| s.name()));
        out.extend(emb_render::barcode::BarcodeKind::ALL.iter().map(|k| k.label()));
        out.extend(emb_render::page::Paper::ALL.iter().map(|p| p.label()));
        out.extend(emb_model::ButtonholeShape::ALL.iter().map(|s| s.name()));
        out.extend(emb_model::ButtonholeShape::ALL.iter().map(|s| s.help()));
        // Names in constants of the app (shown with `tr(name)`).
        out.extend(crate::hoops::BOX_PLACES.iter().map(|(_, name)| *name));
        out.extend(crate::kiosk::THREADS.iter().map(|(name, _)| *name));
        out.extend(crate::kiosk::PRODUCT_COLORS.iter().map(|(name, _)| *name));
        out.extend(crate::viewtools::PANELS.iter().map(|(_, name)| *name));
        out.extend(crate::library::LIST_COLUMNS.iter().map(|(name, _, _)| *name));
        out.into_iter().map(str::to_string).collect()
    }

    fn template_text(texts: &BTreeSet<String>) -> String {
        let mut t = String::from(
            "# All texts of the program, with English as the key. Copy this file to\n\
             # <code>.toml (for example de.toml), set the name, and write each text in\n\
             # the language. An empty text stays English. Keep each {} of a text.\n\
             # A test makes this file again from the source:\n\
             # UPDATE_LANG=1 cargo test -p emb-app i18n\n\
             name = \"\"\n\n[text]\n",
        );
        for k in texts {
            t.push_str(&format!("{} = \"\"\n", quoted(k)));
        }
        t
    }

    /// The keys of the template: the source texts and the engine names.
    pub(crate) fn all_texts() -> BTreeSet<String> {
        let mut t = source_texts();
        t.extend(engine_texts());
        t.extend(engine_source_texts());
        t
    }

    /// Names of the engine that the program does not translate: brands, and
    /// names that it does not show.
    const NAMES_NOT_SHOWN: [&str; 5] = [
        "Bernina",
        "Brother, Baby Lock",
        "Husqvarna Viking, Pfaff",
        "Janome",
        "Remove overlaps",
    ];

    #[test]
    fn each_engine_name_is_in_the_template() {
        // The literals of each `fn name(self)` and `fn label(self)` of the
        // engine crates must be in the template, so that `tr(x.name())`
        // shows a translation (add the list to `engine_texts`).
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let texts = all_texts();
        let mut missing = Vec::new();
        for c in ENGINE_CRATES {
            for f in rs_files(&crates.join(c).join("src"), "text.rs") {
                let s = code_of(&f);
                for head in ["fn name(self) -> &'static str {", "fn label(self) -> &'static str {"] {
                    for (at, _) in s.match_indices(head) {
                        let body = &s[at..];
                        let body = &body[..body.find("\n    }\n").unwrap_or(body.len())];
                        let mut names = BTreeSet::new();
                        literals_after(body, &["=> "], &mut names);
                        for n in names {
                            if !texts.contains(&n) && !NAMES_NOT_SHOWN.contains(&n.as_str()) {
                                missing.push(format!("{}: {n}", f.display()));
                            }
                        }
                    }
                }
            }
        }
        assert!(missing.is_empty(), "{missing:#?}");
    }

    #[test]
    fn the_template_has_all_texts() {
        let texts = all_texts();
        assert!(texts.len() > 500, "{}", texts.len());
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/lang/template.toml");
        let want = template_text(&texts);
        if std::env::var("UPDATE_LANG").is_ok() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &want).unwrap();
        }
        let have = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
        assert!(
            have == want,
            "data/lang/template.toml is not up to date: UPDATE_LANG=1 cargo test -p emb-app i18n"
        );
        // The template reads as a language with no texts.
        assert!(Language::parse("template", &have).unwrap().is_empty());
    }

    #[test]
    fn the_texts_of_the_program_are_keys_of_the_template() {
        // The texts as the program has them at run time: each must be a
        // key that the source scan finds (line breaks and escapes in the
        // literals read the same).
        let texts = all_texts();
        let mut missing = Vec::new();
        let mut check = |t: &str| {
            if !t.is_empty() && t.chars().any(char::is_alphabetic) && !texts.contains(t) {
                missing.push(t.to_string());
            }
        };
        for (cmd, tab, panel) in crate::ribbon::all_commands() {
            let (_, label, tip) = cmd.meta();
            check(label);
            check(tip);
            check(tab.label());
            check(panel);
        }
        for t in crate::draw::Tool::ALL {
            check(t.label());
            check(t.tooltip());
            for n in 0..4 {
                check(t.prompt(n, 0));
                check(t.prompt(n, 1));
            }
        }
        assert!(
            missing.is_empty(),
            "{} texts: {:?}",
            missing.len(),
            &missing[..missing.len().min(10)]
        );
    }

    /// The history keeps the label in the language of the edit, so the
    /// Undo menu and the Undo message show it in that language.
    #[test]
    fn undo_labels_are_in_the_language() {
        use emb_model::{HoopCatalog, MachineCatalog};
        let mut st = crate::app::AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        set_active(Some(test_language(&[
            ("Add Sample Objects", "Beispielobjekte hinzufügen"),
            ("Undo: {}", "Rückgängig: {}"),
        ])));
        st.add_sample_objects();
        assert_eq!(st.doc.undo_label(), Some("Beispielobjekte hinzufügen"));
        st.undo();
        assert_eq!(st.message, "Rückgängig: Beispielobjekte hinzufügen");
        set_active(None);
    }

    /// The engine crates take the language of the thread (ADR 0083): the
    /// errors, the warnings and the labels of the printed sheets.
    #[test]
    fn engine_messages_are_in_the_language() {
        use emb_render::page::Op;
        let engine = || {
            let mut sheet = emb_render::sheet::SheetData {
                name: "Test".into(),
                stitches: 10,
                ..Default::default()
            };
            sheet.date = "today".into();
            let texts: Vec<String> = emb_render::sheet::production(&sheet, emb_render::page::Paper::A4)[0]
                .ops
                .iter()
                .filter_map(|o| match o {
                    Op::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            (
                emb_stitch::StitchError::Empty.to_string(),
                emb_model::ShapeError::TooFew.to_string(),
                emb_formats::read_machine("xyz", &[]).unwrap_err(),
                texts,
            )
        };
        let (stitch, shape, format, sheet) = engine();
        assert_eq!(stitch, "the object has no stitchable geometry");
        assert!(sheet.iter().any(|t| t == "Stitches"));
        set_language("de");
        let (stitch_de, shape_de, format_de, sheet_de) = engine();
        assert_eq!(stitch_de, tr("the object has no stitchable geometry"));
        assert_ne!(stitch_de, stitch);
        assert_ne!(shape_de, shape);
        assert_eq!(format_de, "unbekanntes Maschinenformat .xyz");
        assert_ne!(format_de, format);
        assert!(sheet_de.iter().any(|t| t == "Stiche"), "{sheet_de:?}");
        assert!(
            sheet_de.iter().any(|t| t.starts_with("Produktions-Arbeitsblatt")),
            "{sheet_de:?}"
        );
        set_active(None);
        assert_eq!(engine().0, stitch, "English again");
    }

    #[test]
    fn a_language_changes_the_ribbon() {
        let (_, open, tip) = crate::ribbon::Cmd::Open.meta();
        set_active(Some(test_language(&[
            (open, "Öffnen"),
            (tip, "Ein Design öffnen."),
            ("Help", "Hilfe"),
        ])));
        let (_, open_de, tip_de) = crate::ribbon::Cmd::Open.meta();
        assert_eq!((open_de, tip_de), ("Öffnen", "Ein Design öffnen."));
        assert_eq!(crate::ribbon::RibbonTab::Help.label(), "Hilfe");
        set_active(None);
        assert_eq!(crate::ribbon::Cmd::Open.meta().1, open);
    }

    #[test]
    fn the_languages_cover_the_program() {
        // A new text can wait for its translations, but each language keeps
        // at least this part of the texts (the template has them all).
        let texts = all_texts();
        let names: Vec<(&str, &str)> = languages().iter().map(|l| (l.code.as_str(), l.name.as_str())).collect();
        assert_eq!(names, [("de", "Deutsch"), ("es", "Español"), ("ja", "日本語")]);
        for l in languages() {
            let have = texts.iter().filter(|t| l.get(t).is_some()).count();
            assert!(
                have * 100 >= texts.len() * 95,
                "{}: {have} of {} texts",
                l.code,
                texts.len()
            );
        }
        // The ribbon in each language.
        for (code, edit, open) in [
            ("de", "Bearbeiten", "Öffnen"),
            ("es", "Editar", "Abrir"),
            ("ja", "編集", "開く"),
        ] {
            set_language(code);
            assert_eq!(crate::ribbon::RibbonTab::Edit.label(), edit);
            assert_eq!(crate::ribbon::Cmd::Open.meta().1, open);
        }
        set_language("en");
        assert_eq!(crate::ribbon::RibbonTab::Edit.label(), "Edit");
    }

    #[test]
    fn languages_have_only_texts_of_the_program() {
        let texts = all_texts();
        for l in languages() {
            let stale: Vec<&str> = l.keys().filter(|k| !texts.contains(*k)).collect();
            assert!(
                stale.is_empty(),
                "{}: texts that the program does not have: {stale:?}",
                l.code
            );
            // The {} stay; a translation can leave out the last ones (an
            // English plural "s", for example), not add more.
            for k in l.keys() {
                let (en, here) = (k.matches("{}").count(), slots(l.get(k).unwrap()));
                assert!(here <= en, "{}: {k}: {here} {{}} for {en}", l.code);
            }
        }
    }
}
