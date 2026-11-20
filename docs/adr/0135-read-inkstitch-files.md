# ADR 0135: Read Ink/Stitch files

Date: 2026-11-20 · Status: Accepted (P18 Milestone 5,
`docs/design/p18-plan.md`)

## Context

Ink/Stitch (GPL-3.0) is an embroidery extension of Inkscape. It keeps the
stitch settings of each shape of an SVG file in attributes of its own
namespace (`http://inkstitch.org/namespace`), and its commands (trim, stop,
start point and more) as symbols that a connector links to a shape. Before
Milestone 5, the program read such a file as plain artwork: a fill for
each filled shape, a run for each stroke, and the symbols and connectors
of the commands as more shapes.

## Decision

- **Engine**: `emb_formats::inkstitch` (`is_inkstitch`, `import_inkstitch`).
  `svg::import_svg` uses it for a file with an element or an attribute in
  the namespace of Ink/Stitch, or with the `embroider_` attributes of its
  first versions. Import as Vectors and Import as Lines do not use it.
- **Sources (rules 3 and 4)**: the names, the defaults and the meanings of
  the settings, the commands, the markers, the metadata and the updates of
  earlier files are from the source of Ink/Stitch, commit
  42e771e9d4338ce32a80f4d8c0df186a8c8210e7 (2026-11-03). Two rules are
  ported, with a header: which two subpaths of a satin column are its
  rails (`rail_indices`) and which rails to reverse (`get_rails_to_reverse`).
  The rest of the reader is new code.
- **Method**: usvg reads the geometry, the styles and the transforms.
  Before usvg reads the text, the reader gives each element an id and puts
  a small shape in place of each command symbol (at the point of the
  symbol). The walk of the usvg tree then finds the settings of each shape
  by its id, and the point of each command. In a clone (`use`), usvg gives
  the shapes no id: they get the ids of the shapes of the source, in order.
  Paths with no id are parts of markers; connectors and lines with a marker
  of Ink/Stitch are not sewn.
- **Stitch types** (the rules of `lib/elements/utils/nodes.py`):
  - A fill becomes a fill: tatami (Square with one stagger), contour (rings
    `Offset`, one spiral `Spiral`, two spirals `Maze`), guided (the guide
    lines of its group), circular (the target point), meander (squares),
    cross stitch (cell size and the full, half and upright crosses) and a
    linear gradient (a group of fills, one for each stop color, with the
    rows across the gradient line). The angle turns the other way (Ink/Stitch
    counts counterclockwise on screen). The underlay is tatami layers at
    the underlay angles, relative to the top rows. The end row spacing is
    an accordion; `expand_mm` grows the area. A tartan fill and an unknown
    fill type are tatami, with a warning.
  - A stroke with the satin setting and two or more subpaths becomes a
    satin column: the rails, the rungs (where they touch the rails; with
    no rungs, the middle nodes pair up when the rails have the same number
    of nodes), the spacing, the pull compensation (one value for each
    rail), short stitches, the split length and the underlay layers (center
    walk, contour, zigzag, in that order). With one subpath and a stroke
    wider than 1 mm (the metadata `min_satin_stroke_width_mm`), it is a
    column of the stroke width along the line. E stitch and zigzag are
    column types; S stitch is satin, with a warning. With no split length,
    the column has the split length of the program (7 mm); Ink/Stitch does
    not split.
  - Another stroke becomes a line for each subpath: a run (stitch length,
    bean stitches, two passes as Backtrack, "1 0" bean stitches as
    sculpture run), a zigzag of the stroke width (a line with a zigzag
    border), a ripple (line count, target point, guide line) or manual
    stitches (the nodes are the needle points). `cutwork_needle` makes a
    cut line.
  - A fill and a stroke of one shape are a group; `stroke_first` sews the
    stroke first.
- **Commands and functions**: trim and stop (commands or `trim_after`,
  `stop_after`) go to the last object of the shape; the starting and
  ending points are the start and end points of the objects; `ties` sets
  the lock stitches. A shape with `ignore_object` or in a layer with
  `ignore_layer` becomes a vector object (Ink/Stitch does not sew it).
- **Earlier versions**: the SVG version of the file
  (`inkstitch_svg_version`) gives the changes of `lib/update.py`: the
  `embroider_` attributes, the old values of `ties`, `fill_method` and
  `stroke_method`, a stroke with no dashes as a zigzag, no underlay and a
  run of 1.5 mm in the first versions, 3 mm stitches in fills before
  version 4, the half pull compensation of zigzag lines before version 3,
  and the old names of the commands.
- **Colors**: the fill or stroke color of each shape (the first stop of a
  gradient). When the metadata `thread-palette` names a chart of the
  program, a new thread with a color of the chart gets its name, brand,
  number, kind and weight.
- **Warnings**: a setting that changes the stitches and that the program
  does not read, with a value that is not the default of Ink/Stitch, gives
  one warning with the number of objects; so does a setting that is not
  in the list of Ink/Stitch (a newer version). Settings with no effect on
  the stitches (random seed, tolerances, the lock stitch shape, travel)
  give none.
- **Import**: `ImportedObject` has the start and end points, the machine
  functions and a group number; `SvgImport::chart` is the chart of the
  file; `SvgImport::add_to` adds the objects to a design (threads, points,
  functions, new group numbers). **File > Open** opens an SVG file as a new
  design (`AppState::open_svg_design`); a drop of an Ink/Stitch file opens
  it the same way; **Import Artwork** adds its objects to the open design.
- **Not read**: the tartan sett, the meander tile patterns, S stitch, the
  random settings, push compensation, the split methods, the ripple
  settings other than the line count, the target, the guide and the end
  scale, the pattern marker, the origin and stop position commands, and
  SVGZ files (usvg is built without them).

## Consequences

- The automated part of the exit test is `crates/emb-cli/tests/inkstitch.rs`:
  a badge written by hand in the structure of Ink/Stitch files
  (`tests/data/inkstitch-badge.svg`: layers, command symbols with
  connectors, metadata with a palette) opens with its seven objects, their
  kinds, settings and colors, and it sews (a stitch plan with no warnings,
  the stop, a DST file). The exit test of the owner (three files of testers,
  one sewn) is open.
- The engine of the program sews the settings. Where it is not the same as
  Ink/Stitch, the stitches differ: for example, a contour fill puts its
  needle points evenly along the rings, so long stitches cut the sharp
  corners of a star.
- The plain readers of SVG, PDF, EMF and WMF now give their warnings
  through `text::fill`, so they are translated.
