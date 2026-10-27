# ADR 0060: CI in a private repository

Date: 2026-10-26 · Status: Accepted (owner decision)

## Context

The project is now in a private GitHub repository
(`K1ngNatsu/Ariadne`). Actions minutes of a private repository are
limited, and macOS minutes count about ten times (Windows about two
times). CI ran fmt, clippy and the tests on Windows, macOS and Ubuntu for
each push to `main` and each pull request.

## Decision

1. Each push to `main` and each pull request checks on Windows and
   Ubuntu.
2. macOS checks a tag `v*` and a run by hand (`workflow_dispatch`:
   Actions > CI > Run workflow); then all three platforms check. The
   matrix comes from the event (`fromJSON` of one of two lists).
3. The release workflow (ADR 0043) does not change: a tag builds and
   tests the packages of the three platforms.
4. The commits use the GitHub noreply address of the owner, not a
   personal address (all commits were rewritten before the first push).

## Consequences

A change that breaks only macOS shows at the next tag or manual run. Run
CI by hand before a tag when a change touches macOS code (packaging,
file dialogs, the thumbnail extension).
