//! Reading and writing files without losing or silently changing data (`JARAK-FR-020…024`,
//! `JARAK-FR-027`, `JARAK-SEC-001`, `JARAK-SEC-002`).
//!
//! - **Decoding**: a byte-order mark decides first (UTF-8, UTF-16 LE/BE); then valid UTF-8 is
//!   UTF-8; then UTF-16 without a BOM is recognised by its zero bytes; otherwise the legacy code
//!   page the caller names (the system ANSI code page by default) decodes it.
//! - **Lossless or read-only**: after decoding, the text is encoded again with the same format;
//!   when that does not give back the exact bytes (a byte the code page does not define, an
//!   unpaired UTF-16 surrogate), the document is marked `lossless: false` and the window opens it
//!   read-only, so a save can never change bytes the user did not edit.
//! - **Line endings** are kept as they are in the text (nothing is normalised); the statistics
//!   say which kinds occur, and new lines use the most frequent one.
//! - **Binary**: zero bytes outside UTF-16 mark a file as binary (opened read-only).
//! - **Atomic save**: the new bytes go to a temporary file in the same folder, are flushed to the
//!   disk, and replace the original in one step (`ReplaceFileW`, which keeps the original's
//!   attributes, or a rename for a new file); on any failure the original is untouched and the
//!   temporary file is removed.

use encoding_rs::Encoding;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The largest file Jarak opens by default (larger files are refused with a clear error; Step
/// 109 measures what the editor can serve).
pub const DEFAULT_SIZE_LIMIT: u64 = 1024 * 1024 * 1024;

/// A text encoding Jarak reads and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    /// A single- or multi-byte code page (`encoding_rs`).
    Legacy(&'static Encoding),
}

impl TextEncoding {
    /// A stable name (`utf-8`, `utf-16le`, `utf-16be`, or the WHATWG label, e.g. `euc-kr`).
    pub fn name(self) -> &'static str {
        match self {
            TextEncoding::Utf8 => "utf-8",
            TextEncoding::Utf16Le => "utf-16le",
            TextEncoding::Utf16Be => "utf-16be",
            TextEncoding::Legacy(e) => e.name(),
        }
    }

    /// What the status bar shows (`UTF-8`, `UTF-8 BOM`, `UTF-16 LE BOM`, `windows-1252`, …).
    pub fn label(self, bom: bool) -> String {
        match (self, bom) {
            (TextEncoding::Utf8, false) => "UTF-8".into(),
            (TextEncoding::Utf8, true) => "UTF-8 BOM".into(),
            (TextEncoding::Utf16Le, _) => "UTF-16 LE BOM".into(),
            (TextEncoding::Utf16Be, _) => "UTF-16 BE BOM".into(),
            (TextEncoding::Legacy(e), _) => e.name().to_string(),
        }
    }
}

/// The legacy encoding of a Windows code page (`GetACP()`), UTF-8 for unknown ones.
pub fn encoding_for_codepage(cp: u32) -> TextEncoding {
    use encoding_rs as e;
    let enc: &'static Encoding = match cp {
        65001 => return TextEncoding::Utf8,
        874 => e::WINDOWS_874,
        932 => e::SHIFT_JIS,
        936 => e::GBK,
        949 => e::EUC_KR,
        950 => e::BIG5,
        1250 => e::WINDOWS_1250,
        1251 => e::WINDOWS_1251,
        1252 => e::WINDOWS_1252,
        1253 => e::WINDOWS_1253,
        1254 => e::WINDOWS_1254,
        1255 => e::WINDOWS_1255,
        1256 => e::WINDOWS_1256,
        1257 => e::WINDOWS_1257,
        1258 => e::WINDOWS_1258,
        _ => return TextEncoding::Utf8,
    };
    TextEncoding::Legacy(enc)
}

/// The legacy encoding named by a WHATWG label (`euc-kr`, `windows-1252`, `shift_jis`, …);
/// `utf-8` names UTF-8; Windows code page names (`cp949`, `windows-1252`, `932`) work too. UTF-16
/// is recognised from the file itself, not named here.
pub fn encoding_for_label(label: &str) -> Option<TextEncoding> {
    let label = label.trim();
    // Windows code page names too: `cp949`, `windows-949`, `949`.
    let lower = label.to_ascii_lowercase();
    let digits = lower
        .strip_prefix("cp")
        .or_else(|| lower.strip_prefix("windows-"))
        .unwrap_or(&lower);
    if let Ok(cp) = digits.parse::<u32>() {
        let e = encoding_for_codepage(cp);
        return (cp == 65001 || e != TextEncoding::Utf8).then_some(e);
    }
    let e = Encoding::for_label(label.as_bytes())?;
    if e == encoding_rs::UTF_8 {
        Some(TextEncoding::Utf8)
    } else if e == encoding_rs::UTF_16LE || e == encoding_rs::UTF_16BE {
        None
    } else {
        Some(TextEncoding::Legacy(e))
    }
}

/// The legacy encoding for files that are neither UTF-8 nor UTF-16: the system ANSI code page
/// (`GetACP`), or — when Windows' "Use Unicode UTF-8 for worldwide language support" made that
/// UTF-8 — the ANSI code page of the user's locale (949 for Korean), which is what such files
/// were written in.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn system_encoding() -> TextEncoding {
    use windows_sys::Win32::Globalization::{
        GetACP, GetLocaleInfoEx, LOCALE_IDEFAULTANSICODEPAGE, LOCALE_RETURN_NUMBER,
    };
    // SAFETY: GetACP takes no arguments and only returns a number.
    let cp = unsafe { GetACP() };
    if cp != 65001 {
        return encoding_for_codepage(cp);
    }
    let mut value: u32 = 0;
    // SAFETY: LOCALE_RETURN_NUMBER writes one u32 (two UTF-16 units) into `value`; a null
    // locale name means the user's default locale.
    let n = unsafe {
        GetLocaleInfoEx(
            std::ptr::null(),
            LOCALE_IDEFAULTANSICODEPAGE | LOCALE_RETURN_NUMBER,
            (&raw mut value).cast(),
            2,
        )
    };
    if n == 0 {
        return TextEncoding::Utf8;
    }
    encoding_for_codepage(value)
}

/// How many line endings of each kind a text has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EolCounts {
    pub crlf: usize,
    pub lf: usize,
    pub cr: usize,
}

impl EolCounts {
    pub fn of(text: &str) -> EolCounts {
        let b = text.as_bytes();
        let mut c = EolCounts::default();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'\r' if b.get(i + 1) == Some(&b'\n') => {
                    c.crlf += 1;
                    i += 1;
                }
                b'\r' => c.cr += 1,
                b'\n' => c.lf += 1,
                _ => {}
            }
            i += 1;
        }
        c
    }

    /// More than one kind occurs.
    pub fn mixed(&self) -> bool {
        [self.crlf, self.lf, self.cr]
            .iter()
            .filter(|n| **n > 0)
            .count()
            > 1
    }

    /// The most frequent kind (`None` without any line ending); ties prefer CRLF, then LF.
    pub fn dominant(&self) -> Option<crate::document::Eol> {
        use crate::document::Eol;
        let best = [
            (self.crlf, Eol::CrLf),
            (self.lf, Eol::Lf),
            (self.cr, Eol::Cr),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .max_by(|a, b| a.0.cmp(&b.0).then(rank(b.1).cmp(&rank(a.1))))?;
        Some(best.1)
    }

    /// `crlf`, `lf`, `cr`, `mixed` or `none`.
    pub fn name(&self) -> &'static str {
        if self.mixed() {
            return "mixed";
        }
        match self.dominant() {
            Some(crate::document::Eol::CrLf) => "crlf",
            Some(crate::document::Eol::Lf) => "lf",
            Some(crate::document::Eol::Cr) => "cr",
            None => "none",
        }
    }
}

fn rank(e: crate::document::Eol) -> u8 {
    match e {
        crate::document::Eol::CrLf => 0,
        crate::document::Eol::Lf => 1,
        crate::document::Eol::Cr => 2,
    }
}

/// How a file's bytes map to its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    pub encoding: TextEncoding,
    /// The file starts with a byte-order mark (kept on save).
    pub bom: bool,
}

impl Format {
    pub fn utf8() -> Format {
        Format {
            encoding: TextEncoding::Utf8,
            bom: false,
        }
    }
}

/// A decoded file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub text: String,
    pub format: Format,
    /// Encoding the text again gives back exactly the original bytes.
    pub lossless: bool,
    /// Zero bytes in a file that is not UTF-16: a binary file.
    pub binary: bool,
    pub eol: EolCounts,
}

impl Decoded {
    /// The window opens it read-only (a save could change bytes the user did not edit).
    pub fn read_only(&self) -> bool {
        !self.lossless || self.binary
    }
}

/// Why text cannot be written in a format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmappable {
    /// The first character the encoding cannot represent, and its char position.
    pub ch: char,
    pub at: usize,
    pub encoding: &'static str,
}

impl std::fmt::Display for Unmappable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} cannot represent {:?} (U+{:04X}) at character {}",
            self.encoding, self.ch, self.ch as u32, self.at
        )
    }
}

/// Decodes `bytes`; `fallback` is the legacy encoding for text that is neither UTF-8 nor UTF-16.
pub fn decode(bytes: &[u8], fallback: TextEncoding) -> Decoded {
    let (encoding, bom, body) = if let Some(rest) = bytes.strip_prefix(b"\xEF\xBB\xBF") {
        (TextEncoding::Utf8, true, rest)
    } else if let Some(rest) = bytes.strip_prefix(b"\xFF\xFE") {
        (TextEncoding::Utf16Le, true, rest)
    } else if let Some(rest) = bytes.strip_prefix(b"\xFE\xFF") {
        (TextEncoding::Utf16Be, true, rest)
    } else if std::str::from_utf8(bytes).is_ok() && looks_utf16(bytes).is_none() {
        (TextEncoding::Utf8, false, bytes)
    } else if let Some(e) = looks_utf16(bytes) {
        (e, false, bytes)
    } else {
        (fallback, false, bytes)
    };
    let (text, mut lossless) = match encoding {
        TextEncoding::Utf8 => match std::str::from_utf8(body) {
            Ok(s) => (s.to_string(), true),
            Err(_) => (String::from_utf8_lossy(body).into_owned(), false),
        },
        TextEncoding::Utf16Le | TextEncoding::Utf16Be => {
            let le = encoding == TextEncoding::Utf16Le;
            let units: Vec<u16> = body
                .chunks(2)
                .map(|c| {
                    let pair = [c[0], *c.get(1).unwrap_or(&0)];
                    if le {
                        u16::from_le_bytes(pair)
                    } else {
                        u16::from_be_bytes(pair)
                    }
                })
                .collect();
            let mut ok = body.len().is_multiple_of(2);
            let s: String = char::decode_utf16(units)
                .map(|r| {
                    r.unwrap_or_else(|_| {
                        ok = false;
                        char::REPLACEMENT_CHARACTER
                    })
                })
                .collect();
            (s, ok)
        }
        TextEncoding::Legacy(e) => {
            let (s, had_errors) = e.decode_without_bom_handling(body);
            (s.into_owned(), !had_errors)
        }
    };
    let format = Format { encoding, bom };
    // The decoded text must encode back to exactly these bytes, or a save would change them.
    if lossless {
        lossless = encode(&text, format).is_ok_and(|b| b == bytes);
    }
    let binary =
        !matches!(encoding, TextEncoding::Utf16Le | TextEncoding::Utf16Be) && body.contains(&0);
    let eol = EolCounts::of(&text);
    Decoded {
        text,
        format,
        lossless,
        binary,
        eol,
    }
}

/// UTF-16 without a byte-order mark: ASCII-heavy text has a zero byte in every other position.
fn looks_utf16(bytes: &[u8]) -> Option<TextEncoding> {
    if bytes.len() < 4 || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let pairs = bytes.len() / 2;
    let even_zero = bytes.iter().step_by(2).filter(|b| **b == 0).count();
    let odd_zero = bytes.iter().skip(1).step_by(2).filter(|b| **b == 0).count();
    // At least 70 % of one side is zero and almost none of the other.
    if odd_zero * 10 >= pairs * 7 && even_zero * 20 <= pairs {
        Some(TextEncoding::Utf16Le)
    } else if even_zero * 10 >= pairs * 7 && odd_zero * 20 <= pairs {
        Some(TextEncoding::Utf16Be)
    } else {
        None
    }
}

/// The bytes of `text` in `format` (with its BOM); refuses characters the encoding cannot hold
/// instead of writing a substitute.
pub fn encode(text: &str, format: Format) -> Result<Vec<u8>, Unmappable> {
    let mut out = Vec::with_capacity(text.len() + 3);
    match format.encoding {
        TextEncoding::Utf8 => {
            if format.bom {
                out.extend_from_slice(b"\xEF\xBB\xBF");
            }
            out.extend_from_slice(text.as_bytes());
        }
        TextEncoding::Utf16Le | TextEncoding::Utf16Be => {
            let le = format.encoding == TextEncoding::Utf16Le;
            if format.bom {
                out.extend_from_slice(if le { b"\xFF\xFE" } else { b"\xFE\xFF" });
            }
            for u in text.encode_utf16() {
                out.extend_from_slice(&if le { u.to_le_bytes() } else { u.to_be_bytes() });
            }
        }
        TextEncoding::Legacy(e) => {
            let (bytes, _, unmappable) = e.encode(text);
            if unmappable {
                // Find the first character the encoding cannot hold, for the message.
                for (at, ch) in text.chars().enumerate() {
                    let mut buf = [0u8; 4];
                    let (_, _, bad) = e.encode(ch.encode_utf8(&mut buf));
                    if bad {
                        return Err(Unmappable {
                            ch,
                            at,
                            encoding: e.name(),
                        });
                    }
                }
            }
            out.extend_from_slice(&bytes);
        }
    }
    Ok(out)
}

/// Why a file cannot be opened.
#[derive(Debug)]
pub enum OpenError {
    Io(io::Error),
    TooLarge { size: u64, limit: u64 },
    Directory,
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenError::Io(e) => write!(f, "{e}"),
            OpenError::TooLarge { size, limit } => write!(
                f,
                "the file has {size} bytes; Jarak opens files up to {limit} bytes"
            ),
            OpenError::Directory => write!(f, "this is a folder, not a file"),
        }
    }
}

/// Reads and decodes a file (refusing folders and files above `limit` bytes).
pub fn read(path: &Path, fallback: TextEncoding, limit: u64) -> Result<Decoded, OpenError> {
    let meta = std::fs::metadata(path).map_err(OpenError::Io)?;
    if meta.is_dir() {
        return Err(OpenError::Directory);
    }
    if meta.len() > limit {
        return Err(OpenError::TooLarge {
            size: meta.len(),
            limit,
        });
    }
    let bytes = std::fs::read(path).map_err(OpenError::Io)?;
    Ok(decode(&bytes, fallback))
}

/// Where a save may fail (for the fault-injection tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Create,
    Write,
    Flush,
    Replace,
}

/// Writes `bytes` to `path` atomically: a temporary file next to it, flushed to the disk, then
/// replacing the original. A read-only original is refused (`PermissionDenied`); on any error the
/// original stays as it was.
pub fn save_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    save_atomic_failing(path, bytes, None)
}

#[doc(hidden)]
pub fn save_atomic_failing(path: &Path, bytes: &[u8], fail: Option<Stage>) -> io::Result<()> {
    let injected = |s: Stage| -> io::Result<()> {
        if fail == Some(s) {
            Err(io::Error::other(format!("injected failure at {s:?}")))
        } else {
            Ok(())
        }
    };
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let exists = path.exists();
    if exists && std::fs::metadata(path)?.permissions().readonly() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "the file is read-only",
        ));
    }
    let name = path
        .file_name()
        .map_or_else(|| "file".into(), |n| n.to_string_lossy().into_owned());
    let tmp = dir.join(format!(".{name}.jarak-{}.tmp", std::process::id()));
    let result = (|| {
        injected(Stage::Create)?;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        injected(Stage::Write)?;
        f.write_all(bytes)?;
        injected(Stage::Flush)?;
        f.sync_all()?;
        drop(f);
        injected(Stage::Replace)?;
        if exists {
            replace(path, &tmp)
        } else {
            std::fs::rename(&tmp, path)
        }
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn replace(target: &Path, tmp: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{REPLACEFILE_IGNORE_MERGE_ERRORS, ReplaceFileW};
    let wide = |p: &Path| -> Vec<u16> {
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let (t, s) = (wide(target), wide(tmp));
    // SAFETY: NUL-terminated paths; no backup file; reserved pointers are null.
    let ok = unsafe {
        ReplaceFileW(
            t.as_ptr(),
            s.as_ptr(),
            std::ptr::null(),
            REPLACEFILE_IGNORE_MERGE_ERRORS,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Why a save did not happen (nothing was written in every case).
#[derive(Debug)]
pub enum SaveError {
    /// The document has no file yet: use Save As.
    NoPath,
    /// The document is read-only (a lossy decode, a binary file, or marked read-only).
    ReadOnly,
    /// The encoding cannot hold a character of the text.
    Unmappable(Unmappable),
    Io(io::Error),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::NoPath => write!(f, "the document has no file yet"),
            SaveError::ReadOnly => write!(
                f,
                "the document is read-only (saving could change bytes you did not edit)"
            ),
            SaveError::Unmappable(u) => write!(f, "{u}"),
            SaveError::Io(e) => write!(f, "{e}"),
        }
    }
}

/// A document and the file it belongs to (`JARAK-FR-020`).
#[derive(Debug, Clone)]
pub struct FileDocument {
    /// `None` for an untitled document.
    pub path: Option<PathBuf>,
    pub format: Format,
    /// Saving is refused (lossy decode, binary, or the user's choice).
    pub read_only: bool,
    /// Why it is read-only, for the window to say.
    pub read_only_reason: Option<String>,
    pub doc: crate::document::Document,
}

impl FileDocument {
    /// A new untitled document (UTF-8 without BOM, Windows line endings).
    pub fn untitled() -> FileDocument {
        FileDocument {
            path: None,
            format: Format::utf8(),
            read_only: false,
            read_only_reason: None,
            doc: crate::document::Document::new(),
        }
    }

    /// Opens `path`. A missing file is a new document that will be created at `path` on the
    /// first save; anything else that cannot be read is an error.
    pub fn open(
        path: &Path,
        fallback: TextEncoding,
        limit: u64,
    ) -> Result<FileDocument, OpenError> {
        match read(path, fallback, limit) {
            Ok(d) => Ok(FileDocument::from_decoded(Some(path.to_path_buf()), d)),
            Err(OpenError::Io(e)) if e.kind() == io::ErrorKind::NotFound => Ok(FileDocument {
                path: Some(path.to_path_buf()),
                ..FileDocument::untitled()
            }),
            Err(e) => Err(e),
        }
    }

    /// A document for decoded bytes.
    pub fn from_decoded(path: Option<PathBuf>, d: Decoded) -> FileDocument {
        let reason = if d.binary {
            Some("the file contains zero bytes (a binary file)".to_string())
        } else if !d.lossless {
            Some(format!(
                "{} cannot reproduce every byte of the file",
                d.format.encoding.label(d.format.bom)
            ))
        } else {
            None
        };
        let mut doc = crate::document::Document::from_text(&d.text);
        drop(d.text);
        if let Some(eol) = d.eol.dominant() {
            doc.eol = eol;
        }
        FileDocument {
            path,
            format: d.format,
            read_only: reason.is_some(),
            read_only_reason: reason,
            doc,
        }
    }

    /// The name shown in a tab or the title (`Untitled` without a file).
    pub fn title(&self) -> String {
        self.path.as_ref().and_then(|p| p.file_name()).map_or_else(
            || "Untitled".to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }

    /// The bytes a save would write.
    pub fn bytes(&self) -> Result<Vec<u8>, Unmappable> {
        encode(&self.doc.text().to_string(), self.format)
    }

    /// Saves to the document's file (atomic; refused while read-only or without a path).
    pub fn save(&mut self) -> Result<(), SaveError> {
        let path = self.path.clone().ok_or(SaveError::NoPath)?;
        if self.read_only {
            return Err(SaveError::ReadOnly);
        }
        self.write_to(&path)?;
        self.doc.mark_saved();
        Ok(())
    }

    /// Saves to `path` and makes it the document's file (Save As). A document that is read-only
    /// because its bytes cannot be reproduced is refused too.
    pub fn save_as(&mut self, path: &Path) -> Result<(), SaveError> {
        if self.read_only && self.read_only_reason.is_some() {
            return Err(SaveError::ReadOnly);
        }
        self.write_to(path)?;
        self.path = Some(path.to_path_buf());
        self.read_only = false;
        self.doc.mark_saved();
        Ok(())
    }

    /// Writes a copy to `path`; the document keeps its own file and dirty state.
    pub fn save_copy(&self, path: &Path) -> Result<(), SaveError> {
        self.write_to(path)
    }

    fn write_to(&self, path: &Path) -> Result<(), SaveError> {
        let bytes = self.bytes().map_err(SaveError::Unmappable)?;
        save_atomic(path, &bytes).map_err(SaveError::Io)
    }

    /// Reads the file again, discarding the text in memory (the caller asked the user first).
    pub fn reload(&mut self, fallback: TextEncoding, limit: u64) -> Result<(), OpenError> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        let d = read(&path, fallback, limit)?;
        *self = FileDocument::from_decoded(Some(path), d);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
