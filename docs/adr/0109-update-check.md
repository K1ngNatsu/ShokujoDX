# ADR 0109: A check for a new version

Date: 2026-11-18 · Status: Accepted

## Context

A user of an old version does not learn that a new version exists (the
review of 2026-11-18). The owner wants a simple check, and the user
chooses at the install if the check is automatic (Phase P17 Milestone 1,
`docs/design/p17-plan.md`). Until now the program did not use the
network, and the README said so.

## Decision

- **What the check does** (`emb-app/src/update.rs`). It asks GitHub for
  the newest release of the program (the public address
  `api.github.com/repos/K1ngNatsu/ShokujoDX/releases/latest`) and compares
  the version with the version of the program. A newer version shows in
  the **Check for Updates** window with its notes, **Open Download Page**
  and **Skip This Version**. The program does not download and does not
  install a file.
- **When.** At the start, not more than one time each day, and only if
  the user said yes (`Settings::updates.auto`; `None` means no answer and
  no check). **Help > Check for Updates** and **Check Now** in Options
  check at once. An automatic check that finds nothing new, or that
  fails, shows nothing.
- **The choice of the user.** The Windows installer has a tick box; it
  writes `update_check=yes` or `no` to `install.ini` next to the program,
  and the program applies the choice of each install one time (by the
  time of the file). "No" turns the checks off. "Yes" turns them on only
  if the user did not answer before, so a user who turned the checks off
  in Options keeps them off after an upgrade. With no installer (the
  portable zip, macOS, Linux), the first-start window has the tick box.
  A user of an earlier version gets one question at the next start. The
  lettering kiosk (`--kiosk`) does not ask.
- **The request is made by the `curl` program of the system**, started by
  its full path (`System32\curl.exe` on Windows 10 and later,
  `/usr/bin/curl` on macOS and Linux), in a thread, with a time limit of
  15 s, HTTPS only, and a size limit. The program gets no network library:
  an HTTPS library brings about forty crates and cryptography code in C
  and assembly (`ring`) into the program for one request a day, and it
  does not build for the Linux check on Windows. A system with no `curl`
  gets a clear message and the link to the download page.
- **Privacy.** The request has no data of the user, no design data, no
  version of the program and no identifier. GitHub sees what each web
  request shows (the IP address, the time, the name `ShokujoDX`).
- **Safety of the answer.** The answer is data from the network. The
  program reads the version (digits only), the notes (plain text, at most
  4000 characters, no control characters) and the address of the release.
  The system opens an address only if it is a page of our releases with
  plain characters; for each other answer the fixed releases page opens.
- **What is new.** `Settings::updates.seen_version` keeps the version of
  the last start. At the first start of a new version, the start page
  has a link to the help topic **What is new** (`docs/help/whats-new.md`),
  which lists the changes of each version. Add the changes of a release
  to this topic and to its translations.

## Consequences

- The README and the help say that the program uses the network for this
  check only, and only if the user permits it. With the check off, the
  program makes no request (a test checks that the start makes none).
- `Cargo.lock` does not change.
- A silent install (no pages) ticks the box, as for the other tasks; an
  administrator can clear it with the `/TASKS` option of the installer.
- **Reset to defaults** in Options clears the answer, so the program asks
  again (as it asks for the machine again).
- A Linux system with no `curl` cannot check; the window says so.
- The check trusts HTTPS to GitHub. A false answer can only show a wrong
  version number and notes, and open a page of our releases.
