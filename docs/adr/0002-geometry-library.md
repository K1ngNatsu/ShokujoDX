# ADR 0002: Geometry library is geo with i_overlay

Date: 2026-10-21 · Status: Accepted

## Context

The engine needs polygon booleans (union, difference, intersection) and offsets
(pull compensation, underlay inset). The spec named Clipper2 or i_overlay.

## Decision

`emb-geom` uses the `geo` crate (0.33) for polygon types, booleans and offsets.
`geo` uses `i_overlay` for these operations. Both are pure Rust.
Default features of `geo` are off, so `rayon` multithreading is not used.

`emb-geom` keeps its own types (`Point`, `Path`, `Polyline`, `Region`) and converts
to `geo` types only inside `emb_geom::region`. Other crates do not use `geo` directly,
so a change of library changes only one module.

## Reasons

- The Rust Clipper2 crates call the C++ library through FFI. That needs `unsafe`
  and a C++ toolchain, which rule 9 does not permit in engine crates.
- `i_overlay` is pure Rust and maintained, and `geo` is the standard Rust geometry crate.

## Notes

- Round joins in `i_overlay` use a segment-length-to-radius ratio, and the
  library rounds the segment count down. `emb_geom::region::offset` uses half of
  the step that the flatten tolerance gives, so the error stays within 0.05 mm.
  A test checks this.
- Miter joins: corners sharper than 10° are cut off.
