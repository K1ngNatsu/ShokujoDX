//! Same Density for All Satins
//! Gives each satin column of the selection, or of all the design when
//! nothing is selected, the same spacing between its stitches.
//! input spacing: 0.4 "Spacing (mm)"

if input.spacing < 0.2 || input.spacing > 2.0 {
    throw "Use a spacing from 0.2 to 2.0 mm.";
}

let ids = selection();
if ids.len() == 0 {
    ids = objects();
}

let changed = 0;
for id in ids {
    if kind(id) == "satin" {
        let s = settings(id);
        s.spacing_mm = input.spacing;
        set_settings(id, s);
        changed += 1;
    }
}
print(`${changed} satin columns now have a spacing of ${input.spacing} mm.`);
