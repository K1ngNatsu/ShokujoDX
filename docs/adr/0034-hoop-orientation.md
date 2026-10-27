# ADR 0034: Hoop orientation on the machine

Date: 2026-10-23 · Status: Accepted (owner test of the first test build)

## Context

The owner reported that the canvas showed the Crown hoop across (260 mm
left to right), but the Topaz 20 showed the designs upright. Two test
files settled it:

- A small file (an arrow to the top of the design, an F to its right):
  the machine shows the arrow to the back of the machine and the F
  upright. Its hoop picture is upright and says "260 x 200". So the top of
  the design is at the back of the machine, and the hoop is 200 mm from
  left to right and 260 mm from the front to the back. Husqvarna Viking
  gives the front-to-back size first.
- A wide file (240 mm, too wide for 200 mm): the machine turns the design
  a quarter turn by itself, with the top of the design to the right.

So the preset had its width and height the wrong way round, and every
design wider than 200 mm sewed turned.

## Decision

- **Width is left to right on the machine.** A hoop preset has `width_mm`
  from left to right and `height_mm` from the front to the back. The top
  of the canvas is the back of the machine. The Crown hoop preset is
  200 × 260 mm; its name keeps the maker's "260 x 200". The other presets
  are classes of many brands and stay as they are.
- **A back mark on the canvas.** The canvas shows a small arrow and "Back"
  at the middle of the back edge of the hoop field. It turns with the
  hoop, so with a turned hoop the user sees where the top of the design
  goes on the machine.
- **One turn direction.** A turned hooping (and a hoop position) has the
  hoop angle `hooping::TURN_DEG` (−90°): on the machine the top of the
  design is to the right, as when the machine turns a file itself. The
  corpus designs that fit only across the hoop (inside the safety
  margin) use the same angle (`corpus::fit_the_hoop`), and a corpus
  design that fits but is not on the hoop center gets the hoop at its
  middle.
- **Words on the sheets.** The worksheet gives the hoop size "from left
  to right" and "from the back to the front", puts "Back of machine"
  over the hoop diagram, and calls a hoop angle of a quarter turn a turn
  of the design (not a slant). The hooping sheet says where the top of
  the design is for a turned hooping.

## Consequences

- 21 golden snapshots changed. Turned in the machine files: the 14
  corpus designs that do not fit the upright field inside its 5 mm
  safety margin, and the four hoopings of `multi-hoop` (now 2 × 2 with
  turned hoopings, which the auto layout chooses over 3 × 2 straight
  ones). `hoop-placement` turns its hoop −55° (the bar is on the long side
  of the hoop) and starts at the front end of the bar. `buttonholes` and
  `outline-types` are on a new hoop center. The stitches are the same,
  turned or moved; `repeats` has one more jump because its entry and
  exit points come from the needle position (ADR 0007).
- Projects keep the hoop by its id, so a saved project gets the upright
  hoop when it opens. A design that was laid out across the hoop now
  shows outside it; Auto hoop or a turn of the hoop fixes it.
- The P7 sew-out kit was made again; the old kit must not be sewn.
