//! Line operations and text transformations — Notepad++'s Edit menu (`JARAK-FR-082`…
//! `JARAK-FR-085`, `JARAK-FR-088`, Step 97).
//!
//! Every operation is one undo step. Line operations work on the lines the selection touches (a
//! selection ending at the start of a line leaves that line out); the sorting, removing and blank
//! operations work on the whole document when nothing is selected. Rewritten lines are joined with
//! the document's line ending. Case and encodings work on the selected text of every range. A
//! decoder that meets text it cannot decode changes nothing and says why ([`Document::notice`]).

use crate::document::Document;
use crate::selection::{Range, Selection};
use crate::text;
use sha2::Digest;

/// How lines sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    /// By code point.
    Lex,
    /// Ignoring case.
    LexNoCase,
    /// In the user's locale order (Windows `CompareStringEx`).
    Locale,
    /// By the integer each line starts with (lines without one first).
    Integer,
    /// By the decimal each line starts with, `.` as the separator.
    DecimalDot,
    /// The same with `,`.
    DecimalComma,
    /// By length in characters.
    Length,
}

/// Case conversions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Upper,
    Lower,
    /// The first letter of every word upper, the rest lower; `ProperBlend` keeps the rest.
    Proper,
    ProperBlend,
    /// The first letter of every sentence upper, the rest lower; `SentenceBlend` keeps the rest.
    Sentence,
    SentenceBlend,
    Invert,
    Random,
}

/// Blank operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blank {
    TrimTrailing,
    TrimLeading,
    TrimBoth,
    EolToSpace,
    TrimBothEolToSpace,
    TabToSpace,
    SpaceToTabAll,
    SpaceToTabLeading,
}

/// Encodings of the selected text (as UTF-8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Base64,
    Base64NoPad,
    Base64Url,
    Url,
    QuotedPrintable,
    Hex,
}

/// Digests (lower-case hex).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hash {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl Hash {
    pub fn name(self) -> &'static str {
        match self {
            Hash::Md5 => "MD5",
            Hash::Sha1 => "SHA-1",
            Hash::Sha256 => "SHA-256",
            Hash::Sha512 => "SHA-512",
        }
    }

    /// The digest of `bytes` in lower-case hex.
    pub fn hex(self, bytes: &[u8]) -> String {
        let d: Vec<u8> = match self {
            Hash::Md5 => md5(bytes).to_vec(),
            Hash::Sha1 => sha1(bytes).to_vec(),
            Hash::Sha256 => sha2::Sha256::digest(bytes).to_vec(),
            Hash::Sha512 => sha2::Sha512::digest(bytes).to_vec(),
        };
        d.iter().map(|b| format!("{b:02x}")).collect()
    }
}

// ---- digests (RFC 1321 MD5, FIPS 180-4 SHA-1; SHA-2 comes from `sha2`) ---------------------------

fn pad64(bytes: &[u8], big_endian: bool) -> Vec<u8> {
    let mut m = bytes.to_vec();
    let bits = (bytes.len() as u64).wrapping_mul(8);
    m.push(0x80);
    while m.len() % 64 != 56 {
        m.push(0);
    }
    m.extend_from_slice(&if big_endian {
        bits.to_be_bytes()
    } else {
        bits.to_le_bytes()
    });
    m
}

/// MD5 (RFC 1321).
pub fn md5(bytes: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64)
        .map(|i| ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32)
        .collect();
    let (mut a0, mut b0, mut c0, mut d0) = (
        0x6745_2301u32,
        0xefcd_ab89u32,
        0x98ba_dcfeu32,
        0x1032_5476u32,
    );
    for chunk in pad64(bytes, false).chunks(64) {
        let m: Vec<u32> = chunk
            .chunks(4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    for (i, v) in [a0, b0, c0, d0].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

/// SHA-1 (FIPS 180-4).
pub fn sha1(bytes: &[u8]) -> [u8; 20] {
    let mut h = [
        0x6745_2301u32,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    for chunk in pad64(bytes, true).chunks(64) {
        let mut w = [0u32; 80];
        for (i, word) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i / 20 {
                0 => ((b & c) | (!b & d), 0x5A82_7999),
                1 => (b ^ c ^ d, 0x6ED9_EBA1),
                2 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6u32),
            };
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ---- codecs -----------------------------------------------------------------------------------------

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Step 110 (EDIT-16, SRCH-20): a character from its code — `U+00E9`, `0xE9`, `#xE9`, `&#233;`
/// or `233`.
pub fn parse_char_code(s: &str) -> Option<char> {
    let t = s.trim().trim_end_matches(';');
    let t = t.strip_prefix("&#").unwrap_or(t);
    let n = if let Some(h) = t
        .strip_prefix("U+")
        .or_else(|| t.strip_prefix("u+"))
        .or_else(|| t.strip_prefix("0x"))
        .or_else(|| t.strip_prefix("0X"))
        .or_else(|| t.strip_prefix('x'))
        .or_else(|| t.strip_prefix("#x"))
    {
        u32::from_str_radix(h, 16).ok()?
    } else {
        t.parse::<u32>().ok()?
    };
    char::from_u32(n)
}

pub(crate) fn base64_encode(bytes: &[u8], pad: bool, url: bool) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        let digits = [(n >> 18) & 63, (n >> 12) & 63, (n >> 6) & 63, n & 63];
        for (k, d) in digits.iter().enumerate().take(c.len() + 1) {
            let _ = k;
            let mut ch = B64[*d as usize] as char;
            if url {
                ch = match ch {
                    '+' => '-',
                    '/' => '_',
                    x => x,
                };
            }
            out.push(ch);
        }
        if pad {
            for _ in c.len()..3 {
                out.push('=');
            }
        }
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut bits = 0u32;
    let mut n = 0;
    let mut out = Vec::new();
    for ch in s.chars().filter(|c| !c.is_whitespace()) {
        if ch == '=' {
            break;
        }
        let v = match ch {
            'A'..='Z' => ch as u32 - 'A' as u32,
            'a'..='z' => ch as u32 - 'a' as u32 + 26,
            '0'..='9' => ch as u32 - '0' as u32 + 52,
            '+' | '-' => 62,
            '/' | '_' => 63,
            _ => return Err(format!("not Base64: {ch:?}")),
        };
        bits = (bits << 6) | v;
        n += 6;
        if n >= 8 {
            n -= 8;
            out.push((bits >> n) as u8);
            bits &= (1 << n) - 1;
        }
    }
    Ok(out)
}

fn url_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (*b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn hex_val(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|d| d as u8)
}

fn url_decode(s: &str) -> Result<Vec<u8>, String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let v = b
                .get(i + 1)
                .and_then(|x| hex_val(*x))
                .zip(b.get(i + 2).and_then(|x| hex_val(*x)))
                .ok_or_else(|| format!("not a %XX escape at byte {i}"))?;
            out.push(v.0 * 16 + v.1);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Ok(out)
}

/// Quoted-printable (RFC 2045): line breaks stay hard breaks (CRLF), lines fold at 76 with `=`.
fn qp_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut line = String::new();
    let flush = |out: &mut String, line: &mut String| {
        out.push_str(line);
        line.clear();
    };
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\r' && bytes.get(i + 1) == Some(&b'\n') || b == b'\n' {
            // A space or tab before a hard break is encoded.
            if let Some(last) = line.pop() {
                if last == ' ' || last == '\t' {
                    line.push_str(&format!("={:02X}", last as u8));
                } else {
                    line.push(last);
                }
            }
            flush(&mut out, &mut line);
            out.push_str("\r\n");
            i += if b == b'\r' { 2 } else { 1 };
            continue;
        }
        let piece = if (33..=126).contains(&b) && b != b'=' || b == b' ' || b == b'\t' {
            (b as char).to_string()
        } else {
            format!("={b:02X}")
        };
        if line.len() + piece.len() > 75 {
            line.push('=');
            flush(&mut out, &mut line);
            out.push_str("\r\n");
        }
        line.push_str(&piece);
        i += 1;
    }
    if let Some(last) = line.pop() {
        if last == ' ' || last == '\t' {
            line.push_str(&format!("={:02X}", last as u8));
        } else {
            line.push(last);
        }
    }
    out.push_str(&line);
    out
}

fn qp_decode(s: &str) -> Result<Vec<u8>, String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' {
            if b.get(i + 1) == Some(&b'\r') && b.get(i + 2) == Some(&b'\n') {
                i += 3;
                continue;
            }
            if b.get(i + 1) == Some(&b'\n') {
                i += 2;
                continue;
            }
            let v = b
                .get(i + 1)
                .and_then(|x| hex_val(*x))
                .zip(b.get(i + 2).and_then(|x| hex_val(*x)))
                .ok_or_else(|| format!("not an =XX escape at byte {i}"))?;
            out.push(v.0 * 16 + v.1);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    Ok(out)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let digits: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    if !digits.len().is_multiple_of(2) {
        return Err("an odd number of hex digits".into());
    }
    digits
        .chunks(2)
        .map(|p| {
            hex_val(p[0])
                .zip(hex_val(p[1]))
                .map(|(a, b)| a * 16 + b)
                .ok_or_else(|| format!("not hex: {}", String::from_utf8_lossy(p)))
        })
        .collect()
}

impl Codec {
    pub fn encode(self, s: &str) -> String {
        let b = s.as_bytes();
        match self {
            Codec::Base64 => base64_encode(b, true, false),
            Codec::Base64NoPad => base64_encode(b, false, false),
            Codec::Base64Url => base64_encode(b, false, true),
            Codec::Url => url_encode(b),
            Codec::QuotedPrintable => qp_encode(b),
            Codec::Hex => hex_encode(b),
        }
    }

    /// The decoded text; it must be UTF-8.
    pub fn decode(self, s: &str) -> Result<String, String> {
        let bytes = match self {
            Codec::Base64 | Codec::Base64NoPad | Codec::Base64Url => base64_decode(s)?,
            Codec::Url => url_decode(s)?,
            Codec::QuotedPrintable => qp_decode(s)?,
            Codec::Hex => hex_decode(s)?,
        };
        String::from_utf8(bytes).map_err(|_| "the decoded bytes are not UTF-8 text".to_string())
    }
}

// ---- case ---------------------------------------------------------------------------------------------

/// A small xorshift generator (randomize, ranDOm cAsE).
#[derive(Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    /// Seeded from the clock.
    pub fn from_clock() -> Rng {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9E37_79B9, |d| d.as_nanos() as u64);
        Rng::new(t)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
}

fn upper(c: char) -> String {
    c.to_uppercase().collect()
}

fn lower(c: char) -> String {
    c.to_lowercase().collect()
}

/// `s` in `case`.
pub fn convert_case(s: &str, case: Case, rng: &mut Rng) -> String {
    match case {
        Case::Upper => s.to_uppercase(),
        Case::Lower => s.to_lowercase(),
        Case::Invert => s
            .chars()
            .map(|c| {
                if c.is_uppercase() {
                    lower(c)
                } else if c.is_lowercase() {
                    upper(c)
                } else {
                    c.to_string()
                }
            })
            .collect(),
        Case::Random => s
            .chars()
            .map(|c| {
                if rng.next_u64() & 1 == 0 {
                    upper(c)
                } else {
                    lower(c)
                }
            })
            .collect(),
        Case::Proper | Case::ProperBlend => {
            let mut out = String::with_capacity(s.len());
            let mut in_word = false;
            for c in s.chars() {
                let word = c.is_alphanumeric() || c == '\'';
                if word && !in_word {
                    out.push_str(&upper(c));
                } else if word && case == Case::Proper {
                    out.push_str(&lower(c));
                } else {
                    out.push(c);
                }
                in_word = word;
            }
            out
        }
        Case::Sentence | Case::SentenceBlend => {
            let mut out = String::with_capacity(s.len());
            let mut start = true;
            for c in s.chars() {
                if c.is_alphabetic() {
                    if start {
                        out.push_str(&upper(c));
                        start = false;
                    } else if case == Case::Sentence {
                        out.push_str(&lower(c));
                    } else {
                        out.push(c);
                    }
                } else {
                    if matches!(c, '.' | '!' | '?' | '。' | '！' | '？') {
                        start = true;
                    }
                    out.push(c);
                }
            }
            out
        }
    }
}

// ---- sorting --------------------------------------------------------------------------------------------

/// The number a line starts with (after blanks), for integer and decimal sorts.
fn leading_number(s: &str, decimal: Option<char>) -> Option<f64> {
    let t = s.trim_start();
    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_sep = false;
    for (i, c) in t.char_indices() {
        let ok = c.is_ascii_digit()
            || (i == 0 && (c == '-' || c == '+'))
            || (Some(c) == decimal && !seen_sep && seen_digit);
        if !ok {
            break;
        }
        if Some(c) == decimal {
            seen_sep = true;
        }
        seen_digit |= c.is_ascii_digit();
        end = i + c.len_utf8();
    }
    if !seen_digit {
        return None;
    }
    t[..end]
        .trim_end_matches(|c| Some(c) == decimal)
        .replace(decimal.unwrap_or('.'), ".")
        .parse()
        .ok()
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use windows_sys::Win32::Globalization::CompareStringEx;
    let wa: Vec<u16> = a.encode_utf16().collect();
    let wb: Vec<u16> = b.encode_utf16().collect();
    // SAFETY: both slices live across the call; a null locale name is the user's locale.
    let r = unsafe {
        CompareStringEx(
            std::ptr::null(),
            0,
            wa.as_ptr(),
            wa.len() as i32,
            wb.as_ptr(),
            wb.len() as i32,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    };
    match r {
        1 => std::cmp::Ordering::Less,
        3 => std::cmp::Ordering::Greater,
        2 => std::cmp::Ordering::Equal,
        _ => a.cmp(b),
    }
}

#[cfg(not(windows))]
fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b))
}

/// Sorts `lines` by `keys` (the same length) — stable; descending reverses the order of
/// different keys only.
pub fn sort_lines(lines: &mut Vec<String>, keys: &[String], kind: Sort, descending: bool) {
    let mut idx: Vec<usize> = (0..lines.len()).collect();
    let num = |s: &str| match kind {
        Sort::Integer => leading_number(s, None),
        Sort::DecimalDot => leading_number(s, Some('.')),
        Sort::DecimalComma => leading_number(s, Some(',')),
        _ => None,
    };
    idx.sort_by(|&x, &y| {
        let (a, b) = (&keys[x], &keys[y]);
        let o = match kind {
            Sort::Lex => a.cmp(b),
            Sort::LexNoCase => a.to_lowercase().cmp(&b.to_lowercase()),
            Sort::Locale => locale_cmp(a, b),
            Sort::Length => a.chars().count().cmp(&b.chars().count()),
            _ => match (num(a), num(b)) {
                (Some(p), Some(q)) => p.partial_cmp(&q).unwrap_or(std::cmp::Ordering::Equal),
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            },
        };
        if descending { o.reverse() } else { o }
    });
    let old = std::mem::take(lines);
    *lines = idx.into_iter().map(|i| old[i].clone()).collect();
}

// ---- blanks ---------------------------------------------------------------------------------------------

/// The display width of one character (wide ones two).
fn char_width(c: char) -> usize {
    text::grapheme_width(c.encode_utf8(&mut [0u8; 4]), 0, 8)
}

fn expand_tabs(s: &str, tab: usize) -> String {
    let mut out = String::with_capacity(s.len());
    let mut col = 0;
    for c in s.chars() {
        if c == '\t' {
            let n = tab - col % tab;
            out.extend(std::iter::repeat_n(' ', n));
            col += n;
        } else {
            out.push(c);
            col += char_width(c);
        }
    }
    out
}

/// Spaces to tabs: runs of two or more spaces that reach a tab stop become a tab (leading: only
/// the indentation).
fn spaces_to_tabs(s: &str, tab: usize, leading_only: bool) -> String {
    let mut out = String::with_capacity(s.len());
    let mut col = 0;
    let mut run = 0;
    let mut indent = true;
    for c in s.chars() {
        if c == ' ' && (indent || !leading_only) {
            run += 1;
            col += 1;
            if col % tab == 0 {
                if run >= 2 || (indent && run >= 1 && tab == 1) {
                    out.push('\t');
                } else {
                    out.extend(std::iter::repeat_n(' ', run));
                }
                run = 0;
            }
            continue;
        }
        out.extend(std::iter::repeat_n(' ', run));
        run = 0;
        if c == '\t' {
            out.push('\t');
            col += tab - col % tab;
        } else {
            indent = false;
            out.push(c);
            col += char_width(c);
        }
    }
    out.extend(std::iter::repeat_n(' ', run));
    out
}

impl Document {
    /// The first and last line the primary selection touches (a selection of several lines that
    /// ends at a line's start leaves that line out).
    pub fn selected_line_span(&self) -> (usize, usize) {
        let r = self.selection().primary();
        let t = self.text();
        let l0 = t.char_to_line(r.from());
        let mut l1 = t.char_to_line(r.to());
        if l1 > l0 && t.line_to_char(l1) == r.to() {
            l1 -= 1;
        }
        (l0, l1)
    }

    /// The lines an operation works on: the selection's, or every line when nothing is selected
    /// (`whole`) — without the empty line after a final line break.
    fn op_lines(&self, whole: bool) -> (usize, usize) {
        let t = self.text();
        if whole && self.selection().ranges().iter().all(Range::is_empty) {
            let mut last = t.len_lines().saturating_sub(1);
            if last > 0 && t.line(last).len_chars() == 0 {
                last -= 1;
            }
            return (0, last);
        }
        self.selected_line_span()
    }

    /// The text of lines `l0..=l1` without their line breaks.
    pub fn lines_of(&self, l0: usize, l1: usize) -> Vec<String> {
        let t = self.text();
        (l0..=l1)
            .map(|l| {
                let line = t.line(l);
                line.slice(..text::line_len(line)).to_string()
            })
            .collect()
    }

    /// Replaces lines `l0..=l1` (their text, not the last line break) with `lines`, as one undo
    /// step, and selects them (`select`) or leaves the caret at their start.
    fn put_lines(&mut self, l0: usize, l1: usize, lines: &[String], select: bool) {
        let t = self.text();
        let a = t.line_to_char(l0);
        let last = t.line(l1);
        let b = t.line_to_char(l1) + text::line_len(last);
        let new = lines.join(self.eol.as_str());
        if t.slice(a..b) == new.as_str() {
            return;
        }
        let n = new.chars().count();
        self.replace(vec![(a, b, new)], Some(a));
        if select {
            self.set_selection(&Selection::single(Range::new(a, a + n)));
        }
    }

    /// A message for the user from the last operation that could not run (`None` after one that
    /// did); the window shows it.
    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    // ---- lines --------------------------------------------------------------------------------------

    /// Ctrl+D: a non-empty selection is duplicated after itself; otherwise the caret's line
    /// below itself (the caret stays). Every range.
    pub fn duplicate(&mut self) {
        let t = self.text();
        let mut parts = Vec::new();
        let mut seen = Vec::new();
        for r in self.selection().ranges() {
            if r.is_empty() {
                let l = t.char_to_line(r.head);
                if seen.contains(&l) {
                    continue;
                }
                seen.push(l);
                let line = t.line(l);
                let end = t.line_to_char(l) + text::line_len(line);
                let content = line.slice(..text::line_len(line)).to_string();
                parts.push((end, end, format!("{}{content}", self.eol.as_str())));
            } else {
                parts.push((r.to(), r.to(), t.slice(r.from()..r.to()).to_string()));
            }
        }
        let keep = self.selection().clone();
        self.replace(parts, None);
        // Carets and selections stay on the originals.
        self.set_selection(&keep);
    }

    /// Ctrl+Shift+L: the lines every range touches go (with their line breaks).
    pub fn delete_lines(&mut self) {
        let t = self.text();
        let mut spans: Vec<(usize, usize)> = Vec::new();
        for r in self.selection().ranges() {
            let l0 = t.char_to_line(r.from());
            let mut l1 = t.char_to_line(r.to());
            if l1 > l0 && t.line_to_char(l1) == r.to() {
                l1 -= 1;
            }
            spans.push((l0, l1));
        }
        let last = t.len_lines() - 1;
        let mut parts: Vec<(usize, usize, String)> = spans
            .iter()
            .map(|&(l0, l1)| {
                let a = t.line_to_char(l0);
                let b = if l1 < last {
                    t.line_to_char(l1 + 1)
                } else {
                    t.len_chars()
                };
                // The last line takes the line break before it.
                let a = if l1 == last && l0 > 0 {
                    let prev = t.line(l0 - 1);
                    t.line_to_char(l0 - 1) + text::line_len(prev)
                } else {
                    a
                };
                (a, b, String::new())
            })
            .collect();
        parts.sort_by_key(|p| p.0);
        parts.dedup();
        let caret = parts.first().map(|p| p.0);
        self.replace(parts, caret);
    }

    /// Ctrl+Shift+Up / Down: the selected lines move one line up or down (still selected).
    pub fn move_selected_lines(&mut self, down: bool) {
        let (l0, l1) = self.selected_line_span();
        let t = self.text();
        let last = {
            let mut l = t.len_lines().saturating_sub(1);
            if l > 0 && t.line(l).len_chars() == 0 {
                l -= 1;
            }
            l
        };
        if (!down && l0 == 0) || (down && l1 >= last) {
            return;
        }
        let r = self.selection().primary();
        let (a0, a1) = if down { (l0, l1 + 1) } else { (l0 - 1, l1) };
        let mut lines = self.lines_of(a0, a1);
        if down {
            let x = lines.pop().unwrap_or_default();
            lines.insert(0, x);
        } else {
            let x = lines.remove(0);
            lines.push(x);
        }
        let shift = if down {
            self.lines_of(l1 + 1, l1 + 1)[0].chars().count() + self.eol.as_str().chars().count()
        } else {
            self.lines_of(l0 - 1, l0 - 1)[0].chars().count() + self.eol.as_str().chars().count()
        };
        self.put_lines(a0, a1, &lines, false);
        let mv = |p: usize| if down { p + shift } else { p - shift };
        self.set_selection(&Selection::single(Range::new(mv(r.anchor), mv(r.head))));
    }

    /// Ctrl+J: the selected lines (or this line and the next) become one; a space goes between
    /// joined words that would otherwise touch.
    pub fn join_lines(&mut self) {
        let (l0, mut l1) = self.selected_line_span();
        if l1 == l0 {
            l1 = (l0 + 1).min(self.text().len_lines() - 1);
        }
        let lines = self.lines_of(l0, l1);
        let mut out = String::new();
        for (i, l) in lines.iter().enumerate() {
            if i > 0 {
                let before = out.chars().next_back().is_some_and(char::is_whitespace);
                let after = l.chars().next().is_some_and(char::is_whitespace);
                if !before && !after && !out.is_empty() && !l.is_empty() {
                    out.push(' ');
                }
            }
            out.push_str(l);
        }
        self.put_lines(l0, l1, &[out], false);
    }

    /// Ctrl+I: the selected lines are split at word boundaries to at most `width` display columns
    /// (a longer word stays whole).
    pub fn split_lines(&mut self, width: usize) {
        let width = width.max(1);
        let (l0, l1) = self.selected_line_span();
        let mut out = Vec::new();
        for l in self.lines_of(l0, l1) {
            let mut cur = String::new();
            let mut cur_w = 0;
            for word in l.split_inclusive(' ') {
                let w: usize = word.trim_end().chars().map(char_width).sum();
                if cur_w + w > width && !cur.trim().is_empty() {
                    out.push(cur.trim_end().to_string());
                    cur.clear();
                    cur_w = 0;
                }
                cur.push_str(word);
                cur_w += word.chars().map(char_width).sum::<usize>();
            }
            out.push(cur.trim_end().to_string());
        }
        self.put_lines(l0, l1, &out, true);
    }

    /// Removes duplicate lines (the first stays) — `consecutive`: only repeats next to each other.
    pub fn remove_duplicate_lines(&mut self, consecutive: bool) {
        let (l0, l1) = self.op_lines(true);
        let mut out: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for l in self.lines_of(l0, l1) {
            let keep = if consecutive {
                out.last() != Some(&l)
            } else {
                seen.insert(l.clone())
            };
            if keep {
                out.push(l);
            }
        }
        self.put_lines(l0, l1, &out, false);
    }

    /// Removes empty lines — `blank`: also lines of only blanks.
    pub fn remove_empty_lines(&mut self, blank: bool) {
        let (l0, l1) = self.op_lines(true);
        let out: Vec<String> = self
            .lines_of(l0, l1)
            .into_iter()
            .filter(|l| {
                if blank {
                    !l.trim().is_empty()
                } else {
                    !l.is_empty()
                }
            })
            .collect();
        let out = if out.is_empty() {
            vec![String::new()]
        } else {
            out
        };
        self.put_lines(l0, l1, &out, false);
    }

    /// Ctrl+Alt+Enter / Ctrl+Alt+Shift+Enter: an empty line above or below the caret's line, the
    /// caret on it.
    pub fn insert_blank_line(&mut self, above: bool) {
        let t = self.text();
        let l = t.char_to_line(self.selection().primary().head);
        let eol = self.eol.as_str().to_string();
        let start = t.line_to_char(l);
        let end = start + text::line_len(t.line(l));
        if above {
            self.replace(vec![(start, start, eol)], Some(start));
        } else {
            let n = eol.chars().count();
            self.replace(vec![(end, end, eol)], Some(end + n));
        }
    }

    /// Reverses the order of the lines (selection or document).
    pub fn reverse_lines(&mut self) {
        let (l0, l1) = self.op_lines(true);
        let mut lines = self.lines_of(l0, l1);
        lines.reverse();
        self.put_lines(l0, l1, &lines, false);
    }

    /// Shuffles the lines (selection or document).
    pub fn randomize_lines(&mut self, rng: &mut Rng) {
        let (l0, l1) = self.op_lines(true);
        let mut lines = self.lines_of(l0, l1);
        for i in (1..lines.len()).rev() {
            lines.swap(i, rng.below(i + 1));
        }
        self.put_lines(l0, l1, &lines, false);
    }

    /// Sorts the lines (selection or document); with a column selection by the text in its
    /// columns (`LINE-19`).
    pub fn sort(&mut self, kind: Sort, descending: bool) {
        let block = self.block();
        let (l0, l1) = match block {
            Some(b) => b.lines(),
            None => self.op_lines(true),
        };
        let mut lines = self.lines_of(l0, l1);
        let keys: Vec<String> = match block {
            Some(b) => {
                let (c0, c1) = b.cols();
                (l0..=l1)
                    .map(|l| {
                        let (a, z) = (self.pos_at_display(l, c0), self.pos_at_display(l, c1));
                        self.text().slice(a..z).to_string()
                    })
                    .collect()
            }
            None => lines.clone(),
        };
        sort_lines(&mut lines, &keys, kind, descending);
        self.put_lines(l0, l1, &lines, false);
    }

    // ---- text -----------------------------------------------------------------------------------------

    /// Applies `f` to every selected range's text as one undo step; `false` when nothing is
    /// selected. The ranges stay selected.
    fn map_selections(&mut self, f: &mut dyn FnMut(&str) -> Result<String, String>) -> bool {
        let ranges: Vec<Range> = self
            .selection()
            .ranges()
            .iter()
            .copied()
            .filter(|r| !r.is_empty())
            .collect();
        if ranges.is_empty() {
            self.notice = Some("Select the text first.".into());
            return false;
        }
        let mut parts = Vec::with_capacity(ranges.len());
        for r in &ranges {
            let s = self.text().slice(r.from()..r.to()).to_string();
            match f(&s) {
                Ok(n) => parts.push((r.from(), r.to(), n)),
                Err(e) => {
                    self.notice = Some(e);
                    return false;
                }
            }
        }
        let mut delta: isize = 0;
        let mut sel = Vec::with_capacity(parts.len());
        for (a, b, s) in &parts {
            let n = s.chars().count();
            let start = (*a as isize + delta) as usize;
            sel.push(Range::new(start, start + n));
            delta += n as isize - (*b - *a) as isize;
        }
        let primary = self.selection().primary_index().min(sel.len() - 1);
        self.replace(parts, None);
        self.set_selection(&Selection::new(sel, primary));
        true
    }

    /// Converts the case of the selected text.
    pub fn change_case(&mut self, case: Case, rng: &mut Rng) -> bool {
        self.map_selections(&mut |s| Ok(convert_case(s, case, rng)))
    }

    /// Encodes the selected text.
    pub fn encode(&mut self, c: Codec) -> bool {
        self.map_selections(&mut |s| Ok(c.encode(s)))
    }

    /// Decodes the selected text (nothing changes when any range cannot be decoded).
    pub fn decode(&mut self, c: Codec) -> bool {
        self.map_selections(&mut |s| c.decode(s))
    }

    /// The blank operations (selection's lines or the document).
    pub fn blank(&mut self, op: Blank) {
        let (l0, l1) = self.op_lines(true);
        let tab = self.tab_width.max(1);
        let lines = self.lines_of(l0, l1);
        let out: Vec<String> = match op {
            Blank::TrimTrailing => lines.iter().map(|l| l.trim_end().to_string()).collect(),
            Blank::TrimLeading => lines.iter().map(|l| l.trim_start().to_string()).collect(),
            Blank::TrimBoth => lines.iter().map(|l| l.trim().to_string()).collect(),
            Blank::EolToSpace => vec![lines.join(" ")],
            Blank::TrimBothEolToSpace => {
                vec![lines.iter().map(|l| l.trim()).collect::<Vec<_>>().join(" ")]
            }
            Blank::TabToSpace => lines.iter().map(|l| expand_tabs(l, tab)).collect(),
            Blank::SpaceToTabAll => lines
                .iter()
                .map(|l| spaces_to_tabs(l, tab, false))
                .collect(),
            Blank::SpaceToTabLeading => {
                lines.iter().map(|l| spaces_to_tabs(l, tab, true)).collect()
            }
        };
        self.put_lines(l0, l1, &out, false);
    }
}

/// Line comment commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentMode {
    Toggle,
    Add,
    Remove,
}

impl Document {
    /// Ctrl+Q / Ctrl+K / Ctrl+Shift+K (`CMT-01`, `CMT-02`, Step 98): the selected lines get or lose
    /// the language's line comment (`// `) at their common indentation; a language with only block
    /// comments (HTML, CSS) wraps each line in them. Toggle comments unless every non-empty line is
    /// commented already. Empty lines stay as they are.
    pub fn comment_lines(&mut self, mode: CommentMode) -> bool {
        let (l0, l1) = self.selected_line_span();
        // A selection stays on the rewritten lines (another toggle works on them again).
        let had_selection = !self.selection().primary().is_empty();
        let (open, close) = match (&self.comment_line, &self.comment_block) {
            (Some(lc), _) => (lc.clone(), String::new()),
            (None, Some((o, c))) => (o.clone(), c.clone()),
            (None, None) => {
                self.notice = Some("This language has no comments.".into());
                return false;
            }
        };
        let lines = self.lines_of(l0, l1);
        let is_commented = |l: &str| {
            let t = l.trim_start();
            t.starts_with(open.as_str())
                && (close.is_empty() || t.trim_end().ends_with(close.as_str()))
        };
        let all = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .all(|l| is_commented(l));
        let add = match mode {
            CommentMode::Add => true,
            CommentMode::Remove => false,
            CommentMode::Toggle => !all,
        };
        let indent = lines
            .iter()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').count())
            .min()
            .unwrap_or(0);
        let out: Vec<String> = lines
            .iter()
            .map(|l| {
                if l.trim().is_empty() {
                    return l.clone();
                }
                if add {
                    let (head, rest): (String, String) = (
                        l.chars().take(indent).collect(),
                        l.chars().skip(indent).collect(),
                    );
                    if close.is_empty() {
                        format!("{head}{open} {rest}")
                    } else {
                        format!("{head}{open} {rest} {close}")
                    }
                } else if is_commented(l) {
                    let lead: String = l.chars().take_while(|c| c.is_whitespace()).collect();
                    let t = &l[lead.len()..];
                    let mut body = &t[open.len()..];
                    body = body.strip_prefix(' ').unwrap_or(body);
                    if !close.is_empty() {
                        let b = body.trim_end();
                        body = b.strip_suffix(close.as_str()).unwrap_or(b);
                        body = body.strip_suffix(' ').unwrap_or(body);
                    }
                    format!("{lead}{body}")
                } else {
                    l.clone()
                }
            })
            .collect();
        self.put_lines(l0, l1, &out, had_selection);
        true
    }

    /// Ctrl+Shift+Q (`CMT-03`): the selection (or the caret's line) is wrapped in the block
    /// comment; `remove`: a selection in one is unwrapped. Languages without block comments use
    /// their line comment on each line.
    pub fn block_comment(&mut self, remove: bool) -> bool {
        let Some((open, close)) = self.comment_block.clone() else {
            return self.comment_lines(if remove {
                CommentMode::Remove
            } else {
                CommentMode::Add
            });
        };
        let r = self.selection().primary();
        let (a, b) = if r.is_empty() {
            let (l0, l1) = self.selected_line_span();
            let t = self.text();
            (
                t.line_to_char(l0),
                t.line_to_char(l1) + text::line_len(t.line(l1)),
            )
        } else {
            (r.from(), r.to())
        };
        let s = self.text().slice(a..b).to_string();
        let new = if remove {
            let t = s.trim();
            if !(t.starts_with(open.as_str())
                && t.ends_with(close.as_str())
                && t.len() >= open.len() + close.len())
            {
                self.notice = Some("The selection is not in a block comment.".into());
                return false;
            }
            let lead = &s[..s.len() - s.trim_start().len()];
            let trail = &s[s.trim_end().len()..];
            let inner = &t[open.len()..t.len() - close.len()];
            let inner = inner.strip_prefix(' ').unwrap_or(inner);
            let inner = inner.strip_suffix(' ').unwrap_or(inner);
            format!("{lead}{inner}{trail}")
        } else {
            format!("{open} {s} {close}")
        };
        let n = new.chars().count();
        self.replace(vec![(a, b, new)], Some(a));
        self.set_selection(&Selection::single(Range::new(a, a + n)));
        true
    }
}

/// A date and time for [`format_datetime`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// Notepad++'s custom date format: `yyyy yy MM M dd d HH H hh h mm m ss s tt`, anything else
/// as it is (text in single quotes is literal).
pub fn format_datetime(fmt: &str, t: DateTime) -> String {
    let f: Vec<char> = fmt.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < f.len() {
        let c = f[i];
        if c == '\'' {
            i += 1;
            while i < f.len() && f[i] != '\'' {
                out.push(f[i]);
                i += 1;
            }
            i += 1;
            continue;
        }
        let mut n = 1;
        while i + n < f.len() && f[i + n] == c {
            n += 1;
        }
        let h12 = if t.hour.is_multiple_of(12) {
            12
        } else {
            t.hour % 12
        };
        let piece = match (c, n) {
            ('y', 4..) => format!("{:04}", t.year),
            ('y', _) => format!("{:02}", t.year.rem_euclid(100)),
            ('M', 2..) => format!("{:02}", t.month),
            ('M', _) => t.month.to_string(),
            ('d', 2..) => format!("{:02}", t.day),
            ('d', _) => t.day.to_string(),
            ('H', 2..) => format!("{:02}", t.hour),
            ('H', _) => t.hour.to_string(),
            ('h', 2..) => format!("{h12:02}"),
            ('h', _) => h12.to_string(),
            ('m', 2..) => format!("{:02}", t.minute),
            ('m', _) => t.minute.to_string(),
            ('s', 2..) => format!("{:02}", t.second),
            ('s', _) => t.second.to_string(),
            ('t', _) => (if t.hour < 12 { "AM" } else { "PM" }).to_string(),
            _ => std::iter::repeat_n(c, n).collect(),
        };
        out.push_str(&piece);
        i += n;
    }
    out
}

#[cfg(test)]
mod tests;
