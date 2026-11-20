//! The scanner of the PostScript reader (ADR 0138): the text of a program
//! to objects, with the syntax of the PostScript language reference
//! (section 3.2). A procedure in the text is one object. The scanner is
//! lenient: a stray `)`, `>` or `}` is skipped, a string or a procedure at
//! the end of the file ends there, and an immediate name (`//name`) that
//! has no value stays a name.

use super::filters;
use super::vm::{MAX_PROC_DEPTH, Obj, PsError, R, Vm};

pub(super) fn is_white(c: u8) -> bool {
    // Control-D ends a job on a printer: here it is white space.
    matches!(c, 0 | b'\t' | b'\n' | 12 | b'\r' | b' ' | 4)
}

fn is_delim(c: u8) -> bool {
    matches!(c, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

enum Tok {
    Obj(Obj),
    Open,
    Close,
    Immediate(u32),
}

impl Vm {
    /// The next object of source `src` from `pos` (a procedure is one
    /// object); `None` at the end of the text.
    pub(super) fn scan(&mut self, src: u32, pos: &mut usize) -> R<Option<Obj>> {
        let data = self.sources[src as usize].clone();
        let mut procs: Vec<Vec<Obj>> = Vec::new();
        loop {
            let Some(tok) = self.token(&data, pos)? else {
                // A procedure with no end ends at the end of the text.
                while let Some(items) = procs.pop() {
                    let p = Obj::Arr(self.new_array(items)?, true);
                    match procs.last_mut() {
                        Some(outer) => outer.push(p),
                        None => return Ok(Some(p)),
                    }
                }
                return Ok(None);
            };
            let obj = match tok {
                Tok::Open => {
                    if procs.len() >= MAX_PROC_DEPTH {
                        return Err(PsError::LimitCheck);
                    }
                    procs.push(Vec::new());
                    continue;
                }
                Tok::Close => match procs.pop() {
                    Some(items) => Obj::Arr(self.new_array(items)?, true),
                    None => continue,
                },
                Tok::Immediate(n) => self.lookup(n).unwrap_or(Obj::Name(n, true)),
                Tok::Obj(o) => o,
            };
            match procs.last_mut() {
                Some(p) => p.push(obj),
                None => return Ok(Some(obj)),
            }
        }
    }

    fn token(&mut self, d: &[u8], pos: &mut usize) -> R<Option<Tok>> {
        let mut p = *pos;
        let tok = loop {
            while p < d.len() && is_white(d[p]) {
                p += 1;
            }
            if p >= d.len() {
                *pos = p;
                return Ok(None);
            }
            let c = d[p];
            let next = d.get(p + 1).copied();
            match c {
                b'%' => {
                    while p < d.len() && d[p] != b'\n' && d[p] != b'\r' {
                        p += 1;
                    }
                }
                b'(' => {
                    let (s, end) = literal_string(d, p + 1);
                    p = end;
                    break Tok::Obj(Obj::Str(self.new_string(s)?, false));
                }
                b'<' if next == Some(b'<') => {
                    p += 2;
                    break Tok::Obj(Obj::Name(self.intern(b"<<"), true));
                }
                b'<' if next == Some(b'~') => {
                    let (s, end) = filters::ascii85(d, p + 2);
                    p = end;
                    break Tok::Obj(Obj::Str(self.new_string(s)?, false));
                }
                b'<' => {
                    let (s, end) = filters::ascii_hex(d, p + 1);
                    p = end;
                    break Tok::Obj(Obj::Str(self.new_string(s)?, false));
                }
                b'>' if next == Some(b'>') => {
                    p += 2;
                    break Tok::Obj(Obj::Name(self.intern(b">>"), true));
                }
                b'[' | b']' => {
                    p += 1;
                    break Tok::Obj(Obj::Name(self.intern(&[c]), true));
                }
                b'{' => {
                    p += 1;
                    break Tok::Open;
                }
                b'}' => {
                    p += 1;
                    break Tok::Close;
                }
                // A stray end of a string or of a hex string.
                b')' | b'>' => p += 1,
                b'/' => {
                    let immediate = next == Some(b'/');
                    let start = p + if immediate { 2 } else { 1 };
                    let end = regular_end(d, start);
                    let n = self.intern(&d[start..end]);
                    p = after_white(d, end);
                    break if immediate {
                        Tok::Immediate(n)
                    } else {
                        Tok::Obj(Obj::Name(n, false))
                    };
                }
                _ => {
                    let end = regular_end(d, p);
                    let word = &d[p..end];
                    let o = match number(word) {
                        Some(o) => o,
                        None => Obj::Name(self.intern(word), true),
                    };
                    p = after_white(d, end);
                    break Tok::Obj(o);
                }
            }
        };
        *pos = p;
        Ok(Some(tok))
    }
}

/// The end of a regular token that starts at `p`.
fn regular_end(d: &[u8], mut p: usize) -> usize {
    while p < d.len() && !is_white(d[p]) && !is_delim(d[p]) {
        p += 1;
    }
    p
}

/// The scanner takes the one white-space character after a token (a
/// carriage return and a line feed are one).
fn after_white(d: &[u8], p: usize) -> usize {
    match d.get(p) {
        Some(b'\r') if d.get(p + 1) == Some(&b'\n') => p + 2,
        Some(&c) if is_white(c) => p + 1,
        _ => p,
    }
}

/// A string in parentheses that starts after `(` at `p`: its bytes and the
/// position after its `)`.
pub(super) fn literal_string(d: &[u8], mut p: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let mut depth = 1;
    while p < d.len() {
        let c = d[p];
        p += 1;
        match c {
            b'(' => {
                depth += 1;
                out.push(c);
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                out.push(c);
            }
            b'\\' => {
                let Some(&e) = d.get(p) else { break };
                p += 1;
                match e {
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'b' => out.push(8),
                    b'f' => out.push(12),
                    // A backslash at the end of a line joins the lines.
                    b'\r' => {
                        if d.get(p) == Some(&b'\n') {
                            p += 1;
                        }
                    }
                    b'\n' => {}
                    b'0'..=b'7' => {
                        let mut v = u32::from(e - b'0');
                        for _ in 0..2 {
                            match d.get(p) {
                                Some(&o @ b'0'..=b'7') => {
                                    v = v * 8 + u32::from(o - b'0');
                                    p += 1;
                                }
                                _ => break,
                            }
                        }
                        out.push((v & 0xff) as u8);
                    }
                    other => out.push(other),
                }
            }
            b'\r' => {
                if d.get(p) == Some(&b'\n') {
                    p += 1;
                }
                out.push(b'\n');
            }
            _ => out.push(c),
        }
    }
    (out, p)
}

/// A number token: an integer, a real or a radix number (`16#FF`).
pub(super) fn number(w: &[u8]) -> Option<Obj> {
    if w.is_empty() {
        return None;
    }
    if let Some(h) = w.iter().position(|&c| c == b'#') {
        let base: u32 = std::str::from_utf8(&w[..h]).ok()?.parse().ok()?;
        if !(2..=36).contains(&base) || h + 1 >= w.len() {
            return None;
        }
        let mut v: u64 = 0;
        for &c in &w[h + 1..] {
            let digit = (c as char).to_digit(base)?;
            v = v * u64::from(base) + u64::from(digit);
            if v > u64::from(u32::MAX) {
                return None;
            }
        }
        // A radix number is the 32 bits of an integer.
        return Some(Obj::Int(v as u32 as i32));
    }
    let body = match w[0] {
        b'+' | b'-' => &w[1..],
        _ => w,
    };
    if body.is_empty() {
        return None;
    }
    if body.iter().all(u8::is_ascii_digit) {
        let s = std::str::from_utf8(w).ok()?;
        return Some(match s.parse::<i32>() {
            Ok(i) => Obj::Int(i),
            Err(_) => Obj::Real(s.parse::<f64>().ok()?),
        });
    }
    // A real: digits with one point, then an exponent.
    let (mantissa, exponent) = match body.iter().position(|&c| c == b'e' || c == b'E') {
        Some(e) => (&body[..e], Some(&body[e + 1..])),
        None => (body, None),
    };
    let digits = mantissa.iter().filter(|c| c.is_ascii_digit()).count();
    let points = mantissa.iter().filter(|&&c| c == b'.').count();
    if digits == 0 || points > 1 || digits + points != mantissa.len() {
        return None;
    }
    if let Some(e) = exponent {
        let e = match e.first() {
            Some(b'+' | b'-') => &e[1..],
            _ => e,
        };
        if e.is_empty() || !e.iter().all(u8::is_ascii_digit) {
            return None;
        }
    }
    let v: f64 = std::str::from_utf8(w).ok()?.parse().ok()?;
    v.is_finite().then_some(Obj::Real(v))
}
