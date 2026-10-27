//! The Order Details window (Design > Orders, P13 Milestone 2): the
//! customer, the order number, the garment, the sizes, the placement, the
//! due date and the notes of the order of the design. The worksheet, the
//! approval sheet, the quote and the design properties CSV show them. The
//! engine work is in `emb_model::order`.

use eframe::egui;
use emb_model::order::{OrderInfo, SizeCount};
use emb_model::quote::MAX_QUANTITY;

use crate::app::AppState;
use crate::i18n::{fill, tr};

/// The usual sizes of garments, for Add Usual Sizes.
pub const USUAL_SIZES: [&str; 6] = ["XS", "S", "M", "L", "XL", "XXL"];

/// True if `s` is a date as `YYYY-MM-DD`.
fn is_date(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").is_ok()
}

/// The places of the product of the design, and of all products (each name
/// once), for the placement menu.
fn placements(st: &AppState) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    let first = st.doc.design().product.as_ref().map(|v| v.kind);
    for kind in first.into_iter().chain(emb_model::ProductKind::ALL) {
        for p in kind.placements() {
            if !names.contains(&p.name) {
                names.push(p.name);
            }
        }
    }
    names
}

/// The Order Details window.
pub fn window(ctx: &egui::Context, st: &mut AppState) {
    if !st.show_order {
        return;
    }
    let mut open = true;
    let mut o = st.order();
    let places = placements(st);
    let on_product = st
        .doc
        .design()
        .product
        .as_ref()
        .and_then(|v| v.placement_of(st.doc.design()))
        .map(|p| p.name);
    let price = st.current_quote().filter(|_| st.price_list().is_set()).map(|(q, _)| q);
    let currency = st.settings.thread_prices.currency.clone();
    let mut end_merge = false;
    let mut quote_window = false;
    crate::dock::docked(
        egui::Window::new(tr("Order Details")).id(egui::Id::new("Order Details")),
        ctx,
        egui::Id::new("Order Details"),
        st.canvas_rect,
    )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| {
            ui.label(tr(
                "The order of this design. The worksheet, the approval sheet and the quote show it, \
                 and it is saved with the design.",
            ));
            egui::Grid::new("order_details")
                .num_columns(2)
                .spacing([12.0, 4.0])
                .show(ui, |ui| {
                    let text = |ui: &mut egui::Ui, label: &str, v: &mut String, hint: &str| {
                        ui.label(label);
                        let r = ui.add(egui::TextEdit::singleline(v).hint_text(hint).desired_width(220.0));
                        ui.end_row();
                        r.lost_focus()
                    };
                    end_merge |= text(ui, tr("Order number"), &mut o.number, tr("For example A-1042"));
                    end_merge |= text(ui, tr("Customer"), &mut o.customer, "");
                    end_merge |= text(ui, tr("Garment"), &mut o.garment, tr("For example Polo shirt"));
                    end_merge |= text(ui, tr("Garment color"), &mut o.garment_color, "");
                    ui.label(tr("Placement"));
                    ui.horizontal(|ui| {
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut o.placement)
                                .hint_text(on_product.unwrap_or(""))
                                .desired_width(160.0),
                        );
                        end_merge |= r.lost_focus();
                        let menu = egui::ComboBox::from_id_salt("order_placement")
                            .selected_text("")
                            .width(24.0)
                            .show_ui(ui, |ui| {
                                for name in &places {
                                    if ui.selectable_label(o.placement == *name, *name).clicked() {
                                        o.placement = name.to_string();
                                        end_merge = true;
                                    }
                                }
                            });
                        crate::a11y::named(menu.response, tr("Choose a placement"));
                    });
                    ui.end_row();
                    ui.label(tr("Due date"));
                    ui.horizontal(|ui| {
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut o.due)
                                .hint_text("YYYY-MM-DD")
                                .desired_width(100.0),
                        );
                        end_merge |= r.lost_focus();
                        if ui.small_button(tr("Today")).clicked() {
                            o.due = crate::output::today();
                            end_merge = true;
                        }
                        if !o.due.trim().is_empty() && !is_date(&o.due) {
                            ui.colored_label(egui::Color32::from_rgb(230, 150, 20), tr("Not a date as YYYY-MM-DD."));
                        }
                    });
                    ui.end_row();
                });
            ui.separator();
            ui.strong(tr("Sizes"));
            if o.sizes.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(tr("Quantity"));
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
                });
                ui.weak(tr("Add sizes to give the pieces of each size."));
            }
            let mut remove = None;
            egui::Grid::new("order_sizes")
                .num_columns(3)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for (i, s) in o.sizes.iter_mut().enumerate() {
                        let r = ui.add(egui::TextEdit::singleline(&mut s.size).desired_width(60.0));
                        end_merge |= r.lost_focus();
                        let r = ui.add(
                            egui::DragValue::new(&mut s.count)
                                .range(0..=MAX_QUANTITY)
                                .clamp_existing_to_range(false)
                                .suffix(tr(" pieces")),
                        );
                        end_merge |= r.lost_focus() || r.drag_stopped();
                        if crate::a11y::named(ui.small_button(egui_phosphor::regular::X), tr("Remove the size"))
                            .clicked()
                        {
                            remove = Some(i);
                        }
                        ui.end_row();
                    }
                });
            if let Some(i) = remove {
                o.sizes.remove(i);
                end_merge = true;
            }
            ui.horizontal(|ui| {
                if ui.button(tr("Add a Size")).clicked() {
                    o.sizes.push(SizeCount {
                        size: String::new(),
                        count: 1,
                    });
                    end_merge = true;
                }
                if o.sizes.is_empty()
                    && ui
                        .button(tr("Add Usual Sizes"))
                        .on_hover_text(tr("XS, S, M, L, XL and XXL with 0 pieces each."))
                        .clicked()
                {
                    o.sizes = USUAL_SIZES
                        .iter()
                        .map(|s| SizeCount {
                            size: s.to_string(),
                            count: 0,
                        })
                        .collect();
                    end_merge = true;
                }
            });
            if !o.sizes.is_empty() {
                ui.label(fill("{} pieces in all.", &[&o.total_quantity()]));
            }
            ui.separator();
            ui.strong(tr("Notes for this order"));
            let r = ui.add(
                egui::TextEdit::multiline(&mut o.notes)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            );
            end_merge |= r.lost_focus();
            ui.separator();
            ui.horizontal(|ui| {
                match &price {
                    Some(q) => ui.label(fill(
                        "Price: {} ({} for each piece).",
                        &[
                            &emb_model::quote::money_text(&currency, q.total),
                            &emb_model::quote::money_text(&currency, q.each),
                        ],
                    )),
                    None => ui.weak(tr("No price: set the prices in the Quote window.")),
                };
                if ui.button(tr("Quote")).clicked() {
                    quote_window = true;
                }
            });
            if ui
                .button(tr("Clear Order"))
                .on_hover_text(tr("Remove all the order details of this design."))
                .clicked()
            {
                o = OrderInfo::default();
                end_merge = true;
            }
        });
    });
    st.set_order(o);
    if end_merge {
        st.doc.end_merge();
    }
    if quote_window {
        st.show_quote = true;
    }
    if !open {
        st.show_order = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Harness;

    #[test]
    fn dates_and_placements() {
        assert!(is_date("2026-11-02"));
        assert!(!is_date("next Friday"));
        let h = Harness::new();
        let p = placements(&h.st);
        assert!(p.contains(&"Left chest"));
        assert_eq!(p.iter().filter(|n| **n == "Left chest").count(), 1);
    }

    #[test]
    fn the_order_goes_on_the_sheets_and_in_the_file_names() {
        let mut h = Harness::new();
        h.st.set_order(OrderInfo {
            number: "A-1042".into(),
            customer: "Riverside FC".into(),
            sizes: vec![
                SizeCount {
                    size: "M".into(),
                    count: 20,
                },
                SizeCount {
                    size: "L".into(),
                    count: 4,
                },
            ],
            ..OrderInfo::default()
        });
        h.st.doc.end_merge();
        let mut s = h.st.settings.clone();
        s.price_lists = vec![emb_model::quote::PriceList {
            per_1000: 1.0,
            ..Default::default()
        }];
        h.st.set_settings(s);
        let d = h.st.current_sheet_data();
        assert_eq!(d.order.as_ref().unwrap().total_quantity(), 24);
        assert!(d.price.as_deref().unwrap().ends_with("for 24 pieces"));
        assert_eq!(d.barcode.as_ref().unwrap().1, "A-1042");
        // Output design with the order number in the file names.
        let dir = std::env::temp_dir().join(format!("emb-order-{}", std::process::id()));
        let plan = h.st.plan_view().plan.clone();
        let profile = crate::output::OutputProfile {
            order_in_names: true,
            csv: true,
            ..Default::default()
        };
        let files =
            crate::output::write_files(h.st.doc.design(), &plan, h.st.catalogs(), &profile, &dir, "rose").unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().any(|n| n == "A-1042 rose.csv"), "{names:?}");
        let csv = std::fs::read_to_string(dir.join("A-1042 rose.csv")).unwrap();
        assert!(csv.contains(",A-1042,Riverside FC,,24,,"), "{csv}");
        std::fs::remove_dir_all(&dir).ok();
        // Undo takes the order back in one step.
        h.st.doc.undo();
        assert!(h.st.doc.design().order.is_none());
    }

    #[test]
    fn the_window_draws() {
        let mut h = Harness::new();
        h.st.show_order = true;
        let st = &mut h.st;
        let mut out = h.ctx.run_ui(Default::default(), |ui| window(&ui.ctx().clone(), st));
        out.textures_delta.clear();
        assert!(h.st.show_order);
        assert!(h.st.doc.design().order.is_none(), "drawing the window changes nothing");
    }
}
