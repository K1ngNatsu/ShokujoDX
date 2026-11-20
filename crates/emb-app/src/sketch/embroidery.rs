//! Embroidery from a sketch (ADR 0075, ADR 0076): the regions under the
//! pointer and the picked regions, and the objects from regions and
//! curves.

use super::*;

impl AppState {
    /// The closed regions of the open sketch in sketch coordinates (made
    /// again when the design changes).
    pub fn sketch_regions(&mut self) -> Vec<SketchRegion> {
        let state = self.doc.state_id();
        if let Some((s, r)) = self.sketch.as_ref().and_then(|e| e.regions_cache.clone())
            && s == state
        {
            return r;
        }
        let r = self.sketch_obj().map(|o| o.sketch().regions()).unwrap_or_default();
        if let Some(e) = &mut self.sketch {
            e.regions_cache = Some((state, r.clone()));
        }
        r
    }

    /// The smallest region at sketch point `at`.
    fn sketch_region_at(&mut self, at: Point) -> Option<SketchRegion> {
        self.sketch_regions()
            .into_iter()
            .filter(|r| r.contains(at))
            .min_by(|a, b| a.area.total_cmp(&b.area))
    }

    /// A click of the Regions tool: picks the region at `at`, or unpicks it.
    pub(super) fn toggle_region(&mut self, at: Point) {
        let Some(r) = self.sketch_region_at(at) else {
            self.message = tr("Click inside a closed region.").into();
            return;
        };
        let Some(e) = &mut self.sketch else { return };
        let before = e.picked.len();
        e.picked.retain(|p| !r.contains(*p));
        if e.picked.len() == before {
            e.picked.push(at);
        }
        let n = e.picked.len();
        self.message = fill("{} regions picked.", &[&n]);
    }

    /// The picked regions of the open sketch in design coordinates, else
    /// all its regions.
    pub(super) fn picked_regions(&mut self) -> Vec<SketchRegion> {
        let all = self.sketch_regions();
        let picked = self.sketch.as_ref().map(|e| e.picked.clone()).unwrap_or_default();
        let Some(t) = self.sketch_obj().map(|o| o.placement()) else {
            return Vec::new();
        };
        let mut out: Vec<SketchRegion> = Vec::new();
        for r in &all {
            let smallest_at = |p: &Point| {
                all.iter()
                    .filter(|x| x.contains(*p))
                    .min_by(|a, b| a.area.total_cmp(&b.area))
                    .is_some_and(|x| x == r)
            };
            if picked.is_empty() || picked.iter().any(smallest_at) {
                out.push(r.transformed(&t));
            }
        }
        out
    }

    /// The color of the current thread (gray if there is none).
    fn current_color(&self) -> emb_model::Rgb {
        self.current_thread_id()
            .and_then(|t| self.doc.design().threads.iter().find(|x| x.id == t).map(|x| x.color))
            .unwrap_or(emb_model::Rgb(90, 90, 90))
    }

    /// Makes the objects of `sources` from sketch `sketch` with the current
    /// properties and the current thread, linked to the sketch (ADR 0076):
    /// one undo step. Returns the new objects.
    pub(super) fn make_linked(&mut self, label: &str, sketch: ObjectId, sources: Vec<LinkSource>) -> Vec<ObjectId> {
        let current = self.doc.design().current.clone();
        let thread = self.current_thread_id();
        let color = self.current_color();
        self.doc.edit(label, |d| {
            let t = thread.unwrap_or_else(|| d.add_thread("Thread 1", color));
            sources
                .into_iter()
                .flat_map(|s| d.make_from_sketch(sketch, s, &current, t))
                .collect()
        })
    }

    /// Convert (Artwork > Convert) with sketches selected, outside sketch
    /// mode: To Fill and To Satin make fills or satins of all the closed
    /// regions of each sketch, To Outline makes runs along its curves (not
    /// the construction curves). The objects follow the sketch. Returns the
    /// number of sketches.
    pub fn convert_sketches(&mut self, to: emb_model::ConvertTo) -> usize {
        if self.sketch.is_some() {
            return 0;
        }
        let d = self.doc.design();
        let sketches: Vec<(ObjectId, SketchObject)> = self
            .sel()
            .into_iter()
            .filter_map(|id| match d.object(id).map(|o| &o.kind) {
                Some(ObjectKind::Sketch(s)) => Some((id, (**s).clone())),
                _ => None,
            })
            .collect();
        let mut made = 0;
        for (id, s) in &sketches {
            match to {
                emb_model::ConvertTo::Fill | emb_model::ConvertTo::Satin => {
                    let region_to = if to == emb_model::ConvertTo::Fill {
                        RegionTo::Fill
                    } else {
                        RegionTo::Satin
                    };
                    self.make_regions(*id, s.regions(), region_to);
                }
                emb_model::ConvertTo::Outline => {
                    let sources: Vec<LinkSource> = s
                        .sketch()
                        .curves
                        .iter()
                        .filter(|c| !c.construction)
                        .map(|c| LinkSource::Curve {
                            curve: c.id,
                            to: CurveTo::Run,
                        })
                        .collect();
                    made += self.make_linked("Sketch to Stitches", *id, sources).len();
                    self.message = fill("{} objects made from the sketch.", &[&made]);
                }
            }
        }
        sketches.len()
    }

    /// Makes embroidery objects of the picked regions of the open sketch,
    /// else of all its regions.
    pub fn make_picked_regions(&mut self, to: RegionTo) {
        let Some(id) = self.sketch.as_ref().map(|e| e.object) else {
            return;
        };
        let regions = self.picked_regions();
        self.make_regions(id, regions, to);
    }

    /// Makes embroidery objects of `regions` (design coordinates) of sketch
    /// `sketch` with the current properties and the current thread: one
    /// undo step.
    pub fn make_regions(&mut self, sketch: ObjectId, regions: Vec<SketchRegion>, to: RegionTo) {
        if regions.is_empty() {
            self.message = tr("The sketch has no closed regions.").into();
            return;
        }
        let Some(obj) = self.sketch_of(sketch) else { return };
        let width_mm = self.settings.defaults.column_width_mm;
        let color = self.current_color();
        let sources: Vec<LinkSource> = regions
            .iter()
            .map(|r| LinkSource::Region {
                key: obj.region_key(r),
                to,
                width_mm,
                color,
            })
            .collect();
        let made = self.make_linked(&fill("Sketch to {}", &[&tr(to.name())]), sketch, sources);
        if let Some(e) = &mut self.sketch {
            e.picked.clear();
        }
        self.message = if made.is_empty() {
            tr("Nothing could be made from these regions.").to_string()
        } else {
            fill("{} objects made from the sketch.", &[&made.len()])
        };
    }

    /// Makes runs or satin lines along the selected curves of the open
    /// sketch (also construction curves).
    pub fn make_curves(&mut self, to: CurveTo) {
        let curves = self.selected_curves();
        let Some(id) = self.sketch.as_ref().map(|e| e.object) else {
            return;
        };
        if curves.is_empty() {
            self.message = tr("Select the curves first.").into();
            return;
        }
        let sources = curves
            .into_iter()
            .map(|curve| LinkSource::Curve { curve, to })
            .collect();
        let n = self.make_linked("Sketch to Stitches", id, sources).len();
        self.message = fill("{} objects made from the sketch.", &[&n]);
    }

    /// A satin column between the two selected curves.
    pub fn make_satin_between(&mut self) {
        let curves = self.selected_curves();
        let [a, b] = curves.as_slice() else {
            self.message = tr("Select two curves for a satin column between them.").into();
            return;
        };
        let Some(id) = self.sketch.as_ref().map(|e| e.object) else {
            return;
        };
        let source = LinkSource::SatinBetween { a: *a, b: *b };
        if self.make_linked("Sketch to Satin", id, vec![source]).is_empty() {
            return;
        }
        self.message = tr("Satin column made between the two curves.").into();
    }

    pub(super) fn selected_curves(&self) -> Vec<CurveId> {
        self.sketch
            .as_ref()
            .map(|e| {
                e.selection
                    .iter()
                    .filter_map(|p| match p {
                        Pick::Curve(c) => Some(*c),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
