//! A probe of phase P20 in the app (the audit of P20): the routes of a long
//! Japanese text that a thread makes while the user goes on working, and a
//! large embroidery font file that is damaged after its header.
//! `cargo test --release -p emb-app -- --ignored probe_p20 --nocapture`.

use std::time::{Duration, Instant};

use emb_geom::Point;
use emb_model::{FontRef, HoopCatalog, MachineCatalog, ObjectKind};

use crate::app::AppState;
use crate::lettering::{FontEntry, FontSource, LetteringDialog};

const TEST_FONT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../emb-cli/tests/data/noto-sans-jp-bold-subset.ttf"
);

/// An app with the test font of the lettering bench as a font of the
/// computer, and the routes made in a thread (as in the program).
fn app() -> (AppState, FontRef) {
    let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
    st.fonts.background_routes = true;
    // The names of the file, as the scan of the fonts of the computer
    // gives them: lettering takes the names of its font.
    let bytes = std::fs::read(TEST_FONT).expect("the test font");
    let (family, style) = emb_letter::face_names(&bytes, 0).expect("names");
    let r = FontRef::new(&family, &style);
    st.fonts.entries.push(FontEntry {
        font: r.clone(),
        source: FontSource::File(TEST_FONT.into(), 0),
        tuned: false,
        fill: false,
        score: None,
    });
    (st, r)
}

/// The routed letters of the first lettering object, and all its letters.
fn routed(st: &AppState) -> (usize, usize) {
    let d = st.doc.design();
    d.objects()
        .iter()
        .find_map(|o| match &o.kind {
            ObjectKind::Lettering(l) => Some((l.letters.iter().filter(|x| x.fixed).count(), l.letters.len())),
            _ => None,
        })
        .unwrap_or((0, 0))
}

/// The letters of each lettering object.
fn letters(st: &AppState) -> Vec<Vec<emb_model::Letter>> {
    st.doc
        .design()
        .objects()
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Lettering(l) => Some(l.letters.clone()),
            _ => None,
        })
        .collect()
}

/// Calls `poll_routes` as the frames of the program do, until the routes
/// are made. The longest call, the number of calls that changed the
/// design, and the time to the last route.
fn frames(st: &mut AppState) -> (Duration, usize, Duration) {
    let start = Instant::now();
    let (mut longest, mut changes) = (Duration::ZERO, 0);
    loop {
        let steps = st.doc.undo_len();
        let before = letters(st);
        let t = Instant::now();
        let busy = st.poll_routes();
        longest = longest.max(t.elapsed());
        if letters(st) != before {
            changes += 1;
        }
        assert_eq!(st.doc.undo_len(), steps, "a route made an undo step");
        if !busy {
            // The routes of the last call.
            st.poll_routes();
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(600), "the routes are not made");
        std::thread::sleep(Duration::from_millis(150));
    }
    (longest, changes, start.elapsed())
}

#[test]
#[ignore]
fn probe_p20_routes_in_the_app() {
    let reference: String = emb_letter::japanese::reference().into_iter().collect();
    // 1. A long text: the reference set two times (496 characters), in
    //    lines of 40 characters (an object is at most 3 m).
    let chars: Vec<char> = reference.repeat(2).chars().collect();
    let text = chars
        .chunks(40)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    let (mut st, font) = app();
    let t = Instant::now();
    st.add_lettering(LetteringDialog::new(Point::new(-100.0, 0.0), &text, font.clone(), 15.0));
    println!("typed 496 characters: {:.2?}", t.elapsed());
    let (longest, changes, all) = frames(&mut st);
    let (fixed, letters) = routed(&st);
    println!(
        "long text: {fixed} of {letters} letters routed in {all:.2?}; {changes} new layouts, the longest frame {longest:.2?}"
    );
    let t = Instant::now();
    let stitches = st.plan_view().plan.stitches.stitch_count();
    println!("the plan of the routed text: {stitches} stitches, {:.2?}", t.elapsed());
    // 2. Another edit before the routes come: the routes go into that edit,
    //    and an undo of it must keep the letters.
    let (mut st, font) = app();
    st.add_lettering(LetteringDialog::new(
        Point::new(-30.0, 0.0),
        "刺繍東京龍",
        font.clone(),
        20.0,
    ));
    st.add_lettering(LetteringDialog::new(
        Point::new(-30.0, 40.0),
        "Anna",
        FontRef::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY, ""),
        20.0,
    ));
    frames(&mut st);
    println!("routed after the second edit: {:?}", routed(&st));
    st.undo();
    st.poll_routes();
    println!("after an undo of the second edit: {:?}", routed(&st));
    st.redo();
    st.poll_routes();
    println!("after a redo: {:?}", routed(&st));
    // 3. The design is saved, then the routes come: does it show as changed?
    let (mut st, font) = app();
    st.add_lettering(LetteringDialog::new(Point::new(-30.0, 0.0), "龍", font, 20.0));
    let file = std::env::temp_dir().join(format!("emb-p20-probe-{}.embproj", std::process::id()));
    st.save_to(&file);
    let saved = st.doc.is_modified();
    frames(&mut st);
    println!(
        "saved before the routes: modified {saved} before, {} after the routes",
        st.doc.is_modified()
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
#[ignore]
fn probe_p20_large_font_files() {
    let mut st = AppState::new(HoopCatalog::builtin(), MachineCatalog::builtin(), String::new());
    let dir = std::env::temp_dir().join(format!("emb-p20-fonts-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // A font file of more than 4 MB with a good header and a letter that
    // is not a letter.
    let mut f = emb_letter::bundled::embroidery(emb_letter::bundled::DEFAULT_EMBROIDERY).expect("a bundled font");
    f.family = "Damaged Probe".into();
    f.notes = "x".repeat(5 << 20);
    let text = f.to_json().replacen("\"letters\":{", "\"letters\":{\"a\":5,", 1);
    let path = dir.join("damaged.embfont");
    std::fs::write(&path, text).unwrap();
    let r = st.fonts.add_user_font(&path);
    println!("the list takes the damaged font: {}", r.is_ok());
    if let Ok(r) = r {
        let t = Instant::now();
        let mut got = 0;
        for _ in 0..10 {
            got += usize::from(st.fonts.get(&r).is_some());
        }
        println!("10 reads of the damaged font: {got} fonts, {:.2?}", t.elapsed());
        let mut budget = 1;
        let t = Instant::now();
        let s = st.fonts.sample(&r.family, r.kind, &mut budget);
        println!("its name sample: {:?}, {:.2?}", s.map(|s| s.is_some()), t.elapsed());
    }
    // The fonts of the Japanese font package, if they are made
    // (tools/make-embroidery-fonts.py writes them to target/font-packages).
    let made = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/font-packages/japanese");
    for name in ["noto-sans-jp", "zen-maru-gothic"] {
        let file = made.join(format!("{name}.embfont"));
        if !file.is_file() {
            println!("{name}: not made");
            continue;
        }
        let t = Instant::now();
        let r = st.fonts.add_user_font(&file).expect("the font");
        println!("{name}: header {:.2?}", t.elapsed());
        let mut budget = 1;
        let t = Instant::now();
        let _ = st.fonts.sample(&r.family, r.kind, &mut budget);
        println!("{name}: its name sample {:.2?}", t.elapsed());
        let t = Instant::now();
        let _ = st.fonts.get(&r);
        println!("{name}: read for lettering {:.2?}", t.elapsed());
    }
    let _ = std::fs::remove_dir_all(&dir);
}
