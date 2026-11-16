# Code signing: the steps (SignPath Foundation)

Date: 2026-11-16 · Status: to do. The decision is ADR 0062; this file is
the checklist. Source: the conditions at https://signpath.org/terms and
the GitHub guide at https://docs.signpath.io/trusted-build-systems/github
(read on 2026-11-16).

The certificate is in the name of the SignPath Foundation. Windows shows
"SignPath Foundation" as the publisher, so the name of the owner stays
private.

## 1. Two-factor sign-in (owner)

SignPath requires multi-factor authentication for each team member, on
GitHub and on SignPath. GitHub: Settings > Password and authentication >
Two-factor authentication.

## 2. Code signing policy in the README (Claude)

The home page must have a section with the title "Code signing policy"
(or a link with that name to a page). It contains:

- The sentence "Free code signing provided by SignPath.io, certificate by
  SignPath Foundation", with links to https://about.signpath.io and
  https://signpath.org.
- The team and its roles: authors (trusted committers), reviewers (each
  change from other people, for example a pull request, gets a review),
  approvers (each signing request gets a manual approval). Now: the owner
  (K1ngNatsu) has all three roles.
- Privacy: "This program will not transfer any information to other
  networked systems unless specifically requested by the user or the
  person installing or operating it." (The program has no network code;
  checked on 2026-11-16.)

## 3. Apply (owner)

Apply at https://signpath.org/apply. Have ready:

- the repository: https://github.com/K1ngNatsu/ShokujoDX
- a release: https://github.com/K1ngNatsu/ShokujoDX/releases/tag/v0.3.0
- the license: GPL-3.0-or-later
- a short description of the program
- the link to the code signing policy in the README
- the GitHub account of the team: K1ngNatsu

The conditions (the project meets them): an OSI license for all parts
with no commercial dual license, no proprietary parts, actively
maintained, released in the form to sign, the features described on the
download page, privacy and an uninstaller, builds from the source on
GitHub-hosted runners only. SignPath decides case by case; it can take
some days or weeks.

## 4. After the acceptance (Claude)

- Give the thumbnail handler (`emb_thumbnail.dll`) a version resource
  (product name ShokujoDX and the version), as `emb-app/build.rs` does for
  the program. SignPath checks the product name and version of each
  signed file.
- Add the signing to `.github/workflows/release.yml`, with the action
  `signpath/github-action-submit-signing-request@v3`. Each file goes up
  with `actions/upload-artifact` first. Two signing requests for each
  release:
  1. `ShokujoDX.exe` and `emb_thumbnail.dll`, before Inno Setup makes
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
