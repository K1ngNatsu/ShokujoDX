//! The Needles window (Design > Machine and Hoop, ADR 0063): the number of
//! needles of the machine, the needle of each color, and the threads to put
//! on the needles before the start. The engine work is in
//! `emb_model::needles`.

use eframe::egui;
use emb_model::needles::{NEEDLES_RANGE, NeedleSetup, block_threads};
use emb_model::{NeedlePlan, ThreadLook};

use crate::app::AppState;
use crate::i18n::tr;

impl AppState {
    /// The needle plan of the open design on its machine, or `None` for a
    /// machine with one needle.
    pub fn needle_plan(&mut self) -> Option<NeedlePlan> {
        let plan = self.plan_view().plan.clone();
        self.catalogs().needle_plan(self.doc.design(), &plan.stitches)
    }

    /// Gives the thread `thread` the needle `needle` (`None`: Auto). One
    /// undo step.
    pub fn set_needle(&mut self, thread: emb_model::ThreadId, needle: Option<u8>) {
        if self.doc.design().needle_pin(thread) != needle {
            self.doc.edit("Set Needle", |d| d.set_needle_pin(thread, needle));
        }
    }

    /// The threads of the open design stay on the machine after it sews:
    /// the next design uses them where it can.
    pub fn keep_needle_threads(&mut self) {
        let Some(id) = self.doc.design().machine.clone() else {
            return;
        };
        let plan = self.plan_view().plan.clone();
        let Some(needles) = self.needle_plan() else {
            return;
        };
        let mut s = self.settings.clone();
        let setup = s.needles.get(&id).cloned().unwrap_or_default();
        s.needles
            .insert(id, needles.end_setup(&block_threads(&plan.stitches), &setup));
        self.set_settings(s);
        self.message = tr("The machine has the threads of this design on its needles now.").into();
    }
}

/// A small color square.
fn swatch(ui: &mut egui::Ui, c: emb_model::Rgb) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 2.0, crate::canvas::color32(c));
}

/// A thread as text: the name, and the brand and code if known.
fn thread_text(t: &ThreadLook) -> String {
    match (&t.brand, &t.code) {
        (Some(b), Some(k)) => format!("{} ({b} {k})", t.name),
        (None, Some(k)) => format!("{} ({k})", t.name),
        _ => t.name.clone(),
    }
}

/// The Needles window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.show_needles {
        return;
    }
    let mut open = true;
    let machine = st
        .doc
        .design()
        .machine
        .as_deref()
        .and_then(|m| st.machines.get(m))
        .cloned();
    let stitches = st.plan_view().plan.clone();
    let plan = st.needle_plan();
    let design = st.doc.design().clone();
    let mut s = st.settings.clone();
    let mut pin: Option<(emb_model::ThreadId, Option<u8>)> = None;
    let (mut keep, mut auto_all) = (false, false);
    crate::dock::docked(
        egui::Window::new(tr("Needles")).id(egui::Id::new("Needles")),
        ctx,
        egui::Id::new("Needles"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .default_width(420.0)
    .show(ctx, |ui| {
        let Some(m) = machine else {
            ui.label(tr("Select a machine for the design first (Design > Machine and Hoop)."));
            return;
        };
        ui.label(crate::i18n::fill("The needles of {}.", &[&m.name]));
        let setup = s.needles.get(&m.id).cloned().unwrap_or_default();
        let mut count = NeedleSetup::count_for(Some(&setup), &m).unwrap_or(1);
        ui.horizontal(|ui| {
            ui.label(tr("Needles on the machine"))
                .on_hover_text(tr("Set the number of needles of your machine. Models of one brand have 6 to 15."));
            if ui
                .add(
                    egui::DragValue::new(&mut count)
                        .range(NEEDLES_RANGE)
                        .clamp_existing_to_range(false),
                )
                .changed()
            {
                let mut setup = setup.clone();
                setup.count = (Some(count) != m.needles.or(Some(1))).then_some(count);
                if setup.is_empty() {
                    s.needles.remove(&m.id);
                } else {
                    s.needles.insert(m.id.clone(), setup);
                }
            }
            if setup.count.is_some() && ui.button(tr("Reset")).on_hover_text(tr("Use the number of the machine profile.")).clicked() {
                let mut setup = setup.clone();
                setup.count = None;
                if setup.is_empty() {
                    s.needles.remove(&m.id);
                } else {
                    s.needles.insert(m.id.clone(), setup);
                }
            }
        });
        let Some(plan) = &plan else {
            ui.weak(tr(
                "A machine with one needle: the operator changes the thread at each color change. Set the number of needles if your machine has more.",
            ));
            return;
        };
        let blocks = block_threads(&stitches.stitches);
        ui.label(crate::i18n::fill(
            "{} colors on {} needles, {} thread changes.",
            &[&plan.blocks.len(), &plan.count, &plan.changes.len()],
        ));
        if !plan.changes.is_empty() {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                tr("The design has more threads than the machine has needles. At each thread change (*) the machine stops, and the operator puts the next thread on the needle."),
            );
        }
        ui.separator();
        ui.strong(tr("Colors in sew order"));
        egui::ScrollArea::vertical()
            .id_salt("needle_blocks")
            .max_height(220.0)
            .show(ui, |ui| {
                egui::Grid::new("needle_blocks_grid")
                    .num_columns(4)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        for (i, &needle) in plan.blocks.iter().enumerate() {
                            let Some(t) = blocks.get(i).copied().flatten() else {
                                continue;
                            };
                            ui.label((i + 1).to_string());
                            ui.horizontal(|ui| {
                                swatch(ui, t.color);
                                ui.label(thread_text(&ThreadLook::of(t)));
                            });
                            let pinned = design.needle_pin(t.id).filter(|n| *n <= plan.count);
                            let text = match pinned {
                                Some(n) => n.to_string(),
                                None => crate::i18n::fill("Auto ({})", &[&needle]),
                            };
                            egui::ComboBox::from_id_salt(("needle_of", i))
                                .selected_text(text)
                                .width(90.0)
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(pinned.is_none(), tr("Auto")).clicked() {
                                        pin = Some((t.id, None));
                                    }
                                    for n in 1..=plan.count {
                                        if ui.selectable_label(pinned == Some(n), n.to_string()).clicked() {
                                            pin = Some((t.id, Some(n)));
                                        }
                                    }
                                })
                                .response
                                .on_hover_text(tr(
                                    "The needle of this thread. Auto keeps a thread on the needle that has it and puts a new thread on a free needle.",
                                ));
                            if plan.changes.contains(&i) {
                                ui.label("*").on_hover_text(tr(
                                    "Thread change: the machine stops, and the operator puts this thread on the needle.",
                                ));
                            } else {
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
            });
        ui.separator();
        ui.strong(tr("Needles at the start"));
        egui::Grid::new("needles_at_start")
            .num_columns(3)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.weak(tr("Needle"));
                ui.weak(tr("This design"));
                ui.weak(tr("On the machine now"));
                ui.end_row();
                for k in 1..=plan.count {
                    let need = plan.start[usize::from(k) - 1].and_then(|b| blocks.get(b).copied().flatten());
                    let now = setup.thread(k);
                    if need.is_none() && now.is_none() {
                        continue;
                    }
                    ui.label(k.to_string());
                    ui.horizontal(|ui| match need {
                        Some(t) => {
                            swatch(ui, t.color);
                            ui.label(thread_text(&ThreadLook::of(t)));
                            if plan.loads.contains(&k) {
                                ui.strong(tr("put on"));
                            }
                        }
                        None => {
                            ui.weak(tr("not used"));
                        }
                    });
                    ui.horizontal(|ui| match now {
                        Some(t) => {
                            swatch(ui, t.color);
                            ui.label(thread_text(t));
                        }
                        None => {
                            ui.weak("—");
                        }
                    });
                    ui.end_row();
                }
            });
        ui.label(crate::i18n::fill("Needle sequence: {}", &[&plan.sequence_text()]))
            .on_hover_text(tr("Type this sequence on the machine for DST, TAP and EXP files. TBF and U01 files have the needles."));
        ui.horizontal_wrapped(|ui| {
            keep = ui
                .button(tr("Keep these threads on the machine"))
                .on_hover_text(tr(
                    "After you sew the design, its threads are on the needles. The next design uses them where it can.",
                ))
                .clicked();
            if !setup.threads.iter().all(Option::is_none)
                && ui
                    .button(tr("Clear the machine threads"))
                    .on_hover_text(tr("Forget the threads on the needles of the machine."))
                    .clicked()
            {
                let mut setup = setup.clone();
                setup.threads.clear();
                if setup.is_empty() {
                    s.needles.remove(&m.id);
                } else {
                    s.needles.insert(m.id.clone(), setup);
                }
            }
            auto_all = !design.needles.is_empty()
                && ui
                    .button(tr("Auto for all colors"))
                    .on_hover_text(tr("Remove the needles that you set for the threads of the design."))
                    .clicked();
        });
    });
    st.set_settings(s);
    if let Some((t, n)) = pin {
        st.set_needle(t, n);
    }
    if auto_all {
        st.doc.edit("Auto Needles", |d| d.needles.clear());
    }
    if keep {
        st.keep_needle_threads();
    }
    if !open {
        st.show_needles = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    /// A design on a Tajima with three colors: red, blue, red.
    fn tajima() -> Harness {
        let mut h = Harness::empty();
        h.st.doc.edit("Add", |d| {
            d.machine = Some("tajima".into());
            let red = d.add_thread("Red", emb_model::Rgb(200, 0, 0));
            let blue = d.add_thread("Blue", emb_model::Rgb(0, 0, 200));
            for (k, t) in [red, blue, red].into_iter().enumerate() {
                let x = 20.0 * k as f64;
                let path = emb_geom::Path::new()
                    .move_to(emb_geom::Point::new(x, 0.0))
                    .line_to(emb_geom::Point::new(x + 10.0, 0.0))
                    .line_to(emb_geom::Point::new(x + 10.0, 10.0));
                d.add_object(
                    t,
                    emb_model::ObjectKind::Running {
                        path,
                        params: Default::default(),
                    },
                );
            }
        });
        h
    }

    #[test]
    fn a_tajima_design_gets_needles_and_pins_change_them() {
        let mut h = tajima();
        let plan = h.st.needle_plan().expect("a Tajima has needles");
        assert_eq!(plan.count, 15);
        assert_eq!(plan.blocks, [1, 2, 1], "red keeps needle 1");
        let blue = h.st.doc.design().threads[1].id;
        h.st.set_needle(blue, Some(9));
        assert_eq!(h.st.needle_plan().unwrap().blocks, [1, 9, 1]);
        h.st.doc.undo();
        assert_eq!(h.st.needle_plan().unwrap().blocks, [1, 2, 1]);
        // A home machine has no needle plan.
        h.st.doc
            .edit("Machine", |d| d.machine = Some("husqvarna_topaz20".into()));
        assert!(h.st.needle_plan().is_none());
    }

    #[test]
    fn few_needles_give_thread_changes_in_the_info_and_the_sew_time() {
        let mut h = tajima();
        let before = h.st.design_info().minutes.unwrap();
        let blue = h.st.doc.design().threads[1].id;
        // One needle for all: blue and red again are thread changes.
        let mut s = h.st.settings.clone();
        s.needles.insert(
            "tajima".into(),
            NeedleSetup {
                count: Some(2),
                threads: vec![],
            },
        );
        h.st.set_settings(s);
        h.st.set_needle(blue, Some(1));
        let red = h.st.doc.design().threads[0].id;
        h.st.set_needle(red, Some(1));
        let info = h.st.design_info();
        assert_eq!(info.thread_changes, 2);
        assert!(info.minutes.unwrap() > before);
        assert!(info.issues.iter().any(|i| i.text.contains("thread changes")));
    }

    #[test]
    fn kept_threads_stay_on_their_needles_for_the_next_design() {
        let mut h = tajima();
        h.st.keep_needle_threads();
        let setup = h.st.settings.needles.get("tajima").unwrap().clone();
        assert_eq!(setup.thread(1).map(|t| t.name.as_str()), Some("Red"));
        assert_eq!(setup.thread(2).map(|t| t.name.as_str()), Some("Blue"));
        // A new design with blue first: blue stays on needle 2.
        h.st.doc.edit("Swap", |d| {
            let ids: Vec<_> = d.objects().iter().map(|o| o.id).collect();
            d.move_object(ids[1], 0);
        });
        let plan = h.st.needle_plan().unwrap();
        assert_eq!(plan.blocks[0], 2);
        assert!(plan.loads.is_empty(), "the threads are on the machine");
    }

    #[test]
    fn tbf_and_u01_files_keep_their_needles() {
        let dir = std::env::temp_dir().join(format!("emb-needles-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (ext, format) in [("tbf", crate::export::Format::Tbf), ("u01", crate::export::Format::U01)] {
            let mut h = tajima();
            let blue = h.st.doc.design().threads[1].id;
            h.st.set_needle(blue, Some(7));
            let path = dir.join(format!("patch.{ext}"));
            h.st.export_to(&path, format).unwrap();
            assert_eq!(
                emb_formats::read_needles(ext, &std::fs::read(&path).unwrap()),
                [1, 7, 1],
                "{ext}"
            );
            // Opened again, the needles are pins of the threads.
            let d = h.st.read_machine_file(&path, ext).unwrap();
            let pins: Vec<_> = d.threads.iter().map(|t| d.needle_pin(t.id)).collect();
            assert_eq!(pins, [Some(1), Some(7), Some(1)], "{ext}");
        }
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_window_draws() {
        let mut h = tajima();
        h.st.show_needles = true;
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(Default::default(), |ui| window(&ui.ctx().clone(), st));
        out.textures_delta.clear();
        assert!(h.st.show_needles);
    }
}
