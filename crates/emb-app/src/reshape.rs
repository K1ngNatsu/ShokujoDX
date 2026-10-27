//! The Reshape tool: edit the nodes of one object, and its start and end points.
//!
//! - Click an object to reshape it. Its nodes show: squares are corners,
//!   circles are curve (smooth) nodes. Click a node to select it; its handles show.
//! - Drag a node to move it. Drag a handle to change the curve; a curve node
//!   stays smooth, Alt makes it a corner.
//! - Double-click a node to change it between corner and curve. Double-click
//!   the outline to add a node there. Delete or Backspace removes the selected node.
//! - Satin columns show their stitch angle lines (rungs). Drag an end of a
//!   line to change the stitch direction there; right-click it to delete it.
//!   The Angle tool adds new lines.
//! - The green circle is where the object starts to sew, the red square where
//!   it ends. Drag them to set the start and end points. The right-click menu
//!   has all commands.
//! - Single stones (a rhinestone object of single stones): each stone is a
//!   node. Drag it to move the stone; Delete removes it.
//! - Lettering: see `letter_edit`. The nodes of the selected letter show;
//!   a letter with changed nodes keeps its new shape.
//!
//! Each drag is one undo step (see `manipulate` for the method).

use eframe::egui::{self, Color32, CursorIcon, Pos2, Rect, Stroke, Vec2};
use emb_geom::nodes::{NodeId, NodePath, SegId, Side};
use emb_geom::{Path, Point};
use emb_model::{DesignObject, MergeKey, ObjectId, ObjectKind};

use crate::app::AppState;
use crate::canvas;
use crate::i18n::tr;
use crate::manipulate::{DragOp, ViewMap};

/// Hit distance for nodes, handles and markers, in screen points.
const HIT: f32 = 7.0;
/// Hit distance for the outline, in screen points.
const OUTLINE_HIT: f32 = 6.0;
const NODE: f32 = 4.0;

/// Reshape state that is not in the undo history.
#[derive(Debug, Clone, Default)]
pub struct ReshapeState {
    /// The selected node: path index in the object (see `ObjectKind::paths`) and node.
    pub node: Option<(usize, NodeId)>,
    /// The selected letter of a lettering object (index in `Lettering::letters`).
    pub letter: Option<(ObjectId, usize)>,
}

/// Start or end marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    Start,
    End,
}

/// What is under the pointer.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Marker(Marker),
    /// Rung index, and the end on rail B.
    Rung(usize, bool),
    Handle(usize, NodeId, Side),
    Node(usize, NodeId),
    Outline(usize, SegId, f64),
}

/// The selected object.
fn target(st: &AppState) -> Option<DesignObject> {
    let id = st.selection.single()?;
    st.doc.design().object(id).cloned()
}

/// Single stones as a path with one node for each stone (each stone a
/// subpath with no segments), so Reshape moves them as nodes.
fn stone_nodes(kind: &ObjectKind) -> Option<Path> {
    match kind {
        ObjectKind::Bling(b) => match &b.layout {
            emb_model::BlingLayout::Stones { points } => Some(Path {
                cmds: points.iter().map(|&p| emb_geom::PathCmd::MoveTo(p)).collect(),
            }),
            _ => None,
        },
        _ => None,
    }
}

/// The paths to show nodes for, with their indexes in the object (see
/// `ObjectKind::paths`). For lettering: the paths of the selected letter.
fn node_paths(st: &AppState, obj: &DesignObject) -> Vec<(usize, NodePath)> {
    if let Some(p) = stone_nodes(&obj.kind) {
        return vec![(0, NodePath::from_path(&p))];
    }
    let range = match &obj.kind {
        ObjectKind::Lettering(l) => match crate::letter_edit::selected_letter(st) {
            Some(i) => l.paths_of_letter(i),
            None => 0..0,
        },
        _ => 0..usize::MAX,
    };
    obj.kind
        .paths()
        .enumerate()
        .filter(|(k, _)| range.contains(k))
        .map(|(k, p)| (k, NodePath::from_path(p)))
        .collect()
}

/// Start and end of the object in design coordinates, from the stitch plan.
fn markers(st: &mut AppState, id: ObjectId) -> Option<(Point, Point)> {
    let d = st.doc.design();
    let inv = d.output_transform().inverse()?;
    let span = st.shown_plan().plan.spans.iter().find(|s| s.object == id).cloned()?;
    Some((inv.apply(span.start), inv.apply(span.end)))
}

fn hit(st: &mut AppState, map: &ViewMap, pos: Pos2) -> Option<Hit> {
    let obj = target(st)?;
    if let Some((s, e)) = markers(st, obj.id) {
        // The end marker is on top (they are at the same place for a closed loop).
        if map.to_screen(e).distance(pos) <= HIT {
            return Some(Hit::Marker(Marker::End));
        }
        if map.to_screen(s).distance(pos) <= HIT {
            return Some(Hit::Marker(Marker::Start));
        }
    }
    if let emb_model::ObjectKind::Satin { geometry, .. } = &obj.kind {
        for (i, (a, b)) in geometry.rungs.iter().enumerate() {
            for (p, b_end) in [(*a, false), (*b, true)] {
                if map.to_screen(p).distance(pos) <= HIT {
                    return Some(Hit::Rung(i, b_end));
                }
            }
        }
    }
    let paths = node_paths(st, &obj);
    // The selected node first, then (an option) all nodes.
    let all = st.settings.editing.all_handles;
    let with_handles = st.reshape.node.into_iter().chain(
        paths
            .iter()
            .filter(|_| all)
            .flat_map(|(k, np)| np.nodes().map(move |(n, _)| (*k, n))),
    );
    for (k, n) in with_handles {
        let Some((_, np)) = paths.iter().find(|(j, _)| *j == k) else {
            continue;
        };
        for side in [Side::In, Side::Out] {
            if let Some(h) = np.handle(n, side)
                && map.to_screen(h).distance(pos) <= HIT
                && np.node(n).is_some_and(|p| p.distance(h) > 1e-9)
            {
                return Some(Hit::Handle(k, n, side));
            }
        }
    }
    for (k, np) in &paths {
        for (n, p) in np.nodes() {
            if map.to_screen(p).distance(pos) <= HIT {
                return Some(Hit::Node(*k, n));
            }
        }
    }
    let p = map.to_design(pos);
    let tol = f64::from(OUTLINE_HIT) / map.view.scale();
    paths
        .iter()
        .filter_map(|(k, np)| np.nearest(p).map(|(s, t, d)| (*k, s, t, d)))
        .filter(|x| x.3 <= tol)
        .min_by(|a, b| a.3.total_cmp(&b.3))
        .map(|(k, s, t, _)| Hit::Outline(k, s, t))
}

impl AppState {
    /// Applies `f` to path `k` of the object, as an undo step (merged while `key` is the same).
    fn edit_path(
        &mut self,
        label: &str,
        key: Option<&MergeKey>,
        id: ObjectId,
        original: &Path,
        k: usize,
        f: impl FnOnce(&mut NodePath),
    ) {
        let mut np = NodePath::from_path(original);
        f(&mut np);
        let path = np.to_path();
        let change = |d: &mut emb_model::Design| {
            if let Some(o) = d.object_mut(id)
                && let ObjectKind::Bling(b) = &mut o.kind
                && let emb_model::BlingLayout::Stones { points } = &mut b.layout
            {
                *points = path.points().collect();
            } else if let Some(o) = d.object_mut(id)
                && let Some(p) = o.kind.paths_mut().into_iter().nth(k)
            {
                *p = path;
                // A letter keeps its changed shape in a new layout.
                if let ObjectKind::Lettering(l) = &mut o.kind
                    && let Some(i) = l.letter_of_path(k)
                {
                    l.keep_shape(i);
                }
            }
        };
        match key {
            Some(key) => self.doc.edit_merged(label, key.clone(), change),
            None => self.doc.edit(label, change),
        };
    }

    fn path_of(&self, id: ObjectId, k: usize) -> Option<Path> {
        let kind = &self.doc.design().object(id)?.kind;
        stone_nodes(kind).or_else(|| kind.paths().nth(k).cloned())
    }

    /// The selected object is single stones.
    fn reshapes_stones(&self) -> bool {
        self.selection
            .single()
            .and_then(|id| self.doc.design().object(id))
            .is_some_and(|o| stone_nodes(&o.kind).is_some())
    }

    /// Changes the selected node between corner and curve.
    pub fn toggle_node(&mut self, k: usize, n: NodeId) {
        let Some(id) = self.selection.single() else { return };
        if self.reshapes_stones() {
            return;
        }
        let Some(path) = self.path_of(id, k) else { return };
        let smooth = NodePath::from_path(&path).is_smooth(n);
        let label = if smooth { "Make corner" } else { "Make curve" };
        self.edit_path(label, None, id, &path, k, |np| np.set_smooth(n, !smooth));
    }

    pub fn add_node(&mut self, k: usize, seg: SegId, t: f64) {
        let Some(id) = self.selection.single() else { return };
        let Some(path) = self.path_of(id, k) else { return };
        let mut new = None;
        self.edit_path("Add node", None, id, &path, k, |np| new = np.insert_node(seg, t));
        self.reshape.node = new.map(|n| (k, n));
    }

    /// Removes the selected node. Returns `false` if there is none or it cannot go.
    pub fn delete_node(&mut self) -> bool {
        let (Some(id), Some((k, n))) = (self.selection.single(), self.reshape.node) else {
            return false;
        };
        let Some(path) = self.path_of(id, k) else { return false };
        if self.reshapes_stones() {
            let mut np = NodePath::from_path(&path);
            if np.subs.len() < 2 || n.sub >= np.subs.len() {
                self.message = tr("The last stone cannot go; delete the object.").into();
                return false;
            }
            np.subs.remove(n.sub);
            let path = np.to_path();
            self.doc.edit("Delete stone", |d| {
                if let Some(ObjectKind::Bling(b)) = d.object_mut(id).map(|o| &mut o.kind)
                    && let emb_model::BlingLayout::Stones { points } = &mut b.layout
                {
                    *points = path.points().collect();
                }
            });
            self.reshape.node = None;
            return true;
        }
        let mut np = NodePath::from_path(&path);
        if !np.delete_node(n) {
            self.message = tr("A line needs two nodes and a closed shape three.").into();
            return false;
        }
        self.edit_path("Delete Node", None, id, &path, k, |np| {
            np.delete_node(n);
        });
        self.reshape.node = None;
        true
    }

    /// Deletes a stitch angle line (rung) of the selected satin.
    pub fn delete_rung(&mut self, i: usize) {
        let Some(id) = self.selection.single() else { return };
        self.doc.edit("Delete stitch angle", |d| {
            if let Some(emb_model::ObjectKind::Satin { geometry, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                && i < geometry.rungs.len()
            {
                geometry.rungs.remove(i);
            }
        });
    }

    /// Clears the start and end points that the user set.
    pub fn reset_start_end(&mut self) {
        let Some(id) = self.selection.single() else { return };
        self.doc.edit("Reset start and end", |d| {
            if let Some(o) = d.object_mut(id) {
                o.start = None;
                o.end = None;
            }
        });
    }
}

/// Handles canvas input for the Reshape tool.
pub fn handle_input(ui: &egui::Ui, response: &egui::Response, st: &mut AppState, map: &ViewMap) {
    if crate::letter_edit::handle_input(ui, response, st, map) {
        return;
    }
    let mods = ui.input(|i| i.modifiers);
    let hover = response.hover_pos().and_then(|p| hit(st, map, p));
    if st.drag.is_none() {
        let icon = match hover {
            Some(Hit::Marker(_) | Hit::Rung(..) | Hit::Handle(..) | Hit::Node(..)) => CursorIcon::Grab,
            Some(Hit::Outline(..)) => CursorIcon::Crosshair,
            None => CursorIcon::Default,
        };
        ui.ctx().set_cursor_icon(icon);
    }

    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
        && let Some(obj) = target(st)
    {
        st.drag_counter += 1;
        let key = MergeKey::new(format!("reshape-{}", st.drag_counter));
        match hit(st, map, origin) {
            Some(Hit::Node(k, n)) => {
                st.reshape.node = Some((k, n));
                st.drag = Some(DragOp::ReshapeNode {
                    id: obj.id,
                    path: k,
                    node: n,
                    handle: None,
                    original: stone_nodes(&obj.kind)
                        .or_else(|| obj.kind.paths().nth(k).cloned())
                        .unwrap_or_default(),
                    key,
                });
            }
            Some(Hit::Handle(k, n, side)) => {
                st.drag = Some(DragOp::ReshapeNode {
                    id: obj.id,
                    path: k,
                    node: n,
                    handle: Some(side),
                    original: stone_nodes(&obj.kind)
                        .or_else(|| obj.kind.paths().nth(k).cloned())
                        .unwrap_or_default(),
                    key,
                });
            }
            Some(Hit::Marker(which)) => st.drag = Some(DragOp::ReshapeMarker { id: obj.id, which, key }),
            Some(Hit::Rung(rung, b_end)) => {
                st.drag = Some(DragOp::ReshapeRung {
                    id: obj.id,
                    rung,
                    b_end,
                    key,
                })
            }
            _ => {}
        }
    }
    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let p = map.to_design(pos);
        match st.drag.clone() {
            Some(DragOp::ReshapeNode {
                id,
                path,
                node,
                handle,
                original,
                key,
            }) => match handle {
                None => st.edit_path("Move node", Some(&key), id, &original, path, |np| np.move_node(node, p)),
                Some(side) => st.edit_path("Move handle", Some(&key), id, &original, path, |np| {
                    np.move_handle(node, side, p, !mods.alt)
                }),
            },
            Some(DragOp::ReshapeRung { id, rung, b_end, key }) => {
                st.doc.edit_merged("Move stitch angle", key, |d| {
                    if let Some(emb_model::ObjectKind::Satin { geometry, .. }) = d.object_mut(id).map(|o| &mut o.kind)
                        && let Some(r) = geometry.rungs.get_mut(rung)
                    {
                        if b_end {
                            r.1 = p;
                        } else {
                            r.0 = p;
                        }
                    }
                });
            }
            Some(DragOp::ReshapeMarker { id, which, key }) => {
                let label = match which {
                    Marker::Start => "Move start point",
                    Marker::End => "Move end point",
                };
                st.doc.edit_merged(label, key, |d| {
                    if let Some(o) = d.object_mut(id) {
                        match which {
                            Marker::Start => o.start = Some(p),
                            Marker::End => o.end = Some(p),
                        }
                    }
                });
            }
            _ => {}
        }
    }
    if response.drag_stopped_by(egui::PointerButton::Primary)
        && matches!(
            st.drag,
            Some(DragOp::ReshapeNode { .. } | DragOp::ReshapeMarker { .. } | DragOp::ReshapeRung { .. })
        )
    {
        st.drag = None;
        st.doc.end_merge();
    }

    if response.double_clicked() {
        match hover {
            Some(Hit::Node(k, n)) => st.toggle_node(k, n),
            Some(Hit::Outline(k, s, t)) => st.add_node(k, s, t),
            _ => {}
        }
    } else if response.clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        match hover {
            Some(Hit::Node(k, n)) => st.reshape.node = Some((k, n)),
            Some(Hit::Handle(..) | Hit::Marker(_) | Hit::Rung(..) | Hit::Outline(..)) => {}
            None => {
                st.reshape.node = None;
                st.reshape.letter = None;
                match canvas::pick(st, map.view.to_mm(map.rect, pos)) {
                    Some(id) => st.selection.set(id),
                    None => st.selection.clear(),
                }
            }
        }
    }

    // Right click: the Reshape menu for what is under the pointer.
    if response.secondary_clicked() {
        st.reshape_menu_hit = hover.map(MenuHit::from);
        if let (None, Some(pos)) = (hover, response.interact_pointer_pos())
            && let Some(id) = canvas::pick(st, map.view.to_mm(map.rect, pos))
        {
            st.selection.set(id);
            st.reshape.node = None;
        }
    }
    response.context_menu(|ui| menu(ui, st));
}

/// What the right click was on, kept while the menu is open.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuHit {
    Node(usize, NodeId),
    Rung(usize),
    Outline(usize, SegId, f64),
    Other,
}

impl From<Hit> for MenuHit {
    fn from(h: Hit) -> Self {
        match h {
            Hit::Node(k, n) | Hit::Handle(k, n, _) => MenuHit::Node(k, n),
            Hit::Outline(k, s, t) => MenuHit::Outline(k, s, t),
            Hit::Rung(i, _) => MenuHit::Rung(i),
            Hit::Marker(_) => MenuHit::Other,
        }
    }
}

fn menu(ui: &mut egui::Ui, st: &mut AppState) {
    match st.reshape_menu_hit {
        Some(MenuHit::Node(k, n)) => {
            st.reshape.node = Some((k, n));
            let smooth = st
                .selection
                .single()
                .and_then(|id| st.path_of(id, k))
                .is_some_and(|p| NodePath::from_path(&p).is_smooth(n));
            if ui.add_enabled(smooth, egui::Button::new(tr("Corner Node"))).clicked()
                || ui.add_enabled(!smooth, egui::Button::new(tr("Curve Node"))).clicked()
            {
                st.toggle_node(k, n);
                ui.close();
            }
            if ui
                .add(egui::Button::new(tr("Delete Node")).shortcut_text("Del"))
                .clicked()
            {
                st.delete_node();
                ui.close();
            }
            ui.separator();
        }
        Some(MenuHit::Rung(i)) => {
            if ui.button(tr("Delete stitch angle line")).clicked() {
                st.delete_rung(i);
                ui.close();
            }
            ui.separator();
        }
        Some(MenuHit::Outline(k, s, t)) => {
            if ui.button(tr("Add node here")).clicked() {
                st.add_node(k, s, t);
                ui.close();
            }
            ui.separator();
        }
        _ => {}
    }
    let has_points = st
        .selection
        .single()
        .and_then(|id| st.doc.design().object(id))
        .is_some_and(|o| o.start.is_some() || o.end.is_some());
    if ui
        .add_enabled(has_points, egui::Button::new(tr("Reset Start and End Points")))
        .clicked()
    {
        st.reset_start_end();
        ui.close();
    }
    if ui
        .add(egui::Button::new(tr("Back to Select")).shortcut_text("Esc"))
        .clicked()
    {
        st.set_tool(crate::draw::Tool::Select);
        ui.close();
    }
}

/// Draws the nodes, the handles of the selected node (of all nodes with the
/// option of the editing settings), and the start and end points.
pub fn draw_overlay(painter: &egui::Painter, map: &ViewMap, st: &mut AppState, accent: Color32) {
    let Some(obj) = target(st) else {
        return;
    };
    crate::letter_edit::draw_overlay(painter, map, st, accent);
    let paths = node_paths(st, &obj);
    let line = Stroke::new(1.0, accent);
    if let emb_model::ObjectKind::Satin { geometry, .. } = &obj.kind {
        let rung = Stroke::new(1.0, Color32::from_rgb(230, 150, 30));
        for (a, b) in &geometry.rungs {
            let (sa, sb) = (map.to_screen(*a), map.to_screen(*b));
            painter.line_segment([sa, sb], rung);
            for s in [sa, sb] {
                let diamond = vec![
                    s + Vec2::new(0.0, -4.0),
                    s + Vec2::new(4.0, 0.0),
                    s + Vec2::new(0.0, 4.0),
                    s + Vec2::new(-4.0, 0.0),
                ];
                painter.add(egui::Shape::convex_polygon(diamond, rung.color, Stroke::NONE));
            }
        }
    }
    for (k, np) in &paths {
        for (n, p) in np.nodes() {
            let s = map.to_screen(p);
            let selected = st.reshape.node == Some((*k, n));
            let fill = if selected { accent } else { Color32::WHITE };
            if selected || st.settings.editing.all_handles {
                for side in [Side::In, Side::Out] {
                    if let Some(h) = np.handle(n, side)
                        && h.distance(p) > 1e-9
                    {
                        let hs = map.to_screen(h);
                        painter.line_segment([s, hs], line);
                        painter.circle_filled(hs, 3.5, accent);
                    }
                }
            }
            if np.is_smooth(n) {
                painter.circle_filled(s, NODE, fill);
                painter.circle_stroke(s, NODE, Stroke::new(1.5, accent));
            } else {
                let r = Rect::from_center_size(s, Vec2::splat(NODE * 2.0));
                painter.rect_filled(r, 0.0, fill);
                painter.rect_stroke(r, 0.0, Stroke::new(1.5, accent), egui::StrokeKind::Inside);
            }
        }
    }
    if let Some((s, e)) = markers(st, obj.id) {
        let (s, e) = (map.to_screen(s), map.to_screen(e));
        let ring = Stroke::new(2.0, Color32::WHITE);
        painter.circle_filled(s, 6.0, Color32::from_rgb(40, 170, 80));
        painter.circle_stroke(s, 6.0, ring);
        let r = Rect::from_center_size(e, Vec2::splat(10.0));
        painter.rect_filled(r, 0.0, Color32::from_rgb(210, 50, 50));
        painter.rect_stroke(r, 0.0, ring, egui::StrokeKind::Outside);
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Event, Key, Modifiers, PointerButton};
    use emb_geom::BBox;
    use emb_model::{FillParams, ObjectKind};

    use crate::draw::Tool;
    use crate::test_util::Harness;

    use super::*;

    /// A design with one 40 mm square fill, selected, in the Reshape tool.
    fn square() -> (Harness, ObjectId) {
        let mut h = Harness::empty();
        let id = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::new(-20.0, -20.0), Point::new(20.0, 20.0))),
                    holes: vec![],
                    params: FillParams::default(),
                },
            )
        });
        h.st.zoom_to = Some(BBox::new(Point::new(-30.0, -30.0), Point::new(30.0, 30.0)));
        h.frame(vec![]);
        h.st.set_tool(Tool::Reshape);
        h.st.selection.set(id);
        h.frame(vec![]);
        (h, id)
    }

    fn outer(h: &Harness, id: ObjectId) -> NodePath {
        NodePath::from_path(h.st.doc.design().object(id).unwrap().kind.paths().next().unwrap())
    }

    #[test]
    fn single_stones_move_and_delete_as_nodes() {
        let mut h = Harness::empty();
        let stones = vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(20.0, 0.0)];
        let id = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Stones", emb_model::Rgb(200, 200, 200));
            d.add_object(
                t,
                ObjectKind::Bling(Box::new(emb_model::Bling::new(
                    Default::default(),
                    emb_model::BlingLayout::Stones { points: stones },
                ))),
            )
        });
        h.st.zoom_to = Some(BBox::new(Point::new(-30.0, -30.0), Point::new(30.0, 30.0)));
        h.frame(vec![]);
        h.st.set_tool(Tool::Reshape);
        h.st.selection.set(id);
        h.frame(vec![]);
        let points = |h: &Harness| match &h.st.doc.design().object(id).unwrap().kind {
            ObjectKind::Bling(b) => match &b.layout {
                emb_model::BlingLayout::Stones { points } => points.clone(),
                _ => panic!(),
            },
            _ => panic!(),
        };
        let from = h.screen(Point::new(10.0, 0.0));
        h.drag(
            PointerButton::Primary,
            from,
            from + Vec2::new(0.0, 40.0),
            Modifiers::NONE,
        );
        let p = points(&h);
        assert_eq!(p.len(), 3);
        assert!(p[1].x == 10.0 && p[1].y.abs() > 2.0, "{p:?}");
        assert!(h.st.delete_node());
        assert_eq!(points(&h), vec![Point::new(0.0, 0.0), Point::new(20.0, 0.0)]);
    }

    #[test]
    fn drag_a_node_is_one_undo_step() {
        let (mut h, id) = square();
        let corner = h.screen(Point::new(20.0, 20.0));
        let to = corner + Vec2::new(30.0, 0.0);
        h.drag(PointerButton::Primary, corner, to, Modifiers::NONE);
        let np = outer(&h, id);
        let moved = np.nodes().map(|(_, p)| p).find(|p| p.y == 20.0 && p.x > 20.0);
        assert!(moved.is_some(), "{:?}", np.subs[0].nodes);
        assert!(h.st.reshape.node.is_some());
        h.st.undo();
        assert!(outer(&h, id).nodes().any(|(_, p)| p == Point::new(20.0, 20.0)));
    }

    #[test]
    fn double_click_adds_and_toggles_and_delete_removes() {
        let (mut h, id) = square();
        // Double-click the middle of the top edge: a new node.
        let at = h.screen(Point::new(0.0, -20.0));
        let click = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        h.frame(vec![Event::PointerMoved(at)]);
        for _ in 0..2 {
            h.frame(vec![click(true)]);
            h.frame(vec![click(false)]);
        }
        let np = outer(&h, id);
        assert_eq!(np.subs[0].nodes.len(), 5);
        let (k, n) = h.st.reshape.node.unwrap();
        assert_eq!(k, 0);
        assert!(!np.is_smooth(n));
        // Toggle it to a curve node from the code path the double-click uses.
        h.st.toggle_node(k, n);
        assert!(outer(&h, id).is_smooth(n));
        // Delete removes the selected node.
        h.key(Key::Delete);
        assert_eq!(outer(&h, id).subs[0].nodes.len(), 4);
        assert_eq!(
            h.st.doc.design().objects().len(),
            1,
            "Delete removed the node, not the object"
        );
    }

    #[test]
    fn satin_rungs_drag_and_delete() {
        let mut h = Harness::empty();
        let g = emb_model::SatinGeometry {
            rail_a: Path::new()
                .move_to(Point::new(-20.0, 0.0))
                .line_to(Point::new(20.0, 0.0)),
            rail_b: Path::new()
                .move_to(Point::new(-20.0, 5.0))
                .line_to(Point::new(20.0, 5.0)),
            rungs: vec![(Point::new(0.0, 0.0), Point::new(0.0, 5.0))],
        };
        let id = h.st.doc.edit("Add", |d| {
            let t = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            d.add_object(
                t,
                ObjectKind::Satin {
                    geometry: g,
                    params: Default::default(),
                },
            )
        });
        h.st.zoom_to = Some(BBox::new(Point::new(-25.0, -10.0), Point::new(25.0, 15.0)));
        h.frame(vec![]);
        h.st.set_tool(Tool::Reshape);
        h.st.selection.set(id);
        h.frame(vec![]);
        let from = h.screen(Point::new(0.0, 5.0));
        let to = h.screen(Point::new(4.0, 5.0));
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        let ObjectKind::Satin { geometry, .. } = &h.st.doc.design().object(id).unwrap().kind else {
            panic!()
        };
        assert!((geometry.rungs[0].1.x - 4.0).abs() < 0.1, "{:?}", geometry.rungs[0]);
        h.st.delete_rung(0);
        let ObjectKind::Satin { geometry, .. } = &h.st.doc.design().object(id).unwrap().kind else {
            panic!()
        };
        assert!(geometry.rungs.is_empty());
    }

    #[test]
    fn drag_the_end_marker_sets_the_end_point() {
        let (mut h, id) = square();
        let span = h.st.plan_view().plan.spans[0].clone();
        let c = h.st.doc.design().hoop.center;
        let from = h.screen(span.end + c);
        let to = h.screen(Point::new(19.0, 19.0));
        h.drag(PointerButton::Primary, from, to, Modifiers::NONE);
        let end = h.st.doc.design().object(id).unwrap().end.unwrap();
        assert!(end.distance(Point::new(19.0, 19.0)) < 0.2, "{end:?}");
        let span = h.st.plan_view().plan.spans[0].clone();
        assert!(
            (span.end + c).distance(Point::new(20.0, 20.0)) < 1.0,
            "the fill now ends near the corner: {:?}",
            span.end + c
        );
        h.st.reset_start_end();
        assert!(h.st.doc.design().object(id).unwrap().end.is_none());
    }
}
