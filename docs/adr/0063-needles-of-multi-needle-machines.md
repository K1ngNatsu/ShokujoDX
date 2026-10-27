# ADR 0063: The needles of multi-needle machines

Date: 2026-10-26 · Status: Accepted (owner request)

## Context

A multi-needle machine has a thread on each needle (6 to 15 needles on
most heads). At each color change it goes to the needle of the next color.
Before this change the program did not know the needles:

- TBF and U01 files gave color 1 needle 1, color 2 needle 2, and so on to
  needle 5, then needle 1 again (pyembroidery's default, ADR 0014). A
  design that uses red two times put red on two needles, and a sixth color
  went on needle 1 over the first thread.
- DST, TAP and EXP files have no needle numbers. The operator types the
  needle of each color change on the machine, but the worksheet did not
  give the needles.
- The program did not know the threads that are on the machine now, so it
  could not keep them where they are from one design to the next.

## Decision

1. **Number of needles.** A machine profile can have `needles` (1 to 15)
   in `data/machines.toml`. The multi-needle profiles have 15
   (UNVERIFIED: models of one brand have 6 to 15); the Topaz 20 and the
   generic profile have none (one needle). The user can set the number of
   their machine (Design > Needles). 15 is the largest number: U01 has
   codes for needles 1 to 15 only, and it is the largest common head.
2. **The threads on the machine** (`emb_model::needles::NeedleSetup`):
   the number of needles and the thread on each needle, for each machine,
   in the program settings (not in the design: the machine keeps its
   threads from one design to the next). After a design, the user can keep
   the threads of the design as the new setup.
3. **Pins in the design** (`Design::needles`): the user can give a thread
   of the design a needle. A pin is a design change (`Document::edit`), so
   undo takes it back. A pin of a thread that the design does not sew does
   nothing.
4. **Assignment** (`emb_model::needles::assign`), in sew order, one needle
   for each color block. Two blocks have the same thread if they have the
   same thread id, the same brand and code, or (with no code) the same
   color. For each block:
   1. a pinned needle;
   2. else a needle that has the thread now (from the setup, or from an
      earlier block);
   3. else a needle that is not used yet and has no thread, then one that
      is not used yet and has a thread that the design does not need;
      needles that are pinned for other threads come last;
   4. else (more threads than needles) a used needle whose thread is not
      needed again, else the needle whose thread is needed again latest.
      The operator changes the thread on that needle: a **thread change**.

   The result gives the needle of each block, the threads to put on
   before the start, and the thread changes. The same input always gives
   the same result.
5. **Thread changes stop the machine.** TBF and U01 files get a stop after
   the needle set of a thread change, so the machine is on the needle when
   it stops. The worksheet tells the operator which thread goes on which
   needle. The sew time counts a thread change as a stop.
6. **Machine files.** TBF and U01 files write the needle of each block
   from the assignment (`emb_formats::write_machine_with` takes the
   assignment). DST, TAP and EXP have no needle codes: the worksheet gives
   the needle sequence to type on the machine. With no needles in the
   profile, TBF and U01 keep pyembroidery's needles (1 to 5, then 1 again)
   and no stops are added, so old files do not change.
7. **Open TBF and U01 files.** The needles of the file become pins of the
   threads of the design, so a file that is opened and written again has
   the same needles.
8. **The program.** The Needles window (Design > Machine and Hoop) shows
   the number of needles, each color block with its needle (Auto or a
   pin) and a mark at a thread change, and the threads to put on each
   needle before the start compared with the threads on the machine. The
   production worksheet has a needle column, the threads for each needle
   at the start, the needle sequence, and a note for each thread change.

## Consequences

- No test with a multi-needle machine yet: the needle codes follow
  pyembroidery (TBF `DO` header, U01 codes 9 to 0x17), and the stop after
  a needle set must be checked on a Tajima and a Barudan machine.
- A design for a machine with fewer needles than threads sews with stops;
  the warning is the thread-change count in the Needles window and on the
  worksheet.
- The settings file has a new map `needles` (machine id to setup); older
  settings files load with no setups.
