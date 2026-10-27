//! Damaged and wrong files give an error, not a crash: each reader gets
//! corpus files that are cut short or have changed bytes, and bytes of
//! other formats.

use std::panic::{AssertUnwindSafe, catch_unwind};

use emb_cli::{Input, corpus, stitches};

/// A small random generator, so that the test is the same each time.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

/// Cut and changed copies of `bytes`.
fn damaged(bytes: &[u8], rng: &mut Lcg) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    let n = bytes.len();
    for cut in [
        0,
        1,
        2,
        3,
        4,
        7,
        8,
        16,
        31,
        64,
        100,
        128,
        255,
        256,
        512,
        n / 3,
        n / 2,
        n.saturating_sub(1),
    ] {
        if cut < n {
            v.push(bytes[..cut].to_vec());
        }
    }
    for _ in 0..24 {
        let mut b = bytes.to_vec();
        for _ in 0..1 + rng.next() % 8 {
            if !b.is_empty() {
                let i = (rng.next() as usize) % b.len().min(600);
                b[i] = rng.next() as u8;
            }
        }
        v.push(b);
    }
    // Large numbers in the header (counts and offsets).
    for fill in [0x00u8, 0x7f, 0xff] {
        let mut b = bytes.to_vec();
        for x in b.iter_mut().take(64) {
            *x = fill;
        }
        v.push(b);
    }
    v
}

fn check<T>(what: &str, f: impl FnOnce() -> T) -> Option<String> {
    catch_unwind(AssertUnwindSafe(f)).err().map(|e| {
        let msg = e
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default();
        format!("{what}: {msg}")
    })
}

#[test]
fn readers_do_not_crash_on_damaged_files() {
    // Panics are expected here; the test collects them.
    std::panic::set_hook(Box::new(|_| {}));
    let mut rng = Lcg(7);
    let mut crashes = Vec::new();
    let mut samples: Vec<(String, Vec<u8>)> = Vec::new();
    for name in ["two-color-patch", "lettering-names", "applique-patch", "full-field"] {
        let design = corpus::build(name).unwrap();
        let (s, _) = stitches(&Input::Design(Box::new(design)), None).unwrap();
        for ext in emb_formats::WRITE_EXTENSIONS {
            let bytes = emb_formats::write_machine(ext, &s, name, "20000101000000").unwrap();
            samples.push((format!("{name}.{ext}"), bytes));
        }
    }
    // Each reader, with its own files damaged and with the files of the other formats.
    for (label, bytes) in &samples {
        let own = label.rsplit('.').next().unwrap().to_string();
        for (i, b) in damaged(bytes, &mut rng).into_iter().enumerate() {
            if let Some(c) = check(&format!("{label} #{i}"), || emb_formats::read_machine(&own, &b)) {
                crashes.push(c);
            }
        }
        for ext in emb_formats::READ_EXTENSIONS {
            if let Some(c) = check(&format!("{label} as .{ext}"), || emb_formats::read_machine(ext, bytes)) {
                crashes.push(c);
            }
        }
    }
    // Random bytes for each reader, the color files and the artwork readers.
    for len in [0usize, 1, 5, 64, 513, 4096] {
        let b: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        for ext in emb_formats::READ_EXTENSIONS {
            if let Some(c) = check(&format!("random {len} as .{ext}"), || {
                emb_formats::read_machine(ext, &b)
            }) {
                crashes.push(c);
            }
        }
        for ext in ["col", "edr", "inf"] {
            if let Some(c) = check(&format!("random {len} as .{ext}"), || {
                emb_formats::colors::read(ext, &b)
            }) {
                crashes.push(c);
            }
        }
        for ext in ["svg", "pdf", "ai", "emf", "wmf"] {
            if let Some(c) = check(&format!("random {len} as .{ext}"), || {
                emb_formats::artwork::import_artwork(ext, &b, false)
            }) {
                crashes.push(c);
            }
        }
    }
    let _ = std::panic::take_hook();
    assert!(crashes.is_empty(), "{} crashes:\n{}", crashes.len(), crashes.join("\n"));
}
