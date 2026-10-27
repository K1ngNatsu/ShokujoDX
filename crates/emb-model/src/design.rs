use std::sync::Arc;

use emb_geom::{BBox, Point, Transform};
use serde::{Deserialize, Serialize};

use crate::object::fresh_revision;
use crate::{Backdrop, Colorway, CurrentProperties, DesignObject, ObjectId, ObjectKind, Rgb, Style, Thread, ThreadId};

/// One embroidery design.
///
/// `Design` is cheap to clone: objects are shared with `Arc` and an edit
/// clones only the objects that it changes (see [`Design::object_mut`]).
/// The undo history in [`crate::Document`] keeps clones of `Design`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Design {
    pub name: String,
    pub threads: Vec<Thread>,
    /// Private, so that each change goes through `object_mut` and gets a new revision.
    pub(crate) objects: Vec<Arc<DesignObject>>,
    #[serde(default)]
    pub hoop: HoopPlacement,
    /// Machine profile id from `machines.toml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<String>,
    #[serde(default)]
    pub rotation: Rotation,
    /// A bitmap under the design, for tracing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<Backdrop>,
    /// Guide lines for snapping, in view coordinates (after the design rotation).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guides: Vec<Guide>,
    /// The background (fabric) color of the current colorway. `None`: the
    /// normal canvas color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Rgb>,
    /// Color versions of the design (see `colorway`). Empty: only the
    /// current colors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colorways: Vec<Colorway>,
    /// The index of the current colorway.
    #[serde(default)]
    pub colorway: usize,
    /// Notes for the worksheets (job details, comments).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// Fabric preset id from `fabrics.toml` (see `fabric`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fabric: Option<String>,
    /// The stitch settings of new objects (see `style`).
    #[serde(default, skip_serializing_if = "CurrentProperties::is_default")]
    pub current: CurrentProperties,
    /// Named sets of stitch settings (see `style`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub styles: Vec<Style>,
    /// The monograms of the design (see `monogram`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monograms: Vec<crate::MonogramRecord>,
    /// The team names list of the design (see `teamnames`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_names: Option<crate::TeamNames>,
    /// The multi blends of the design (see `decor`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blends: Vec<crate::BlendRecord>,
    /// The border traces of the design (see `trace`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub border_traces: Vec<crate::BorderTrace>,
    /// The linked shaping results of the design (see `link`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shape_links: Vec<crate::ShapeLink>,
    /// The links from sketches to the objects made from them (see
    /// `sketch_link`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sketch_links: Vec<crate::SketchLink>,
    /// Where the needle starts and ends the design (ADR 0032).
    #[serde(default, skip_serializing_if = "StartEnd::is_off")]
    pub start_end: StartEnd,
    /// The hoopings of a design that is larger than the hoop (see `hooping`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hooping: Option<crate::HoopingLayout>,
    /// The product under the design on the canvas (see `product`). It does not sew.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<crate::ProductView>,
    /// The repeats of the design (see `repeat`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat: Option<crate::RepeatSpec>,
    /// The running-stitch object that sews the registration marks of the
    /// rhinestone template (P9 Milestone 5, ADR 0039).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_marks: Option<ObjectId>,
    /// The needles that the user gave threads of the design, for a
    /// multi-needle machine (see `needles`, ADR 0063).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub needles: Vec<crate::NeedlePin>,
    /// The order of the design (P13 Milestone 2): customer, order number,
    /// garment, sizes, placement, due date. `None`: no order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::order::OrderInfo>,
}

/// A guide line across the canvas. `vertical` guides are at `x = pos`,
/// the others at `y = pos` (mm, view coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub vertical: bool,
    pub pos: f64,
}

/// Which hoop the design uses and where the hoop center is.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HoopPlacement {
    pub selection: HoopSelection,
    /// Hoop center in design coordinates. A new design in a hoop has its origin at the hoop center.
    #[serde(default)]
    pub center: Point,
    /// The hoop turns by this angle on the canvas (degrees, clockwise on
    /// screen). The stitch plan turns the stitches back, so the hoop is
    /// straight in the machine file (ADR 0032).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub angle_deg: f64,
    /// Auto centering: each edit puts the hoop center at the center of the
    /// design. `false`: manual placement.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub auto_center: bool,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// Where the needle starts and ends the design (ADR 0032). The stitch plan
/// starts with a jump to the start point and ends with a trim and a jump to
/// the end point. No stitch sews at the points.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct StartEnd {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<Anchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<Anchor>,
}

impl Anchor {
    pub const DESIGN_CENTER: Anchor = Anchor::DesignBox { x: 0.5, y: 0.5 };
}

impl StartEnd {
    pub fn is_off(&self) -> bool {
        self.start.is_none() && self.end.is_none()
    }
}

/// A point that the design refers to.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Anchor {
    /// A place on the box of the objects that sew, as they sit in the hoop
    /// (the design rotation and the hoop angle applied): `x` and `y` from 0
    /// (left, top) to 1 (right, bottom). The center is (0.5, 0.5).
    DesignBox {
        x: f64,
        y: f64,
    },
    HoopCenter,
    /// A point in design coordinates.
    Point {
        point: Point,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HoopSelection {
    /// No hoop boundary.
    #[default]
    OpenField,
    /// A preset id from `hoops.toml`.
    Preset { id: String },
}

/// Design-level rotation in quarter turns, applied when the engine builds the stitch plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub enum Rotation {
    #[default]
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

impl Rotation {
    pub fn degrees(self) -> u16 {
        match self {
            Rotation::Deg0 => 0,
            Rotation::Deg90 => 90,
            Rotation::Deg180 => 180,
            Rotation::Deg270 => 270,
        }
    }

    /// Returns the next rotation, 90° clockwise.
    pub fn turned_cw(self) -> Rotation {
        match self {
            Rotation::Deg0 => Rotation::Deg90,
            Rotation::Deg90 => Rotation::Deg180,
            Rotation::Deg180 => Rotation::Deg270,
            Rotation::Deg270 => Rotation::Deg0,
        }
    }
}

impl TryFrom<u16> for Rotation {
    type Error = String;
    fn try_from(v: u16) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Rotation::Deg0),
            90 => Ok(Rotation::Deg90),
            180 => Ok(Rotation::Deg180),
            270 => Ok(Rotation::Deg270),
            _ => Err(format!("invalid rotation {v}, expected 0, 90, 180 or 270")),
        }
    }
}

impl From<Rotation> for u16 {
    fn from(r: Rotation) -> u16 {
        r.degrees()
    }
}

impl Default for Design {
    fn default() -> Self {
        Self::new("Untitled")
    }
}

impl Design {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            threads: Vec::new(),
            objects: Vec::new(),
            hoop: HoopPlacement::default(),
            machine: None,
            rotation: Rotation::Deg0,
            backdrop: None,
            guides: Vec::new(),
            background: None,
            colorways: Vec::new(),
            colorway: 0,
            notes: String::new(),
            fabric: None,
            current: CurrentProperties::default(),
            styles: Vec::new(),
            monograms: Vec::new(),
            team_names: None,
            blends: Vec::new(),
            border_traces: Vec::new(),
            shape_links: Vec::new(),
            sketch_links: Vec::new(),
            start_end: StartEnd::default(),
            hooping: None,
            product: None,
            repeat: None,
            template_marks: None,
            needles: Vec::new(),
            order: None,
        }
    }

    /// Objects in sew order.
    pub fn objects(&self) -> &[Arc<DesignObject>] {
        &self.objects
    }

    pub fn object(&self, id: ObjectId) -> Option<&DesignObject> {
        self.objects.iter().find(|o| o.id == id).map(|o| o.as_ref())
    }

    pub fn index_of(&self, id: ObjectId) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    /// A new object id: higher than the ids of all objects and of the
    /// originals that border traces and shape links keep.
    pub(crate) fn next_object_id(&self) -> ObjectId {
        let ids = self.objects.iter().map(|o| o.id).chain(self.kept_ids());
        ObjectId(ids.map(|id| id.0 + 1).max().unwrap_or(1))
    }

    /// Adds an object at the end of the sew order and returns its id.
    pub fn add_object(&mut self, thread: ThreadId, kind: ObjectKind) -> ObjectId {
        self.insert_object(self.objects.len(), thread, kind)
    }

    /// Inserts an object at `index` in the sew order and returns its id.
    /// An index past the end adds the object at the end.
    pub fn insert_object(&mut self, index: usize, thread: ThreadId, kind: ObjectKind) -> ObjectId {
        let id = self.next_object_id();
        let obj = DesignObject {
            id,
            revision: fresh_revision(),
            name: None,
            thread,
            start: None,
            end: None,
            group: None,
            locked: false,
            functions: Default::default(),
            kind,
        };
        self.objects.insert(index.min(self.objects.len()), Arc::new(obj));
        id
    }

    /// Returns the object for a change, and gives it a new revision.
    ///
    /// If an undo snapshot shares the object, this clones it first (copy on write),
    /// so the snapshot keeps the old version.
    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut DesignObject> {
        let arc = self.objects.iter_mut().find(|o| o.id == id)?;
        let obj = Arc::make_mut(arc);
        obj.revision = fresh_revision();
        Some(obj)
    }

    /// Removes object `id`. Its sketch links end: the links of a sketch,
    /// and a link with no objects left.
    pub fn remove_object(&mut self, id: ObjectId) -> Option<Arc<DesignObject>> {
        let i = self.index_of(id)?;
        if !self.sketch_links.is_empty() {
            self.sketch_links.retain_mut(|l| {
                l.objects.retain(|o| *o != id);
                l.sketch != id && !l.objects.is_empty()
            });
        }
        Some(self.objects.remove(i))
    }

    /// Moves an object to `new_index` in the sew order. Returns `false` if the id is not found.
    pub fn move_object(&mut self, id: ObjectId, new_index: usize) -> bool {
        let Some(i) = self.index_of(id) else { return false };
        let obj = self.objects.remove(i);
        self.objects.insert(new_index.min(self.objects.len()), obj);
        true
    }

    /// Adds a thread to the palette and returns its id.
    /// The first thread with this color, or a new thread named after the color.
    pub fn thread_for_color(&mut self, color: Rgb) -> ThreadId {
        if let Some(t) = self.threads.iter().find(|t| t.color == color) {
            return t.id;
        }
        let n = self.threads.len() + 1;
        self.add_thread(format!("Thread {n} ({})", color.to_hex()), color)
    }

    pub fn add_thread(&mut self, name: impl Into<String>, color: Rgb) -> ThreadId {
        let id = ThreadId(self.threads.iter().map(|t| t.id.0 + 1).max().unwrap_or(0));
        self.threads.push(Thread {
            id,
            name: name.into(),
            color,
            brand: None,
            code: None,
        });
        id
    }

    pub fn thread(&self, id: ThreadId) -> Option<&Thread> {
        self.threads.iter().find(|t| t.id == id)
    }

    /// Bounding box of all object geometry (control points included), before rotation.
    pub fn extents(&self) -> Option<BBox> {
        self.objects.iter().filter_map(|o| o.control_bbox()).reduce(BBox::union)
    }

    /// The design rotation as a transform about the hoop center.
    pub fn rotation_transform(&self) -> Transform {
        let c = self.hoop.center;
        Transform::translate(-c.x, -c.y)
            .then(Transform::rotate_deg(f64::from(self.rotation.degrees())))
            .then(Transform::translate(c.x, c.y))
    }

    /// From view coordinates (after the design rotation) to the hoop frame:
    /// the origin at the hoop center, the hoop straight (ADR 0032).
    pub fn hoop_transform(&self) -> Transform {
        let c = self.hoop.center;
        Transform::translate(-c.x, -c.y).then(Transform::rotate_deg(-self.hoop.angle_deg))
    }

    /// From the hoop frame (machine coordinates) back to view coordinates.
    pub fn from_hoop_transform(&self) -> Transform {
        let c = self.hoop.center;
        Transform::rotate_deg(self.hoop.angle_deg).then(Transform::translate(c.x, c.y))
    }

    /// From design coordinates to machine coordinates: the design
    /// rotation, then the hoop frame (ADR 0032).
    pub fn output_transform(&self) -> Transform {
        self.rotation_transform().then(self.hoop_transform())
    }

    /// The point of an anchor, in design coordinates. `None` for a place on
    /// the design box of a design with no objects that sew.
    pub fn anchor_point(&self, a: Anchor) -> Option<Point> {
        match a {
            Anchor::DesignBox { x, y } => {
                let out = self.output_transform();
                let v = self
                    .objects
                    .iter()
                    .filter(|o| o.kind.sews())
                    .flat_map(|o| {
                        o.kind
                            .paths()
                            .filter_map(|p| p.transformed(&out).control_bbox())
                            .collect::<Vec<_>>()
                    })
                    .reduce(BBox::union)?;
                let p = Point::new(v.min.x + x * v.width(), v.min.y + y * v.height());
                Some(out.inverse()?.apply(p))
            }
            Anchor::HoopCenter => Some(self.hoop.center),
            Anchor::Point { point } => Some(point),
        }
    }

    /// With auto centering, puts the hoop center at the center of the
    /// design. `Document` calls this after each edit.
    pub fn settle_hoop(&mut self) {
        if self.hoop.auto_center
            && let Some(b) = self.extents()
        {
            self.hoop.center = b.center();
        }
    }

    /// As [`Design::color_blocks`], for the objects that sew: vectors (ADR
    /// 0024) are left out, so they do not split a color block.
    pub fn sewn_color_blocks(&self) -> Vec<(ThreadId, Vec<ObjectId>)> {
        let mut blocks: Vec<(ThreadId, Vec<ObjectId>)> = Vec::new();
        for o in self.objects.iter().filter(|o| o.kind.sews()) {
            match blocks.last_mut() {
                Some((t, ids)) if *t == o.thread => ids.push(o.id),
                _ => blocks.push((o.thread, vec![o.id])),
            }
        }
        blocks
    }

    /// Groups objects that follow each other with the same thread. This is
    /// the structure of the Sequence panel.
    pub fn color_blocks(&self) -> Vec<(ThreadId, Vec<ObjectId>)> {
        let mut blocks: Vec<(ThreadId, Vec<ObjectId>)> = Vec::new();
        for o in &self.objects {
            match blocks.last_mut() {
                Some((t, ids)) if *t == o.thread => ids.push(o.id),
                _ => blocks.push((o.thread, vec![o.id])),
            }
        }
        blocks
    }

    /// Compares all saved fields. Revisions are not compared.
    pub fn same_content(&self, o: &Design) -> bool {
        self.name == o.name
            && self.threads == o.threads
            && self.hoop == o.hoop
            && self.machine == o.machine
            && self.rotation == o.rotation
            && self.backdrop == o.backdrop
            && self.guides == o.guides
            && self.background == o.background
            && self.colorways == o.colorways
            && self.colorway == o.colorway
            && self.notes == o.notes
            && self.fabric == o.fabric
            && self.current == o.current
            && self.styles == o.styles
            && self.monograms == o.monograms
            && self.team_names == o.team_names
            && self.blends == o.blends
            && self.border_traces == o.border_traces
            && self.shape_links == o.shape_links
            && self.sketch_links == o.sketch_links
            && self.start_end == o.start_end
            && self.hooping == o.hooping
            && self.product == o.product
            && self.repeat == o.repeat
            && self.template_marks == o.template_marks
            && self.objects.len() == o.objects.len()
            && self.objects.iter().zip(&o.objects).all(|(a, b)| a.same_content(b))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use emb_geom::Path;

    use super::*;
    use crate::RunningParams;

    pub(crate) fn line(x0: f64, x1: f64) -> ObjectKind {
        ObjectKind::Running {
            path: Path::new().move_to(Point::new(x0, 0.0)).line_to(Point::new(x1, 0.0)),
            params: RunningParams::default(),
        }
    }

    #[test]
    fn object_mut_copies_on_write() {
        let mut d = Design::new("t");
        let t = d.add_thread("Red", Rgb(255, 0, 0));
        let id = d.add_object(t, line(0.0, 10.0));
        let snapshot = d.clone();
        let rev0 = d.object(id).unwrap().revision;

        d.object_mut(id).unwrap().name = Some("changed".into());

        assert!(d.object(id).unwrap().revision > rev0);
        assert_eq!(snapshot.object(id).unwrap().revision, rev0);
        assert_eq!(snapshot.object(id).unwrap().name, None);
    }

    #[test]
    fn unchanged_objects_stay_shared() {
        let mut d = Design::new("t");
        let a = d.add_object(ThreadId(0), line(0.0, 1.0));
        let b = d.add_object(ThreadId(0), line(2.0, 3.0));
        let snapshot = d.clone();
        d.object_mut(a).unwrap().name = Some("x".into());
        let ib = d.index_of(b).unwrap();
        assert!(Arc::ptr_eq(&d.objects()[ib], &snapshot.objects()[ib]));
    }

    #[test]
    fn color_blocks_follow_sew_order() {
        let mut d = Design::new("t");
        let a = d.add_object(ThreadId(0), line(0.0, 1.0));
        let b = d.add_object(ThreadId(0), line(0.0, 1.0));
        let c = d.add_object(ThreadId(1), line(0.0, 1.0));
        let e = d.add_object(ThreadId(0), line(0.0, 1.0));
        assert_eq!(
            d.color_blocks(),
            vec![
                (ThreadId(0), vec![a, b]),
                (ThreadId(1), vec![c]),
                (ThreadId(0), vec![e])
            ]
        );
        d.move_object(e, 0);
        assert_eq!(d.color_blocks()[0], (ThreadId(0), vec![e, a, b]));
    }

    #[test]
    fn output_transform_turns_the_hoop_back() {
        let mut d = Design::new("t");
        d.hoop.center = Point::new(10.0, 0.0);
        d.hoop.angle_deg = 90.0;
        // A point right of the center is above it (at the back) in a hoop
        // that turned 90° clockwise.
        let p = d.output_transform().apply(Point::new(20.0, 0.0));
        assert!(p.distance(Point::new(0.0, -10.0)) < 1e-9, "{p:?}");
        d.rotation = Rotation::Deg90;
        let q = d.output_transform().apply(Point::new(20.0, 0.0));
        assert!(q.distance(Point::new(10.0, 0.0)) < 1e-9, "{q:?}");
    }

    #[test]
    fn anchors_on_the_design_box_follow_the_hoop() {
        let mut d = Design::new("t");
        d.add_object(ThreadId(0), line(10.0, 30.0));
        let left = Anchor::DesignBox { x: 0.0, y: 0.5 };
        assert_eq!(d.anchor_point(left), Some(Point::new(10.0, 0.0)));
        // Turned 90°, the line goes down from its start: its top is the start.
        d.rotation = Rotation::Deg90;
        let top = d.anchor_point(Anchor::DesignBox { x: 0.5, y: 0.0 }).unwrap();
        assert!(top.distance(Point::new(10.0, 0.0)) < 1e-9, "{top:?}");
        assert_eq!(d.anchor_point(Anchor::HoopCenter), Some(Point::ZERO));
        assert_eq!(Design::new("e").anchor_point(Anchor::DESIGN_CENTER), None);
    }

    #[test]
    fn auto_center_follows_the_design() {
        let mut doc = crate::Document::new(Design::new("t"));
        doc.edit("Auto", |d| d.hoop.auto_center = true);
        doc.edit("Add", |d| d.add_object(ThreadId(0), line(10.0, 30.0)));
        assert_eq!(doc.design().hoop.center, Point::new(20.0, 0.0));
        doc.undo();
        assert_eq!(doc.design().hoop.center, Point::ZERO);
    }

    #[test]
    fn rotation_about_hoop_center() {
        let mut d = Design::new("t");
        d.hoop.center = Point::new(10.0, 0.0);
        d.rotation = Rotation::Deg90;
        let p = d.rotation_transform().apply(Point::new(20.0, 0.0));
        assert_eq!(p, Point::new(10.0, 10.0));
    }
}
