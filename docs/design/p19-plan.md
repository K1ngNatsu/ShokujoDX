# Plan: help and tutorials (phase P19)

Date: 2026-11-18 · Status: Proposed. The owner wants much better
tutorials and help (2026-11-18); the owner selects the milestones and
their order before the work starts.

## Goal

The reference product has a manual of more than a thousand pages, an
online academy and many videos. The program has 39 help topics (seven of
them tutorials) as text with no pictures, a search that answers from the
text, F1 on each button and window, and Find a Command (Ctrl+K).

More text of the same kind does not close this gap. The program can do
what a manual and a video cannot do: it knows the design of the user, it
can show the real button, and it can check that a step is done. This plan
makes the help a part of the work.

Nine milestones are proposed. Milestones 1 to 4 change the most for a new
user.

What stays:

- The help works with no network. No account, and no data of the user
  leaves the computer.
- The topics are Markdown files in Simplified Technical English, with
  translations of the same blocks (ADR 0084).
- No names of closed-source programs (the glossary gives the other common
  names of a tool, not the names of products).
- A new user starts with a clean slate (ADR 0080): samples open from the
  help and are not put in the folders of the user.

## Milestone 1: pictures in the help (M)

1. A help topic can show pictures (a new block of the help Markdown).
2. The pictures are made from the program itself by the test of P17
   Milestone 2: a small script for each picture opens a window or a
   design and cuts the part to show. So the pictures are made again when
   the program changes, and each language has its own pictures.
3. A size limit for the pictures of the program; large ones go in a help
   package.

**Exit test:** each topic of the start group and each tutorial has
pictures in four languages; a changed button changes its pictures with no
work by hand.

## Milestone 2: Show Me (S)

1. A command name in a help topic is a link. A click opens the ribbon tab
   of the command and marks its button for a few seconds.
2. An answer of the help search and a row of Find a Command have **Show
   Me** too.
3. When the command cannot run now, the help says why (the text that the
   dimmed button has).

**Exit test (a new user):** the user finds five commands from the help
with no search by hand on the ribbon.

## Milestone 3: guided lessons (L)

A tutorial that the user does in the program, step by step.

1. A lesson is a file: a start design, and steps. A step has a text, the
   button or the place on the canvas to mark, and a check of the design
   (for example: the design has a satin column). The check is a small
   script with no access to files (the script engine of ADR 0067).
2. A lesson panel shows the step, marks the target, and goes to the next
   step when the check passes. The user can go back, skip a step, or
   stop; the program keeps the place.
3. The seven tutorials become lessons. New lessons: the first ten
   minutes, a logo from a picture, a name on a cap, a file from a
   customer (P18), a multi-needle order.
4. Users and shops can write lessons of their own (a lesson is an asset).

**Exit test (new users):** three new users complete "the first ten
minutes" and the patch lesson with no other help.

## Milestone 4: the sew-out doctor (M)

Help for the problem that the user has on the machine.

1. **Help > Sew-out Problem** asks what the user sees: puckered fabric,
   a gap between a fill and its border, fabric that shows through, loops,
   thread breaks, small text that cannot be read, outlines that are not
   in their place.
2. The program checks the open design for the usual causes (the density
   map, the pull compensation, the underlay, the stitch lengths, the
   fabric setting, the sew order), shows the objects that have them, and
   offers a correction for each (one undo step), as the checks of the
   Picture to design wizard do.
3. It also gives the causes that are not in the design (stabilizer,
   needle, thread tension), from the help text.

**Exit test (owner and testers):** for three real sew-out problems, the
doctor finds the cause and the corrected design sews better.

## Milestone 5: samples and Try It (S)

1. **Help > Samples** opens small sample designs from the program (not
   from a folder of the user): one for each object kind and task.
2. A help topic of a stitch type or an effect has **Try It**, which opens
   its sample with the object selected and its settings open.

**Exit test:** each stitch type topic has a sample that opens.

## Milestone 6: ways to start (M)

1. The start page asks one time what the user wants to do: a home
   machine, a shop, or "I know another embroidery program". Each way has
   its lessons and its first topics.
2. For users of other programs: the glossary and Find a Command know the
   other common names of each tool, so a search with a known word finds
   our command.
3. **What is new** after an update (P17 Milestone 1) links to the topics
   and lessons of the new features.

**Exit test (testers):** a professional tester finds ten tools by the
names that the tester knows.

## Milestone 7: the help on the web and as a manual (M)

1. The same Markdown files make a website (GitHub Pages) with the
   pictures and a search, in each language. A search engine finds the
   program by its features.
2. The same files make a manual (PDF) for print.
3. The program has a link to the web page of each topic, to share it.

**Exit test:** the website and the manual are made by one command in CI;
they have each topic of the program.

## Milestone 8: moving pictures (M)

1. The test of Milestone 1 can run the steps of a lesson and write the
   frames as a short animation (the GIF writer of ADR 0108), so a topic
   can show a tool in use.
2. The owner can record videos with a voice from the same lessons; the
   start page and the website link to them.

**Exit test:** each lesson has an animation in the help on the web.

## Milestone 9: better answers (S)

1. The help search gets the words that testers use in their questions
   (a list of other words for each topic).
2. A question with no answer is kept in a list on the computer; the user
   can send the list with a report (the user decides; the program sends
   nothing).

**Exit test:** twenty questions of testers each give a useful first
answer.
