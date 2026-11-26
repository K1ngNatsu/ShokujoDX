//! TrueType and OpenType fonts: names, metrics, glyph outlines and kerning.
//!
//! Units are font units with Y down (the font's Y axis is flipped), so the
//! baseline is at Y = 0 and capitals are at negative Y.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use emb_geom::{Path, Point, Transform, text};
use emb_model::{FontRef, ObjectKind, SatinGeometry, SatinParams};
use serde::{Deserialize, Serialize};
use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};
use ttf_parser::{Face, GlyphId, Tag};

use crate::glyph::GlyphShapes;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FontError {
    #[error("{}", text::fill("not a TrueType or OpenType font: {}", &[.0]))]
    Parse(String),
}

/// How the letters of a TrueType or OpenType font get their routes (P20
/// Milestone 4, ADR 0144): the order and the directions of their columns
/// with the fewest trims, and travel runs under the columns that sew later
/// (`route::route_with_splits`, as the embroidery fonts have them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteMode {
    /// The route of a letter is made when the letter is first laid out:
    /// the same letters each time (the tools and the tests).
    Wait,
    /// A thread makes the route of a letter that has none yet, and the
    /// letter has no route until it is ready (the app: typing stays quick).
    /// [`Font::routes_made`] counts the routes that are ready.
    Background,
    /// No routes.
    Off,
}

/// The routes of the letters of the TrueType fonts (of the computer and
/// the bundled ones), kept on the disk (ADR 0179): a route made in one
/// session is read in the next one, so a letter has its route at once. One
/// folder for each build of the program, in it one folder for each font (a
/// hash of its file, its face and the values of its axes), and one file for
/// each glyph. A build reads and writes only its own folder, because a
/// change of the stroke finder, of the tuning or of the route changes the
/// routes; the folders of other builds are deleted.
#[derive(Debug, Clone)]
pub struct RouteStore {
    dir: PathBuf,
}

/// The end of the name of a folder or a file of a route store that is
/// deleted.
const STORE_OLD: &str = ".old";

/// The largest file of a route store that is read: a route of 64 parts is
/// about 200 KB.
const MAX_STORED_BYTES: u64 = 8 << 20;

impl RouteStore {
    /// The store in the folder `dir` for the build `build` of the program
    /// (a text that changes with each build): the folder of the build in
    /// `dir` (a hash of `build`). The other folders and files in `dir` (of
    /// other builds) are moved out of the way and deleted on a thread. A
    /// program of another build that runs at the same time makes its
    /// folder again and writes there, not in this store (the audit of
    /// 2026-11-26: with one folder for all builds, it wrote its routes into
    /// the store of the new build). `None` if the folder cannot be used.
    pub fn open(dir: PathBuf, build: &str) -> Option<RouteStore> {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        build.hash(&mut h);
        let id = format!("{:016x}", h.finish());
        let mine = dir.join(&id);
        std::fs::create_dir_all(&mine).ok()?;
        let mut old = Vec::new();
        for e in std::fs::read_dir(&dir).ok()?.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name == id {
                continue;
            }
            // Left by a program that stopped before it deleted them.
            if name.ends_with(STORE_OLD) {
                old.push(e.path());
                continue;
            }
            let to = dir.join(format!("{name}.{}{STORE_OLD}", std::process::id()));
            if std::fs::rename(e.path(), &to).is_ok() {
                old.push(to);
            }
        }
        if !old.is_empty() {
            std::thread::spawn(move || {
                for p in old {
                    let _ = if p.is_dir() {
                        std::fs::remove_dir_all(p)
                    } else {
                        std::fs::remove_file(p)
                    };
                }
            });
        }
        Some(RouteStore { dir: mine })
    }
}

/// A part of a route in a route store: only its shape. A column has the
/// settings of the columns of a font of the computer, and a travel run the
/// settings of the route (so a damaged file cannot give other settings).
#[derive(Serialize, Deserialize)]
enum StoredPart {
    Column(SatinGeometry),
    Run(Path),
}

/// The glyph data and the caches of a font, which its route thread shares.
struct Shared {
    data: Arc<Vec<u8>>,
    index: u32,
    /// The values of the axes of a variable font (P20 Milestone 6: the
    /// Bold instance of a font that has its weights in one file).
    variations: Vec<(Tag, f32)>,
    units_per_em: f64,
    cap_height: f64,
    shapes: Mutex<HashMap<u16, Arc<GlyphShapes>>>,
    /// The character of each glyph (the lowest one that maps to it), for
    /// the split of its outline (ADR 0152). Made when first used.
    chars: std::sync::OnceLock<HashMap<u16, char>>,
    /// The route of each glyph that has one made (`None`: no columns to
    /// route).
    routes: Mutex<HashMap<u16, Option<Arc<Vec<ObjectKind>>>>>,
    /// The glyphs that wait for the route thread.
    todo: Mutex<Vec<u16>>,
    /// The folder of the font in a route store, if it has one.
    store: Mutex<Option<PathBuf>>,
    working: AtomicBool,
    made: AtomicU64,
    mode: AtomicU8,
}

impl Shared {
    fn face(&self) -> Face<'_> {
        let mut face = Face::parse(&self.data, self.index).expect("parsed in from_bytes");
        for (axis, value) in &self.variations {
            face.set_variation(*axis, *value);
        }
        face
    }

    /// The character of a glyph: the lowest character that the Unicode
    /// tables of the font map to it. `None` for a glyph that only shaping
    /// gives (a ligature, a form in a word).
    fn char_of(&self, glyph: u16) -> Option<char> {
        self.chars
            .get_or_init(|| {
                let mut map: HashMap<u16, char> = HashMap::new();
                let face = self.face();
                if let Some(cmap) = face.tables().cmap {
                    for table in cmap.subtables {
                        if !table.is_unicode() {
                            continue;
                        }
                        table.codepoints(|cp| {
                            if let (Some(c), Some(g)) = (char::from_u32(cp), table.glyph_index(cp)) {
                                let e = map.entry(g.0).or_insert(c);
                                if c < *e {
                                    *e = c;
                                }
                            }
                        });
                    }
                }
                map
            })
            .get(&glyph)
            .copied()
    }

    fn contours(&self, glyph: u16) -> Vec<Vec<Point>> {
        let mut b = Builder::default();
        if self.face().outline_glyph(GlyphId(glyph), &mut b).is_none() {
            return Vec::new();
        }
        let tol = self.units_per_em / 2000.0;
        let rings: Vec<Vec<Point>> = b
            .path
            .flatten(tol)
            .into_iter()
            .map(|pl| {
                let mut p = pl.points;
                if p.len() > 1 && p.first() == p.last() {
                    p.pop();
                }
                p
            })
            .filter(|p| p.len() >= 3)
            .collect();
        emb_geom::region::without_overlaps(&rings)
    }

    fn shapes(&self, glyph: u16) -> Arc<GlyphShapes> {
        if let Some(s) = self.shapes.lock().ok().and_then(|c| c.get(&glyph).cloned()) {
            return s;
        }
        let split = self
            .char_of(glyph)
            .map_or(crate::stroke::Split::Off, crate::stroke::Split::for_char);
        let s = Arc::new(GlyphShapes::with_split(&self.contours(glyph), self.cap_height, split));
        if let Ok(mut c) = self.shapes.lock() {
            c.insert(glyph, s.clone());
        }
        s
    }

    /// The columns of a glyph in the order and the directions of their
    /// route, with travel runs, in font units. The route is made at the
    /// capital height of the embroidery fonts. `None`: no columns.
    fn make_route(&self, glyph: u16) -> Option<Arc<Vec<ObjectKind>>> {
        let shapes = self.shapes(glyph);
        if shapes.strokes.satins.is_empty() {
            return None;
        }
        let s = crate::embfont::DESIGN_CAP_MM / self.cap_height;
        let mut parts = shapes.satin_parts();
        for p in &mut parts {
            p.transform(&Transform::scale(s, s));
        }
        // The steps of the converter (`EmbFont::extend_ends`, P21 Milestone
        // 6): round dots, and ends of columns on to the corners of the
        // outline where that adds no needle point out of it. Not for kana
        // and kanji, as in the converter (`EmbFont::extend_ends`): it makes
        // the ends that stop at a crossing long again.
        if !self.char_of(glyph).is_some_and(crate::layout::is_cjk) {
            let scale = |r: &Vec<Point>| -> Vec<Point> { r.iter().map(|p| Point::new(p.x * s, p.y * s)).collect() };
            let fills: Vec<(Vec<Point>, Vec<Vec<Point>>)> = shapes
                .fills
                .iter()
                .map(|(outer, holes)| (scale(outer), holes.iter().map(scale).collect()))
                .collect();
            crate::tune::round_parts(&mut parts, &fills, crate::embfont::DESIGN_CAP_MM);
            crate::tune::extend_ends(&mut parts, &fills, crate::embfont::DESIGN_CAP_MM);
        }
        let advance = f64::from(self.face().glyph_hor_advance(GlyphId(glyph)).unwrap_or(0));
        let letter = crate::embfont::EmbLetter {
            advance_mm: advance * s,
            parts,
            fixed: false,
            tuning: Vec::new(),
            fill_parts: Vec::new(),
        };
        let outline: Vec<Vec<Point>> = self
            .contours(glyph)
            .into_iter()
            .map(|r| r.into_iter().map(|p| Point::new(p.x * s, p.y * s)).collect())
            .collect();
        let rtl = self.char_of(glyph).is_some_and(crate::route::is_rtl);
        let (routed, _) = crate::route::route_with_splits_dir(
            &letter,
            crate::embfont::DESIGN_CAP_MM,
            Some(&outline),
            rtl,
            crate::route::threads(),
        );
        let mut parts = routed.parts;
        for p in &mut parts {
            p.transform(&Transform::scale(1.0 / s, 1.0 / s));
        }
        Some(Arc::new(parts))
    }

    /// The route of a glyph from the route store: `None` if the store has
    /// none (or a file that is damaged or not of a route); `Some(None)` for
    /// a glyph with no columns.
    fn stored(&self, glyph: u16) -> Option<Option<Arc<Vec<ObjectKind>>>> {
        let dir = self.store.lock().ok()?.clone()?;
        let file = dir.join(format!("{glyph}.json"));
        if std::fs::metadata(&file).ok()?.len() > MAX_STORED_BYTES {
            return None;
        }
        let bytes = std::fs::read(file).ok()?;
        let parts: Option<Vec<StoredPart>> = serde_json::from_slice(&bytes).ok()?;
        let Some(parts) = parts else {
            return Some(None);
        };
        // A route has parts (else a letter would sew nothing), fewer than
        // this, in the box of the outline of its glyph and one em round it.
        let rings = self.contours(glyph);
        let mut points = rings.iter().flatten();
        let first = *points.next()?;
        let (lo, hi) = points.fold((first, first), |(lo, hi), p| {
            (
                Point::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        });
        let m = self.units_per_em;
        let fits = |p: &Point| {
            p.x.is_finite()
                && p.y.is_finite()
                && (lo.x - m..=hi.x + m).contains(&p.x)
                && (lo.y - m..=hi.y + m).contains(&p.y)
        };
        let ok = !parts.is_empty()
            && parts.len() <= 4096
            && parts.iter().all(|part| match part {
                StoredPart::Column(g) => {
                    g.rail_a.points().all(|p| fits(&p))
                        && g.rail_b.points().all(|p| fits(&p))
                        && g.rungs.iter().all(|(a, b)| fits(a) && fits(b))
                }
                StoredPart::Run(path) => path.points().all(|p| fits(&p)),
            });
        if !ok {
            return None;
        }
        let parts = parts
            .into_iter()
            .map(|part| match part {
                StoredPart::Column(geometry) => ObjectKind::Satin {
                    geometry,
                    params: SatinParams::default(),
                },
                StoredPart::Run(path) => ObjectKind::Running {
                    path,
                    params: crate::route::run_params(),
                },
            })
            .collect();
        Some(Some(Arc::new(parts)))
    }

    /// Writes the route of a glyph to the route store (if the font has one).
    /// A route with a part that the store cannot keep is not written.
    fn keep(&self, glyph: u16, route: &Option<Arc<Vec<ObjectKind>>>) {
        let Some(dir) = self.store.lock().ok().and_then(|s| s.clone()) else {
            return;
        };
        let parts: Option<Option<Vec<StoredPart>>> = match route {
            None => Some(None),
            Some(parts) => parts
                .iter()
                .map(|p| match p {
                    ObjectKind::Satin { geometry, params } if *params == SatinParams::default() => {
                        Some(StoredPart::Column(geometry.clone()))
                    }
                    ObjectKind::Running { path, params } if *params == crate::route::run_params() => {
                        Some(StoredPart::Run(path.clone()))
                    }
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
                .map(Some),
        };
        let Some(Ok(bytes)) = parts.map(|p| serde_json::to_vec(&p)) else {
            return;
        };
        // A whole file or none: written to a file of its own, then moved;
        // a file that is not moved is deleted.
        let tmp = dir.join(format!("{glyph}.{}.tmp", std::process::id()));
        if std::fs::create_dir_all(&dir).is_ok()
            && (std::fs::write(&tmp, bytes).is_err()
                || std::fs::rename(&tmp, dir.join(format!("{glyph}.json"))).is_err())
        {
            let _ = std::fs::remove_file(&tmp);
        }
    }

    /// The route of a glyph, made now and kept in the route store (the
    /// caller looks in the store first).
    fn route_of(&self, glyph: u16) -> Option<Arc<Vec<ObjectKind>>> {
        let r = self.make_route(glyph);
        self.keep(glyph, &r);
        r
    }

    fn mode(&self) -> RouteMode {
        match self.mode.load(Ordering::Relaxed) {
            0 => RouteMode::Wait,
            1 => RouteMode::Background,
            _ => RouteMode::Off,
        }
    }

    /// Starts the route thread if it does not run.
    fn start(self: &Arc<Self>) {
        if self.working.swap(true, Ordering::AcqRel) {
            return;
        }
        let me = self.clone();
        std::thread::spawn(move || {
            loop {
                let next = me.todo.lock().ok().and_then(|mut t| t.pop());
                match next {
                    Some(g) => {
                        // A fault in the route of one letter does not stop
                        // the thread: that letter keeps no route, and the
                        // other letters get theirs (the audit of P20).
                        let route =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| me.route_of(g))).unwrap_or(None);
                        if let Ok(mut r) = me.routes.lock() {
                            r.insert(g, route);
                        }
                        me.made.fetch_add(1, Ordering::AcqRel);
                    }
                    None => {
                        me.working.store(false, Ordering::Release);
                        // A glyph that came after the last look, and no
                        // other thread took it: go on.
                        let more = me.todo.lock().is_ok_and(|t| !t.is_empty());
                        if !(more && !me.working.swap(true, Ordering::AcqRel)) {
                            break;
                        }
                    }
                }
            }
        });
    }
}

/// One font face, with a cache of the glyph shapes made from it and of the
/// routes of its letters.
pub struct Font {
    shared: Arc<Shared>,
    pub family: String,
    pub style: String,
    pub units_per_em: f64,
    /// Height of the capitals (of "H").
    pub cap_height: f64,
    /// The height of the Japanese, Chinese or Korean characters, if the
    /// font has them ([`character_height_of`]).
    pub character_height: Option<f64>,
    pub ascender: f64,
    /// Negative: below the baseline.
    pub descender: f64,
    pub line_gap: f64,
}

impl std::fmt::Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Font({} {})", self.family, self.style)
    }
}

/// The number of faces in a font file (a collection has more than one).
pub fn face_count(data: &[u8]) -> u32 {
    ttf_parser::fonts_in_collection(data).unwrap_or(1)
}

/// The best name for `ids` (in order of preference): English first, then any.
fn name(face: &Face, ids: &[u16]) -> Option<String> {
    for &id in ids {
        let mut any = None;
        for n in face.names() {
            if n.name_id != id {
                continue;
            }
            if let Some(s) = n.to_string().filter(|s| !s.trim().is_empty()) {
                if n.language_id == 0x0409 {
                    return Some(s);
                }
                any.get_or_insert(s);
            }
        }
        if any.is_some() {
            return any;
        }
    }
    None
}

/// The family and style names of a face, without reading its glyphs.
pub fn face_names(data: &[u8], index: u32) -> Option<(String, String)> {
    let face = Face::parse(data, index).ok()?;
    let family = name(&face, &[16, 1])?;
    let style = name(&face, &[17, 2]).unwrap_or_else(|| "Regular".to_string());
    Some((family, style))
}

impl Font {
    pub fn from_bytes(data: Vec<u8>, index: u32) -> Result<Font, FontError> {
        Font::from_bytes_with(data, index, &[])
    }

    /// [`Font::from_bytes`] for a variable font at the values `variations`
    /// of its axes (for example `("wght", 700.0)`, the Bold instance). An
    /// axis that the font does not have is not used.
    pub fn from_bytes_with(data: Vec<u8>, index: u32, variations: &[(&str, f32)]) -> Result<Font, FontError> {
        let data = Arc::new(data);
        let mut face = Face::parse(&data, index).map_err(|e| FontError::Parse(e.to_string()))?;
        let variations: Vec<(Tag, f32)> = variations
            .iter()
            .map(|(axis, value)| (Tag::from_bytes_lossy(axis.as_bytes()), *value))
            .filter(|(axis, _)| face.variation_axes().into_iter().any(|a| a.tag == *axis))
            .collect();
        for (axis, value) in &variations {
            face.set_variation(*axis, *value);
        }
        // Typographic family and subfamily first (16, 17), then the legacy names (1, 2).
        let family = name(&face, &[16, 1]).unwrap_or_else(|| "Unnamed".to_string());
        let style = name(&face, &[17, 2]).unwrap_or_else(|| "Regular".to_string());
        let units_per_em = f64::from(face.units_per_em().max(1));
        let h_top = face
            .glyph_index('H')
            .and_then(|h| face.glyph_bounding_box(h))
            .map(|b| f64::from(b.y_max));
        let cap_height = cap_height_of(face.capital_height().map(f64::from), h_top, units_per_em);
        let character_height = character_height_of(|c| {
            face.glyph_index(c)
                .and_then(|g| face.glyph_bounding_box(g))
                .map(|b| f64::from(b.y_max) - f64::from(b.y_min))
        });
        let (ascender, descender, line_gap) = (
            f64::from(face.ascender()),
            f64::from(face.descender()),
            f64::from(face.line_gap()),
        );
        Ok(Font {
            shared: Arc::new(Shared {
                data,
                index,
                variations,
                units_per_em,
                cap_height,
                shapes: Mutex::new(HashMap::new()),
                chars: std::sync::OnceLock::new(),
                routes: Mutex::new(HashMap::new()),
                todo: Mutex::new(Vec::new()),
                store: Mutex::new(None),
                working: AtomicBool::new(false),
                made: AtomicU64::new(0),
                mode: AtomicU8::new(0),
            }),
            family,
            style,
            units_per_em,
            cap_height,
            character_height,
            ascender,
            descender,
            line_gap,
        })
    }

    fn face(&self) -> Face<'_> {
        self.shared.face()
    }

    /// How the letters get their routes (`Wait` when the font is read).
    pub fn set_route_mode(&self, mode: RouteMode) {
        let v = match mode {
            RouteMode::Wait => 0,
            RouteMode::Background => 1,
            RouteMode::Off => 2,
        };
        self.shared.mode.store(v, Ordering::Relaxed);
    }

    /// Keeps the routes of this font in `store` (ADR 0179): a route that
    /// is in the store is read, not made, and a route that is made is
    /// written to it.
    pub fn set_route_store(&self, store: &RouteStore) {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.shared.data.as_slice().hash(&mut h);
        self.shared.index.hash(&mut h);
        for (axis, value) in &self.shared.variations {
            axis.to_bytes().hash(&mut h);
            value.to_bits().hash(&mut h);
        }
        if let Ok(mut s) = self.shared.store.lock() {
            *s = Some(store.dir.join(format!("{:016x}", h.finish())));
        }
    }

    /// The route of a glyph: its columns in the order and the directions
    /// with the fewest trims, with travel runs, in font units. With
    /// [`RouteMode::Wait`] it is made now if it is not made yet; with
    /// `Background` a thread makes it, and it is `None` until it is ready.
    /// A route in the route store of the font is read at once in both.
    /// `None` also for a glyph with no columns.
    pub fn route(&self, glyph: u16) -> Option<Arc<Vec<ObjectKind>>> {
        if let Some(r) = self.shared.routes.lock().ok().and_then(|r| r.get(&glyph).cloned()) {
            return r;
        }
        if self.shared.mode() != RouteMode::Off
            && let Some(r) = self.shared.stored(glyph)
        {
            if let Ok(mut m) = self.shared.routes.lock() {
                m.insert(glyph, r.clone());
            }
            return r;
        }
        match self.shared.mode() {
            RouteMode::Wait => {
                let r = self.shared.route_of(glyph);
                if let Ok(mut m) = self.shared.routes.lock() {
                    m.insert(glyph, r.clone());
                }
                r
            }
            RouteMode::Background => {
                if let Ok(mut t) = self.shared.todo.lock()
                    && !t.contains(&glyph)
                {
                    t.push(glyph);
                }
                self.shared.start();
                None
            }
            RouteMode::Off => None,
        }
    }

    /// `true` if the route of the glyph is made (no route is made now).
    pub fn has_route(&self, glyph: u16) -> bool {
        self.shared
            .routes
            .lock()
            .is_ok_and(|r| r.get(&glyph).is_some_and(Option::is_some))
    }

    /// The number of routes that the route thread made: a change means that
    /// letters laid out before can have their routes now.
    pub fn routes_made(&self) -> u64 {
        self.shared.made.load(Ordering::Acquire)
    }

    /// `true` while the route thread has glyphs to route.
    pub fn routing(&self) -> bool {
        self.shared.working.load(Ordering::Acquire) || self.shared.todo.lock().is_ok_and(|t| !t.is_empty())
    }

    pub fn font_ref(&self) -> FontRef {
        FontRef::new(&self.family, &self.style)
    }

    /// The distance between two baselines.
    pub fn line_height(&self) -> f64 {
        let h = self.ascender - self.descender + self.line_gap;
        if h > 0.0 { h } else { 1.2 * self.units_per_em }
    }

    pub fn glyph(&self, c: char) -> Option<u16> {
        self.face().glyph_index(c).map(|g| g.0).filter(|&g| g != 0)
    }

    pub fn advance(&self, glyph: u16) -> f64 {
        f64::from(self.face().glyph_hor_advance(GlyphId(glyph)).unwrap_or(0))
    }

    /// The outline of a glyph as closed rings (flattened curves), Y down.
    /// Rings that overlap (the strokes of a variable font) are merged with
    /// the nonzero rule of the font (`region::without_overlaps`, P20), so
    /// that each ring is an outer ring or a hole.
    pub fn contours(&self, glyph: u16) -> Vec<Vec<Point>> {
        self.shared.contours(glyph)
    }

    /// The horizontal kerning between two glyphs (usually negative): the
    /// OpenType pair positioning of the "kern" feature, else the `kern` table.
    pub fn kerning(&self, left: u16, right: u16) -> f64 {
        let face = self.face();
        let (l, r) = (GlyphId(left), GlyphId(right));
        if let Some(v) = gpos_kerning(&face, l, r) {
            return f64::from(v);
        }
        face.tables()
            .kern
            .and_then(|k| {
                k.subtables
                    .into_iter()
                    .filter(|s| s.horizontal && !s.variable)
                    .find_map(|s| s.glyphs_kerning(l, r))
            })
            .map_or(0.0, f64::from)
    }

    /// The glyphs of one line of text, shaped with the tables of the font
    /// (see [`crate::shape`]).
    pub fn shape_line(&self, line: &str, kerning: bool) -> Option<Vec<crate::shape::ShapedGlyph>> {
        crate::shape::shape_line(&self.shared.data, self.shared.index, line, kerning)
    }

    /// The copyright notice of the font (name 0).
    pub fn copyright(&self) -> Option<String> {
        name(&self.face(), &[0])
    }

    /// The license description of the font (name 13).
    pub fn license(&self) -> Option<String> {
        name(&self.face(), &[13])
    }

    /// The typical stroke width as a part of the capital height: the middle
    /// value over a few plain letters.
    pub fn stroke_ratio(&self) -> Option<f64> {
        let mut v: Vec<f64> = "IlHnoiu"
            .chars()
            .filter_map(|c| self.glyph(c))
            .map(|g| self.shapes(g).strokes.typical_width)
            .filter(|w| *w > 0.0)
            .collect();
        v.sort_by(f64::total_cmp);
        Some(*v.get(v.len() / 2)? / self.cap_height)
    }

    /// The shapes of a glyph (cached).
    pub fn shapes(&self, glyph: u16) -> Arc<GlyphShapes> {
        self.shared.shapes(glyph)
    }
}

/// The x-advance change of the first glyph of a pair in the lookups of the
/// "kern" feature. `None` if no subtable has the pair.
fn gpos_kerning(face: &Face, left: GlyphId, right: GlyphId) -> Option<i16> {
    let gpos = face.tables().gpos?;
    let kern = Tag::from_bytes(b"kern");
    let mut seen = Vec::new();
    for feature in gpos.features.into_iter().filter(|f| f.tag == kern) {
        for index in feature.lookup_indices {
            if seen.contains(&index) {
                continue;
            }
            seen.push(index);
            let Some(lookup) = gpos.lookups.get(index) else {
                continue;
            };
            for k in 0..lookup.subtables.len() {
                let Some(PositioningSubtable::Pair(pair)) = lookup.subtables.get::<PositioningSubtable>(k) else {
                    continue;
                };
                match pair {
                    PairAdjustment::Format1 { coverage, sets } => {
                        if let Some(set) = coverage.get(left).and_then(|i| sets.get(i))
                            && let Some((v, _)) = set.get(right)
                        {
                            return Some(v.x_advance);
                        }
                    }
                    PairAdjustment::Format2 {
                        coverage,
                        classes,
                        matrix,
                    } => {
                        if coverage.contains(left)
                            && let Some((v, _)) = matrix.get((classes.0.get(left), classes.1.get(right)))
                        {
                            return Some(v.x_advance);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Collects a glyph outline as a path with Y down.
#[derive(Default)]
struct Builder {
    path: Path,
    last: Point,
}

fn pt(x: f32, y: f32) -> Point {
    Point::new(f64::from(x), -f64::from(y))
}

impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.last = pt(x, y);
        self.path = std::mem::take(&mut self.path).move_to(self.last);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.last = pt(x, y);
        self.path = std::mem::take(&mut self.path).line_to(self.last);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        // The cubic with the same curve as the quadratic.
        let (p0, q, p) = (self.last, pt(x1, y1), pt(x, y));
        let c1 = p0 + (q - p0) * (2.0 / 3.0);
        let c2 = p + (q - p) * (2.0 / 3.0);
        self.last = p;
        self.path = std::mem::take(&mut self.path).cubic_to(c1, c2, p);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.last = pt(x, y);
        self.path = std::mem::take(&mut self.path).cubic_to(pt(x1, y1), pt(x2, y2), self.last);
    }

    fn close(&mut self) {
        self.path = std::mem::take(&mut self.path).close();
    }
}

/// Kanji that fill the box of the characters (the ideographic character
/// face): their middle height is the height of the characters.
pub const HEIGHT_KANJI: &str = "国永東字書漢日";

/// The middle height of the kanji of [`HEIGHT_KANJI`] that `height` gives
/// (the height of a character, if the source has it); `None` with fewer than
/// three of them.
pub fn character_height_of(height: impl Fn(char) -> Option<f64>) -> Option<f64> {
    let mut v: Vec<f64> = HEIGHT_KANJI.chars().filter_map(height).filter(|h| *h > 0.0).collect();
    if v.len() < 3 {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

/// The height of the capitals in font units: the value of the font (OS/2),
/// else the top of 'H', else 0.7 em. Some fonts have a wrong value (for
/// example 0.15 em): a value out of 0.3 to 1.2 em is not used.
fn cap_height_of(declared: Option<f64>, h_top: Option<f64>, units_per_em: f64) -> f64 {
    let ok = |v: &f64| (0.3 * units_per_em..=1.2 * units_per_em).contains(v);
    declared
        .filter(ok)
        .or_else(|| h_top.filter(ok))
        .unwrap_or(0.7 * units_per_em)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_glyph_knows_its_character_for_the_split() {
        let f = crate::bundled::font("Anton", "Regular").expect("bundled font");
        for c in ['A', 'g', '4'] {
            let g = f.glyph(c).expect("glyph");
            assert_eq!(f.shared.char_of(g), Some(c));
        }
        assert_eq!(f.shared.char_of(0), None);
    }

    #[test]
    fn a_wrong_cap_height_is_not_used() {
        use super::cap_height_of;
        assert_eq!(cap_height_of(Some(700.0), Some(710.0), 1000.0), 700.0);
        // A font that declares 301 of 2048 units (as Courgette): the 'H'.
        assert_eq!(cap_height_of(Some(301.0), Some(1434.0), 2048.0), 1434.0);
        assert_eq!(cap_height_of(Some(0.0), None, 1000.0), 700.0);
        assert_eq!(cap_height_of(None, Some(5000.0), 1000.0), 700.0);
    }

    use crate::bundled;

    #[test]
    fn names_metrics_and_outlines() {
        let f = bundled::font("Lato", "Bold").unwrap();
        assert_eq!((f.family.as_str(), f.style.as_str()), ("Lato", "Bold"));
        assert!(f.cap_height > 0.6 * f.units_per_em && f.cap_height < 0.8 * f.units_per_em);
        let o = f.glyph('O').unwrap();
        assert_eq!(f.contours(o).len(), 2, "outer ring and counter");
        // Capitals are above the baseline: negative Y.
        let top = f
            .contours(f.glyph('H').unwrap())
            .iter()
            .flatten()
            .map(|p| p.y)
            .fold(0.0, f64::min);
        assert!((top + f.cap_height).abs() < 2.0, "{top}");
        assert!(f.advance(o) > 0.0);
        assert!(f.glyph('\u{10FFFF}').is_none());
    }

    /// A folder of its own for a test of a route store, in the temporary
    /// folder of the system.
    fn store_dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("shokujodx-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// The routes of a font of the computer are kept on the disk (ADR
    /// 0179): a font read again has them at once, from the store.
    #[test]
    fn routes_are_kept_on_the_disk() {
        let dir = store_dir("routes");
        let store = super::RouteStore::open(dir.join("routes"), "build 1").unwrap();
        let bytes = bundled::FILES
            .iter()
            .find(|f| f.0 == "Lato-Bold.ttf")
            .unwrap()
            .1
            .to_vec();
        let first = super::Font::from_bytes(bytes.clone(), 0).unwrap();
        first.set_route_store(&store);
        let h = first.glyph('H').unwrap();
        let made = first.route(h).expect("a route");
        let files: Vec<_> = std::fs::read_dir(&store.dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_dir())
            .collect();
        assert_eq!(files.len(), 1, "one folder for the font");
        let file = files[0].path().join(format!("{h}.json"));
        assert!(file.exists());
        // The same font read again: the route from the store, the same.
        let again = || {
            let f = super::Font::from_bytes(bytes.clone(), 0).unwrap();
            f.set_route_store(&store);
            f.route(h)
        };
        assert_eq!(again(), Some(made.clone()));
        // It is read, not made: a route in the file with one part less.
        let text = std::fs::read_to_string(&file).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v.as_array_mut().unwrap().pop();
        std::fs::write(&file, v.to_string()).unwrap();
        assert_eq!(again().map(|r| r.len()), Some(made.len() - 1));
        // A damaged file, a route with a point far out, a route with no
        // parts and a file that is too large: the route is made again.
        fn far_out(v: &mut serde_json::Value) -> bool {
            match v {
                serde_json::Value::Number(_) => {
                    *v = serde_json::json!(1e30);
                    true
                }
                serde_json::Value::Array(a) => a.iter_mut().any(far_out),
                serde_json::Value::Object(o) => o.values_mut().any(far_out),
                _ => false,
            }
        }
        let mut far: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(far_out(&mut far));
        // The file with the far point is a route in the form of the store.
        assert!(serde_json::from_value::<Option<Vec<super::StoredPart>>>(far.clone()).is_ok());
        let large = format!("[{}]", vec![r#"{"Run":[{"M":[0,0]}]}"#; 450_000].join(","));
        assert!(large.len() as u64 > super::MAX_STORED_BYTES);
        for bad in ["[{\"Column\": 3".to_string(), far.to_string(), "[]".into(), large] {
            std::fs::write(&file, &bad).unwrap();
            assert_eq!(again(), Some(made.clone()), "{bad}");
        }
        // Another font file has its own folder.
        let other = bundled::FILES
            .iter()
            .find(|f| f.0 == "Lato-Regular.ttf")
            .unwrap()
            .1
            .to_vec();
        let f = super::Font::from_bytes(other, 0).unwrap();
        f.set_route_store(&store);
        f.route(f.glyph('H').unwrap());
        assert_eq!(
            std::fs::read_dir(&store.dir)
                .unwrap()
                .flatten()
                .filter(|e| e.path().is_dir())
                .count(),
            2
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A route store of another build is not used: its routes can be of
    /// another stroke finder or route. A program of the old build that runs
    /// on writes in its own folder, not in the new store.
    #[test]
    fn a_store_of_another_build_is_not_used() {
        let dir = store_dir("build");
        let routes = dir.join("routes");
        let one = super::RouteStore::open(routes.clone(), "build 1").unwrap();
        let f = bundled::font("Lato", "Bold").unwrap();
        f.set_route_store(&one);
        f.route(f.glyph('H').unwrap());
        let fonts = |store: &super::RouteStore| {
            std::fs::read_dir(&store.dir).map_or(0, |d| d.flatten().filter(|e| e.path().is_dir()).count())
        };
        assert_eq!(fonts(&one), 1);
        // The same build: the store stays.
        let again = super::RouteStore::open(routes.clone(), "build 1").unwrap();
        assert_eq!((again.dir.clone(), fonts(&again)), (one.dir.clone(), 1));
        // Another build: an empty store, and the store of the other build
        // is out of the way.
        let two = super::RouteStore::open(routes.clone(), "build 2").unwrap();
        assert_ne!(two.dir, one.dir);
        assert_eq!(fonts(&two), 0);
        assert!(!one.dir.exists());
        // The program of build 1 runs on: its routes go to its own folder.
        f.route(f.glyph('E').unwrap());
        assert_eq!(fonts(&two), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn routes_wait_or_come_from_a_thread() {
        // P20 Milestone 4: the route of a letter with columns.
        use emb_model::ObjectKind;
        let f = bundled::font("Lato", "Bold").unwrap();
        let h = f.glyph('H').unwrap();
        let routed = f.route(h).expect("a route, made now");
        assert!(f.has_route(h));
        assert!(routed.iter().any(|p| matches!(p, ObjectKind::Satin { .. })));
        // In the background: no route at first, then the thread makes it.
        let f = bundled::font("Lato", "Bold").unwrap();
        f.set_route_mode(super::RouteMode::Background);
        let e = f.glyph('E').unwrap();
        let made = f.routes_made();
        let first = f.route(e);
        let start = std::time::Instant::now();
        while f.routing() {
            assert!(start.elapsed().as_secs() < 30, "the route is not made");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(first.is_none() || f.has_route(e));
        assert!(f.has_route(e));
        assert!(f.routes_made() > made || first.is_some());
        assert_eq!(f.route(e), f.route(e), "kept for the font");
        // No routes.
        let f = bundled::font("Lato", "Bold").unwrap();
        f.set_route_mode(super::RouteMode::Off);
        assert!(f.route(h).is_none() && !f.has_route(h));
    }

    #[test]
    fn kerning_pairs() {
        let f = bundled::font("Lato", "Regular").unwrap();
        let (a, v) = (f.glyph('A').unwrap(), f.glyph('V').unwrap());
        assert!(f.kerning(a, v) < 0.0, "A V kerns closer: {}", f.kerning(a, v));
        let (h, i) = (f.glyph('H').unwrap(), f.glyph('I').unwrap());
        assert_eq!(f.kerning(h, i), 0.0);
    }
}
