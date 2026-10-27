//! Design information and the warnings list (roadmap P3, Milestone 4; spec
//! UX rule 6).
//!
//! - The numbers come from the stitch plan: size, stitches, colors, trims,
//!   stops, jumps, the sew time, and for each color its stitches and the
//!   length of thread on the fabric.
//! - The checks look at each object: stitches outside the hoop field or the
//!   safety margin, density too high, stitches too short, many trims, no
//!   stitches. A click on a warning selects the object and shows it.

use std::sync::Arc;

use eframe::egui::{self, Color32, RichText};
use emb_geom::{BBox, Point};
use emb_model::{HoopSelection, ObjectId, ObjectKind, Rgb, StitchKind};

use crate::app::AppState;
use crate::canvas::color32;
use crate::i18n::{fill, tr};

/// Rows or columns closer than this are "too dense" (mm).
pub const DENSE_MM: f64 = 0.25;
/// More trims than this inside one object give a warning.
pub const MANY_TRIMS: usize = 4;
/// What the thread use is (the tooltip of the column and the Options).
pub const THREAD_USE_HELP: &str = "An estimate from the stitches: the thread on the front, the part that the tension pulls to the back, the thickness of the fabric at each needle point and a tail at each start and trim. The tension, the thread and the fabric change it.";
/// The most places of thread build-up in the warnings.
pub const MAX_BUILD_UP: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Blocks the export (a stitch outside the hoop field).
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub severity: Severity,
    pub object: Option<ObjectId>,
    pub text: String,
    /// Where to look, in canvas coordinates.
    pub at: Option<Point>,
}

/// One color block of the plan.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorInfo {
    pub name: String,
    pub color: Rgb,
    pub stitches: usize,
    /// Length of the stitches on the fabric, in mm.
    pub length_mm: f64,
    /// The top thread that the color uses, in mm (`emb_model::thread_use`).
    pub thread_mm: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DesignInfo {
    pub size: Option<(f64, f64)>,
    pub objects: usize,
    pub stitches: usize,
    pub jumps: usize,
    pub trims: usize,
    pub stops: usize,
    /// The thread changes of a multi-needle machine (ADR 0063): the design
    /// has more threads than the machine has needles.
    pub thread_changes: usize,
    pub colors: Vec<ColorInfo>,
    /// The number of different threads (a thread that sews two times is
    /// one), for the quote (ADR 0064).
    pub threads: usize,
    /// Sew time in minutes (see `SewTimes`).
    pub minutes: Option<f64>,
    /// The top and bobbin thread of the design, and its cost if the
    /// Options have prices.
    pub top_mm: f64,
    pub bobbin_mm: f64,
    pub cost: Option<String>,
    /// The most layers of thread of the density map.
    pub max_layers: Option<f64>,
    /// The number of threads to buy for the order (ADR 0066), if the user
    /// has a thread stock.
    pub to_buy: Option<usize>,
    pub issues: Vec<Issue>,
}

impl DesignInfo {
    pub fn errors(&self) -> usize {
        self.issues.iter().filter(|i| i.severity == Severity::Error).count()
    }
}

impl AppState {
    /// In open-field mode: the hoops that a design of `w` × `h` mm fits
    /// (inside the safety margin, straight or turned 90°), smallest first,
    /// at most four (spec, "Open-field mode").
    pub fn hoops_that_fit(&self, w: f64, h: f64) -> Vec<String> {
        let m = self.margin_mm;
        let mut fits: Vec<&emb_model::HoopPreset> = self.hoops.fitting(w, h, m);
        for t in self.hoops.fitting(h, w, m) {
            if !fits.iter().any(|f| f.id == t.id) {
                fits.push(t);
            }
        }
        fits.retain(|f| f.positions.is_empty());
        fits.sort_by(|a, b| (a.width_mm * a.height_mm).total_cmp(&(b.width_mm * b.height_mm)));
        fits.into_iter().take(4).map(|f| f.name.clone()).collect()
    }

    /// The design information of the stitch plan that shows (cached).
    pub fn design_info(&mut self) -> Arc<DesignInfo> {
        let state = self.shown_plan().state;
        let margin = self.margin_mm;
        if let Some((s, m, info)) = &self.info_cache
            && *s == state
            && *m == margin
        {
            return info.clone();
        }
        let info = Arc::new(self.compute_info());
        self.info_cache = Some((state, margin, info.clone()));
        info
    }

    fn compute_info(&mut self) -> DesignInfo {
        let opts = self.plan_options();
        let plan = self.shown_plan().plan.clone();
        let density = self.density_map();
        let d = self.doc.design();
        let s = &plan.stitches;
        let recs = &s.records;
        let mut info = DesignInfo {
            size: s.extents().map(|b| (b.width(), b.height())),
            objects: d.objects().len(),
            stitches: s.stitch_count(),
            ..DesignInfo::default()
        };
        let times = self.sew_times_of(d.machine.as_deref());

        // Counts and colors.
        let thread = |i: usize| s.threads.get(i);
        let mut block = 0;
        let mut colors = vec![ColorInfo {
            name: thread(0).map_or("Thread 1".into(), |t| t.name.clone()),
            color: thread(0).map_or(emb_render::FALLBACK_COLOR, |t| t.color),
            stitches: 0,
            length_mm: 0.0,
            thread_mm: 0.0,
        }];
        let mut prev: Option<(StitchKind, Point)> = None;
        for r in recs {
            match r.kind {
                StitchKind::Stitch => {
                    colors[block].stitches += 1;
                    if let Some((StitchKind::Stitch, p)) = prev {
                        colors[block].length_mm += p.distance(r.pos);
                    }
                }
                StitchKind::Jump => info.jumps += 1,
                StitchKind::Trim => info.trims += 1,
                StitchKind::Stop => info.stops += 1,
                StitchKind::Slow | StitchKind::Fast => {}
                StitchKind::ColorChange => {
                    block += 1;
                    colors.push(ColorInfo {
                        name: thread(block).map_or(format!("Thread {}", block + 1), |t| t.name.clone()),
                        color: thread(block).map_or(emb_render::FALLBACK_COLOR, |t| t.color),
                        stitches: 0,
                        length_mm: 0.0,
                        thread_mm: 0.0,
                    });
                }
                StitchKind::End => {}
            }
            prev = Some((r.kind, r.pos));
        }
        // Thread use, on the fabric of the design (P11 Milestone 1).
        let thickness = self.fabrics.of(d).thickness_mm;
        let thread = emb_model::thread_use::thread_use(s, thickness);
        for (c, u) in colors.iter_mut().zip(&thread.colors) {
            c.thread_mm = u.top_mm;
        }
        info.top_mm = thread.top_mm;
        info.bobbin_mm = thread.bobbin_mm;
        let prices = &self.settings.thread_prices;
        info.cost = prices.is_set().then(|| prices.cost_text(&thread));
        if info.stitches > 0 {
            info.colors = colors;
        }
        info.threads = emb_model::quote::thread_count(s);
        if !self.settings.thread_stock.is_empty() {
            let quantity = d.order.as_ref().map_or(1, |o| o.total_quantity());
            let buy = emb_model::stock::threads_to_buy(
                &self.settings.thread_stock,
                s,
                thickness,
                quantity,
                self.settings.thread_prices.spool_m,
            );
            info.to_buy = Some(buy.len());
        }
        let needles = self.catalogs().needle_plan(d, s);
        info.thread_changes = needles.as_ref().map_or(0, |p| p.changes.len());
        if let Some(p) = needles.as_ref().filter(|p| !p.changes.is_empty()) {
            info.issues.push(Issue {
                severity: Severity::Warning,
                object: None,
                text: crate::i18n::fill(
                    "The design has more threads than the machine has needles: {} thread changes. See Design > Needles.",
                    &[&p.changes.len()],
                ),
                at: None,
            });
        }
        info.minutes = times.map(|t| {
            t.minutes(
                info.stitches,
                info.colors.len().saturating_sub(1),
                info.trims,
                info.stops + info.thread_changes,
            )
        });

        // Checks for each object.
        let hoop = match &d.hoop.selection {
            HoopSelection::Preset { id } => self.hoops.get(id),
            HoopSelection::OpenField => None,
        };
        let back = d.from_hoop_transform();
        for span in &plan.spans {
            let id = span.object;
            let r = &recs[span.records.clone()];
            let mut issue = |severity, text: String, at: Option<Point>| {
                info.issues.push(Issue {
                    severity,
                    object: Some(id),
                    text,
                    at,
                });
            };
            let stitches = r.iter().filter(|x| x.kind == StitchKind::Stitch);
            if let Some(h) = hoop {
                if let Some(p) = stitches.clone().find(|x| !h.contains(x.pos, 0.0)) {
                    issue(
                        Severity::Error,
                        tr("Stitches outside the hoop field").into(),
                        Some(back.apply(p.pos)),
                    );
                } else if let Some(p) = stitches.clone().find(|x| !h.contains(x.pos, self.margin_mm)) {
                    issue(
                        Severity::Warning,
                        crate::i18n::fill(
                            "Stitches outside the {} mm safety margin",
                            &[&format!("{:.0}", self.margin_mm)],
                        ),
                        Some(back.apply(p.pos)),
                    );
                }
            }
            let short: Vec<Point> = r
                .windows(2)
                .filter(|w| w[0].kind == StitchKind::Stitch && w[1].kind == StitchKind::Stitch)
                .filter(|w| {
                    let len = w[0].pos.distance(w[1].pos);
                    len > 1e-6 && len < opts.min_stitch_mm - 1e-9
                })
                .map(|w| w[1].pos)
                .collect();
            if let Some(&p) = short.first() {
                issue(
                    Severity::Warning,
                    crate::i18n::fill(
                        "Stitches shorter than {} mm: {} (thread breaks)",
                        &[&format!("{:.1}", opts.min_stitch_mm), &short.len()],
                    ),
                    Some(back.apply(p)),
                );
            }
            // Appliqué trims between its rings at each step, and lettering between
            // its letters; another start does not help.
            let applique = d
                .object(id)
                .is_some_and(|o| matches!(o.kind, ObjectKind::Applique { .. } | ObjectKind::Lettering(_)));
            let trims = r.iter().filter(|x| x.kind == StitchKind::Trim).count();
            if trims > MANY_TRIMS && !applique {
                issue(
                    Severity::Warning,
                    crate::i18n::fill(
                        "{} trims in one object (slow to sew; try another start or end point)",
                        &[&trims],
                    ),
                    r.first().map(|x| back.apply(x.pos)),
                );
            }
            if let Some(text) = d.object(id).and_then(|o| match &o.kind {
                ObjectKind::Lettering(l) => l.size_warning(),
                _ => None,
            }) {
                issue(Severity::Warning, text, r.first().map(|x| back.apply(x.pos)));
            }
            let dense = d.object(id).and_then(|o| match &o.kind {
                ObjectKind::Fill { params, .. } => Some((
                    tr("Density too high: rows {} mm apart (normal 0.40 mm)"),
                    params.spacing_mm,
                )),
                ObjectKind::Satin { params, .. } => Some((
                    tr("Density too high: stitches {} mm apart (normal 0.40 mm)"),
                    params.spacing_mm,
                )),
                _ => None,
            });
            if let Some((what, spacing)) = dense.filter(|(_, s)| *s < DENSE_MM) {
                issue(
                    Severity::Warning,
                    crate::i18n::fill(what, &[&format!("{spacing:.2}")]),
                    r.first().map(|x| back.apply(x.pos)),
                );
            }
        }
        for w in &plan.warnings {
            let at = w.object.and_then(|id| d.bbox_of(&[id])).map(|b| b.center());
            info.issues.push(Issue {
                severity: Severity::Warning,
                object: w.object,
                text: crate::i18n::fill("No stitches: {}", &[&w.message]),
                at,
            });
        }
        // A cap frame: the order from the center out (ADR 0050).
        if let HoopSelection::Preset { id } = &d.hoop.selection
            && self.hoops.get(id).is_some_and(|h| h.cap)
            && d.objects().len() > 1
        {
            let ids: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
            if emb_stitch::order::cap_order(d) != ids {
                info.issues.push(Issue {
                    severity: Severity::Warning,
                    object: None,
                    text: tr(
                        "Cap frame: the sew order is not from the center out; Arrange > Sew Order > Cap Order makes it",
                    )
                    .into(),
                    at: None,
                });
            }
        }
        // Free-standing lace: each part holds to the others (ADR 0048).
        if emb_stitch::lace::is_lace(d, &self.fabrics) {
            for part in emb_stitch::lace::loose_parts(d, &plan) {
                info.issues.push(Issue {
                    severity: Severity::Warning,
                    object: Some(part.object),
                    text: crate::i18n::fill(
                        "Loose lace part: {} mm of thread that touches no other part; it falls off when the stabilizer washes away",
                        &[&format!("{:.0}", part.length_mm)],
                    ),
                    at: Some(part.center),
                });
            }
        }
        // Build-up of all objects together (the density map).
        if let Some(m) = &density {
            info.max_layers = Some(m.max_layers());
            let limit = self.settings.density_limit;
            for spot in m.hot_spots(limit).iter().take(MAX_BUILD_UP) {
                info.issues.push(Issue {
                    severity: Severity::Warning,
                    object: None,
                    text: crate::i18n::fill(
                        "Thread builds up: {} layers on {} mm² (limit {}); View > Density Map shows it",
                        &[
                            &format!("{:.1}", spot.max_layers),
                            &format!("{:.0}", spot.area_mm2),
                            &format!("{limit:.1}"),
                        ],
                    ),
                    at: Some(back.apply(spot.peak)),
                });
            }
        }
        info.issues.sort_by_key(|i| i.severity);
        info
    }

    /// Selects the object of an issue and zooms to it.
    pub fn show_issue(&mut self, issue: &Issue) {
        if let Some(id) = issue.object
            && self.doc.design().object(id).is_some()
        {
            self.selection.set(id);
            self.zoom_to_selection();
        } else if let Some(p) = issue.at {
            self.zoom_to = Some(BBox::from_center_size(p, 30.0, 30.0));
        }
    }
}

/// `m:ss` for minutes.
pub fn time_text(minutes: f64) -> String {
    let secs = (minutes * 60.0).round() as i64;
    if secs >= 3600 {
        format!("{}:{:02}:{:02}", secs / 3600, secs % 3600 / 60, secs % 60)
    } else {
        format!("{}:{:02}", secs / 60, secs % 60)
    }
}

/// The Design information panel.
pub fn panel(ui: &mut egui::Ui, st: &mut AppState) {
    let info = st.design_info();
    let u = st.settings.units;
    let fabric = st.fabric_name();
    // The price and the run time of the order of the Quote window.
    let quoted = st.current_quote();
    let price_set = st.price_list().is_set();
    let currency = st.settings.thread_prices.currency.clone();
    let order = st.doc.design().order.as_ref().map(|o| {
        [o.number.trim(), o.customer.trim()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    });
    let order = order.filter(|s| !s.is_empty());
    let fits = match (&st.doc.design().hoop.selection, info.size) {
        (HoopSelection::OpenField, Some((w, h))) => {
            let names = st.hoops_that_fit(w, h);
            Some(if names.is_empty() {
                tr("none").to_string()
            } else {
                names.join(", ")
            })
        }
        _ => None,
    };
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        egui::Grid::new("design_info")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                let mut row = |k: &str, v: String| {
                    ui.label(k);
                    ui.label(v);
                    ui.end_row();
                };
                row(
                    tr("Size"),
                    info.size
                        .map(|(w, h)| format!("{} × {}", u.fmt(w), u.fmt(h)))
                        .unwrap_or_else(|| "—".into()),
                );
                if let Some(f) = fits {
                    row(tr("Hoops it fits"), f);
                }
                row(tr("Objects"), info.objects.to_string());
                row(tr("Stitches"), info.stitches.to_string());
                row(tr("Colors"), info.colors.len().to_string());
                row(tr("Color changes"), info.colors.len().saturating_sub(1).to_string());
                row(tr("Trims"), info.trims.to_string());
                row(tr("Jumps"), info.jumps.to_string());
                if info.stops > 0 {
                    row(tr("Stops"), info.stops.to_string());
                }
                row(
                    tr("Sew time"),
                    info.minutes
                        .map(|m| crate::i18n::fill("about {}", &[&time_text(m)]))
                        .unwrap_or_else(|| tr("select a machine").into()),
                );
                row(tr("Fabric"), fabric);
                if info.stitches > 0 {
                    let len = emb_model::thread_use::length_text;
                    row(tr("Top thread"), crate::i18n::fill("about {}", &[&len(info.top_mm)]));
                    row(
                        tr("Bobbin thread"),
                        crate::i18n::fill("about {}", &[&len(info.bobbin_mm)]),
                    );
                    if let Some(c) = &info.cost {
                        row(tr("Thread cost"), c.clone());
                    }
                }
                if let Some(o) = &order {
                    row(tr("Customer order"), o.clone());
                }
                if let Some(n) = info.to_buy {
                    row(
                        tr("Threads to buy"),
                        if n == 0 {
                            tr("none (Design > Thread Stock)").to_string()
                        } else {
                            fill("{} (Design > Thread Stock)", &[&n])
                        },
                    );
                }
                if let Some((q, run)) = &quoted {
                    if price_set {
                        row(
                            tr("Price"),
                            fill(
                                "{} for each piece ({} pieces)",
                                &[&emb_model::quote::money_text(&currency, q.each), &q.quantity],
                            ),
                        );
                    }
                    if let Some(r) = run.filter(|r| r.quantity > 1) {
                        row(
                            tr("Run time"),
                            fill(
                                "about {} for {} pieces on {} heads",
                                &[&emb_model::quote::duration_text(r.minutes), &r.quantity, &r.heads],
                            ),
                        );
                    }
                }
                if let Some(l) = info.max_layers {
                    row(
                        tr("Density"),
                        crate::i18n::fill(
                            "up to {} layers of thread (limit {})",
                            &[&format!("{l:.1}"), &format!("{:.1}", st.settings.density_limit)],
                        ),
                    );
                }
            });
        if !info.colors.is_empty() {
            ui.separator();
            ui.strong(tr("Colors in sew order"));
            egui::Grid::new("design_colors")
                .num_columns(5)
                .striped(true)
                .show(ui, |ui| {
                    ui.weak("");
                    ui.weak(tr("Thread"));
                    ui.weak(tr("Stitches"));
                    ui.weak(tr("Thread on fabric"));
                    ui.weak(tr("Top thread")).on_hover_text(tr(THREAD_USE_HELP));
                    ui.end_row();
                    for (i, c) in info.colors.iter().enumerate() {
                        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(14.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, color32(c.color));
                        ui.label(format!("{}. {}", i + 1, c.name));
                        ui.label(c.stitches.to_string());
                        ui.label(if c.length_mm >= 1000.0 {
                            format!("{:.1} m", c.length_mm / 1000.0)
                        } else {
                            u.fmt(c.length_mm)
                        });
                        ui.label(emb_model::thread_use::length_text(c.thread_mm));
                        ui.end_row();
                    }
                });
        }
        ui.separator();
        crate::output::notes_ui(ui, st);
        ui.separator();
        warnings_list(ui, st, &info);
    });
}

/// The warnings with click-to-select.
pub fn warnings_list(ui: &mut egui::Ui, st: &mut AppState, info: &DesignInfo) {
    if info.issues.is_empty() {
        ui.strong(tr("No warnings"));
        return;
    }
    ui.strong(crate::i18n::fill("Warnings ({})", &[&(info.issues.len())]));
    ui.weak(tr("Click a warning to select the object."));
    let mut picked = None;
    for (i, issue) in info.issues.iter().enumerate() {
        let (mark, color) = match issue.severity {
            Severity::Error => ("⛔", Color32::from_rgb(220, 60, 50)),
            Severity::Warning => ("⚠", Color32::from_rgb(230, 150, 20)),
        };
        let name = issue.object.and_then(|id| st.doc.design().object(id)).map(|o| {
            let n = st.doc.design().index_of(o.id).map_or(0, |i| i + 1);
            format!("#{n} {}", o.name.clone().unwrap_or_else(|| o.type_name().to_string()))
        });
        let text = match name {
            Some(n) => format!("{n}: {}", issue.text),
            None => issue.text.clone(),
        };
        let r = ui.horizontal(|ui| {
            ui.label(RichText::new(mark).color(color));
            ui.add(egui::Button::new(text).frame(false))
        });
        if r.inner.clicked() {
            picked = Some(i);
        }
    }
    if let Some(i) = picked {
        st.show_issue(&info.issues[i]);
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Path;
    use emb_model::{FillParams, HoopCatalog, MachineCatalog, RunningParams};

    use super::*;

    #[test]
    fn open_field_lists_the_hoops_that_the_design_fits() {
        let st = state();
        // The 120 × 120 hoop first, then larger ones; four at most.
        let names = st.hoops_that_fit(70.0, 70.0);
        assert_eq!(names.len(), 4);
        assert!(names[0].starts_with("Small 100 x 100"), "{names:?}");
        // 250 × 150 fits only turned in the 160 × 260 hoop, and straight in some others.
        let tall = st.hoops_that_fit(250.0, 150.0);
        assert!(tall.iter().any(|n| n.starts_with("Long 160 x 260")), "{tall:?}");
        assert!(st.hoops_that_fit(1000.0, 1000.0).is_empty());
    }

    fn state() -> AppState {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.add_sample_objects();
        st
    }

    #[test]
    fn a_cap_frame_asks_for_the_cap_order() {
        let mut st = state();
        st.doc.edit("Cap frame", |d| {
            d.hoop.selection = HoopSelection::Preset {
                id: "cap_360x75".into(),
            };
            let old: Vec<ObjectId> = d.objects().iter().map(|o| o.id).collect();
            for id in old {
                d.remove_object(id);
            }
            let t = d.threads[0].id;
            for x in [60.0, 0.0, -30.0] {
                d.add_object(
                    t,
                    ObjectKind::Running {
                        path: Path::new()
                            .move_to(Point::new(x, 0.0))
                            .line_to(Point::new(x + 5.0, 0.0)),
                        params: RunningParams::default(),
                    },
                );
            }
        });
        let cap = |st: &mut AppState| st.design_info().issues.iter().any(|i| i.text.starts_with("Cap frame"));
        assert!(cap(&mut st));
        st.cap_sew_order();
        assert!(!cap(&mut st), "{:?}", st.design_info().issues);
        let xs: Vec<f64> = st
            .doc
            .design()
            .objects()
            .iter()
            .map(|o| o.control_bbox().unwrap().min.x)
            .collect();
        assert_eq!(xs, [0.0, -30.0, 60.0]);
    }

    #[test]
    fn a_lace_template_has_no_loose_part_and_a_loose_line_shows() {
        let mut st = state();
        st.new_from_lace(0);
        assert_eq!(st.doc.design().fabric.as_deref(), Some(emb_model::lace::FABRIC));
        let loose = |st: &mut AppState| {
            st.design_info()
                .issues
                .iter()
                .filter(|i| i.text.starts_with("Loose lace part"))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert!(loose(&mut st).is_empty());
        let t = st.doc.design().threads[0].id;
        let id = st.doc.edit("Add", |d| {
            d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new()
                        .move_to(Point::new(70.0, 0.0))
                        .line_to(Point::new(80.0, 0.0)),
                    params: RunningParams::default(),
                },
            )
        });
        let issues = loose(&mut st);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].object, Some(id));
        // A click selects the loose part.
        st.show_issue(&issues[0]);
        assert!(st.selection.contains(id));
    }

    #[test]
    fn numbers_of_the_sample() {
        let mut st = state();
        let info = st.design_info();
        let plan = st.plan_view().plan.clone();
        assert_eq!(info.stitches, plan.stitches.stitch_count());
        assert_eq!(info.colors.len(), 4, "red, white, gold, red");
        assert_eq!(info.colors.iter().map(|c| c.stitches).sum::<usize>(), info.stitches);
        assert!(info.colors.iter().all(|c| c.length_mm > 0.0));
        assert!(info.minutes.unwrap() > 0.0);
        assert!(info.issues.is_empty(), "{:?}", info.issues);
        // Cached: the same Arc until the design changes.
        assert!(Arc::ptr_eq(&info, &st.design_info()));
    }

    #[test]
    fn checks_find_problems() {
        let mut st = state();
        st.doc.edit("Add", |d| {
            let t = d.threads[0].id;
            // Far outside the 200 mm wide field.
            d.add_object(
                t,
                ObjectKind::Running {
                    path: Path::new()
                        .move_to(Point::new(200.0, 0.0))
                        .line_to(Point::new(220.0, 0.0)),
                    params: RunningParams::default(),
                },
            );
            // Very dense fill.
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::rect(BBox::new(Point::new(-90.0, -60.0), Point::new(-80.0, -50.0))),
                    holes: vec![],
                    params: FillParams {
                        spacing_mm: 0.15,
                        ..FillParams::default()
                    },
                },
            );
            // No area: no stitches.
            d.add_object(
                t,
                ObjectKind::Fill {
                    outer: Path::new().move_to(Point::new(0.0, 0.0)).line_to(Point::new(1.0, 0.0)),
                    holes: vec![],
                    params: FillParams::default(),
                },
            );
        });
        let info = st.design_info();
        let texts: Vec<&str> = info.issues.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(info.issues[0].severity, Severity::Error, "{texts:?}");
        assert!(texts[0].contains("outside the hoop"));
        assert!(texts.iter().any(|t| t.contains("Density too high")), "{texts:?}");
        assert!(texts.iter().any(|t| t.starts_with("No stitches")), "{texts:?}");
        assert_eq!(info.errors(), 1);

        let issue = info.issues[0].clone();
        st.show_issue(&issue);
        assert_eq!(st.selection.single(), issue.object);
        assert!(st.zoom_to.is_some());
    }

    #[test]
    fn times() {
        assert_eq!(time_text(0.5), "0:30");
        assert_eq!(time_text(75.25), "1:15:15");
    }

    #[test]
    fn panel_draws() {
        let mut st = state();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| panel(ui, &mut st));
            });
            out.textures_delta.clear();
        }
    }
}
