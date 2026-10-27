//! The production worksheet and the approval sheet (roadmap P3,
//! Milestone 5). Both are pages (`page::Page`), so the app can preview
//! them and `pdf` can write them.
//!
//! - Production worksheet: for the person at the machine. The design
//!   picture, the numbers (size, stitches, colors, trims, stops, sew time),
//!   machine, hoop and files, how the design sits in the hoop (a small
//!   diagram and its orientation), the colors in sew order, and the notes.
//! - Approval sheet: for the customer. A large picture, the size, the
//!   colors, the notes, and lines to sign.
//! - Quote sheet (P13 Milestone 1, ADR 0064): for the customer. The
//!   picture, the numbers of the design, the lines of the price, the total,
//!   and lines to sign.

use emb_geom::{BBox, Point};
use emb_model::Rgb;

use crate::Segment;
use crate::page::{Op, Page, Paper, fit_segments, wrap};

const MARGIN: f64 = 15.0;
const BLACK: Rgb = Rgb(0, 0, 0);
const RULE: Rgb = Rgb(160, 160, 160);
const BOX: Rgb = Rgb(200, 200, 200);
/// The picture background when the design has no fabric color: light
/// gray, so white thread shows.
const PAPER_GRAY: Rgb = Rgb(232, 232, 232);

/// One color block, in sew order.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetColor {
    pub name: String,
    pub brand: Option<String>,
    pub code: Option<String>,
    pub color: Rgb,
    pub stitches: usize,
    pub length_mm: f64,
    /// The top thread that the color uses, in mm (`emb_model::thread_use`).
    pub thread_mm: f64,
    /// The needle of the color on a multi-needle machine (ADR 0063).
    pub needle: Option<u8>,
    /// The operator changes the thread on the needle before this color.
    pub thread_change: bool,
}

impl SheetColor {
    /// The thread as text: the name, and the brand and code if known.
    pub fn thread_text(&self) -> String {
        match (&self.brand, &self.code) {
            (Some(b), Some(k)) => format!("{} ({b} {k})", self.name),
            (None, Some(k)) => format!("{} ({k})", self.name),
            _ => self.name.clone(),
        }
    }
}

/// The thread that goes on a needle before the start (ADR 0063).
#[derive(Debug, Clone, PartialEq)]
pub struct SheetNeedle {
    pub needle: u8,
    /// The thread as text (name, brand and code).
    pub thread: String,
    pub color: Rgb,
    /// The thread is not on the needle now: the operator puts it on.
    pub put_on: bool,
}

/// How the design sits in the hoop: the hoop size, and the design box in
/// output coordinates (origin at the hoop center, Y down, the design
/// rotation applied).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoopLayout {
    pub width_mm: f64,
    pub height_mm: f64,
    pub design: BBox,
    /// The design rotation in the hoop: 0, 90, 180 or 270 degrees clockwise.
    pub rotation_deg: u16,
    /// The hoop angle on the canvas (ADR 0032): in the hoop the design is
    /// on a slant of this many degrees the other way.
    pub hoop_angle_deg: f64,
}

impl HoopLayout {
    /// The layout of `design` in `hoop`, from its stitches in output
    /// coordinates. `None` for a design with no stitches.
    pub fn new(hoop: &emb_model::HoopPreset, design: &emb_model::Design, s: &emb_model::StitchDesign) -> Option<Self> {
        Some(HoopLayout {
            width_mm: hoop.width_mm,
            height_mm: hoop.height_mm,
            design: s.extents()?,
            rotation_deg: design.rotation.degrees(),
            hoop_angle_deg: design.hoop.angle_deg,
        })
    }

    /// The shape of the hoop as the operator puts it on the machine.
    pub fn hoop_text(&self) -> String {
        let shape = match self.width_mm - self.height_mm {
            d if d > 0.5 => "Landscape",
            d if d < -0.5 => "Portrait",
            _ => "Square",
        };
        format!(
            "{shape}: {:.0} mm from left to right, {:.0} mm from the back to the front",
            self.width_mm, self.height_mm
        )
    }

    /// Where the top of the design points, and where its center is.
    pub fn design_text(&self) -> String {
        // A hoop angle of a quarter turn (a turned hooping) adds to the
        // design rotation; other angles are a slant.
        let a = self.hoop_angle_deg;
        let total = (f64::from(self.rotation_deg) - a).rem_euclid(360.0);
        let quarter = (total / 90.0).round();
        let (turn_deg, a) = if (total - quarter * 90.0).abs() < 0.05 {
            ((quarter as i32 * 90).rem_euclid(360), 0.0)
        } else {
            (self.rotation_deg as i32, a)
        };
        let turn = match turn_deg {
            90 => "Turned 90° clockwise: the top of the design is at the right side of the hoop",
            180 => "Upside down: the top of the design is at the bottom of the hoop",
            270 => "Turned 90° counterclockwise: the top of the design is at the left side of the hoop",
            _ => "Upright: the top of the design is at the top of the hoop",
        };
        let c = self.design.center();
        let side = |v: f64, plus: &str, minus: &str| {
            if v.abs() < 0.5 {
                None
            } else {
                Some(format!("{:.0} mm {}", v.abs(), if v > 0.0 { plus } else { minus }))
            }
        };
        let off: Vec<String> = [side(c.x, "right", "left"), side(c.y, "down", "up")]
            .into_iter()
            .flatten()
            .collect();
        let center = if off.is_empty() {
            "The design is at the hoop center.".to_string()
        } else {
            format!("The design center is {} from the hoop center.", off.join(" and "))
        };
        let slant = if a.abs() < 0.05 {
            String::new()
        } else {
            format!(
                " The design is on a slant: turned {:.1}° {} in the hoop.",
                a.abs(),
                if a > 0.0 { "counterclockwise" } else { "clockwise" }
            )
        };
        format!("{turn}. {center}{slant}")
    }

    /// The unit vector from the design center toward the top of the design.
    fn up(&self) -> Point {
        let up = match self.rotation_deg {
            90 => Point::new(1.0, 0.0),
            180 => Point::new(0.0, 1.0),
            270 => Point::new(-1.0, 0.0),
            _ => Point::new(0.0, -1.0),
        };
        emb_geom::Transform::rotate_deg(-self.hoop_angle_deg).apply(up)
    }
}

/// Width of the hoop diagram on the worksheet, in mm.
const HOOP_DIAGRAM_MM: f64 = 46.0;

/// The hoop with the design box in it, at `x, y` (top left), with "Back of
/// machine" above it and an arrow to the top of the design. Returns the y under it.
fn hoop_diagram(p: &mut Page, l: &HoopLayout, x: f64, y: f64) -> f64 {
    let s = HOOP_DIAGRAM_MM / l.width_mm.max(l.height_mm).max(1.0);
    let (w, h) = (l.width_mm * s, l.height_mm * s);
    let label = "Back of machine";
    let lw = crate::page::text_width_mm(label, 8.0);
    p.gray_text(x + (w - lw) / 2.0, y + 3.0, 8.0, label);
    let top = y + 5.0;
    let c = Point::new(x + w / 2.0, top + h / 2.0);
    p.ops.push(Op::Rect {
        rect: BBox::new(Point::new(x, top), Point::new(x + w, top + h)),
        fill: None,
        stroke: Some((0.4, BLACK)),
    });
    // The hoop center.
    for d in [Point::new(1.5, 0.0), Point::new(0.0, 1.5)] {
        p.line(c - d, c + d, 0.15, RULE);
    }
    let to_page = |q: Point| c + q * s;
    let b = BBox::new(to_page(l.design.min), to_page(l.design.max));
    p.ops.push(Op::Rect {
        rect: b,
        fill: Some(Rgb(215, 225, 240)),
        stroke: Some((0.25, Rgb(60, 90, 150))),
    });
    // An arrow from the design center toward its top.
    let up = l.up();
    let reach = (0.4 * if up.x == 0.0 { b.height() } else { b.width() }).max(2.5);
    let (from, tip) = (b.center() - up * (reach * 0.5), b.center() + up * (reach * 0.5));
    let side = Point::new(-up.y, up.x);
    let blue = Rgb(30, 60, 140);
    p.line(from, tip, 0.35, blue);
    p.line(tip, tip - up * 1.4 + side * 1.0, 0.35, blue);
    p.line(tip, tip - up * 1.4 - side * 1.0, 0.35, blue);
    top + h + 3.0
}

/// What the sheets show. The caller fills it from the design and its stitch plan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SheetData {
    pub name: String,
    pub notes: String,
    /// The date to print, as text.
    pub date: String,
    pub machine: Option<String>,
    pub hoop: Option<String>,
    /// How the design sits in the hoop (none for an open field).
    pub hoop_layout: Option<HoopLayout>,
    pub colorway: Option<String>,
    /// File names that go with the design (machine file, project file).
    pub files: Vec<String>,
    pub size: Option<(f64, f64)>,
    pub stitches: usize,
    pub trims: usize,
    pub stops: usize,
    pub minutes: Option<f64>,
    pub colors: Vec<SheetColor>,
    /// The top and bobbin thread of the design, in mm.
    pub top_mm: f64,
    pub bobbin_mm: f64,
    /// The stitches in design mm.
    pub segments: Vec<Segment>,
    pub background: Option<Rgb>,
    /// The fabric name, and the backing and topping for it.
    pub fabric: Option<String>,
    pub backing: Option<String>,
    pub topping: Option<String>,
    /// What the operator does at each stop, in sew order.
    pub stop_notes: Vec<String>,
    /// Appliqué fabric under the stitches: rings (holes inside) in the
    /// coordinates of `segments`, and the color.
    pub fabric_areas: Vec<(Vec<Vec<Point>>, Rgb)>,
    /// A barcode at the top right of the production worksheet, and its
    /// text (the file name, so a shop system can find the design).
    pub barcode: Option<(crate::barcode::BarcodeKind, String)>,
    /// Rhinestones (P9): the center, radius and color of each stone in
    /// the coordinates of `segments`, and the number of each kind of stone
    /// (its name and color).
    pub stones: SheetStones,
    pub stone_counts: StoneCounts,
    /// The order of the processes of a design with more than one
    /// (embroidery and rhinestones), as steps.
    pub process_steps: Vec<String>,
    /// A multi-needle machine (ADR 0063): the thread for each needle at the
    /// start, the needles in sew order as text, and what the operator does
    /// at each thread change.
    pub needles: Vec<SheetNeedle>,
    pub needle_sequence: String,
    pub thread_changes: Vec<String>,
    /// The order of the design (P13 Milestone 2): the sheets show its rows
    /// and notes, and the barcode has its number.
    pub order: Option<emb_model::order::OrderInfo>,
    /// The price of the order as text, for the approval sheet (the quote,
    /// ADR 0064). `None`: no price is set.
    pub price: Option<String>,
    /// The threads to buy for the order (ADR 0066), one line each.
    pub to_buy: Vec<String>,
}

impl SheetData {
    /// The text of the barcode: the order number, else `file_name` (so a
    /// shop system finds the order or the design).
    pub fn barcode_text(&self, file_name: &str) -> String {
        match self.order.as_ref().map(|o| o.number.trim()).filter(|n| !n.is_empty()) {
            Some(n) => n.to_string(),
            None => file_name.to_string(),
        }
    }

    /// The design notes, and the notes of the order after them.
    pub fn notes_text(&self) -> String {
        match self.order.as_ref().map(|o| o.notes.trim()).filter(|n| !n.is_empty()) {
            Some(n) if self.notes.trim().is_empty() => format!("Order: {n}"),
            Some(n) => format!(
                "{}
Order: {n}",
                self.notes.trim_end()
            ),
            None => self.notes.clone(),
        }
    }

    /// The rows of the order (none with no order).
    fn order_rows(&self) -> Vec<(&'static str, String)> {
        self.order.as_ref().map(|o| o.rows()).unwrap_or_default()
    }

    /// The numbers, colors and stitches of a stitch list. The caller adds
    /// the machine, hoop, sew time, notes, date and files.
    pub fn from_stitches(name: &str, s: &emb_model::StitchDesign) -> SheetData {
        use emb_model::StitchKind;
        let thread = |i: usize| s.threads.get(i);
        let color_at = |i: usize| SheetColor {
            name: thread(i).map_or(format!("Thread {}", i + 1), |t| t.name.clone()),
            brand: thread(i).and_then(|t| t.brand.clone()),
            code: thread(i).and_then(|t| t.code.clone()),
            color: thread(i).map_or(crate::FALLBACK_COLOR, |t| t.color),
            stitches: 0,
            length_mm: 0.0,
            thread_mm: 0.0,
            needle: None,
            thread_change: false,
        };
        let mut colors = vec![color_at(0)];
        let (mut trims, mut stops) = (0, 0);
        let mut prev: Option<(StitchKind, Point)> = None;
        for r in &s.records {
            let last = colors.len() - 1;
            match r.kind {
                StitchKind::Stitch => {
                    colors[last].stitches += 1;
                    if let Some((StitchKind::Stitch, p)) = prev {
                        colors[last].length_mm += p.distance(r.pos);
                    }
                }
                StitchKind::Trim => trims += 1,
                StitchKind::Stop => stops += 1,
                StitchKind::Slow | StitchKind::Fast => {}
                StitchKind::ColorChange => colors.push(color_at(colors.len())),
                StitchKind::Jump | StitchKind::End => {}
            }
            prev = Some((r.kind, r.pos));
        }
        let stitches = s.stitch_count();
        let mut d = SheetData {
            name: name.to_string(),
            size: s.extents().map(|b| (b.width(), b.height())),
            stitches,
            trims,
            stops,
            colors: if stitches > 0 { colors } else { Vec::new() },
            segments: crate::segments(s),
            ..SheetData::default()
        };
        d.set_thread_use(s, emb_model::thread_use::DEFAULT_THICKNESS_MM);
        d
    }

    /// The needles of a multi-needle machine (ADR 0063): the needle of each
    /// color, the threads at the start, the sequence and the thread
    /// changes. Each thread change is a stop.
    pub fn set_needles(&mut self, plan: &emb_model::NeedlePlan) {
        for (i, c) in self.colors.iter_mut().enumerate() {
            c.needle = plan.blocks.get(i).copied();
            c.thread_change = plan.changes.contains(&i);
        }
        self.needles = plan
            .start
            .iter()
            .enumerate()
            .filter_map(|(k, b)| {
                let c = self.colors.get((*b)?)?;
                let needle = k as u8 + 1;
                Some(SheetNeedle {
                    needle,
                    thread: c.thread_text(),
                    color: c.color,
                    put_on: plan.loads.contains(&needle),
                })
            })
            .collect();
        self.needle_sequence = plan.sequence_text();
        self.thread_changes = plan
            .changes
            .iter()
            .filter_map(|&b| {
                let c = self.colors.get(b)?;
                Some(format!(
                    "Color {}: the machine stops. Put {} on needle {}.",
                    b + 1,
                    c.thread_text(),
                    c.needle?
                ))
            })
            .collect();
        self.stops += plan.changes.len();
    }

    /// The thread use of the stitches `s` on a fabric `thickness_mm` thick
    /// (the caller knows the fabric of the design).
    pub fn set_thread_use(&mut self, s: &emb_model::StitchDesign, thickness_mm: f64) {
        let u = emb_model::thread_use::thread_use(s, thickness_mm);
        for (c, t) in self.colors.iter_mut().zip(&u.colors) {
            c.thread_mm = t.top_mm;
        }
        self.top_mm = u.top_mm;
        self.bobbin_mm = u.bobbin_mm;
    }
}

/// Stones on a sheet: the center, radius and color of each stone.
pub type SheetStones = Vec<(Point, f64, Rgb)>;
/// The number of each kind of stone: its name and color.
pub type StoneCounts = Vec<(String, Rgb, usize)>;

/// The order of work of a design with rhinestones (P9 Milestone 5): sew,
/// make the template, put the stones on, press. `name` is the file name of
/// the output (the template files are `<name> template.*`). Empty for a
/// design with no stones.
pub fn process_steps(design: &emb_model::Design, name: &str) -> Vec<String> {
    let stones = design.stones();
    if stones.is_empty() {
        return Vec::new();
    }
    let marks = design.sewn_mark_centers().is_some();
    let mut s = vec![if design.objects().iter().any(|o| o.kind.sews()) {
        if marks {
            "Sew the embroidery (the machine file). The last color sews four crosses: the template marks.".to_string()
        } else {
            "Sew the embroidery (the machine file).".to_string()
        }
    } else {
        "The design has no embroidery: only rhinestones.".to_string()
    }];
    s.push(format!(
        "Cut the template ({name} template.svg) from template material. Brush the {} stones into it, the largest size first.",
        stones.len()
    ));
    if marks {
        s.push(
            "Pick up the stones with transfer tape. Mark the four crosses of the template on the tape with a pen."
                .into(),
        );
        s.push("Put the tape on the fabric with its crosses on the sewn crosses.".into());
    } else {
        s.push("Pick up the stones with transfer tape. Put the tape on the fabric where the picture shows the stones (no sewn marks: add them with Specialty > Template marks).".into());
    }
    s.push("Press with a heat press (the time and temperature that the maker of the stones gives).".into());
    if marks {
        s.push("Pull out the threads of the template marks.".into());
    }
    s
}

/// The stones of a design (`Design::stones`) in output coordinates (as
/// [`fabric_areas`]), and the number of each kind of stone.
pub fn stones_of(design: &emb_model::Design) -> (SheetStones, StoneCounts) {
    let to_out = design.output_transform();
    let stones = design.stones();
    let placed = stones.iter().map(|s| (to_out.apply(s.at), s.radius, s.color)).collect();
    let mut counts: StoneCounts = Vec::new();
    for s in &stones {
        let Some(emb_model::ObjectKind::Bling(b)) = design.object(s.object).map(|o| &o.kind) else {
            continue;
        };
        let label = b.stone.label();
        match counts.iter_mut().find(|(l, c, _)| *l == label && *c == b.stone.color) {
            Some((_, _, n)) => *n += 1,
            None => counts.push((label, b.stone.color, 1)),
        }
    }
    (placed, counts)
}

/// The areas of the design picture: the appliqué fabric and each stone as
/// a disk in its color (output coordinates).
pub fn picture_areas(design: &emb_model::Design) -> Vec<(Vec<Vec<Point>>, Rgb)> {
    let mut out = fabric_areas(design);
    for (c, r, color) in stones_of(design).0 {
        let disk = (0..16)
            .map(|i| {
                let a = emb_geom::math::to_radians(22.5 * f64::from(i));
                Point::new(c.x + r * emb_geom::math::cos(a), c.y + r * emb_geom::math::sin(a))
            })
            .collect();
        out.push((vec![disk], color));
    }
    out
}

/// The appliqué fabric of a design as areas in output coordinates (origin
/// at the hoop center, the design rotation applied), as the stitch plan.
pub fn fabric_areas(design: &emb_model::Design) -> Vec<(Vec<Vec<Point>>, Rgb)> {
    let to_out = design.output_transform();
    design
        .objects()
        .iter()
        .filter_map(|o| match &o.kind {
            emb_model::ObjectKind::Applique { pieces, params } => {
                // Each piece in its color, the bottom layer first.
                let mut pieces: Vec<&emb_model::AppliquePiece> = pieces.iter().collect();
                pieces.sort_by_key(|pc| pc.layer);
                Some(pieces.into_iter().map(|pc| {
                    let rings = std::iter::once(&pc.outer)
                        .chain(pc.holes.iter())
                        .flat_map(|path| path.flatten(0.1))
                        .map(|pl| pl.points.iter().map(|q| to_out.apply(*q)).collect())
                        .collect();
                    (rings, pc.fabric_color(params))
                }))
            }
            _ => None,
        })
        .flatten()
        .collect()
}

fn time_text(minutes: f64) -> String {
    let secs = (minutes * 60.0).round() as i64;
    if secs >= 3600 {
        format!("{}:{:02}:{:02}", secs / 3600, secs % 3600 / 60, secs % 60)
    } else {
        format!("{}:{:02}", secs / 60, secs % 60)
    }
}

fn size_text(size: Option<(f64, f64)>) -> String {
    size.map(|(w, h)| format!("{w:.1} × {h:.1} mm ({:.2} × {:.2} in)", w / 25.4, h / 25.4))
        .unwrap_or_else(|| "—".into())
}

/// The header of a page: name, kind of sheet and date, and a rule.
fn header(p: &mut Page, d: &SheetData, kind: &str) -> f64 {
    p.text(MARGIN, MARGIN + 7.0, 18.0, true, d.name.clone());
    p.gray_text(MARGIN, MARGIN + 13.0, 10.0, format!("{kind} · {}", d.date));
    let y = MARGIN + 16.0;
    p.line(Point::new(MARGIN, y), Point::new(p.width_mm - MARGIN, y), 0.3, RULE);
    y + 6.0
}

/// The picture in a box, with the size under it. Returns the box bottom.
fn picture(p: &mut Page, d: &SheetData, area: BBox) -> f64 {
    p.ops.push(Op::Rect {
        rect: area,
        fill: Some(d.background.unwrap_or(PAPER_GRAY)),
        stroke: Some((0.2, BOX)),
    });
    let inner = BBox::new(area.min + Point::new(4.0, 4.0), area.max - Point::new(4.0, 4.0));
    let (mut segments, mut fit) = fit_segments(&d.segments, inner);
    // With rhinestones, the picture fits the stitches and the stones.
    if let Some(stones) = d
        .stones
        .iter()
        .map(|(c, r, _)| BBox::from_center_size(*c, 2.0 * r, 2.0 * r))
        .reduce(BBox::union)
    {
        let all = d
            .segments
            .iter()
            .filter(|s| s.kind == crate::SegmentKind::Stitch)
            .flat_map(|s| [s.a, s.b])
            .fold(stones, |b, q| b.union(BBox::new(q, q)));
        fit = crate::page::Fit::of(all, inner);
        segments = d
            .segments
            .iter()
            .filter(|s| s.kind == crate::SegmentKind::Stitch)
            .map(|s| Segment {
                a: fit.apply(s.a),
                b: fit.apply(s.b),
                ..*s
            })
            .collect();
    }
    let scale = fit.scale;
    for (rings, color) in &d.fabric_areas {
        p.ops.push(Op::Area {
            rings: rings
                .iter()
                .map(|r| r.iter().map(|q| fit.apply(*q)).collect())
                .collect(),
            fill: *color,
        });
    }
    if !segments.is_empty() {
        p.ops.push(Op::Stitches {
            segments,
            width_mm: (0.35 * scale).clamp(0.1, 0.6),
        });
    }
    // Each stone: a gray disk a little larger, so light stones show on a
    // light background, and the stone in its color on it.
    for (c, r, color) in &d.stones {
        let (c, r) = (fit.apply(*c), r * scale);
        let disk = |r: f64| -> Vec<Point> {
            (0..16)
                .map(|i| {
                    let a = emb_geom::math::to_radians(22.5 * f64::from(i));
                    Point::new(c.x + r * emb_geom::math::cos(a), c.y + r * emb_geom::math::sin(a))
                })
                .collect()
        };
        p.ops.push(Op::Area {
            rings: vec![disk(r + 0.15)],
            fill: Rgb(110, 110, 110),
        });
        p.ops.push(Op::Area {
            rings: vec![disk(r)],
            fill: *color,
        });
    }
    let shown = if (scale - 1.0).abs() < 0.005 {
        "shown at real size".to_string()
    } else {
        format!("shown at {:.0} %", scale * 100.0)
    };
    p.gray_text(
        area.min.x,
        area.max.y + 5.0,
        9.0,
        format!("{} · {shown}", size_text(d.size)),
    );
    area.max.y + 5.0
}

/// Label and value rows; long values wrap to the page margin. Returns the
/// y below the last row.
fn rows(p: &mut Page, x: f64, mut y: f64, rows: &[(&str, String)]) -> f64 {
    let vx = x + 27.0;
    let chars = ((p.width_mm - MARGIN - vx) / crate::page::text_width_mm("x", 9.5)).floor() as usize;
    for (k, v) in rows {
        p.gray_text(x, y, 9.0, k.to_string());
        for (i, line) in wrap(v, chars.max(8)).into_iter().enumerate() {
            if i > 0 {
                y += 4.6;
            }
            p.text(vx, y, 9.5, false, line);
        }
        y += 6.0;
    }
    y
}

fn notes(p: &mut Page, notes: &str, y: f64, chars: usize) -> f64 {
    if notes.trim().is_empty() {
        return y;
    }
    p.text(MARGIN, y, 10.0, true, "Notes");
    let mut y = y + 5.5;
    for line in wrap(notes, chars) {
        p.text(MARGIN, y, 9.5, false, line);
        y += 4.8;
    }
    y
}

/// The height of [`needle_list`] in mm.
fn needle_list_height(d: &SheetData) -> f64 {
    if d.needles.is_empty() {
        return 0.0;
    }
    let changes = if d.thread_changes.is_empty() {
        0.0
    } else {
        5.5 + 4.8 * d.thread_changes.len() as f64
    };
    8.5 + 5.5 * d.needles.len().div_ceil(2) as f64 + 9.6 + changes
}

/// The needles of a multi-needle machine (ADR 0063): the thread for each
/// needle at the start (two columns), the needle sequence, and the thread
/// changes.
fn needle_list(p: &mut Page, d: &SheetData, y: f64) -> f64 {
    if d.needles.is_empty() {
        return y;
    }
    p.text(MARGIN, y, 10.0, true, "Needles at the start");
    let mut y = y + 5.5;
    let col_w = (p.width_mm - 2.0 * MARGIN) / 2.0;
    for (i, n) in d.needles.iter().enumerate() {
        let x = MARGIN + col_w * (i % 2) as f64;
        let yy = y + 5.5 * (i / 2) as f64;
        p.text(x, yy, 9.5, true, n.needle.to_string());
        p.ops.push(Op::Rect {
            rect: BBox::new(Point::new(x + 7.0, yy - 3.6), Point::new(x + 14.0, yy + 1.0)),
            fill: Some(n.color),
            stroke: Some((0.15, BLACK)),
        });
        let mut t: String = n.thread.chars().take(46).collect();
        if n.put_on {
            t.push_str(" (put on)");
        }
        p.text(x + 17.0, yy, 9.5, false, t);
    }
    y += 5.5 * d.needles.len().div_ceil(2) as f64 + 1.0;
    p.text(MARGIN, y, 9.0, false, format!("Needle sequence: {}", d.needle_sequence));
    y += 4.8;
    p.gray_text(
        MARGIN,
        y,
        8.5,
        "Type this sequence on the machine for DST, TAP and EXP files. TBF and U01 files have the needles.",
    );
    y += 4.8;
    if !d.thread_changes.is_empty() {
        y += 1.0;
        p.text(MARGIN, y, 10.0, true, "Thread changes (*)");
        y += 5.5;
        for s in &d.thread_changes {
            p.text(MARGIN, y, 9.5, false, s.chars().take(110).collect::<String>());
            y += 4.8;
        }
    }
    y + 3.0
}

/// The list of stops with what to do at each. Returns the y under it.
fn stop_list(p: &mut Page, stops: &[String], y: f64) -> f64 {
    if stops.is_empty() {
        return y;
    }
    p.text(MARGIN, y, 10.0, true, "Stops");
    let mut y = y + 5.5;
    for (i, s) in stops.iter().enumerate() {
        for (k, line) in wrap(&format!("{}. {s}", i + 1), 100).into_iter().enumerate() {
            p.text(MARGIN + if k == 0 { 0.0 } else { 4.0 }, y, 9.5, false, line);
            y += 4.8;
        }
    }
    y + 3.0
}

/// The threads to buy for the order.
fn buy_list(p: &mut Page, lines: &[String], y: f64) -> f64 {
    if lines.is_empty() {
        return y;
    }
    p.text(MARGIN, y, 10.0, true, "Threads to buy");
    let mut y = y + 5.5;
    for l in lines {
        for (k, line) in wrap(l, 100).into_iter().enumerate() {
            p.text(MARGIN + if k == 0 { 0.0 } else { 4.0 }, y, 9.5, false, line);
            y += 4.8;
        }
    }
    y + 3.0
}

/// The order of work, when the design has more than one process.
fn step_list(p: &mut Page, steps: &[String], y: f64) -> f64 {
    if steps.is_empty() {
        return y;
    }
    p.text(MARGIN, y, 10.0, true, "Order of work");
    let mut y = y + 5.5;
    for (i, s) in steps.iter().enumerate() {
        for (k, line) in wrap(&format!("{}. {s}", i + 1), 100).into_iter().enumerate() {
            p.text(MARGIN + if k == 0 { 0.0 } else { 4.0 }, y, 9.5, false, line);
            y += 4.8;
        }
    }
    y + 3.0
}

/// The rhinestones of the design: each kind of stone and its number.
fn stone_list(p: &mut Page, counts: &[(String, Rgb, usize)], y: f64) -> f64 {
    if counts.is_empty() {
        return y;
    }
    let total: usize = counts.iter().map(|c| c.2).sum();
    p.text(
        MARGIN,
        y,
        10.0,
        true,
        format!("Rhinestones ({total}, not sewn: use the template)"),
    );
    let mut y = y + 5.5;
    for (label, color, n) in counts {
        p.ops.push(Op::Rect {
            rect: BBox::new(Point::new(MARGIN, y - 3.4), Point::new(MARGIN + 4.0, y + 0.6)),
            fill: Some(*color),
            stroke: Some((0.15, BLACK)),
        });
        p.text(MARGIN + 7.0, y, 9.5, false, format!("{label}: {n}"));
        y += 4.8;
    }
    y + 3.0
}

/// The production worksheet: one page, and more pages if the color table
/// is long.
pub fn production(d: &SheetData, paper: Paper) -> Vec<Page> {
    let mut pages = Vec::new();
    let mut p = Page::new(paper);
    let w = p.width_mm;
    let top = header(&mut p, d, "Production worksheet");
    if let Some((kind, text)) = &d.barcode {
        crate::barcode::draw(&mut p, *kind, text, w - MARGIN, MARGIN, 9.0, 70.0);
    }
    let pic = BBox::new(Point::new(MARGIN, top), Point::new(MARGIN + 100.0, top + 110.0));
    picture(&mut p, d, pic);

    let mut info: Vec<(&str, String)> = d.order_rows();
    info.extend([
        (
            "Size",
            d.size
                .map(|(a, b)| format!("{a:.1} × {b:.1} mm"))
                .unwrap_or_else(|| "—".into()),
        ),
        ("Stitches", d.stitches.to_string()),
        ("Colors", d.colors.len().to_string()),
        ("Color changes", d.colors.len().saturating_sub(1).to_string()),
        ("Trims", d.trims.to_string()),
    ]);
    if d.stops > 0 {
        info.push(("Stops", d.stops.to_string()));
    }
    info.push((
        "Sew time",
        d.minutes
            .map(|m| format!("about {}", time_text(m)))
            .unwrap_or_else(|| "—".into()),
    ));
    info.push(("Machine", d.machine.clone().unwrap_or_else(|| "—".into())));
    info.push(("Hoop", d.hoop.clone().unwrap_or_else(|| "Open field".into())));
    if let Some(l) = &d.hoop_layout {
        info.push(("Hooping", l.hoop_text()));
        info.push(("Orientation", l.design_text()));
    }
    if let Some(c) = &d.colorway {
        info.push(("Colorway", c.clone()));
    }
    for (k, v) in [("Fabric", &d.fabric), ("Backing", &d.backing), ("Topping", &d.topping)] {
        if let Some(v) = v.as_ref().filter(|v| !v.trim().is_empty()) {
            info.push((k, v.clone()));
        }
    }
    let x = MARGIN + 107.0;
    let mut y = rows(&mut p, x, top + 4.0, &info);
    if !d.files.is_empty() {
        p.gray_text(x, y, 9.0, "Files");
        y += 5.0;
        for f in &d.files {
            p.text(x, y, 8.5, false, f.clone());
            y += 4.5;
        }
    }
    if let Some(l) = &d.hoop_layout {
        y = hoop_diagram(&mut p, l, x, y + 1.0);
    }

    // Colors in sew order. A multi-needle machine gets a needle column.
    let with_needles = d.colors.iter().any(|c| c.needle.is_some());
    let shift = if with_needles { 13.0 } else { 0.0 };
    let col = [
        MARGIN,
        MARGIN + 9.0 + shift,
        MARGIN + 22.0 + shift,
        MARGIN + 92.0 + shift * 0.6,
        MARGIN + 132.0 + shift * 0.4,
        MARGIN + 158.0 + shift * 0.2,
    ];
    let (name_chars, brand_chars) = if with_needles { (28, 18) } else { (34, 20) };
    let needle_x = MARGIN + 9.0;
    let table_head = |p: &mut Page, y: f64| {
        for (x, t) in col
            .iter()
            .zip(["#", "Color", "Thread", "Brand and code", "Stitches", "Top thread"])
        {
            p.text(*x, y, 9.0, true, t);
        }
        if with_needles {
            p.text(needle_x, y, 9.0, true, "Needle");
        }
        p.line(Point::new(MARGIN, y + 1.8), Point::new(w - MARGIN, y + 1.8), 0.2, RULE);
        y + 7.0
    };
    // Below the picture, and below the info and files if they are longer.
    let mut y = table_head(&mut p, (top + 110.0 + 16.0).max(y + 6.0));
    let bottom = p.height_mm - MARGIN - 8.0;
    for (i, c) in d.colors.iter().enumerate() {
        if y > bottom {
            pages.push(std::mem::replace(&mut p, Page::new(paper)));
            let t = header(&mut p, d, "Production worksheet (continued)");
            y = table_head(&mut p, t + 4.0);
        }
        p.text(col[0], y, 9.5, false, (i + 1).to_string());
        if let Some(n) = c.needle {
            // A thread change: the operator changes the thread on the needle.
            let t = if c.thread_change {
                format!("{n} *")
            } else {
                n.to_string()
            };
            p.text(needle_x + 2.0, y, 9.5, c.thread_change, t);
        }
        p.ops.push(Op::Rect {
            rect: BBox::new(Point::new(col[1], y - 3.6), Point::new(col[1] + 9.0, y + 1.0)),
            fill: Some(c.color),
            stroke: Some((0.15, BLACK)),
        });
        let name: String = c.name.chars().take(name_chars).collect();
        p.text(col[2], y, 9.5, false, name);
        let brand = match (&c.brand, &c.code) {
            (Some(b), Some(k)) => format!("{b} {k}"),
            (Some(b), None) => b.clone(),
            (None, Some(k)) => k.clone(),
            (None, None) => String::new(),
        };
        p.text(
            col[3],
            y,
            9.5,
            false,
            brand.chars().take(brand_chars).collect::<String>(),
        );
        p.text(col[4], y, 9.5, false, c.stitches.to_string());
        p.text(col[5], y, 9.5, false, emb_model::thread_use::length_text(c.thread_mm));
        y += 6.5;
    }
    if !d.colors.is_empty() {
        let len = emb_model::thread_use::length_text;
        let text = format!(
            "Thread (an estimate): top thread {}, bobbin thread {}",
            len(d.top_mm),
            len(d.bobbin_mm)
        );
        p.text(col[2], y, 9.0, false, text);
        y += 6.0;
    }
    let mut y = y + 4.0;
    let notes_text = d.notes_text();
    let notes_h = if notes_text.trim().is_empty() { 0.0 } else { 20.0 };
    let stops_h = if d.stop_notes.is_empty() {
        0.0
    } else {
        8.5 + 4.8 * d.stop_notes.len() as f64
    } + if d.to_buy.is_empty() {
        0.0
    } else {
        8.5 + 4.8 * d.to_buy.len() as f64
    };
    let needles_h = needle_list_height(d);
    let stones_h = needles_h
        + if d.stone_counts.is_empty() {
            0.0
        } else {
            8.5 + 4.8 * d.stone_counts.len() as f64
        }
        + if d.process_steps.is_empty() {
            0.0
        } else {
            8.5 + 9.6 * d.process_steps.len() as f64
        };
    if stops_h + stones_h + notes_h > 0.0 && y + stops_h + stones_h + notes_h > p.height_mm - MARGIN {
        pages.push(std::mem::replace(&mut p, Page::new(paper)));
        y = header(&mut p, d, "Production worksheet (continued)") + 4.0;
    }
    let y = needle_list(&mut p, d, y);
    let y = stop_list(&mut p, &d.stop_notes, y);
    let y = buy_list(&mut p, &d.to_buy, y);
    let y = stone_list(&mut p, &d.stone_counts, y);
    let y = step_list(&mut p, &d.process_steps, y);
    notes(&mut p, &notes_text, y, 95);
    pages.push(p);
    let n = pages.len();
    if n > 1 {
        for (i, pg) in pages.iter_mut().enumerate() {
            let h = pg.height_mm;
            pg.gray_text(MARGIN, h - MARGIN + 6.0, 8.0, format!("Page {} of {n}", i + 1));
        }
    }
    pages
}

/// The approval sheet: one page.
pub fn approval(d: &SheetData, paper: Paper) -> Page {
    let mut p = Page::new(paper);
    let w = p.width_mm;
    let top = header(&mut p, d, "Design approval");
    let pic = BBox::new(Point::new(MARGIN, top), Point::new(w - MARGIN, top + 140.0));
    let mut y = picture(&mut p, d, pic) + 10.0;
    p.text(MARGIN, y, 10.0, true, "Colors");
    y += 6.0;
    let col_w = (w - 2.0 * MARGIN) / 2.0;
    for (i, c) in d.colors.iter().enumerate() {
        let x = MARGIN + col_w * (i % 2) as f64;
        let yy = y + 6.0 * (i / 2) as f64;
        p.ops.push(Op::Rect {
            rect: BBox::new(Point::new(x, yy - 3.6), Point::new(x + 9.0, yy + 1.0)),
            fill: Some(c.color),
            stroke: Some((0.15, BLACK)),
        });
        let mut t = format!("{}. {}", i + 1, c.name);
        if let Some(code) = &c.code {
            t.push_str(&format!(" ({code})"));
        }
        p.text(x + 12.0, yy, 9.5, false, t.chars().take(44).collect::<String>());
    }
    y += 6.0 * d.colors.len().div_ceil(2) as f64 + 6.0;
    if let Some(f) = &d.fabric {
        p.text(MARGIN, y, 10.0, true, "Fabric");
        p.text(MARGIN + 22.0, y, 10.0, false, f.clone());
        y += 9.0;
    }
    let mut order = d.order_rows();
    if let Some(price) = &d.price {
        order.push(("Price", price.clone()));
    }
    if !order.is_empty() {
        y = rows(&mut p, MARGIN, y, &order) + 3.0;
    }
    notes(&mut p, &d.notes_text(), y, 95);

    // Lines to sign at the bottom.
    let h = p.height_mm;
    let sy = h - MARGIN - 6.0;
    p.line(Point::new(MARGIN, sy), Point::new(MARGIN + 95.0, sy), 0.3, BLACK);
    p.line(
        Point::new(w - MARGIN - 60.0, sy),
        Point::new(w - MARGIN, sy),
        0.3,
        BLACK,
    );
    p.gray_text(MARGIN, sy + 4.5, 9.0, "Approved by");
    p.gray_text(w - MARGIN - 60.0, sy + 4.5, 9.0, "Date");
    p
}

/// The quote sheet: one page with the picture, the lines of the quote
/// `q` (money with the currency text `currency`) and lines to sign.
pub fn quote_sheet(d: &SheetData, q: &emb_model::quote::Quote, currency: &str, paper: Paper) -> Page {
    use crate::page::text_width_mm;
    use emb_model::quote::money_text;
    let mut p = Page::new(paper);
    let w = p.width_mm;
    let top = header(&mut p, d, "Quote");
    let pic = BBox::new(Point::new(MARGIN, top), Point::new(w - MARGIN, top + 90.0));
    let mut y = picture(&mut p, d, pic) + 10.0;
    let threads = d.colors.len();
    p.gray_text(
        MARGIN,
        y,
        9.5,
        format!("{} stitches · {threads} colors · {}", d.stitches, size_text(d.size)),
    );
    y += 8.0;
    let order: Vec<(&str, String)> = d.order_rows().into_iter().filter(|(k, _)| *k != "Quantity").collect();
    if !order.is_empty() {
        y = rows(&mut p, MARGIN, y, &order) + 2.0;
    }
    // Columns: the line, the quantity, the price of each, the amount (the
    // numbers right-aligned).
    let (x_qty, x_each, x_amount) = (w - MARGIN - 75.0, w - MARGIN - 35.0, w - MARGIN);
    let right = |p: &mut Page, x: f64, y: f64, bold: bool, t: String| {
        let tw = text_width_mm(&t, 10.0);
        p.text(x - tw, y, 10.0, bold, t);
    };
    p.text(MARGIN, y, 10.0, true, "Price");
    right(&mut p, x_qty, y, true, "Quantity".into());
    right(&mut p, x_each, y, true, "Each".into());
    right(&mut p, x_amount, y, true, "Amount".into());
    y += 2.0;
    p.line(Point::new(MARGIN, y), Point::new(w - MARGIN, y), 0.2, RULE);
    y += 6.0;
    for l in &q.lines {
        p.text(MARGIN, y, 10.0, false, l.kind.label());
        if l.kind.per_piece() {
            right(&mut p, x_qty, y, false, l.quantity.to_string());
            right(&mut p, x_each, y, false, money_text(currency, l.each));
        }
        right(&mut p, x_amount, y, false, money_text(currency, l.amount));
        y += 6.0;
    }
    if q.lines.is_empty() {
        p.gray_text(MARGIN, y, 10.0, "No prices are set.");
        y += 6.0;
    }
    y -= 4.0;
    p.line(Point::new(MARGIN, y), Point::new(w - MARGIN, y), 0.3, BLACK);
    y += 7.0;
    p.text(MARGIN, y, 12.0, true, "Total");
    let total = money_text(currency, q.total);
    p.text(x_amount - text_width_mm(&total, 12.0), y, 12.0, true, total);
    y += 6.0;
    p.gray_text(
        MARGIN,
        y,
        9.5,
        format!(
            "{} pieces · {} for each piece",
            q.quantity,
            money_text(currency, q.each)
        ),
    );
    y += 10.0;
    notes(&mut p, &d.notes_text(), y, 95);

    let h = p.height_mm;
    let sy = h - MARGIN - 6.0;
    p.line(Point::new(MARGIN, sy), Point::new(MARGIN + 95.0, sy), 0.3, BLACK);
    p.line(
        Point::new(w - MARGIN - 60.0, sy),
        Point::new(w - MARGIN, sy),
        0.3,
        BLACK,
    );
    p.gray_text(MARGIN, sy + 4.5, 9.0, "Accepted by");
    p.gray_text(w - MARGIN - 60.0, sy + 4.5, 9.0, "Date");
    p
}

/// The design properties as CSV: a header line and one line per design.
pub fn csv_header() -> &'static str {
    "name,width_mm,height_mm,stitches,colors,color_changes,trims,stops,sew_minutes,machine,hoop,threads,fabric,     order,customer,garment,quantity,placement,due"
}

/// One CSV line for the design (fields with commas or quotes are quoted).
pub fn csv_line(d: &SheetData) -> String {
    let q = |s: &str| {
        if s.contains([',', '"', '\n']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    let (w, h) = d.size.unwrap_or((0.0, 0.0));
    let threads = d
        .colors
        .iter()
        .map(|c| match &c.code {
            Some(code) => format!("{} {code}", c.name),
            None => c.name.clone(),
        })
        .collect::<Vec<_>>()
        .join("; ");
    [
        q(&d.name),
        format!("{w:.1}"),
        format!("{h:.1}"),
        d.stitches.to_string(),
        d.colors.len().to_string(),
        d.colors.len().saturating_sub(1).to_string(),
        d.trims.to_string(),
        d.stops.to_string(),
        d.minutes.map(|m| format!("{m:.1}")).unwrap_or_default(),
        q(d.machine.as_deref().unwrap_or("")),
        q(d.hoop.as_deref().unwrap_or("")),
        q(&threads),
        q(d.fabric.as_deref().unwrap_or("")),
        q(d.order.as_ref().map_or("", |o| o.number.trim())),
        q(d.order.as_ref().map_or("", |o| o.customer.trim())),
        q(&d.order.as_ref().map(|o| o.garment_text()).unwrap_or_default()),
        d.order
            .as_ref()
            .map(|o| o.total_quantity().to_string())
            .unwrap_or_default(),
        q(d.order.as_ref().map_or("", |o| o.placement.trim())),
        q(d.order.as_ref().map_or("", |o| o.due.trim())),
    ]
    .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SegmentKind;

    fn data(colors: usize) -> SheetData {
        SheetData {
            name: "Patch".into(),
            notes: "Use a medium tear-away stabilizer. Hoop the cap front tight.".into(),
            date: "2026-10-21".into(),
            machine: Some("Topaz 20".into()),
            hoop: Some("Crown 260 × 200".into()),
            files: vec!["patch.vp3".into(), "patch.embproj".into()],
            size: Some((72.0, 50.0)),
            stitches: 4586,
            trims: 3,
            minutes: Some(5.7),
            colors: (0..colors)
                .map(|i| SheetColor {
                    name: format!("Color {i}"),
                    brand: Some("Brother PEC".into()),
                    code: Some(i.to_string()),
                    color: Rgb(200, 30, (i * 10) as u8),
                    stitches: 100 + i,
                    length_mm: 1234.0,
                    thread_mm: 2400.0,
                    needle: None,
                    thread_change: false,
                })
                .collect(),
            segments: vec![Segment {
                a: Point::new(-36.0, -25.0),
                b: Point::new(36.0, 25.0),
                color: Rgb(200, 0, 0),
                kind: SegmentKind::Stitch,
            }],
            ..SheetData::default()
        }
    }

    fn texts(p: &Page) -> Vec<String> {
        p.ops
            .iter()
            .filter_map(|o| match o {
                Op::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn quote_sheet_has_the_lines_and_the_total() {
        use emb_model::quote::{Order, Piece, PriceList, quote};
        let list = PriceList {
            per_1000: 1.5,
            setup: 20.0,
            ..PriceList::default()
        };
        let q = quote(
            &list,
            Piece {
                stitches: 4586,
                colors: 3,
            },
            &Order {
                quantity: 12,
                ..Order::default()
            },
        );
        let page = quote_sheet(&data(3), &q, "$", Paper::Letter);
        let t = texts(&page);
        for want in [
            "Quote · 2026-10-21",
            "Embroidery",
            "Setup",
            "Total",
            "$6.88",
            "$82.56",
            "$102.56",
        ] {
            assert!(t.iter().any(|x| x == want), "{want} not in {t:?}");
        }
        assert!(t.iter().any(|x| x == "12 pieces · $8.55 for each piece"), "{t:?}");
        let empty = quote_sheet(
            &data(1),
            &quote(&PriceList::default(), Piece::default(), &Order::default()),
            "",
            Paper::A4,
        );
        assert!(texts(&empty).iter().any(|x| x == "No prices are set."));
    }

    #[test]
    fn production_sheet() {
        let pages = production(&data(3), Paper::Letter);
        assert_eq!(pages.len(), 1);
        let t = texts(&pages[0]);
        for want in [
            "Patch",
            "4586",
            "about 5:42",
            "Topaz 20",
            "patch.vp3",
            "Color 2",
            "Brother PEC 2",
            "2.4 m",
            "Notes",
        ] {
            assert!(t.iter().any(|x| x.contains(want)), "{want} in {t:?}");
        }
        // Everything is on the page.
        for o in &pages[0].ops {
            if let Op::Text { at, .. } = o {
                assert!(at.x >= 0.0 && at.x < pages[0].width_mm && at.y > 0.0 && at.y < pages[0].height_mm);
            }
        }
        assert!(pages[0].ops.iter().any(|o| matches!(o, Op::Stitches { .. })));
    }

    #[test]
    fn hoop_orientation_is_on_the_worksheet() {
        let mut d = data(2);
        d.hoop_layout = Some(HoopLayout {
            width_mm: 260.0,
            height_mm: 200.0,
            design: BBox::new(Point::new(-26.0, -45.0), Point::new(46.0, 5.0)),
            rotation_deg: 90,
            hoop_angle_deg: 0.0,
        });
        let pages = production(&d, Paper::A4);
        let t = texts(&pages[0]).join(" ");
        for want in [
            "Landscape: 260 mm from left to right, 200 mm from the back to the front",
            "Turned 90° clockwise",
            "right side of the hoop",
            "10 mm right and 20 mm up",
            "Back of machine",
        ] {
            assert!(t.contains(want), "{want} in {t}");
        }
        // The hoop outline: 46 mm wide, 35.4 mm high.
        let hoop = |o: &Op| match o {
            Op::Rect { rect, fill: None, .. } => {
                (rect.width() - 46.0).abs() < 1e-9 && (rect.height() - 200.0 * 46.0 / 260.0).abs() < 1e-9
            }
            _ => false,
        };
        assert!(pages[0].ops.iter().any(hoop));
        let upright = HoopLayout {
            rotation_deg: 0,
            design: BBox::new(Point::new(-10.0, -10.0), Point::new(10.0, 10.0)),
            ..d.hoop_layout.unwrap()
        };
        assert!(upright.design_text().starts_with("Upright"));
        assert!(upright.design_text().ends_with("at the hoop center."));
        // A turned hoop puts the design on a slant the other way.
        let slant = HoopLayout {
            hoop_angle_deg: 15.0,
            ..upright
        };
        assert!(
            slant
                .design_text()
                .ends_with("turned 15.0° counterclockwise in the hoop.")
        );
        assert!(
            (slant.up().x + emb_geom::math::sin_cos_deg(15.0).0).abs() < 1e-9,
            "{:?}",
            slant.up()
        );
        // A turned hooping (−90°) is a quarter turn, not a slant.
        let turned = HoopLayout {
            hoop_angle_deg: -90.0,
            ..upright
        };
        assert!(
            turned.design_text().starts_with("Turned 90° clockwise"),
            "{}",
            turned.design_text()
        );
        assert!(!turned.design_text().contains("slant"));
        assert!(turned.up().distance(Point::new(1.0, 0.0)) < 1e-9);
    }

    #[test]
    fn the_worksheet_lists_and_shows_the_rhinestones() {
        let d = SheetData {
            stones: vec![
                (Point::new(0.0, 0.0), 1.4, Rgb(200, 0, 0)),
                (Point::new(30.0, 0.0), 1.4, Rgb(200, 0, 0)),
            ],
            stone_counts: vec![("SS10 Red".into(), Rgb(200, 0, 0), 2)],
            ..data(2)
        };
        let pages = production(&d, Paper::Letter);
        let texts: Vec<&str> = pages
            .iter()
            .flat_map(|p| &p.ops)
            .filter_map(|o| match o {
                Op::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(
            texts.contains(&"Rhinestones (2, not sewn: use the template)"),
            "{texts:?}"
        );
        assert!(texts.contains(&"SS10 Red: 2"));
        let disks = pages[0]
            .ops
            .iter()
            .filter(|o| matches!(o, Op::Area { fill, .. } if *fill == Rgb(200, 0, 0)))
            .count();
        assert_eq!(disks, 2);
    }

    #[test]
    fn the_picture_shows_the_stones() {
        use emb_model::bling::{Bling, BlingLayout, Stone};
        let mut d = emb_model::Design::new("t");
        let th = d.add_thread("Stones", Rgb(0, 0, 0));
        d.add_object(
            th,
            emb_model::ObjectKind::Bling(Box::new(Bling::new(
                Stone::default(),
                BlingLayout::Stones {
                    points: vec![Point::new(0.0, 0.0), Point::new(20.0, 0.0)],
                },
            ))),
        );
        let areas = picture_areas(&d);
        assert_eq!(areas.len(), 2);
        // With no stitches, the picture is the box of the stones.
        let pic = crate::raster::picture(&[], &areas, 10.0, 0.4, 2.0, None);
        assert_eq!(pic.width, ((22.8 + 4.8) * 10.0_f64).ceil() as u32);
    }

    #[test]
    fn the_order_of_work_follows_the_marks() {
        use emb_model::bling::{Bling, BlingLayout, Stone};
        let mut d = emb_model::Design::new("t");
        let th = d.add_thread("Navy", Rgb(0, 0, 90));
        assert!(process_steps(&d, "t").is_empty(), "no stones, no steps");
        d.add_object(
            th,
            emb_model::ObjectKind::Running {
                path: emb_geom::Path::new()
                    .move_to(Point::new(-20.0, 0.0))
                    .line_to(Point::new(20.0, 0.0)),
                params: Default::default(),
            },
        );
        d.add_object(
            th,
            emb_model::ObjectKind::Bling(Box::new(Bling::new(
                Stone::default(),
                BlingLayout::Stones {
                    points: vec![Point::new(0.0, 10.0)],
                },
            ))),
        );
        let without = process_steps(&d, "logo");
        assert_eq!(without.len(), 4);
        assert!(without[1].contains("logo template.svg") && without[2].contains("Template marks"));
        d.add_template_marks();
        let with = process_steps(&d, "logo");
        assert_eq!(with.len(), 6);
        assert!(with[0].contains("four crosses") && with[5].contains("Pull out"));
        let pages = production(
            &SheetData {
                process_steps: with,
                ..data(2)
            },
            Paper::Letter,
        );
        let has = |t: &str| {
            pages
                .iter()
                .flat_map(|p| &p.ops)
                .any(|o| matches!(o, Op::Text { text, .. } if text == t))
        };
        assert!(has("Order of work"));
    }

    #[test]
    fn fabric_rows_push_the_color_table_down() {
        let base = production(&data(3), Paper::Letter);
        let d = SheetData {
            fabric: Some("Towel (terry)".into()),
            backing: Some("Tear-away, medium (cut-away for thin towels)".into()),
            topping: Some("Water-soluble film (necessary)".into()),
            colorway: Some("Navy".into()),
            files: (0..6).map(|i| format!("patch-{i}.vp3")).collect(),
            ..data(3)
        };
        let pages = production(&d, Paper::Letter);
        let t = texts(&pages[0]);
        for want in ["Towel (terry)", "Water-soluble film", "Backing"] {
            assert!(t.iter().any(|x| x.contains(want)), "{want} in {t:?}");
        }
        let head_y = |p: &Page| {
            p.ops
                .iter()
                .find_map(|o| match o {
                    Op::Text { at, text, .. } if text == "Brand and code" => Some(at.y),
                    _ => None,
                })
                .unwrap()
        };
        let last_file_y = pages[0]
            .ops
            .iter()
            .filter_map(|o| match o {
                Op::Text { at, text, .. } if text.starts_with("patch-") => Some(at.y),
                _ => None,
            })
            .fold(0.0, f64::max);
        assert!(head_y(&pages[0]) > last_file_y, "the table is below the files");
        assert!(head_y(&pages[0]) >= head_y(&base[0]));
        assert!(texts(&approval(&d, Paper::A4)).iter().any(|x| x == "Towel (terry)"));
        assert!(csv_line(&d).ends_with(",Towel (terry),,,,,,"));
    }

    #[test]
    fn stops_are_listed() {
        let d = SheetData {
            stop_notes: vec![
                "After color 1, #1 Patch: Put the appliqué fabric on".into(),
                "After color 1, #1 Patch: Cut away the appliqué fabric outside the tack-down".into(),
            ],
            ..data(2)
        };
        let pages = production(&d, Paper::Letter);
        assert_eq!(pages.len(), 1);
        let t = texts(&pages[0]);
        assert!(t.iter().any(|x| x == "Stops"));
        assert!(t.iter().any(|x| x.starts_with("2. After color 1")));
        // Many colors and stops: the stops go to the next page.
        let long = SheetData {
            stop_notes: d.stop_notes.clone(),
            ..data(26)
        };
        let pages = production(&long, Paper::Letter);
        assert!(texts(pages.last().unwrap()).iter().any(|x| x == "Stops"));
        for pg in &pages {
            for o in &pg.ops {
                if let Op::Text { at, .. } = o {
                    assert!(at.y < pg.height_mm - MARGIN + 7.0, "{at:?}");
                }
            }
        }
    }

    #[test]
    fn needles_are_listed() {
        // Three colors on two needles: color 3 changes the thread on needle 1.
        let plan = emb_model::NeedlePlan {
            count: 2,
            blocks: vec![1, 2, 1],
            changes: vec![2],
            start: vec![Some(0), Some(1)],
            loads: vec![2],
        };
        let mut d = data(3);
        d.set_needles(&plan);
        assert_eq!(d.stops, 1, "a thread change is a stop");
        assert_eq!(d.colors[2].needle, Some(1));
        assert!(d.colors[2].thread_change);
        assert_eq!(d.needles.len(), 2);
        assert!(!d.needles[0].put_on && d.needles[1].put_on);
        let pages = production(&d, Paper::Letter);
        assert_eq!(pages.len(), 1);
        let t = texts(&pages[0]);
        for want in [
            "Needle",
            "1 *",
            "Needles at the start",
            "Color 1 (Brother PEC 1) (put on)",
            "Needle sequence: 1, 2, 1",
            "Color 3: the machine stops. Put Color 2 (Brother PEC 2) on needle 1.",
        ] {
            assert!(t.iter().any(|x| x == want), "{want} in {t:?}");
        }
        // With no needle plan the table has no needle column.
        assert!(
            !texts(&production(&data(3), Paper::Letter)[0])
                .iter()
                .any(|x| x == "Needle")
        );
    }

    #[test]
    fn many_colors_continue_on_more_pages() {
        let pages = production(&data(40), Paper::A4);
        assert!(pages.len() >= 2);
        let last = texts(pages.last().unwrap());
        assert!(last.iter().any(|x| x.starts_with("Page ")));
        assert!(last.iter().any(|x| x == "Color 39"));
    }

    #[test]
    fn data_from_stitches() {
        use emb_model::{StitchDesign, StitchKind, StitchRecord};
        let r = |x: f64, k| StitchRecord::new(Point::new(x, 0.0), k);
        let s = StitchDesign {
            threads: vec![],
            records: vec![
                r(0.0, StitchKind::Jump),
                r(0.0, StitchKind::Stitch),
                r(3.0, StitchKind::Stitch),
                r(3.0, StitchKind::Trim),
                r(3.0, StitchKind::ColorChange),
                r(10.0, StitchKind::Jump),
                r(10.0, StitchKind::Stitch),
                r(14.0, StitchKind::Stitch),
                r(14.0, StitchKind::End),
            ],
        };
        let d = SheetData::from_stitches("x", &s);
        assert_eq!(d.stitches, 4);
        assert_eq!(d.trims, 1);
        assert_eq!(d.colors.len(), 2);
        assert_eq!((d.colors[0].length_mm, d.colors[1].length_mm), (3.0, 4.0));
        assert_eq!(d.colors[1].name, "Thread 2");
        assert_eq!(d.size, Some((14.0, 0.0)));
    }

    #[test]
    fn approval_sheet_and_csv() {
        let p = approval(&data(3), Paper::A4);
        let t = texts(&p);
        assert!(t.iter().any(|x| x == "Approved by"));
        assert!(t.iter().any(|x| x.contains("72.0 × 50.0 mm")));
        let line = csv_line(&SheetData {
            name: "A, \"B\"".into(),
            ..data(2)
        });
        assert!(
            line.starts_with("\"A, \"\"B\"\"\",72.0,50.0,4586,2,1,3,0,5.7,"),
            "{line}"
        );
        assert_eq!(
            csv_header().split(',').count(),
            line.split(',').count() - 1,
            "the name has one comma"
        );
    }

    fn with_order() -> SheetData {
        SheetData {
            order: Some(emb_model::order::OrderInfo {
                customer: "Riverside FC".into(),
                number: "A-1042".into(),
                garment: "Polo shirt".into(),
                quantity: 30,
                placement: "Left chest".into(),
                due: "2026-11-02".into(),
                notes: "Logo on the left side only.".into(),
                ..Default::default()
            }),
            price: Some("$12.13 for each piece, $363.90 for 30 pieces".into()),
            ..data(2)
        }
    }

    #[test]
    fn the_sheets_show_the_order() {
        let d = with_order();
        let work: Vec<String> = production(&d, Paper::A4).iter().flat_map(texts).collect();
        for want in [
            "A-1042",
            "Riverside FC",
            "Polo shirt",
            "Left chest",
            "2026-11-02",
            "Order: Logo on the left side only.",
        ] {
            assert!(work.iter().any(|x| x == want), "worksheet: {want} not in {work:?}");
        }
        let with_buy = SheetData {
            to_buy: vec!["Red (Madeira 1747): 2 spools, not in stock".into()],
            ..d.clone()
        };
        let work: Vec<String> = production(&with_buy, Paper::A4).iter().flat_map(texts).collect();
        assert!(work.iter().any(|x| x == "Threads to buy"));
        assert!(work.iter().any(|x| x == "Red (Madeira 1747): 2 spools, not in stock"));
        let appr = texts(&approval(&d, Paper::A4));
        for want in [
            "A-1042",
            "Price",
            "$12.13 for each piece, $363.90 for 30 pieces",
            "Approved by",
        ] {
            assert!(appr.iter().any(|x| x == want), "approval: {want} not in {appr:?}");
        }
        assert_eq!(d.barcode_text("rose-patch"), "A-1042");
        assert_eq!(data(1).barcode_text("rose-patch"), "rose-patch");
        assert_eq!(data(1).notes_text(), data(1).notes);
        let line = csv_line(&d);
        assert!(
            line.ends_with(",A-1042,Riverside FC,Polo shirt,30,Left chest,2026-11-02"),
            "{line}"
        );
        assert_eq!(csv_header().split(',').count(), line.split(',').count());
        // No order: empty fields.
        assert!(csv_line(&data(1)).ends_with(",,,,,,"));
    }

    #[test]
    fn the_worksheet_has_a_barcode_of_the_file_name() {
        let mut d = data(2);
        d.barcode = Some((crate::barcode::BarcodeKind::Code128, "rose-patch".into()));
        let pages = production(&d, Paper::Letter);
        let bars = pages[0]
            .ops
            .iter()
            .filter(|o| {
                matches!(
                    o,
                    Op::Rect {
                        fill: Some(Rgb(0, 0, 0)),
                        stroke: None,
                        ..
                    }
                )
            })
            .count();
        assert!(bars > 20, "{bars}");
        assert!(texts(&pages[0]).iter().any(|t| t == "rose-patch"));
        let none = production(&data(2), Paper::Letter);
        assert!(!texts(&none[0]).iter().any(|t| t == "rose-patch"));
    }
}
