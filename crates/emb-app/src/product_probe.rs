//! A probe of the product of a project (the audit of P16): each number of
//! the product in a project file is set in turn to wrong values, and the
//! project is opened and its product pictures are made with a time limit.
//! `cargo test --release -p emb-app -- --ignored probe_product_values --nocapture`.

use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use emb_geom::Point;
use emb_model::{PhotoPlacement, ProductKind, Rgb, Trim};
use serde_json::Value;

use crate::test_util::Harness;

/// The paths of the numbers in `v` (the first two items of each list).
fn numbers(v: &Value, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match v {
        Value::Number(_) => out.push(path.clone()),
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate().take(2) {
                path.push(i.to_string());
                numbers(x, path, out);
                path.pop();
            }
        }
        Value::Object(o) => {
            for (k, x) in o {
                path.push(k.clone());
                numbers(x, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

fn at<'a>(v: &'a mut Value, path: &[String]) -> &'a mut Value {
    path.iter().fold(v, |v, k| match v {
        Value::Array(a) => &mut a[k.parse::<usize>().unwrap()],
        Value::Object(o) => o.get_mut(k).unwrap(),
        _ => unreachable!(),
    })
}

/// Two projects with every product field: a drawn polo with a trim, a
/// design on its back and both sides; a photo with a mask, a place and a
/// design on it.
fn projects() -> Vec<String> {
    let mut out = Vec::new();
    let mut h = Harness::new();
    let extra = crate::product::tests::small_project("probe-extra", Rgb(20, 90, 200));
    h.st.set_product(ProductKind::Polo);
    h.st.turn_product();
    h.st.add_product_design(&extra).unwrap();
    h.st.turn_product();
    h.st.edit_product_for_test(|v| {
        v.both_sides = true;
        v.trim = Some(Trim {
            color: Rgb(20, 20, 20),
            off: vec!["Sleeve cuffs".into()],
        });
    });
    out.push(emb_model::project::to_string(h.st.doc.design()));
    h.st.set_product_picture(crate::product_photo::tests::shirt_png(), "shirt.png", 400.0)
        .unwrap();
    let ctx = eframe::egui::Context::default();
    h.st.pick_photo_color(&ctx, (20, 20), true);
    h.st.edit_product_for_test(|v| {
        if let Some(p) = v.picture.as_mut() {
            p.placements.push(PhotoPlacement {
                name: "Left chest".into(),
                at: Point::new(250.0, 120.0),
                max: (100.0, 100.0),
            });
        }
    });
    h.st.add_product_design(&extra).unwrap();
    out.push(emb_model::project::to_string(h.st.doc.design()));
    out
}

#[test]
#[ignore]
fn probe_product_values() {
    let mut h = Harness::new();
    let machines = h.st.machines.clone();
    let look = crate::product::picture_look(&h.st.settings.look);
    let settings_look = h.st.settings.look;
    let (mut cases, mut refused, mut slow) = (0, 0, Vec::new());
    for text in projects() {
        let root: Value = serde_json::from_str(&text).unwrap();
        let mut paths = Vec::new();
        numbers(
            &root["design"]["product"],
            &mut vec!["design".into(), "product".into()],
            &mut paths,
        );
        for path in &paths {
            let mut probe = root.clone();
            let whole = at(&mut probe, path).is_u64();
            let values: Vec<Value> = if whole {
                vec![0.into(), 100_000.into(), 1_000_000_000u64.into()]
            } else {
                [0.0, -5.0, 1e-9, 1e9].map(|x| serde_json::json!(x)).to_vec()
            };
            for value in values {
                cases += 1;
                let mut v = root.clone();
                *at(&mut v, path) = value.clone();
                let Ok(design) = emb_model::project::from_str(&v.to_string()) else {
                    refused += 1;
                    continue;
                };
                h.st.doc.edit("probe", |d| *d = design.clone());
                let plan = h.st.plan_view().plan.clone();
                let (m, l, sl) = (machines.clone(), look, settings_look);
                let start = Instant::now();
                let job = std::thread::spawn(move || {
                    let mut px = 0usize;
                    if let Some(view) = &design.product {
                        px += emb_render::product::areas(view).len();
                        if view.picture.is_none() {
                            let (pic, _) = emb_render::product::product_picture(view, 1.0);
                            px += pic.rgba.len() / 4;
                        }
                    }
                    for (_, pic) in crate::product::pictures_of_design(&design, &plan, &m, &sl, 1.0) {
                        px += pic.rgba.len() / 4;
                    }
                    let mut first = true;
                    crate::turntable::make_views(
                        &design,
                        &plan,
                        &m,
                        &l,
                        &mut |crate::turntable::Msg::View(_, p)| {
                            px += p.rgba.len() / 4;
                            std::mem::take(&mut first)
                        },
                        &AtomicBool::new(false),
                    );
                    px
                });
                let limit = Duration::from_secs(20);
                while !job.is_finished() && start.elapsed() < limit {
                    std::thread::sleep(Duration::from_millis(5));
                }
                let name = format!("{} = {value}", path.join("."));
                if !job.is_finished() {
                    slow.push(format!("{name}: over {limit:?}"));
                    continue;
                }
                match job.join() {
                    Ok(px) if px > 60_000_000 => slow.push(format!("{name}: {px} pixels")),
                    Ok(_) if start.elapsed() > Duration::from_secs(5) => {
                        slow.push(format!("{name}: {:?}", start.elapsed()))
                    }
                    Ok(_) => {}
                    Err(_) => slow.push(format!("{name}: panic")),
                }
            }
        }
    }
    println!("cases {cases}, refused {refused}, problems {}", slow.len());
    for s in &slow {
        println!("  {s}");
    }
    assert!(slow.is_empty());
}
