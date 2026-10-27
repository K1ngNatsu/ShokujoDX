//! The Quote window (Design > Orders, P13 Milestone 1, ADR 0064): the price
//! lists of the shop, the order, the lines of the price, the run time on
//! the machine, and the quote sheet for the customer. The engine work is in
//! `emb_model::quote`. The order is the order of the design
//! (`Design::order`, P13 Milestone 2), so it is saved with the design.

use eframe::egui;
use emb_model::order::OrderInfo;
use emb_model::quote::{
    MAX_PRICE, MAX_QUANTITY, Piece, PriceList, QuantityBreak, Quote, RunTime, duration_text, money_text, quote,
    run_time,
};

use crate::app::AppState;
use crate::i18n::{fill, tr};

impl AppState {
    /// The order of the open design (an empty order if it has none).
    pub fn order(&self) -> OrderInfo {
        self.doc.design().order.clone().unwrap_or_default()
    }

    /// Changes the order of the open design. Typing in one field is one
    /// undo step: call `doc.end_merge` when the field loses the focus. An
    /// empty order removes the order.
    pub fn set_order(&mut self, o: OrderInfo) {
        let o = o.cleaned();
        if o == self.order() {
            return;
        }
        let order = (!o.is_empty()).then_some(o);
        self.doc
            .edit_merged("Order Details", emb_model::MergeKey::new("design-order"), |d| {
                d.order = order
            });
    }

    /// The price list of the order: the list of its name, else the first
    /// list.
    pub fn price_list(&self) -> PriceList {
        self.catalogs()
            .price_list(self.doc.design())
            .cloned()
            .unwrap_or_default()
    }

    /// The quote of the open design for its order, and its run time (none
    /// with no machine). `None` for a design with no stitches.
    pub fn current_quote(&mut self) -> Option<(Quote, Option<RunTime>)> {
        let info = self.design_info();
        if info.stitches == 0 {
            return None;
        }
        let piece = Piece {
            stitches: info.stitches,
            colors: info.threads,
        };
        let q = quote(&self.price_list(), piece, &self.order().quote_order());
        let run = self
            .sew_times_of(self.doc.design().machine.as_deref())
            .zip(info.minutes)
            .map(|(t, m)| run_time(&t, m, q.quantity));
        Some((q, run))
    }

    /// The quote sheet of the open design (one page).
    pub fn quote_sheet(&mut self) -> Option<emb_render::page::Page> {
        let (q, _) = self.current_quote()?;
        let plan = self.plan_view().plan.clone();
        let data = crate::output::sheet_data(self.doc.design(), &plan.stitches, self.catalogs(), Vec::new());
        let currency = self.settings.thread_prices.currency.clone();
        Some(emb_render::sheet::quote_sheet(
            &data,
            &q,
            &currency,
            self.settings.output.paper,
        ))
    }

    /// Asks for a file and writes the quote sheet (PDF) to it.
    fn save_quote_sheet(&mut self) {
        let Some(page) = self.quote_sheet() else {
            return;
        };
        let name = format!("{} quote.pdf", self.doc.design().name);
        let Some(p) = rfd::FileDialog::new()
            .set_title("Quote sheet")
            .add_filter("PDF", &["pdf"])
            .set_directory(self.dialog_dir())
            .set_file_name(name)
            .save_file()
        else {
            return;
        };
        let title = format!("{} quote", self.doc.design().name);
        match std::fs::write(&p, emb_render::pdf::write(&[page], &title)) {
            Ok(()) => self.message = fill("Wrote the quote sheet to {}", &[&p.display()]),
            Err(e) => {
                self.notice = Some(crate::file::Notice {
                    title: "Quote sheet not written".into(),
                    lines: vec![format!("{}: {e}", p.display())],
                })
            }
        }
    }
}

/// A money field of a price list.
fn money(ui: &mut egui::Ui, v: &mut f64, currency: &str) -> egui::Response {
    ui.add(
        egui::DragValue::new(v)
            .range(0.0..=MAX_PRICE)
            .clamp_existing_to_range(false)
            .speed(0.05)
            .fixed_decimals(2)
            .prefix(currency),
    )
}

/// A percent field of a price list.
fn percent(ui: &mut egui::Ui, v: &mut f64) -> egui::Response {
    ui.add(
        egui::DragValue::new(v)
            .range(0.0..=100.0)
            .clamp_existing_to_range(false)
            .speed(0.5)
            .fixed_decimals(1)
            .suffix(" %"),
    )
}

/// The fields of one price list.
fn price_list_ui(ui: &mut egui::Ui, l: &mut PriceList, currency: &str) {
    egui::Grid::new("price_list")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(tr("Name"));
            ui.text_edit_singleline(&mut l.name);
            ui.end_row();
            ui.label(tr("Each 1000 stitches"))
                .on_hover_text(tr("The price of 1000 stitches of one piece."));
            money(ui, &mut l.per_1000, currency);
            ui.end_row();
            ui.label(tr("Lowest price for each piece"))
                .on_hover_text(tr("A small design costs at least this much for each piece."));
            money(ui, &mut l.min_piece, currency);
            ui.end_row();
            ui.label(tr("Colors in the price"))
                .on_hover_text(tr("The stitch price includes this many threads."));
            ui.add(egui::DragValue::new(&mut l.free_colors).range(0..=99));
            ui.end_row();
            ui.label(tr("Each added color"))
                .on_hover_text(tr("For each piece, for each thread over the colors in the price."));
            money(ui, &mut l.per_color, currency);
            ui.end_row();
            ui.label(tr("Setup"))
                .on_hover_text(tr("Once for each order: to set up the machine and the threads."));
            money(ui, &mut l.setup, currency);
            ui.end_row();
            ui.label(tr("Digitizing"))
                .on_hover_text(tr("Once, for a new design: a fixed price."));
            money(ui, &mut l.digitizing, currency);
            ui.end_row();
            ui.label(tr("Digitizing, each 1000 stitches"))
                .on_hover_text(tr("Once, for a new design: added to the fixed price."));
            money(ui, &mut l.digitizing_per_1000, currency);
            ui.end_row();
            ui.label(tr("Rush order"))
                .on_hover_text(tr("Added to the sum of a rush order."));
            percent(ui, &mut l.rush_percent);
            ui.end_row();
            ui.label(tr("Tax"));
            percent(ui, &mut l.tax_percent);
            ui.end_row();
        });
    ui.label(tr("Quantity prices"))
        .on_hover_text(tr("From a quantity, a percent off the embroidery price."));
    let mut remove = None;
    for (i, b) in l.breaks.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(tr("From"));
            ui.add(
                egui::DragValue::new(&mut b.from)
                    .range(1..=MAX_QUANTITY)
                    .clamp_existing_to_range(false)
                    .suffix(tr(" pieces")),
            );
            percent(ui, &mut b.percent_off);
            ui.label(tr("off"));
            if crate::a11y::named(
                ui.small_button(egui_phosphor::regular::X),
                tr("Remove the quantity price"),
            )
            .clicked()
            {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        l.breaks.remove(i);
    }
    if ui.button(tr("Add a Quantity Price")).clicked() {
        let from = l.breaks.iter().map(|b| b.from).max().map_or(12, |f| f * 2);
        l.breaks.push(QuantityBreak {
            from: from.min(MAX_QUANTITY),
            percent_off: 10.0,
        });
    }
}

/// The Quote window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.show_quote {
        return;
    }
    let mut open = true;
    let quoted = st.current_quote();
    let cost = st.design_info().cost.clone();
    let has_machine = st.doc.design().machine.is_some();
    let mut s = st.settings.clone();
    if s.price_lists.is_empty() {
        s.price_lists.push(PriceList::default());
    }
    let mut order = st.order();
    // The list of the order: the list of its name, else the first list.
    let mut list = s
        .price_lists
        .iter()
        .position(|l| !order.price_list.is_empty() && l.name == order.price_list)
        .unwrap_or(0);
    let (list_before, name_before) = (list, s.price_lists[list].name.clone());
    let mut end_merge = false;
    let mut save_sheet = false;
    let mut order_window = false;
    let mut machine_settings = false;
    crate::dock::docked(
        egui::Window::new(tr("Quote")).id(egui::Id::new("Quote")),
        ctx,
        egui::Id::new("Quote"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| {
            let currency = s.thread_prices.currency.clone();
            ui.horizontal(|ui| {
                ui.label(tr("Price list"));
                egui::ComboBox::from_id_salt("price_list_pick")
                    .selected_text(s.price_lists[list].name.clone())
                    .show_ui(ui, |ui| {
                        for (i, l) in s.price_lists.iter().enumerate() {
                            ui.selectable_value(&mut list, i, l.name.clone());
                        }
                    });
                if ui
                    .button(tr("New List"))
                    .on_hover_text(tr("A new price list, for example for caps."))
                    .clicked()
                {
                    let mut l = s.price_lists[list].clone();
                    l.name = fill("{} (copy)", &[&l.name]);
                    s.price_lists.push(l);
                    list = s.price_lists.len() - 1;
                }
                if s.price_lists.len() > 1 && ui.button(tr("Remove List")).clicked() {
                    s.price_lists.remove(list);
                    list = list.saturating_sub(1);
                }
            });
            let list_set = s.price_lists[list].is_set();
            egui::CollapsingHeader::new(tr("Prices"))
                .id_salt("quote_prices")
                .default_open(!list_set)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("Currency"));
                        ui.add(egui::TextEdit::singleline(&mut s.thread_prices.currency).desired_width(40.0));
                    });
                    price_list_ui(ui, &mut s.price_lists[list], &currency);
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.strong(tr("Customer order"));
                if ui
                    .small_button(tr("Order Details"))
                    .on_hover_text(tr(
                        "The customer, the order number, the garment, the sizes and the due date.",
                    ))
                    .clicked()
                {
                    order_window = true;
                }
            });
            egui::Grid::new("quote_order")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    let o = &mut order;
                    ui.label(tr("Quantity"));
                    if o.sizes.is_empty() {
                        let mut n = o.total_quantity();
                        let r = ui.add(
                            egui::DragValue::new(&mut n)
                                .range(1..=MAX_QUANTITY)
                                .clamp_existing_to_range(false)
                                .suffix(tr(" pieces")),
                        );
                        if r.changed() {
                            o.quantity = n;
                        }
                        end_merge |= r.lost_focus() || r.drag_stopped();
                    } else {
                        ui.label(fill("{} pieces (the sizes of Order Details)", &[&o.total_quantity()]));
                    }
                    ui.end_row();
                    ui.label(tr("Garment price")).on_hover_text(tr(
                        "The price of one garment, if you supply them. 0: the customer supplies them.",
                    ));
                    let r = money(ui, &mut o.garment_price, &currency);
                    end_merge |= r.lost_focus() || r.drag_stopped();
                    ui.end_row();
                });
            end_merge |= ui
                .checkbox(&mut order.new_design, tr("New design (add the digitizing price)"))
                .changed();
            end_merge |= ui.checkbox(&mut order.rush, tr("Rush order")).changed();
            ui.separator();
            let Some((q, run)) = &quoted else {
                ui.label(tr("The design has no stitches yet."));
                return;
            };
            if !list_set {
                ui.label(tr("Set the prices of the price list first."));
            }
            egui::Grid::new("quote_lines")
                .num_columns(4)
                .striped(true)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    ui.weak("");
                    ui.weak(tr("Quantity"));
                    ui.weak(tr("Each"));
                    ui.weak(tr("Sum"));
                    ui.end_row();
                    for l in &q.lines {
                        ui.label(tr(l.kind.label()));
                        if l.kind.per_piece() {
                            ui.label(l.quantity.to_string());
                            ui.label(money_text(&currency, l.each));
                        } else {
                            ui.label("");
                            ui.label("");
                        }
                        ui.label(money_text(&currency, l.amount));
                        ui.end_row();
                    }
                    ui.strong(tr("Total"));
                    ui.label("");
                    ui.label("");
                    ui.strong(money_text(&currency, q.total));
                    ui.end_row();
                });
            ui.label(fill(
                "{} for each piece ({} pieces).",
                &[&money_text(&currency, q.each), &q.quantity],
            ));
            if let Some(c) = &cost {
                ui.weak(fill(
                    "Thread cost: about {} for each piece (not on the quote sheet).",
                    &[c],
                ));
            }
            ui.separator();
            ui.strong(tr("Run time"));
            match run {
                Some(r) => run_time_ui(ui, r),
                None if has_machine => {
                    ui.label(tr("The machine of the design is not known."));
                }
                None => {
                    ui.label(tr("Select a machine for the design to see the run time."));
                }
            }
            if has_machine
                && ui
                    .button(tr("Heads and Hooping Times"))
                    .on_hover_text(tr("Open Design > Machine Settings."))
                    .clicked()
            {
                machine_settings = true;
            }
            ui.separator();
            if ui
                .button(tr("Save Quote Sheet"))
                .on_hover_text(tr(
                    "A PDF page for the customer: the design picture, the price and lines to sign.",
                ))
                .clicked()
            {
                save_sheet = true;
            }
        });
    });
    // The order keeps the name of its list (none for the first list). Only
    // a change of the list or of its name changes the order, so drawing the
    // window changes nothing.
    if list != list_before || s.price_lists[list].name != name_before {
        order.price_list = if list == 0 {
            String::new()
        } else {
            s.price_lists[list].name.clone()
        };
    }
    st.set_settings(s);
    st.set_order(order);
    if end_merge {
        st.doc.end_merge();
    }
    if order_window {
        st.show_order = true;
    }
    if machine_settings {
        st.show_sew_times = true;
    }
    if save_sheet {
        st.save_quote_sheet();
    }
    if !open {
        st.show_quote = false;
    }
}

/// The run time of an order.
fn run_time_ui(ui: &mut egui::Ui, r: &RunTime) {
    ui.label(fill(
        "{} pieces on {} heads: {} cycles, about {}.",
        &[&r.quantity, &r.heads, &r.cycles, &duration_text(r.minutes)],
    ));
    ui.label(fill(
        "About {} pieces in an hour.",
        &[&format!("{:.1}", r.pieces_per_hour)],
    ));
    if r.operator_bound {
        ui.colored_label(
            egui::Color32::from_rgb(230, 150, 20),
            tr("The operator sets the speed: hooping the pieces of all heads takes longer than one piece sews."),
        );
    } else if r.cycles > 1 {
        ui.weak(tr("The machine sets the speed."));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    fn with_prices(h: &mut Harness) {
        let mut s = h.st.settings.clone();
        s.price_lists = vec![PriceList {
            per_1000: 1.0,
            setup: 10.0,
            ..PriceList::default()
        }];
        h.st.set_settings(s);
    }

    #[test]
    fn the_quote_uses_the_design_and_the_order() {
        let mut h = Harness::new();
        with_prices(&mut h);
        let (q, run) = h.st.current_quote().unwrap();
        let stitches = h.st.design_info().stitches as f64;
        assert_eq!(q.lines[0].each, emb_model::quote::cents(stitches / 1000.0));
        assert_eq!(q.quantity, 1);
        // The Topaz 20 has one head.
        assert_eq!(run.unwrap().heads, 1);
        h.st.set_order(OrderInfo {
            quantity: 24,
            ..OrderInfo::default()
        });
        let (q24, _) = h.st.current_quote().unwrap();
        assert_eq!(q24.quantity, 24);
        assert!(q24.total > q.total);
        // The order is in the design: undo takes it back.
        h.st.doc.end_merge();
        h.st.doc.undo();
        assert!(h.st.doc.design().order.is_none());
        // A list name that is gone takes the first list.
        h.st.set_order(OrderInfo {
            price_list: "Caps".into(),
            ..OrderInfo::default()
        });
        assert_eq!(h.st.price_list().setup, 10.0);
        // An empty order removes the order.
        h.st.set_order(OrderInfo::default());
        assert!(h.st.doc.design().order.is_none());
    }

    #[test]
    fn heads_change_the_run_time() {
        let mut h = Harness::new();
        h.st.set_order(OrderInfo {
            quantity: 12,
            ..OrderInfo::default()
        });
        let one = h.st.current_quote().unwrap().1.unwrap();
        let id = h.st.doc.design().machine.clone().unwrap();
        let mut t = h.st.sew_times_of(Some(&id)).unwrap();
        t.heads = 6;
        let mut s = h.st.settings.clone();
        s.sew_times.insert(id, t);
        h.st.set_settings(s);
        let six = h.st.current_quote().unwrap().1.unwrap();
        assert_eq!((one.cycles, six.cycles), (12, 2));
        assert!(six.minutes < one.minutes);
    }

    #[test]
    fn quote_sheet_and_window() {
        let mut e = Harness::empty();
        assert!(e.st.current_quote().is_none(), "no stitches");
        let mut h = Harness::new();
        with_prices(&mut h);
        let page = h.st.quote_sheet().unwrap();
        assert!(!emb_render::pdf::write(&[page], "q").is_empty());
        // Drawing the window does not change an order with a list that is gone.
        h.st.set_order(OrderInfo {
            price_list: "Caps".into(),
            ..OrderInfo::default()
        });
        h.st.doc.end_merge();
        h.st.show_quote = true;
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(Default::default(), |ui| window(&ui.ctx().clone(), st));
        out.textures_delta.clear();
        assert!(h.st.show_quote);
        assert_eq!(h.st.order().price_list, "Caps");
    }
}
