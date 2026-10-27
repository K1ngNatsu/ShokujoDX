//! Check Before Output
//! Checks the design against the limits of your shop before you make the
//! machine file. It changes nothing.
//! input max_colors: 12 "Most colors"
//! input max_trims: 30 "Most trims"
//! input max_width: 300.0 "Widest design (mm)"
//! input smallest: 1.0 "Smallest object (mm)"

let i = info();
let max_colors = input.max_colors.to_int();
let max_trims = input.max_trims.to_int();
let problems = 0;

if i.stitches == 0 {
    print("The design has no stitches.");
    problems += 1;
}
if i.colors > max_colors {
    print(`${i.colors} colors: more than ${max_colors}.`);
    problems += 1;
}
if i.trims > max_trims {
    print(`${i.trims} trims: more than ${max_trims}.`);
    problems += 1;
}
if i.width_mm > input.max_width || i.height_mm > input.max_width {
    print(`The design is ${i.width_mm.round()} × ${i.height_mm.round()} mm: larger than ${input.max_width} mm.`);
    problems += 1;
}
for id in objects() {
    let k = kind(id);
    if k == "vector" || k == "bling" {
        continue;
    }
    let s = size(id);
    if s.width < input.smallest && s.height < input.smallest {
        print(`Object ${sew_index(id) + 1} (${name(id)}) is smaller than ${input.smallest} mm.`);
        problems += 1;
    }
}
if i.warnings > 0 {
    print(`${i.warnings} warnings of the stitch engine: see Design info.`);
    problems += 1;
}

if problems == 0 {
    print("No problems found.");
} else {
    print(`${problems} problems found.`);
}
