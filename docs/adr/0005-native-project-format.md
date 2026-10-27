# ADR 0005: Native project format is versioned JSON

Date: 2026-10-21 · Status: Accepted

## Context

The spec asks for a versioned, human-readable project file (JSON or RON) that
keeps objects and parameters.

## Decision

- Format: pretty-printed JSON, file extension `.embproj`.
- Top level: `{ "format": "emb-project", "version": 1, "design": { ... } }`.
- The code is in `emb_model::project`, not in `emb-formats`, because it stores the
  model, not stitches.
- The loader rejects a different `format`, a newer `version`, and broken references
  (duplicate ids, an object that uses a thread that is not in the palette).
- Older versions are migrated step by step on the JSON value before the design is
  read (`project::migrate`).
- Compact forms keep files small and readable: points are `[x, y]`, path commands
  are `{"M":[x,y]}`, `{"L":[x,y]}`, `{"C":[[..],[..],[..]]}` and `"Z"`, colors are `"#rrggbb"`.
- Parameters have defaults (`#[serde(default)]`), so a file from an older version
  with fewer parameters still loads.

## Reasons

- JSON has more tools (viewers, diff tools, `jq`, other languages) than RON.
- Other programs can read and write designs with no Rust code.

## Note: exact float round trip

`serde_json` by default parses floats with a fast method that can be wrong in the
last bit. We enable the `float_roundtrip` feature. Without it, a save and load
changed coordinates by one bit, and the round-trip test failed.
