//! Step 107 (HELP-10): what a bug report needs about Jarak and the system — the versions and
//! where things are; never a document's contents.

use std::path::Path;

/// The Windows build (`10.0.19045`): the file version of the system's `kernel32.dll`, read from
/// its version resource (no console, no registry).
pub fn windows_build() -> Option<String> {
    let sys = std::env::var_os("SystemRoot")?;
    let k32 = Path::new(&sys).join("System32").join("kernel32.dll");
    let bytes = std::fs::read(&k32).ok()?;
    // VS_FIXEDFILEINFO starts with the signature 0xFEEF04BD; the file version follows it.
    let sig = 0xFEEF_04BDu32.to_le_bytes();
    let at = bytes.windows(4).position(|w| w == sig)?;
    let word = |o: usize| -> Option<u32> {
        let b = bytes.get(at + o..at + o + 4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let ms = word(8)?;
    let ls = word(12)?;
    Some(format!("{}.{}.{}", ms >> 16, ms & 0xFFFF, ls >> 16))
}

/// The debug information as text: Jarak's version, this program, the Windows build and the
/// settings, shortcuts and session files with whether they exist.
pub fn debug_text(program: &str) -> String {
    let exe = std::env::current_exe().map_or_else(|_| "?".to_string(), |e| e.display().to_string());
    let mut lines = vec![
        format!("Jarak {} ({program})", crate::VERSION),
        format!("exe: {exe}"),
        format!(
            "Windows {} {}",
            windows_build().as_deref().unwrap_or("?"),
            std::env::consts::ARCH
        ),
    ];
    if let Some(p) = crate::paths::Paths::from_env() {
        for (name, f) in [
            ("settings", p.settings_file()),
            ("shortcuts", p.hotkeys_file()),
            ("session", p.session_file()),
        ] {
            lines.push(format!(
                "{name}: {}{}",
                f.display(),
                if f.exists() { "" } else { " (none yet)" }
            ));
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_windows_build_and_the_debug_text_name_the_system() {
        assert!(super::windows_build().is_some_and(|b| b.starts_with("10.")));
        let t = super::debug_text("test");
        assert!(t.starts_with(&format!("Jarak {} (test)", crate::VERSION)));
        assert!(t.contains("\nWindows 10."));
    }
}
