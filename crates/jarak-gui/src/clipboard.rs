//! The clipboard (`JARAK-FR-031`): Jarak reads and writes it itself through Win32, only on an
//! explicit Copy, Cut or Paste (`JARAK-SEC-003`), so it controls the formats (text now, images in
//! Step 90) and needs no extra crate. eframe's own clipboard shortcuts are off
//! (`NativeOptions::clipboard_shortcuts`), so Ctrl+C/X/V reach Jarak as ordinary keys.

use crate::images::ClipImage;

/// Where Copy, Cut and Paste go (the Windows clipboard in the program, a fake in tests).
pub trait Clipboard: Send + Sync {
    fn get_text(&self) -> Option<String>;
    fn set_text(&self, text: &str) -> bool;
    /// Step 103: text that may hold NUL characters (Copy / Paste binary content); by default the
    /// text itself.
    fn set_binary(&self, text: &str) -> bool {
        self.set_text(text)
    }
    fn get_binary(&self) -> Option<String> {
        self.get_text()
    }
    /// An image on the clipboard (Step 90): PNG, a bitmap, or copied image files.
    fn get_image(&self) -> Option<ClipImage> {
        None
    }
    /// Step 106: text with an HTML fragment (`HTML Format`) or RTF beside it (Copy as HTML /
    /// RTF); by default only the text.
    fn set_rich(&self, text: &str, _html: Option<&str>, _rtf: Option<&str>) -> bool {
        self.set_text(text)
    }
    /// Step 110 (EDIT-13): the HTML fragment and the RTF on the clipboard, read only on an
    /// explicit Paste HTML / RTF as source; by default none.
    fn get_rich(&self) -> (Option<String>, Option<String>) {
        (None, None)
    }
}

/// An in-memory clipboard (tests, and no system clipboard).
#[derive(Debug, Default)]
pub struct MemoryClipboard {
    pub text: std::sync::Mutex<Option<String>>,
    pub image: std::sync::Mutex<Option<ClipImage>>,
    /// Step 106: the HTML fragment and RTF of the last Copy as HTML / RTF.
    pub html: std::sync::Mutex<Option<String>>,
    pub rtf: std::sync::Mutex<Option<String>>,
}

impl MemoryClipboard {
    /// Puts an image on it (and takes any text away, as a screenshot tool does).
    pub fn set_image(&self, img: ClipImage) {
        if let Ok(mut t) = self.text.lock() {
            *t = None;
        }
        if let Ok(mut i) = self.image.lock() {
            *i = Some(img);
        }
    }
}

impl Clipboard for MemoryClipboard {
    fn get_text(&self) -> Option<String> {
        self.text.lock().ok()?.clone()
    }
    fn set_text(&self, text: &str) -> bool {
        self.text
            .lock()
            .map(|mut c| *c = Some(text.to_string()))
            .is_ok()
    }
    fn get_image(&self) -> Option<ClipImage> {
        self.image.lock().ok()?.clone()
    }
    fn set_rich(&self, text: &str, html: Option<&str>, rtf: Option<&str>) -> bool {
        if let Ok(mut h) = self.html.lock() {
            *h = html.map(str::to_string);
        }
        if let Ok(mut r) = self.rtf.lock() {
            *r = rtf.map(str::to_string);
        }
        self.set_text(text)
    }
    fn get_rich(&self) -> (Option<String>, Option<String>) {
        (
            self.html.lock().ok().and_then(|h| h.clone()),
            self.rtf.lock().ok().and_then(|r| r.clone()),
        )
    }
}

/// The Windows clipboard, `CF_UNICODETEXT`.
#[derive(Debug, Default)]
pub struct WinClipboard;

#[cfg(windows)]
#[allow(unsafe_code)]
mod win {
    use windows_sys::Win32::Foundation::{GlobalFree, HANDLE};
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable,
        OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock,
    };

    const CF_UNICODETEXT: u32 = 13;
    const CF_DIB: u32 = 8;
    const CF_DIBV5: u32 = 17;
    const CF_HDROP: u32 = 15;

    /// The bytes of clipboard format `format` (the clipboard must be open).
    unsafe fn bytes_of(format: u32) -> Option<Vec<u8>> {
        // SAFETY: the caller opened the clipboard; the handle stays owned by the clipboard; the
        // locked memory has `GlobalSize` bytes.
        unsafe {
            let h = GetClipboardData(format);
            if h.is_null() {
                return None;
            }
            let size = windows_sys::Win32::System::Memory::GlobalSize(h as _);
            if size == 0 || size > crate::images::MAX_BYTES {
                return None;
            }
            let p = GlobalLock(h as _) as *const u8;
            if p.is_null() {
                return None;
            }
            let v = std::slice::from_raw_parts(p, size).to_vec();
            GlobalUnlock(h as _);
            Some(v)
        }
    }

    pub fn get_image() -> Option<crate::images::ClipImage> {
        use crate::images::ClipImage;
        use windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW;
        let name: Vec<u16> = "PNG".encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: a NUL-terminated name.
        let png = unsafe { RegisterClipboardFormatW(name.as_ptr()) };
        // SAFETY: plain queries.
        let has = |f: u32| f != 0 && unsafe { IsClipboardFormatAvailable(f) } != 0;
        if !(has(png) || has(CF_DIBV5) || has(CF_DIB) || has(CF_HDROP)) || !open() {
            return None;
        }
        // SAFETY: the clipboard is open for the reads below and closed after.
        let img = unsafe {
            if has(png)
                && let Some(b) = bytes_of(png)
            {
                Some(ClipImage::Png(b))
            } else if has(CF_DIBV5)
                && let Some(b) = bytes_of(CF_DIBV5)
            {
                Some(ClipImage::Dib(b))
            } else if has(CF_DIB)
                && let Some(b) = bytes_of(CF_DIB)
            {
                Some(ClipImage::Dib(b))
            } else if has(CF_HDROP) {
                use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};
                let h = GetClipboardData(CF_HDROP) as HDROP;
                if h.is_null() {
                    None
                } else {
                    let n = DragQueryFileW(h, u32::MAX, std::ptr::null_mut(), 0);
                    let mut files = Vec::new();
                    for i in 0..n.min(32) {
                        let len = DragQueryFileW(h, i, std::ptr::null_mut(), 0) as usize;
                        let mut buf = vec![0u16; len + 1];
                        DragQueryFileW(h, i, buf.as_mut_ptr(), buf.len() as u32);
                        buf.truncate(len);
                        files.push(std::path::PathBuf::from(String::from_utf16_lossy(&buf)));
                    }
                    // Only files whose bytes say they are images.
                    files.retain(|f| {
                        std::fs::File::open(f)
                            .and_then(|mut file| {
                                let mut head = [0u8; 16];
                                use std::io::Read;
                                let n = file.read(&mut head)?;
                                Ok(crate::images::sniff(&head[..n]).is_some())
                            })
                            .unwrap_or(false)
                    });
                    (!files.is_empty()).then_some(ClipImage::Files(files))
                }
            } else {
                None
            }
        };
        close();
        img
    }

    /// Opens the clipboard, retrying briefly while another program holds it.
    fn open() -> bool {
        for _ in 0..10 {
            // SAFETY: no owner window; the clipboard is closed again by `close`.
            if unsafe { OpenClipboard(std::ptr::null_mut()) } != 0 {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        false
    }

    fn close() {
        // SAFETY: only called after a successful `open`.
        unsafe {
            CloseClipboard();
        }
    }

    pub fn get_text() -> Option<String> {
        // SAFETY: a plain query.
        if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) } == 0 || !open() {
            return None;
        }
        // SAFETY: the clipboard is open; the handle stays owned by the clipboard; the locked
        // memory is a NUL-terminated UTF-16 string, read up to its NUL or its size.
        let text = unsafe {
            let h = GetClipboardData(CF_UNICODETEXT);
            if h.is_null() {
                None
            } else {
                let p = GlobalLock(h as _) as *const u16;
                if p.is_null() {
                    None
                } else {
                    let size = windows_sys::Win32::System::Memory::GlobalSize(h as _) / 2;
                    let mut n = 0usize;
                    while n < size && *p.add(n) != 0 {
                        n += 1;
                    }
                    let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
                    GlobalUnlock(h as _);
                    Some(s)
                }
            }
        };
        close();
        text
    }

    /// Step 103: the clipboard format of text with NULs: its UTF-16 length (u64, little-endian),
    /// then the units.
    fn binary_format() -> u32 {
        let name: Vec<u16> = "Jarak Binary Text"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: a NUL-terminated name; registering an existing name returns its id.
        unsafe { windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW(name.as_ptr()) }
    }

    /// Step 103: `text` as plain text (up to a NUL, as other programs read it) and in full in the
    /// binary format.
    pub fn set_binary(text: &str) -> bool {
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut packed = Vec::with_capacity(units.len() * 2 + 8);
        packed.extend_from_slice(&(units.len() as u64).to_le_bytes());
        for u in &units {
            packed.extend_from_slice(&u.to_le_bytes());
        }
        let format = binary_format();
        let wide: Vec<u16> = units.iter().copied().chain(std::iter::once(0)).collect();
        if format == 0 || !open() {
            return false;
        }
        // SAFETY: as in `set_text`: memory filled while locked, owned by the clipboard after a
        // successful SetClipboardData, freed here when that fails.
        let ok = unsafe {
            EmptyClipboard();
            let put = |data: &[u8], fmt: u32| -> bool {
                let mem = GlobalAlloc(GMEM_MOVEABLE, data.len().max(1));
                if mem.is_null() {
                    return false;
                }
                let p = GlobalLock(mem) as *mut u8;
                if p.is_null() {
                    GlobalFree(mem);
                    return false;
                }
                std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
                GlobalUnlock(mem);
                if SetClipboardData(fmt, mem as HANDLE).is_null() {
                    GlobalFree(mem);
                    false
                } else {
                    true
                }
            };
            let bytes: Vec<u8> = wide.iter().flat_map(|u| u.to_le_bytes()).collect();
            put(&bytes, CF_UNICODETEXT) & put(&packed, format)
        };
        close();
        ok
    }

    /// Step 103: the binary format's text, else the plain text.
    pub fn get_binary() -> Option<String> {
        let format = binary_format();
        // SAFETY: a plain query.
        if format == 0 || unsafe { IsClipboardFormatAvailable(format) } == 0 {
            return get_text();
        }
        if !open() {
            return None;
        }
        // SAFETY: the clipboard is open; the handle stays the clipboard's; the locked memory holds
        // the length then the units (read within its size).
        let text = unsafe {
            let h = GetClipboardData(format);
            if h.is_null() {
                None
            } else {
                let p = GlobalLock(h as _) as *const u8;
                if p.is_null() {
                    None
                } else {
                    let size = windows_sys::Win32::System::Memory::GlobalSize(h as _);
                    let all = std::slice::from_raw_parts(p, size);
                    let n = all.get(..8).map_or(0, |b| {
                        u64::from_le_bytes(b.try_into().unwrap_or([0; 8])) as usize
                    });
                    let units: Vec<u16> = all
                        .get(8..8 + n * 2)
                        .unwrap_or(&[])
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    GlobalUnlock(h as _);
                    Some(String::from_utf16_lossy(&units))
                }
            }
        };
        close();
        text
    }

    /// Puts `bytes` on the open clipboard as `format` (the clipboard owns the memory after).
    unsafe fn put(format: u32, bytes: &[u8]) -> bool {
        // SAFETY: the caller opened the clipboard; the memory is filled while locked and handed
        // to the clipboard, which owns it after a successful SetClipboardData (freed here only
        // when that fails).
        unsafe {
            let mem = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1));
            if mem.is_null() {
                return false;
            }
            let p = GlobalLock(mem) as *mut u8;
            if p.is_null() {
                GlobalFree(mem);
                return false;
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            GlobalUnlock(mem);
            if SetClipboardData(format, mem as HANDLE).is_null() {
                GlobalFree(mem);
                false
            } else {
                true
            }
        }
    }

    /// Step 106: the text with `HTML Format` and/or `Rich Text Format` beside it.
    pub fn set_rich(text: &str, html: Option<&str>, rtf: Option<&str>) -> bool {
        use windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW;
        let name = |n: &str| -> Vec<u16> { n.encode_utf16().chain(std::iter::once(0)).collect() };
        let wide: Vec<u8> = text
            .encode_utf16()
            .chain(std::iter::once(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        if !open() {
            return false;
        }
        // SAFETY: the clipboard is open until `close`; the names are NUL-terminated.
        let ok = unsafe {
            EmptyClipboard();
            let mut ok = put(CF_UNICODETEXT, &wide);
            if let Some(h) = html {
                let f = RegisterClipboardFormatW(name("HTML Format").as_ptr());
                let mut b = jarak_core::export::cf_html(h).into_bytes();
                b.push(0);
                ok &= f != 0 && put(f, &b);
            }
            if let Some(r) = rtf {
                let f = RegisterClipboardFormatW(name("Rich Text Format").as_ptr());
                let mut b = r.as_bytes().to_vec();
                b.push(0);
                ok &= f != 0 && put(f, &b);
            }
            ok
        };
        close();
        ok
    }

    /// Step 110 (EDIT-13): the `HTML Format` fragment (UTF-8) and `Rich Text Format` (bytes up
    /// to their NUL), when present.
    pub fn get_rich() -> (Option<String>, Option<String>) {
        use windows_sys::Win32::System::DataExchange::RegisterClipboardFormatW;
        let name = |n: &str| -> Vec<u16> { n.encode_utf16().chain(std::iter::once(0)).collect() };
        let text = |b: Vec<u8>| -> String {
            let n = b.iter().position(|&x| x == 0).unwrap_or(b.len());
            String::from_utf8_lossy(&b[..n]).into_owned()
        };
        // SAFETY: NUL-terminated names; registering an existing name returns its id.
        let (fh, fr) = unsafe {
            (
                RegisterClipboardFormatW(name("HTML Format").as_ptr()),
                RegisterClipboardFormatW(name("Rich Text Format").as_ptr()),
            )
        };
        if !open() {
            return (None, None);
        }
        // SAFETY: the clipboard is open until `close`.
        let (h, r) = unsafe {
            (
                (fh != 0 && IsClipboardFormatAvailable(fh) != 0)
                    .then(|| bytes_of(fh))
                    .flatten(),
                (fr != 0 && IsClipboardFormatAvailable(fr) != 0)
                    .then(|| bytes_of(fr))
                    .flatten(),
            )
        };
        close();
        (
            h.map(|b| jarak_core::export::cf_html_fragment(&text(b))),
            r.map(text),
        )
    }

    pub fn set_text(text: &str) -> bool {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        if !open() {
            return false;
        }
        // SAFETY: the clipboard is open; the global memory is filled while locked and handed to
        // the clipboard, which owns it after a successful SetClipboardData (freed here only when
        // that fails).
        let ok = unsafe {
            EmptyClipboard();
            let mem = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2);
            if mem.is_null() {
                false
            } else {
                let p = GlobalLock(mem) as *mut u16;
                if p.is_null() {
                    GlobalFree(mem);
                    false
                } else {
                    std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
                    GlobalUnlock(mem);
                    if SetClipboardData(CF_UNICODETEXT, mem as HANDLE).is_null() {
                        GlobalFree(mem);
                        false
                    } else {
                        true
                    }
                }
            }
        };
        close();
        ok
    }
}

impl Clipboard for WinClipboard {
    fn get_image(&self) -> Option<ClipImage> {
        #[cfg(windows)]
        {
            win::get_image()
        }
    }
    fn get_text(&self) -> Option<String> {
        #[cfg(windows)]
        {
            win::get_text()
        }
    }
    fn set_text(&self, text: &str) -> bool {
        #[cfg(windows)]
        {
            win::set_text(text)
        }
    }
    fn set_binary(&self, text: &str) -> bool {
        #[cfg(windows)]
        {
            win::set_binary(text)
        }
    }
    fn set_rich(&self, text: &str, html: Option<&str>, rtf: Option<&str>) -> bool {
        #[cfg(windows)]
        {
            win::set_rich(text, html, rtf)
        }
    }
    fn get_binary(&self) -> Option<String> {
        #[cfg(windows)]
        {
            win::get_binary()
        }
    }
    fn get_rich(&self) -> (Option<String>, Option<String>) {
        #[cfg(windows)]
        {
            win::get_rich()
        }
    }
}

/// Pasted text with its line endings changed to the document's (`\r\n`, `\n` or `\r`), as
/// Notepad++ pastes by default.
pub fn with_line_endings(text: &str, eol: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str(eol);
            }
            '\n' => out.push_str(eol),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pasted_line_endings_follow_the_document() {
        assert_eq!(
            with_line_endings("a\nb\r\nc\rd", "\r\n"),
            "a\r\nb\r\nc\r\nd"
        );
        assert_eq!(with_line_endings("a\r\nb", "\n"), "a\nb");
        assert_eq!(with_line_endings("한글", "\r\n"), "한글");
    }

    #[test]
    fn the_memory_clipboard_keeps_text() {
        let c = MemoryClipboard::default();
        assert_eq!(c.get_text(), None);
        assert!(c.set_text("x"));
        assert_eq!(c.get_text().as_deref(), Some("x"));
    }
}
