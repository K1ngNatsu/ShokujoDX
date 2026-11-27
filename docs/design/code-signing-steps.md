# Code signing: the steps (SignPath Foundation)

Date: 2026-11-16 · Status: step 2 done (2026-11-27); step 3 (the
application) is the owner's. The decision is ADR 0062; this file is the
checklist. Source: the conditions at https://signpath.org/terms and the
GitHub guide at https://docs.signpath.io/trusted-build-systems/github
(read on 2026-11-16; the terms and the form read again on 2026-11-27).

The certificate is in the name of the SignPath Foundation. Windows shows
"SignPath Foundation" as the publisher, so the name of the owner stays
private.

## 1. Two-factor sign-in (owner)

SignPath requires multi-factor authentication for each team member, on
GitHub and on SignPath. GitHub: Settings > Password and authentication >
Two-factor authentication.

## 2. Code signing policy in the README (Claude) — done 2026-11-27

The README has the section "Code signing policy", and the Install
section links to it. The section has:

- The sentence "Free code signing provided by SignPath.io, certificate by
  SignPath Foundation", with links to https://about.signpath.io and
  https://signpath.org.
- The files that get the signature (the program, `emb-cli`, the
  thumbnail handler, the installer) and the build that makes them.
- The team and its roles: committers and reviewers (each change from
  other people, for example a pull request, gets a review), approvers
  (each signing request gets a manual approval). Now: the owner
  (K1ngNatsu) has all the roles.
- Privacy: "This program will not transfer any information to other
  networked systems unless specifically requested by the user or the
  person installing or operating it." The only network use is the update
  check (ADR 0109), on the choice of the user; the GitHub privacy
  statement applies to it.

The terms also ask for the term "Code signing policy" on the download
and release pages: the notes of each release end with a link to the
section (the notes of v0.6.0 got it on 2026-11-27, before the
application).

## 3. Apply (owner)

The form at https://signpath.org/apply (read on 2026-11-27; * is a field
that must be filled):

| Field | Answer |
| --- | --- |
| Project Name * | ShokujoDX |
| Repository URL * | https://github.com/K1ngNatsu/ShokujoDX |
| Homepage URL * | https://github.com/K1ngNatsu/ShokujoDX |
| Download URL | https://github.com/K1ngNatsu/ShokujoDX#install (the page must say that the project uses the SignPath Foundation for code signing; the Install section does) |
| Privacy Policy URL | https://github.com/K1ngNatsu/ShokujoDX#code-signing-policy (only needed for a program that collects user data; this program does not) |
| Wikipedia URL | Empty |
| Tagline * | Free, open-source embroidery digitizing for Windows, macOS and Linux. |
| Category * | Graphics |
| Description * | ShokujoDX is a free, open-source embroidery digitizer. It turns shapes, lettering and pictures into stitches and writes the files that home and industrial embroidery machines read (DST, PES, JEF, EXP, VP3 and others). It has the full workflow of a commercial digitizer: running, satin and fill stitches with many stitch types, lettering with embroidery fonts and the fonts of the computer, picture to design, a 3D view of the stitches, and production sheets. It is written in Rust, licensed under GPL-3.0-or-later, and its interface and help are in English, German, Spanish and Japanese. |
| Reputation * | The facts of the day of the application: the age of the public repository, the releases and their downloads (`gh api repos/K1ngNatsu/ShokujoDX/releases`), the web version on GitHub Pages, the tests and the CI on three systems, the help with its tutorials. The reputation is small, so SignPath can refuse; then apply again when the program has more users |
| Maintainer Type | Individual maintainer(s) |
| Build System * | GitHub Actions |
| First Name *, Last Name *, Email * | The owner's (the SignPath user account; the certificate and Windows show "SignPath Foundation") |
| Company Name | Empty |
| Primary Discovery Channel * | AI / LLM tools (the research of ADR 0062) |
| Exact source | Optional |
| Check boxes | The Code of Conduct and the storage of personal data must be ticked; the other communications are optional |

The conditions (the project meets them): an OSI license for all parts
with no commercial dual license, no proprietary parts, actively
maintained, released in the form to sign, the features described on the
download page, privacy and an uninstaller, builds from the source on
GitHub-hosted runners only, and a verifiable reputation for a program
that people download and start. SignPath decides case by case; it can
take some days or weeks.

## 4. After the acceptance (Claude)

- Give the command-line tool (`emb-cli.exe`) and the thumbnail handler
  (`emb_thumbnail.dll`) a version resource (product name ShokujoDX and
  the version), as `emb-app/build.rs` does for the program. SignPath
  checks the product name and version of each signed file.
- Add the signing to `.github/workflows/release.yml`, with the action
  `signpath/github-action-submit-signing-request@v3`. Each file goes up
  with `actions/upload-artifact` first. Two signing requests for each
  release:
  1. `ShokujoDX.exe`, `emb-cli.exe` and `emb_thumbnail.dll`, before Inno Setup makes
     the installer (Inno Setup packs the files, so SignPath cannot sign
     them inside the installer);
  2. the installer.
  The portable zip gets the signed program too.
- Fix the AppImage warnings of the release build at the same time: a
  copy of the metainfo as `usr/share/metainfo/shokujodx.appdata.xml` in
  the AppDir, and `appstream` on the runner so that `appimagetool` checks
  it. Try the change in a practice build ("Run workflow") first.
- Practice build, then the next release.

## 5. The owner's part after the acceptance

- Install the SignPath GitHub app (https://github.com/apps/signpath) on
  the repository.
- Make an API token in SignPath and keep it as the repository secret
  `SIGNPATH_API_TOKEN` (Settings > Secrets and variables > Actions). Only
  the owner handles the token.
- Give Claude the SignPath organization id and the project slug (they
  are not secret).

## Each release after that

The owner approves the two signing requests on signpath.io; the release
build waits for them.

## What to expect

A signed installer shows "SignPath Foundation" as the publisher, not
"Unknown publisher". SmartScreen can still warn for the first signed
releases: the reputation of a program grows with its downloads. macOS
signing needs the Apple Developer Program (the legal name of the owner)
and stays open (ADR 0062).
