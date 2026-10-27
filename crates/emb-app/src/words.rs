//! Plain-language words for stitch settings (spec UX rule 2). The app
//! shows each value in mm and as a word, for example "0.40 mm (normal)",
//! and each setting has a short help text.

use emb_model::{ColumnPattern, FillPattern, Underlay, UnderlayWeight};

use crate::i18n::tr;

/// A word for a stitch spacing (satin or fill density).
pub fn density(spacing_mm: f64) -> &'static str {
    match spacing_mm {
        s if s < 0.3 => tr("dense"),
        s if s <= 0.5 => tr("normal"),
        s if s <= 0.8 => tr("light"),
        _ => tr("very light"),
    }
}

/// A word for a stitch length.
pub fn length(len_mm: f64) -> &'static str {
    match len_mm {
        l if l < 2.0 => tr("short"),
        l if l <= 4.0 => tr("normal"),
        _ => tr("long"),
    }
}

/// A word for a pull compensation.
pub fn pull(mm: f64) -> &'static str {
    match mm {
        v if v < 0.01 => tr("none"),
        v if v < 0.15 => tr("a little"),
        v if v <= 0.3 => tr("normal"),
        _ => tr("strong"),
    }
}

/// A word for the auto split length of a satin column.
pub fn split(mm: f64) -> &'static str {
    match mm {
        v if v <= 4.0 => tr("short, a turning fill"),
        v if v <= 8.0 => tr("normal"),
        _ => tr("long stitches"),
    }
}

/// A word for a fill angle. 0° is along +X; positive turns clockwise on screen.
pub fn angle(deg: f64) -> &'static str {
    let a = deg.rem_euclid(180.0);
    match a {
        a if !(10.0..=170.0).contains(&a) => tr("horizontal"),
        a if (80.0..=100.0).contains(&a) => tr("vertical"),
        a if a < 90.0 => tr("diagonal, down to the right"),
        _ => tr("diagonal, up to the right"),
    }
}

/// The name of an underlay setting.
pub fn underlay(u: &Underlay) -> &'static str {
    match u {
        Underlay::Auto => tr("Normal"),
        Underlay::Light => tr("Light"),
        Underlay::Heavy => tr("Heavy"),
        Underlay::Double => tr("Double"),
        Underlay::Lace => tr("Lace"),
        Underlay::None => tr("None"),
        Underlay::Custom(_) => tr("Custom"),
    }
}

/// The help text of an underlay setting.
pub fn underlay_help(u: &Underlay) -> &'static str {
    match u {
        Underlay::Auto => tr("The app selects the underlay from the size of the object."),
        Underlay::Light => tr("Less underlay, for thin or delicate fabric."),
        Underlay::Heavy => tr("More underlay, for soft, stretchy or high-pile fabric."),
        Underlay::Double => tr("Two layers that cross: two zigzags on columns, two tatami layers on fills."),
        Underlay::Lace => tr(
            "For free-standing lace: a run along the edge two times, so its threads lock, then a zigzag or rows across it.",
        ),
        Underlay::None => tr("No underlay. Use it for small text and thin details."),
        Underlay::Custom(_) => tr("Underlay layers set one by one."),
    }
}

pub fn underlay_weight(w: UnderlayWeight) -> &'static str {
    underlay(&w.underlay())
}

/// A word for the width of a line with a border.
pub fn width(mm: f64) -> &'static str {
    match mm {
        w if w < 1.5 => tr("narrow"),
        w if w <= 5.0 => tr("normal"),
        _ => tr("wide"),
    }
}

/// A word for the spacing of an open column type (zigzag, E stitch, square, coil).
pub fn open_spacing(mm: f64) -> &'static str {
    match mm {
        s if s < 1.0 => tr("close"),
        s if s <= 2.5 => tr("normal"),
        _ => tr("open"),
    }
}

/// The help text of a column type.
pub fn column_help(p: &ColumnPattern) -> &'static str {
    match p {
        ColumnPattern::Satin => tr("Smooth stitches from side to side. For borders and lettering."),
        ColumnPattern::Zigzag => tr("Open satin: stitches from side to side with space between them."),
        ColumnPattern::EStitch => {
            tr("Blanket stitch: a line on one side, with stitches across like the teeth of a comb.")
        }
        ColumnPattern::Raised { .. } => {
            tr("Satin over more satin layers, so the column stands up from the fabric. Best up to 7 mm wide.")
        }
        ColumnPattern::Tatami { .. } => tr("Rows of short stitches across the column, for wide borders."),
        ColumnPattern::Square => tr("Square teeth from side to side, an open border."),
        ColumnPattern::Coil => tr("Loops across the column, like a spring seen from above. Avoid sharp corners."),
        ColumnPattern::Contour { .. } => tr("Lines along the column that follow its shape."),
        ColumnPattern::String { .. } => tr("Slanted stitches that overlap like twisted string."),
        ColumnPattern::Motif { .. } => tr("A motif repeated along the column, as high as the column is wide."),
        ColumnPattern::ProgramSplit { .. } => {
            tr("Satin rows with the needle points in a pattern that bends with the column (flexi split).")
        }
    }
}

/// The help text of a fill type.
pub fn fill_help(p: &FillPattern) -> &'static str {
    match p {
        FillPattern::Tatami => tr("Straight rows of stitches at the angle, in a pattern that does not show lines."),
        FillPattern::Columns { column } => match column {
            ColumnPattern::Satin => tr("Smooth stitches across the shape at the angle, from one side to the other."),
            ColumnPattern::Raised { .. } => tr("Satin across the shape over more satin layers, so it stands up."),
            _ => column_help(column),
        },
        FillPattern::Square => tr("Straight rows with the needle points in line: a grid of even stitches."),
        FillPattern::DoubleSquare { .. } => tr("Two layers of square stitching that cross, like a woven grid."),
        FillPattern::String => tr("Straight lines from edge to edge that overlap in curves, like string art."),
        FillPattern::CrossStitch { .. } => {
            tr("Crosses on a grid, as in hand cross stitch. The crosses of all objects line up.")
        }
        FillPattern::Stipple { .. } => tr("A meandering line that fills the shape and never crosses itself."),
        FillPattern::ProgramSplit { .. } => tr("Tatami rows with the needle points in a pattern that makes a picture."),
        FillPattern::FlexiSplit { .. } => tr("A program-split pattern across the shape that bends with its sides."),
        FillPattern::UserSplit { .. } => tr(
            "Rows split where they cross lines that you draw. Select the fill and lines, then Edit > Stitch Types > Split Lines.",
        ),
        FillPattern::Motif { .. } => {
            tr("Rows of a motif. A gradient changes the size from row to row; the ball bends the rows.")
        }
        FillPattern::Laydown { .. } => tr(
            "A light fill under the design that holds down the pile of towels and fur. Use a thread of the fabric color.",
        ),
        FillPattern::Contour => tr("Lines that follow the shape from one end to the other, along the angle."),
        FillPattern::Offset => tr("Rings inside the edge, from the edge to the middle."),
        FillPattern::Spiral => tr("One line that turns from the edge to the middle."),
        FillPattern::IslandCoil => tr("Rings of loops from the edge to the middle, like hand chenille work."),
        FillPattern::Radial { .. } => tr("Lines from a center point out to the edge, like the rays of a sun."),
        FillPattern::Maze => tr("An open line that goes round to the middle and back out between its rows."),
        FillPattern::Florentine { .. } => {
            tr("Rows that bend like a guide line. Select the fill and a line, then Edit > Stitch Types > Guide Line.")
        }
        FillPattern::Tartan { .. } => tr(
            "Stripes of color in two directions, as on woven tartan cloth. Make a tartan of many colors in Edit > Stitch Types > Tartan.",
        ),
        FillPattern::Gradient { .. } => tr(
            "Rows that change from one color to the next across the shape. Make a gradient of many colors in Edit > Stitch Types > Gradient.",
        ),
        FillPattern::Meander { .. } => {
            tr("One line along a pattern of tiles that fills the shape and never crosses itself.")
        }
        FillPattern::Circular { .. } => tr("Rings round a center point, sewn as a spiral."),
        FillPattern::Mesh { .. } => tr(
            "The net of free-standing lace: open rows in two directions, each sewn there and back, so the threads lock where they cross.",
        ),
        FillPattern::Guided { .. } => tr(
            "Rows that follow one or more guide lines and change from one to the next. Select the fill and lines, then Edit > Stitch Types > Guide Lines.",
        ),
    }
}

pub const PUFF_HELP: &str = "The column sews over foam, so it stands up: dense satin, no underlay rows across it, stitches over each end that cut the foam, and a stop before it to put the foam on.";
pub const TILE_HELP: &str =
    "The pattern of tiles that the line goes along. The size is the length of one side of a tile.";
pub const RIPPLE_HELP: &str = "Copies of the line that get smaller round a point, or move along a guide line.";

pub const CHORD_HELP: &str = "How far along the column each slanted stitch goes.";
pub const MOTIF_HELP: &str = "Choose another motif, or make one, in the Motifs window (Edit > Stitch Types > Motifs).";
pub const SPLIT_PATTERN_HELP: &str =
    "Where the needle goes in along the rows: the pattern that the needle points make.";
pub const SPLIT_LINES_HELP: &str = "The rows are split where they cross these lines. To add lines, select the fill and lines, then Edit > Stitch Types > Split Lines.";
pub const STAMP_HELP: &str =
    "Shapes pressed into the fill: the rows get needle points along their lines, so they show like a carving.";
pub const HAND_HELP: &str = "Needle points move a little at random, so the stitches look sewn by hand. 0: off.";
pub const CORDING_HELP: &str = "A line, then a stop to lay a cord on it, then an open zigzag that holds the cord down.";
pub const CORNER_HELP: &str = "Mitre: the outer edge comes to a point. Cap: the outer edge is cut off straight. Lap: the column stops and a new one starts over the corner.";
pub const AUTO_JUMP_HELP: &str = "No extra needle points in long stitches. Stitches longer than the machine allows go as jumps, so the thread lies long.";
pub const FRACTIONAL_HELP: &str =
    "Where across the column the spacing is measured on curves: 0 at rail A, 0.5 in the middle, 1 at rail B.";
pub const AUTO_SPACING_HELP: &str = "Narrow parts of the column get more open spacing, wide parts denser spacing.";
pub const ACCORDION_HELP: &str = "The spacing changes from start to end, for shading. Custom: set the spacing (× the set spacing) at points along the object.";
pub const JAGGED_HELP: &str = "The edge stitches end at random distances, for fur or grass. 0: off.";
pub const PULL_PERCENT_HELP: &str = "More pull compensation for wider columns: a percent of the width on each side.";
pub const PULL_SIDES_HELP: &str = "Rail B gets its own pull compensation (the set value is for rail A).";
pub const TRAPUNTO_HELP: &str = "Travel goes along the edge, not across the inside, so the inside can be padded.";
pub const BLEND_HELP: &str =
    "A blend sews the shape in two or more colors. Each layer goes from dense to open (or back), so the colors mix.";
pub const FILL_HELP: &str = "How the stitches fill the shape.";
pub const GUIDE_HELP: &str =
    "The rows bend like the guide line. To set it, select the fill and a line, then Edit > Stitch Types > Guide Line.";
pub const CENTER_HELP: &str = "Where the lines meet, measured from the middle of the shape (right, down).";
pub const LIQUID_HELP: &str =
    "The stitch angle changes evenly along the column, also where the stitch angle lines turn it sharply.";
pub const WIDTH_HELP: &str = "The width of the column that the line sews as. The line is its middle.";
pub const OUTLINE_HELP: &str =
    "A plain line sews as a running stitch. The other types sew a column of a set width along the line.";
pub const COLUMN_HELP: &str = "How the stitches fill the column.";
pub const SPACING_HELP: &str = "The distance between the repeats of the pattern along the column.";
pub const DENSITY_HELP: &str = "The distance between stitches. A smaller value gives more stitches and a denser cover.";
pub const LENGTH_HELP: &str =
    "The length of each stitch. Short stitches follow curves better; long stitches are faster.";
pub const PULL_HELP: &str = "Extra width, because the stitches pull the fabric in. Stretchy fabric needs more.";
pub const SPLIT_HELP: &str = "Satin stitches longer than this get extra needle points, so that they do not snag.";
pub const ANGLE_HELP: &str = "The direction of the stitch rows.";
pub const SHORT_HELP: &str =
    "Shorter stitches on the inside of tight curves, so that the curve does not get too dense.";
pub const UNDERLAY_HELP: &str = "Stitches under the top stitches. They hold the fabric flat and lift the top stitches.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words() {
        assert_eq!(density(0.4), "normal");
        assert_eq!(density(0.25), "dense");
        assert_eq!(length(5.0), "long");
        assert_eq!(pull(0.0), "none");
        assert_eq!(pull(0.2), "normal");
        assert_eq!(split(3.5), "short, a turning fill");
        assert_eq!(angle(0.0), "horizontal");
        assert_eq!(angle(175.0), "horizontal");
        assert_eq!(angle(-90.0), "vertical");
        assert_eq!(angle(45.0), "diagonal, down to the right");
        assert_eq!(angle(135.0), "diagonal, up to the right");
        assert_eq!(underlay(&Underlay::Auto), "Normal");
        assert_eq!(underlay_weight(UnderlayWeight::Heavy), "Heavy");
    }
}
