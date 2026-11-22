//! What Windows says about the user's preferences, read-only: the app theme, "Show animations
//! in Windows" and the display language. Nothing here changes a setting.

/// Windows' "Choose your default app mode" is *Light* (`AppsUseLightTheme` = 1).
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn apps_use_light_theme() -> bool {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    let key: Vec<u16> = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"
        .encode_utf16()
        .chain([0])
        .collect();
    let name: Vec<u16> = "AppsUseLightTheme".encode_utf16().chain([0]).collect();
    let mut v: u32 = 0;
    let mut len = size_of::<u32>() as u32;
    // SAFETY: both strings are NUL-terminated and live across the call; `v` and `len` are valid
    // for writes of a DWORD and its size.
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&raw mut v).cast(),
            &raw mut len,
        )
    };
    r == 0 && v == 1
}

#[cfg(not(windows))]
pub fn apps_use_light_theme() -> bool {
    false
}

/// Windows High Contrast is on: its system colours (`SPI_GETHIGHCONTRAST`, `GetSysColor`);
/// `None` when it is off or cannot be read (DESIGN_SYSTEM §4.3).
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn high_contrast() -> Option<crate::SystemColors> {
    use windows_sys::Win32::Graphics::Gdi::{
        COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_HOTLIGHT, COLOR_WINDOW,
        COLOR_WINDOWTEXT, GetSysColor,
    };
    use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETHIGHCONTRAST, SystemParametersInfoW};
    let mut hc = HIGHCONTRASTW {
        cbSize: size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    // SAFETY: SPI_GETHIGHCONTRAST fills the HIGHCONTRASTW whose size is given.
    let ok =
        unsafe { SystemParametersInfoW(SPI_GETHIGHCONTRAST, hc.cbSize, (&raw mut hc).cast(), 0) };
    if ok == 0 || hc.dwFlags & HCF_HIGHCONTRASTON == 0 {
        return None;
    }
    let c = |i| {
        // SAFETY: GetSysColor takes a colour index and returns 0x00BBGGRR.
        let v = unsafe { GetSysColor(i) };
        crate::Rgb {
            r: (v & 0xFF) as u8,
            g: ((v >> 8) & 0xFF) as u8,
            b: ((v >> 16) & 0xFF) as u8,
        }
    };
    Some(crate::SystemColors {
        window: c(COLOR_WINDOW),
        window_text: c(COLOR_WINDOWTEXT),
        highlight: c(COLOR_HIGHLIGHT),
        highlight_text: c(COLOR_HIGHLIGHTTEXT),
        gray_text: c(COLOR_GRAYTEXT),
        hot_light: c(COLOR_HOTLIGHT),
    })
}

/// "Show animations in Windows" is on (`SPI_GETCLIENTAREAANIMATION`; on when it cannot be read).
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn animations_enabled() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SPI_GETCLIENTAREAANIMATION, SystemParametersInfoW,
    };
    let mut on: i32 = 1;
    // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL to the pointer.
    let ok =
        unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, (&raw mut on).cast(), 0) };
    ok == 0 || on != 0
}

#[cfg(not(windows))]
pub fn animations_enabled() -> bool {
    true
}

/// Windows' display language is Korean (primary language `LANG_KOREAN`, 0x12).
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn ui_language_is_korean() -> bool {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    // SAFETY: no arguments; returns a LANGID.
    let id = unsafe { GetUserDefaultUILanguage() };
    id & 0x3ff == 0x12
}

#[cfg(not(windows))]
pub fn ui_language_is_korean() -> bool {
    false
}

/// ADR 0099: the primary monitor's work area (without the taskbar) in **logical** pixels: the
/// pixels Windows reports divided by the system DPI scale (a process that is not DPI aware yet
/// already gets 96-DPI values and a system DPI of 96, so both cases agree).
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn work_area_logical() -> Option<(f32, f32)> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETWORKAREA, SystemParametersInfoW};
    let mut r = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: SPI_GETWORKAREA writes one RECT; GetDpiForSystem takes nothing.
    let (ok, dpi) = unsafe {
        (
            SystemParametersInfoW(SPI_GETWORKAREA, 0, (&raw mut r).cast(), 0),
            GetDpiForSystem(),
        )
    };
    let ds = crate::scale::dpi_scale(dpi.max(96));
    (ok != 0 && r.right > r.left && r.bottom > r.top).then(|| {
        (
            (r.right - r.left) as f32 / ds,
            (r.bottom - r.top) as f32 / ds,
        )
    })
}

#[cfg(not(windows))]
pub fn work_area_logical() -> Option<(f32, f32)> {
    None
}

/// ADR 0100: the client area of this process's visible top-level window titled `title`, copied
/// from the screen as RGBA (width, height, pixels) — for a UI audit's photograph of a window
/// egui cannot photograph itself (a child viewport). `None` when there is no such window.
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn grab_window(title: &str) -> Option<(usize, usize, Vec<u8>)> {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, ClientToScreen, CreateCompatibleBitmap,
        CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
        SRCCOPY, SelectObject,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClientRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    };

    struct Find {
        pid: u32,
        title: Vec<u16>,
        hwnd: HWND,
    }
    unsafe extern "system" fn each(h: HWND, l: LPARAM) -> i32 {
        // SAFETY: `l` is the `Find` passed to `EnumWindows` below, alive for the whole call.
        let f = unsafe { &mut *(l as *mut Find) };
        let mut pid = 0u32;
        // SAFETY: plain queries of a window handle Windows just gave us.
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        // SAFETY: a plain query of the same window handle.
        let visible = unsafe { IsWindowVisible(h) } != 0;
        if pid != f.pid || !visible {
            return 1;
        }
        let mut buf = [0u16; 256];
        // SAFETY: the buffer holds 256 UTF-16 units, as passed.
        let n = unsafe { GetWindowTextW(h, buf.as_mut_ptr(), 256) };
        if n > 0 && buf[..n as usize] == f.title[..] {
            f.hwnd = h;
            return 0;
        }
        1
    }
    let mut f = Find {
        // SAFETY: no arguments, no preconditions.
        pid: unsafe { GetCurrentProcessId() },
        title: title.encode_utf16().collect(),
        hwnd: std::ptr::null_mut(),
    };
    // SAFETY: the callback only reads windows and writes `f`, which outlives the call.
    unsafe { EnumWindows(Some(each), &mut f as *mut Find as LPARAM) };
    if f.hwnd.is_null() {
        return None;
    }
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut at = POINT { x: 0, y: 0 };
    // SAFETY: a window of this process found above; both out-pointers are valid.
    unsafe {
        GetClientRect(f.hwnd, &mut rc);
        ClientToScreen(f.hwnd, &mut at);
    }
    let (w, h) = (rc.right - rc.left, rc.bottom - rc.top);
    if w <= 0 || h <= 0 {
        return None;
    }
    let mut px = vec![0u8; (w * h * 4) as usize];
    // SAFETY: the usual GDI copy of a screen rectangle into a 32-bit top-down DIB; every
    // object created here is released before returning.
    let ok = unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bmp);
        let copied = BitBlt(mem, 0, 0, w, h, screen, at.x, at.y, SRCCOPY) != 0;
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let lines = GetDIBits(
            mem,
            bmp,
            0,
            h as u32,
            px.as_mut_ptr().cast(),
            &mut bi,
            DIB_RGB_COLORS,
        );
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        ReleaseDC(std::ptr::null_mut(), screen);
        copied && lines == h
    };
    if !ok {
        return None;
    }
    let rgba: Vec<u8> = px
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|b| [b[2], b[1], b[0], 255])
        .collect();
    Some((w as usize, h as usize, rgba))
}

#[cfg(not(windows))]
pub fn grab_window(_: &str) -> Option<(usize, usize, Vec<u8>)> {
    None
}
