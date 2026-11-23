//! The canvas on tiles (P18 view plan, Milestone 5, ADR 0126).
//!
//! The 3D thread look of the canvas shows the surface of the stitches
//! (`emb_render::surface` and `emb_render::sheen`, ADR 0124 and 0125) as
//! tiles: textures of 256 × 256 pixels on one grid of pixels for each zoom
//! level (level `L` has 2^L pixels for each mm). One cache keeps the tiles of
//! all open designs and of all views (the canvas, the split view, the views
//! of View > New view, the other design and the overview), by design, lay,
//! light, shown stitches, level and place, in a budget of memory
//! ([`BUDGET`], ADR 0077); the tiles that were not used for the longest time
//! go first.
//!
//! - **The lay**: a thread lays the stitches of each new stitch plan
//!   (`surface::relay` from the lay before, so only the stitches that the
//!   change reaches are laid again). While it works, the tiles near the
//!   objects that changed show the look of today.
//! - **The tiles**: threads make the tiles that the views need: the level
//!   of the screen first, from the middle of the view out, then the levels
//!   under and over it. A view shows a tile of another level until the
//!   right one is there, and the look of today where no tile is there.
//! - **A change** (a new lay, another view filter, a step of the stitch
//!   player, a drag) keeps the tiles that it does not reach; only the
//!   others are made again.
//! - **A first view**: where no tile of another level can show, a draft
//!   with half the detail ([`DRAFT_PX_PER_MM`], a quarter of the work)
//!   shows first, then the tile.
//! - **A change of the light**: the cache keeps the surface of the tiles
//!   that show ([`SURFACE_BUDGET`]), so a tile gets a quick picture in the
//!   new light from its surface and the surfaces round it
//!   (`sheen::overlay_around`, the shadows of its level) until it is made
//!   again.
//! - **The stitch player**: the tiles follow the cursor in steps, and the
//!   stitches after the step show as today. A step waits until the tiles of
//!   the step before are made (at most [`HOLD_MAX`]), so the threads finish
//!   their work and the 3D look stays near the needle at each speed. A tile
//!   that a step forward reaches keeps its picture until the new one is
//!   there, with the new stitches as today over it: no part of the view
//!   changes to the look of today in a rectangle.
//! - **A drag** shows the objects that move as today, and the tiles of the
//!   other objects (their lay stays as it was until the drag ends).
//!
//! The canvas draws the other marks (the selection, needle points, jumps,
//! functions, the density map) over the tiles, as before.

use crate::clock::{Duration, Instant};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::{Arc, Condvar, Mutex, mpsc};

use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, Shape, TextureHandle, TextureOptions};
use emb_geom::{BBox, Point, Transform};
use emb_model::{Design, ObjectId};
use emb_render::fabric::Ground;
use emb_render::strand::Strand;
use emb_render::surface::{self, APRON_MM, Changes, Laid, Surface};
use emb_render::thread3d::Lighting;
use emb_stitch::StitchPlan;

use crate::app::AppState;
use crate::canvas::{CanvasView, MeshLook, fitted_mesh, stitch_mesh};
use crate::viewing::ViewFilter;

/// The memory of the lays of the designs that the views showed last: a
/// design whose lay goes out is laid again when it shows (ADR 0130).
const LAY_BUDGET: usize = 256 * 1024 * 1024;
/// The levels of the tiles: level `L` has 2^L pixels for each mm.
pub const MIN_LEVEL: i32 = -4;
pub const MAX_LEVEL: i32 = 7;
/// The side of a tile, pixels.
const SIDE: i64 = surface::TILE as i64;
/// The side of the texture of a tile: one pixel of the tiles round it on
/// each side, so the tiles join with no seam.
const TEXTURE: usize = surface::TILE as usize + 2;
/// The longest time that a step of the stitch player waits for the tiles
/// of the step before (a tile that cannot be made does not stop the player).
const HOLD_MAX: Duration = Duration::from_millis(500);
/// The finished tiles that one frame gives to the screen at most.
const UPLOADS: usize = 24;
/// The finished jobs that one frame takes at most: the results of tiles
/// that no view shows any more cost nearly nothing, so a frame takes many
/// of them (a drag of the player slider leaves thousands).
const TAKES: usize = 4096;
/// The memory of the tiles of all open designs (ADR 0077), bytes.
pub const BUDGET: usize = 256 * 1024 * 1024;
/// The size that an empty tile counts for, bytes.
const EMPTY_BYTES: usize = 64;
/// The memory of the surfaces that the cache keeps for a change of the
/// light (a part of [`BUDGET`]), bytes: about 70 tiles.
const SURFACE_BUDGET: usize = 96 * 1024 * 1024;
/// The lay of a design that no view used for this long goes.
const KEEP_LAY: Duration = Duration::from_secs(60);
/// The changes of the last lays that are kept (to keep the tiles that they
/// do not reach).
const KEEP_CHANGES: usize = 16;
/// From this level, one job makes 2 × 2 tiles (the apron of a tile is large
/// for its size).
const GROUP_LEVEL: i32 = 5;
/// A draft tile is made with this many pixels for each mm (the tiles with
/// `surface::DETAIL_PX_PER_MM`).
pub const DRAFT_PX_PER_MM: f64 = 8.0;
/// The most threads that make tiles.
const MAX_WORKERS: usize = 12;

/// The pixels of a level for each mm.
pub fn level_ppm(level: i32) -> f64 {
    2f64.powi(level)
}

/// The level for `px_per_mm` pixels of the screen for each mm: the nearest
/// one.
pub fn level_for(px_per_mm: f64) -> i32 {
    (px_per_mm.max(1e-9).log2().round() as i32).clamp(MIN_LEVEL, MAX_LEVEL)
}

/// The area of the tile (`x`, `y`) of `level`, mm.
fn tile_box(level: i32, x: i64, y: i64) -> BBox {
    let mm = SIDE as f64 / level_ppm(level);
    BBox::new(
        Point::new(x as f64 * mm, y as f64 * mm),
        Point::new((x + 1) as f64 * mm, (y + 1) as f64 * mm),
    )
}

/// The tiles of `level` that the area `b` meets: columns and rows.
fn tiles_in(level: i32, b: BBox) -> (Range<i64>, Range<i64>) {
    let k = level_ppm(level) / SIDE as f64;
    let r = |lo: f64, hi: f64| (lo * k).floor() as i64..(hi * k).floor() as i64 + 1;
    (r(b.min.x, b.max.x), r(b.min.y, b.max.y))
}

/// The part of the texture of a tile in the tile (not the pixels of the
/// tiles round it).
fn inner_uv() -> Rect {
    let (a, b) = (1.0 / TEXTURE as f32, (TEXTURE - 1) as f32 / TEXTURE as f32);
    Rect::from_min_max(Pos2::new(a, a), Pos2::new(b, b))
}

fn hash_of(v: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// A tile in the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    variant: u64,
    level: i8,
    x: i32,
    y: i32,
}

enum Cached {
    /// No thread and no shadow in the tile.
    Empty,
    Texture(TextureHandle),
    /// The thread that made it stopped with an error: the look of today
    /// shows there.
    Failed,
}

struct Entry {
    tile: Cached,
    bytes: usize,
    /// The frame that used it last.
    used: u64,
    /// The pixels of the texture that a change reached (`x0`, `y0`, `x1`,
    /// `y1`): they are made again, and the look of today shows there until
    /// then.
    dirty: Option<[u32; 4]>,
    /// The parts of `dirty` that are not made yet (each one a job).
    parts: Vec<[u32; 4]>,
    /// The surface of the pixels of the texture, for a quick picture in
    /// another light.
    surface: Option<Arc<Surface>>,
    /// A picture that shows until the tile is made again: a quick picture
    /// in a new light, or the tile of a step of the stitch player before
    /// (`lags`).
    provisional: bool,
    /// The picture shows the stitches of a step of the stitch player before:
    /// the records from this one show as today over it, until the part that
    /// the step reached (`dirty`, or all of it if `provisional`) is made
    /// again.
    lags: Option<u32>,
}

impl Entry {
    /// The bytes of the surface that this entry keeps (a quick picture
    /// keeps the surface of another entry).
    fn surface_bytes(&self) -> usize {
        match &self.surface {
            Some(s) if !self.provisional => s.bytes(),
            _ => 0,
        }
    }
}

/// A tile whose dirty part is more than this part of it is made again all.
const PATCH_MAX: f64 = 0.5;
/// A part of a dirty part has at most this many pixels of its detail on a
/// side, so the threads make the parts of a change together.
const PART_DETAIL: u32 = 256;

/// The parts of the dirty part `p` of a texture of `level` for the jobs.
fn patch_parts(level: i32, p: [u32; 4]) -> Vec<[u32; 4]> {
    // The detail of the level (as `Laid::block`): 16 pixels for each mm or
    // more, at most 16 times the level.
    let f = 1u32 << (4 - level).clamp(0, 4);
    let side = (PART_DETAIL / f).max(8);
    let mut out = Vec::new();
    let mut y = p[1];
    while y < p[3] {
        let mut x = p[0];
        while x < p[2] {
            out.push([x, y, (x + side).min(p[2]), (y + side).min(p[3])]);
            x += side;
        }
        y += side;
    }
    out
}

/// The pixels of the texture of the tile (`x`, `y`) of `level` that the
/// area `b` (mm) meets: `x0`, `y0`, `x1`, `y1`.
fn texture_part(level: i32, x: i64, y: i64, b: BBox) -> Option<[u32; 4]> {
    let ppm = level_ppm(level);
    let (ox, oy) = ((x * SIDE - 1) as f64, (y * SIDE - 1) as f64);
    let n = TEXTURE as f64;
    let x0 = (b.min.x * ppm - ox).floor().clamp(0.0, n);
    let x1 = (b.max.x * ppm - ox).ceil().clamp(0.0, n);
    let y0 = (b.min.y * ppm - oy).floor().clamp(0.0, n);
    let y1 = (b.max.y * ppm - oy).ceil().clamp(0.0, n);
    (x1 > x0 && y1 > y0).then_some([x0 as u32, y0 as u32, x1 as u32, y1 as u32])
}

/// The union of two parts of a texture.
fn union(a: Option<[u32; 4]>, b: [u32; 4]) -> [u32; 4] {
    match a {
        None => b,
        Some(a) => [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
    }
}

/// The part of a texture to make again after a change that reaches `part`,
/// with the part that the changes before reached and that is not made yet
/// (`dirty`). `None` if that is more than [`PATCH_MAX`] of the texture: all
/// of it is made again then (many changes in a row, as a drag of the player
/// slider, reach all of it part by part).
fn patch_of(dirty: Option<[u32; 4]>, part: [u32; 4]) -> Option<[u32; 4]> {
    let u = union(dirty, part);
    let n = TEXTURE as f64;
    (f64::from((u[2] - u[0]) * (u[3] - u[1])) <= PATCH_MAX * n * n).then_some(u)
}

/// Draws the texture `t` of a tile at `at` but its part `hole` (pixels of
/// the texture), and gives the place of the hole on the screen.
fn with_hole(shapes: &mut Vec<Shape>, t: &TextureHandle, at: Rect, hole: [u32; 4]) -> Rect {
    let n = TEXTURE as f32;
    // A pixel of the texture on the screen (the pixels 1 to 257 fill `at`).
    let sx = |u: f32| at.min.x + (u - 1.0) / (n - 2.0) * at.width();
    let sy = |v: f32| at.min.y + (v - 1.0) / (n - 2.0) * at.height();
    let c = |v: u32| (v as f32).clamp(1.0, n - 1.0);
    let (x0, y0, x1, y1) = (c(hole[0]), c(hole[1]), c(hole[2]), c(hole[3]));
    let (lo, hi) = (1.0, n - 1.0);
    for (u0, v0, u1, v1) in [(lo, lo, hi, y0), (lo, y1, hi, hi), (lo, y0, x0, y1), (x1, y0, hi, y1)] {
        if u1 > u0 && v1 > v0 {
            let r = Rect::from_min_max(Pos2::new(sx(u0), sy(v0)), Pos2::new(sx(u1), sy(v1)));
            let uv = Rect::from_min_max(Pos2::new(u0 / n, v0 / n), Pos2::new(u1 / n, v1 / n));
            shapes.push(Shape::image(t.id(), r, uv, Color32::WHITE));
        }
    }
    Rect::from_min_max(Pos2::new(sx(x0), sy(y0)), Pos2::new(sx(x1), sy(y1)))
}

/// The strands that show, by record.
#[derive(Clone, Default)]
struct Shown {
    /// The records that the view filter shows, if it is on.
    filter: Option<Arc<Vec<bool>>>,
    /// The records from this one do not show (the stitch player).
    upto: Option<u32>,
    /// The records of the objects that move (a drag).
    hidden: Arc<Vec<Range<u32>>>,
    /// The same number for the same records that show.
    key: u64,
    /// The same number for the same view filter.
    asked: u64,
}

impl Shown {
    /// `true` if `self` shows the records of `old` and the records after its
    /// end: a step forward of the stitch player, or its end.
    fn adds_to(&self, old: &Shown) -> bool {
        let same_filter = match (&self.filter, &old.filter) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a == b,
            _ => false,
        };
        same_filter
            && self.hidden == old.hidden
            && match (old.upto, self.upto) {
                (Some(o), Some(n)) => n > o,
                (Some(_), None) => true,
                _ => false,
            }
    }

    fn shows(&self, r: u32) -> bool {
        self.upto.is_none_or(|u| r < u)
            && self
                .filter
                .as_ref()
                .is_none_or(|f| f.get(r as usize).copied().unwrap_or(true))
            && !self.hidden.iter().any(|h| h.contains(&r))
    }
}

/// The tiles of one lay in one light on one fabric with the strands that
/// show.
#[derive(Clone)]
struct Variant {
    id: u64,
    generation: u64,
    light: Lighting,
    ground: Option<Ground>,
    shown: Shown,
}

/// The stitches of a stitch plan, laid.
struct Lay {
    /// The state of the design of the plan.
    state: u64,
    laid: Arc<Laid>,
    plan: Arc<StitchPlan>,
    /// From the coordinates of the plan to those of the canvas.
    back: Transform,
    /// The number of the lay (each new lay of the design has the next one).
    generation: u64,
}

/// A job that lays the stitches of a plan: a thread, or (no threads and a
/// budget for each frame: the browser) a lay in steps, one in each frame,
/// so the first 3D view does not stop the page.
enum LayJob {
    Thread(mpsc::Receiver<LayDone>),
    Steps(Box<LaySteps>),
}

struct LaySteps {
    layer: surface::Layer,
    strands: Vec<Strand>,
    state: u64,
    plan: Arc<StitchPlan>,
    back: Transform,
    /// The time of the steps until now, ms.
    ms: f64,
    /// The frame of the last step.
    frame: u64,
}

impl LaySteps {
    /// One step of at most `budget` in frame `frame` (one step a frame);
    /// the lay when all is laid.
    fn step(&mut self, frame: u64, budget: Duration) -> Option<LayDone> {
        if self.frame == frame {
            return None;
        }
        self.frame = frame;
        let t0 = Instant::now();
        let all = self.layer.step(&self.strands, || t0.elapsed() < budget);
        self.ms += t0.elapsed().as_secs_f64() * 1000.0;
        all.then(|| {
            let layer = std::mem::replace(&mut self.layer, surface::Layer::new(&[], &[]));
            LayDone {
                state: self.state,
                laid: layer.finish(),
                changes: Changes::everywhere(),
                plan: self.plan.clone(),
                back: self.back,
                ms: self.ms,
                relaid: false,
            }
        })
    }
}

struct LayDone {
    state: u64,
    laid: Laid,
    changes: Changes,
    plan: Arc<StitchPlan>,
    back: Transform,
    ms: f64,
    relaid: bool,
}

/// The lay and the tiles of one design.
#[derive(Default)]
struct DesignLay {
    lay: Option<Lay>,
    job: Option<LayJob>,
    /// The state whose lay stopped with an error (not tried again).
    failed: Option<u64>,
    /// The changes that made each lay from the lay before, and the highest
    /// point of the lay before (a change can take a high part away).
    changes: VecDeque<(u64, Changes, f64)>,
    /// The tiles that showed last.
    shown: Option<Variant>,
    /// The lead view of the design (`Scene::lead`) had all its tiles in its
    /// last frame: the stitch player makes its next step.
    settled: bool,
    /// The time from which a step of the stitch player waits.
    held: Option<Instant>,
    /// The tiles in the light before a change of the light: they show
    /// until the tiles in the new light are there.
    before_light: Option<u64>,
    /// The records of the view filter of a lay.
    filter: Option<(u64, ViewFilter, Arc<Vec<bool>>, u64)>,
    /// The areas that changed after the lay, for a state of the design.
    stale: Option<(u64, Arc<Vec<BBox>>)>,
    used: Option<Instant>,
}

/// The work of a tile thread: `n` × `n` tiles from (`gx`, `gy`), or a part
/// of one tile.
struct Job {
    variant: u64,
    level: i32,
    gx: i64,
    gy: i64,
    n: i64,
    /// The pixels of the texture to make again (`x0`, `y0`, `x1`, `y1`).
    patch: Option<[u32; 4]>,
    /// A quick picture in a new light from the surfaces of the tile and of
    /// the eight tiles round it (the tile in the middle, row by row).
    relight: Option<Neighbors>,
    /// A draft with less detail ([`DRAFT_PX_PER_MM`]).
    draft: bool,
    laid: Arc<Laid>,
    shown: Shown,
    light: Lighting,
    ground: Option<Ground>,
}

impl Job {
    /// A quick picture (in a new light, or a draft): the tile is made again.
    fn quick(&self) -> bool {
        self.relight.is_some() || self.draft
    }

    fn keys(&self) -> Vec<Key> {
        let mut out = Vec::with_capacity((self.n * self.n) as usize);
        for j in 0..self.n {
            for i in 0..self.n {
                out.push(Key {
                    variant: self.variant,
                    level: self.level as i8,
                    x: (self.gx + i) as i32,
                    y: (self.gy + j) as i32,
                });
            }
        }
        out
    }
}

struct Done {
    keys: Vec<Key>,
    /// The part of the texture that the image is (a patch).
    patch: Option<[u32; 4]>,
    /// The picture of each tile (`None` for an empty tile); `None` if the
    /// work stopped with an error.
    images: Option<Vec<Option<ColorImage>>>,
    /// The surface of each tile.
    surfaces: Vec<Option<Arc<Surface>>>,
    /// A quick picture (in a new light, or a draft).
    quick: bool,
    ms: f64,
}

/// The surfaces of a tile and of the eight tiles round it (the tile in the
/// middle, row by row).
type Neighbors = Arc<[Option<Arc<Surface>>; 9]>;

/// A quick picture to make: the tile, and the surfaces round it for a
/// picture in a new light (none for a draft).
type Quick = (i64, i64, Option<Neighbors>);

/// What a job made: the picture and the surface of each tile.
struct Made {
    images: Vec<Option<ColorImage>>,
    surfaces: Vec<Option<Arc<Surface>>>,
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    /// The tiles that a thread makes now, and the parts of tiles.
    running: HashSet<Key>,
    running_parts: HashSet<(Key, [u32; 4])>,
    /// The quick pictures that a thread makes now.
    running_quick: HashSet<Key>,
    stop: bool,
    repaint: Option<egui::Context>,
}

#[derive(Default)]
struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
}

impl Shared {
    fn lock(&self) -> std::sync::MutexGuard<'_, Queue> {
        self.queue.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Numbers for the bench and the tests.
#[derive(Debug, Clone, Default)]
pub struct Stats {
    /// The lays made, and the time of the last one, ms.
    pub lays: usize,
    pub relays: usize,
    pub lay_ms: f64,
    /// The tiles made (not the parts of tiles) and the time of all jobs,
    /// ms.
    pub tiles: usize,
    pub tile_ms: f64,
    /// The tiles that the budget took out.
    pub dropped: usize,
    /// The finished tiles and parts that no view shows any more: not given
    /// to the screen.
    pub discarded: usize,
    /// The parts of tiles made again after a change.
    pub patches: usize,
    /// The quick pictures of tiles: in a new light, and drafts.
    pub quick: usize,
}

/// What a view shows (see [`TileCache::draw`]).
pub struct Scene<'a> {
    /// The id of the design (`AppState::doc_id`).
    pub doc: u64,
    /// The state of the design of `plan`.
    pub state: u64,
    pub plan: &'a Arc<StitchPlan>,
    pub design: &'a Design,
    pub lighting: Lighting,
    /// The fabric under the stitches: its pile and its needle holes (ADR
    /// 0127; `AppState::ground_of`).
    pub ground: Option<Ground>,
    pub filter: &'a ViewFilter,
    /// The stitch cursor (a record of `plan`).
    pub cursor: Option<usize>,
    /// The objects that a drag moves.
    pub moving: &'a [ObjectId],
    /// The view that sets the steps of the stitch player (the canvas, or the
    /// kiosk in its place): a step waits until the tiles of this view are
    /// there. Another view of the same design (the overview) shows the same
    /// steps.
    pub lead: bool,
}

/// The tiles of a view in one frame.
#[derive(Default)]
pub struct TileFrame {
    /// The pictures of the tiles, to paint.
    pub shapes: Vec<Shape>,
    /// The places on the screen with no tile: the look of today shows there.
    pub missing: Vec<Rect>,
    /// All tiles of the level of the screen are there.
    pub complete: bool,
    /// All tiles of the level of the screen show their tile or its quick
    /// picture in a new light (no tile of another level, no look of today).
    pub shown: bool,
    /// The level of the tiles.
    pub level: i32,
    /// The records from this one show as today over the tiles (the stitch
    /// player).
    pub today_from: Option<usize>,
}

/// The cache of the tiles (see the module text).
pub struct TileCache {
    designs: HashMap<u64, DesignLay>,
    tiles: HashMap<Key, Entry>,
    /// The memory of the tiles (their textures and surfaces) and of their
    /// surfaces alone.
    bytes: usize,
    surface_bytes: usize,
    pub budget: usize,
    shared: Arc<Shared>,
    tx: mpsc::Sender<Done>,
    rx: mpsc::Receiver<Done>,
    workers: usize,
    /// The frame of the jobs in the queue: the views of one frame add their
    /// jobs, the next frame starts again.
    queue_frame: u64,
    queued: HashSet<Key>,
    queued_parts: HashSet<(Key, [u32; 4])>,
    queued_relights: HashSet<Key>,
    /// Make the lays and the tiles in the frame that asks for them, with no
    /// threads (the tests, the browser).
    pub sync: bool,
    /// With `sync`: the time that a frame can use to make tiles. The next
    /// frame makes the others, so the page does not stop (the browser trial
    /// of P15). `None`: all tiles in the frame that asks for them (the
    /// tests).
    pub frame_budget: Option<Duration>,
    /// The frame and the start of the budget of that frame.
    budget_frame: u64,
    budget_start: Option<Instant>,
    pub stats: Stats,
}

impl Default for TileCache {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        TileCache {
            designs: HashMap::new(),
            tiles: HashMap::new(),
            bytes: 0,
            surface_bytes: 0,
            budget: BUDGET,
            shared: Arc::new(Shared::default()),
            tx,
            rx,
            workers: 0,
            queue_frame: u64::MAX,
            queued: HashSet::new(),
            queued_parts: HashSet::new(),
            queued_relights: HashSet::new(),
            sync: false,
            frame_budget: None,
            budget_frame: u64::MAX,
            budget_start: None,
            stats: Stats::default(),
        }
    }
}

impl Drop for TileCache {
    fn drop(&mut self) {
        self.shared.lock().stop = true;
        self.shared.wake.notify_all();
    }
}

impl TileCache {
    /// With no threads and a budget for each frame: `true` while frame
    /// `frame` can make one more tile. The first tile of a frame is always
    /// made, so the tiles come.
    fn may_make(&mut self, frame: u64) -> bool {
        let Some(budget) = self.frame_budget else {
            return true;
        };
        if self.budget_frame != frame {
            self.budget_frame = frame;
            self.budget_start = Some(Instant::now());
            return true;
        }
        self.budget_start.is_some_and(|t| t.elapsed() < budget)
    }

    /// The memory of the tiles now, bytes.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// The memory of the lays of all designs, bytes.
    pub fn lay_bytes(&self) -> usize {
        self.designs
            .values()
            .filter_map(|d| d.lay.as_ref())
            .map(|l| l.laid.bytes())
            .sum()
    }

    /// The number of tiles in the cache.
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// `true` if the lay of design `doc` is for the plan state `state`.
    #[cfg(test)]
    pub fn laid(&self, doc: u64, state: u64) -> bool {
        self.designs
            .get(&doc)
            .and_then(|d| d.lay.as_ref())
            .is_some_and(|l| l.state == state)
    }

    /// `true` while a thread lays or makes tiles, or finished tiles wait.
    #[cfg(test)]
    pub fn busy(&self) -> bool {
        let q = self.shared.lock();
        !q.jobs.is_empty()
            || !q.running.is_empty()
            || !q.running_parts.is_empty()
            || !q.running_quick.is_empty()
            || self.designs.values().any(|d| d.job.is_some())
    }

    /// Takes the tiles of design `doc` and its lay out (the design closed).
    pub fn forget(&mut self, doc: u64) {
        self.designs.remove(&doc);
    }

    /// The stitches of `scene` laid (canvas coordinates) and a number for
    /// the lay (another lay has another number), when the lay of its plan
    /// is there; starts the lay if it is not (ADR 0129: the tilt view, the
    /// magnifier and the check views use the lay of the tiles).
    pub fn laid_for(&mut self, ctx: &egui::Context, scene: &Scene) -> Option<(u64, Arc<Laid>)> {
        self.update_lay(ctx, scene);
        let lay = self.designs.get(&scene.doc)?.lay.as_ref()?;
        (lay.state == scene.state).then(|| (hash_of((scene.doc, lay.generation)), lay.laid.clone()))
    }

    /// Waits until the threads have made what the views asked for and
    /// takes their results (the tests), at most `limit`.
    #[cfg(test)]
    pub fn wait(&mut self, ctx: &egui::Context, limit: Duration) {
        let t0 = Instant::now();
        let frame = ctx.cumulative_pass_nr();
        loop {
            for d in self.designs.values_mut() {
                let done = match &mut d.job {
                    Some(LayJob::Thread(rx)) => rx.recv_timeout(Duration::from_millis(1)).ok(),
                    Some(LayJob::Steps(s)) => {
                        let frame = s.frame.wrapping_add(1);
                        s.step(frame, limit)
                    }
                    None => None,
                };
                if let Some(done) = done {
                    d.job = None;
                    adopt(d, done, &mut self.stats);
                }
            }
            while let Ok(done) = self.rx.try_recv() {
                self.take(ctx, done, frame);
            }
            let idle = {
                let q = self.shared.lock();
                q.jobs.is_empty() && q.running.is_empty() && q.running_parts.is_empty() && q.running_quick.is_empty()
            };
            if (idle && self.designs.values().all(|d| d.job.is_none())) || t0.elapsed() > limit {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    /// Draws the tiles of `scene` in the view `view` of `rect` (the canvas,
    /// a view or the overview). `None` if the view has no tiles now (no lay
    /// yet, or the stitch player on a plan that is not laid yet): the look
    /// of today shows.
    pub fn draw(&mut self, ctx: &egui::Context, rect: Rect, view: &CanvasView, scene: &Scene) -> Option<TileFrame> {
        let frame = ctx.cumulative_pass_nr();
        self.receive(ctx, frame);
        self.update_lay(ctx, scene);
        self.forget_old(scene.doc);
        let d = self.designs.get_mut(&scene.doc)?;
        let lay = d.lay.as_ref()?;
        if scene.cursor.is_some() && lay.state != scene.state {
            return None;
        }
        // The stitch player: the tiles go to the cursor when the tiles of
        // the step before are there (or after `HOLD_MAX`); a step back goes
        // at once.
        let target = scene.cursor.map(|c| c as u32 + 1);
        let before = d
            .shown
            .as_ref()
            .filter(|v| v.generation == lay.generation)
            .and_then(|v| v.shown.upto);
        let upto = match (target, before) {
            (Some(t), Some(b)) if b < t && !d.settled && d.held.is_none_or(|h| h.elapsed() < HOLD_MAX) => {
                d.held.get_or_insert_with(Instant::now);
                Some(b)
            }
            _ => {
                d.held = None;
                target
            }
        };
        let shown = shown_of(&mut d.filter, lay, scene, upto);
        let variant = Variant {
            id: hash_of((
                scene.doc,
                lay.generation,
                scene.lighting.azimuth_deg.to_bits(),
                scene.lighting.elevation_deg.to_bits(),
                scene.lighting.strength.to_bits(),
                scene.lighting.lights_off,
                scene.lighting.two,
                scene.lighting.shadows,
                scene.ground.as_ref().map(Ground::key),
                shown.key,
            )),
            generation: lay.generation,
            light: scene.lighting,
            ground: scene.ground.clone(),
            shown,
        };
        let laid = lay.laid.clone();
        let stale = if lay.state != scene.state && scene.moving.is_empty() {
            stale_of(d, scene)
        } else {
            Arc::new(Vec::new())
        };
        self.carry_over(scene.doc, &variant, &laid);
        let before_light = self.designs.get(&scene.doc).and_then(|d| d.before_light);

        let ppp = f64::from(ctx.pixels_per_point());
        let level = level_for(view.scale() * ppp);
        let mut out = TileFrame {
            level,
            complete: true,
            shown: true,
            today_from: variant.shown.upto.map(|u| u as usize),
            ..Default::default()
        };
        let visible = view.visible(rect);
        let bounds = laid.bounds().expand(APRON_MM);
        if !visible.intersects(&bounds) {
            if scene.lead {
                self.settle(scene.doc, true);
            }
            return Some(out);
        }
        let area = BBox::new(
            Point::new(visible.min.x.max(bounds.min.x), visible.min.y.max(bounds.min.y)),
            Point::new(visible.max.x.min(bounds.max.x), visible.max.y.min(bounds.max.y)),
        );
        let is_stale = |b: BBox| {
            let b = b.expand(APRON_MM);
            stale.iter().any(|s| s.intersects(&b))
        };
        let mid = visible.center();
        let sorted = |level: i32| {
            let (xs, ys) = tiles_in(level, area);
            let mut list: Vec<(i64, i64)> = ys.flat_map(|y| xs.clone().map(move |x| (x, y))).collect();
            list.sort_by(|a, b| {
                let d = |t: &(i64, i64)| {
                    let c = tile_box(level, t.0, t.1).center();
                    (c.x - mid.x).powi(2) + (c.y - mid.y).powi(2)
                };
                d(a).total_cmp(&d(b))
            });
            list
        };
        let screen = |level: i32, x: i64, y: i64| {
            let b = tile_box(level, x, y);
            Rect::from_two_pos(view.to_screen(rect, b.min), view.to_screen(rect, b.max))
        };
        let mut wanted: Vec<(i32, i64, i64, Option<[u32; 4]>)> = Vec::new();
        // Quick pictures: in a new light (with their surfaces), or drafts.
        let mut relights: Vec<Quick> = Vec::new();
        for (x, y) in sorted(level) {
            let at = screen(level, x, y);
            if is_stale(tile_box(level, x, y)) {
                out.missing.push(at);
                out.complete = false;
                out.shown = false;
                continue;
            }
            let key = Key {
                variant: variant.id,
                level: level as i8,
                x: x as i32,
                y: y as i32,
            };
            if let Some(e) = self.tiles.get_mut(&key) {
                e.used = frame;
                if let Some(u) = e.lags {
                    let u = u as usize;
                    out.today_from = Some(out.today_from.map_or(u, |t| t.min(u)));
                }
                if e.provisional {
                    // A quick picture in a new light: the tile is made again.
                    out.complete = false;
                    wanted.push((level, x, y, None));
                }
                match (&e.tile, e.dirty) {
                    (Cached::Texture(t), Some(hole)) if e.lags.is_some() => {
                        // A step forward of the stitch player: the picture
                        // of the step before, with the new stitches as
                        // today over it, until the part is made again.
                        out.shapes.push(Shape::image(t.id(), at, inner_uv(), Color32::WHITE));
                        out.complete = false;
                        wanted.push((level, x, y, Some(hole)));
                    }
                    (Cached::Texture(t), Some(hole)) => {
                        // The part that a change reached: the look of today
                        // until it is made again.
                        out.missing.push(with_hole(&mut out.shapes, t, at, hole));
                        out.complete = false;
                        out.shown = false;
                        wanted.push((level, x, y, Some(hole)));
                    }
                    (Cached::Texture(t), None) => out.shapes.push(Shape::image(t.id(), at, inner_uv(), Color32::WHITE)),
                    (Cached::Empty, _) => {}
                    (Cached::Failed, _) => {
                        out.missing.push(at);
                        out.complete = false;
                        out.shown = false;
                    }
                }
                continue;
            }
            out.complete = false;
            out.shown = false;
            wanted.push((level, x, y, None));
            // A quick picture in the new light from the surfaces of the
            // tiles in the light before.
            let mut quick = false;
            if let Some(b) = before_light {
                match self.relight_of(b, variant.id, level, x, y) {
                    Some(Relight::Job(tiles)) => {
                        relights.push((x, y, Some(tiles)));
                        quick = true;
                    }
                    Some(Relight::Empty) => {
                        self.bytes += EMPTY_BYTES;
                        self.tiles.insert(
                            key,
                            Entry {
                                tile: Cached::Empty,
                                bytes: EMPTY_BYTES,
                                used: frame,
                                dirty: None,
                                parts: Vec::new(),
                                surface: None,
                                provisional: true,
                                lags: None,
                            },
                        );
                    }
                    None => {}
                }
            }
            if !self.stand_in(&mut out.shapes, variant.id, before_light, level, x, y, at, frame) {
                out.missing.push(at);
                // Nothing shows there: a draft first, where it takes less
                // work than the tile.
                if !quick && level_ppm(level) < DRAFT_PX_PER_MM * 2.0 - 1e-9 {
                    relights.push((x, y, None));
                }
            }
        }
        self.request_relights(ctx, frame, level, &relights, &variant, &laid);
        // The levels under and over it, for a zoom, when all tiles of the
        // screen are there (they take as long as the screen): the level of a
        // larger zoom only for the middle of the view.
        let middle = BBox::from_center_size(mid, visible.width() / 2.0, visible.height() / 2.0);
        let next = if out.complete {
            vec![level - 1, level + 1]
        } else {
            Vec::new()
        };
        for l in next {
            if (MIN_LEVEL..=MAX_LEVEL).contains(&l) {
                for (x, y) in sorted(l) {
                    let b = tile_box(l, x, y);
                    if !is_stale(b) && (l < level || b.intersects(&middle)) {
                        let key = Key {
                            variant: variant.id,
                            level: l as i8,
                            x: x as i32,
                            y: y as i32,
                        };
                        wanted.push((l, x, y, self.tiles.get(&key).and_then(|e| e.dirty)));
                    }
                }
            }
        }
        self.request(ctx, frame, &wanted, &variant, &laid);
        if scene.lead {
            self.settle(scene.doc, out.complete);
        }
        Some(out)
    }

    /// Keeps whether the lead view of design `doc` had all its tiles (see
    /// `DesignLay::settled`).
    fn settle(&mut self, doc: u64, complete: bool) {
        if let Some(d) = self.designs.get_mut(&doc) {
            d.settled = complete;
        }
    }

    /// Draws a tile of another level (or in the light before) in place of
    /// the tile (`x`, `y`) of `level`, if the cache has one. `true` if it
    /// did.
    #[allow(clippy::too_many_arguments)]
    fn stand_in(
        &mut self,
        shapes: &mut Vec<Shape>,
        variant: u64,
        before_light: Option<u64>,
        level: i32,
        x: i64,
        y: i64,
        at: Rect,
        frame: u64,
    ) -> bool {
        let key = |variant: u64, level: i32, x: i64, y: i64| Key {
            variant,
            level: level as i8,
            x: x as i32,
            y: y as i32,
        };
        // The tile in the light before.
        if let Some(b) = before_light
            && let Some(e) = self.tiles.get_mut(&key(b, level, x, y))
            && !matches!(e.tile, Cached::Failed)
        {
            e.used = frame;
            if let Cached::Texture(t) = &e.tile {
                shapes.push(Shape::image(t.id(), at, inner_uv(), Color32::WHITE));
            }
            return true;
        }
        // A part of a tile of a smaller zoom.
        for d in 1..=3 {
            let l = level - d;
            if l < MIN_LEVEL {
                break;
            }
            let (px, py) = (x >> d, y >> d);
            if let Some(e) = self.tiles.get_mut(&key(variant, l, px, py))
                && !matches!(e.tile, Cached::Failed)
            {
                e.used = frame;
                if let Cached::Texture(t) = &e.tile {
                    let part = (SIDE >> d) as f32;
                    let (ox, oy) = (((x - (px << d)) as f32) * part, ((y - (py << d)) as f32) * part);
                    let s = TEXTURE as f32;
                    let uv = Rect::from_min_max(
                        Pos2::new((1.0 + ox) / s, (1.0 + oy) / s),
                        Pos2::new((1.0 + ox + part) / s, (1.0 + oy + part) / s),
                    );
                    shapes.push(Shape::image(t.id(), at, uv, Color32::WHITE));
                }
                return true;
            }
        }
        // The four tiles of a larger zoom.
        let l = level + 1;
        if l <= MAX_LEVEL {
            let keys: Vec<Key> = [(0, 0), (1, 0), (0, 1), (1, 1)]
                .iter()
                .map(|&(i, j)| key(variant, l, 2 * x + i, 2 * y + j))
                .collect();
            if keys
                .iter()
                .all(|k| self.tiles.get(k).is_some_and(|e| !matches!(e.tile, Cached::Failed)))
            {
                let half = at.size() / 2.0;
                for k in keys {
                    let e = self.tiles.get_mut(&k).expect("checked");
                    e.used = frame;
                    if let Cached::Texture(t) = &e.tile {
                        let min = at.min + egui::vec2((k.x as i64 - 2 * x) as f32, (k.y as i64 - 2 * y) as f32) * half;
                        shapes.push(Shape::image(
                            t.id(),
                            Rect::from_min_size(min, half),
                            inner_uv(),
                            Color32::WHITE,
                        ));
                    }
                }
                return true;
            }
        }
        false
    }

    /// The surfaces for a quick picture of the tile (`x`, `y`) of `level` in
    /// the light of `variant`, from the tiles of the variant `before` (in
    /// the light before): the tile and the tiles round it.
    fn relight_of(&self, before: u64, variant: u64, level: i32, x: i64, y: i64) -> Option<Relight> {
        let entry = |v: u64, dx: i64, dy: i64| {
            self.tiles.get(&Key {
                variant: v,
                level: level as i8,
                x: (x + dx) as i32,
                y: (y + dy) as i32,
            })
        };
        let middle = entry(before, 0, 0)?;
        if matches!(middle.tile, Cached::Empty) && middle.dirty.is_none() {
            return Some(Relight::Empty);
        }
        middle.surface.as_ref()?;
        let mut tiles: [Option<Arc<Surface>>; 9] = Default::default();
        for dy in -1..=1 {
            for dx in -1..=1 {
                tiles[((dy + 1) * 3 + dx + 1) as usize] = [before, variant]
                    .into_iter()
                    .find_map(|v| entry(v, dx, dy).and_then(|e| e.surface.clone()));
            }
        }
        Some(Relight::Job(Arc::new(tiles)))
    }

    /// Puts the jobs of quick pictures (in a new light, or drafts) first in
    /// the queue.
    fn request_relights(
        &mut self,
        ctx: &egui::Context,
        frame: u64,
        level: i32,
        relights: &[Quick],
        v: &Variant,
        laid: &Arc<Laid>,
    ) {
        if relights.is_empty() {
            return;
        }
        if !self.sync {
            self.start_workers();
        }
        let mut jobs = Vec::new();
        {
            let mut q = self.shared.lock();
            q.repaint = Some(ctx.clone());
            if self.queue_frame != frame {
                q.jobs.clear();
                self.queued.clear();
                self.queued_parts.clear();
                self.queued_relights.clear();
                self.queue_frame = frame;
            }
            for (x, y, tiles) in relights {
                let job = Job {
                    variant: v.id,
                    level,
                    gx: *x,
                    gy: *y,
                    n: 1,
                    patch: None,
                    relight: tiles.clone(),
                    draft: tiles.is_none(),
                    laid: laid.clone(),
                    shown: v.shown.clone(),
                    light: v.light,
                    ground: v.ground.clone(),
                };
                let key = job.keys()[0];
                if q.running_quick.contains(&key) || !self.queued_relights.insert(key) {
                    continue;
                }
                jobs.push(job);
            }
            if !self.sync {
                // Before the other jobs, in the order of the view.
                for job in jobs.drain(..).rev() {
                    q.jobs.push_front(job);
                }
            }
        }
        self.shared.wake.notify_all();
        for job in jobs {
            if !self.may_make(frame) {
                ctx.request_repaint();
                break;
            }
            let t0 = Instant::now();
            let made = make(&job);
            let done = Done {
                keys: job.keys(),
                images: Some(made.images),
                surfaces: made.surfaces,
                quick: true,
                patch: None,
                ms: t0.elapsed().as_secs_f64() * 1000.0,
            };
            self.take(ctx, done, frame);
        }
    }

    /// Puts the jobs for the tiles `wanted` (in this order) in the queue of
    /// the threads; the first view of a frame starts the queue again.
    fn request(
        &mut self,
        ctx: &egui::Context,
        frame: u64,
        wanted: &[(i32, i64, i64, Option<[u32; 4]>)],
        v: &Variant,
        laid: &Arc<Laid>,
    ) {
        if wanted.is_empty() {
            return;
        }
        if !self.sync {
            self.start_workers();
        }
        let mut jobs = Vec::new();
        {
            let mut q = self.shared.lock();
            q.repaint = Some(ctx.clone());
            if self.queue_frame != frame {
                q.jobs.clear();
                self.queued.clear();
                self.queued_parts.clear();
                self.queued_relights.clear();
                self.queue_frame = frame;
            }
            for &(level, x, y, patch) in wanted {
                if patch.is_some() {
                    // The parts of a dirty tile that are not made yet.
                    let key = Key {
                        variant: v.id,
                        level: level as i8,
                        x: x as i32,
                        y: y as i32,
                    };
                    let parts = self.tiles.get(&key).map(|e| e.parts.clone()).unwrap_or_default();
                    for part in parts {
                        if q.running_parts.contains(&(key, part)) || !self.queued_parts.insert((key, part)) {
                            continue;
                        }
                        let job = Job {
                            variant: v.id,
                            level,
                            gx: x,
                            gy: y,
                            n: 1,
                            patch: Some(part),
                            relight: None,
                            draft: false,
                            laid: laid.clone(),
                            shown: v.shown.clone(),
                            light: v.light,
                            ground: v.ground.clone(),
                        };
                        if self.sync {
                            jobs.push(job);
                        } else {
                            q.jobs.push_back(job);
                        }
                    }
                    continue;
                }
                // One tile, or 2 × 2 tiles of a large zoom.
                let n = if level >= GROUP_LEVEL { 2 } else { 1 };
                let (gx, gy) = (x.div_euclid(n) * n, y.div_euclid(n) * n);
                let job = Job {
                    variant: v.id,
                    level,
                    gx,
                    gy,
                    n,
                    patch,
                    relight: None,
                    draft: false,
                    laid: laid.clone(),
                    shown: v.shown.clone(),
                    light: v.light,
                    ground: v.ground.clone(),
                };
                let keys = job.keys();
                let made = |k: &Key| self.tiles.get(k).is_some_and(|e| !e.provisional);
                if !self.queued.insert(keys[0]) || keys.iter().all(made) || keys.iter().any(|k| q.running.contains(k)) {
                    continue;
                }
                if self.sync {
                    jobs.push(job);
                } else {
                    q.jobs.push_back(job);
                }
            }
        }
        self.shared.wake.notify_all();
        // With no threads: here, now (in the browser, as many as the budget
        // of the frame lets; the next frame makes the others).
        for job in jobs {
            if !self.may_make(frame) {
                ctx.request_repaint();
                break;
            }
            let t0 = Instant::now();
            let made = make(&job);
            let done = Done {
                keys: job.keys(),
                images: Some(made.images),
                surfaces: made.surfaces,
                quick: false,
                patch: job.patch,
                ms: t0.elapsed().as_secs_f64() * 1000.0,
            };
            self.take(ctx, done, frame);
        }
    }

    /// Starts the threads that make tiles.
    fn start_workers(&mut self) {
        if self.workers > 0 {
            return;
        }
        let n = tile_workers();
        for i in 0..n {
            let (shared, tx) = (self.shared.clone(), self.tx.clone());
            let started = std::thread::Builder::new()
                .name(format!("stitch-tiles-{i}"))
                .stack_size(8 * 1024 * 1024)
                .spawn(move || work(&shared, &tx));
            if started.is_ok() {
                self.workers += 1;
            }
        }
        if self.workers == 0 {
            // No thread could start: the tiles are made in the frame.
            self.sync = true;
        }
    }

    /// Takes the tiles that the threads finished (at most [`UPLOADS`] jobs
    /// in a frame) and keeps the cache in its budget.
    fn receive(&mut self, ctx: &egui::Context, frame: u64) {
        let (mut uploads, mut taken) = (0, 0);
        let mut more = false;
        while uploads < UPLOADS {
            if taken == TAKES {
                more = true;
                break;
            }
            let Ok(done) = self.rx.try_recv() else { break };
            taken += 1;
            if self.take(ctx, done, frame) {
                uploads += 1;
                more = uploads == UPLOADS;
            }
        }
        if more {
            ctx.request_repaint();
        }
        self.trim(frame);
    }

    /// `true` if a view shows the tiles of `variant` now, or shows them in
    /// place of the tiles in a new light (`DesignLay::before_light`).
    fn alive(&self, variant: u64) -> bool {
        self.designs
            .values()
            .any(|d| d.shown.as_ref().is_some_and(|v| v.id == variant) || d.before_light == Some(variant))
    }

    /// Puts the tiles of a finished job in the cache. Returns `true` if it
    /// gave a picture to the screen. The tiles of a variant that no view
    /// shows any more (a step of the player slider, a light that moved on)
    /// are not kept: their pictures would only cost uploads and memory.
    fn take(&mut self, ctx: &egui::Context, done: Done, frame: u64) -> bool {
        self.stats.tile_ms += done.ms;
        if done.keys.first().is_some_and(|k| !self.alive(k.variant)) {
            self.stats.discarded += done.keys.len();
            return false;
        }
        if let Some(part) = done.patch {
            self.stats.patches += 1;
            let image = done.images.and_then(|mut i| i.pop()).flatten();
            if let Some(e) = done.keys.first().and_then(|k| self.tiles.get_mut(k))
                && let Cached::Texture(t) = &mut e.tile
            {
                match image {
                    Some(image) => {
                        t.set_partial([part[0] as usize, part[1] as usize], image, TextureOptions::LINEAR);
                        e.parts.retain(|p| *p != part);
                        if e.parts.is_empty() {
                            e.dirty = None;
                            e.lags = None;
                        }
                    }
                    // The work stopped with an error: the look of today there.
                    None => e.tile = Cached::Failed,
                }
                return true;
            }
            return false;
        }
        if done.quick {
            self.stats.quick += done.keys.len();
        } else {
            self.stats.tiles += done.keys.len();
        }
        let images: Vec<Option<Option<ColorImage>>> = match done.images {
            Some(images) => images.into_iter().map(Some).collect(),
            None => done.keys.iter().map(|_| None).collect(),
        };
        let mut surfaces = done.surfaces.into_iter();
        let mut uploaded = false;
        for (key, image) in done.keys.into_iter().zip(images) {
            let surface = surfaces.next().flatten();
            // A quick picture does not take the place of a tile.
            if done.quick && self.tiles.get(&key).is_some_and(|e| !e.provisional) {
                continue;
            }
            uploaded = true;
            let (tile, bytes) = match image {
                None => (Cached::Failed, EMPTY_BYTES),
                Some(None) => (Cached::Empty, EMPTY_BYTES),
                Some(Some(img)) => {
                    let name = format!("stitch tile {} {} {} {}", key.variant, key.level, key.x, key.y);
                    (
                        Cached::Texture(ctx.load_texture(name, img, TextureOptions::LINEAR)),
                        TEXTURE * TEXTURE * 4,
                    )
                }
            };
            let entry = Entry {
                tile,
                bytes,
                used: frame,
                dirty: None,
                parts: Vec::new(),
                surface,
                provisional: done.quick,
                lags: None,
            };
            let extra = entry.surface_bytes();
            self.bytes += bytes + extra;
            self.surface_bytes += extra;
            if let Some(old) = self.tiles.insert(key, entry) {
                self.remove_bytes(&old);
            }
        }
        uploaded
    }

    /// Takes the memory of an entry that goes out of the counts.
    fn remove_bytes(&mut self, e: &Entry) {
        let extra = e.surface_bytes();
        self.bytes -= e.bytes + extra;
        self.surface_bytes -= extra;
    }

    /// Takes out the tiles that were not used for the longest time, until
    /// the cache is in its budget (not the tiles of this frame and the frame
    /// before).
    fn trim(&mut self, frame: u64) {
        if self.surface_bytes > SURFACE_BUDGET {
            // The surfaces of the tiles that showed longest ago go first.
            let mut kept: Vec<(u64, Key)> = self
                .tiles
                .iter()
                .filter(|(_, e)| e.surface_bytes() > 0)
                .map(|(k, e)| (e.used, *k))
                .collect();
            kept.sort_unstable_by_key(|(used, _)| *used);
            for (_, k) in kept {
                if self.surface_bytes <= SURFACE_BUDGET / 10 * 9 {
                    break;
                }
                if let Some(e) = self.tiles.get_mut(&k) {
                    let extra = e.surface_bytes();
                    e.surface = None;
                    self.bytes -= extra;
                    self.surface_bytes -= extra;
                }
            }
        }
        if self.bytes <= self.budget {
            return;
        }
        let mut old: Vec<(u64, Key)> = self
            .tiles
            .iter()
            .filter(|(_, e)| e.used + 1 < frame)
            .map(|(k, e)| (e.used, *k))
            .collect();
        old.sort_unstable_by_key(|(used, _)| *used);
        for (_, k) in old {
            if self.bytes <= self.budget / 10 * 9 {
                break;
            }
            if let Some(e) = self.tiles.remove(&k) {
                self.remove_bytes(&e);
                self.stats.dropped += 1;
            }
        }
    }

    /// Takes in the lay that a thread finished, and starts a lay for the
    /// plan of `scene` if it has none (not during a drag).
    fn update_lay(&mut self, ctx: &egui::Context, scene: &Scene) {
        let d = self.designs.entry(scene.doc).or_default();
        d.used = Some(Instant::now());
        match &mut d.job {
            Some(LayJob::Thread(rx)) => match rx.try_recv() {
                Ok(done) => {
                    d.job = None;
                    adopt(d, done, &mut self.stats);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    d.job = None;
                    d.failed = Some(scene.state);
                }
            },
            Some(LayJob::Steps(s)) => {
                let budget = self.frame_budget.unwrap_or(Duration::from_millis(30));
                match s.step(ctx.cumulative_pass_nr(), budget) {
                    Some(done) => {
                        d.job = None;
                        adopt(d, done, &mut self.stats);
                    }
                    None => ctx.request_repaint(),
                }
            }
            None => {}
        }
        let wanted = d.lay.as_ref().is_none_or(|l| l.state != scene.state);
        let frozen = !scene.moving.is_empty() && d.lay.is_some();
        if wanted && d.job.is_none() && !frozen && d.failed != Some(scene.state) {
            start_lay(d, ctx, scene, self.sync, self.frame_budget.is_some(), &mut self.stats);
        }
    }

    /// Takes out the lays of designs that no view used for a while, and the
    /// lays that no view used for the longest time while all of them have
    /// more than [`LAY_BUDGET`] (the audit of P18 view: ten open designs).
    fn forget_old(&mut self, keep: u64) {
        self.designs
            .retain(|doc, d| *doc == keep || d.job.is_some() || d.used.is_some_and(|t| t.elapsed() < KEEP_LAY));
        let mut total = self.lay_bytes();
        if total <= LAY_BUDGET {
            return;
        }
        let mut old: Vec<(Option<Instant>, u64, usize)> = self
            .designs
            .iter()
            .filter(|(doc, d)| **doc != keep && d.job.is_none())
            .filter_map(|(doc, d)| d.lay.as_ref().map(|l| (d.used, *doc, l.laid.bytes())))
            .collect();
        old.sort_by_key(|(used, doc, _)| (*used, *doc));
        for (_, doc, bytes) in old {
            if total <= LAY_BUDGET {
                break;
            }
            self.designs.remove(&doc);
            total -= bytes;
        }
    }

    /// The tiles of the variant that showed before `v` that a change does
    /// not reach go to `v`.
    fn carry_over(&mut self, doc: u64, v: &Variant, laid: &Laid) {
        let Some(d) = self.designs.get_mut(&doc) else { return };
        let old = match d.shown.replace(v.clone()) {
            Some(o) if o.id != v.id => o,
            _ => return,
        };
        if old.light != v.light {
            // No tile is the same in another light, but the tiles in the
            // light before show until the new ones are there.
            d.before_light = (old.generation == v.generation && old.shown.key == v.shown.key).then_some(old.id);
            return;
        }
        d.before_light = None;
        // A step forward of the stitch player: the tiles that it reaches keep
        // their pictures until the new ones are there.
        let forward = old.generation == v.generation && v.shown.adds_to(&old.shown);
        let from = old.shown.upto.unwrap_or(u32::MAX);
        // The highest point of the stitches that changed (in the lays from
        // the old one to the new one).
        let mut top = laid.top();
        let changes = if old.generation == v.generation {
            laid.changes_where(|r| old.shown.shows(r) != v.shown.shows(r))
        } else if old.generation < v.generation
            && old.shown.asked == v.shown.asked
            && v.shown.upto.is_none()
            && old.shown.upto.is_none()
            && v.shown.hidden.is_empty()
            && old.shown.hidden.is_empty()
        {
            let mut c = Changes::default();
            let mut count = 0;
            for (g, ch, before) in &d.changes {
                if *g > old.generation && *g <= v.generation {
                    c.merge(ch);
                    top = top.max(*before);
                    count += 1;
                }
            }
            if count == v.generation - old.generation {
                c
            } else {
                Changes::everywhere()
            }
        } else {
            Changes::everywhere()
        };
        if changes.is_everywhere() {
            return;
        }
        // A pixel changes if a changed stitch is this near.
        let apron = surface::apron_for(&v.light, top, v.ground.as_ref());
        let keys: Vec<Key> = self.tiles.keys().filter(|k| k.variant == old.id).copied().collect();
        for k in keys {
            let ppm = level_ppm(i32::from(k.level));
            let (level, x, y) = (i32::from(k.level), i64::from(k.x), i64::from(k.y));
            let b = tile_box(level, x, y).expand(apron + 2.0 / ppm);
            let mut part = None;
            let mut whole = false;
            if let Some(c) = changes.within(b) {
                // The pixels that the change reaches (with the apron), with
                // the parts that changes before reached: made again if they
                // are a small part of a texture, else all (`patch_of`).
                let old = self.tiles.get(&k);
                let tex = old.is_some_and(|e| matches!(e.tile, Cached::Texture(_)));
                part = texture_part(level, x, y, c.expand(apron));
                let small = part.and_then(|p| patch_of(old.and_then(|e| e.dirty), p)).is_some();
                // A tile that is made again all already needs no parts.
                let remade = forward && old.is_some_and(|e| e.provisional);
                if !tex || !small || remade {
                    if !forward {
                        continue;
                    }
                    whole = true;
                    part = None;
                }
            }
            let mut e = self.tiles.remove(&k).expect("listed");
            if forward && (whole || part.is_some()) {
                e.lags = Some(e.lags.map_or(from, |l| l.min(from)));
            }
            if whole {
                // All the tile is made again; its picture shows until then.
                let extra = e.surface_bytes();
                e.surface = None;
                self.bytes -= extra;
                self.surface_bytes -= extra;
                e.provisional = true;
                e.dirty = None;
                e.parts.clear();
            }
            if let Some(p) = part {
                let dirty = union(e.dirty, p);
                e.dirty = Some(dirty);
                e.parts = patch_parts(level, dirty);
                // Its surface is not the surface of the tile any more.
                let extra = e.surface_bytes();
                e.surface = None;
                self.bytes -= extra;
                self.surface_bytes -= extra;
            }
            let to = Key { variant: v.id, ..k };
            match self.tiles.entry(to) {
                std::collections::hash_map::Entry::Occupied(_) => self.remove_bytes(&e),
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert(e);
                }
            }
        }
    }
}

/// The number of threads that make tiles: all processors but two (the
/// window and the system), at most [`MAX_WORKERS`].
pub fn tile_workers() -> usize {
    std::thread::available_parallelism()
        .map_or(2, usize::from)
        .saturating_sub(2)
        .clamp(1, MAX_WORKERS)
}

/// A stitch plan of the synthetic design of `n` stitches
/// (`emb_render::stress`) with an object for each 2,000 records (the bench
/// and the tests).
pub fn stress_plan(n: usize) -> StitchPlan {
    let stitches = emb_render::stress::make(n);
    let count = stitches.records.len();
    let spans = (0..count.div_ceil(2000))
        .map(|k| {
            let records = k * 2000..((k + 1) * 2000).min(count);
            emb_stitch::ObjectSpan {
                object: ObjectId(k as u64 + 1),
                start: stitches.records[records.start].pos,
                end: stitches.records[records.end - 1].pos,
                records,
            }
        })
        .collect();
    StitchPlan {
        stitches,
        warnings: Vec::new(),
        spans,
        stops: Vec::new(),
    }
}

/// The objects that the drag of `st` moves (they show as today).
pub fn moving_objects(st: &AppState) -> Vec<ObjectId> {
    use crate::manipulate::DragOp;
    match &st.drag {
        Some(DragOp::Move { originals, .. } | DragOp::Scale { originals, .. } | DragOp::Rotate { originals, .. }) => {
            originals.iter().map(|o| o.id).collect()
        }
        Some(DragOp::ReshapeNode { id, .. }) => vec![*id],
        _ => Vec::new(),
    }
}

/// The tiles of the design of `st` in the view `view` of `rect` (the
/// canvas, the split view, the views and the overview).
/// `lead`: the view sets the steps of the stitch player (`Scene::lead`).
pub fn tiles_of(
    ctx: &egui::Context,
    rect: Rect,
    view: &CanvasView,
    st: &mut AppState,
    lead: bool,
) -> Option<TileFrame> {
    let (state, plan) = {
        let pv = st.shown_plan();
        (pv.state, pv.plan.clone())
    };
    let moving = moving_objects(st);
    let ground = st.ground_of(st.doc.design());
    let scene = Scene {
        doc: st.doc_id,
        state,
        plan: &plan,
        design: st.doc.design(),
        lighting: st.settings.look.light,
        ground,
        filter: &st.view_filter,
        cursor: st.cursor,
        moving: &moving,
        lead,
    };
    st.stitch_tiles.draw(ctx, rect, view, &scene)
}

/// The stitches of the design of `st` laid, when the lay is there (see
/// [`TileCache::laid_for`]).
pub fn laid_of(ctx: &egui::Context, st: &mut AppState) -> Option<(u64, Arc<Laid>)> {
    let (state, plan) = {
        let pv = st.shown_plan();
        (pv.state, pv.plan.clone())
    };
    let ground = st.ground_of(st.doc.design());
    let filter = ViewFilter::default();
    let scene = Scene {
        doc: st.doc_id,
        state,
        plan: &plan,
        design: st.doc.design(),
        lighting: st.settings.look.light,
        ground,
        filter: &filter,
        cursor: None,
        moving: &[],
        lead: false,
    };
    st.stitch_tiles.laid_for(ctx, &scene)
}

/// Paints the tiles of the canvas: the look of today under them where a
/// tile is not there yet, the tiles, then the stitches that show as today
/// over them (the objects that a drag moves, the stitches after the step of
/// the stitch player) and the jumps.
#[allow(clippy::too_many_arguments)]
pub fn paint(
    painter: &egui::Painter,
    rect: Rect,
    st: &mut AppState,
    frame: TileFrame,
    segs: &[(emb_render::Segment, Color32)],
    look: MeshLook,
    table: &emb_render::thread3d::ShadeTable,
) {
    let view = st.view;
    if let Some(missing) = frame.missing.iter().copied().reduce(|a, b| a.union(b)) {
        let mesh = fitted_mesh(rect, &view, segs, false, look, table).0;
        painter
            .with_clip_rect(missing.intersect(painter.clip_rect()))
            .add(Shape::mesh(mesh));
    }
    painter.extend(frame.shapes);
    let moving = moving_objects(st);
    let over = if !moving.is_empty() {
        Some(crate::viewing::segments_where(st, 0, Some(&moving)))
    } else {
        frame
            .today_from
            .map(|from| crate::viewing::segments_where(st, from, None))
    };
    if let Some(over) = over {
        painter.add(Shape::mesh(fitted_mesh(rect, &view, &over, false, look, table).0));
    }
    if st.show_jumps {
        let jumps: Vec<(emb_render::Segment, Color32)> = segs
            .iter()
            .filter(|s| s.0.kind == emb_render::SegmentKind::Jump)
            .copied()
            .collect();
        painter.add(Shape::mesh(stitch_mesh(rect, &view, &jumps, true).0));
    }
}

/// Takes in a finished lay.
fn adopt(d: &mut DesignLay, done: LayDone, stats: &mut Stats) {
    let generation = d.lay.as_ref().map_or(0, |l| l.generation + 1);
    let top = d.lay.as_ref().map_or(0.0, |l| l.laid.top());
    d.changes.push_back((generation, done.changes, top));
    while d.changes.len() > KEEP_CHANGES {
        d.changes.pop_front();
    }
    d.lay = Some(Lay {
        state: done.state,
        laid: Arc::new(done.laid),
        plan: done.plan,
        back: done.back,
        generation,
    });
    d.stale = None;
    stats.lays += 1;
    stats.lay_ms = done.ms;
    if done.relaid {
        stats.relays += 1;
    }
}

/// Lays the stitches of the plan of `scene` on a thread (in place with
/// `sync`): again from the lay before where it has the same place on the
/// canvas. With `steps` (and `sync`: the browser), a first lay is made in
/// steps, one in each frame (`LaySteps`); a lay again after a change is
/// quick and is made at once.
fn start_lay(d: &mut DesignLay, ctx: &egui::Context, scene: &Scene, sync: bool, steps: bool, stats: &mut Stats) {
    let plan = scene.plan.clone();
    let back = scene.design.from_hoop_transform();
    let marks = emb_render::strand::marks(scene.design, plan.spans.iter().map(|s| (s.object, s.records.clone())));
    let areas: Vec<surface::Piece> = emb_render::strand::pieces(scene.design)
        .iter()
        .map(|p| p.transformed(&back))
        .collect();
    let old = d.lay.as_ref().filter(|l| l.back == back).map(|l| l.laid.clone());
    let state = scene.state;
    let strands_of = move |plan: &StitchPlan| -> Vec<Strand> {
        emb_render::strand::strands(&plan.stitches, &marks)
            .into_iter()
            .map(|s| Strand {
                a: back.apply(s.a),
                b: back.apply(s.b),
                ..s
            })
            .collect()
    };
    if sync && steps && old.is_none() {
        let strands = strands_of(&plan);
        d.job = Some(LayJob::Steps(Box::new(LaySteps {
            layer: surface::Layer::new(&strands, &areas),
            strands,
            state,
            plan,
            back,
            ms: 0.0,
            frame: u64::MAX,
        })));
        ctx.request_repaint();
        return;
    }
    let work = move || {
        let t0 = Instant::now();
        let strands = strands_of(&plan);
        let (laid, changes, relaid) = match &old {
            Some(o) => {
                let (l, c) = surface::relay(o, &strands, &areas);
                (l, c, true)
            }
            None => (surface::lay(&strands, &areas), Changes::everywhere(), false),
        };
        LayDone {
            state,
            laid,
            changes,
            plan,
            back,
            ms: t0.elapsed().as_secs_f64() * 1000.0,
            relaid,
        }
    };
    if sync {
        adopt(d, work(), stats);
        return;
    }
    let (tx, rx) = mpsc::channel();
    let repaint = ctx.clone();
    let started = std::thread::Builder::new()
        .name("stitch-lay".into())
        .stack_size(crate::resources::JOB_STACK)
        .spawn(move || {
            let _ = tx.send(work());
            repaint.request_repaint();
        });
    match started {
        Ok(_) => d.job = Some(LayJob::Thread(rx)),
        Err(_) => d.failed = Some(state),
    }
}

/// The strands that `scene` shows, on the records of the plan of `lay`.
fn shown_of(
    cache: &mut Option<(u64, ViewFilter, Arc<Vec<bool>>, u64)>,
    lay: &Lay,
    scene: &Scene,
    upto: Option<u32>,
) -> Shown {
    let filter = if scene.filter.is_active() {
        match cache {
            Some((g, f, v, h)) if *g == lay.generation && f == scene.filter => Some((v.clone(), *h)),
            _ => {
                let v = Arc::new(filter_records(&lay.plan, scene.design, scene.filter));
                let h = hash_of(&*v);
                *cache = Some((lay.generation, scene.filter.clone(), v.clone(), h));
                Some((v, h))
            }
        }
    } else {
        None
    };
    let hidden: Vec<Range<u32>> = if scene.moving.is_empty() {
        Vec::new()
    } else {
        lay.plan
            .spans
            .iter()
            .filter(|s| scene.moving.contains(&s.object))
            .map(|s| s.records.start as u32..s.records.end as u32)
            .collect()
    };
    let key = hash_of((filter.as_ref().map(|f| f.1), upto, &hidden));
    let asked = hash_of(format!("{:?}", scene.filter));
    Shown {
        filter: filter.map(|f| f.0),
        upto,
        hidden: Arc::new(hidden),
        key,
        asked,
    }
}

/// For each record of `plan`, `true` if the view filter shows it (as
/// `viewing::visible_segments`).
fn filter_records(plan: &StitchPlan, design: &Design, filter: &ViewFilter) -> Vec<bool> {
    let objects: HashMap<ObjectId, bool> = design.objects().iter().map(|o| (o.id, filter.shows(o))).collect();
    let mut show = vec![!filter.is_active(); plan.stitches.records.len()];
    for s in &plan.spans {
        let v = objects.get(&s.object).copied().unwrap_or(true);
        show[s.records.clone()].fill(v);
    }
    show
}

/// The areas where the plan of `scene` differs from the plan of the lay
/// (canvas coordinates), made one time for each state of the design.
fn stale_of(d: &mut DesignLay, scene: &Scene) -> Arc<Vec<BBox>> {
    if let Some((s, v)) = &d.stale
        && *s == scene.state
    {
        return v.clone();
    }
    let Some(lay) = d.lay.as_ref() else {
        return Arc::new(Vec::new());
    };
    let back = scene.design.from_hoop_transform();
    let v = Arc::new(changed_areas(&lay.plan, &lay.back, scene.plan, &back));
    d.stale = Some((scene.state, v.clone()));
    v
}

/// The areas of the objects whose stitches differ in the two plans: where
/// they were and where they are (canvas coordinates).
fn changed_areas(old: &StitchPlan, old_back: &Transform, new: &StitchPlan, back: &Transform) -> Vec<BBox> {
    if old_back != back {
        let far = 1.0e6;
        return vec![BBox::new(Point::new(-far, -far), Point::new(far, far))];
    }
    let spans: HashMap<ObjectId, Range<usize>> = old.spans.iter().map(|s| (s.object, s.records.clone())).collect();
    let area = |plan: &StitchPlan, r: Range<usize>, t: &Transform| {
        BBox::from_points(plan.stitches.records[r].iter().map(|x| t.apply(x.pos))).map(|b| b.expand(1.0))
    };
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for s in &new.spans {
        seen.insert(s.object);
        let records = &new.stitches.records[s.records.clone()];
        let same = spans
            .get(&s.object)
            .is_some_and(|r| old.stitches.records[r.clone()] == *records);
        if !same {
            out.extend(area(new, s.records.clone(), back));
            if let Some(r) = spans.get(&s.object) {
                out.extend(area(old, r.clone(), old_back));
            }
        }
    }
    for s in &old.spans {
        if !seen.contains(&s.object) {
            out.extend(area(old, s.records.clone(), old_back));
        }
    }
    out
}

/// The loop of a tile thread.
fn work(shared: &Shared, tx: &mpsc::Sender<Done>) {
    loop {
        let job = {
            let mut q = shared.lock();
            loop {
                if q.stop {
                    return;
                }
                if let Some(job) = q.jobs.pop_front() {
                    match job.patch {
                        Some(p) => {
                            q.running_parts.insert((job.keys()[0], p));
                        }
                        None if job.quick() => q.running_quick.extend(job.keys()),
                        None => q.running.extend(job.keys()),
                    }
                    break job;
                }
                q = shared.wake.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        };
        let t0 = Instant::now();
        let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| make(&job))).ok();
        let (images, surfaces) = match made {
            Some(m) => (Some(m.images), m.surfaces),
            None => (None, Vec::new()),
        };
        let keys = job.keys();
        let repaint = {
            let mut q = shared.lock();
            match job.patch {
                Some(p) => {
                    q.running_parts.remove(&(keys[0], p));
                }
                None if job.quick() => {
                    for k in &keys {
                        q.running_quick.remove(k);
                    }
                }
                None => {
                    for k in &keys {
                        q.running.remove(k);
                    }
                }
            }
            q.repaint.clone()
        };
        let done = Done {
            keys,
            images,
            surfaces,
            quick: job.quick(),
            patch: job.patch,
            ms: t0.elapsed().as_secs_f64() * 1000.0,
        };
        if tx.send(done).is_err() {
            return;
        }
        if let Some(ctx) = repaint {
            ctx.request_repaint();
        }
    }
}

/// The pictures of the tiles of a job (`None` for an empty tile).
fn make(job: &Job) -> Made {
    let ppm = level_ppm(job.level);
    if let Some(tiles) = &job.relight {
        return relight(tiles, &job.light);
    }
    if let Some(p) = job.patch {
        // A part of a tile: its pixels, transparent where nothing is.
        let (w, h) = (p[2] - p[0], p[3] - p[1]);
        let (x0, y0) = (job.gx * SIDE - 1 + i64::from(p[0]), job.gy * SIDE - 1 + i64::from(p[1]));
        let image = match job
            .laid
            .block(ppm, x0, y0, w, h, &job.light, job.ground.as_ref(), &|r| {
                job.shown.shows(r)
            }) {
            Some(s) => {
                let picture = emb_render::sheen::overlay(&s, &job.light);
                ColorImage::from_rgba_premultiplied([w as usize, h as usize], &picture.rgba)
            }
            None => ColorImage::new([w as usize, h as usize], vec![Color32::TRANSPARENT; (w * h) as usize]),
        };
        return Made {
            images: vec![Some(image)],
            surfaces: vec![None],
        };
    }
    let count = (job.n * job.n) as usize;
    let side = (job.n * SIDE + 2) as u32;
    let detail = if job.draft {
        DRAFT_PX_PER_MM
    } else {
        surface::DETAIL_PX_PER_MM
    };
    let block = job.laid.block_with(
        ppm,
        job.gx * SIDE - 1,
        job.gy * SIDE - 1,
        side,
        side,
        &job.light,
        job.ground.as_ref(),
        &|r| job.shown.shows(r),
        detail,
    );
    let Some(s) = block else {
        return Made {
            images: vec![None; count],
            surfaces: vec![None; count],
        };
    };
    let picture = emb_render::sheen::overlay(&s, &job.light);
    let mut out = Made {
        images: Vec::with_capacity(count),
        surfaces: Vec::with_capacity(count),
    };
    for j in 0..job.n {
        for i in 0..job.n {
            let (x0, y0) = ((i * SIDE) as usize, (j * SIDE) as usize);
            let mut rgba = Vec::with_capacity(TEXTURE * TEXTURE * 4);
            for row in 0..TEXTURE {
                let at = ((y0 + row) * side as usize + x0) * 4;
                rgba.extend_from_slice(&picture.rgba[at..at + TEXTURE * 4]);
            }
            let seen = rgba.iter().skip(3).step_by(4).any(|a| *a > 0);
            out.images
                .push(seen.then(|| ColorImage::from_rgba_premultiplied([TEXTURE, TEXTURE], &rgba)));
            out.surfaces.push((seen && !job.draft).then(|| {
                Arc::new(if job.n == 1 {
                    s.clone()
                } else {
                    s.crop(x0 as u32, y0 as u32, TEXTURE as u32, TEXTURE as u32)
                })
            }));
        }
    }
    out
}

/// A quick picture of a tile in a new light from its surface and the
/// surfaces of the tiles round it (`tiles`, the tile in the middle): the
/// shadows from the heights of its level.
fn relight(tiles: &[Option<Arc<Surface>>; 9], light: &Lighting) -> Made {
    let Some(middle) = tiles[4].as_ref() else {
        return Made {
            images: vec![None],
            surfaces: vec![None],
        };
    };
    // The pixels 1 to 256 of each texture are those of its tile.
    let height = |x: i64, y: i64| -> f64 {
        let (gx, gy) = (x - 1, y - 1);
        let (dx, dy) = (gx.div_euclid(SIDE), gy.div_euclid(SIDE));
        let other = (dx.abs() <= 1 && dy.abs() <= 1)
            .then(|| tiles[((dy + 1) * 3 + dx + 1) as usize].as_ref())
            .flatten();
        match other {
            Some(s) => f64::from(s.height(gx.rem_euclid(SIDE) + 1, gy.rem_euclid(SIDE) + 1)),
            None => f64::from(middle.height(x, y)),
        }
    };
    let top = tiles
        .iter()
        .flatten()
        .map(|s| f64::from(s.max_height()))
        .fold(0.0, f64::max);
    let around = emb_render::sheen::Around {
        height: &height,
        pixels: (1 - SIDE, 1 - SIDE, 1 + 2 * SIDE, 1 + 2 * SIDE),
        top,
    };
    let picture = emb_render::sheen::overlay_around(middle, light, &around);
    let seen = picture.rgba.iter().skip(3).step_by(4).any(|a| *a > 0);
    Made {
        images: vec![seen.then(|| ColorImage::from_rgba_premultiplied([TEXTURE, TEXTURE], &picture.rgba))],
        surfaces: vec![Some(middle.clone())],
    }
}

/// Where the quick picture of a tile in a new light comes from.
enum Relight {
    /// The tile was empty: it is empty in the new light too.
    Empty,
    /// The surfaces of the tile and of the tiles round it.
    Job(Neighbors),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;
    use emb_model::Rgb;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, egui::vec2(1200.0, 900.0))
    }

    /// The harness with the surface on, its lays and tiles made in the
    /// frame, the design at `zoom` times the size that fits.
    fn harness(zoom: f64) -> Harness {
        let mut h = Harness::new();
        h.st.settings.look.surface = true;
        h.st.stitch_tiles.sync = true;
        let b = h.st.fit_box().expect("the sample objects");
        h.st.view.fit(rect(), b);
        h.st.view.zoom *= zoom;
        h
    }

    fn frame_or_none(h: &mut Harness) -> Option<TileFrame> {
        let view = h.st.view;
        let ctx = h.ctx.clone();
        tiles_of(&ctx, rect(), &view, &mut h.st, true)
    }

    fn frame(h: &mut Harness) -> TileFrame {
        frame_or_none(h).expect("a lay")
    }

    #[test]
    fn a_view_shows_all_its_tiles_when_they_are_made() {
        let mut h = harness(1.0);
        let first = frame(&mut h);
        assert!(
            !first.complete && !first.missing.is_empty(),
            "the first frame asks for them"
        );
        let second = frame(&mut h);
        assert!(second.complete && second.missing.is_empty(), "{:?}", second.missing);
        assert!(!second.shapes.is_empty());
        let s = &h.st.stitch_tiles.stats;
        assert_eq!((s.lays, s.relays), (1, 0));
        assert!(h.st.stitch_tiles.bytes() > 0);
    }

    #[test]
    fn the_browser_lays_a_design_in_steps() {
        // With a budget for each frame (the browser has no threads), the
        // first lay comes in steps, one in each frame, so the page does not
        // stop; then the tiles come as before.
        let mut h = harness(1.0);
        h.st.stitch_tiles.frame_budget = Some(Duration::ZERO);
        let mut frames = 0;
        while frame_or_none(&mut h).is_none() {
            frames += 1;
            assert!(frames < 10_000, "the lay does not come");
            h.frame(vec![]);
        }
        assert!(frames > 2, "the lay took {frames} frames");
        assert_eq!(h.st.stitch_tiles.stats.lays, 1);
        for _ in 0..200 {
            h.frame(vec![]);
            if frame(&mut h).complete {
                return;
            }
        }
        panic!("the tiles do not come");
    }

    #[test]
    fn a_change_of_one_object_makes_only_its_tiles_again() {
        let mut h = harness(3.0);
        frame(&mut h);
        assert!(frame(&mut h).complete);
        let made = h.st.stitch_tiles.stats.tiles;
        // Move the first object (the fill in the middle) a little.
        let id = h.st.doc.design().objects()[0].id;
        h.st.doc
            .edit("Move", |d| d.transform_objects(&[id], &Transform::translate(0.5, 0.0)));
        frame(&mut h);
        let f = frame(&mut h);
        assert!(f.complete, "{:?}", f.missing);
        let again = h.st.stitch_tiles.stats.tiles - made;
        assert!(again < made, "{again} of {made}");
        assert_eq!(h.st.stitch_tiles.stats.relays, 1);
        assert!(h.st.stitch_tiles.stats.patches > 0, "parts of tiles");
    }

    #[test]
    fn the_tiles_wait_for_the_lay_where_the_design_changed() {
        let mut h = harness(3.0);
        frame(&mut h);
        frame(&mut h);
        // A plan whose lay is not there yet: the tiles near the change show
        // the look of today, the others stay.
        let id = h.st.doc.design().objects()[0].id;
        h.st.doc
            .edit("Move", |d| d.transform_objects(&[id], &Transform::translate(3.0, 0.0)));
        let (state, plan) = {
            let pv = h.st.shown_plan();
            (pv.state, pv.plan.clone())
        };
        let doc = h.st.doc_id;
        let filter = ViewFilter::default();
        let scene = Scene {
            doc,
            state,
            plan: &plan,
            design: h.st.doc.design(),
            lighting: h.st.settings.look.light,
            ground: None,
            filter: &filter,
            cursor: None,
            moving: &[],
            lead: false,
        };
        let d = h.st.stitch_tiles.designs.get_mut(&doc).expect("a lay");
        let stale = stale_of(d, &scene);
        assert!(!stale.is_empty());
        // Only near the moved object.
        let b = h.st.doc.design().bbox_of(&[id]).expect("a fill");
        assert!(stale.iter().all(|s| s.width() < b.width() + 10.0), "{stale:?}");
    }

    #[test]
    fn the_stitch_player_shows_the_tiles_up_to_its_cursor() {
        let mut h = harness(1.0);
        let records = h.st.shown_plan().plan.stitches.records.len();
        assert!(records > 1000, "{records}");
        h.st.cursor = Some(records / 3);
        frame(&mut h);
        let f = frame(&mut h);
        assert_eq!(f.today_from, Some(records / 3 + 1));
        assert!(f.complete);
        // The cursor goes off: all stitches on the tiles again.
        h.st.cursor = None;
        let f = until_complete(&mut h, records / 3 + 1);
        assert!(f.today_from.is_none());
    }

    /// Runs frames until all tiles of the view are there; each frame on the
    /// way must show a picture on each tile (no look of today in a
    /// rectangle) and the stitches from `from` or later as today.
    fn until_complete(h: &mut Harness, from: usize) -> TileFrame {
        for _ in 0..50 {
            let f = frame(h);
            assert!(f.missing.is_empty(), "no look of today in a rectangle: {:?}", f.missing);
            if f.complete {
                return f;
            }
            assert!(f.today_from.is_some_and(|t| t >= from), "{:?}", f.today_from);
        }
        panic!("the tiles are not made");
    }

    /// A step forward of the player keeps the pictures of the tiles that it
    /// reaches (no rectangle of the look of today), with the new stitches
    /// as today over them; the next step waits until the tiles are made.
    #[test]
    fn a_step_of_the_player_keeps_the_pictures_until_the_new_ones_are_there() {
        let mut h = harness(1.0);
        let records = h.st.shown_plan().plan.stitches.records.len();
        let (c1, c2, c3) = (records / 4, records / 2, 3 * records / 4);
        h.st.cursor = Some(c1);
        frame(&mut h);
        assert!(frame(&mut h).complete);
        // The step: the tiles that it reaches show their picture of before.
        h.st.cursor = Some(c2);
        let f = frame(&mut h);
        assert!(f.missing.is_empty(), "no look of today in a rectangle: {:?}", f.missing);
        assert!(!f.complete, "the tiles are made again");
        assert_eq!(f.today_from, Some(c1 + 1), "the new stitches show as today");
        // The cursor goes on before they are there: the step waits.
        h.st.cursor = Some(c3);
        let f = until_complete(&mut h, c1 + 1);
        assert_eq!(f.today_from, Some(c2 + 1), "the step of before, now made");
        // Then the next step.
        let f = frame(&mut h);
        assert!(f.missing.is_empty(), "{:?}", f.missing);
        assert_eq!(f.today_from, Some(c2 + 1));
        let f = until_complete(&mut h, c2 + 1);
        assert_eq!(f.today_from, Some(c3 + 1));
        // The end of the player: the pictures stay until all stitches are on
        // the tiles.
        h.st.cursor = None;
        let f = until_complete(&mut h, c3 + 1);
        assert_eq!(f.today_from, None);
    }

    /// A step back of the player goes at once (the stitches after the
    /// cursor must not show in 3D).
    #[test]
    fn a_step_back_of_the_player_goes_at_once() {
        let mut h = harness(1.0);
        let records = h.st.shown_plan().plan.stitches.records.len();
        h.st.cursor = Some(records / 2);
        frame(&mut h);
        frame(&mut h);
        h.st.cursor = Some(records / 3);
        let f = frame(&mut h);
        assert_eq!(f.today_from, Some(records / 3 + 1));
    }

    #[test]
    fn a_hidden_object_makes_only_its_tiles_again() {
        let mut h = harness(3.0);
        frame(&mut h);
        frame(&mut h);
        let made = h.st.stitch_tiles.stats.tiles;
        let id = h.st.doc.design().objects()[0].id;
        h.st.view_filter.hidden.insert(id);
        frame(&mut h);
        assert!(frame(&mut h).complete);
        let again = h.st.stitch_tiles.stats.tiles - made;
        assert!(again < made, "{again} of {made}");
        assert!(h.st.stitch_tiles.stats.patches > 0, "parts of tiles");
        assert_eq!(h.st.stitch_tiles.stats.lays, 1, "the same lay");
    }

    #[test]
    fn a_drag_keeps_the_lay_and_hides_the_objects_that_move() {
        let mut h = harness(1.0);
        frame(&mut h);
        frame(&mut h);
        let o = (*h.st.doc.design().objects()[0]).clone();
        h.st.drag = Some(crate::manipulate::DragOp::Move {
            originals: vec![o.clone()],
            start: Point::ZERO,
            key: emb_model::MergeKey::new("test drag"),
        });
        assert_eq!(moving_objects(&h.st), vec![o.id]);
        h.st.doc.edit("Move", |d| {
            d.transform_objects(&[o.id], &Transform::translate(5.0, 0.0))
        });
        frame(&mut h);
        assert!(frame(&mut h).complete);
        assert_eq!(h.st.stitch_tiles.stats.lays, 1, "no lay during the drag");
        h.st.drag = None;
        frame(&mut h);
        assert!(frame(&mut h).complete);
        assert_eq!(h.st.stitch_tiles.stats.relays, 1, "a lay after it");
    }

    #[test]
    fn a_new_light_shows_at_once_and_the_tiles_follow() {
        let mut h = harness(1.0);
        frame(&mut h);
        assert!(frame(&mut h).complete);
        h.st.settings.look.light.azimuth_deg = 45.0;
        // The first frame in the new light asks for the quick pictures; in
        // the next one they show (with no threads, they are made at once).
        let f = frame(&mut h);
        assert!(!f.complete);
        let f = frame(&mut h);
        assert!(f.shown && f.missing.is_empty(), "{:?}", f.missing);
        assert!(h.st.stitch_tiles.stats.quick > 0);
        let f = frame(&mut h);
        assert!(f.complete, "the tiles in the new light");
    }

    #[test]
    fn a_quick_picture_in_a_new_light_is_near_the_tile() {
        // The puff letters (high, with long shadows) at 8 pixels for each mm.
        let design = emb_cli::corpus::build("view-puff").expect("corpus design");
        let opts = emb_cli::plan_options(&design, None).expect("a machine");
        let plan = emb_cli::plan(&design, &opts);
        let back = design.from_hoop_transform();
        let strands: Vec<Strand> = emb_render::strand::strands(&plan.stitches, &Default::default())
            .into_iter()
            .map(|s| Strand {
                a: back.apply(s.a),
                b: back.apply(s.b),
                ..s
            })
            .collect();
        let laid = Arc::new(surface::lay(&strands, &[]));
        let level = 3;
        let c = laid.bounds().center();
        let (cx, cy) = (
            (c.x * level_ppm(level) / SIDE as f64).floor() as i64,
            (c.y * level_ppm(level) / SIDE as f64).floor() as i64,
        );
        let job = |x: i64, y: i64, light: Lighting| Job {
            variant: 0,
            level,
            gx: x,
            gy: y,
            n: 1,
            patch: None,
            relight: None,
            draft: false,
            laid: laid.clone(),
            shown: Shown::default(),
            light,
            ground: None,
        };
        let (a, b) = (
            Lighting::default(),
            Lighting {
                azimuth_deg: 30.0,
                elevation_deg: 30.0,
                ..Default::default()
            },
        );
        let mut tiles: [Option<Arc<Surface>>; 9] = Default::default();
        for dy in -1..=1 {
            for dx in -1..=1 {
                tiles[((dy + 1) * 3 + dx + 1) as usize] = make(&job(cx + dx, cy + dy, a)).surfaces.pop().flatten();
            }
        }
        let quick = relight(&tiles, &b).images.pop().flatten().expect("stitches");
        let exact = make(&job(cx, cy, b)).images.pop().flatten().expect("stitches");
        // The mean light of the threads and the shadows, in the colors of
        // the screen: near (the shadows are made from this level).
        let mean = |img: &ColorImage| {
            let sum: f64 = img
                .pixels
                .iter()
                .map(|p| f64::from(p.r()) + f64::from(p.g()) + f64::from(p.b()) + f64::from(p.a()))
                .sum();
            sum / img.pixels.len() as f64
        };
        let (q, e) = (mean(&quick), mean(&exact));
        assert!((q - e).abs() < 0.05 * e, "{q:.1} {e:.1}");
        // A draft of a small zoom (made with half the detail): near too.
        for level in [1, 2] {
            let c = laid.bounds().center();
            let at = |v: f64| (v * level_ppm(level) / SIDE as f64).floor() as i64;
            let (x, y) = (at(c.x), at(c.y));
            let draft = make(&Job {
                level,
                draft: true,
                ..job(x, y, a)
            })
            .images
            .pop()
            .flatten()
            .expect("stitches");
            let exact = make(&Job { level, ..job(x, y, a) })
                .images
                .pop()
                .flatten()
                .expect("stitches");
            let (d, e) = (mean(&draft), mean(&exact));
            assert!((d - e).abs() < 0.03 * e, "level {level}: {d:.1} {e:.1}");
        }
    }

    #[test]
    fn the_cache_keeps_its_budget() {
        let mut h = harness(2.0);
        h.st.stitch_tiles.budget = 1;
        h.st.show_plan = true;
        h.st.show_thread_look = true;
        for _ in 0..6 {
            h.frame(vec![]);
        }
        let t = &h.st.stitch_tiles;
        assert!(t.stats.dropped > 0, "{:?}", t.stats);
        // The tiles of the last frames stay.
        assert!(t.len() > 0);
    }

    #[test]
    fn threads_make_the_tiles() {
        let mut h = harness(1.0);
        h.st.stitch_tiles.sync = false;
        assert!(frame_or_none(&mut h).is_none(), "no lay in the first frame");
        let ctx = h.ctx.clone();
        h.st.stitch_tiles.wait(&ctx, Duration::from_secs(60));
        frame(&mut h);
        h.st.stitch_tiles.wait(&ctx, Duration::from_secs(60));
        let f = frame(&mut h);
        assert!(f.complete, "{:?}", f.missing);
        // Then the levels under and over the screen.
        h.st.stitch_tiles.wait(&ctx, Duration::from_secs(60));
        assert!(!h.st.stitch_tiles.busy());
    }

    /// The numbers of the budgets of the plan of the view (P18 view plan,
    /// Milestone 5) on this computer, with the threads of the program: the
    /// first full view, new tiles after a change of one object, a change of
    /// the light, and the time of a frame of a pan with the tiles made. Run
    /// with `cargo test --release -p emb-app -- --ignored tile_budgets
    /// --nocapture`.
    #[test]
    #[ignore]
    fn tile_budgets() {
        let ctx = egui::Context::default();
        let size = egui::vec2(1920.0, 1080.0);
        let rect = Rect::from_min_size(Pos2::ZERO, size);
        let design = Design::new("stress");
        // On woven cotton (the needle holes; no pile).
        let ground = Some(emb_render::fabric::Ground::new(
            emb_render::fabric::Weave::Plain,
            0.25,
            true,
        ));
        let filter = ViewFilter::default();
        // Runs frames until all tiles of the screen show; gives the time, ms.
        // Runs frames until a frame has what `ready` asks; gives the time, ms.
        let until = |cache: &mut TileCache, view: &CanvasView, scene: &Scene, ready: fn(&TileFrame) -> bool| {
            let t0 = Instant::now();
            while t0.elapsed() < Duration::from_secs(60) {
                let input = egui::RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                };
                let mut done = false;
                let mut out = ctx.run_ui(input, |ui| {
                    done = cache.draw(ui.ctx(), rect, view, scene).is_some_and(|f| ready(&f));
                });
                out.textures_delta.clear();
                if done {
                    return t0.elapsed().as_secs_f64() * 1000.0;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            f64::NAN
        };
        let until_complete =
            |cache: &mut TileCache, view: &CanvasView, scene: &Scene| until(cache, view, scene, |f| f.complete);
        let fit = |plan: &StitchPlan| {
            let segs: Vec<emb_render::Segment> = emb_render::segments(&plan.stitches);
            let mut v = CanvasView::default();
            v.fit(rect, emb_render::bounds(&segs).expect("stitches").expand(2.0));
            v
        };
        for n in [10_000, 100_000, 1_000_000] {
            let plan = Arc::new(stress_plan(n));
            let view = fit(&plan);
            let mut cache = TileCache::default();
            let scene = Scene {
                doc: 1,
                state: 1,
                plan: &plan,
                design: &design,
                lighting: Lighting::default(),
                ground: ground.clone(),
                filter: &filter,
                cursor: None,
                moving: &[],
                lead: false,
            };
            let shown = until(&mut cache, &view, &scene, |f| f.shown);
            let first = until_complete(&mut cache, &view, &scene) + shown;
            println!(
                "first view, {n:>9} stitches: shown in {shown:>5.0} ms, all tiles in {first:>6.0} ms (lay {:.0} ms), {} tiles of level {}",
                cache.stats.lay_ms,
                cache.len(),
                level_for(view.scale())
            );
            cache.wait(&ctx, Duration::from_secs(120));
            // A pan with the tiles made: the time of the work of a frame.
            let mut pan = view;
            let t0 = Instant::now();
            for _ in 0..60 {
                pan.center.x += 0.5;
                let input = egui::RawInput {
                    screen_rect: Some(rect),
                    ..Default::default()
                };
                let mut out = ctx.run_ui(input, |ui| {
                    cache.draw(ui.ctx(), rect, &pan, &scene);
                });
                out.textures_delta.clear();
            }
            println!(
                "  a frame of a pan: {:.2} ms",
                t0.elapsed().as_secs_f64() * 1000.0 / 60.0
            );
            // Another light.
            let lighting = Lighting {
                azimuth_deg: 105.0,
                ..Default::default()
            };
            let light = Scene { lighting, ..scene };
            let shown = until(&mut cache, &view, &light, |f| f.shown);
            let t = until_complete(&mut cache, &view, &light) + shown;
            println!(
                "  another light: shown in {shown:.0} ms, all tiles in {t:.0} ms; {} MB of tiles",
                cache.bytes() >> 20
            );
        }
        // A change of one object in 200,000 stitches.
        let plan = stress_plan(200_000);
        let view = fit(&plan);
        let mut cache = TileCache::default();
        let plan = Arc::new(plan);
        let scene = Scene {
            doc: 1,
            state: 1,
            plan: &plan,
            design: &design,
            lighting: Lighting::default(),
            ground: ground.clone(),
            filter: &filter,
            cursor: None,
            moving: &[],
            lead: false,
        };
        until_complete(&mut cache, &view, &scene);
        cache.wait(&ctx, Duration::from_secs(120));
        let mut changed = (*plan).clone();
        let r = changed.spans[40].records.clone();
        for rec in &mut changed.stitches.records[r] {
            rec.pos.x += 0.3;
        }
        let changed = Arc::new(changed);
        let scene = Scene {
            state: 2,
            plan: &changed,
            ..scene
        };
        let made = cache.stats.tiles;
        let made_ms = cache.stats.tile_ms;
        let t0 = Instant::now();
        while !cache.laid(1, 2) {
            let input = egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                cache.draw(ui.ctx(), rect, &view, &scene);
            });
            out.textures_delta.clear();
            std::thread::sleep(Duration::from_millis(1));
        }
        println!(
            "  the lay after the change: {:.0} ms",
            t0.elapsed().as_secs_f64() * 1000.0
        );
        let t = until_complete(&mut cache, &view, &scene);
        println!(
            "a change of one object (2,000 stitches) in 200,000 stitches: {t:.0} ms after the lay (relay {:.0} ms, {} tiles again)",
            cache.stats.lay_ms,
            cache.stats.tiles - made
        );
        println!(
            "  the work of the tiles: {:.0} ms, {} parts of tiles",
            cache.stats.tile_ms - made_ms,
            cache.stats.patches
        );
    }

    /// Writes the tiles of the views of some corpus designs, put together
    /// over the color of their fabric as the canvas shows them, to
    /// `target/tmp/tiles` (to look at the joins of the tiles). Run with
    /// `cargo test --release -p emb-app -- --ignored write_tile_pictures`.
    #[test]
    #[ignore]
    fn write_tile_pictures() {
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp/tiles");
        std::fs::create_dir_all(&out).expect("a folder");
        for (name, zoom, draft, bare) in [
            ("view-fill-border", 1.0, false, false),
            ("view-fill-border", 1.0, true, false),
            ("view-lettering", 3.0, false, false),
            ("view-puff", 2.0, false, false),
            ("view-towel", 2.5, false, false),
            ("view-towel", 2.5, false, true),
        ] {
            let mut design = emb_cli::corpus::build(name).expect("corpus design");
            // With no topping: thin stitches sink into the pile (ADR 0127).
            design.no_topping = bare;
            let preset = emb_model::FabricCatalog::builtin().of(&design);
            let ground = design
                .background
                .map(|_| emb_render::fabric::Ground::of(&preset, &design));
            let opts = emb_cli::plan_options(&design, None).expect("a machine");
            let plan = Arc::new(emb_cli::plan(&design, &opts));
            let mut cache = TileCache::default();
            cache.sync = true;
            let r = Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 700.0));
            let segs = emb_render::segments(&plan.stitches);
            let back = design.from_hoop_transform();
            let b = emb_render::bounds(&segs).expect("stitches");
            let b = BBox::from_points([back.apply(b.min), back.apply(b.max)]).expect("two points");
            let mut view = CanvasView::default();
            view.fit(r, b.expand(3.0));
            view.zoom *= zoom;
            let filter = ViewFilter::default();
            let scene = Scene {
                doc: 1,
                state: 1,
                plan: &plan,
                design: &design,
                lighting: Lighting::default(),
                ground: ground.clone(),
                filter: &filter,
                cursor: None,
                moving: &[],
                lead: false,
            };
            let ctx = egui::Context::default();
            cache.draw(&ctx, r, &view, &scene);
            let f = cache.draw(&ctx, r, &view, &scene).expect("a lay");
            assert!(f.complete, "{name}");
            // The tiles of the frame from their jobs, over the fabric (its
            // mean light in each pixel, as the canvas draws it).
            let c = design.background.unwrap_or(Rgb(240, 236, 228));
            let (w, h) = (r.width() as usize, r.height() as usize);
            let look = emb_render::fabric::FabricLook::of(&preset, &design);
            let shade = look.fabric().shade(&Lighting::default());
            let lin = |c: Rgb| [c.0, c.1, c.2].map(emb_render::light::to_linear);
            let size = 1.0 / view.scale();
            let mut px: Vec<[f32; 3]> = (0..w * h)
                .map(|i| {
                    let p = view.to_mm(r, Pos2::new((i % w) as f32 + 0.5, (i / w) as f32 + 0.5));
                    let l = shade.color(lin(c), look.mix.map(lin), p.x, p.y, size);
                    l.map(|v| f32::from(emb_render::light::to_srgb(v.min(1.0))))
                })
                .collect();
            let level = f.level;
            let vis = view.visible(r);
            let (xs, ys) = tiles_in(level, vis);
            let laid = cache.designs[&1].lay.as_ref().expect("a lay").laid.clone();
            for ty in ys {
                for tx in xs.clone() {
                    let job = Job {
                        variant: 0,
                        level,
                        gx: tx,
                        gy: ty,
                        n: 1,
                        patch: None,
                        relight: None,
                        draft,
                        laid: laid.clone(),
                        shown: Shown::default(),
                        light: Lighting::default(),
                        ground: ground.clone(),
                    };
                    let Some(img) = make(&job).images.pop().flatten() else {
                        continue;
                    };
                    let at = Rect::from_two_pos(
                        view.to_screen(r, tile_box(level, tx, ty).min),
                        view.to_screen(r, tile_box(level, tx, ty).max),
                    );
                    // Each pixel of the screen from the nearest texel (no
                    // filter: the joins show as they are).
                    for y in at.min.y.max(0.0) as usize..(at.max.y.min(h as f32)) as usize {
                        for x in at.min.x.max(0.0) as usize..(at.max.x.min(w as f32)) as usize {
                            let u = 1.0 + (x as f32 + 0.5 - at.min.x) / at.width() * 256.0;
                            let v = 1.0 + (y as f32 + 0.5 - at.min.y) / at.height() * 256.0;
                            let t = img.pixels[(v as usize).min(257) * TEXTURE + (u as usize).min(257)];
                            let a = 1.0 - f32::from(t.a()) / 255.0;
                            let p = &mut px[y * w + x];
                            for (k, ch) in [t.r(), t.g(), t.b()].into_iter().enumerate() {
                                p[k] = f32::from(ch) + p[k] * a;
                            }
                        }
                    }
                }
            }
            let mut img = image::RgbImage::new(w as u32, h as u32);
            for (i, p) in px.iter().enumerate() {
                img.put_pixel(
                    (i % w) as u32,
                    (i / w) as u32,
                    image::Rgb(p.map(|v| v.round().clamp(0.0, 255.0) as u8)),
                );
            }
            let file = match (draft, bare) {
                (true, _) => format!("{name}-draft.png"),
                (_, true) => format!("{name}-no-topping.png"),
                _ => format!("{name}.png"),
            };
            img.save(out.join(file)).expect("a picture");
        }
    }

    /// The exit test of Milestone 5 of the view: no view of a corpus design
    /// shows a missing tile when the work is done, at the zoom that fits and
    /// at two larger zooms. Run with `cargo test --release -p emb-app --
    /// --ignored every_corpus_design_has_all_its_tiles --nocapture`.
    #[test]
    #[ignore]
    fn every_corpus_design_has_all_its_tiles() {
        let mut h = Harness::empty();
        h.st.settings.look.surface = true;
        for name in emb_cli::corpus::NAMES {
            let design = emb_cli::corpus::build(name).expect("corpus design");
            h.st.doc = emb_model::Document::new(design);
            h.st.doc_id += 1;
            let Some(b) = h.st.fit_box() else { continue };
            for zoom in [1.0, 4.0, 16.0] {
                h.st.view.fit(rect(), b);
                h.st.view.zoom *= zoom;
                // The time until all tiles of the level of the screen show,
                // then until the threads are done.
                let t0 = Instant::now();
                let mut screen = None;
                while t0.elapsed() < Duration::from_secs(120) {
                    if frame_or_none(&mut h).is_some_and(|f| f.complete) {
                        screen = Some(t0.elapsed().as_secs_f64() * 1000.0);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                let ctx = h.ctx.clone();
                h.st.stitch_tiles.wait(&ctx, Duration::from_secs(120));
                let all = t0.elapsed().as_secs_f64() * 1000.0;
                let f = frame_or_none(&mut h).expect("a lay");
                assert!(f.complete && f.missing.is_empty(), "{name} at {zoom}: {:?}", f.missing);
                println!(
                    "{name:<28} zoom {zoom:>4}: level {:>2}, {:>3} tiles, screen {:>6.0} ms, all {:>6.0} ms",
                    f.level,
                    f.shapes.len(),
                    screen.expect("all tiles of the screen"),
                    all
                );
            }
        }
    }

    /// The audit of P18 view (Milestone 9): a machine file 1 km wide (a
    /// wrong value) shows its tiles at the zoom of all of it and near a
    /// corner, with no error and in a bounded memory.
    #[test]
    fn a_design_of_one_km_shows_its_tiles() {
        use emb_model::{StitchDesign, StitchKind, StitchRecord, Thread};
        let thread = |c| Thread {
            color: c,
            ..Default::default()
        };
        let mut s = StitchDesign {
            threads: vec![thread(emb_model::Rgb(20, 60, 160)), thread(emb_model::Rgb(200, 40, 40))],
            ..Default::default()
        };
        // Two small objects 1 km apart (an object larger than 3 m does not
        // sew: `emb_stitch::MAX_OBJECT_MM`).
        for (x, y, k) in [
            (0.0, 0.0, StitchKind::Stitch),
            (10.0, 0.0, StitchKind::Stitch),
            (10.0, 5.0, StitchKind::Stitch),
            (10.0, 5.0, StitchKind::ColorChange),
            (1_000_000.0, 1_000_000.0, StitchKind::Jump),
            (1_000_000.0, 1_000_000.0, StitchKind::Stitch),
            (1_000_010.0, 1_000_000.0, StitchKind::Stitch),
        ] {
            s.records.push(StitchRecord::new(Point::new(x, y), k));
        }
        let mut h = Harness::empty();
        h.st.settings.look.surface = true;
        h.st.stitch_tiles.sync = true;
        h.st.show_thread_look = true;
        h.st.doc = emb_model::Document::new(emb_model::design_from_stitches(&s, "one km"));
        h.st.doc_id += 1;
        let t0 = Instant::now();
        let b = h.st.fit_box().expect("the stitches");
        h.st.view.fit(rect(), b);
        // The first frame asks for the tiles, the second shows them.
        frame(&mut h);
        let f = frame(&mut h);
        assert!(
            f.complete,
            "all tiles at the zoom of all of it: level {}, {} shapes, {} missing, zoom {}, {:?}",
            f.level,
            f.shapes.len(),
            f.missing.len(),
            h.st.view.zoom,
            h.st.stitch_tiles.stats
        );
        // Near the first stitches (canvas coordinates).
        let plan = h.st.shown_plan().plan.clone();
        let span = plan.stitches.extents().expect("the stitches sew");
        assert!(span.width() > 999_000.0, "{span:?}");
        let first = plan.stitches.records[1].pos;
        h.st.view.zoom = 4.0;
        h.st.view.center = h.st.doc.design().from_hoop_transform().apply(first);
        frame(&mut h);
        let f = frame(&mut h);
        assert!(f.complete && !f.shapes.is_empty(), "{} {}", f.complete, f.shapes.len());
        assert!(h.st.stitch_tiles.bytes() <= h.st.stitch_tiles.budget);
        assert!(t0.elapsed() < Duration::from_secs(60), "{:?}", t0.elapsed());
    }

    /// The audit of P18 view (Milestone 9): the memory of the tiles and the
    /// lays of ten open designs of 100,000 stitches, each shown on the
    /// canvas. Run with `cargo test --release -p emb-app -- --ignored
    /// ten_open_designs --nocapture`.
    #[test]
    #[ignore]
    fn ten_open_designs() {
        let mut cache = TileCache::default();
        cache.sync = true;
        let ctx = crate::test_util::ready_context();
        let plan = Arc::new(stress_plan(100_000));
        let design = Design::new("");
        let filter = ViewFilter::default();
        let b = plan.stitches.extents().expect("stitches");
        let mut view = CanvasView::default();
        view.fit(rect(), b);
        for doc in 0..10u64 {
            let scene = Scene {
                doc: 1000 + doc,
                state: doc,
                plan: &plan,
                design: &design,
                lighting: Lighting::default(),
                ground: None,
                filter: &filter,
                cursor: None,
                moving: &[],
                lead: false,
            };
            for _ in 0..2 {
                let _ = ctx.run_ui(Default::default(), |_| {});
                cache.draw(&ctx, rect(), &view, &scene);
            }
        }
        let (tiles, lays) = (cache.bytes(), cache.lay_bytes());
        println!(
            "ten designs of 100,000 stitches: tiles {} MB (budget {} MB), lays {} MB (budget {} MB, {} lays kept)",
            tiles >> 20,
            cache.budget >> 20,
            lays >> 20,
            LAY_BUDGET >> 20,
            cache.designs.values().filter(|d| d.lay.is_some()).count()
        );
        assert!(tiles <= cache.budget);
        assert!(lays <= LAY_BUDGET, "{lays}");
    }

    #[test]
    fn levels_are_powers_of_two() {
        assert_eq!(level_for(16.0), 4);
        assert_eq!(level_for(11.0), 3);
        assert_eq!(level_for(12.0), 4);
        assert_eq!(level_for(0.001), MIN_LEVEL);
        assert_eq!(level_for(1.0e6), MAX_LEVEL);
        let b = tile_box(3, -1, 2);
        assert_eq!((b.min.x, b.min.y, b.max.x, b.max.y), (-32.0, 64.0, 0.0, 96.0));
        let (xs, ys) = tiles_in(3, BBox::new(Point::new(-1.0, 0.0), Point::new(33.0, 31.9)));
        assert_eq!((xs, ys), (-1..2, 0..1));
    }
    /// The stitch player on a large corpus design with the tiles made by
    /// their threads, frame by frame in real time: the time of a frame, the
    /// part of the screen where the tiles are missing (the look of today
    /// shows there in a rectangle), and how far the tiles are behind the
    /// cursor (those stitches show as today). `cargo test --release -p
    /// emb-app -- --ignored player_budget --nocapture`.
    #[test]
    #[ignore]
    fn player_budget() {
        for speed in [200.0, 2000.0] {
            let mut h = Harness::new();
            h.st.doc = emb_model::Document::new(emb_cli::corpus::build("full-field").expect("corpus design"));
            h.st.settings.look.surface = true;
            let b = h.st.fit_box().expect("stitches");
            h.st.view.fit(rect(), b);
            h.st.view.zoom *= 2.0;
            // All tiles of the design first, with the player at the start.
            h.st.toggle_player();
            let t0 = Instant::now();
            while !frame_or_none(&mut h).is_some_and(|f| f.complete) && t0.elapsed() < Duration::from_secs(30) {
                std::thread::sleep(Duration::from_millis(5));
            }
            h.st.player.speed = speed;
            h.st.play_pause();
            let (mut times, mut flat, mut lags) = (Vec::new(), Vec::new(), Vec::new());
            let mut missing_frames = 0;
            let area = rect().area();
            let start = Instant::now();
            let mut last = Instant::now();
            while start.elapsed() < Duration::from_secs(8) && h.st.player.playing {
                let next = last + Duration::from_secs_f64(1.0 / 60.0);
                if let Some(wait) = next.checked_duration_since(Instant::now()) {
                    std::thread::sleep(wait);
                }
                let now = Instant::now();
                h.st.player_tick((now - last).as_secs_f64());
                last = now;
                let t = Instant::now();
                let f = frame(&mut h);
                times.push(t.elapsed().as_secs_f64() * 1000.0);
                let gone: f32 = f.missing.iter().map(|r| r.intersect(rect()).area()).sum();
                if gone > 0.0 {
                    missing_frames += 1;
                }
                flat.push(f64::from(gone / area));
                let cursor = h.st.cursor.unwrap_or(0) + 1;
                lags.push(cursor.saturating_sub(f.today_from.unwrap_or(cursor)) as f64);
            }
            let n = times.len().max(1) as f64;
            let mut sorted = times.clone();
            sorted.sort_by(f64::total_cmp);
            let p95 = sorted.get((sorted.len() * 95) / 100).copied().unwrap_or(0.0);
            let max = sorted.last().copied().unwrap_or(0.0);
            let mean_lag = lags.iter().sum::<f64>() / n;
            let max_lag = lags.iter().copied().fold(0.0, f64::max);
            let max_flat = flat.iter().copied().fold(0.0, f64::max);
            println!(
                "{speed} stitches/s: {} frames; frame p95 {p95:.1} ms, max {max:.1} ms; frames with missing tiles {missing_frames} (largest {:.0} % of the view); behind the cursor: mean {mean_lag:.0} records ({:.2} s), max {max_lag:.0} ({:.2} s)",
                times.len(),
                max_flat.abs() * 100.0,
                mean_lag / speed,
                max_lag / speed,
            );
        }
    }
    /// The stitch player after a light by hand and a drag of the slider back
    /// and forth (owner report of 2026-11-20: after the close of the player
    /// the view was flat, low in detail and slow for a minute). It runs the
    /// owner's steps in real time with the tiles made by their threads, and
    /// prints the time until all tiles are there after the close. The design
    /// is the file of `EMB_PLAYER_DESIGN`, else the corpus design
    /// full-field. `cargo test --release -p emb-app -- --ignored
    /// player_slider_budget --nocapture`.
    #[test]
    #[ignore]
    fn player_slider_budget() {
        let design = match std::env::var("EMB_PLAYER_DESIGN") {
            Ok(path) => emb_model::project::load(std::path::Path::new(&path)).expect("design"),
            Err(_) => emb_cli::corpus::build("full-field").expect("corpus design"),
        };
        let canvas = Rect::from_min_size(Pos2::ZERO, egui::vec2(1050.0, 570.0));
        let mut h = Harness::new();
        h.st.doc = emb_model::Document::new(design);
        h.st.settings.look.surface = true;
        let b = h.st.fit_box().unwrap();
        h.st.view.fit(canvas, b);
        let mut last_t = Instant::now();
        let mut worst = 0.0f64;
        let mut step = |h: &mut Harness, worst: &mut f64| -> Option<TileFrame> {
            std::thread::sleep(Duration::from_millis(16));
            let now = Instant::now();
            h.st.player_tick((now - last_t).as_secs_f64());
            last_t = now;
            let ctx = h.ctx.clone();
            let view = h.st.view;
            let mut got = None;
            let input = egui::RawInput {
                screen_rect: Some(canvas),
                ..Default::default()
            };
            let t = Instant::now();
            let mut o = ctx.run_ui(input, |ui| got = tiles_of(ui.ctx(), canvas, &view, &mut h.st, true));
            *worst = worst.max(t.elapsed().as_secs_f64() * 1000.0);
            o.textures_delta.clear();
            got
        };
        let report = |label: &str, h: &Harness, f: &Option<TileFrame>, worst: f64, t0: Instant| {
            let c = &h.st.stitch_tiles;
            let (jobs, running, parts, quick) = {
                let q = c.shared.lock();
                (
                    q.jobs.len(),
                    q.running.len(),
                    q.running_parts.len(),
                    q.running_quick.len(),
                )
            };
            let s = &c.stats;
            println!(
                "{label} t={:.1}s: {} | worst frame {worst:.1} ms | tiles made {} quick {} patches {} discarded {} | queue {jobs} running {running}+{parts}+{quick} | entries {} MB {}",
                t0.elapsed().as_secs_f64(),
                f.as_ref().map_or("no frame".to_string(), |f| format!(
                    "complete {} today_from {:?} missing {} shapes {}",
                    f.complete,
                    f.today_from,
                    f.missing.len(),
                    f.shapes.len()
                )),
                s.tiles,
                s.quick,
                s.patches,
                s.discarded,
                c.tiles.len(),
                c.bytes() / 1_000_000
            );
        };
        let t0 = Instant::now();
        let mut f = None;
        while t0.elapsed() < Duration::from_secs(5) {
            f = step(&mut h, &mut worst);
            if f.as_ref().is_some_and(|f| f.complete) {
                break;
            }
        }
        report("first view", &h, &f, worst, t0);
        // 1. The light by hand, then the default light.
        let t0 = Instant::now();
        worst = 0.0;
        let mut az = 135.0;
        while t0.elapsed() < Duration::from_secs(2) {
            az += 7.0;
            h.st.settings.look.light.azimuth_deg = az % 360.0;
            f = step(&mut h, &mut worst);
        }
        h.st.settings.look.light = emb_render::thread3d::Lighting::default();
        for _ in 0..10 {
            f = step(&mut h, &mut worst);
        }
        report("light", &h, &f, worst, t0);
        // 2. The player at 5,000 stitches a second, the slider back and forth.
        h.st.toggle_player();
        h.st.player.speed = 5000.0;
        let n = h.st.shown_plan().plan.stitches.records.len();
        let t0 = Instant::now();
        worst = 0.0;
        let mut x: u64 = 12345;
        while t0.elapsed() < Duration::from_secs(5) {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let c = ((x >> 33) as usize) % n;
            h.st.player.playing = false;
            h.st.cursor = Some(c);
            f = step(&mut h, &mut worst);
        }
        report("slider", &h, &f, worst, t0);
        // 3. Stitch 0, then close.
        h.st.cursor = Some(0);
        for _ in 0..10 {
            f = step(&mut h, &mut worst);
        }
        h.st.close_player();
        // 4. Then a zoom in, when the tiles are there (or after 10 s).
        let all = |f: &Option<TileFrame>| {
            f.as_ref()
                .is_some_and(|f| f.complete && f.today_from.is_none() && f.missing.is_empty())
        };
        let t0 = Instant::now();
        worst = 0.0;
        while t0.elapsed() < Duration::from_secs(10) && !all(&f) {
            f = step(&mut h, &mut worst);
        }
        report("closed: all tiles", &h, &f, worst, t0);
        h.st.view.zoom *= 4.0;
        let t0 = Instant::now();
        worst = 0.0;
        f = None;
        while t0.elapsed() < Duration::from_secs(10) && !all(&f) {
            f = step(&mut h, &mut worst);
        }
        report("zoomed in: all tiles", &h, &f, worst, t0);
    }
    /// Small changes in a row make a tile again in parts until their parts
    /// together are more than `PATCH_MAX` of it; then all of it is made
    /// again (a drag of the player slider made a tile in 81 parts again for
    /// each step: thousands of jobs).
    #[test]
    fn many_small_changes_make_the_whole_tile_again() {
        let n = TEXTURE as u32;
        assert!(patch_of(None, [0, 0, 10, 10]).is_some(), "one small part");
        let mut dirty = None;
        let mut strips = 0;
        for i in 0..8 {
            let x = i * n / 8;
            match patch_of(dirty, [x, 0, x + n / 8, n]) {
                Some(d) => {
                    dirty = Some(d);
                    strips += 1;
                }
                None => break,
            }
        }
        assert_eq!(
            strips, 4,
            "four strips are half of the texture; the fifth makes all of it again"
        );
    }

    /// A finished tile of a variant that no view shows any more is not kept
    /// (no upload, no memory); a tile of the variant that shows is.
    #[test]
    fn finished_tiles_that_no_view_shows_are_not_kept() {
        let mut h = harness(1.0);
        frame(&mut h);
        frame(&mut h);
        let ctx = h.ctx.clone();
        let c = &mut h.st.stitch_tiles;
        let live = c
            .designs
            .values()
            .find_map(|d| d.shown.as_ref().map(|v| v.id))
            .expect("a variant");
        let done = |variant: u64| Done {
            keys: vec![Key {
                variant,
                level: MAX_LEVEL as i8,
                x: 1000,
                y: 1000,
            }],
            patch: None,
            images: Some(vec![Some(ColorImage::new(
                [TEXTURE, TEXTURE],
                vec![Color32::RED; TEXTURE * TEXTURE],
            ))]),
            surfaces: vec![None],
            quick: false,
            ms: 0.0,
        };
        let (entries, bytes) = (c.tiles.len(), c.bytes());
        assert!(!c.take(&ctx, done(live.wrapping_add(1)), 0), "not given to the screen");
        assert_eq!((c.tiles.len(), c.bytes(), c.stats.discarded), (entries, bytes, 1));
        assert!(c.take(&ctx, done(live), 0));
        assert_eq!(c.tiles.len(), entries + 1);
    }
}
