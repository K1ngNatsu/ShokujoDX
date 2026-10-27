# ADR 0080: A clean start for new users

Date: 2026-10-27 · Status: Accepted (owner request)

## Context

The packages (ADR 0033, 0043) put 14 designs of the test corpus in a
`samples` folder, and the start page showed them as "Sample designs", with
the names of the test files (`multi-hoop`, `repeats`, `rhinestones` and
others). The File menu had Add Sample Objects, a command of the first
prototype that adds a small test patch, and the empty Sequence panel told
the user to use it. The owner wants a new user to start with a clean slate.

## Decision

- The packages ship no sample designs: the scripts do not make the corpus
  and do not copy designs, and `tools/samples.txt` goes. A test
  (`packaging::no_package_ships_samples_or_settings`) checks that no
  package script ships samples, the corpus or a settings file.
- The start page has no Sample designs section; it shows Start, Learn and
  the user's own recent designs (empty on a new install).
- Add Sample Objects is not a command any more. `AppState::add_sample_objects`
  stays for the tests only. The empty Sequence panel tells the user how to
  start (draw with the Digitize tab, add lettering, or open a design).
- What a new install starts with is in the user's folders only: the
  settings, the recent designs and the backups to recover are made when
  the user works, so a new user sees none of them.

## Consequences

- The test corpus stays the source of the golden tests and of the demo
  designs for the developers (`emb-cli corpus`).
- The release checks of the thumbnails (Linux, macOS) use a test design of
  the repository, `packaging/test/two-color-patch.embproj`, which is not in
  a package (`packaging::the_release_checks_use_the_test_design`).
- Good example designs for users can come later, made for that purpose,
  with names and pictures for users.
- The test zip still has the tester guide and the feedback form
  (ADR 0033); a release for all users can leave them out.
