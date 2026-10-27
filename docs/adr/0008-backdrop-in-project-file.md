# ADR 0008: The backdrop picture is stored in the project file

Date: 2026-10-21 · Status: Accepted

## Context

Tracing (roadmap P2) puts a bitmap under the design. The project file
(ADR 0005) must keep it, and undo must work on its position, size and
settings.

## Decision

- `Design::backdrop` holds the image file bytes as they are (PNG, JPEG or
  BMP) in an `Arc<Vec<u8>>`, with the pixel size, the top-left corner and the
  width in mm, the strength (opacity), and the show and lock flags.
- The project file has the bytes as base64 (crate `base64`). A project is one
  file; it cannot lose its picture when it is moved.
- The model does not decode the image. The app decodes it with the crate
  `image` (pure Rust; PNG, JPEG and BMP only) and keeps one texture, made again
  only when the bytes change. Pictures larger than 4096 pixels per side are
  made smaller for the texture only.
- Changes go through `Document::edit`, like all design changes (ADR 0004).
  Undo snapshots share the bytes, so they do not copy the picture.
- The project format version stays 1: the field is optional, and older files
  load with no backdrop.

## Consequences

- A project file with a picture is about 4/3 of the picture size larger.
  This is acceptable for tracing pictures (usually less than a few MB).
- One backdrop per design. Several pictures, cropping and adjustment come
  with the bitmap tools in P6.
- Rule 9: `image` and `base64` are pure Rust. `image` is in the app only.
