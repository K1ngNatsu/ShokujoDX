# ADR 0152: Split the outline of a letter into strokes at its concave corners

Date: 2026-11-23 · Status: Accepted (P21 Milestone 8,
`docs/design/p21-plan.md`).

## Context

The stroke finder finds the strokes of a letter from the middle line of
the whole letter: at each junction it links the edges that continue most
straight, cuts the shorter of two strokes that cross, and holds and cuts
back the ends that meet another stroke. Most faults at joints came from
these rules (P21 Milestones 1 to 3), and the trial of a Voronoi middle
line (ADR 0151) failed at them. Research on Chinese characters (Make Me
a Hanzi) splits the outline first, at bridges between concave corners,
so each piece is one stroke. The owner asked for a trial of it
(Milestone 8) and, after it, chose it for Latin letters and for kana and
kanji.

## Decision

1. `emb_letter::split` splits the outline of a glyph before the stroke
   finder:
   - concave corners: the outline turns into the shape 40 degrees or more
     over 2 % of the capital height;
   - bridges between two concave corners: at most 1.3 times the typical
     width of the strokes (2 times the area over the outline) long, in
     the shape and clear of the outline, at most 35 degrees off the
     outline that they continue at each end, with the sides of the stroke
     that ends at least 35 degrees off them; the bridges with the least
     cost (length and angles) win, each corner in one bridge at most, so
     where strokes cross the thinner stroke is cut;
   - a bridge that does not cut the shape (it joins the outline to a
     hole) goes;
   - with `Split::Notches`, a sharp concave corner with no bridge (the
     outline turns 110 degrees or more) gets a cut along the line that
     halves it, at most 2 widths long;
   - the pieces are the faces of the outline and the links; the end of a
     piece at a bridge or a cut goes 0.1 of its length into the next
     piece.
2. The stroke finder runs on each piece (`stroke::strokes_split`); the
   columns and the coverage of the pieces are one result.
3. `stroke::Split::for_char` chooses the split by the character of the
   glyph: Latin letters and digits get bridges and cuts
   (`Split::Notches`), kana and kanji get bridges (`Split::Bridges`: the
   cuts gave more areas with two layers there), other characters are not
   split (not measured). `Font` keeps the character of each glyph (the
   lowest character that its Unicode tables map to it); a glyph that
   only shaping gives (a ligature) is not split. Shapes made with no font
   (bitmap fonts, pictures) are not split.
4. The method is from the description of Make Me a Hanzi; its code has
   no license, and no code is copied. Its stroke data is not used.

## Consequences

- The letter fault bench (lettering in the source fonts at 25 mm), before
  and after: spikes 17 to 2, 44 to 23, 37 to 25 and 15 to 0 (Block Sans,
  Anton, Pacifico, Varsity); crossing pairs 27 to 15, 40 to 24, 40 to 22
  and 11 to 6; open area 0.57 % to 0.22 %, 1.67 % to 1.33 %, 0.56 % to
  0.45 % and 0.49 % to 0.04 %; open corners 43 to 27, 54 to 45 and 33 to
  1; bulges the same or less.
- The lettering bench of P20: 5 to 7 characters over 3 % with two layers
  (18 to 19 before), each character 96.9 % covered or more at 15 mm and
  more (96.0 % before). The eight
  words have 1, 1 and 2 more trims at 10, 15 and 25 mm (49, 50 and 56):
  P21 Milestone 5 takes them back.
- Travel runs out of the outline in Block Sans: 0.4 mm to 2.3 mm (Milestone
  5).
- The bench takes about 1.4 times as long for Latin letters; the
  lettering bench is a little quicker.
- The converted letters have other parts, so the corrections of the
  bundled fonts (ADR 0112) must be checked when the fonts are made again
  (Milestone 6). The bundled fonts do not change until then.
- A new script gets the split only after a measure with the split.
