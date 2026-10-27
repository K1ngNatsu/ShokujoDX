# ADR 0033: Test builds for test users

Date: 2026-10-23 · Status: Accepted

## Context

The owner wants embroiderers who digitize to try the program on their
own Windows computers, before P8, for the open user exit tests of P2 and
P3 and for the features that the next phases need. Installers, code
signing and builds for macOS and Linux are P10 work.

## Decision

- **A zip, not an installer.** `tools/package-windows.ps1` runs the tests,
  builds the release program and writes
  `dist/EmbroideryDigitizer-<version>-<build>.zip`: the program, its `.pdb`
  file, the licenses (GPL and the font licenses), the tester guide, the
  feedback form (`docs/testing/`) and sample designs from the test
  corpus. The program has all its data inside (fonts, hoops, machines,
  fabrics, thread charts), so it runs from any folder; nothing is
  installed and no administrator rights are needed. The build has no
  signature: the guide says how to pass the Windows SmartScreen message.
- **Each build names itself.** `emb-app/build.rs` puts the git commit (with
  `+changes` for files that are not in a commit) and its date in the
  program. The About window (File > About) shows them and copies them for
  a report; the zip name has them too. The version is 0.1.0.
- **Crash reports.** A panic writes a report (the build, the error, the
  place and a backtrace, no design data) to the `crashes` folder next to
  the settings file; at the next start the program says where it is.
  The `.pdb` file of a normal release build (about 10 MB) gives the
  function names in the backtraces. Line tables would add the files and
  lines, but they make the `.pdb` 90 MB and the zip 40 MB; the owner wants
  the zip under 20 MB (it is about 15 MB). The automatic backups (ADR
  0013) keep the work.

## Consequences

- A test build is made from a commit, so each report maps to the code.
- The macOS and Linux builds, the installers and the signature stay in P10.
  ADR 0043 adds them (P10 Milestone 8); the zip is now
  `<name>-windows-portable.zip`, and a build of a version tag has no commit in its name.
