//! Tests of the PostScript reader (ADR 0138): the operators on small
//! programs, the painting rules, the AI script, the limits and damaged
//! files. The exit test with whole files is `crates/emb-cli/tests/eps.rs`.

use emb_model::ObjectKind;

use super::vm::{Limits, Vm};
use super::*;

/// The operand stack after `program`, as text.
fn stack(program: &str) -> Vec<String> {
    let mut vm = Vm::new(program.as_bytes().to_vec(), false, Limits::default());
    let _ = vm.run();
    assert_eq!(vm.errors, 0, "{program}: {:?}", vm.first_error);
    vm.ops
        .iter()
        .map(|&o| match o {
            vm::Obj::Mark => "mark".to_string(),
            o => String::from_utf8_lossy(&ops::text_of(&vm, o)).into_owned(),
        })
        .collect()
}

fn eps(program: &str) -> Vec<u8> {
    format!("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 100\n{program}\n").into_bytes()
}

fn import(program: &str) -> SvgImport {
    import_eps(&eps(program), false).unwrap()
}

fn small(steps: u64, memory: usize) -> Limits {
    Limits { steps, memory }
}

#[test]
fn the_scanner_reads_numbers_strings_and_names() {
    assert_eq!(
        stack("16#FF 8#17 -5 +3 1.5 .5 -2e2 1E-1 /lit 2147483648"),
        [
            "255",
            "15",
            "-5",
            "3",
            "1.5",
            "0.5",
            "-200.0",
            "0.1",
            "lit",
            "2147483648.0"
        ]
    );
    assert_eq!(
        stack("(a\\(b\\)c\\101\\\nd) (x(y)z) <41 42 4> <~9jqo^~>"),
        ["a(b)cAd", "x(y)z", "AB@", "Man "]
    );
    // Comments, and a procedure as one object.
    assert_eq!(stack("1 % 2\n3 {4 5} length"), ["1", "3", "2"]);
    // An immediate name has the value at the time of the scan.
    assert_eq!(stack("/two 2 def { //two } exec /two 3 def"), ["2"]);
}

#[test]
fn arithmetic_and_relations() {
    assert_eq!(
        stack("3 4 mul 10 sub 7 2 idiv -7 2 mod 1 2 div"),
        ["2", "3", "-1", "0.5"]
    );
    assert_eq!(
        stack("2147483647 1 add 9 sqrt 0 1 atan 1 0 atan round"),
        ["2147483648.0", "3.0", "0.0", "90.0"]
    );
    assert_eq!(
        stack("-1.5 round 2.7 truncate 2.2 ceiling -2.2 floor 5 neg"),
        ["-1.0", "2.0", "3.0", "-3.0", "-5"]
    );
    assert_eq!(
        stack("(abc) (abd) lt 1 1.0 eq (a) /a eq 3 2 gt not"),
        ["true", "true", "true", "false"]
    );
    assert_eq!(
        stack("12 10 and 12 10 or 1 4 bitshift 16 -2 bitshift"),
        ["8", "14", "16", "4"]
    );
    assert_eq!(
        stack("90 sin 0 cos 2 3 exp 100 log 0.5 add cvi"),
        ["1.0", "1.0", "8.0", "2"]
    );
}

#[test]
fn the_stack_operators() {
    assert_eq!(stack("1 2 3 3 1 roll"), ["3", "1", "2"]);
    assert_eq!(stack("1 2 3 1 index"), ["1", "2", "3", "2"]);
    assert_eq!(stack("1 2 2 copy exch pop"), ["1", "2", "2"]);
    assert_eq!(stack("mark 1 2 counttomark"), ["mark", "1", "2", "2"]);
    assert_eq!(stack("1 mark 2 3 cleartomark count"), ["1", "1"]);
}

#[test]
fn control_loops_and_stopped() {
    assert_eq!(stack("0 1 1 10 { add } for"), ["55"]);
    assert_eq!(stack("0 0.5 0.5 1.0 { add } for"), ["1.5"]);
    assert_eq!(stack("0 5 { 1 add } repeat"), ["5"]);
    assert_eq!(stack("0 { 1 add dup 3 eq { exit } if } loop"), ["3"]);
    assert_eq!(stack("0 [1 2 3] { add } forall"), ["6"]);
    assert_eq!(stack("0 (AB) { add } forall"), ["131"]);
    assert_eq!(stack("{ nosuchname } stopped"), ["true"]);
    assert_eq!(stack("{ 1 } stopped"), ["1", "false"]);
    assert_eq!(stack("{ 1 stop 2 } stopped"), ["1", "true"]);
    // Recursion, and a procedure that calls itself last (no growth).
    assert_eq!(stack("/f { dup 0 gt { 1 sub f } if } def 50 f"), ["0"]);
    assert_eq!(
        stack("/n 0 def /g { /n n 1 add def n 10000 lt { g } if } def g n"),
        ["10000"]
    );
    assert_eq!(stack("true { 1 } { 2 } ifelse false { 3 } if"), ["1"]);
}

#[test]
fn dictionaries_arrays_and_strings() {
    assert_eq!(
        stack("/d 3 dict def d /x 5 put d /x get d /x known d /y known"),
        ["5", "true", "false"]
    );
    assert_eq!(stack("<< /a 1 (b) 2 >> /b get"), ["2"]);
    assert_eq!(stack("/x 1 def /x where { pop (yes) } { (no) } ifelse"), ["yes"]);
    assert_eq!(stack("4 dict begin /v 7 def currentdict end /v get"), ["7"]);
    assert_eq!(stack("/s 1 def 3 dict begin /s 2 store end s"), ["2"]);
    assert_eq!(stack("[1 2 3] length [1 2 3] 1 get"), ["3", "2"]);
    assert_eq!(stack("[1 2 3] aload pop add add"), ["6"]);
    assert_eq!(stack("1 2 3 3 array astore 2 get"), ["3"]);
    assert_eq!(stack("[ 1 2 ] { 10 mul } forall"), ["10", "20"]);
    assert_eq!(stack("/a 3 array def a 0 [7 8] putinterval a 1 get"), ["8"]);
    assert_eq!(stack("(hello) 1 3 getinterval"), ["ell"]);
    assert_eq!(stack("(abc) (b) search"), ["c", "b", "a", "true"]);
    assert_eq!(stack("(abc) (x) search"), ["abc", "false"]);
    assert_eq!(stack("(abc) (ab) anchorsearch pop pop"), ["c"]);
    assert_eq!(
        stack("123 10 string cvs -1.5 10 string cvs 255 16 10 string cvrs"),
        ["123", "-1.5", "FF"]
    );
    assert_eq!(
        stack("(12.5) cvr (7) cvi 3.9 cvi (name) cvn"),
        ["12.5", "7", "3", "name"]
    );
    assert_eq!(
        stack("/abc cvx xcheck 1 type /x type"),
        ["true", "integertype", "nametype"]
    );
    assert_eq!(stack("(1 2 add) cvx exec"), ["3"]);
    assert_eq!(stack("( 15 (x) rest) token pop exch"), ["15", "(x) rest"]);
    assert_eq!(stack("{ 1 2 add } bind exec"), ["3"]);
    assert_eq!(stack("3 string dup 0 65 put"), ["A\0\0"]);
    // A dictionary with itself in it.
    assert_eq!(stack("/d 2 dict def d /me d put d /me get /me get length"), ["1"]);
}

#[test]
fn the_graphics_state_and_the_transform() {
    assert_eq!(stack("gsave 3 setlinewidth grestore currentlinewidth"), ["1.0"]);
    assert_eq!(
        stack("1 setlinewidth save 5 setlinewidth restore currentlinewidth"),
        ["1.0"]
    );
    assert_eq!(stack("10 20 translate 0 0 transform"), ["10.0", "20.0"]);
    assert_eq!(stack("2 3 scale 1 1 transform"), ["2.0", "3.0"]);
    assert_eq!(stack("90 rotate 1 0 transform exch round exch round"), ["0.0", "1.0"]);
    assert_eq!(stack("2 2 scale 4 4 itransform"), ["2.0", "2.0"]);
    assert_eq!(stack("matrix 5 6 matrix translate matrix concatmatrix 4 get"), ["5.0"]);
    assert_eq!(stack("10 10 moveto 5 0 rlineto currentpoint"), ["15.0", "10.0"]);
    assert_eq!(
        stack("0 0 moveto 10 20 lineto pathbbox"),
        ["0.0", "0.0", "10.0", "20.0"]
    );
    assert_eq!(stack("0 1 0 setrgbcolor currentrgbcolor"), ["0.0", "1.0", "0.0"]);
    assert_eq!(stack("0.25 setgray currentgray 100 mul round"), ["25.0"]);
}

#[test]
fn paths_colors_and_the_page() {
    // The page is the bounding box: y down from its top-left, in mm.
    let r = import("1 0 0 setrgbcolor 10 10 moveto 40 10 lineto 40 30 lineto closepath fill");
    assert_eq!(r.objects.len(), 1);
    assert_eq!(r.objects[0].color, Rgb(255, 0, 0));
    let b = r.objects[0].kind.control_bbox().unwrap();
    assert!(
        (b.min.x - 10.0 * MM_PER_PT).abs() < 1e-9 && (b.max.y - 90.0 * MM_PER_PT).abs() < 1e-9,
        "{b:?}"
    );
    // A circle with `arc` is four curves that touch its box.
    let r = import("50 50 20 0 360 arc fill");
    let b = r.objects[0].kind.control_bbox().unwrap();
    assert!((b.width() - 40.0 * MM_PER_PT).abs() < 1e-6, "{b:?}");
    // A stroke: a line of the width of the stroke in the transform.
    let r = import("2 2 scale 3 setlinewidth 0 0 1 0 setcmykcolor 5 5 moveto 40 5 lineto stroke");
    assert!(matches!(r.objects[0].kind, ObjectKind::Running { .. }));
    assert_eq!(r.objects[0].color, Rgb(255, 255, 0));
    let v = import_eps(&eps("2 2 scale 3 setlinewidth 5 5 moveto 40 5 lineto stroke"), true).unwrap();
    let ObjectKind::Vector { style, .. } = &v.objects[0].kind else {
        panic!()
    };
    assert!((style.stroke_mm - 6.0 * MM_PER_PT).abs() < 1e-9);
    // Indexed, Separation and DeviceN colors.
    let r = import(
        "[/Indexed /DeviceRGB 1 <FF0000 0000FF>] setcolorspace 1 setcolor 0 0 10 10 rectfill \
         [/Separation /Spot /DeviceCMYK { dup 0 0 }] setcolorspace 1 setcolor 0 0 10 10 rectfill \
         [/DeviceN [/A /B] /DeviceGray { add 2 div 1 exch sub }] setcolorspace 1 1 setcolor 0 0 10 10 rectfill",
    );
    let colors: Vec<Rgb> = r.objects.iter().map(|o| o.color).collect();
    assert_eq!(colors, [Rgb(0, 0, 255), Rgb(0, 0, 255), Rgb(0, 0, 0)]);
    // rectfill with an array of rectangles.
    assert_eq!(import("[0 0 10 10 20 20 10 10] rectfill").objects.len(), 2);
}

#[test]
fn what_is_not_painted() {
    // The clip path as a background, `nulldevice`, a path with no paint.
    let r = import(
        "clippath 1 setgray fill gsave nulldevice 0 0 50 50 rectfill grestore \
         0 0 moveto 5 5 lineto newpath 0 0 10 10 rectfill",
    );
    assert_eq!(r.objects.len(), 1);
    // `strokepath fill` is the stroke.
    let r = import("5 setlinewidth 10 10 moveto 90 10 lineto strokepath fill");
    assert!(
        matches!(r.objects[0].kind, ObjectKind::Running { .. }),
        "{:?}",
        r.objects[0].kind
    );
    // Patterns and shadings.
    let r = import("/Pattern setcolorspace << /PatternType 1 >> setcolor 0 0 10 10 rectfill << >> shfill");
    assert_eq!(r.objects[0].color, crate::svg::FALLBACK);
    let w = r.warnings.join(" ");
    assert!(w.contains("pattern") && w.contains("shading"), "{w}");
    // Text with a font of the file, and a picture with binary data.
    let r = import(
        "/F findfont 12 scalefont setfont 0 0 moveto (abc) show \
         (ab) stringwidth pop 0 gt { 20 20 10 10 rectfill } if \
         2 2 8 [2 0 0 2 0 0] currentfile image\nABCD 0 0 10 10 rectfill",
    );
    assert_eq!(r.objects.len(), 2, "{:?}", r.warnings);
    assert!(r.warnings.iter().any(|w| w.contains("1 text")), "{:?}", r.warnings);
    assert!(r.warnings.iter().any(|w| w.contains("1 image")), "{:?}", r.warnings);
}

#[test]
fn the_first_page_with_artwork() {
    let r = import("%%Pages: 2\n0 0 10 10 rectfill showpage 20 20 10 10 rectfill showpage");
    assert_eq!(r.objects.len(), 1);
    assert!(
        r.warnings.iter().any(|w| w.contains("first of 2 pages")),
        "{:?}",
        r.warnings
    );
    // A page with nothing on it does not count.
    assert_eq!(import("showpage 0 0 10 10 rectfill").objects.len(), 1);
}

#[test]
fn errors_and_limits_stop_with_what_was_drawn() {
    let r = import("nosuchoperator 0 0 10 10 rectfill");
    assert_eq!(r.objects.len(), 1);
    assert!(
        r.warnings
            .iter()
            .any(|w| w.contains("1 PostScript error") && w.contains("undefined nosuchoperator")),
        "{:?}",
        r.warnings
    );
    // A loop with no end, and too much memory.
    let r = import_with(&eps("0 0 10 10 rectfill { } loop"), false, small(100_000, 1 << 20)).unwrap();
    assert_eq!(r.objects.len(), 1);
    assert!(r.warnings.iter().any(|w| w.contains("too long")), "{:?}", r.warnings);
    let r = import_with(
        &eps("0 0 10 10 rectfill { 1000 string } loop"),
        false,
        small(1_000_000, 1 << 20),
    )
    .unwrap();
    assert!(r.warnings.iter().any(|w| w.contains("memory")), "{:?}", r.warnings);
    // A procedure that calls itself with no end, and procedures in
    // procedures with no end: errors, not a crash.
    let r = import("/r { r 1 } def r 0 0 10 10 rectfill");
    assert_eq!(r.objects.len(), 1, "{:?}", r.warnings);
    let deep = format!("{}{} 0 0 10 10 rectfill", "{".repeat(5000), "}".repeat(5000));
    assert_eq!(import(&deep).objects.len(), 1);
    // Many errors stop the run.
    let r = import(&"x ".repeat(5000));
    assert!(
        r.warnings.iter().any(|w| w.contains("PostScript error")),
        "{:?}",
        r.warnings
    );
}

#[test]
fn an_eps_file_with_a_binary_header() {
    let ps = eps("0 0 10 10 rectfill");
    let mut data = Vec::new();
    data.extend_from_slice(&DOS_EPS);
    for v in [30u32, ps.len() as u32, 0, 0, 30 + ps.len() as u32, 8] {
        data.extend_from_slice(&v.to_le_bytes());
    }
    data.extend_from_slice(&[0xff, 0xff]);
    data.extend_from_slice(&ps);
    data.extend_from_slice(b"II*\0TIFF");
    assert!(is_postscript(&data));
    assert_eq!(import_eps(&data, false).unwrap().objects.len(), 1);
    // A header with a part outside the file.
    data[8] = 0xff;
    assert!(import_eps(&data, false).is_err());
    assert!(!is_postscript(b"hello"));
    assert!(is_postscript(b"\x04\r\n%!PS"));
}

#[test]
fn the_details_of_the_ai_script() {
    let script = |s: &str| {
        let file = format!(
            "%!PS-Adobe-3.0\n%%BoundingBox: 0 0 100 100\n%AI5_FileFormat 3.0\n%%EndSetup\n{s}\n%%PageTrailer\n"
        );
        import_eps(file.as_bytes(), true).unwrap()
    };
    // RGB fill and stroke, a named RGB color with a tint, a gradient.
    let r = script(
        "1 0 0 Xa 0 0 1 XA 0 0 m 10 0 L 10 10 L B \
         0 1 0 (Green) 0.5 1 Xx 0 0 m 10 0 L 10 10 L f \
         0 Bb 0 0 m 10 0 L 10 10 L f (Black, White) 0 0 0 10 1 0 0 1 0 0 Bg 0 BB",
    );
    let styles: Vec<_> = r
        .objects
        .iter()
        .map(|o| match &o.kind {
            ObjectKind::Vector { style, .. } => (style.fill, style.stroke),
            _ => panic!(),
        })
        .collect();
    assert_eq!(
        styles,
        [
            (Some(Rgb(255, 0, 0)), Some(Rgb(0, 0, 255))),
            (Some(Rgb(128, 255, 128)), None),
            (Some(crate::svg::FALLBACK), None),
        ]
    );
    // The clip path, a text path and the paths of a hidden layer are not
    // painted; an operator that the reader does not know clears its
    // operands.
    let r = script(
        "0 0 m 50 0 L 50 50 L W n 0 To 0 0 m 5 5 L S TO \
         0 1 1 1 Lb 0 0 m 5 0 L 5 5 L f LB 1 2 3 Zz 0 0 m 5 0 L 5 5 L f",
    );
    assert_eq!(r.objects.len(), 1, "{:?}", r.warnings);
}

#[test]
fn damaged_programs_do_not_crash() {
    let base = include_bytes!("../../../emb-cli/tests/data/eps-procset.eps");
    let mut seed: u32 = 7;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        seed >> 8
    };
    for _ in 0..150 {
        let mut d = base.to_vec();
        for _ in 0..1 + next() % 6 {
            let i = next() as usize % d.len();
            match next() % 3 {
                0 => d[i] = (next() % 256) as u8,
                1 => {
                    d.remove(i);
                }
                _ => d.insert(i, b"{}[]()<>/ %"[next() as usize % 11]),
            }
        }
        let _ = import_with(&d, false, small(200_000, 16 << 20));
    }
}
