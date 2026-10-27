# ADR 0042: Translations by English key, and accessibility in egui

Date: 2026-10-24 · Status: Accepted (P10 Milestone 7)

## Context

P10 Milestone 7 asks for a translation system (the texts of the program
in a table, English as the source, one file for each language) and for
accessibility: a screen reader reads the ribbon, keyboard access to all
ribbon commands, a text size setting and a high-contrast theme.

The program has about 1 900 texts in `emb-app`, written as string
literals at the widgets (`ui.label("…")`, tooltips, messages, the
`Cmd::meta` table of the ribbon). egui has no translation layer. eframe
has AccessKit on by default, and egui gives each widget its text as its
name; the ribbon buttons and the canvas are painted by the program and
had no name.

## Decision

1. **The English text is the key** (`emb-app/src/i18n.rs`). `tr("Open")`
   gives the text in the active language, or the English text. A text with
   values is a template: `fill("Joined {} objects.", &[&n])` puts the values
   in place of each `{}`, in order, so a translation can move them. The
   texts stay readable in the source, and a missing translation shows
   English, never a key.
2. **One TOML file for each language**, `data/lang/<code>.toml`, with its
   own name and a `[text]` table, built into the program. The template
   `data/lang/template.toml` has all texts with empty values; a test makes
   it again from the source (the literals of `tr(…)` and `fill(…)`, the
   panel titles, the help constants `*_HELP`, and the names of the engine
   that the program shows with `tr(x.name())`) and fails if it is not up
   to date (`UPDATE_LANG=1 cargo test -p emb-app i18n`). A second test fails
   if a language has a text that the program does not have, or a
   translation drops a `{}`.
3. The active language is kept **for each thread** (the program draws on
   one thread), so tests that run in parallel do not change each other.
   Options > Language sets it; `workspace::apply_theme` applies it each
   frame.
4. The texts were marked with scripts (widgets, tooltips, messages, the
   ribbon table, the helper functions that take a label). Texts inside the
   engine crates stay English (the engine has no UI dependency, rule 5);
   the app translates the names that it shows. The help topics in
   `docs/help` stay English for now: they are documents, and a language
   can add its own topic files later.
5. **Accessibility.** `a11y.rs` gives screen reader names to buttons that
   the program paints (each ribbon button, with its on or off state), to
   the icon buttons (their tooltip) and to the canvas ("Design canvas: n
   objects, m selected"). Keyboard: Ctrl+K opens Find a command (all
   ribbon commands by name, with their shortcuts; Enter runs one); F6
   puts the keyboard focus on the ribbon (Tab, Shift+Tab, Enter, Esc).
   Options: a text size (90 % to 200 %, also Ctrl+Plus, Ctrl+Minus,
   Ctrl+0; egui's own keyboard zoom is off, so the setting keeps it) and
   high contrast (black or white panels, strong lines, a yellow or blue
   selection), in the dark and the light mode.

## Consequences

- A change of an English text makes its translations stale: the test
  shows them, and the translator updates the file. This is the usual cost
  of English keys; it is small because the texts change seldom.
- Some texts stay English in this milestone: formatted texts that are
  not messages or widget texts, the plain-language words built from parts,
  the help topics, and the texts of the engine crates (warnings of the
  stitch plan, file errors).
- Window titles are translated, with a fixed id, so a window keeps its
  place when the language changes.
- The first languages (owner decision, 2026-10-24): German, Spanish and
  Japanese, each with all texts of the template. A test keeps each language
  at 95 % of the texts or more, so a new English text can wait for its
  translations.
- A translation can leave out the last values of a text, and `{_}` takes
  a value and shows nothing: the English texts add a plural "s" with a
  value, which other languages do not use.
- Japanese needs a font with its characters. egui has none, so the
  program adds a font of the computer as a fallback after its own fonts
  (Yu Gothic or Meiryo on Windows, Hiragino on macOS, Noto Sans CJK on
  most Linux systems; `icons::CJK_FONTS`). A computer with none of them
  shows boxes in place of the Japanese characters. Shipping a font (Noto
  Sans JP, OFL, about 5 MB) is the fallback if testers have such systems.
- Long words of a translation make the text of a large ribbon button
  smaller, so that no word breaks in its middle.
- `--lang <code>` on the command line runs the program in a language for
  one run (for testers and shops); the settings keep theirs.
