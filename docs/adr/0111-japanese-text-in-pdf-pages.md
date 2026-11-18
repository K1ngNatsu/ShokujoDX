# ADR 0111: Japanese text in the PDF pages, and the place of Japanese characters in a line

Date: 2026-11-18 · Status: Accepted

## Context

The pictures of the windows (ADR 0110) showed two problems of the
Japanese language:

- In the print preview, lines of the worksheet were longer than their
  column. The cause was larger: the PDF writer (`emb_render::pdf`) knows
  only the Latin-1 characters of the standard Helvetica fonts, so each
  Japanese character of a worksheet, an approval sheet, a quote or a
  catalog printed as `?`. The labels of the sheets are in the language of
  the program since ADR 0083.
- On the screen, the Japanese characters were higher than the Latin
  letters and digits in the same line.

## Decision

- **A standard Japanese font in the PDF, not embedded.** A page with wide
  characters (Chinese, Japanese and Korean writing and the full-width
  forms, `page::wide`) gets a third font: `HeiseiKakuGo-W5` with the
  encoding `UniJIS-UCS2-H` (a CID font of the collection Adobe-Japan1).
  The PDF reader shows it with a Japanese font of the computer. A text is
  written in parts: the Latin parts in Helvetica, the wide parts in this
  font, one after the other. A document with no wide character is the
  same bytes as before.
- **Why not an embedded font.** The program has no Japanese font
  (ADR 0042), and to embed a part of a font of the computer needs a font
  subsetter. The standard font adds about 500 bytes to a document.
- **Widths and line breaks** (`page::text_width_mm`, `page::wrap`). A wide
  character is 1 em wide (a Latin character about 0.52 em). A line can
  break before and after a wide character, because Japanese has no
  spaces, but not before a mark that closes a phrase.
- **More Latin characters.** The euro sign and the typographic quotation
  marks print (they printed as `?`). The title of a document with wide
  characters is UTF-16.
- **The place of Japanese characters on the screen.** Each font of
  `icons::CJK_FONTS` has a correction: how far down its characters go, as
  a part of the text size (`FontTweak::y_offset_factor`). Yu Gothic (the
  Japanese font of Windows 8.1 and later) has 0.18, from the pictures of
  the windows. The fonts that were not checked have 0.

## Consequences

- A Japanese worksheet was checked with the PDF engine of Windows
  (`Windows.Data.Pdf`): the text shows and stays in its column. Other PDF
  readers were not checked; a reader with no Japanese font shows boxes.
- Japanese text in a PDF has one weight: a bold title is not bold.
- A character outside the Unicode basic plane prints as the mark 〓.
- Meiryo and MS Gothic (older Windows), Hiragino (macOS) and Noto Sans
  CJK (Linux) need their values from pictures on those systems.
