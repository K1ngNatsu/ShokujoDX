# ADR 0083: Engine texts in the language of the program

Date: 2026-10-29 · Status: Accepted (changes point 4 of ADR 0042)

## Context

ADR 0042 kept the texts of the engine crates in English: the engine has no
UI dependency (rule 5), and the translations are in the program. But the
user sees many engine texts: the errors of the file readers and of project
files, the refusals of the shaping commands, the warnings of the stitch
plan and of the hoopings, the stop notes, script errors, and all the
labels of the printed sheets (worksheet, approval, quote, hooping sheet,
templates, cross-stitch chart, library catalog). In German, Spanish or
Japanese these were English.

## Decision

- `emb_geom::text` (the base crate of all engine crates) has a translation
  hook: `set_translator` (one plain function, set one time), `tr` and
  `fill`, with the same rules as the program (`{}` in order, `{_}` for a
  plural). With no translator the texts stay English: the command line,
  the tests of the engine and the golden snapshots do not change.
- The program sets the translator when it sets a language
  (`i18n::set_active`): the translator gives the text in the language of
  the thread. The threads that make texts for the user (export jobs, the
  Picture to design jobs) take the language of the program with them.
- Engine code writes a message as `text::tr("…")` or `text::fill("… {} …",
  &[&v])`; a `thiserror` text is `#[error("{}", text::tr("…"))]`. The
  template test reads these literals in the engine sources too
  (`i18n::tests::engine_source_texts`), so `UPDATE_LANG=1` finds them.
- Names that are data stay English: names saved in files (default thread
  and object names, registration marks), the type names of objects, the
  labels of the order rows and of quote lines, which the code compares.
  The program translates them where it shows them (`tr(name)`, listed in
  `engine_texts`).
- Texts made on a thread with no language (the stitch plan, the library
  scan) are English; the program translates a text with no values where it
  shows it (`tr(&w.message)`).

## Consequences

- 219 engine texts are translated (German, Spanish, Japanese). A native
  speaker should check them with the other texts.
- Two labels of the sheets changed in English: the order row "Order" is
  "Order number" and the quote column "Amount" is "Total", because the
  same English words have other meanings in the program (the sew order, an
  effect amount), and a key has one translation.
- The messages of libraries (`toml`, `serde_json`, `usvg`, `lopdf`, Rhai)
  stay English inside the translated message.
- A new engine message needs `text::tr` or `text::fill`, as a new UI text
  needs `tr` or `fill`.
- The names of the engine (`fn name(self)` and `fn label(self)`, shown
  with `tr(x.name())`) are listed in `i18n::tests::engine_texts`. The test
  `each_engine_name_is_in_the_template` finds a name that is not in the
  template; brands and names that the program does not show are in
  `NAMES_NOT_SHOWN`.
