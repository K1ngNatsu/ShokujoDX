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

/// Width of text in Helvetica, in mm, near enough for layout (the mean
/// width of a character is about 0.52 em).
pub fn text_width_mm(text: &str, size_pt: f64) -> f64 {
    text.chars().count() as f64 * size_pt * 0.52 * 25.4 / 72.0
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

/// Cuts text into lines of at most `max` characters, at spaces.
pub fn wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.lines() {
        let mut line = String::new();
        for word in para.split_whitespace() {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > max {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
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
        assert!(text_width_mm("abc", 10.0) > 5.0);
    }
}
