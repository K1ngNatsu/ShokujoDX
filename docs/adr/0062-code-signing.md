# ADR 0062: Code signing through the SignPath Foundation

Date: 2026-10-26 · Status: Accepted (owner decision); to do after the first public release

## Context

The Windows installer and program have no signature, so Windows shows
"Windows protected your PC" and the publisher "Unknown". A code signing
certificate puts the name of the publisher in each signed file. The owner
wants to keep their name private (the commits use the GitHub noreply
address), so the name on the certificate matters.

The options checked on 2026-10-26:

- **SignPath Foundation**: free for open source projects. The certificate
  is in the name of the SignPath Foundation, which is shown as the
  publisher. Conditions: an OSI license for all parts, a public and
  maintained repository, a release that already exists, the features
  described on the download page, a code signing policy, and signing from
  the CI build.
- **Azure Artifact Signing** (Microsoft): about $9.99 a month; for
  individuals only in the USA and Canada; the certificate shows the legal
  name of the owner.
- **Certum open source certificate**: from about €49 a year; the
  certificate shows the name of the owner.

A signature does not stop the SmartScreen warning at once: the reputation
of a signed program grows with its downloads (Microsoft: EV certificates
no longer change this).

## Decision

Sign the Windows packages through the SignPath Foundation, after the
repository is public. The order of the work:

1. Make the repository public.
2. Turn on private vulnerability reporting (Settings > Security; GitHub
   has it only for public repositories). CONTRIBUTING.md points to it.
3. Tag `v0.1.0`. The release workflow builds the Windows, macOS and Linux
   packages (macOS minutes are free when the repository is public) and
   makes a draft release; the owner publishes it unsigned. Remove the "no
   release yet" note from the README.
4. Apply at signpath.org with the repository and the release.
5. When SignPath accepts the project: add a code signing policy to the
   README and the SignPath signing step to `.github/workflows/release.yml`
   (the program, the thumbnail handler DLL and the installer).

macOS signing and notarization need the Apple Developer Program, which
shows the legal name of the owner; that stays open. Until then the README
tells Mac users how to open the program.

## Consequences

- Release 0.1.0 is not signed.
- If SignPath stops the subscription, the releases after that are not
  signed; the program still works.
