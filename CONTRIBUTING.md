# Contributing to ShokujoDX

Thank you for your help. ShokujoDX is a free embroidery digitizer, and it
gets better with each bug report, sew-out photo, translation and fix.

## Ways to help

- **Report a bug** or a design that sews badly (see below).
- **Sew test designs** on your machine and tell us how they sew. The
  [tester guide](docs/testing/TESTERS.md) tells what to try.
- **Translate** the program (German, Spanish and Japanese exist; more
  are welcome).
- **Improve the help** in [docs/help](docs/help).
- **Write code**: fix a bug or build a feature of the
  [roadmap](docs/design/roadmap.md).

For a large change, open an issue first, so that we can agree on the way
before you start.

## Report a bug

Open an issue on GitHub. Tell us:

- The version and the build: **File > About**, then **Copy build
  information**.
- Your system (Windows, macOS or Linux, and its version) and your
  embroidery machine.
- What you did, what you expected, and what happened.
- If the program stopped: the crash report from the `crashes` folder in
  the settings folder (for example `%APPDATA%\ShokujoDX\crashes`). The
  report has no design data.
- If stitches sew badly: the project file (`.embproj`), the machine file,
  and a photo of the sew-out.

Attach only designs that you are free to share.

**Security problems:** do not open a public issue. Use **Security >
Report a vulnerability** on the GitHub page of the repository.

## Set up

You need [Git](https://git-scm.com) and [Rust](https://rustup.rs) 1.95 or
later. On Linux, also install the libraries of the window system:

```bash
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

```bash
git clone https://github.com/K1ngNatsu/ShokujoDX.git
cd ShokujoDX
cargo run -p emb-app --release
```

## Before you send a pull request

Run these three commands. CI runs them too, on Windows and Linux (and on
macOS for a release), and a pull request must pass them:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Code for one system only (for example `#[cfg(windows)]`) must also build
with no warnings on the other systems. If you have no Linux computer, the
Linux check of CI tells you.

Keep a pull request to one topic. Say what it changes and why, and how you
tested it.

## Rules of the project

[docs/SPEC.md](docs/SPEC.md) is the specification. Read it before a large
change. These rules are the most important:

1. **Clean room.** Do not decompile, disassemble or copy code, assets,
   icons, help text, shortcut tables or sample files of closed-source
   embroidery software. Describe what such software does in your own
   words only, from its public documentation, and do not name commercial
   programs in the code or the documentation (write "commercial
   programs" or "CAD programs").
2. **Ported code** from Ink/Stitch (GPL-3.0) or pyembroidery (MIT) gets a
   header comment with the source file and the commit hash.
3. **Engine crates** (`emb-geom`, `emb-model`, `emb-stitch`,
   `emb-formats`, `emb-render`, `emb-letter`, `emb-art`, `emb-library`,
   `emb-script`, `emb-sketch`)
   have no UI dependency and no C or C++ library. A lower crate does not
   depend on a higher crate.
4. **No `unsafe` code.** Each crate has `#![forbid(unsafe_code)]`. The one
   exception is `emb-thumbnail` (the Windows thumbnail handler, ADR 0046).
5. **Units:** lengths are mm as `f64` everywhere outside `emb-formats`.
6. **Design changes** go through `emb_model::Document::edit`, so that undo
   works (ADR 0004).
7. **Tests:** add or update tests with each change. If a golden snapshot
   changes (`crates/emb-cli/tests/golden.rs`), say in the commit message
   why the stitches changed.
8. **Decisions:** record an architecture decision as a short ADR in
   [docs/adr](docs/adr).
9. **Writing:** write documentation and comments in simple, clear English
   ([ASD-STE100](https://www.asd-ste100.org/)): short sentences, one idea
   each, active voice.

### Text in the program

- A new text on the screen goes through `crate::i18n::tr("…")`, or
  `crate::i18n::fill("… {} …", &[&value])` for values. Then run
  `UPDATE_LANG=1 cargo test -p emb-app i18n` to update
  `data/lang/template.toml`, and add the text to `de.toml`, `es.toml`
  and `ja.toml` (ask in the pull request if you cannot translate it).
- Command names, panel titles and menu items are in title case
  ("Dim Vectors"); tooltips and messages are sentences.
- A new command needs a clear tooltip: the help topics of the ribbon are
  made from the tooltips.

### Translations

Each language is one file in [data/lang](data/lang), built into the
program (`FILES` in `crates/emb-app/src/i18n.rs`). To add a language, copy
`template.toml` to `<code>.toml` (for example `fr.toml`), fill in the
texts, add the file to `FILES` and to the test
`the_languages_cover_the_program`, and open a pull request. To improve a
translation, change its file. The help topics are in English only.

## License of your contribution

ShokujoDX is under the [GNU General Public License, version 3 or later](LICENSE).
When you send a contribution, you agree that it is under the same license,
and you confirm that you have the right to give it. Fonts must have an
open font license (for example the SIL Open Font License), with the
license text next to the font.

**Your email address:** each Git commit shows the email address of its
author. To keep your private address out of the public history, use your
GitHub noreply address
([how to set it](https://docs.github.com/en/account-and-profile/setting-up-and-managing-your-personal-account-on-github/managing-email-preferences/setting-your-commit-email-address)).
