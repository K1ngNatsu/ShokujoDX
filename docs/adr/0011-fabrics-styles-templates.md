# ADR 0011: Fabrics change settings by difference; current properties live in the design

Date: 2026-10-21 · Status: Accepted

## Context

P3 Milestone 6 adds fabric presets, the current properties (the settings of
new objects), styles and templates. Three questions needed an answer:

1. How does a fabric change objects that already have settings? A fabric
   that sets absolute values removes the tuning that the user made on each
   object (for example a denser satin for small text).
2. Where do the current properties live? Before this milestone they were
   in the user settings file, so a design or a template could not carry
   them.
3. What is a style: a link that objects follow, or a copy?

## Decision

- **Fabrics are relative.** A fabric (`data/fabrics.toml`) has a spacing
  factor, a fill stitch length factor, a pull compensation offset and an
  underlay weight (light, normal, heavy), relative to the base settings of
  the app (woven cotton). A change from fabric A to fabric B changes each
  satin and fill object by the difference between A and B
  (`emb_model::FabricChange`). The design records its fabric id. The change
  of fabric, current properties and objects is one undo step.
- **Underlay weights.** `Underlay::Light` and `Underlay::Heavy` are new
  automatic modes next to `Underlay::Auto`. A fabric sets the weight of
  automatic underlay. No underlay and custom layers are choices of the
  user, so a fabric does not change them.
- **The current properties are part of the design** (`Design::current`).
  The property bar edits them through `Document::edit` (ADR 0004). The user
  settings keep only the shape settings of the drawing tools.
- **A style is a copy**, stored in the design (`Design::styles`). Apply
  style copies the settings; a later change to the style does not change
  objects. This keeps the object model simple: an object has no link that
  can break.
- **A template is a project file** with the extension `.embtemplate`
  (ADR 0005 format) in the user's `templates` folder. It carries the
  machine, hoop, fabric, current properties, styles, threads and,
  optionally, objects.

## Consequences

- The project format stays at version 1: the new fields are optional and
  an older file loads with the defaults.
- A change to a fabric and back gives the first values again, except where
  a value was at a limit (pull compensation is never less than 0).
- An old settings file with stitch settings for new objects loses them;
  new designs start with the built-in settings or a template.
- Styles do not update objects. If users ask for linked styles, an object
  can get a style name later, without a change to this model.
