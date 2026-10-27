//! The Thread Stock window (Design > Orders, P13 Milestone 3, ADR 0066):
//! the threads of the shop, the threads of the design against the stock,
//! Use Threads in Stock, and the threads to buy for the order. The stock is
//! also a thread chart ("Threads in stock"), so the color matching of the
//! Threads panel, the picture tools and Picture to design can use only the
//! threads in stock. The engine work is in `emb_model::stock`.

use eframe::egui;
use emb_model::stock::{BuyLine, MAX_SPOOL_M, MAX_SPOOLS, STOCK_CHART, StockChange, StockThread, ThreadStock};
use emb_model::{Rgb, ThreadLook};

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The state of the Thread Stock window.
#[derive(Debug, Clone, Default)]
pub struct StockWindow {
    /// The search of the stock list.
    pub query: String,
    /// The chart and the search of "Add from a chart".
    pub chart: usize,
    pub chart_query: String,
    /// The changes of the last Use Threads in Stock.
    pub changes: Vec<StockChange>,
}

/// One different thread of the design and its stock.
#[derive(Debug, Clone, PartialEq)]
pub struct DesignStock {
    pub look: ThreadLook,
    /// The spools in stock, if the stock has the thread.
    pub spools: Option<u32>,
    /// The nearest thread in stock and the color difference, if the
    /// thread is not in stock.
    pub nearest: Option<(String, f64)>,
}

impl AppState {
    /// Puts the chart of the threads in stock last in the chart list (or
    /// takes it away when nothing is in stock). The chart that shows stays
    /// the same.
    pub fn refresh_stock_chart(&mut self) {
        let shown = self.chart().map(|c| c.name.clone());
        self.charts.retain(|c| c.name != STOCK_CHART);
        if let Some(c) = self.settings.thread_stock.as_chart() {
            self.charts.push(c);
        }
        self.chart_index = shown
            .and_then(|n| self.charts.iter().position(|c| c.name == n))
            .unwrap_or(0);
    }

    /// True if the Threads panel and the color matching use only the
    /// threads in stock.
    pub fn stock_only(&self) -> bool {
        self.chart().is_some_and(|c| c.name == STOCK_CHART)
    }

    /// Uses only the threads in stock for the color matching (`on`), or the
    /// chart that showed before.
    pub fn set_stock_only(&mut self, on: bool) {
        if on == self.stock_only() {
            return;
        }
        if on {
            if let Some(i) = self.charts.iter().position(|c| c.name == STOCK_CHART) {
                self.chart_before_stock = self.chart_index;
                self.chart_index = i;
            }
        } else {
            let back = self.chart_before_stock;
            self.chart_index = if self.charts.get(back).is_some_and(|c| c.name != STOCK_CHART) {
                back
            } else {
                0
            };
        }
    }

    /// The different threads of the design and their stock.
    pub fn design_stock(&mut self) -> Vec<DesignStock> {
        let plan = self.plan_view().plan.clone();
        let stock = &self.settings.thread_stock;
        emb_model::stock::thread_lengths(&plan.stitches, 0.0)
            .into_iter()
            .map(|(look, _)| {
                let spools = stock.find(&look).map(|i| stock.threads[i].spools).filter(|n| *n > 0);
                let nearest = spools
                    .is_none()
                    .then(|| stock.nearest(look.color))
                    .flatten()
                    .map(|(k, d)| (stock_text(&stock.threads[k]), d));
                DesignStock { look, spools, nearest }
            })
            .collect()
    }

    /// The threads to buy for the order of the design, or `None` with no
    /// stock.
    pub fn threads_to_buy(&mut self) -> Option<Vec<BuyLine>> {
        if self.settings.thread_stock.is_empty() {
            return None;
        }
        let plan = self.plan_view().plan.clone();
        let thickness = self.fabrics.of(self.doc.design()).thickness_mm;
        let quantity = self.order().total_quantity();
        Some(emb_model::stock::threads_to_buy(
            &self.settings.thread_stock,
            &plan.stitches,
            thickness,
            quantity,
            self.settings.thread_prices.spool_m,
        ))
    }

    /// Gives each used thread that is not in stock the nearest thread in
    /// stock. One undo step.
    pub fn use_stock_threads(&mut self) -> Vec<StockChange> {
        let stock = self.settings.thread_stock.clone();
        let mut probe = self.doc.design().clone();
        if probe.use_stock_threads(&stock).is_empty() {
            self.message = tr("All the used threads are in stock.").into();
            return Vec::new();
        }
        let changes = self.doc.edit("Use Threads in Stock", |d| d.use_stock_threads(&stock));
        self.message = fill("{} threads changed to threads in stock.", &[&changes.len()]);
        changes
    }

    /// Adds chart thread `k` of chart `chart` to the stock (one spool).
    pub fn add_to_stock(&mut self, chart: usize, k: usize) {
        let Some(c) = self.charts.get(chart) else { return };
        if k >= c.threads.len() {
            return;
        }
        let t = StockThread::of_chart(c, k);
        let mut s = self.settings.clone();
        s.thread_stock.add(t);
        self.set_settings(s);
    }

    fn import_stock_dialog(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .set_title("Import thread stock")
            .add_filter("CSV", &["csv"])
            .set_directory(self.dialog_dir())
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&p)
            .map_err(|e| e.to_string())
            .and_then(|t| ThreadStock::from_csv(&t))
        {
            Ok(add) => {
                let n = add.threads.len();
                let mut s = self.settings.clone();
                for t in add.threads {
                    s.thread_stock.add(t);
                }
                self.set_settings(s);
                self.message = fill("{} threads imported to the stock.", &[&n]);
            }
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "Thread stock not imported".into(),
                    lines: vec![format!("{}: {e}", p.display())],
                })
            }
        }
    }

    fn save_csv_dialog(&mut self, title: &str, name: &str, text: String) {
        let Some(p) = rfd::FileDialog::new()
            .set_title(title)
            .add_filter("CSV", &["csv"])
            .set_directory(self.dialog_dir())
            .set_file_name(name)
            .save_file()
        else {
            return;
        };
        match std::fs::write(&p, text) {
            Ok(()) => self.message = fill("Wrote {}", &[&p.display()]),
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "File not written".into(),
                    lines: vec![format!("{}: {e}", p.display())],
                })
            }
        }
    }
}

/// A stock thread as text: name, brand and code.
fn stock_text(t: &StockThread) -> String {
    let id = [t.brand.trim(), t.code.trim()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if id.is_empty() {
        t.name.clone()
    } else {
        format!("{} ({id})", t.name)
    }
}

/// A thread look as text: name, brand and code.
pub fn look_text(t: &ThreadLook) -> String {
    match (&t.brand, &t.code) {
        (Some(b), Some(c)) => format!("{} ({b} {c})", t.name),
        (None, Some(c)) => format!("{} ({c})", t.name),
        _ => t.name.clone(),
    }
}

fn swatch(ui: &mut egui::Ui, c: Rgb) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 2.0, crate::canvas::color32(c));
}

/// The Thread Stock window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.show_stock {
        return;
    }
    let mut open = true;
    let mut s = st.settings.clone();
    let mut w = std::mem::take(&mut st.stock_window);
    let design = st.design_stock();
    let to_buy = st.threads_to_buy();
    let quantity = st.order().total_quantity();
    let charts: Vec<(usize, String)> = st
        .charts
        .iter()
        .enumerate()
        .filter(|(_, c)| c.name != STOCK_CHART)
        .map(|(i, c)| (i, c.name.clone()))
        .collect();
    if !charts.iter().any(|(i, _)| *i == w.chart) {
        w.chart = charts.first().map_or(0, |(i, _)| *i);
    }
    let hits: Vec<(usize, Rgb, String)> = st
        .charts
        .get(w.chart)
        .filter(|_| !w.chart_query.trim().is_empty())
        .map(|c| {
            c.search(&w.chart_query)
                .into_iter()
                .take(40)
                .map(|k| {
                    let t = &c.threads[k];
                    let text = match &t.code {
                        Some(code) => format!("{code}  {}", t.name),
                        None => t.name.clone(),
                    };
                    (k, t.color, text)
                })
                .collect()
        })
        .unwrap_or_default();
    let mut stock_only = st.stock_only();
    let mut add = None;
    let mut use_stock = false;
    let mut import = false;
    let mut export = false;
    let mut save_buy = false;
    crate::dock::docked(
        egui::Window::new(tr("Thread Stock")).id(egui::Id::new("Thread Stock")),
        ctx,
        egui::Id::new("Thread Stock"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        egui::ScrollArea::vertical().max_height(600.0).show(ui, |ui| {
            ui.label(tr("The threads that you have. Each spool that you began counts as one."));
            ui.add_enabled(
                s.thread_stock.as_chart().is_some(),
                egui::Checkbox::new(&mut stock_only, tr("Only threads in stock")),
            )
            .on_hover_text(tr(
                "The Threads panel, Match Design to Chart, the picture tools and Picture to Design use only the threads in stock.",
            ));

            // The threads of the design.
            ui.separator();
            ui.strong(tr("Threads of this design"));
            if design.is_empty() {
                ui.weak(tr("The design has no stitches yet."));
            }
            egui::Grid::new("stock_design").num_columns(3).spacing([8.0, 4.0]).show(ui, |ui| {
                for t in &design {
                    swatch(ui, t.look.color);
                    ui.label(look_text(&t.look));
                    match (&t.spools, &t.nearest) {
                        (Some(n), _) => ui.label(fill("in stock: {} spools", &[n])),
                        (None, Some((name, d))) => ui.colored_label(
                            egui::Color32::from_rgb(230, 150, 20),
                            fill("not in stock; nearest: {} (ΔE {})", &[name, &format!("{d:.1}")]),
                        ),
                        (None, None) => ui.colored_label(egui::Color32::from_rgb(230, 150, 20), tr("not in stock")),
                    };
                    ui.end_row();
                }
            });
            if ui
                .add_enabled(
                    design.iter().any(|t| t.nearest.is_some()),
                    egui::Button::new(tr("Use Threads in Stock")),
                )
                .on_hover_text(tr(
                    "Each used thread that is not in stock gets the nearest thread in stock. ΔE is the color difference: under about 2.3 it does not show.",
                ))
                .clicked()
            {
                use_stock = true;
            }
            if !w.changes.is_empty() {
                ui.weak(tr("Last changes:"));
                for c in &w.changes {
                    ui.weak(fill(
                        "{} to {} (ΔE {})",
                        &[&look_text(&c.from), &look_text(&c.to), &format!("{:.1}", c.delta_e)],
                    ));
                }
            }

            // The threads to buy.
            if let Some(buy) = &to_buy {
                ui.separator();
                ui.strong(fill("Threads to buy for {} pieces", &[&quantity]));
                if buy.is_empty() {
                    ui.label(tr("The stock has all the thread for the order."));
                } else {
                    egui::Grid::new("stock_buy").num_columns(4).spacing([8.0, 4.0]).show(ui, |ui| {
                        for b in buy {
                            swatch(ui, b.look.color);
                            ui.label(b.thread_text());
                            ui.label(fill("{} spools", &[&b.spools]));
                            ui.weak(fill(
                                "{}: needs {} m, has {} m",
                                &[&tr(b.reason.label()), &format!("{:.0}", b.need_m), &format!("{:.0}", b.have_m)],
                            ));
                            ui.end_row();
                        }
                    });
                    if ui.button(tr("Save the List of Threads to Buy")).clicked() {
                        save_buy = true;
                    }
                }
                ui.weak(tr(
                    "The quantity is the quantity of Order Details. The bobbin thread is not in the list.",
                ));
            }

            // Add threads.
            ui.separator();
            ui.strong(tr("Add from a chart"));
            ui.horizontal(|ui| {
                let shown = charts
                    .iter()
                    .find(|(i, _)| *i == w.chart)
                    .map(|(_, n)| n.clone())
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("stock_chart")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for (i, n) in &charts {
                            ui.selectable_value(&mut w.chart, *i, n);
                        }
                    });
                ui.add(
                    egui::TextEdit::singleline(&mut w.chart_query)
                        .hint_text(tr("name or code"))
                        .desired_width(140.0),
                );
            });
            for (k, color, text) in &hits {
                ui.horizontal(|ui| {
                    swatch(ui, *color);
                    ui.label(text);
                    if ui.small_button(tr("Add")).clicked() {
                        add = Some(*k);
                    }
                });
            }

            // The stock.
            ui.separator();
            ui.horizontal(|ui| {
                ui.strong(fill("Stock ({} threads)", &[&s.thread_stock.threads.len()]));
                ui.add(
                    egui::TextEdit::singleline(&mut w.query)
                        .hint_text(tr("Search"))
                        .desired_width(120.0),
                );
            });
            let words: Vec<String> = w.query.split_whitespace().map(str::to_lowercase).collect();
            let mut remove = None;
            let default_m = s.thread_prices.spool_m;
            egui::Grid::new("stock_list")
                .num_columns(6)
                .striped(true)
                .spacing([8.0, 3.0])
                .show(ui, |ui| {
                    ui.weak("");
                    ui.weak(tr("Thread"));
                    ui.weak(tr("Spools"));
                    ui.weak(tr("Spool length")).on_hover_text(fill(
                        "0: the length of the thread prices in Options ({} m).",
                        &[&format!("{default_m:.0}")],
                    ));
                    ui.weak(tr("Lowest"))
                        .on_hover_text(tr("The lowest number of spools to keep."));
                    ui.weak("");
                    ui.end_row();
                    for (i, t) in s.thread_stock.threads.iter_mut().enumerate() {
                        let text = stock_text(t);
                        let lower = text.to_lowercase();
                        if !words.iter().all(|w| lower.contains(w.as_str())) {
                            continue;
                        }
                        swatch(ui, t.color);
                        ui.label(text);
                        ui.add(egui::DragValue::new(&mut t.spools).range(0..=MAX_SPOOLS));
                        let mut m = t.spool_m.unwrap_or(0.0);
                        if ui
                            .add(egui::DragValue::new(&mut m).range(0.0..=MAX_SPOOL_M).speed(10.0).suffix(" m"))
                            .changed()
                        {
                            t.spool_m = (m > 0.0).then_some(m);
                        }
                        ui.add(egui::DragValue::new(&mut t.min_spools).range(0..=MAX_SPOOLS));
                        if crate::a11y::named(ui.small_button(egui_phosphor::regular::X), tr("Remove the thread from the stock")).clicked() {
                            remove = Some(i);
                        }
                        ui.end_row();
                    }
                });
            if let Some(i) = remove {
                s.thread_stock.threads.remove(i);
            }
            ui.horizontal(|ui| {
                if ui.button(tr("Import CSV")).clicked() {
                    import = true;
                }
                if ui
                    .add_enabled(!s.thread_stock.is_empty(), egui::Button::new(tr("Export CSV")))
                    .on_hover_text(tr("brand, code, name, color, spools, spool length, lowest number"))
                    .clicked()
                {
                    export = true;
                }
            });
        });
    });
    s.thread_stock = s.thread_stock.cleaned();
    let stock_changed = s.thread_stock != st.settings.thread_stock;
    st.set_settings(s);
    if stock_changed {
        st.refresh_stock_chart();
    }
    st.set_stock_only(stock_only);
    if let Some(k) = add {
        st.add_to_stock(w.chart, k);
        st.refresh_stock_chart();
    }
    if use_stock {
        w.changes = st.use_stock_threads();
    }
    st.stock_window = w;
    if import {
        st.import_stock_dialog();
        st.refresh_stock_chart();
    }
    if export {
        let csv = st.settings.thread_stock.to_csv();
        st.save_csv_dialog("Export thread stock", "thread stock.csv", csv);
    }
    if save_buy && let Some(buy) = st.threads_to_buy() {
        let name = format!("{} threads to buy.csv", st.doc.design().name);
        st.save_csv_dialog("Threads to buy", &name, emb_model::stock::buy_csv(&buy));
    }
    if !open {
        st.show_stock = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    /// The sample design and a stock with its first thread.
    fn with_stock() -> Harness {
        let mut h = Harness::new();
        let first = h.st.design_stock()[0].look.clone();
        let mut s = h.st.settings.clone();
        s.thread_stock.add(StockThread {
            brand: first.brand.clone().unwrap_or_default(),
            code: first.code.clone().unwrap_or_default(),
            name: first.name.clone(),
            color: first.color,
            spools: 1,
            spool_m: Some(1.0),
            min_spools: 0,
        });
        h.st.set_settings(s);
        h.st.refresh_stock_chart();
        h
    }

    #[test]
    fn the_stock_is_a_chart_and_only_in_stock_switches_to_it() {
        let mut h = with_stock();
        assert_eq!(h.st.charts.last().unwrap().name, STOCK_CHART);
        let before = h.st.chart().unwrap().name.clone();
        h.st.set_stock_only(true);
        assert!(h.st.stock_only());
        assert_eq!(h.st.chart().unwrap().threads.len(), 1);
        h.st.set_stock_only(false);
        assert_eq!(h.st.chart().unwrap().name, before);
        // An empty stock has no chart.
        let mut s = h.st.settings.clone();
        s.thread_stock = ThreadStock::default();
        h.st.set_settings(s);
        h.st.refresh_stock_chart();
        assert!(!h.st.charts.iter().any(|c| c.name == STOCK_CHART));
    }

    #[test]
    fn design_threads_and_threads_to_buy() {
        let mut h = with_stock();
        let d = h.st.design_stock();
        assert!(d[0].spools.is_some(), "the first thread is in stock");
        if d.len() > 1 {
            assert!(d[1].nearest.is_some(), "the others get the nearest thread in stock");
        }
        // One spool of 1 m is too little for 1000 pieces.
        h.st.set_order(emb_model::order::OrderInfo {
            quantity: 1000,
            ..Default::default()
        });
        let buy = h.st.threads_to_buy().unwrap();
        assert!(buy.iter().any(|b| b.look.name == d[0].look.name && b.spools > 0));
        assert_eq!(buy.len(), d.len());
        // Use Threads in Stock: all threads are the stock thread, in one undo step.
        let changes = h.st.use_stock_threads();
        assert_eq!(changes.len(), d.len() - 1);
        assert!(h.st.design_stock().iter().all(|t| t.spools.is_some()));
        h.st.doc.undo();
        assert_eq!(h.st.design_stock(), d);
        // With no stock, no list.
        let mut e = Harness::new();
        assert!(e.st.threads_to_buy().is_none());
    }

    #[test]
    fn matching_with_only_threads_in_stock_gives_threads_in_stock() {
        let mut h = with_stock();
        h.st.set_stock_only(true);
        h.st.match_to_chart();
        let stock = h.st.settings.thread_stock.clone();
        let d = h.st.doc.design();
        for id in d.used_threads() {
            let t = d.thread(id).unwrap();
            assert!(stock.has(&ThreadLook::of(t)), "{} is not in stock", t.name);
        }
    }

    #[test]
    fn the_window_draws() {
        let mut h = with_stock();
        h.st.show_stock = true;
        h.st.stock_window.chart_query = "red".into();
        let before = h.st.settings.thread_stock.clone();
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(Default::default(), |ui| window(&ui.ctx().clone(), st));
        out.textures_delta.clear();
        assert!(h.st.show_stock);
        assert_eq!(h.st.settings.thread_stock, before, "drawing changes nothing");
    }
}
