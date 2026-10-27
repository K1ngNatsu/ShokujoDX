# ADR 0068: The sketch model and the constraint solver

Date: 2026-10-27 · Status: Accepted (P14 Milestone 1)

## Context

Sketch mode (P14) needs lines, arcs and circles that keep geometric
constraints and dimensions while the user drags them and changes values.
Open-source solvers (the solvers of FreeCAD and SolveSpace) are C++, so
the program cannot call them (rule 9). The methods are textbook numerical
methods; we write our own in Rust and port no code.

## Decision

1. **A new engine crate `emb-sketch`** that depends on `emb-geom` only.
   It has the sketch model, the constraint equations and the solver.
2. **Model.** A sketch has points, curves and constraints, each with an id
   of one id space. A curve is a line (two points), a circle (a center
   point and a radius) or an arc (a center, a start and an end point,
   counterclockwise, and a radius). A curve can be a construction curve
   (it helps to draw, and it does not become embroidery). Sizes are mm,
   angles degrees. The model is serde, so the design file can keep it
   (Milestone 2).
3. **Variables and equations.** The variables are the x and y of each
   point and the radius of each circle and arc. Each constraint is one or
   two equations that are 0 when it is true. An arc has two more
   equations: its start and end are at its radius from its center.
   The derivatives are central differences over the few variables of each
   equation, so a new constraint needs only its equation.
4. **Solver.** The equations are split into groups that share no
   variables, and each group is solved alone. Each step is a damped
   Newton step of the smallest weighted size
   (`Δ = −W⁻¹Jᵀ(JW⁻¹Jᵀ + λI)⁻¹F`), with a line search and a damping λ
   that goes up when a step does not make the error smaller. The smallest
   step keeps a sketch that has free motions near to where it is.
   `JW⁻¹Jᵀ` is sparse: a sparse Cholesky solution in reverse
   Cuthill-McKee order (envelope storage) solves it. A step that does not
   make the error smaller after three halvings raises λ (as
   Levenberg-Marquardt). The solver stops when each equation is under
   10⁻¹⁰ mm, or after 100 steps. (Dense solutions and conjugate gradients
   were tried: the first was too slow at 300 equations, the second
   converged too slowly.)
5. **Drag.** The dragged point goes to the pointer and the sketch is
   solved with the point held there. If the constraints do not let it be
   there, the point gets a large weight in `W` instead, so the solver
   moves the other variables first: the point stays as near to the
   pointer as the constraints let it.
6. **Free motions and conflicts.** The rows of the Jacobian at the
   solution go into an orthonormal basis one at a time (the arc equations
   first, then the constraints from the oldest). A row that adds nothing
   is a constraint that repeats others (if the equations are true) or
   conflicts with them (if they are not). The free motions are the
   variables less the rank; a point is fully constrained if both its
   variables are in the row space.

## Consequences

- A fully constrained sketch needs a fixed point or dimensions from a
  fixed point, as in CAD programs: else it can still move as a whole.
- Tangency is distance from a center equal to the radius; it does not
  check that the touch point is on the arc. Milestone 3 adds constraints
  that the drawing tools need.
- The analysis is dense and slower than a drag; the app makes it again
  when the sketch changes, not at each pointer move.
- A sketch that is far from its constraints (for example a long chain of
  lines that all must turn) can need many steps or not solve. In the app
  each constraint comes from the drawing, which is near to it, and a drag
  starts from a solved sketch; then the solver needs a few steps. A test
  chain of 200 lines (302 equations) drags in about 6 ms (release build).
