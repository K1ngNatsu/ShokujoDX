//! Prints the glyphs of a shaped text (a check of the text shaping, ADR 0114).
//! Usage: cargo run -p emb-letter --example shape_text -- <font file> <face> <text>
//!        [<height mm> <out.embproj>]
//! In the text, `\uXXXX` is the character with that code (for a terminal
//! that does not take Arabic or Devanagari text). With a height and a file,
//! the text is also written as a design with one lettering object.

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(at) = rest.find("\\u") {
        out.push_str(&rest[..at]);
        let hex: String = rest[at + 2..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .take(6)
            .collect();
        match u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
            Some(c) => out.push(c),
            None => out.push_str("\\u"),
        }
        rest = &rest[at + 2 + hex.len()..];
    }
    out.push_str(rest);
    out
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 4 {
        eprintln!("usage: shape_text <font file> <face> <text> [<height mm> <out.embproj>]");
        return;
    }
    let data = std::fs::read(&a[1]).expect("font file");
    let face: u32 = a[2].parse().expect("face number");
    let text = unescape(&a[3]);
    let chars: Vec<char> = text.chars().collect();
    let Some(glyphs) = emb_letter::shape::shape_line(&data, face, &text, true) else {
        eprintln!("the font cannot be read");
        return;
    };
    println!("{} characters, {} glyphs", chars.len(), glyphs.len());
    for g in glyphs {
        println!(
            "glyph {:5}  cluster {:2} (U+{:04X})  x {:8.0}  y {:6.0}  advance {:6.0}  {}",
            g.letter,
            g.cluster,
            chars.get(g.cluster).map_or(0, |c| *c as u32),
            g.x,
            g.y,
            g.advance,
            if g.rtl { "right to left" } else { "" }
        );
    }
    if let (Some(height), Some(out)) = (a.get(4).and_then(|h| h.parse::<f64>().ok()), a.get(5)) {
        let font = emb_letter::Font::from_bytes(data, face).expect("font");
        let mut d = emb_model::Design::new("text");
        let thread = d.add_thread("Black", emb_model::Rgb(0, 0, 0));
        let at = emb_geom::Point::new(0.0, 0.0);
        let mut l = emb_model::Lettering::new(&text, font.font_ref(), height, at);
        l.justify = emb_model::Justify::Center;
        emb_letter::relayout(&mut l, &font);
        println!("{} letters:", l.letters.len());
        for letter in &l.letters {
            let codes: Vec<String> = letter.text.chars().map(|c| format!("U+{:04X}", c as u32)).collect();
            println!(
                "  character {:2}: {} ({} parts)",
                letter.index,
                codes.join(" "),
                letter.parts.len()
            );
        }
        d.add_object(thread, emb_model::ObjectKind::Lettering(Box::new(l)));
        emb_model::project::save(&d, std::path::Path::new(out)).expect("save");
    }
}
