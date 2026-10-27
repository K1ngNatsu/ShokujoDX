# ADR 0078: Sketch parameters and expressions

Date: 2026-10-27 · Status: Accepted (P14 Milestone 6)

## Context

The owner wants the sketch features of a CAD program (P14 plan). Milestone
6 (a) adds named parameters and expressions: `width = 80 mm`,
`border = width / 20`, used by the dimensions, and templates that ask for
their parameters, so one badge template makes badges of many sizes.

## Decision

1. **Expressions** (`emb_sketch::expr`): numbers, units after a number
   (mm, cm, m, in, ", ft, pt; deg, °, rad), the names of parameters,
   `+ - * / ^`, parentheses, `pi`, and the functions sqrt, abs, min, max,
   sin, cos and tan (degrees). A usual recursive descent parser in our own
   code. Each value has a power of length (0 a plain number, 1 a length in
   mm, 2 an area); `*` and `/` add and subtract the powers, so `width / 20`
   is a length and `width * width` is not. In a sum, a plain number is a
   length in the units of the sketch (`Sketch::units`, the units of the
   Options when the first parameter is made), as CAD programs do. A
   dimension takes a length (a plain number in the units of the sketch),
   an angle takes a plain number.
2. **Parameters** (`emb_sketch::params`, `Sketch::parameters`): a name and
   an expression. The values are found in the order of their dependencies;
   a parameter that uses itself, directly or through others, is a loop and
   is refused with the names of the loop. A name has letters, digits and
   `_`, starts with a letter, and is not a unit or a function. Rename
   changes the expressions that use the name; a parameter in use cannot be
   removed.
3. **Dimensions with expressions** (`Sketch::exprs`, by dimension id): the
   value box takes an expression when the text has the name of a
   parameter; a plain value ends the expression. After each change of a
   parameter, `Sketch::apply_parameters` gives the dimensions their new
   values, then the sketch is solved and its linked objects are made again
   (ADR 0076), all in one undo step. A change that makes a dimension wrong
   (for example a negative length) is refused with no change. A driven
   dimension takes no expression.
4. **A scale of a sketch with parameters** keeps the expressions and
   multiplies their lengths (`Sketch::expr_scale`), so a scaled design stays
   the same size after a later change of a parameter. The Parameters window
   shows the factor.
5. **The app**: the Parameters window (Sketch tab > Dimension > Parameters)
   lists the name, the expression and the value of each parameter, adds,
   renames and removes them, and sets the units of plain numbers; errors
   show in the window. A dimension label with a parameter shows
   `width = 80.00`, with another expression `ƒx 80.00`. The Properties panel
   of a closed sketch lists its parameters.
6. **Templates** (`emb_model::template::parameters`): the parameters of
   the sketches of a template are its questions. New from Template opens
   the Template Parameters window with each parameter and its value; Make
   Design sets them (`Design::set_sketch_parameter`) and makes the objects
   again, then opens the new design. A wrong value shows its error and the
   design is not made.
7. **A fix**: the project file reads maps by dimension id (the label
   places of ADR 0072 and the expressions) through a JSON value, which does
   not change a text key into an id; a small serde helper (`id_map`) reads
   and writes them. A project with a dragged dimension label could not be
   opened before; it opens now. The file form did not change.

## Consequences

- One sketch has its own parameters; parameters shared by two sketches
  are a later step.
- A scale factor of the expressions is not visible in the values of the
  parameters; the window shows it.
