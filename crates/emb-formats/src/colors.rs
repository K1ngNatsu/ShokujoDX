//! Thread color files: COL, EDR and INF (read and write). They go next to
//! a machine file that has no colors (DST, EXP, …), so that programs and
//! machines that read them show the right colors.
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/ColReader.py`, `pyembroidery/ColWriter.py`: a text file,
//!   the count, then `index,red,green,blue` on each line
//! - `pyembroidery/EdrReader.py`, `pyembroidery/EdrWriter.py`: 4 bytes for
//!   each thread, `red, green, blue, 0`
//! - `pyembroidery/InfReader.py`, `pyembroidery/InfWriter.py`: big-endian
//!   records with the color, the needle, the name and the brand
//!
//! Deviations from pyembroidery, on purpose:
//! - The COL reader keeps the first number of a line (the index) as the
//!   code of the thread, as pyembroidery does, and skips lines that are not
//!   four numbers.

use emb_model::{Rgb, Thread, ThreadId};

use crate::common::{Bytes, color_thread};

/// The extensions of the thread color files.
pub const EXTENSIONS: [&str; 3] = ["inf", "edr", "col"];

/// Writes the threads as a COL file.
pub fn write_col(threads: &[Thread]) -> Vec<u8> {
    let mut s = format!("{}\r\n", threads.len());
    for (i, t) in threads.iter().enumerate() {
        let Rgb(r, g, b) = t.color;
        s.push_str(&format!("{i},{r},{g},{b}\r\n"));
    }
    s.into_bytes()
}

/// Reads a COL file.
pub fn read_col(bytes: &[u8]) -> Vec<Thread> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.lines();
    let count: usize = lines.next().and_then(|l| l.trim().parse().ok()).unwrap_or(0);
    lines
        .take(count)
        .filter_map(|l| {
            let v: Vec<&str> = l.split(',').map(str::trim).collect();
            let n = |i: usize| v.get(i).and_then(|x| x.parse::<u8>().ok());
            Some((v.first()?.to_string(), Rgb(n(1)?, n(2)?, n(3)?)))
        })
        .enumerate()
        .map(|(i, (code, c))| {
            let mut t = color_thread(i, c);
            t.code = Some(code);
            t
        })
        .collect()
}

/// Writes the threads as an EDR file.
pub fn write_edr(threads: &[Thread]) -> Vec<u8> {
    threads
        .iter()
        .flat_map(|t| [t.color.0, t.color.1, t.color.2, 0])
        .collect()
}

/// Reads an EDR file.
pub fn read_edr(bytes: &[u8]) -> Vec<Thread> {
    bytes
        .chunks(4)
        .filter(|c| c.len() >= 3)
        .enumerate()
        .map(|(i, c)| color_thread(i, Rgb(c[0], c[1], c[2])))
        .collect()
}

/// Writes the threads as an INF file. A thread with no brand gets
/// "Unknown", as in pyembroidery.
pub fn write_inf(threads: &[Thread]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend(1u32.to_be_bytes());
    b.extend(8u32.to_be_bytes());
    let placeholder = b.len();
    b.extend(0u32.to_be_bytes());
    b.extend((threads.len() as u32).to_be_bytes());
    for (i, t) in threads.iter().enumerate() {
        let name = if t.name.is_empty() { "Unknown" } else { t.name.as_str() };
        let chart = t.brand.as_deref().unwrap_or("Unknown");
        b.extend(((11 + name.len() + chart.len()) as u16).to_be_bytes());
        b.extend((i as u16).to_be_bytes());
        b.extend([t.color.0, t.color.1, t.color.2]);
        b.extend(((i + 1) as u16).to_be_bytes()); // needle
        b.extend(name.as_bytes());
        b.push(0);
        b.extend(chart.as_bytes());
        b.push(0);
    }
    let len = (b.len() - placeholder - 4) as u32;
    b[placeholder..placeholder + 4].copy_from_slice(&len.to_be_bytes());
    b
}

/// Reads an INF file.
pub fn read_inf(bytes: &[u8]) -> Vec<Thread> {
    let mut r = Bytes::new(bytes);
    r.skip(12);
    let count = r.u32be().unwrap_or(0);
    let mut out = Vec::new();
    for i in 0..count {
        let Some(len) = r.u16be() else { break };
        let len = usize::from(len).saturating_sub(2);
        let Some(data) = bytes.get(r.at..r.at + len) else { break };
        r.skip(len);
        if data.len() < 7 {
            break;
        }
        let mut t = Thread {
            id: ThreadId(i),
            name: String::new(),
            color: Rgb(data[2], data[3], data[4]),
            brand: None,
            code: None,
        };
        let mut parts = data[7..].split(|&c| c == 0);
        if let Some(name) = parts.next() {
            t.name = String::from_utf8_lossy(name).into_owned();
        }
        if let Some(chart) = parts.next().filter(|c| !c.is_empty()) {
            t.brand = Some(String::from_utf8_lossy(chart).into_owned());
        }
        if t.name.is_empty() {
            t.name = t.color.to_hex();
        }
        out.push(t);
    }
    out
}

/// Reads a thread color file by its extension.
pub fn read(ext: &str, bytes: &[u8]) -> Option<Vec<Thread>> {
    match ext {
        "col" => Some(read_col(bytes)),
        "edr" => Some(read_edr(bytes)),
        "inf" => Some(read_inf(bytes)),
        _ => None,
    }
}

/// Writes a thread color file by its extension.
pub fn write(ext: &str, threads: &[Thread]) -> Option<Vec<u8>> {
    match ext {
        "col" => Some(write_col(threads)),
        "edr" => Some(write_edr(threads)),
        "inf" => Some(write_inf(threads)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn threads() -> Vec<Thread> {
        vec![
            Thread {
                id: ThreadId(0),
                name: "Red".into(),
                color: Rgb(200, 10, 20),
                brand: Some("Madeira".into()),
                code: Some("1147".into()),
            },
            color_thread(1, Rgb(0, 0, 255)),
        ]
    }

    #[test]
    fn each_file_keeps_the_colors() {
        for ext in EXTENSIONS {
            let b = write(ext, &threads()).unwrap();
            let back = read(ext, &b).unwrap();
            let colors: Vec<Rgb> = back.iter().map(|t| t.color).collect();
            assert_eq!(colors, [Rgb(200, 10, 20), Rgb(0, 0, 255)], "{ext}");
        }
        let inf = read_inf(&write_inf(&threads()));
        assert_eq!(inf[0].name, "Red");
        assert_eq!(inf[0].brand.as_deref(), Some("Madeira"));
        assert_eq!(
            std::str::from_utf8(&write_col(&threads())).unwrap(),
            "2\r\n0,200,10,20\r\n1,0,0,255\r\n"
        );
        assert_eq!(write_edr(&threads()), [200, 10, 20, 0, 0, 0, 255, 0]);
    }
}
