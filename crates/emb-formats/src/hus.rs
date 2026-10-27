//! Husqvarna Viking HUS read (read only).
//!
//! Ported from pyembroidery (MIT), <https://github.com/EmbroidePy/pyembroidery>,
//! commit c16d1b46390a1632ee36ff98eb8b3ec01934a8e9:
//! - `pyembroidery/HusReader.py`: `read` (6-61)
//! - `pyembroidery/EmbCompress.py`: `expand`, `compress` (the stored form,
//!   for tests), `Huffman`, `EmbCompress` (the decompressor)
//! - `pyembroidery/EmbThreadHus.py`: the thread table (`tables::HUS_THREADS`)
//!
//! Format summary: a little-endian header (stitch count, color count,
//! extents, the offsets of three compressed streams), one table index per
//! color, then three compressed streams: the commands, the x moves and the
//! y moves, one byte per stitch. Commands: `80` stitch, `81` jump, `84`
//! color change, `88` trim, `90` end. Moves are signed bytes in 0.1 mm, Y up.
//!
//! The streams use a Huffman and back-reference compression. A bad stream
//! stops the read; the stitches up to that point stay.

use emb_model::StitchDesign;

use crate::common::{Bytes, Low, Pattern, signed8, table_thread, to_design};
use crate::tables::HUS_THREADS;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HusError {
    #[error("file is shorter than the HUS header")]
    TooShort,
}

pub fn read(bytes: &[u8]) -> Result<StitchDesign, HusError> {
    let mut b = Bytes::new(bytes);
    let header = (|| {
        let _magic = b.i32le()?;
        let stitches = b.i32le()?.max(0) as usize;
        let colors = b.i32le()?.max(0) as usize;
        b.skip(8); // extents
        let offsets = (b.i32le()?, b.i32le()?, b.i32le()?);
        b.skip(8 + 2); // text and an unknown value
        Some((stitches, colors, offsets))
    })();
    let Some((count, colors, (cmd_at, x_at, y_at))) = header else {
        return Err(HusError::TooShort);
    };
    let mut threads = Vec::new();
    for i in 0..colors {
        let Some(index) = b.u16le() else { break };
        threads.push(table_thread(
            i,
            HUS_THREADS[usize::from(index) % HUS_THREADS.len()],
            "Husqvarna Viking",
        ));
    }
    let slice = |from: i32, to: Option<i32>| -> &[u8] {
        let from = (from.max(0) as usize).min(bytes.len());
        let to = to.map_or(bytes.len(), |t| (t.max(0) as usize).clamp(from, bytes.len()));
        &bytes[from..to]
    };
    let cmds = expand(slice(cmd_at, Some(x_at)), count);
    let xs = expand(slice(x_at, Some(y_at)), count);
    let ys = expand(slice(y_at, None), count);

    let mut p = Pattern::default();
    for ((&cmd, &x), &y) in cmds.iter().zip(&xs).zip(&ys) {
        let (x, y) = (signed8(x), -signed8(y));
        match cmd {
            0x80 => p.stitch(x, y),
            0x81 => p.jump(x, y),
            0x84 => p.add(Low::ColorChange, x, y),
            0x88 => {
                if x != 0 || y != 0 {
                    p.jump(x, y);
                }
                p.here(Low::Trim);
            }
            _ => break,
        }
    }
    Ok(to_design(p.recs, threads))
}

/// A Huffman table: the code length of each value; the lowest value wins a tie.
struct Huffman {
    lengths: Vec<usize>,
    default: usize,
    width: usize,
    table: Vec<usize>,
}

impl Huffman {
    fn fixed(value: usize) -> Self {
        Huffman {
            lengths: Vec::new(),
            default: value,
            width: 0,
            table: Vec::new(),
        }
    }

    fn new(lengths: Vec<usize>, default: usize) -> Self {
        let width = lengths.iter().copied().max().unwrap_or(0);
        let mut table = Vec::new();
        if width <= 16 {
            let mut size = 1usize << width;
            for bits in 1..=width {
                size /= 2;
                for (i, &l) in lengths.iter().enumerate() {
                    if l == bits {
                        table.extend(std::iter::repeat_n(i, size));
                    }
                }
            }
        }
        Huffman {
            lengths,
            default,
            width,
            table,
        }
    }

    /// The value and its code length for the next 16 bits. `None`: a bad code.
    fn lookup(&self, bits16: usize) -> Option<(usize, usize)> {
        if self.lengths.is_empty() {
            return Some((self.default, 0));
        }
        if self.width > 16 {
            return None;
        }
        let v = *self.table.get(bits16 >> (16 - self.width))?;
        Some((v, self.lengths[v]))
    }
}

/// The decompressor (`EmbCompress`).
struct Expander<'a> {
    data: &'a [u8],
    pos: usize,
    block_left: i64,
    chars: Huffman,
    dist: Huffman,
}

impl<'a> Expander<'a> {
    fn bits(&self, start: usize, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        let end = start + len - 1;
        let mut v: u64 = 0;
        for i in start / 8..=end / 8 {
            v = (v << 8) | u64::from(*self.data.get(i).unwrap_or(&0));
        }
        let unused = (8 - (end + 1) % 8) % 8;
        ((v >> unused) & ((1u64 << len) - 1)) as usize
    }

    fn peek(&self, n: usize) -> usize {
        self.bits(self.pos, n)
    }

    fn pop(&mut self, n: usize) -> usize {
        let v = self.peek(n);
        self.pos += n;
        v
    }

    fn variable_length(&mut self) -> usize {
        let mut m = self.pop(3);
        if m != 7 {
            return m;
        }
        for _ in 0..13 {
            if self.pop(1) == 1 {
                m += 1;
            } else {
                break;
            }
        }
        m
    }

    fn character_length_huffman(&mut self) -> Huffman {
        let count = self.pop(5);
        if count == 0 {
            return Huffman::fixed(self.pop(5));
        }
        let mut lengths = vec![0; count];
        let mut i = 0;
        while i < count {
            if i == 3 {
                i += self.pop(2);
            }
            if let Some(l) = lengths.get_mut(i) {
                *l = self.variable_length();
            }
            i += 1;
        }
        Huffman::new(lengths, 8)
    }

    fn character_huffman(&mut self, length_huffman: &Huffman) -> Option<Huffman> {
        let count = self.pop(9);
        if count == 0 {
            return Some(Huffman::fixed(self.pop(9)));
        }
        let mut lengths = vec![0; count];
        let mut i = 0;
        while i < count {
            let (c, len) = length_huffman.lookup(self.peek(16))?;
            self.pos += len;
            match c {
                0 => i += 1,
                1 => i += 3 + self.pop(4),
                2 => i += 20 + self.pop(9),
                c => {
                    *lengths.get_mut(i)? = c - 2;
                    i += 1;
                }
            }
        }
        Some(Huffman::new(lengths, 0))
    }

    fn distance_huffman(&mut self) -> Huffman {
        let count = self.pop(5);
        if count == 0 {
            return Huffman::fixed(self.pop(5));
        }
        let lengths = (0..count).map(|_| self.variable_length()).collect();
        Huffman::new(lengths, 0)
    }

    fn load_block(&mut self) -> Option<()> {
        self.block_left = self.pop(16) as i64;
        let lengths = self.character_length_huffman();
        self.chars = self.character_huffman(&lengths)?;
        self.dist = self.distance_huffman();
        Some(())
    }

    fn token(&mut self) -> Option<usize> {
        if self.block_left <= 0 {
            self.load_block()?;
        }
        self.block_left -= 1;
        let (v, len) = self.chars.lookup(self.peek(16))?;
        self.pos += len;
        Some(v)
    }

    fn position(&mut self) -> Option<usize> {
        let (v, len) = self.dist.lookup(self.peek(16))?;
        self.pos += len;
        if v == 0 {
            return Some(0);
        }
        let n = v - 1;
        if n >= usize::BITS as usize - 1 {
            return None;
        }
        Some((1 << n) + self.pop(n))
    }
}

/// Decompresses one stream to at most about `size` bytes (`expand`).
pub fn expand(data: &[u8], size: usize) -> Vec<u8> {
    let mut e = Expander {
        data,
        pos: 0,
        block_left: -1,
        chars: Huffman::fixed(0),
        dist: Huffman::fixed(0),
    };
    let mut out: Vec<u8> = Vec::new();
    let total = data.len() * 8;
    while total > e.pos && out.len() <= size {
        let Some(c) = e.token() else { break };
        if c <= 255 {
            out.push(c as u8);
        } else if c == 510 {
            break;
        } else {
            let length = c - 253;
            let Some(back) = e.position().map(|p| p + 1) else { break };
            let Some(start) = out.len().checked_sub(back) else {
                break;
            };
            for i in start..start + length {
                out.push(out[i]);
            }
        }
    }
    out
}

/// The stored form of `compress`: no real compression, for tests. Deviation:
/// pyembroidery writes the 16-bit literal count little-endian, but the
/// decompressor reads it big-endian (pyembroidery's test of this is turned
/// off), so this writes it big-endian. At most 65535 bytes.
#[cfg(test)]
pub fn compress(data: &[u8]) -> Vec<u8> {
    let n = data.len();
    assert!(n <= 0xffff);
    let mut out = vec![(n >> 8) as u8, n as u8, 0x02, 0xa0, 0x01, 0xfe];
    out.extend_from_slice(data);
    out
}

#[cfg(test)]
mod tests {
    use emb_geom::Point;
    use emb_model::StitchKind;

    use super::*;

    #[test]
    fn stored_streams_expand() {
        // As pyembroidery's `test_fake_compression`.
        let mut seed: u32 = 12345;
        for n in [10, 255, 1000, 40000] {
            let data: Vec<u8> = (0..n)
                .map(|_| {
                    seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                    (seed >> 16) as u8
                })
                .collect();
            let got = expand(&compress(&data), n);
            let first = got.iter().zip(&data).position(|(a, b)| a != b);
            assert_eq!((got.len(), first), (n, None), "{n} bytes");
        }
    }

    #[test]
    fn bad_streams_do_not_panic() {
        for data in [
            vec![],
            vec![0xff; 40],
            vec![0x00; 40],
            vec![0x12, 0x34, 0xff, 0x00, 0x99],
        ] {
            let _ = expand(&data, 100);
        }
    }

    /// A file made from the reader's rules with stored streams (no HUS
    /// sample file is public).
    #[test]
    fn reads_a_built_file() {
        let cmds = [0x80u8, 0x80, 0x88, 0x84, 0x81, 0x80, 0x90];
        let xs = [10u8, 10, 0, 0, 50, 0xf6, 0];
        let ys = [0u8, 0xfb, 0, 0, 0, 0, 0];
        let (c, x, y) = (compress(&cmds), compress(&xs), compress(&ys));
        let mut f = Vec::new();
        let int = |f: &mut Vec<u8>, v: i32| f.extend(v.to_le_bytes());
        int(&mut f, 0x0000_7b5b);
        int(&mut f, cmds.len() as i32);
        int(&mut f, 2);
        f.extend([0; 8]);
        let start = 4 * 3 + 8 + 4 * 3 + 8 + 2 + 2 * 2;
        int(&mut f, start as i32);
        int(&mut f, (start + c.len()) as i32);
        int(&mut f, (start + c.len() + x.len()) as i32);
        f.extend(b"        ");
        f.extend([0, 0]);
        f.extend(3u16.to_le_bytes()); // Red
        f.extend(1u16.to_le_bytes()); // Blue
        assert_eq!(f.len(), start);
        f.extend(&c);
        f.extend(&x);
        f.extend(&y);
        let d = read(&f).unwrap();
        let kinds: Vec<StitchKind> = d.records.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            [
                StitchKind::Stitch,
                StitchKind::Stitch,
                StitchKind::Trim,
                StitchKind::ColorChange,
                StitchKind::Jump,
                StitchKind::Stitch,
                StitchKind::End
            ]
        );
        assert_eq!(d.records[1].pos, Point::new(2.0, 0.5));
        assert_eq!(d.records[5].pos, Point::new(6.0, 0.5));
        assert_eq!(d.threads[0].name, "Red");
        assert_eq!(d.threads[1].name, "Blue");
        assert!(read(&[0; 10]).is_err());
    }
}
