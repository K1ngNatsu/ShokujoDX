//! Outline Every Fill
//! Adds a running outline round each fill of the selection, or of all the
//! design when nothing is selected. The outline sews after its fill.
//! input distance: 0.0 "Distance from the edge of the fill (mm)"

let ids = selection();
if ids.len() == 0 {
    ids = objects();
}

let added = 0;
for id in ids {
    if kind(id) == "fill" {
        let made = add_outline([id], input.distance);
        added += made.len();
    }
}
print(`${added} outlines added.`);
