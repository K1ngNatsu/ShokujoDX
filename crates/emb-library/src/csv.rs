//! The design list as CSV (comma between the fields, a line for each
//! design), for a spreadsheet.

use crate::Index;

/// The column titles.
pub const HEADER: [&str; 13] = [
    "Name",
    "File",
    "Folder",
    "Format",
    "Width (mm)",
    "Height (mm)",
    "Stitches",
    "Colors",
    "Trims",
    "Threads",
    "Machine",
    "Notes",
    "Path",
];

/// The CSV text of the entries `items` (in that order), with a title line.
pub fn design_list(index: &Index, items: &[usize]) -> String {
    let mut out = String::new();
    line(&mut out, HEADER.iter().map(|s| s.to_string()));
    for &i in items {
        let e = &index.entries[i];
        let threads = e.threads.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join("; ");
        line(
            &mut out,
            [
                e.name.clone(),
                e.file.clone(),
                e.folder.clone(),
                e.format.to_uppercase(),
                format!("{:.1}", e.width_mm),
                format!("{:.1}", e.height_mm),
                e.stitches.to_string(),
                e.colors.to_string(),
                e.trims.to_string(),
                threads,
                e.machine.clone().unwrap_or_default(),
                e.error
                    .clone()
                    .map_or_else(|| e.notes.clone(), |err| format!("Cannot read: {err}")),
                e.path.to_string_lossy().into_owned(),
            ]
            .into_iter(),
        );
    }
    out
}

fn line(out: &mut String, fields: impl Iterator<Item = String>) {
    let cells: Vec<String> = fields.map(|f| quote(&f)).collect();
    out.push_str(&cells.join(","));
    out.push_str("\r\n");
}

/// A field in quotes when it has a comma, a quote or a line break.
fn quote(f: &str) -> String {
    if f.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", f.replace('"', "\"\""))
    } else {
        f.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_with_commas_and_quotes_are_quoted() {
        let mut idx = Index::default();
        let mut e = crate::search::tests::entry("Rose", "flowers", "vp3", 90.0, 80.0, 4);
        e.notes = "Rush, \"gold\" thread".into();
        idx.entries.push(e);
        let csv = design_list(&idx, &[0]);
        let lines: Vec<&str> = csv.split("\r\n").collect();
        assert_eq!(lines[0], HEADER.join(","));
        assert!(lines[1].starts_with("Rose,Rose.vp3,flowers,VP3,90.0,80.0,7200,4,4,Navy,"));
        assert!(lines[1].contains(",\"Rush, \"\"gold\"\" thread\","), "{}", lines[1]);
        assert_eq!(lines.len(), 3, "the last line ends with a line break");
    }
}
