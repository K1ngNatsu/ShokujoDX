# Plan: four follow-ups after the audit of 2026-10-29

Status: Done (prepared on 2026-10-29; the owner asked to do all four
the same day). Items 1, 2, 3 and 4 are done. These are not features of a phase:
they make the program easier to use in other languages and easier to
change. Each item is one commit or a few, with the checks of rule 8.

| # | Item | Size | Needs the owner |
| --- | --- | --- | --- |
| 1 | Translate the texts of the engine crates | Large (about 300 texts) | No; a native speaker should check the new texts |
| 2 | Split `ribbon.rs` and remove the long argument lists | Medium | No |
| 3 | Skip the CI build for commits that change only documents | Small | Yes: the branch rules on GitHub |
| 4 | Help topics in German, Spanish and Japanese | Very large (38 topics, about 30 000 words a language) | Yes: after the review of the UI texts |

Recommended order: 3 (small), 2 (no change to what the user sees), 1,
then 4 after a native speaker has checked the texts of the program.

## 1. Texts of the engine crates

**Now.** ADR 0042 keeps the texts of the engine crates in English (rule 5:
no UI dependency). The user sees many of them: the warnings of the stitch
plan (`PlanWarning::message` is a `String`, from `StitchError`), the
errors of the file readers (`emb-formats`, `thiserror` texts), the
other sketch errors (the refusals of `emb_sketch::edit::MESSAGES` and
`ExprError::MESSAGES` are translated already, through `engine_texts`), script errors
(`emb-script`), the labels of the printed sheets and catalogs
(`emb-render`, `emb-library`), and the stop notes
(`StopReason::text`). A rough count of the literals: emb-model 51,
emb-formats 50, emb-render 53, emb-script 34, emb-letter 21, emb-sketch
18, emb-library 16, emb-art 6, emb-stitch 4.

**Plan.**

1. A small text type in `emb-model` (no UI dependency): `Text { key:
   &'static str, values: Vec<String> }`. The key is the English template
   with `{}` for each value, as `i18n::fill` takes it. `Display` gives the
   English text, so logs, the CLI and the tests stay as they are.
2. Engine errors and warnings keep their enums; each gets `fn text(&self)
   -> Text`. `PlanWarning` gets a `Text` in place of its `String`
   (`message()` still gives English).
3. The app shows a `Text` with `i18n::text(&t)` (`fill(tr(key), values)`).
   The places that show engine texts now (Info panel warnings, messages
   after a failed open or import, the Sketch tab refusals, the Scripts
   window) change to it.
4. Printed sheets and catalogs: the `emb-render` and `emb-library`
   functions take a label function (`&dyn Fn(&str) -> String`); the app
   passes `tr`, the CLI passes the identity.
5. The template test (`i18n::tests`) also reads the literals of
   `Text::new("…")` in the engine crates, as it reads `tr("…")` now, so
   `UPDATE_LANG=1` finds the new texts. Then the texts are translated in
   `de.toml`, `es.toml` and `ja.toml`.
6. A new ADR changes point 4 of ADR 0042.

**Done (2026-10-29), ADR 0083.** The `Text` type of step 1 was not needed:
a translation hook in `emb_geom::text` (the base of all engine crates)
does the work with no change of the engine types. The engine writes its
messages with `text::tr` and `text::fill`; the program sets the
translator with the language. Names that are data stay English. 219 texts
are translated. The app also translated two texts that the earlier pass
missed (the undo label "Draw …" and "… object(s) skipped"), and it shows
the type names of objects in its language.

**Risk.** Golden snapshots use English texts (`emb_cli::summary`): they
must not change. The test of step 5 must not take texts that are only for
the developer (panics, debug output).

## 2. `ribbon.rs` and the long argument lists

**Now.** `crates/emb-app/src/ribbon.rs` has 5 262 lines: the `Cmd` enum
and its `meta`, `can` and `run` (about 1 500 lines), the panels of each
tab, the button drawing, the drop-down controls and the File menu, and
the tests. Seven functions have `#[allow(clippy::too_many_arguments)]`:
`effects_ui.rs:19`, `panels.rs:19`, `panels.rs:1710`,
`sequence_panel.rs:214`, `sketch.rs:2343`, `emb-stitch/src/network.rs:235`,
`emb-stitch/src/rings.rs:181`.

**Plan.**

1. Make `ribbon/` a folder module, with the same public items:
   `mod.rs` (`RibbonTab`, `Panel`, the drawing of the ribbon),
   `commands.rs` (`Cmd`, `Flag`, `meta`, `can`, `run`), `tabs.rs` (the
   panel tables of each tab, `S_*` of simple mode), `widgets.rs` (buttons,
   frames, tooltips, focus), `menus.rs` (the drop-down controls and the
   File menu), and `tests.rs`. Only moves: no change of behavior. The
   tests that read the ribbon (help topics made from the ribbon, the
   different icons of each tab, the all-commands test) find any error.
2. Each of the seven functions gets a small struct for the values that go
   together (for example the rect, the colors and the state of a row in
   the Sequence panel), and the `allow` goes. The two in `emb-stitch` must
   give the same stitches: the golden snapshots check it.
3. `CLAUDE.md` and ADR 0010 name the new files.

**Risk.** Only the size of the diff: it is one commit of moves (easy to
review with `git diff --color-moved`), then one commit for the structs.

**Done (2026-10-29).** `ribbon/` has `mod.rs` (347 lines: the tabs and the
drawing of the ribbon), `commands.rs` (2 147: most of it the table of
`Cmd::meta`), `run.rs` (609), `tabs.rs` (797), `widgets.rs` (390),
`controls.rs` (548) and `tests.rs` (466). A check of the lines found only
the moved lines, the new `mod` and `use` lines and the file comments. The
template test of the translations now reads the module folders too (and
not `tests.rs` files). The seven `allow`s went: two were not needed any
more (`rings::guided`, `sketch::draw_label` have seven arguments); the
network walk is a method of a `Tree`; the three sliders of the object
properties take a struct of what they show (`Effect`, `Param`,
`FillSetting`); the color block header of the Sequence panel takes a
`Block`. No golden snapshot changed.

## 3. CI for commits with documents only

**Now.** `.github/workflows/ci.yml` builds and tests on three systems for
each push to `main` and each pull request, also when only a document
changed.

**Plan.** Add `paths-ignore` to `push` and `pull_request` for the files
that the program does not build in: `docs/adr/**`, `docs/design/**`,
`docs/testing/**`, `README.md`, `CONTRIBUTING.md` and `CLAUDE.md` (no
code or test reads them). Not `docs/help/**` (built into the program and
checked by tests) and not `data/**`. Tags are not
filtered (GitHub does not apply path filters to tags), so a release still
builds.

**Owner.** If a branch rule on GitHub asks for the CI checks before a
merge, a pull request with documents only waits for checks that never
run. Then either the rule goes, or a small job that always passes runs for
such pull requests. The owner looks at the branch rules first.

**Done (2026-10-29).** `main` has no branch protection and no rulesets,
so nothing waits. The list also has `docs/SPEC.md` (no code reads it).
The engine check of CI also checks `emb-script` and `emb-sketch` (it had
8 of the 10 engine crates). A test
(`packaging::the_ci_skips_only_documents_and_checks_each_engine_crate`)
fails if the list has a path that the program builds in, or if the engine
check leaves out an engine crate.

## 4. Help topics in other languages

**Now.** The 38 help topics (`docs/help`, built in by `help.rs`) are
English only (ADR 0042, point 4).

**Plan.**

1. A topic in another language is a file of the same name in
   `docs/help/<code>/` (`de`, `es`, `ja`). `help.rs` takes the topic of the
   active language if there is one, else the English topic, so the
   languages can grow one topic at a time.
2. A test checks that each translated topic has the same name, group and
   links as the English topic, and that each link goes to a topic that
   exists.
3. The topics made from the ribbon (tabs and keys) are made from the
   translated command names already, so they need no files.
4. Search: the keywords of a translated topic are in its language and in
   English, so an English word still finds it.
5. The work: first the start group (getting started, basics, questions
   and problems), then the tutorials, then the rest. In Simplified
   Technical English the sentences are short, which helps the translation,
   but a native speaker must check each language.

**Owner.** Start it after a native speaker has checked the texts of the
program (an open exit test of P10), so the help uses the same words.

**Done (2026-10-29).** All 38 topics are in German, Spanish and Japanese
(ADR 0084). The test `help::tests::each_translation_matches_its_topic`
checks the blocks, lists, tables, links and code of each one. The help
uses the words of the program, and the work found texts of the program
to correct: the Run button of the Scripts window, the theme choice
("Dark theme", "Light theme"), the German "Distance" ("Entfernung"), and
names of the engine that were not translated (a new test,
`i18n::tests::each_engine_name_is_in_the_template`). A native speaker must
still check the UI texts and the help of each language.
