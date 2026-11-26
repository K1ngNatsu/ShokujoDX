# ADR 0164: The oval monogram

Date: 2026-11-24 · Status: Accepted (the owner: "Fix the oval monogram",
after ADR 0163 left the template "Oval in a double circle" in the list of
known places over the density limit).

## Context

After ADR 0163, the template "Oval in a double circle" still gave "Thread
builds up" warnings in Design Info: 8.4 layers with the initials ABC on
woven cotton, and 7 to 11 layers with JMK, ABC, SRW and EAN on towel
fabric. A probe of the places found two causes:

1. **The ends of the oval were vertical.** The oval envelope moved the
   top and the bottom edge of the text along a whole ellipse, and an
   ellipse is vertical at its ends. Near each end the edges moved a long
   distance over a short width, so the letter at each end had no limit to
   its shear: the foot of "A" and of "W" went almost along its column, and
   the stitches of the column fanned there (8 to 11 layers).
2. **The middle initial was larger than the height of the spec.** The
   oval style (amount 0.45) made the middle 1.45 times the height of the
   text, and the diamond style (amount 0.6) 1.6 times. A monogram of 30 mm
   was 43 mm high (48 mm for the diamond), but `MonogramSpec::height_mm`
   is the height of the largest initial, and the help says so. The joints
   of the letters had the same layers as in a smaller monogram, but on a
   larger area: the joint of the crossbar of a middle "A" on towel fabric
   went over 4 mm² (`density::MIN_SPOT_MM2`). The diamond style had the
   same fault ("Diamond with hearts", EAN on towel).

## Decision

1. **The oval envelope uses the middle of an ellipse.** The edges follow
   the middle `envelope::OVAL_ARC` (0.9) of the width of an ellipse, made
   as high as the whole ellipse: the edges move as far as before at the
   ends and in the middle, but their slope at the ends has a limit. This
   is for all lettering with the oval envelope, not only for monograms.
   The probe measured 0.8, 0.85 and 0.9; 0.9 is the nearest to an ellipse
   with no warning.
2. **The middle initial is the height of the spec.** The oval and the
   diamond style make the text smaller by the factor of their envelope
   (`1 + amount`, also the letter spacing), so the middle initial is
   `height_mm` high, as in the other styles (`monogram::fit_envelope`).

## Consequences

- All six monogram templates sew with no warning for the initials of the
  test (JMK, ABC and EAN on woven cotton; JMK, ABC and SRW on towel
  fabric), except "Block in a square" with JMK on towel fabric (one place
  of 4 mm², 5.8 layers; `docs/design/open-work.md`).
- Oval and diamond monograms of a given height are smaller than before
  (the height is now correct). The golden snapshot `monogram-templates`
  changes (1,598 fewer stitches, the longest stitch 3.71 mm, before 4.21
  mm); the commit message says why.
- Text with the oval envelope has ends that are a little less pointed.
- The limit of a place (4 mm²) does not change with the size of the
  letters, so a monogram of 60 mm has places over the limit in most styles
  on towel fabric, and the oval style also on cotton. The open-work list
  has this.
