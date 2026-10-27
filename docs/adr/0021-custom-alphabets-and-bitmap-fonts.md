# ADR 0021: Custom alphabets and bitmap fonts

Date: 2026-10-22 · Status: Accepted

## Context

P4 Milestone 7 adds custom lettering: letters made from objects that the
user draws, an alphabet creator, and the management of alphabets. The
owner also asked for bitmap fonts: BDF (`.bdf`) and Windows raster fonts
(`.fon`). Both formats have public specifications.

## Decision

- **An alphabet is an embroidery font** (`.embfont`, ADR 0016) in the
  user's `fonts` folder. The font lists show it at once; lettering uses it
  as any embroidery font.
- **A letter from objects.** The selected satin, fill and running objects
  (or the parts of a lettering object) become a letter. The alphabet
  creator sets a baseline and a capital height and can put guides there;
  the letter starts at the left of the objects, and its advance is its
  width plus a small side space. The letter is stored at the design size
  of embroidery fonts.
- **Manage alphabets** in one window: new, rename (the file follows the
  name), delete (two clicks), import, export, delete a letter, and letter
  with the alphabet.
- **Bitmap fonts become embroidery fonts.** BDF is read as text; `.fon`
  files are New Executable files with FNT resources (versions 2 and 3,
  raster only). The largest size is used. The outline of the pixels of
  each glyph (pixels that touch only at a corner stay apart) goes to the
  glyph shapes of TrueType letters (ADR 0015), so a letter is satin (the
  stroke method smooths the steps) or fill (the pixel look). One pixel is
  the stroke width for the size range.
- **Brother `.bfn` stays out:** there is no public specification
  (clean-room rule).

## Consequences

- Alphabets made by hand have no kerning pairs yet.
- Bitmap fonts with few pixels give large minimum sizes (for example
  26 mm for MS Sans Serif); the size warning says so.
- `emb-cli font` also converts `.bdf` and `.fon` files.
