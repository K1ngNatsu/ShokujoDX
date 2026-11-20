//! The decode filters of the PostScript reader (ADR 0138): ASCII85,
//! ASCIIHex, RunLength and Flate (PostScript language reference, section
//! 3.13). Each function gives the decoded bytes and the position after
//! the encoded data, so the reader goes on after it. The other filters
//! (LZW, DCT, CCITT fax) give no bytes: the reader uses them only for the
//! data of pictures, which it does not import.

/// The most bytes that one filter gives.
const MAX_OUT: usize = 64 << 20;

/// ASCII85 data from `p` to `~>`.
pub(super) fn ascii85(d: &[u8], mut p: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let mut group = [0u8; 5];
    let mut n = 0;
    while p < d.len() {
        let c = d[p];
        p += 1;
        match c {
            b'~' => {
                if d.get(p) == Some(&b'>') {
                    p += 1;
                }
                break;
            }
            b'z' if n == 0 => out.extend_from_slice(&[0; 4]),
            b'!'..=b'u' => {
                group[n] = c - b'!';
                n += 1;
                if n == 5 {
                    let v = group.iter().fold(0u64, |v, &g| v * 85 + u64::from(g));
                    out.extend_from_slice(&(v as u32).to_be_bytes());
                    n = 0;
                }
            }
            _ => {}
        }
        if out.len() > MAX_OUT {
            break;
        }
    }
    if n > 1 {
        for g in group.iter_mut().skip(n) {
            *g = 84;
        }
        let v = group.iter().fold(0u64, |v, &g| v * 85 + u64::from(g));
        out.extend_from_slice(&(v as u32).to_be_bytes()[..n - 1]);
    }
    (out, p)
}

/// ASCIIHex data from `p` to `>`.
pub(super) fn ascii_hex(d: &[u8], mut p: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let mut high: Option<u8> = None;
    while p < d.len() {
        let c = d[p];
        p += 1;
        if c == b'>' {
            break;
        }
        let Some(v) = (c as char).to_digit(16) else {
            continue;
        };
        match high.take() {
            Some(h) => out.push(h << 4 | v as u8),
            None => high = Some(v as u8),
        }
        if out.len() > MAX_OUT {
            break;
        }
    }
    if let Some(h) = high {
        out.push(h << 4);
    }
    (out, p)
}

/// RunLength data from `p` to its end byte (128).
pub(super) fn run_length(d: &[u8], mut p: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    while p < d.len() && out.len() <= MAX_OUT {
        let len = d[p];
        p += 1;
        match len {
            128 => break,
            0..=127 => {
                let end = (p + usize::from(len) + 1).min(d.len());
                out.extend_from_slice(&d[p..end]);
                p = end;
            }
            _ => {
                let Some(&b) = d.get(p) else { break };
                p += 1;
                out.extend(std::iter::repeat_n(b, 257 - usize::from(len)));
            }
        }
    }
    (out, p)
}

/// Flate (zlib) data from `p` to the end of its stream.
pub(super) fn flate(d: &[u8], p: usize) -> (Vec<u8>, usize) {
    let input = d.get(p..).unwrap_or_default();
    let mut z = flate2::Decompress::new(true);
    let mut out = Vec::with_capacity(input.len() * 2 + 64);
    loop {
        if out.len() == out.capacity() {
            out.reserve(out.len().max(4096));
        }
        let before = (z.total_in(), z.total_out());
        let at = (z.total_in() as usize).min(input.len());
        match z.decompress_vec(&input[at..], &mut out, flate2::FlushDecompress::None) {
            Ok(flate2::Status::StreamEnd) => break,
            Ok(_) if (z.total_in(), z.total_out()) == before => break,
            Ok(_) if out.len() > MAX_OUT => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    (out, p + z.total_in() as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filters_decode_and_find_their_ends() {
        // "Hello" in ASCII85 is 87cURD]i... ; here a known pair.
        let (b, end) = ascii85(b"9jqo^~> rest", 0);
        assert_eq!(b, b"Man ");
        assert_eq!(end, 7);
        assert_eq!(ascii85(b"z~>", 0).0, vec![0; 4]);
        assert_eq!(ascii85(b"9jqo~>", 0).0, b"Man");
        let (h, end) = ascii_hex(b"48 65 6c6C 6>x", 0);
        assert_eq!(h, b"Hell`");
        assert_eq!(end, 13);
        let (r, end) = run_length(&[2, b'a', b'b', b'c', 254, b'x', 128, 9], 0);
        assert_eq!(r, b"abcxxx");
        assert_eq!(end, 7);
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut z, b"0 0 moveto 10 10 lineto stroke").unwrap();
        let mut data = z.finish().unwrap();
        let n = data.len();
        data.extend_from_slice(b"after");
        let (f, end) = flate(&data, 0);
        assert_eq!(f, b"0 0 moveto 10 10 lineto stroke");
        assert_eq!(end, n);
    }
}
