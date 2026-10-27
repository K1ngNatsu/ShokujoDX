# ADR 0003: Same stitch output on all platforms

Date: 2026-10-21 · Status: Accepted

## Context

Golden-file tests compare stitch output with stored snapshots. CI runs on
Windows, macOS and Linux. The spec requires deterministic output.

Rust `f64::sin`, `cos`, `atan2` and similar functions call the platform math
library. Results can differ in the last bit between platforms. One bit of
difference can move a stitch, change a rounding, and break a snapshot.

## Decision

1. Engine crates use `emb_geom::math` (a wrapper on the `libm` crate) for all
   transcendental functions. `sqrt` and `+ - * /` are exact under IEEE 754, so
   `std` is safe for them.
2. `math::sin_cos_deg` returns exact values for multiples of 90°, so quarter-turn
   rotations add no error.
3. Do not iterate over a `HashMap` or `HashSet` where the order changes the output.
   Use `BTreeMap`, `IndexMap` or sorted vectors.
4. No multithreading inside the stitch generation of one object.
5. Golden snapshots round coordinates to 0.01 mm.

## Consequences

- Code review must check for `f64::sin` and similar calls in engine crates.
  A later CI step can search for them.
