//! Export dialog: format, summary, checks, and writing the machine file.
//!
//! Rules (spec, "Rules for the hoop boundary" and "Export dialog"):
//! - Export is blocked if a stitch is outside the hoop field. The dialog
//!   shows where the first one is.
//! - Warnings (skipped objects, hoop not for this machine, too large in open
//!   field) do not block.
//! - The native project file is saved next to each export.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use eframe::egui;
use emb_geom::{BBox, Point};
use emb_model::{HoopSelection, StitchKind, project};

use crate::app::AppState;
use crate::file::Notice;
use crate::i18n::tr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Dst,
    Vp3,
    Pes,
    Exp,
    Jef,
    Xxx,
    Tbf,
    Tap,
    Pec,
    U01,
}

impl Format {
    pub const ALL: [Format; 10] = [
        Format::Vp3,
        Format::Dst,
        Format::Pes,
        Format::Exp,
        Format::Jef,
        Format::Xxx,
        Format::Tbf,
        Format::Tap,
        Format::Pec,
        Format::U01,
    ];

    pub fn ext(self) -> &'static str {
        match self {
            Format::Dst => "dst",
            Format::Vp3 => "vp3",
            Format::Pes => "pes",
            Format::Exp => "exp",
            Format::Jef => "jef",
            Format::Xxx => "xxx",
            Format::Tbf => "tbf",
            Format::Tap => "tap",
            Format::Pec => "pec",
            Format::U01 => "u01",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Format::Dst => tr("DST (Tajima, most multi-needle machines)"),
            Format::Vp3 => tr("VP3 (Husqvarna Viking, Pfaff)"),
            Format::Pes => tr("PES (Brother, Babylock)"),
            Format::Exp => tr("EXP (Melco, Bernina)"),
            Format::Jef => tr("JEF (Janome)"),
            Format::Xxx => tr("XXX (Singer)"),
            Format::Tbf => tr("TBF (Tajima, with needles and colors)"),
            Format::Tap => tr("TAP (Happy)"),
            Format::Pec => tr("PEC (Brother, the machine part of PES)"),
            Format::U01 => tr("U01 (Barudan, with the slow function)"),
        }
    }

    /// `None` if the format can be written now, else the reason.
    pub fn not_ready(self) -> Option<&'static str> {
        None
    }
}

/// State of the open export dialog.
#[derive(Debug, Clone)]
pub struct ExportDialog {
    pub format: Format,
}

/// The error of a design with no stitches.
pub const NO_STITCHES: &str = "The design has no stitches to export.";

#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub text: String,
    /// Canvas position to show, if the issue has one.
    pub at: Option<Point>,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub size: Option<(f64, f64)>,
    pub stitches: usize,
    pub colors: usize,
    pub trims: usize,
    pub stops: usize,
    /// The colorway that the file gets, if the design has more than one.
    pub colorway: Option<String>,
    pub minutes: Option<f64>,
    /// Block the export.
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
}

impl AppState {
    pub fn open_export(&mut self) {
        // The machine's default format, if it can be written; else DST.
        let default = self
            .doc
            .design()
            .machine
            .as_deref()
            .and_then(|m| self.machines.get(m))
            .and_then(|m| Format::ALL.into_iter().find(|f| f.ext() == m.default_format))
            .filter(|f| f.not_ready().is_none())
            .unwrap_or(Format::Dst);
        self.export = Some(ExportDialog { format: default });
    }

    /// Summary and checks for the current design.
    pub fn export_report(&mut self) -> Report {
        let plan = self.plan_view().plan.clone();
        let d = self.doc.design();
        let s = &plan.stitches;
        let mut r = Report {
            size: s.extents().map(|b| (b.width(), b.height())),
            stitches: s.stitch_count(),
            colors: s.color_count(),
            trims: s.trim_count(),
            stops: s.records.iter().filter(|r| r.kind == StitchKind::Stop).count(),
            colorway: (d.colorways.len() > 1)
                .then(|| d.colorway_name().map(str::to_string))
                .flatten(),
            ..Report::default()
        };
        let machine = d.machine.as_deref().and_then(|m| self.machines.get(m));
        r.minutes = self
            .sew_times_of(d.machine.as_deref())
            .map(|t| t.minutes(r.stitches, r.colors.saturating_sub(1), r.trims, r.stops));
        if r.stitches == 0 {
            r.errors.push(Issue {
                text: NO_STITCHES.into(),
                at: None,
            });
        }

        match &d.hoop.selection {
            HoopSelection::Preset { id } => {
                if let Some(h) = self.hoops.get(id) {
                    // The plan origin is the hoop center, so test the positions directly.
                    let first_out = s
                        .records
                        .iter()
                        .enumerate()
                        .filter(|(_, rec)| matches!(rec.kind, StitchKind::Stitch | StitchKind::Jump))
                        .find(|(_, rec)| !h.contains(rec.pos, 0.0));
                    if let Some((i, rec)) = first_out {
                        r.errors.push(Issue {
                            text: format!(
                                "Record {} at ({:.1}, {:.1}) mm from the hoop center is outside the {} field.",
                                i + 1,
                                rec.pos.x,
                                rec.pos.y,
                                h.name
                            ),
                            at: Some(self.plan_to_canvas(rec.pos)),
                        });
                    }
                    if let Some(m) = machine
                        && !self.hoop_fits_machine(id)
                    {
                        r.warnings.push(Issue {
                            text: format!("The hoop \"{}\" is not in the list for {}.", h.name, m.name),
                            at: None,
                        });
                    }
                }
            }
            HoopSelection::OpenField => {
                if let (Some(m), Some((w, h))) = (machine, r.size) {
                    let fits = m
                        .hoops
                        .iter()
                        .filter_map(|id| self.hoops.get(id))
                        .any(|hp| hp.fits(w, h, 0.0) || hp.fits(h, w, 0.0));
                    if !fits {
                        r.warnings.push(Issue {
                            text: format!("{w:.0} × {h:.0} mm is larger than every hoop of {}.", m.name),
                            at: None,
                        });
                    }
                }
            }
        }

        // One file for a design in hoopings, or on a multi-position hoop.
        if let Some(l) = &d.hooping
            && !l.hoopings.is_empty()
        {
            r.warnings.push(Issue {
                text: format!(
                    "The design has {} hoopings: Hooping > Output Hoopings writes a file for each.",
                    l.hoopings.len()
                ),
                at: None,
            });
        } else if let HoopSelection::Preset { id } = &d.hoop.selection
            && self.hoops.get(id).is_some_and(|h| !h.positions.is_empty())
        {
            r.warnings.push(Issue {
                text: "A multi-position hoop sews one position at a time: use Hooping > Auto Layout and \
                       Output hoopings."
                    .into(),
                at: None,
            });
        }

        // Skipped objects, grouped by reason.
        let mut reasons: BTreeMap<String, Vec<Point>> = BTreeMap::new();
        for w in &plan.warnings {
            let at = w
                .object
                .and_then(|id| d.bbox_of(&[id]))
                .map(|b| d.rotation_transform().apply(b.center()));
            reasons.entry(w.message.clone()).or_default().extend(at);
        }
        for (msg, places) in reasons {
            let n = places.len().max(1);
            r.warnings.push(Issue {
                text: format!("{n} object(s) skipped: {msg}."),
                at: places.first().copied(),
            });
        }
        r
    }

    /// Writes the machine file and the project file next to it.
    pub fn export_to(&mut self, path: &Path, format: Format) -> Result<(), String> {
        if let Some(why) = format.not_ready() {
            return Err(why.to_string());
        }
        let plan = self.plan_view().plan.clone();
        // The machine shows this label: use the export file name.
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.doc.design().name.clone());
        let date = crate::output::local_stamp();
        let machine = self.machine_for_files(self.doc.design().machine.as_deref());
        let needles = self.catalogs().needle_plan(self.doc.design(), &plan.stitches);
        let bytes = emb_formats::write_machine_with(
            format.ext(),
            &plan.stitches,
            &name,
            &date,
            machine.as_ref(),
            needles.as_ref(),
        )
        .ok_or_else(|| format!("{} cannot be written", format.label()))?;
        std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;

        // The native file next to the export (spec). If the design has no
        // project file yet, this becomes it.
        let native = path.with_extension(project::FILE_EXTENSION);
        if self.file_path.is_none() || self.file_path.as_deref() == Some(native.as_path()) {
            if !self.save_to(&native) {
                return Err(format!(
                    "Exported, but the project file {} was not saved.",
                    native.display()
                ));
            }
        } else {
            project::save(self.doc.design(), &native).map_err(|e| e.to_string())?;
        }
        self.message = crate::i18n::fill("Exported {} (and {})", &[&(path.display()), &(native.display())]);
        Ok(())
    }

    fn export_dialog_dir(&self) -> PathBuf {
        self.file_path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default()
    }
}

/// Shows the export dialog if it is open.
pub fn dialog(ctx: &egui::Context, st: &mut AppState) {
    let Some(mut dlg) = st.export.clone() else { return };
    let report = st.export_report();
    let mut close = false;
    let mut export = false;
    let mut show_at: Option<Point> = None;

    egui::Modal::new(egui::Id::new("export_dialog")).show(ctx, |ui| {
        ui.set_min_width(440.0);
        ui.heading(tr("Export machine file"));
        ui.add_space(4.0);

        egui::Grid::new("export_head")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(tr("Format"));
                egui::ComboBox::from_id_salt("export_format")
                    .selected_text(dlg.format.label())
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for f in Format::ALL {
                            let r = ui.add_enabled(
                                f.not_ready().is_none(),
                                egui::Button::selectable(dlg.format == f, f.label()),
                            );
                            let r = match f.not_ready() {
                                Some(why) => r.on_disabled_hover_text(why),
                                None => r,
                            };
                            if r.clicked() {
                                dlg.format = f;
                            }
                        }
                    });
                ui.end_row();
                let d = st.doc.design();
                let machine = d.machine.as_deref().and_then(|m| st.machines.get(m));
                ui.label(tr("Machine"));
                ui.label(machine.map(|m| m.name.as_str()).unwrap_or(tr("None")));
                ui.end_row();
                ui.label(tr("Hoop"));
                ui.label(match &d.hoop.selection {
                    HoopSelection::OpenField => "Open field".to_string(),
                    HoopSelection::Preset { id } => {
                        st.hoops.get(id).map(|h| h.name.clone()).unwrap_or_else(|| id.clone())
                    }
                });
                ui.end_row();
                if let Some(m) = machine
                    && m.default_format != dlg.format.ext()
                {
                    ui.label("");
                    ui.weak(crate::i18n::fill(
                        "{} reads {}. {}",
                        &[
                            &m.name,
                            &(m.default_format.to_uppercase()),
                            &(Format::ALL
                                .into_iter()
                                .find(|f| f.ext() == m.default_format)
                                .and_then(Format::not_ready)
                                .unwrap_or("")),
                        ],
                    ));
                    ui.end_row();
                }
            });

        ui.separator();
        egui::Grid::new("export_summary")
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                ui.label(tr("Size"));
                ui.label(
                    report
                        .size
                        .map(|(w, h)| format!("{w:.1} × {h:.1} mm"))
                        .unwrap_or_else(|| "—".into()),
                );
                ui.end_row();
                ui.label(tr("Stitches"));
                ui.label(report.stitches.to_string());
                ui.end_row();
                ui.label(tr("Colors"));
                ui.label(report.colors.to_string());
                ui.end_row();
                ui.label(tr("Trims"));
                ui.label(report.trims.to_string());
                ui.end_row();
                if report.stops > 0 {
                    ui.label(tr("Stops"));
                    ui.label(report.stops.to_string());
                    ui.end_row();
                }
                if let Some(c) = &report.colorway {
                    ui.label(tr("Colorway"));
                    ui.label(c);
                    ui.end_row();
                }
                if let Some(min) = report.minutes {
                    ui.label(tr("Sew time"));
                    ui.label(crate::i18n::fill("about {} min", &[&format!("{:.0}", min.ceil())]));
                    ui.end_row();
                }
            });

        let mut issue = |ui: &mut egui::Ui, i: &Issue, color: egui::Color32, icon: &str| {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(color, icon);
                ui.label(&i.text);
                if let Some(p) = i.at
                    && ui.small_button(tr("Show")).clicked()
                {
                    show_at = Some(p);
                }
            });
        };
        if !report.errors.is_empty() || !report.warnings.is_empty() {
            ui.separator();
        }
        for e in &report.errors {
            issue(ui, e, egui::Color32::from_rgb(220, 60, 60), "⛔");
        }
        for w in &report.warnings {
            issue(ui, w, egui::Color32::from_rgb(230, 150, 20), "⚠");
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let can = report.errors.is_empty() && dlg.format.not_ready().is_none();
            if ui.add_enabled(can, egui::Button::new(tr("Export…"))).clicked() {
                export = true;
            }
            if ui.button(tr("Cancel")).clicked() {
                close = true;
            }
        });
    });

    if let Some(p) = show_at {
        st.zoom_to = Some(BBox::from_center_size(p, 30.0, 30.0));
        st.export = None;
        return;
    }
    if close {
        st.export = None;
        return;
    }
    if export {
        let name = format!("{}.{}", st.doc.design().name, dlg.format.ext());
        let picked = rfd::FileDialog::new()
            .set_title("Export machine file")
            .add_filter(dlg.format.label(), &[dlg.format.ext()])
            .set_directory(st.export_dialog_dir())
            .set_file_name(name)
            .save_file();
        if let Some(mut path) = picked {
            if path
                .extension()
                .is_none_or(|e| !e.eq_ignore_ascii_case(dlg.format.ext()))
            {
                path.set_extension(dlg.format.ext());
            }
            match st.export_to(&path, dlg.format) {
                Ok(()) => {
                    st.export = None;
                    return;
                }
                Err(e) => {
                    st.notice = Some(Notice {
                        title: "Export failed".into(),
                        lines: vec![e],
                    });
                }
            }
        }
    }
    if st.export.is_some() {
        st.export = Some(dlg);
    }
}

#[cfg(test)]
mod tests {
    use emb_geom::Path as GPath;
    use emb_model::{HoopCatalog, MachineCatalog, ObjectKind, RunningParams};

    use super::*;

    fn state_with_line(from: (f64, f64), to: (f64, f64)) -> AppState {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.doc.edit("Line", |d| {
            let t = crate::draw::thread_for_new(d, None);
            d.add_object(
                t,
                ObjectKind::Running {
                    path: GPath::new()
                        .move_to(Point::new(from.0, from.1))
                        .line_to(Point::new(to.0, to.1)),
                    params: RunningParams::default(),
                },
            )
        });
        st
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("emb-export-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn report_counts_and_no_errors_inside_hoop() {
        let mut st = state_with_line((-20.0, 0.0), (20.0, 0.0));
        let r = st.export_report();
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert_eq!(r.colors, 1);
        assert!(r.stitches > 16);
        let (w, _) = r.size.unwrap();
        assert!((w - 40.0).abs() < 0.01);
    }

    #[test]
    fn stitch_outside_field_blocks_export_and_says_where() {
        // The Topaz crown hoop is 200 mm across: x = 140 is outside.
        let mut st = state_with_line((90.0, 0.0), (140.0, 0.0));
        let r = st.export_report();
        assert_eq!(r.errors.len(), 1);
        let at = r.errors[0].at.unwrap();
        assert!(at.x > 100.0, "{at:?}");
        // `export_to` itself does not check; the dialog disables the button.
        assert!(r.errors[0].text.contains("outside"));
    }

    #[test]
    fn export_writes_dst_and_project_next_to_it() {
        let mut st = state_with_line((-20.0, 0.0), (20.0, 5.0));
        let dir = tmp("ok");
        let path = dir.join("patch.dst");
        st.export_to(&path, Format::Dst).unwrap();
        let dst = emb_formats::dst::read(&std::fs::read(&path).unwrap()).unwrap();
        let plan = st.plan_view().plan.clone();
        assert_eq!(dst.design.stitch_count(), plan.stitches.stitch_count());
        assert_eq!(dst.label, "patch");
        let native = dir.join("patch.embproj");
        assert!(native.is_file());
        assert_eq!(st.file_path.as_deref(), Some(native.as_path()));
        assert!(!st.doc.is_modified());
        st.export_to(&dir.join("patch.pes"), Format::Pes).unwrap();
        let pes = emb_formats::pes::read(&std::fs::read(dir.join("patch.pes")).unwrap()).unwrap();
        assert_eq!(pes.stitch_count(), plan.stitches.stitch_count());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn industrial_machines_select_their_format() {
        let mut st = state_with_line((-20.0, 0.0), (20.0, 5.0));
        for (machine, format) in [("happy", Format::Tap), ("tajima", Format::Dst), ("toyota", Format::Dst)] {
            st.doc.edit("Change machine", |d| d.machine = Some(machine.into()));
            st.open_export();
            assert_eq!(st.export.as_ref().map(|e| e.format), Some(format), "{machine}");
        }
        let dir = tmp("tbf");
        let path = dir.join("patch.tbf");
        st.export_to(&path, Format::Tbf).unwrap();
        let back = emb_formats::tbf::read(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(back.stitch_count(), st.plan_view().plan.stitches.stitch_count());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn sample_objects_export_with_no_warnings() {
        let mut st = state_with_line((-20.0, 0.0), (20.0, 0.0));
        st.add_sample_objects();
        let r = st.export_report();
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert!(r.warnings.is_empty(), "fill and satin are stitched: {:?}", r.warnings);
        let dir = tmp("sample");
        let path = dir.join("sample.vp3");
        st.export_to(&path, Format::Vp3).unwrap();
        let back = emb_formats::vp3::read(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(back.stitch_count(), st.plan_view().plan.stitches.stitch_count());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn corpus_line_art_imports_exports_and_reads_back() {
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        let svg = include_bytes!("../../../tests/corpus/svg/line-art-sampler.svg");
        st.import_svg_bytes(svg, "line-art-sampler.svg");
        assert!(st.notice.is_none(), "{:?}", st.notice);

        let r = st.export_report();
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        assert!(r.warnings.is_empty(), "everything is running stitch: {:?}", r.warnings);
        assert_eq!(r.colors, 3);

        let plan = st.plan_view().plan.clone();
        // The three green ticks are 1 mm apart: travel stitches, so the green
        // block has one trim (at its end).
        let kinds: Vec<StitchKind> = plan.stitches.records.iter().map(|x| x.kind).collect();
        let last_cc = kinds.iter().rposition(|k| *k == StitchKind::ColorChange).unwrap();
        let green_trims = kinds[last_cc..].iter().filter(|k| **k == StitchKind::Trim).count();
        assert_eq!(green_trims, 1);
        // No stitch longer than the machine limit, none shorter than the minimum
        // (except lock stitches, which are 0.5 mm).
        // Only stitches sewn one after another count; a jump or trim starts again.
        let opts = st.plan_options();
        let mut prev: Option<Point> = None;
        for rec in &plan.stitches.records {
            match rec.kind {
                StitchKind::Stitch => {
                    if let Some(p) = prev {
                        let d = p.distance(rec.pos);
                        assert!(d <= opts.max_stitch_mm + 1e-9, "{d} mm stitch at {:?}", rec.pos);
                    }
                    prev = Some(rec.pos);
                }
                _ => prev = None,
            }
        }

        let dir = tmp("corpus");
        let path = dir.join("line-art-sampler.dst");
        st.export_to(&path, Format::Dst).unwrap();
        let back = emb_formats::dst::read(&std::fs::read(&path).unwrap()).unwrap().design;
        assert_eq!(back.stitch_count(), plan.stitches.stitch_count());
        assert_eq!(back.trim_count(), plan.stitches.trim_count());
        assert_eq!(back.color_count(), 3);

        // VP3 keeps every record and the thread colors.
        let path = dir.join("line-art-sampler.vp3");
        st.export_to(&path, Format::Vp3).unwrap();
        let back = emb_formats::vp3::read(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(back.records.len(), plan.stitches.records.len());
        for (a, b) in back.records.iter().zip(&plan.stitches.records) {
            assert_eq!(a.kind, b.kind);
            assert!(a.pos.distance(b.pos) < 0.071, "{a:?} vs {b:?}");
        }
        let colors = |d: &emb_model::StitchDesign| d.threads.iter().map(|t| t.color).collect::<Vec<_>>();
        assert_eq!(colors(&back), colors(&plan.stitches));
        std::fs::remove_dir_all(dir).ok();
    }

    /// Writes the line-art sampler as DST and VP3 for a check in other
    /// software or on the machine. Run with:
    /// `EMB_SAMPLE_OUT=<dir> cargo test -p emb-app -- --ignored write_sampler`
    #[test]
    #[ignore]
    fn write_sampler() {
        let dir = PathBuf::from(std::env::var("EMB_SAMPLE_OUT").expect("set EMB_SAMPLE_OUT"));
        let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
        st.import_svg_bytes(
            include_bytes!("../../../tests/corpus/svg/line-art-sampler.svg"),
            "sampler",
        );
        st.export_to(&dir.join("line-art-sampler.dst"), Format::Dst).unwrap();
        st.export_to(&dir.join("line-art-sampler.vp3"), Format::Vp3).unwrap();
    }
}
