# ADR 0138: EPS files, and a message for CDR files

Date: 2026-11-20 · Status: Accepted (P18 Milestone 8,
`docs/design/p18-plan.md`; changes ADR 0027 for EPS)

## Context

Customers of a shop send logos as EPS files, AI files and CDR files.
ADR 0027 read PDF, AI files with a PDF inside, EMF and WMF, and not EPS
(a PostScript interpreter) or CDR (no public description). The plan of
P18 Milestone 8 was to convert such files with a program that the user
has (a vector editor). The owner changed the milestone (2026-11-20): the
program must not depend on a second program. A shop must not install a
vector editor to open a logo, a helper program changes between versions
and systems and cannot be tested in CI, and the program must not run
another program on the files of customers.

## Decision

- **A PostScript interpreter in pure Rust**: `emb_formats::eps`, from the
  public PostScript language reference (third edition) and the EPS file
  format specification (version 3.0). No C library (rule 9); the free
  interpreters are C code.
  - The machine (`eps/vm.rs`): composite objects in tables of the machine
    (an object is a small value with an index, so a dictionary can contain
    itself and all memory goes at the end), an execution stack with
    frames (no recursion in Rust: a deep PostScript recursion is an error,
    not a crash), tail calls, `stopped`, `exit`, errors that `stopped`
    takes, else the run goes on after the object that caused the error.
  - The scanner (`eps/scan.rs`): the syntax of section 3.2, lenient.
  - About 300 operators (`eps/ops.rs`, `eps/graphics.rs`): the stacks,
    arithmetic (`emb_geom::math`), arrays, strings, dictionaries, types,
    control, `save` and `restore` (the graphics state only), files
    (`currentfile`, reads, the filters ASCII85, ASCIIHex, RunLength, Flate
    and SubFileDecode, which read when they are read first), resources,
    the graphics state, the transform, paths with arcs, color spaces
    (device, CIE and ICC as device spaces, Indexed, Separation and DeviceN
    with their tint procedures, Pattern), painting, forms. Operators of a
    printer with no effect on the artwork take their operands.
  - Not drawn: text (counted; a Type 1 font with an encrypted part is
    defined and skipped), pictures (counted; their data is read and
    skipped), patterns and gradients (gray, as in ADR 0027), smooth
    shadings (counted), the clip (as in the PDF import). `clippath` gives
    the page, which is not painted; `nulldevice` paints nothing;
    `strokepath fill` is the stroke; `gsave fill grestore stroke` is one
    path with a fill and a stroke.
  - The page is the bounding box (`%%HiResBoundingBox`, else
    `%%BoundingBox`, also at the end); a file with a binary header gives
    its PostScript part; the first page with artwork is imported.
  - Limits: 40 million steps, 256 MB, 2,000 errors; the import keeps what
    the program drew before it stopped, and a warning says why.
- **The AI script** (`eps/ai.rs`): AI files of the old kind (PostScript,
  before AI files had a PDF inside) draw with the short operators of a
  procedure set that the file does not always contain. A file with the
  comments of an AI script (and not the printing procedure set of newer
  files) runs its script part (after `%%EndSetup`) with the reader's own
  operators: paths, paint, compound paths, clip paths, the colors (gray,
  CMYK, RGB, named colors with a tint), gradients (gray), text objects and
  pictures (counted), hidden layers (not imported). A file runs in the
  other way when the first way draws nothing. The published description
  of the AI format could not be read when this was written (its copies
  were offline); the operators follow the format as it is known, and the
  files of testers must check them.
- **The file types**: `.eps`, `.ps` (and `.epsf`, `.epsi`) read as
  PostScript; `.ai` and `.pdf` read as PostScript when they are PostScript
  inside, else as PDF. `artwork::import_artwork` gives the same objects as
  the other readers (fills and lines, lines, or vectors), so Import
  Artwork, the drop on the window and the outlines of Recognize Objects
  (ADR 0137) take EPS files.
- **CDR**: a CDR file gives a message: the format has no public
  description; save the file as SVG, PDF or EPS in the program that made
  it. A port of the open reader of the format (a large C++ library) can
  come later if testers get CDR files that they cannot convert.
- **No helper program.** If testers need it, a later option can convert a
  rare format with a program that the user has (off by default).

## Consequences

- The exit test (`crates/emb-cli/tests/eps.rs`): three EPS files of the
  same badge (short procedures with a transform; a large printing
  procedure set with a spot color, a font with an encrypted part and a
  picture with compressed data; an AI script) open as the same six
  objects with their colors and sizes, and sew; a CDR file gives the
  message. The PostScript of these files is our own. The unit tests
  (`eps/tests.rs`) check the operators, the painting rules, the limits
  and 150 damaged programs.
- The files of other programs are not tested yet: the printing procedure
  sets of commercial programs are large, and an operator that the reader
  does not have makes an error (the warning names the first one). The
  exit test of the owner uses the EPS and AI files of testers; each
  missing operator is a small addition.
- Text in an EPS file must be outlines, as in the PDF import.
- `emb-formats` uses `flate2` (already in the build through `lopdf`).
