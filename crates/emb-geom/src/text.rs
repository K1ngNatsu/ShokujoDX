//! Texts for the user (ADR 0083). The engine crates write the messages
//! that the user sees (errors, warnings, the labels of printed sheets) in
//! English, as the program does: [`tr`] for a text, [`fill`] for a text
//! with values. The English text is the key of its translation.
//!
//! The program sets a translator one time ([`set_translator`]); it gives
//! the text in the language of the thread. With no translator (the command
//! line, the tests of the engine, the golden snapshots) the texts stay
//! English. Names that are saved in files (threads, charts, products) are
//! data, not messages: they stay English, and the program translates them
//! where it shows them.
//!
//! This module has no UI dependency (rule 5): a translator is a plain
//! function.

use std::fmt::Display;
use std::sync::OnceLock;

/// Gives the translation of an English text, or `None` (English).
pub type Translator = fn(&str) -> Option<&'static str>;

static TRANSLATOR: OnceLock<Translator> = OnceLock::new();

/// Sets the translator. The first one stays; later calls change nothing.
pub fn set_translator(f: Translator) {
    let _ = TRANSLATOR.set(f);
}

/// The text in the language of the translator, else the English text.
pub fn tr(english: &'static str) -> &'static str {
    TRANSLATOR.get().and_then(|f| f(english)).unwrap_or(english)
}

/// The translated `template` with its values, as [`place`] puts them.
pub fn fill(template: &'static str, values: &[&dyn Display]) -> String {
    place(tr(template), values)
}

/// `text` with each `{}` in turn replaced by a value. A translation can
/// move the `{}` (the values stay in their order), and it can leave out the
/// last ones. `{_}` takes a value and shows nothing: for an English plural
/// "s" in the middle of a text ("{} piece{} to {}").
pub fn place(text: &str, values: &[&dyn Display]) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_go_in_place_and_english_stays_with_no_translator() {
        assert_eq!(fill("{} of {} objects", &[&2, &5]), "2 of 5 objects");
        assert_eq!(tr("Stitches"), "Stitches");
        // A translation moves the values and drops a plural "s".
        assert_eq!(place("{_}{} Teile von {}", &[&"s", &3, &"A"]), "3 Teile von A");
        assert_eq!(place("no values", &[&1]), "no values");
        assert_eq!(place("{} and {}", &[&1]), "1 and ");
    }
}
