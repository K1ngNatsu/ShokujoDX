# ADR 0115: Quality score of embroidery fonts, and font packages

Date: 2026-11-18 · Status: Accepted (ADR 0119 adds two measures to the score)

## Context

The program had 30 embroidery fonts, all converted from OFL fonts by the
stroke finder (ADR 0016, 0100). The user could not see how good a font is
before a sew-out, and some fonts were not good: letters with parts that
have no stitches, or thin strokes that are too narrow at the smallest size
of the font (the review of 2026-11-18; Phase P17 Milestone 7,
`docs/design/p17-plan.md`). The reference product has some hundred fonts;
30 is few.

## Decision

### The score

Each embroidery font has a **quality score** from 0 to 100
(`emb_letter::quality`). It comes from three measures of each letter. The
measured letters are the capital letters, the small letters and the digits
(62 characters). Marks are not measured: they have thin strokes in most
fonts (the arms of "*"), and a text has few of them.

- **Shape:** the part of the outline of the source font that the parts of
  the letter cover, on a grid of 0.1 mm at the design size. The shape
  value is 0 at 85 % (below that, the converter does not use the columns
  of the stroke finder) and 1 at 97 % (some cells at the edges are always
  open). A letter that sews as a fill has its full shape but not the look
  of the other letters: its shape value is 0.5.
- **Narrow:** the part of the length of the satin columns that is too
  narrow at the smallest size of the font. Too narrow is below two thirds
  of the narrowest column of the size range: 0.8 mm (0.53 mm for a font
  for small text). The thin strokes of a plain font (the bar of "e") are
  about three quarters of its stems, so they are not too narrow. A letter
  with half of its columns too narrow has the score 0.
- **Long:** the part of the length of the satin columns with stitches
  longer than 12 mm at the largest size (the longest stitch of the common
  machine formats is 12.1 mm). The stitch engine splits such stitches, so
  the letter sews, with another look. A letter loses its whole score only
  if all of its columns are too long.

The score of a letter is the product of the three values. The score of the
font is the mean of its letters, times 100.

The measures need the font that the embroidery font was converted from. So
the converter makes the score (`emb-cli font` prints it, the Fonts window
of the program makes it for a font that the user converts), and the font
file keeps it (`EmbFont::quality`: the score and the three means). An
alphabet made by hand, a bitmap font and a file of an earlier version have
no score. `build.rs` of `emb-letter` writes a table of the scores of the
bundled fonts, so the font lists show and sort them with no font read
(`bundled::quality`). `emb-cli font-quality <font> --source <ttf>` prints
the score and the letters with the lowest scores.

### The size range fits the columns

The size range of a converted font came from one number, the typical
stroke of the font (ADR 0016). A font with thin and thick strokes (a
script, a serif font with hairlines) then had thin strokes far below the
narrowest column at its smallest size, and thick strokes far above the
widest column at its largest size. The first measures showed this as the
largest fault of the fonts: Brush Hand, a tuned font, had 31 % of its
columns too narrow at 8 mm.

Now the converter makes the range smaller (`EmbFont::fit_size_range`)
until at most 10 % of the length of the columns is too narrow at the
smallest size, and at most 10 % is too long at the largest size. The range
never gets larger. Ten of the fonts of the program have a smaller range
now:

| Font | Range before, mm | Range now, mm |
|---|---|---|
| Bangers | 6 to 49 | 7.5 to 49 |
| Black Ops One | 5 to 30 | 6.5 to 30 |
| Bold Script | 9 to 72 | 13 to 62 |
| Brush Hand | 8 to 65 | 13 to 65 |
| Casual Script | 9.5 to 77 | 10 to 77 |
| Damion | 11 to 87 | 11 to 80 |
| Leckerli One | 5.5 to 42 | 6 to 41 |
| Pacifico | 9 to 72 | 9 to 70 |
| Retro Script | 7 to 57 | 9 to 47 |
| Slab Round | 6 to 49 | 6.5 to 49 |

The letters of these fonts did not change. Lettering below the new range
gets the warning of the design checks.

### The limit

A font of the program has a score of **65** or more
(`quality::LIMIT`). The limit comes from proof pictures
(`emb-cli font-audit --proof`) of fonts with scores from 47 to 73: below
about 65, some letters have gaps that the eye sees (a "G" with no bar, an
"S" with an open curve); from about 65, the faults are small. The test
`each_bundled_font_has_a_score_above_the_limit` is the exit test of the
milestone.

Four fonts of earlier versions are below the limit: Slab Heavy (44), Fat
Face Serif (50), Great Vibes (56) and Shrikhand (59). They are not in the
program now (`bundled::EARLIER`). Lettering in a design keeps its letters
when its font is not there (ADR 0015), and Object Properties says where
the font is: the font package "Fonts of earlier versions". This package is
the one place for a font below the limit.

### More fonts

111 more OFL fonts were converted and measured, from the google/fonts
repository at the commit of `tools/make-embroidery-fonts.py`. They were
chosen for even strokes: sans, slab, display, script and hand fonts. With
the 30 fonts of the program, 141 fonts were measured.

- **97** of the 111 have a score of 65 or more.
- **In the program:** the 29 new fonts with a score of **85** or more
  (`quality::BUNDLE`). The program has 55 embroidery fonts now: the 26 of
  earlier versions that pass the limit, and these 29.
- **In font packages:** the 68 new fonts with a score from 65 to 84, in
  five packages by kind: sans (23), display (19), serif and slab (6),
  script (12) and hand (8).
- **Not included:** 14 fonts with a score below 65.

A font whose license has a Reserved Font Name has a new name (52 of the 97
fonts), as before.

### Font packages

A font package is an asset package (My assets, P10 Milestone 6): a zip
file with the fonts in the folder `fonts`, the license text of each font
next to it, and a `README.txt`. The user adds it with **File > My Assets >
Import a package**; the import takes the font files only.

`tools/make-embroidery-fonts.py` makes the packages
(`data/font-packages/shokujodx-fonts-<name>.zip`, 9.5 MB together; the
same fonts give the same bytes). They are in the repository, so a release
needs no font sources and no network. The release workflow puts them in
the release as files of their own. **Lettering > Fonts > More Fonts** opens
the page of the releases in the browser of the system. The program
downloads nothing itself (ADR 0109: the update check is its one use of the
network).

### The Fonts window

The font list shows the score after the name of each embroidery font.
**Sort by > Quality** puts the best fonts first. The details of a font
show the score, one sentence about what it says, and the three measures.

## Consequences

- The Windows program is 5.8 MB larger (69.2 MB; 63.4 MB before): 25 more
  fonts. The exit test allows 10 MB.
- The corpus designs of the fonts changed: `embroidery-fonts-2`, `-3` and
  `-4` have one or two fonts less, and `embroidery-fonts-5`, `-6` and `-7`
  show the new fonts. No other golden snapshot changed.
- The score is a measure of pictures, not of thread. No font was sewn for
  this decision. The limit of 65 comes from a look at proof pictures.
- The new fonts are not tuned (ADR 0113): the stitch engine finds the
  order of the parts of their letters, and they have more trims than the
  ten tuned fonts. The score does not measure trims.
- The score needs the source font. An alphabet made by hand and an
  embroidery font of an earlier version have no score; the Fonts window
  says so. A TrueType font has no score.
- The marks and the letters with accents are not in the score.
- The step of the release workflow that copies the packages has not run
  yet (the next tag runs it).
- A user of an earlier version who used one of the four fonts must add a
  package to change the text of that lettering.
