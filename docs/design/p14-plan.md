# Plan: roadmap phase P14 (Sketch mode)

Date: 2026-10-27 · Status: Accepted (owner request, 2026-10-27). Milestones 1 to 6 done in software.

## Goal

The owner asked (2026-10-27) for a parametric sketch mode, as in CAD
programs: the user goes into a sketch, draws lines, arcs and circles,
gives them geometric constraints (horizontal, tangent, equal and others)
and dimensions (lengths, angles, radii), and goes out of the sketch. The
closed regions of the sketch then become fills, satins, runs or appliqué.
A later change of a dimension changes the sketch, and the embroidery
objects made from it change too.

This suits designs with exact geometry: patches, badges, logos, frames,
appliqué pieces and quilt blocks. Hand-drawn art keeps the draw tools of
P2 and P6.

The methods are common CAD practice, in our own words. CAD programs with
sketches are closed source (spec, rule 3); describe their behavior in
our own words only. Open-source constraint solvers (for example the
solvers of FreeCAD and SolveSpace) are C++, so the program cannot call
them (rule 9); an ADR decides if we port a method from one, with a
header (rule 4).

## What we have

- Vector objects with no stitches (ADR 0024), and the conversions to
  fill, satin, outline and appliqué (`emb_model::vector`).
- Snaps to the grid, guides and objects; the Measure tool; numbers that
  the user types for sizes.
- Shaping and offsets (`emb_model::shaping`, `arrange`), and the nesting
  of outlines and holes (`emb_geom::region::nest`).
- Records that make objects again when their source changes: border
  traces and linked shapes (ADR 0025, 0026), blends and monograms.
- Undo through `Document::edit` (ADR 0004).

## Order

| # | Scope | Size | Exit test |
| --- | --- | --- | --- |
| 1 | **Sketch model and solver.** (a) A new engine crate `emb-sketch` (depends on `emb-geom` only): points, lines, arcs, circles and construction lines; constraints coincident, horizontal, vertical, parallel, perpendicular, tangent, equal, concentric, midpoint, point on curve, symmetric and fix; dimensions distance, horizontal and vertical distance, angle, radius and diameter. (b) The solver: the constraints as equations, solved by damped least squares (Newton steps with a damping factor) on each group of entities that share constraints. A drag moves the dragged point as near to the pointer as the constraints let it. (c) The free motions that are left (degrees of freedom), and the constraints that conflict or repeat (from the rank of the equations). (d) An ADR for the solver and the sketch model. | L | Unit and property tests: random sketches that are solvable solve to 10⁻⁶ mm; a sketch with a conflict names the constraints; a fully constrained sketch has 0 free motions; 200 entities solve in less than 20 ms for a drag. |
| 2 | **Sketch mode in the app.** (a) `ObjectKind::Sketch` (no stitches, as vectors) in `Design`; Start Sketch and Finish Sketch (a new Sketch ribbon tab while a sketch is open; the other objects are dim and do not take clicks). (b) Draw tools: line and chain of lines, arc (three points, center, tangent), circle, rectangle, polygon, point, construction line. (c) Constraint commands and the Dimension tool: click the entities, type the value. Constraint marks and dimensions on the canvas; a click on a dimension changes its value. (d) The color of the entities shows the state: free, fully constrained, conflict. The status bar shows the free motions. (e) Drag of entities with the solver. | L | Unit tests of the commands; the smoke test draws a sketch; a tester draws a fully constrained shield shape in less than 10 minutes. |
| 3 | **Drawing help and sketch edits.** (a) Constraints that the program adds while the user draws (horizontal, vertical, coincident, tangent, on a curve), with a mark that shows them before the click and a key that stops them. (b) Trim, extend, split, fillet, chamfer, offset, mirror (with symmetric constraints) and rectangular and circular patterns. (c) Delete an entity with its constraints; Relax (remove the dimensions that conflict, with a list); undo for each step. | M | Unit tests of each edit; a sketch with fillets stays tangent after a change of a dimension. |
| 4 | **Profiles to embroidery.** (a) The closed regions of the sketch (the curves cut each other; outer shapes with holes, `region::nest`). (b) Finish Sketch, then select regions and make a fill, satin fill, appliqué or vector; a satin column between two curves; a run or a line with a width along curves (also construction curves). (c) The stitch settings come from the current properties (P3). | M | Corpus design `sketch-badge` with a golden snapshot; the regions of test sketches with holes and islands are correct. |
| 5 | **Parametric links.** (a) Each object made from a sketch keeps a record (the sketch, the regions or curves, how the object was made), as the linked shapes of ADR 0026. (b) Edit Sketch again (double-click, or the object menu); a change of a dimension or a constraint makes the objects again and keeps their stitch settings, thread and sew order. (c) A region that is gone: the object stays as it is, with a warning and Unlink. (d) A move or a turn of the sketch moves its objects. | M | Unit tests: a width of 60 mm changed to 80 mm makes the fills and the satin border again, with the same settings; undo takes it back in one step; the badge sews on the Topaz 20 before and after the change. |
| 6 | **Parameters and more curves.** (a) Named parameters and expressions (for example `width = 80 mm`, `border = width / 20`) in a Parameters window, used by the dimensions; a template keeps them, so a new design from a template asks for its parameters. (b) Splines through points, with constraints on their points and ends (tangent). (c) A vector object or an imported SVG or DXF to a sketch, with the constraints that the program finds (horizontal, tangent, equal). | M | Unit tests: expressions, a loop of parameters is refused; a badge template with three parameters makes three sizes; an SVG logo becomes a sketch that solves. |

Then the exit test of the phase: the exit tests of each milestone pass.

## Sketch features of the reference CAD program

The owner asked (2026-10-27) for the sketch features of a leading parametric
CAD program (the reference CAD program),
made under the clean-room rules (rule 3): the list below names each job in
our own words; the commands, the icons and the keys are ours. State: ✓
done, ◐ part, ✗ open (with the milestone or batch that takes it).

| Group | Feature | State |
| --- | --- | --- |
| Draw | Line and chain of lines, close on the first point | ✓ |
| Draw | Tangent arc from the end of a line by a drag | ✓ (ADR 0073) |
| Draw | Circle from the center; circle tangent to three lines | ✓ (tangent: ADR 0074) |
| Draw | Ellipse | ✗ (a new curve kind; after Milestone 6) |
| Draw | Arc: three points, center, tangent | ✓ |
| Draw | Rectangle: two points, three points, center, three points from the center | ✓ (ADR 0072, 0074) |
| Draw | Slot: center to center, overall, center point, three-point arc, center point arc | ✓ (ADR 0072, 0074) |
| Draw | Polygon in a circle and round a circle | ✓ (ADR 0072) |
| Draw | Point, center point | ✓ (center point: ADR 0074) |
| Draw | Spline (through points, control points) | ✓ (ADR 0079): cubic pieces with their handles as points; trim and split of splines ✗ |
| Draw | Text in a sketch | ✗ (lettering objects exist) |
| Draw | Project geometry, import (SVG, DXF) | ✓ import (ADR 0079: Vector to Sketch, DXF files); project the edges of other objects ✗ |
| Modify | Trim, extend, split, offset | ✓ (ADR 0070) |
| Modify | Fillet; chamfer with one distance, two distances, a distance and an angle | ✓ (ADR 0074) |
| Modify | Move by a drag (also of a selection) | ✓ (ADR 0072) |
| Modify | Move, copy, rotate, scale with values; stretch | ✓ (ADR 0073, 0074) |
| Pattern | Mirror (symmetric), rectangular, circular | ✓; copies that follow the spacing ✗ (a later step) |
| Constrain | Coincident, collinear, concentric, fix, parallel, perpendicular, horizontal, vertical, tangent, symmetric, equal, midpoint, on curve | ✓ (collinear ADR 0072) |
| Constrain | Smooth (G2) | ✗ (tangent G1 joints of splines ✓, ADR 0079) |
| Dimension | One tool for length, distance, angle, radius, diameter, point to line, with a ghost and a click to place it | ✓ (ADR 0072) |
| Dimension | Aligned, horizontal or vertical distance by the place of the click; the kind by a right click | ✓ (ADR 0073) |
| Dimension | Arc length, diameter to a centerline | ✓ (ADR 0073) |
| Dimension | Driven dimensions (in parentheses), automatic when the size is set already | ✓ (ADR 0072) |
| Dimension | Values with units and calculations | ✓ (ADR 0071) |
| Dimension | Named parameters and expressions | ✓ (ADR 0078), templates that ask for their parameters |
| Dimension | Automatic dimensions and constraints | ✓ (ADR 0074) |
| Drawing help | Snaps: point, midpoint, crossing, on curve; alignment lines | ✓ (ADR 0072) |
| Drawing help | Horizontal, vertical, tangent, parallel and perpendicular (to the line that the pointer went over) | ✓ (ADR 0070, 0072) |
| Drawing help | A key that stops the help | ✓ Alt |
| Show | Constraints on and off (F8, F9); the colors of the state; free motions | ✓ |
| Show | The constraints of a part when the pointer is on it, the parts of a constraint | ✓ (ADR 0073) |
| Show | Relax mode (a drag that changes the dimensions) | ✓ (ADR 0073); Relax of conflicts ✓ |
| Format | Construction curves, driven dimensions | ✓ |
| Format | Centerline (for diameter dimensions), center point | ✓ (ADR 0073, 0074) |
| Use | Esc, Delete, right-click menus, double-click to change a value, box selection, hover light | ✓ (ADR 0071) |

## Not in this phase

| Item | Reason |
| --- | --- |
| 3D models, extrusions, assemblies | An embroidery design is flat. |
| Constraints between two sketches, or between a sketch and other objects | One sketch at a time keeps the solver and the links simple. Revisited after Milestone 5 (ADR 0076): the objects follow their sketch, which is enough for now. |
| A copy of the commands, icons or keys of a CAD program | Rule 3 and the spec: the jobs are common CAD practice; the design is ours. |

## Progress

| # | State | Notes |
| --- | --- | --- |
| 1 | Done in software | ADR 0068. New engine crate `emb-sketch` (depends on `emb-geom` only): `Sketch` (points, lines, circles, arcs counterclockwise with a radius, construction curves; one id space; serde), `Constraint` (coincident, horizontal, vertical, parallel, perpendicular, equal, tangent, concentric, midpoint, on curve, symmetric, fix; distance, horizontal and vertical distance, length, angle, radius, diameter), checks of the curves of each constraint and of the values. `equations`: one or two equations for each constraint, two for each arc, central differences over the few variables of each. `solve`: groups with no shared variables; damped Newton steps of the smallest weighted size with a sparse Cholesky solution (reverse Cuthill-McKee, envelope) and a Levenberg-Marquardt damping; `drag` holds the point at the pointer, else weights it. `analyze`: free motions, fixed points and curves, redundant and conflicting constraints (the newer one). Tests: unit tests of each constraint, a property test of random solvable polygons (64 cases, error under 10⁻⁶ mm), conflicts named, 0 free motions for a fully constrained rectangle, a chain of 200 lines drags in about 6 ms (release). Not done: a sketch far from its constraints can need many steps (ADR 0068, consequences). |
| 2 | Done in software | ADR 0069. `ObjectKind::Sketch` with `emb_model::sketch::SketchObject` (the sketch, a placement for moves, turns and mirrors, a scale into the sketch and its dimensions; the paths of the curves made again after each change; the file keeps the sketch and the placement). App (`emb-app/src/sketch.rs`): Digitize > Shapes > Start Sketch, a double-click on a sketch, the Sketch tab only in sketch mode (Finish Sketch, Select, Line chain, Rectangle, Circle, Polygon, Three-Point, Center and Tangent Arc, Point, Construction; 12 constraint commands that take the selection or wait for clicks; Dimension with a value box in the units of the Options); the other objects dim; colors of the state (free, fixed, conflict, construction, repeated); marks and dimension labels; drag of points, lines, arcs and circle edges (`Sketch::drag_points`); the Properties panel lists the constraints with Change and ✕; the status line gives the free motions; each change one undo step, a drag one step. Help topic `sketch`. Tested in the app on 2026-10-27 (a shield: lines, constraints, a width, a drag, Finish); fixed then: a ribbon command could not change the tab, and the value box showed mm while the label showed inches. Not done: the exit test with a tester (a fully constrained shield in less than 10 minutes). |
| 3 | Done in software | ADR 0070. Drawing help (`emb_sketch::infer`): snap to points (coincident) and curves (on curve), horizontal and vertical within 4°, tangent at the end of an arc; marks next to the pointer before the click; Alt or a check box stops them. Edits (`emb_sketch::edit`): trim, extend, split (new ends on the cutting curves, pieces in line or on one circle), fillet and chamfer (the corner stays as a point on both lines; a length becomes a distance to the corner), offset of chains (parallel at a new dimension `LineDistance`, arcs keep their center, tangents copied), mirror (symmetric points), rectangular and circular patterns (the constraints between the copied curves), relax (conflicting constraints, dimensions first). A tangent at a shared end is now an angle of 90° (exact; the distance form let the end slide by about 10⁻⁵ mm). App: Sketch tab panels Modify and Copy, Relax; refused edits make no undo step; the Dimension tool waits after a line (angle, distance to the line, length). Tests: each edit in `tests_edit.rs` (16 tests), a fillet stays tangent after a change of the width and of the radius; app tests of the line help, the edit tools, mirror, offset, patterns and relax. Owner test on 2026-10-27, then ADR 0071: sketch keys before the keys of the program (Esc goes back to Select, Delete deletes parts and not the sketch, the value box closes with Esc), typed values with units and calculations in all length fields (`workspace::parse_length`), a right-click menu, hover light, box selection, dimension labels out of the shape, units in the Properties panel. Checked in the real app on 2026-10-27: Esc, Delete of a dimension, the menu of a line and of a corner, 4 in into a mm dimension, a fillet of 1/2". Then ADR 0072 (the second request of the owner): the Dimension tool shows the dimension that a click makes as a ghost at the pointer, and a click places it; driven dimensions; label places (drag a label); previews of all draw tools with the points of the clicks; snaps to midpoints and crossings, alignment lines; parallel and perpendicular hints; center and three-point rectangles, slots, polygons round a circle; collinear; a drag of the selection; F8 and F9. Then ADR 0073 (the third request): a tangent arc by a drag from the end of a line, Move, Copy, Rotate and Scale with values and previews, the kind of a distance by the place of its label (and by a right click), arc length, centerlines (a distance to one is a diameter), the constraints of the part under the pointer, relax mode. In the real app on 2026-10-27: the arc drag, the kinds of a dimension, Move with its preview. Then ADR 0074 (the fourth request): the tangent circle, the three-point center rectangle, the slot kinds, the chamfer kinds, stretch, center points, automatic dimensions, and the constraint commands as a grid of icons (the Sketch tab does not fold at about 1450 points). In the real app: the tangent circle and automatic dimensions. |
| 4 | Done in software | ADR 0075. Regions (`emb_sketch::regions`): the curves cut where they cross, the faces of the plane graph with holes and islands, free ends removed, exact paths (lines and arc pieces). Model: `region_objects` (fill, satin, appliqué, vector), `curve_objects` (run, satin line), `SketchObject::satin_between`; current properties and the current thread. App: the Regions tool (pick by a click, shaded), Make Embroidery in the Properties panel and the right-click menu, buttons for all regions of a closed sketch. Corpus design `sketch-badge` with its golden snapshot (a satin ring, a fill with a hole, a fill of the island, a satin line). Tests: regions of a rectangle cut by a line, rings, squares in squares, a disc with a hexagon, two crossing circles, a filleted rectangle, a circle in a triangle (4 regions); conversions; the app picks and makes. In the real app on 2026-10-27: a circle and a rectangle, three regions picked, a fill and a satin made. |
| 5 | Done in software | ADR 0076. `Design::sketch_links` (`emb_model::sketch_link`): a record for each make (the sketch, a region key or curves, how, the objects); `Design::make_from_sketch` for all make commands; `refresh_sketch_links` in the same edit as each change of the sketch (one undo step), the objects in place with their ids, sew order, thread and settings. Region keys (`emb_sketch::RegionKey`, `Sketch::find_region`): the sides with their directions and a point, else more than half of the curves round the point. A lost source: the objects stay, a warning, Unlink. Moves of the sketch move the objects; a move of the objects alone is kept (`offset`). A double-click or Edit Sketch on a made object opens its sketch; Unlink in the Properties panel and on the Artwork tab. Conversions keep the link. Tests: the frame of 60 mm to 80 mm (the fill and the satin border again, settings and thread kept, the same sew order, one undo), moves and turns, a lost region that comes back, conversions, save and load; the badge with diameters 30 mm larger sews in the Topaz 20 hoop before and after (golden test). |
| 6 | Done in software | ADR 0078: expressions (`emb_sketch::expr`: units, powers of length, names, functions), parameters (`emb_sketch::params`: values in the order of their dependencies, loops refused, rename, remove if not in use), dimensions with expressions, the scale of a sketch with parameters (`expr_scale`); the Parameters window, labels `width = 80.00`; templates ask for their parameters (`template::parameters`, `Design::set_sketch_parameter`, the Template Parameters window); a fix: maps by dimension id load from a project (the label places of ADR 0072 did not). ADR 0079: spline pieces (`Geometry::Bezier`, `Shape::Cubic`: nearest points and crossings with Newton steps), `Sketch::add_spline` and the Spline tool (tangents at the joints), tangents and points on splines; `emb_sketch::import` (lines, arcs and circles found in cubic curves, splines, shared ends and centers, horizontal, vertical, tangent and equal constraints with no repeated ones), DXF (LINE, ARC, CIRCLE, LWPOLYLINE with bulges, POLYLINE; `$INSUNITS`); Vector to Sketch, DXF files open as sketches. Tests: expressions, a loop of parameters refused, parameters drive dimensions; a badge template with three parameters makes badges of 60, 90 and 120 mm that sew in the Topaz 20 hoop (golden test); an SVG logo becomes a sketch that solves with no repeated constraints (golden test); splines stay smooth after a drag, regions of closed splines and two crossing splines; DXF arcs, circles of two bulges and inches. In the real app on 2026-10-27: a spline (preview, handles, tangents, a drag), parameters (a dimension with width, a change of width, a loop refused) and Vector to Sketch; fixed then: a drag of a spline point left its handles and made a cusp (the tangent of splines allowed parallel directions); the remove buttons showed an empty box (the font has no ✕; now the icon), and so did the arrow of a loop message; Vector to Sketch is also in the right-click menu. |
