//! A printed page as a list of drawing steps in mm (origin at the top left,
//! Y down). The PDF writer (`pdf`) and the print preview of the app draw
//! the same list, so the preview shows what the file has.

use emb_geom::{BBox, Point};
use emb_model::Rgb;

use crate::Segment;

/// Paper sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Paper {
    #[default]
    Letter,
    A4,
}

impl Paper {
    pub const ALL: [Paper; 2] = [Paper::Letter, Paper::A4];

    /// Width and height in mm (portrait).
    pub fn size_mm(self) -> (f64, f64) {
        match self {
            Paper::Letter => (215.9, 279.4),
            Paper::A4 => (210.0, 297.0),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Paper::Letter => "Letter (8.5 × 11 in)",
            Paper::A4 => "A4 (210 × 297 mm)",
        }
    }
}

/// One drawing step.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// Text with its baseline starting at `at`. `size_pt` is in points.
    Text {
        at: Point,
        size_pt: f64,
        bold: bool,
        color: Rgb,
        text: String,
    },
    Line {
        a: Point,
        b: Point,
        width_mm: f64,
        color: Rgb,
    },
    Rect {
        rect: BBox,
        fill: Option<Rgb>,
        stroke: Option<(f64, Rgb)>,
    },
    /// Stitches in page mm, drawn as lines of `width_mm` with round ends.
    Stitches { segments: Vec<Segment>, width_mm: f64 },
    /// A filled area (appliqué fabric): rings in page mm, filled by the
    /// even-odd rule, so the inner rings are holes.
    Area { rings: Vec<Vec<Point>>, fill: Rgb },
    /// A picture (the design on the product) that fills `rect` (page mm).
    /// Transparent pixels print white.
    Image {
        rect: BBox,
        picture: std::sync::Arc<crate::raster::Picture>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub width_mm: f64,
    pub height_mm: f64,
    pub ops: Vec<Op>,
}

impl Page {
    pub fn new(paper: Paper) -> Page {
        let (w, h) = paper.size_mm();
        Page {
            width_mm: w,
            height_mm: h,
            ops: Vec::new(),
        }
    }

    pub fn text(&mut self, x: f64, y: f64, size_pt: f64, bold: bool, text: impl Into<String>) {
        self.ops.push(Op::Text {
            at: Point::new(x, y),
            size_pt,
            bold,
            color: Rgb(0, 0, 0),
            text: text.into(),
        });
    }

    pub fn gray_text(&mut self, x: f64, y: f64, size_pt: f64, text: impl Into<String>) {
        self.ops.push(Op::Text {
            at: Point::new(x, y),
            size_pt,
            bold: false,
            color: Rgb(100, 100, 100),
            text: text.into(),
        });
    }

    pub fn line(&mut self, a: Point, b: Point, width_mm: f64, color: Rgb) {
        self.ops.push(Op::Line { a, b, width_mm, color });
    }
}

/// True for a character of Chinese, Japanese or Korean writing, and for
/// the full-width forms: it is as wide as it is high, and a line can
/// break before or after it. The PDF shows it in its own font.
pub fn wide(c: char) -> bool {
    matches!(
        c as u32,
        0x1100..=0x115F
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
    )
}

/// The width of text in characters of Latin text: a wide character
/// counts as two.
fn units(text: &str) -> usize {
    text.chars().map(|c| if wide(c) { 2 } else { 1 }).sum()
}

/// Width of text in Helvetica, in mm, near enough for layout (the mean
/// width of a character is about 0.52 em; a wide character is 1 em).
pub fn text_width_mm(text: &str, size_pt: f64) -> f64 {
    units(text) as f64 * size_pt * 0.52 * 25.4 / 72.0
}

/// How design mm map to page mm: a scale about a center.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    pub scale: f64,
    from: Point,
    to: Point,
}

impl Fit {
    /// The fit that puts the box `b` in `area`: as large as fits, the
    /// aspect ratio kept, centered.
    pub fn of(b: BBox, area: BBox) -> Fit {
        Fit {
            scale: (area.width() / b.width().max(1e-6)).min(area.height() / b.height().max(1e-6)),
            from: b.center(),
            to: area.center(),
        }
    }

    pub fn apply(&self, p: Point) -> Point {
        Point::new(
            self.to.x + (p.x - self.from.x) * self.scale,
            self.to.y + (p.y - self.from.y) * self.scale,
        )
    }
}

/// Places stitches (design mm) in `area` (page mm): as large as fits, the
/// aspect ratio kept, centered. Returns the segments and the fit.
pub fn fit_segments(segments: &[Segment], area: BBox) -> (Vec<Segment>, Fit) {
    let stitches: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.kind == crate::SegmentKind::Stitch)
        .collect();
    let Some(b) = BBox::from_points(stitches.iter().flat_map(|s| [s.a, s.b])) else {
        let c = area.center();
        return (
            Vec::new(),
            Fit {
                scale: 1.0,
                from: c,
                to: c,
            },
        );
    };
    let fit = Fit::of(b, area);
    let out = stitches
        .into_iter()
        .map(|s| Segment {
            a: fit.apply(s.a),
            b: fit.apply(s.b),
            ..*s
        })
        .collect();
    (out, fit)
}

/// The parts of a word that a line can break between: text with no wide
/// characters is one part, and a line can break before and after each
/// wide character (Japanese has no spaces), but not before a mark that
/// closes a phrase.
fn parts(word: &str) -> Vec<&str> {
    const NO_START: &str = "、。，．）」』】！？：；ー…,.):;!?%";
    let mut out = Vec::new();
    let mut start = 0;
    let mut before: Option<char> = None;
    for (i, c) in word.char_indices() {
        if let Some(b) = before
            && (wide(c) || wide(b))
            && !NO_START.contains(c)
        {
            out.push(&word[start..i]);
            start = i;
        }
        before = Some(c);
    }
    out.push(&word[start..]);
    out
}

/// Cuts text into lines of at most `max` characters, at spaces. A wide
/// character counts as two, and a line can also break at it.
pub fn wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.lines() {
        let mut line = String::new();
        let mut width = 0;
        for word in para.split_whitespace() {
            for (k, part) in parts(word).into_iter().enumerate() {
                // A space comes before a new word, not inside a word.
                let gap = usize::from(k == 0 && !line.is_empty());
                if !line.is_empty() && width + gap + units(part) > max {
                    lines.push(std::mem::take(&mut line));
                    width = 0;
                } else if gap == 1 {
                    line.push(' ');
                    width += 1;
                }
                line.push_str(part);
                width += units(part);
            }
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SegmentKind;

    #[test]
    fn fit_keeps_the_aspect_ratio() {
        let seg = Segment {
            a: Point::new(0.0, 0.0),
            b: Point::new(20.0, 10.0),
            color: Rgb(0, 0, 0),
            kind: SegmentKind::Stitch,
        };
        let (out, fit) = fit_segments(&[seg], BBox::new(Point::new(10.0, 10.0), Point::new(110.0, 110.0)));
        assert!((fit.scale - 5.0).abs() < 1e-9);
        assert_eq!(fit.apply(Point::new(10.0, 5.0)), Point::new(60.0, 60.0));
        assert_eq!(out[0].a, Point::new(10.0, 35.0));
        assert_eq!(out[0].b, Point::new(110.0, 85.0));
    }

    #[test]
    fn wrapping() {
        assert_eq!(wrap("one two three four", 9), ["one two", "three", "four"]);
        assert_eq!(wrap("a\n\nb", 9), ["a", "", "b"]);
        // Japanese has no spaces: a line breaks at a character, a wide
        // character counts as two, and no line starts with 。 or 、.
        assert_eq!(
            wrap("枠の中心から右へ30 mmです。", 12),
            ["枠の中心から", "右へ30 mmで", "す。"]
        );
        assert_eq!(wrap("あいう、えお", 8), ["あいう、", "えお"]);
        assert!((text_width_mm("あ", 10.0) - text_width_mm("ab", 10.0)).abs() < 1e-9);
        assert!(text_width_mm("abc", 10.0) > 5.0);
    }
}
