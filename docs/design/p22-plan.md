# Plan: Checks, swatches and agents (phase P22)

Date: 2026-11-25 · Status: Accepted (owner, 2026-11-25: "build those
ideas into a new phase with milestones ordered so work builds
efficiently on itself"). Milestone 1 is made (ADR 0167).

## Goal

The program gets the ideas of the comparison with zigzag that are worth
adding (`docs/design/open-work.md`, "Ideas from zigzag"):

- it tells the user where travel shows, and what a setting costs in
  trims and in thread that shows, while the user changes it;
- it makes test swatches to sew on the fabric of the user;
- satin columns can lean, and can have more than one thread across;
- it checks and fixes machine files in place;
- the measures of the letters agree with what people see, and a trial
  of a new way to sew the joints of letters;
- the open tabs come back after a quit, and a file opens in the program
  that runs;
- the AI agent of the user can use the program (MCP).

Each milestone uses the work of the milestones before it, so that no
work is done two times.

## Where the ideas come from

zigzag (`github.com/simpsoka/zigzag`) is an open-source embroidery
program in Rust and egui, under the Apache-2.0 license. The comparison
of 2026-11-25 read its README, its changelog and its code; it did not
run the program. Code under Apache-2.0 can go into this program
(GPL-3.0-or-later): a ported part gets a header with the source file and
the commit (rule 4), and the license text goes in the third-party
licenses. Describe its behavior in our own words. Its README names
closed-source programs: do not copy those names into this repository.

## The ideas and the milestones

| Idea | Milestone |
| --- | --- |
| A warning for travel that shows | 1 |
| Settings with live numbers; a window that makes a design quicker to sew | 2 |
| Test swatches | 3 |
| Satin settings: lean, and more than one thread across a column | 4 |
| Fix a machine file in place; a check of the writers with pyembroidery | 5 |
| Letter measures against the eye | 6 |
| Overlapped strokes in letters (a trial) | 7 |
| The open tabs come back; one program | 8 |
| Agents through MCP | 9 |
| Send to the Brother Skitch over Bluetooth | Not in this phase (see the end) |

## Order

```text
M1 travel that shows ─┬─► M2 live numbers, Quicker to Sew ─┐
                      └─► M5 machine files: check and fix ─┤
M3 test swatches ───────► M4 satin lean and bands ─────────┤
M6 letters against the eye ─► M7 overlapped strokes ───────┤
M8 tabs come back, one program ────────────────────────────┴─► M9 agents (MCP)
```

- Milestone 1 makes the measure that Milestones 2, 5 and 9 show.
- Milestone 3 comes before Milestone 4: the swatches of the new satin
  settings are its sew-out.
- Milestone 6 comes before Milestone 7: the trial is measured with
  measures that agree with the eye.
- Milestone 8 comes before Milestone 9: an agent shows its design in the
  program that runs through the inbox of Milestone 8.
- Milestone 9 comes last: its tools are the functions of the phase.

The lettering milestones (6 and 7) do not use Milestones 1 to 5. They
can move before them if the owner wants them first.

Each milestone with a window or a command also has: the texts through
`tr` with German, Spanish and Japanese, the help topic and its
translations, the pictures of the windows (ADR 0110), an ADR, and its
row in the inventory (`docs/design/feature-inventory.md`, section 9).

## Milestone 1: travel that shows (M)

Today the plan hides travel where it can: under objects that sew later
(hidden travel, ADR 0082) and along the rows of a fill (ADR 0166). But
the program does not tell the user where travel lies on top of stitches
that sew before it. The measure of ADR 0166 is a test for tatami fills
only.

1. `emb_stitch::shown_travel`: for a stitch plan, the travel stitches
   that lie on top of stitches sewn before them (a grid of the sewn
   stitches, as `emb_stitch::hidden` has), with the length of each run
   and the object that it goes from and to. Travel under stitches that
   sew later does not count.
2. A warning in Design info for each object with such travel ("Travel
   shows on top of sewn stitches: {} mm"); a click selects the object.
   The Check panel of the View tab (`emb_render::check`, with Fabric
   Shows Through and High Thread) gets a view of the travel that shows.
3. Design info shows the length of travel that shows, next to the trims.
4. A snapshot of the corpus (`crates/emb-cli/tests/travel_shows.rs`): the
   travel that shows in each corpus design. It is a new snapshot, so the
   golden snapshots do not change.

**Exit test:** a test case with travel straight across sewn rows (the
travel of before ADR 0166) gives the length of that travel; a design
with only hidden travel gives 0; the corpus numbers are in the snapshot;
the measure of the largest corpus design takes less than 10 % of the
time of its plan.

**Made (2026-11-25, ADR 0167).** The plan marks its travel stitches
(`Run::travel`, `StitchPlan::travel`; the stitches do not change, and
the golden snapshots stay). `emb_stitch::shown_travel` measures, at
points 0.2 mm apart, the travel that no later stitch covers: on top of
the stitch sewn last there if it crosses it (not at the edge of an
area, where the rows turn), or on the fabric. The first measure counted
all stitches under the travel; the pictures of the corpus showed that it
marked the travel along the edge of a fill and along the lanes between
sections, so the top stitch and the turns of the needle path decide.
Design info shows the travel that shows and warns of more than 2 mm on
sewn stitches in an object, or a piece of 4 mm on the fabric (not in
lace); View > Check > Travel That Shows draws it (red on stitches, blue
on the fabric). The snapshot `tests/travel_shows.rs` keeps the numbers
of the corpus.

The exit test is met in part: the travel straight across 25 rows gives
9.1 mm (the test `travel_across_sewn_rows_shows`), hidden travel gives 0
(`hidden_travel_does_not_show`), and the snapshot runs in CI. The time is
not under 10 %: the design with the most stitches (`full-field`, 43,680
stitches) plans in 2.5 ms and the measure takes 1.7 ms; over the corpus
the measure takes about 11 % of the time of the plans (32 ms against
300 ms; at most 5.5 ms for one design, `embroidery-fonts-8` with 3.9 m of
travel). The measure runs once for each plan (Design info), so this is
small; the test `the_measure_of_the_corpus_is_quick` (release) keeps it
under 15 %. Three steps made it faster: one pass over the records, the
cells that the travel goes through with a table of sums (a stitch far
from all travel is left out with one look), and each turn of the needle
path in one cell.

What the measure found in the corpus (open work): travel straight across
sewn rows in guided fills (55 and 42 mm) and a circular fill (37 mm),
which ADR 0166 did not change; a fill in `shaping` whose travel crosses
its own cut-out part (19 mm on the fabric); travel of 8 to 20 mm across
the fabric in lettering of five corpus designs.

## Milestone 2: live numbers and Quicker to Sew (M)

1. **Sew across gaps.** Today the trim distance is a setting of the
   machine (`MachineProfile::trim_gap_mm`, My Machines). A design gets
   its own value, "Sew across gaps up to" (none by default: the value of
   the machine). While the user drags it, the plan thread makes the plan
   again, and the program shows the trims, the length of travel that
   shows (Milestone 1) and the sew time, and marks on the canvas the
   travel that the value adds.
2. **Quicker to Sew** (a window; Arrange tab, Sew Order panel, next to
   Fewer Color Changes): steps that the user can turn on and off, each
   with a value:
   - merge threads that look almost the same (a color difference; the
     thread that sews more stays);
   - remove small areas and details (the size, as Clean Up does);
   - sew across small gaps (step 1);
   - fewer color changes (`Cmd::OptimizeColors`).

   A table shows the threads, color changes, trims, stitches, sew time
   and travel that shows, before and after. The canvas shows the result
   while the values change; OK keeps it as one undo step, and Cancel
   goes back (as the Clean Up window does).

**Exit test:** for each corpus design, the numbers show less than 1 s
after the last change of a value; OK is one undo step; Cancel gives the
design of before (the same project text); the numbers after OK are the
numbers that the window showed.

## Milestone 3: test swatches (M)

The user finds the settings for a fabric by sewing them out.

1. `emb_model::swatch`: copies of the selected objects in a grid. One
   setting changes in equal steps from one value to another: spacing,
   stitch length, pull compensation, column width, fill angle, or
   underlay. Each copy has a label with its value under it (lettering in
   a font for small text, at its smallest size or more). The grid fits
   the hoop, or it goes on more hoopings (ADR 0032) with a message.
2. A window Test Swatches (Design tab, Output panel) with a preview. The
   result is a new design tab, so the design of the user does not
   change.
3. `emb-cli swatch`, so that the tools that make the sew-out kits can
   use it.

**Exit test:** five copies of a satin with spacing from 0.30 mm to
0.50 mm have the spacings 0.30, 0.35, 0.40, 0.45 and 0.50 mm and these
labels; no label touches a copy; the labels use the units of the
program; each setting works for each object kind that has it; the owner
sews a sheet of swatches on the Topaz 20.

## Milestone 4: satin lean and bands of threads (M)

1. **Lean.** `SatinParams::lean_deg` (0 by default): each top stitch
   leans from square across the column by this angle. At the open ends
   of a column the stitches fan from the corner, so that no needle point
   goes out of the column. The underlay does not lean. Lettering parts
   with satin can have it. Use: on a knit, stitches that lean a little
   do not sink between the rows of the knit; a larger angle is a look.
2. **Bands of threads.** More than one thread across one column: a group
   of satin columns with the same rails, each with a band of the width
   (a new column setting with the number of the band, the number of
   bands and an overlap). This is the same model as the tartan and
   gradient fills (ADR 0047: a group of fills, each sews only its
   rows). The Ink/Stitch writer writes each band as a satin column.
3. Swatches (Milestone 3) of the lean (0° to 15°) and of the overlap of
   the bands (0 mm to 0.6 mm) go in a sew-out kit.

**Exit test:** the mean angle of the top stitches to the rungs is the
lean within 1°; no needle point out of the column, and no pairs that
cross (`satin::pairs_cross`); bands with no overlap leave no gap wider
than 0.1 mm, and two layers are only in the overlap; the golden
snapshots do not change (lean 0, one band); the owner sews the swatches.

## Milestone 5: machine files: check, fix and cross-check (M)

Recognize Objects (ADR 0134) changes a machine file into objects. Many
users only want to sew a file from a customer with no faults.

1. **Check.** For a stitch object (a machine file opened as stitches)
   and for `emb-cli check <file>`: stitches shorter than the shortest
   stitch of the machine, stitches longer than its longest, color blocks
   of the same thread one after the other, long jumps with no trim,
   travel that shows (the measure of Milestone 1 on the runs of the
   file, `emb_art::parts`), pieces between two trims too short for the
   trimmer, and starts and ends with no lock stitches. They show as
   warnings in Design info.
2. **Fix Stitches** (Edit tab, Machine Functions panel): join color
   blocks of the same thread when nothing sews between them, remove
   stitches that are too short
   (the next stitch starts at the point before), split stitches that are
   too long, trim long jumps, and add lock stitches at starts and ends.
   Each fix has a check box and its count; OK is one undo step. The
   other needle points do not move. `emb-cli fix <in> <out>` does the
   same.
3. **A cross-check of the writers.** `tools/check-writers.py` writes the
   corpus in each format with `emb-cli`, reads the files with
   pyembroidery, and compares the needle points and the color changes.
   It needs pyembroidery (`pip install pyembroidery`: a download that
   the owner permits).

**Exit test:** a test file with each fault (made by a test) is fixed,
with the right count for each fix; the needle points that no fix
touches keep their place exactly; the files of each writer of the corpus
read back in pyembroidery with the same needle points (to 0.1 mm) and
color changes, or the report gives the reason of each difference.

## Milestone 6: letter measures against the eye (S)

The letter fault bench (P21) counts faults from the needle points. No
test shows that the counts agree with what people see.

1. **A judged set.** The letters of the fault bench and a part of the
   survey of 127 fonts (ADR 0158), as proof pictures
   (`tools/letter-faults.py`). Each letter is clean or not, with the
   fault that the eye sees (a spike, a bulge, an open corner, an open
   area, a column on top, other). Claude judges from the proof pictures;
   the owner looks at 50 of them and corrects them. The set is a file in
   `crates/emb-cli/tests/data/`.
2. `emb-cli letter-faults --judged <file>` compares: for each measure,
   the letters that it calls faulty and the eye calls clean, and the
   letters that the eye calls faulty and no measure finds. Change a
   limit or a measure where they do not agree, with a note in this plan.

**Exit test:** the measures agree with the eye on 90 % of the set or
more; each letter where they do not agree is listed with the reason; a
change of the snapshot of the fault bench has a note.

## Milestone 7: overlapped strokes (trial) (L)

Today a stroke that meets another stroke at a joint ends at its edge (a
butt joint, ADR 0152, 0153 and 0159). The trial: the stroke that meets
the stem goes on under the stem to the middle of the joint and sews
before it; the stem sews from end to end. A hand digitizer often sews a
joint so. zigzag measured more clean letters with it (81 % against
74 %, with its own measures) and about one sixth more trims.

1. An option of the stroke finder for the trial
   (`stroke::Split`, `GlyphShapes::with_split`).
2. The measure: the fault bench and the judged set (Milestone 6) for the
   four fonts and the survey: clean letters, open area at joints, two
   layers, trims.
3. The owner chooses with the numbers and the proof pictures. If the
   trial wins (more clean letters, and not more than 10 % more trims),
   it is the way for Latin letters, the fonts are made again
   (`tools/make-embroidery-fonts.py`), and a sew-out kit is made. If not,
   its code goes to a branch (as P21 Milestone 7).

**Exit test:** the numbers of both ways are in this plan; the owner's
choice; with the choice, the snapshot of the fault bench and the fonts.

## Milestone 8: the open tabs come back, and one program (M)

1. **The tabs come back.** At a normal quit, the session of the backups
   (`backup.rs`) keeps the list of the open tabs: the file of each, its
   changes that are not saved (a backup project), the tab in front and
   the view. At the next start, the tabs open again with no question
   (Options; on by default). After a crash, the program asks as today
   (recovery).
2. **One program.** A file that the user opens while the program runs
   (a double-click, Open with, the command line) opens in a tab of that
   program. The new process finds a session whose heartbeat is new,
   writes the path of the file in the inbox folder of that session, and
   stops. The program that runs reads its inbox each second and comes to
   the front. This uses a folder: no network and no sockets (ADR 0109).
   `--new-window` starts a second program.

**Exit test:** a test with two processes: the second opens its file in
the first and stops in less than 2 s; a quit with three tabs (one with
changes that are not saved) gives the three tabs, the changes and the
same tab in front at the next start; after a crash, the recovery
question as today.

## Milestone 9: agents through MCP (L)

An MCP server in `emb-cli` (`emb-cli mcp`) on standard input and output
(JSON-RPC 2.0, the stdio transport of MCP). The AI agent of the user (an
MCP client) starts it. The program has no AI in it, and no network code
(ADR 0109). The assistant of P10 Milestone 9 stays benched: this is a
way for the user's own agent to use the program.

1. **Tools.** Each tool is a function that the program has: open and
   new design, the list of objects, add objects (a path as SVG path
   text, lettering), change the settings of objects, move, scale,
   rotate, delete, run a script (`emb-script`), the checks (with
   Milestone 1), a picture (PNG of the 3D look), export machine files
   and sheets, test swatches (Milestone 3), Quicker to Sew (Milestone 2,
   with a dry run that gives only the numbers), check and fix a machine
   file (Milestone 5), undo, redo and save. Each change is one
   `Document::edit` (one undo step); a refused change gives its reason
   (`Document::take_refused`).
2. **Checked arguments.** A tool refuses an argument that it does not
   take, and gives the names that it takes. Each tool says if it only
   reads.
3. **Show.** The tool `show` opens the design in the program that runs,
   through the inbox of Milestone 8. A design that is open in the
   program and that the agent saves gives a message in the program (the
   file changed: open it again?).
4. A help topic "Agents", with the setup line for an MCP client, in the
   four languages; a line in the README.

**Exit test:** a test runs a session through pipes (initialize, the
list of tools, a call of each tool) and makes a badge (a round fill,
lettering, a satin border), checks it, and writes a picture and a PES
file that reads back; an argument that a tool does not take is refused;
each change is one undo step; the owner's agent makes a design from one
sentence.

## Exit test of the phase

The exit tests of the milestones; the sew-outs of the swatches
(Milestones 3 and 4) and, if the fonts change, of Milestone 7; the
owner's test of Milestone 9 with an agent.

## Not in this phase

- **Send to the Brother Skitch PP1 over Bluetooth.** Its protocol is in
  open-source code (Respira, Apache-2.0), but the owner and the testers
  have no Skitch, and Bluetooth on Linux needs a C library (D-Bus). It
  stays in `docs/design/open-work.md` until a tester has one.
- Designs shared on a website, and accounts: these are network services
  (ADR 0109).
- An AI panel in the program: the assistant stays benched (P10
  Milestone 9).
- Stitch settings in the names of SVG layers: Ink/Stitch files do the
  job (ADR 0135, 0136).
- The drawing tools in the layout of a vector drawing program: the
  ribbon stays (ADR 0010, 0055).
- Updates that install themselves: the signing of the packages comes
  first (ADR 0062).
- Speed and size work: the owner shelved it for a later phase.
