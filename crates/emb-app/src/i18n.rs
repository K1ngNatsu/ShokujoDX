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

/// Sets the language of this thread (tests use their own languages).
pub fn set_active(lang: Option<&'static Language>) {
    ACTIVE.with(|a| a.set(lang));
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
    let t = tr(template);
    let mut out = String::with_capacity(t.len() + 16);
    let mut rest = t;
    let mut i = 0;
    loop {
        let next = [
            rest.find("{}").map(|a| (a, 2, true)),
            rest.find("{_}").map(|a| (a, 3, false)),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(a, ..)| *a);
        let Some((at, len, show)) = next else { break };
        out.push_str(&rest[..at]);
        if show && let Some(v) = values.get(i) {
            out.push_str(&v.to_string());
        }
        i += 1;
        rest = &rest[at + len..];
    }
    out.push_str(rest);
    out
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
    pub(crate) fn source_texts() -> BTreeSet<String> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut out = BTreeSet::new();
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            // This file has no texts to show, only the code of the scan.
            .filter(|p| p.file_name().is_some_and(|n| n != "i18n.rs"))
            .collect();
        files.sort();
        for f in files {
            // (A checkout on Windows can have CRLF line ends.)
            let s = std::fs::read_to_string(&f).unwrap().replace("\r\n", "\n");
            // Not the tests of a file.
            let s = s.split("#[cfg(test)]\nmod tests").next().unwrap_or(&s);
            // Help texts in constants (the program shows them with `tr(X_HELP)`).
            let mut rest = s;
            while let Some(at) = rest.find("_HELP: &str =") {
                rest = rest[at + "_HELP: &str =".len()..].trim_start();
                if let Some(lit) = rest.strip_prefix('"')
                    && let Some((text, used)) = literal(lit)
                {
                    out.insert(text);
                    rest = &lit[used..];
                }
            }
            for marker in ["tr(", "fill(", "title:"] {
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
        out
    }

    /// The names of the engine (stitch types, shaping, products, …) that the
    /// program shows with `tr(x.name())`.
    pub(crate) fn engine_texts() -> BTreeSet<String> {
        let mut out: Vec<&'static str> = Vec::new();
        out.extend(emb_model::FillPattern::all().iter().map(|p| p.name()));
        out.extend(emb_model::ColumnPattern::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::MeanderTile::ALL.iter().map(|t| t.name()));
        out.extend(emb_model::tartan::CODES.iter().map(|(_, name, _)| *name));
        out.extend(crate::color_fills::SETTS.iter().map(|(name, _, _)| *name));
        out.extend(emb_model::lace::TEMPLATES.iter().map(|(name, _)| *name));
        out.extend(emb_model::ShapeOp::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::ConvertTo::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::BorderPlace::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::ProductKind::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::TraceStitch::all().iter().map(|p| p.name()));
        out.extend(emb_art::click::ClickStitch::ALL.iter().map(|p| p.name()));
        out.extend(emb_art::smart::SmartStitch::ALL.iter().map(|p| p.name()));
        out.extend(emb_model::quote::LineKind::ALL.iter().map(|k| k.label()));
        out.extend(emb_model::stock::BuyReason::ALL.iter().map(|k| k.label()));
        out.extend(emb_sketch::Constraint::NAMES);
        out.extend(emb_sketch::edit::MESSAGES);
        out.extend(emb_sketch::ExprError::MESSAGES);
        out.extend(emb_model::sketch::RegionTo::ALL.map(|r| r.name()));
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
        t
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
