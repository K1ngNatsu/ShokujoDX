//! The bench of the 3D view (P18 view plan, Milestone 1, ADR 0122).
//!
//! The reference set ([`REFERENCE`]) is twelve small designs of the corpus.
//! Each one shows one thing that a sew-out shows and that the view must
//! show too. [`run`] makes the pictures of a design at three zooms
//! ([`ZOOMS`]) and measures the time and the memory of each step;
//! [`stress`] measures the same steps for large synthetic stitch lists.
//! `emb-cli view-bench` prints the numbers and writes the pictures
//! ([`write`]); the test `tests/view_bench.rs` keeps a hash of each
//! picture, so that a change of the look is found.
//!
//! The memory of a step is the size of the data that it makes (the stitch
//! records, the segments, the pixels), not the memory of the process.
//!
//! Each design has two pictures at each zoom: the 3D thread look of today
//! (`<design>-<zoom>`) and the surface with its first light
//! (`<design>-<zoom>-surface`, Milestone 3, ADR 0124).

use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use emb_geom::{BBox, Point};
use emb_model::Rgb;
use emb_render::raster::{self, Look, Picture};

use crate::corpus;

/// A design of the reference set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reference {
    /// The name of the corpus design.
    pub name: &'static str,
    /// What the design shows.
    pub shows: &'static str,
    /// The middle of the close picture, in design coordinates (mm).
    pub close: (f64, f64),
}

/// The reference set: the faults of the list "What a sew-out shows and the
/// view does not" of the plan, one design for each (twelve of Milestone 1,
/// and the kinds and the weights of thread of Milestone 2).
pub const REFERENCE: [Reference; 13] = [
    Reference {
        name: "view-satin-angles",
        shows: "Sheen by direction: satin columns at eight angles",
        close: (-45.0, -15.0),
    },
    Reference {
        name: "view-fill-border",
        shows: "Height: a satin border on a fill",
        close: (15.6, 15.6),
    },
    Reference {
        name: "view-underlay",
        shows: "Height: satins and fills with no underlay and with underlay",
        close: (-32.5, 0.0),
    },
    Reference {
        name: "view-lettering",
        shows: "Lettering at 6 mm and at 60 mm",
        close: (-15.0, -52.0),
    },
    Reference {
        name: "view-fill-over-fill",
        shows: "Height: a dense fill over a fill",
        close: (-16.0, 0.0),
    },
    Reference {
        name: "view-puff",
        shows: "Puff: satin over foam",
        close: (0.0, 15.0),
    },
    Reference {
        name: "view-applique",
        shows: "Appliqué: a piece of fabric under its cover",
        close: (-22.0, 0.0),
    },
    Reference {
        name: "view-motif-fill",
        shows: "The pattern of a motif fill",
        close: (0.0, 0.0),
    },
    Reference {
        name: "view-run",
        shows: "Runs: single, triple and backstitch",
        close: (0.0, -15.0),
    },
    Reference {
        name: "view-blend",
        shows: "A blend of two colors",
        close: (0.0, 0.0),
    },
    Reference {
        name: "view-towel",
        shows: "Thin stitches on a towel",
        close: (0.0, 12.0),
    },
    Reference {
        name: "view-dark",
        shows: "Light and dark threads on a dark fabric",
        close: (0.0, 8.0),
    },
    Reference {
        name: "view-thread-kinds",
        shows: "Kinds of thread (polyester, rayon, cotton, metallic, wool, glow) and weights 30 and 60",
        close: (-11.0, -12.0),
    },
];

/// A zoom of the pictures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zoom {
    pub name: &'static str,
    pub px_per_mm: f64,
    /// The size of the close picture (a square at [`Reference::close`]);
    /// `None`: the whole design.
    pub close_mm: Option<f64>,
}

/// The three zooms: the whole design as the canvas shows it to fit a
/// window (a thread is less than a pixel), a usual zoom for work (a
/// thread is about 3 pixels), and a close look at a 16 mm square.
pub const ZOOMS: [Zoom; 3] = [
    Zoom {
        name: "whole",
        px_per_mm: 2.0,
        close_mm: None,
    },
    Zoom {
        name: "normal",
        px_per_mm: 8.0,
        close_mm: None,
    },
    Zoom {
        name: "close",
        px_per_mm: 32.0,
        close_mm: Some(16.0),
    },
];

/// The width of a thread in the pictures of today (as `emb-cli output`).
pub const THREAD_MM: f64 = 0.4;
/// The space round the stitches in a picture of the whole design.
pub const MARGIN_MM: f64 = 2.0;
/// The numbers of stitches of [`stress`].
pub const STRESS: [usize; 3] = [10_000, 100_000, 1_000_000];
/// The fabric of the stress pictures (natural white cotton).
const STRESS_FABRIC: Rgb = Rgb(238, 234, 226);

/// One step: its time and the size of the data that it makes.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub name: String,
    pub ms: f64,
    pub bytes: usize,
}

/// A picture of the 3D look and its hash.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    /// `<design>-<zoom>`.
    pub name: String,
    pub picture: Picture,
    pub hash: u64,
}

/// The measures of one design.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    pub name: String,
    pub stitches: usize,
    pub steps: Vec<Step>,
    pub shots: Vec<Shot>,
}

/// The stitches of the stitch plan of `design` laid on the fabric (ADR
/// 0124): the strands with the foam of puff objects, on the appliqué
/// fabric and the stones of the design.
pub fn lay_design(design: &emb_model::Design) -> emb_render::surface::Laid {
    let opts = crate::plan_options(design, None).expect("the machine of the design");
    lay_plan(design, &crate::plan(design, &opts))
}

/// As [`lay_design`], for a stitch plan of `design`.
fn lay_plan(design: &emb_model::Design, plan: &emb_stitch::StitchPlan) -> emb_render::surface::Laid {
    let lifts = emb_render::strand::lifts(design, plan.spans.iter().map(|s| (s.object, s.records.clone())));
    let strands = emb_render::strand::strands(&plan.stitches, &lifts);
    emb_render::surface::lay(&strands, &emb_render::sheet::picture_areas(design))
}

/// The time of `f`, ms.
fn timed<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let t = Instant::now();
    let v = f();
    (v, t.elapsed().as_secs_f64() * 1000.0)
}

/// The FNV-1a hash of the size and the pixels of a picture. The pictures
/// are the same on each system (ADR 0003), so their hashes are too.
pub fn hash(p: &Picture) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in p
        .width
        .to_le_bytes()
        .iter()
        .chain(&p.height.to_le_bytes())
        .chain(&p.rgba)
    {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The area of the picture of the whole design, as `raster::picture_look`.
fn whole(segments: &[emb_render::Segment], areas: &raster::Areas) -> BBox {
    BBox::from_points(
        segments
            .iter()
            .filter(|s| s.kind == emb_render::SegmentKind::Stitch)
            .flat_map(|s| [s.a, s.b])
            .chain(areas.iter().flat_map(|(rings, _)| rings.iter().flatten().copied())),
    )
    .unwrap_or(BBox::new(Point::ZERO, Point::ZERO))
    .expand(MARGIN_MM + THREAD_MM)
}

/// Makes the pictures of the design of `r` at each zoom: the 3D look and,
/// with `flat`, also the flat look (only timed). The steps: the stitch
/// plan, the segments, and the pictures.
pub fn run(r: &Reference, flat: bool) -> Measure {
    let design = corpus::build(r.name).expect("a design of the corpus");
    let opts = crate::plan_options(&design, None).expect("the machine of the corpus");
    let mut steps = Vec::new();
    let (plan, ms) = timed(|| crate::plan(&design, &opts));
    steps.push(Step {
        name: "plan".into(),
        ms,
        bytes: std::mem::size_of_val(plan.stitches.records.as_slice()),
    });
    let (segments, ms) = timed(|| emb_render::segments(&plan.stitches));
    steps.push(Step {
        name: "segments".into(),
        ms,
        bytes: std::mem::size_of_val(segments.as_slice()),
    });
    let areas = emb_render::sheet::picture_areas(&design);
    let look = crate::picture_look(&design, false);
    let all = whole(&segments, &areas);
    let at = design.output_transform().apply(Point::new(r.close.0, r.close.1));
    let (laid, ms) = timed(|| lay_plan(&design, &plan));
    steps.push(Step {
        name: "lay".into(),
        ms,
        bytes: laid.bytes(),
    });
    let fabric = design.background.zip(look.fabric);
    let mut surface_shots = Vec::new();
    let mut shots = Vec::new();
    for z in ZOOMS {
        let b = z.close_mm.map_or(all, |s| BBox::from_center_size(at, s, s));
        let draw = |look: &Look| {
            timed(|| raster::picture_at_look(&segments, &areas, b, z.px_per_mm, THREAD_MM, design.background, look))
        };
        if flat {
            let (p, ms) = draw(&Look::default());
            steps.push(Step {
                name: format!("flat {}", z.name),
                ms,
                bytes: p.rgba.len(),
            });
        }
        let (p, ms) = draw(&look);
        steps.push(Step {
            name: format!("3D {}", z.name),
            ms,
            bytes: p.rgba.len(),
        });
        shots.push(Shot {
            name: format!("{}-{}", r.name, z.name),
            hash: hash(&p),
            picture: p,
        });
        let (surface, ms) = timed(|| laid.surface(b, z.px_per_mm));
        steps.push(Step {
            name: format!("surface {}", z.name),
            ms,
            bytes: surface.bytes(),
        });
        let (p, ms) = timed(|| emb_render::surface::light(&surface, &Default::default(), fabric));
        steps.push(Step {
            name: format!("light {}", z.name),
            ms,
            bytes: p.rgba.len(),
        });
        surface_shots.push(Shot {
            name: format!("{}-{}-surface", r.name, z.name),
            hash: hash(&p),
            picture: p,
        });
    }
    shots.extend(surface_shots);
    Measure {
        name: r.name.to_string(),
        stitches: plan.stitches.stitch_count(),
        steps,
        shots,
    }
}

/// Measures the segments and the pictures of a synthetic design of about
/// `n` stitches (`emb_render::stress`, about 200 × 180 mm) at the whole and
/// the normal zoom, in the flat and the 3D look, and the surface and its
/// light at 10 pixels for each mm (the budget of Milestone 3: 1 s for
/// 100,000 stitches). It keeps no picture.
pub fn stress(n: usize) -> Measure {
    let s = emb_render::stress::make(n);
    let mut steps = Vec::new();
    let (segments, ms) = timed(|| emb_render::segments(&s));
    steps.push(Step {
        name: "segments".into(),
        ms,
        bytes: std::mem::size_of_val(segments.as_slice()),
    });
    let all = whole(&segments, &[]);
    let three_d = Look::three_d(Some(emb_render::thread3d::Weave::Plain));
    for z in ZOOMS.iter().filter(|z| z.close_mm.is_none()) {
        for (name, look) in [("flat", Look::default()), ("3D", three_d)] {
            let (p, ms) = timed(|| {
                raster::picture_at_look(&segments, &[], all, z.px_per_mm, THREAD_MM, Some(STRESS_FABRIC), &look)
            });
            steps.push(Step {
                name: format!("{name} {}", z.name),
                ms,
                bytes: p.rgba.len(),
            });
        }
    }
    let strands = emb_render::strand::strands(&s, &[]);
    let (laid, ms) = timed(|| emb_render::surface::lay(&strands, &[]));
    steps.push(Step {
        name: "lay".into(),
        ms,
        bytes: laid.bytes(),
    });
    let (surface, ms) = timed(|| laid.surface(all, 10.0));
    steps.push(Step {
        name: "surface 10/mm".into(),
        ms,
        bytes: surface.bytes(),
    });
    let fabric = Some((STRESS_FABRIC, emb_render::thread3d::Weave::Plain));
    let (p, ms) = timed(|| emb_render::surface::light(&surface, &Default::default(), fabric));
    steps.push(Step {
        name: "light 10/mm".into(),
        ms,
        bytes: p.rgba.len(),
    });
    Measure {
        name: format!("stress {n}"),
        stitches: s.stitch_count(),
        steps,
        shots: Vec::new(),
    }
}

/// The measures as two tables: the time of each step (ms) and the size of
/// the data of each step (KB).
pub fn report(measures: &[Measure]) -> String {
    let mut names: Vec<&str> = Vec::new();
    for m in measures {
        for s in &m.steps {
            if !names.contains(&s.name.as_str()) {
                names.push(&s.name);
            }
        }
    }
    let width = |n: &str| n.len().max(9);
    let name_w = measures.iter().map(|m| m.name.len()).max().unwrap_or(6).max(6);
    let mut out = String::new();
    for (title, value) in [
        (
            "Time of each step, ms",
            (|s: &Step| format!("{:.1}", s.ms)) as fn(&Step) -> String,
        ),
        ("Data of each step, KB", |s: &Step| {
            format!("{:.0}", s.bytes as f64 / 1024.0)
        }),
    ] {
        let _ = writeln!(out, "{title}:");
        let _ = write!(out, "{:<name_w$} {:>9}", "design", "stitches");
        for n in &names {
            let _ = write!(out, " {:>w$}", n, w = width(n));
        }
        out.push('\n');
        for m in measures {
            let _ = write!(out, "{:<name_w$} {:>9}", m.name, m.stitches);
            for n in &names {
                let v = m.steps.iter().find(|s| s.name == *n).map_or("-".to_string(), value);
                let _ = write!(out, " {:>w$}", v, w = width(n));
            }
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

/// Reads the hashes of `hashes.txt` (`<name> <hash>` on each line).
fn read_hashes(path: &Path) -> std::collections::BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once(' '))
        .map(|(a, b)| (a.to_string(), b.trim().to_string()))
        .collect()
}

/// Writes the pictures of the reference set to `dir` (`<design>-<zoom>.png`),
/// the report (`report.txt`), the hashes of the pictures (`hashes.txt`), the
/// pictures that changed since the run before (`changed.txt`, as the
/// pictures of the windows, ADR 0110) and a page that shows them
/// (`index.html`). Returns the names of the changed pictures.
pub fn write(dir: &Path, measures: &[Measure], report_text: &str) -> Result<Vec<String>, String> {
    let io = |p: &Path, e: std::io::Error| format!("{}: {e}", p.display());
    std::fs::create_dir_all(dir).map_err(|e| io(dir, e))?;
    let old = read_hashes(&dir.join("hashes.txt"));
    let mut hashes = String::new();
    let mut changed = Vec::new();
    for s in measures.iter().flat_map(|m| &m.shots) {
        let h = format!("{:016x}", s.hash);
        if old.get(&s.name) != Some(&h) {
            changed.push(s.name.clone());
        }
        let _ = writeln!(hashes, "{} {h}", s.name);
        let path = dir.join(format!("{}.png", s.name));
        std::fs::write(&path, s.picture.png()).map_err(|e| io(&path, e))?;
    }
    let files = [
        ("hashes.txt", hashes),
        ("changed.txt", changed.iter().map(|c| format!("{c}\n")).collect()),
        ("report.txt", report_text.to_string()),
        ("index.html", index_page(measures, &changed)),
    ];
    for (name, text) in files {
        let path = dir.join(name);
        std::fs::write(&path, text).map_err(|e| io(&path, e))?;
    }
    Ok(changed)
}

/// A page with the pictures of each design in a row, the zooms side by
/// side; a changed picture has a red frame.
fn index_page(measures: &[Measure], changed: &[String]) -> String {
    let mut out = String::from(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>Reference set of the 3D view</title>\n\
         <style>body{font-family:sans-serif;margin:16px;background:#f4f4f4}td{vertical-align:top;padding:6px}\
         img{max-width:480px;border:3px solid transparent}img.changed{border-color:#d22}</style></head><body>\n\
         <h1>Reference set of the 3D view</h1>\n<p>The pictures of today (ADR 0122). A red frame: changed since the run before.</p>\n<table>\n",
    );
    for m in measures {
        let shows = REFERENCE.iter().find(|r| r.name == m.name).map_or("", |r| r.shows);
        let _ = writeln!(out, "<tr><td><b>{}</b><br>{}</td>", m.name, shows.replace('&', "&amp;"));
        for s in &m.shots {
            let class = if changed.contains(&s.name) {
                " class=\"changed\""
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "<td><a href=\"{0}.png\"><img src=\"{0}.png\"{class}></a><br>{0}</td>",
                s.name
            );
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>\n</body></html>\n");
    out
}

/// Writes the files to sew the reference set to `dir`: for each design the
/// machine file of the machine of the corpus, the project file and the
/// worksheet (for Milestone 1, item 5: the owner sews the set).
pub fn write_sew_files(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let options = crate::OutputOptions {
        formats: Vec::new(),
        worksheet: true,
        approval: false,
        picture: false,
        svg: false,
        paper: emb_render::page::Paper::Letter,
        fabric: None,
        barcode: emb_render::barcode::BarcodeKind::None,
        stone_template: false,
        stone_clearance_mm: emb_model::bling::DEFAULT_CLEARANCE_MM,
        flat_picture: false,
    };
    for r in REFERENCE {
        let design = corpus::build(r.name).expect("a design of the corpus");
        let path = dir.join(format!("{}.{}", r.name, emb_model::project::FILE_EXTENSION));
        emb_model::project::save(&design, &path).map_err(|e| e.to_string())?;
        crate::output(&path, dir, None, &options)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_set_is_in_the_corpus() {
        for r in REFERENCE {
            assert!(corpus::NAMES.contains(&r.name), "{}", r.name);
            let d = corpus::build(r.name).unwrap();
            assert!(d.background.is_some(), "{}: a fabric color", r.name);
            // The close picture shows stitches of the design.
            let e = d.extents().unwrap();
            assert!(e.contains(Point::new(r.close.0, r.close.1)), "{}", r.name);
        }
    }

    #[test]
    fn the_report_has_a_column_for_each_step() {
        let m = Measure {
            name: "a".into(),
            stitches: 12,
            steps: vec![
                Step {
                    name: "plan".into(),
                    ms: 1.25,
                    bytes: 2048,
                },
                Step {
                    name: "3D whole".into(),
                    ms: 3.0,
                    bytes: 4096,
                },
            ],
            shots: Vec::new(),
        };
        let text = report(&[m]);
        assert!(text.contains("Time of each step, ms:"), "{text}");
        assert!(text.contains("3D whole"), "{text}");
        assert!(text.contains("1.2") || text.contains("1.3"), "{text}");
        assert!(text.contains("   2 ") || text.contains("        2"), "{text}");
    }

    #[test]
    fn a_hash_sees_one_pixel() {
        let p = Picture {
            width: 2,
            height: 1,
            rgba: vec![0; 8],
        };
        let mut q = p.clone();
        q.rgba[5] = 1;
        assert_ne!(hash(&p), hash(&q));
        let turned = Picture {
            width: 1,
            height: 2,
            ..p.clone()
        };
        assert_ne!(hash(&p), hash(&turned));
    }
}
