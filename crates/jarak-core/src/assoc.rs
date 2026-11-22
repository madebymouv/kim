//! Step 111 (SET-09, `JARAK-FR-130`, `KIM-SEC-001`): offering Jarak in Windows' *Open with* list —
//! explicit and reversible.
//!
//! - Only on the user's request (`jarak associate add`, Help → *Offer Jarak in "Open with"…*).
//! - Per user (`HKEY_CURRENT_USER\Software\Classes`), never machine-wide, no elevation.
//! - It **never changes a default program**: Jarak registers one ProgID (`mouv.Jarak.Document`) and
//!   adds it to each chosen extension's `OpenWithProgids`; Windows lets the user pick the default
//!   (Settings → Apps → Default apps).
//! - Reversible: what was added (the extensions, the keys Jarak created) is recorded in
//!   `associations.json`; `remove` deletes exactly that — Jarak's own value under each extension,
//!   its ProgID, and keys it created only while they are still empty. Other programs' values stay.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

/// Jarak's ProgID.
pub const PROG_ID: &str = "mouv.Jarak.Document";

/// The extensions offered by default (plain text, Markdown, logs, settings, data and code).
pub const DEFAULT_EXTS: &[&str] = &[
    "txt", "md", "markdown", "log", "ini", "cfg", "conf", "json", "xml", "yaml", "yml", "toml",
    "csv",
];

/// Extensions Jarak never offers itself for (programs and scripts Windows runs).
const NEVER: &[&str] = &[
    "exe", "com", "bat", "cmd", "ps1", "vbs", "vbe", "js", "jse", "wsf", "wsh", "msi", "msc",
    "lnk", "scr", "cpl", "dll", "sys", "pif", "reg", "hta", "url",
];

/// What a store of registry keys under one base does (the real per-user classes, a private test
/// key, or memory).
pub trait Store {
    /// The default (`name` empty) or named string value; `None` writes an empty `REG_NONE`
    /// marker (what `OpenWithProgids` entries are).
    fn set(&mut self, key: &str, name: &str, value: Option<&str>) -> io::Result<()>;
    /// Whether the value exists, and its text when it is a string.
    fn get(&self, key: &str, name: &str) -> io::Result<Option<Option<String>>>;
    fn delete_value(&mut self, key: &str, name: &str) -> io::Result<bool>;
    /// Deletes `key` and everything under it.
    fn delete_tree(&mut self, key: &str) -> io::Result<()>;
    fn key_exists(&self, key: &str) -> io::Result<bool>;
    /// A key with no values and no subkeys.
    fn key_empty(&self, key: &str) -> io::Result<bool>;
    /// Deletes an empty key.
    fn delete_key(&mut self, key: &str) -> io::Result<()>;
    /// Where the keys are, for messages (`HKCU\Software\Classes`).
    fn location(&self) -> String;
    /// Tells the shell the associations changed (the real store only).
    fn notify(&self) {}
}

/// What was offered (`associations.json`, schema `jarak-associations-v1`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    /// The program the ProgID opens.
    pub program: String,
    /// The extensions offered (without the dot, lower case).
    pub extensions: Vec<String>,
    /// Keys Jarak created (relative to the base), deleted on removal only while still empty.
    #[serde(default)]
    pub created: Vec<String>,
}

impl Record {
    pub fn read(path: &Path) -> Option<Record> {
        let t = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&t).ok()
    }
    pub fn write(&self, path: &Path) -> io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let mut s = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        s.push('\n');
        crate::file::save_atomic(path, s.as_bytes())
    }
}

/// An extension as given (`.md`, `MD`, `md`) → `md`; `None` for one Jarak never takes or that is
/// not a plain extension.
pub fn normalize(ext: &str) -> Option<String> {
    let e = ext.trim().trim_start_matches('.').to_ascii_lowercase();
    let ok = !e.is_empty()
        && e.len() <= 16
        && e.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    (ok && !NEVER.contains(&e.as_str())).then_some(e)
}

/// One change of an offer or a removal (for `--dry-run` and the report).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    /// `set`, `delete-value`, `delete-key`.
    pub action: &'static str,
    pub key: String,
    pub name: String,
    pub value: Option<String>,
}

fn prog_keys(program: &Path) -> Vec<(String, String, Option<String>)> {
    let p = program.display().to_string();
    vec![
        (
            PROG_ID.to_string(),
            String::new(),
            Some("Jarak document".into()),
        ),
        (
            format!("{PROG_ID}\\DefaultIcon"),
            String::new(),
            Some(format!("\"{p}\",0")),
        ),
        (
            format!("{PROG_ID}\\shell\\open\\command"),
            String::new(),
            Some(format!("\"{p}\" \"%1\"")),
        ),
    ]
}

/// Offers Jarak for `exts` (merged with an earlier offer); with `dry_run` only says what it would
/// do. Returns the changes and the new record.
pub fn offer(
    store: &mut dyn Store,
    record: Option<&Record>,
    program: &Path,
    exts: &[String],
    dry_run: bool,
) -> io::Result<(Vec<Change>, Record)> {
    let mut rec = record.cloned().unwrap_or_else(|| Record {
        schema: "jarak-associations-v1".into(),
        ..Record::default()
    });
    rec.program = program.display().to_string();
    let mut changes = Vec::new();
    let mut created = Vec::new();
    for (k, n, v) in prog_keys(program) {
        if !store.key_exists(&k)? {
            created.push(k.clone());
        }
        changes.push(Change {
            action: "set",
            key: k,
            name: n,
            value: v,
        });
    }
    for e in exts {
        let ext_key = format!(".{e}");
        let owp = format!(".{e}\\OpenWithProgids");
        for k in [&ext_key, &owp] {
            if !store.key_exists(k)? && !rec.created.contains(k) && !created.contains(k) {
                created.push(k.clone());
            }
        }
        if store.get(&owp, PROG_ID)?.is_none() {
            changes.push(Change {
                action: "set",
                key: owp,
                name: PROG_ID.into(),
                value: None,
            });
        }
        if !rec.extensions.contains(e) {
            rec.extensions.push(e.clone());
        }
    }
    rec.extensions.sort();
    for c in created {
        if !rec.created.contains(&c) {
            rec.created.push(c);
        }
    }
    if !dry_run {
        for c in &changes {
            store.set(&c.key, &c.name, c.value.as_deref())?;
        }
        store.notify();
    }
    Ok((changes, rec))
}

/// Removes what [`offer`] added (the record says what); with `dry_run` only says what it would
/// do.
pub fn remove(store: &mut dyn Store, record: &Record, dry_run: bool) -> io::Result<Vec<Change>> {
    let mut changes = Vec::new();
    for e in &record.extensions {
        let owp = format!(".{e}\\OpenWithProgids");
        if store.get(&owp, PROG_ID)?.is_some() {
            changes.push(Change {
                action: "delete-value",
                key: owp,
                name: PROG_ID.into(),
                value: None,
            });
        }
    }
    if store.key_exists(PROG_ID)? {
        changes.push(Change {
            action: "delete-key",
            key: PROG_ID.into(),
            name: String::new(),
            value: None,
        });
    }
    if dry_run {
        return Ok(changes);
    }
    for c in &changes {
        match c.action {
            "delete-value" => {
                store.delete_value(&c.key, &c.name)?;
            }
            _ => store.delete_tree(&c.key)?,
        }
    }
    // Keys Jarak created, deepest first, only while nothing else was put in them.
    let mut created: Vec<&String> = record
        .created
        .iter()
        .filter(|k| !k.starts_with(PROG_ID))
        .collect();
    created.sort_by_key(|k| std::cmp::Reverse(k.matches('\\').count()));
    for k in created {
        if store.key_exists(k)? && store.key_empty(k)? {
            store.delete_key(k)?;
            changes.push(Change {
                action: "delete-key",
                key: k.clone(),
                name: String::new(),
                value: None,
            });
        }
    }
    store.notify();
    Ok(changes)
}

/// Which of `record`'s extensions still carry Jarak in the store.
pub fn offered(store: &dyn Store, record: &Record) -> io::Result<Vec<String>> {
    let mut v = Vec::new();
    for e in &record.extensions {
        if store
            .get(&format!(".{e}\\OpenWithProgids"), PROG_ID)?
            .is_some()
        {
            v.push(e.clone());
        }
    }
    Ok(v)
}

/// Keys in memory (tests).
#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    /// key → (value name → text, `None` for REG_NONE).
    pub keys: BTreeMap<String, BTreeMap<String, Option<String>>>,
}

impl MemoryStore {
    fn ensure(&mut self, key: &str) {
        let mut path = String::new();
        for part in key.split('\\') {
            if !path.is_empty() {
                path.push('\\');
            }
            path.push_str(part);
            self.keys.entry(path.clone()).or_default();
        }
    }
}

impl Store for MemoryStore {
    fn set(&mut self, key: &str, name: &str, value: Option<&str>) -> io::Result<()> {
        self.ensure(key);
        self.keys
            .get_mut(key)
            .expect("made")
            .insert(name.to_string(), value.map(str::to_string));
        Ok(())
    }
    fn get(&self, key: &str, name: &str) -> io::Result<Option<Option<String>>> {
        Ok(self.keys.get(key).and_then(|k| k.get(name).cloned()))
    }
    fn delete_value(&mut self, key: &str, name: &str) -> io::Result<bool> {
        Ok(self
            .keys
            .get_mut(key)
            .is_some_and(|k| k.remove(name).is_some()))
    }
    fn delete_tree(&mut self, key: &str) -> io::Result<()> {
        let sub = format!("{key}\\");
        self.keys.retain(|k, _| k != key && !k.starts_with(&sub));
        Ok(())
    }
    fn key_exists(&self, key: &str) -> io::Result<bool> {
        Ok(self.keys.contains_key(key))
    }
    fn key_empty(&self, key: &str) -> io::Result<bool> {
        let sub = format!("{key}\\");
        Ok(self.keys.get(key).is_some_and(BTreeMap::is_empty)
            && !self.keys.keys().any(|k| k.starts_with(&sub)))
    }
    fn delete_key(&mut self, key: &str) -> io::Result<()> {
        self.keys.remove(key);
        Ok(())
    }
    fn location(&self) -> String {
        "memory".into()
    }
}

/// A [`MemoryStore`] shared with the test that looks at it.
#[derive(Debug, Default, Clone)]
pub struct SharedMemory(pub std::sync::Arc<std::sync::Mutex<MemoryStore>>);

impl SharedMemory {
    fn with<R>(&self, f: impl FnOnce(&mut MemoryStore) -> R) -> R {
        let mut g = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut g)
    }
}

impl Store for SharedMemory {
    fn set(&mut self, key: &str, name: &str, value: Option<&str>) -> io::Result<()> {
        self.with(|m| m.set(key, name, value))
    }
    fn get(&self, key: &str, name: &str) -> io::Result<Option<Option<String>>> {
        self.with(|m| m.get(key, name))
    }
    fn delete_value(&mut self, key: &str, name: &str) -> io::Result<bool> {
        self.with(|m| m.delete_value(key, name))
    }
    fn delete_tree(&mut self, key: &str) -> io::Result<()> {
        self.with(|m| m.delete_tree(key))
    }
    fn key_exists(&self, key: &str) -> io::Result<bool> {
        self.with(|m| m.key_exists(key))
    }
    fn key_empty(&self, key: &str) -> io::Result<bool> {
        self.with(|m| m.key_empty(key))
    }
    fn delete_key(&mut self, key: &str) -> io::Result<()> {
        self.with(|m| m.delete_key(key))
    }
    fn location(&self) -> String {
        "memory".into()
    }
}

/// The record's file beside Jarak's other local state.
pub fn record_path(local: &Path) -> PathBuf {
    local.join("associations.json")
}

#[cfg(windows)]
pub use win::{Registry, TEST_PREFIX};

#[cfg(windows)]
#[allow(unsafe_code)]
mod win {
    use super::Store;
    use std::io;
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE, REG_NONE,
        REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_ANY, RegCloseKey, RegCreateKeyExW, RegDeleteKeyW,
        RegDeleteTreeW, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegQueryInfoKeyW,
        RegSetValueExW,
    };

    /// Prefix every test base must have.
    pub const TEST_PREFIX: &str = "Software\\mouv-test\\";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn err(code: u32) -> io::Error {
        io::Error::from_raw_os_error(i32::try_from(code).unwrap_or(-1))
    }

    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: a key handle opened below, closed once.
            unsafe { RegCloseKey(self.0) };
        }
    }

    /// Keys under `HKEY_CURRENT_USER\<base>`.
    #[derive(Debug, Clone)]
    pub struct Registry {
        base: String,
    }

    impl Registry {
        /// The user's own file classes (`HKCU\Software\Classes`).
        pub fn user_classes() -> Registry {
            Registry {
                base: "Software\\Classes".into(),
            }
        }

        /// A private test base (must start with [`TEST_PREFIX`]).
        pub fn test_base(base: &str) -> Option<Registry> {
            base.starts_with(TEST_PREFIX).then(|| Registry {
                base: base.to_string(),
            })
        }

        /// The store the CLI and the window use: the user's classes, or in debug builds a test
        /// base from `JARAK_DEV_CLASSES_KEY` (compiled out of release builds).
        pub fn for_program() -> Registry {
            #[cfg(debug_assertions)]
            if let Some(r) = std::env::var("JARAK_DEV_CLASSES_KEY")
                .ok()
                .and_then(|k| Registry::test_base(&k))
            {
                return r;
            }
            Registry::user_classes()
        }

        /// Deletes the whole test base (tests only; refuses anything else).
        pub fn delete_test_base(&self) -> io::Result<()> {
            if !self.base.starts_with(TEST_PREFIX) {
                return Err(io::Error::other(
                    "refusing to delete a key outside the test prefix",
                ));
            }
            let path = wide(&self.base);
            // SAFETY: NUL-terminated path below HKCU\Software\mouv-test.
            match unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, path.as_ptr()) } {
                ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
                e => Err(err(e)),
            }
        }

        fn full(&self, key: &str) -> String {
            if key.is_empty() {
                self.base.clone()
            } else {
                format!("{}\\{key}", self.base)
            }
        }

        fn open(&self, key: &str, access: u32, create: bool) -> io::Result<Option<Key>> {
            let path = wide(&self.full(key));
            let mut h: HKEY = std::ptr::null_mut();
            // SAFETY: NUL-terminated path; `h` receives a key handle on success.
            let rc = unsafe {
                if create {
                    RegCreateKeyExW(
                        HKEY_CURRENT_USER,
                        path.as_ptr(),
                        0,
                        std::ptr::null(),
                        REG_OPTION_NON_VOLATILE,
                        access,
                        std::ptr::null(),
                        &raw mut h,
                        std::ptr::null_mut(),
                    )
                } else {
                    RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, access, &raw mut h)
                }
            };
            match rc {
                ERROR_SUCCESS => Ok(Some(Key(h))),
                ERROR_FILE_NOT_FOUND => Ok(None),
                e => Err(err(e)),
            }
        }
    }

    impl Store for Registry {
        fn set(&mut self, key: &str, name: &str, value: Option<&str>) -> io::Result<()> {
            let k = self
                .open(key, KEY_SET_VALUE, true)?
                .ok_or_else(|| io::Error::other("the key could not be created"))?;
            let n = wide(name);
            let (kind, data): (u32, Vec<u16>) = match value {
                Some(v) => (REG_SZ, wide(v)),
                None => (REG_NONE, Vec::new()),
            };
            // SAFETY: valid key; `data` holds `data.len() * 2` bytes (none for REG_NONE).
            let rc = unsafe {
                RegSetValueExW(
                    k.0,
                    n.as_ptr(),
                    0,
                    kind,
                    if data.is_empty() {
                        std::ptr::null()
                    } else {
                        data.as_ptr().cast()
                    },
                    u32::try_from(data.len() * 2).expect("small"),
                )
            };
            if rc == ERROR_SUCCESS {
                Ok(())
            } else {
                Err(err(rc))
            }
        }

        fn get(&self, key: &str, name: &str) -> io::Result<Option<Option<String>>> {
            let Some(k) = self.open(key, KEY_QUERY_VALUE, false)? else {
                return Ok(None);
            };
            let n = wide(name);
            let mut kind = 0u32;
            let mut buf = vec![0u16; 2048];
            let mut bytes = u32::try_from(buf.len() * 2).expect("small");
            // SAFETY: valid key; `buf` has `bytes` writable bytes.
            let rc = unsafe {
                RegGetValueW(
                    k.0,
                    std::ptr::null(),
                    n.as_ptr(),
                    RRF_RT_ANY,
                    &raw mut kind,
                    buf.as_mut_ptr().cast(),
                    &raw mut bytes,
                )
            };
            match rc {
                ERROR_SUCCESS if kind == REG_SZ => {
                    let len = (bytes as usize / 2).min(buf.len());
                    buf.truncate(len);
                    while buf.last() == Some(&0) {
                        buf.pop();
                    }
                    Ok(Some(Some(String::from_utf16_lossy(&buf))))
                }
                ERROR_SUCCESS => Ok(Some(None)),
                ERROR_FILE_NOT_FOUND => Ok(None),
                e => Err(err(e)),
            }
        }

        fn delete_value(&mut self, key: &str, name: &str) -> io::Result<bool> {
            let Some(k) = self.open(key, KEY_SET_VALUE, false)? else {
                return Ok(false);
            };
            let n = wide(name);
            // SAFETY: valid key; NUL-terminated value name.
            match unsafe { RegDeleteValueW(k.0, n.as_ptr()) } {
                ERROR_SUCCESS => Ok(true),
                ERROR_FILE_NOT_FOUND => Ok(false),
                e => Err(err(e)),
            }
        }

        fn delete_tree(&mut self, key: &str) -> io::Result<()> {
            let path = wide(&self.full(key));
            // SAFETY: NUL-terminated path under the base.
            match unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, path.as_ptr()) } {
                ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
                e => Err(err(e)),
            }
        }

        fn key_exists(&self, key: &str) -> io::Result<bool> {
            Ok(self.open(key, KEY_QUERY_VALUE, false)?.is_some())
        }

        fn key_empty(&self, key: &str) -> io::Result<bool> {
            let Some(k) = self.open(key, KEY_READ, false)? else {
                return Ok(false);
            };
            let (mut subkeys, mut values) = (0u32, 0u32);
            // SAFETY: valid key; only the two counts are asked for.
            let rc = unsafe {
                RegQueryInfoKeyW(
                    k.0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    &raw mut subkeys,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &raw mut values,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if rc == ERROR_SUCCESS {
                Ok(subkeys == 0 && values == 0)
            } else {
                Err(err(rc))
            }
        }

        fn delete_key(&mut self, key: &str) -> io::Result<()> {
            let path = wide(&self.full(key));
            // SAFETY: NUL-terminated path under the base (an empty key, checked by the caller).
            match unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, path.as_ptr()) } {
                ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
                e => Err(err(e)),
            }
        }

        fn location(&self) -> String {
            format!("HKCU\\{}", self.base)
        }

        fn notify(&self) {
            if self.base != "Software\\Classes" {
                return;
            }
            // SAFETY: SHCNE_ASSOCCHANGED with no items, as documented.
            unsafe {
                windows_sys::Win32::UI::Shell::SHChangeNotify(
                    windows_sys::Win32::UI::Shell::SHCNE_ASSOCCHANGED as i32,
                    windows_sys::Win32::UI::Shell::SHCNF_IDLIST,
                    std::ptr::null(),
                    std::ptr::null(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exts(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn extensions_are_normalized_and_programs_never_offered() {
        assert_eq!(normalize(".MD").as_deref(), Some("md"));
        assert_eq!(normalize(" txt ").as_deref(), Some("txt"));
        for bad in [
            ".exe",
            "BAT",
            "ps1",
            "lnk",
            "",
            ".",
            "a b",
            "a\\b",
            "x".repeat(17).as_str(),
        ] {
            assert_eq!(normalize(bad), None, "{bad}");
        }
    }

    #[test]
    fn an_offer_adds_jarak_to_open_with_and_never_a_default() {
        let mut st = MemoryStore::default();
        // Another program's own value and a default (the user's choice) are already there.
        st.set(".md", "", Some("Other.md")).unwrap();
        st.set(".md\\OpenWithProgids", "Other.md", None).unwrap();
        let prog = Path::new(r"C:\Programs\jarak\jarak-gui.exe");
        let (dry, rec) = offer(&mut st, None, prog, &exts(&["md", "txt"]), true).unwrap();
        assert!(dry.iter().any(|c| c.key == ".txt\\OpenWithProgids"));
        assert!(!st.key_exists(PROG_ID).unwrap(), "a dry run writes nothing");
        assert_eq!(rec.extensions, ["md", "txt"]);
        let (_, rec) = offer(&mut st, None, prog, &exts(&["md", "txt"]), false).unwrap();
        assert_eq!(
            st.get(".md", "").unwrap(),
            Some(Some("Other.md".into())),
            "the default program is untouched"
        );
        assert_eq!(st.get(".md\\OpenWithProgids", PROG_ID).unwrap(), Some(None));
        assert_eq!(
            st.get(&format!("{PROG_ID}\\shell\\open\\command"), "")
                .unwrap(),
            Some(Some(r#""C:\Programs\jarak\jarak-gui.exe" "%1""#.into()))
        );
        assert_eq!(offered(&st, &rec).unwrap(), ["md", "txt"]);
        assert!(rec.created.contains(&".txt".to_string()));
        assert!(
            !rec.created.contains(&".md".to_string()),
            "it was there before"
        );
        // Offering again changes nothing new.
        let (again, rec2) = offer(&mut st, Some(&rec), prog, &exts(&["md"]), false).unwrap();
        assert!(again.iter().all(|c| c.key.starts_with(PROG_ID)));
        assert_eq!(rec2.extensions, ["md", "txt"]);
    }

    #[test]
    fn removing_takes_back_exactly_what_was_added() {
        let mut st = MemoryStore::default();
        st.set(".md\\OpenWithProgids", "Other.md", None).unwrap();
        let before = st.keys.clone();
        let prog = Path::new(r"C:\x\jarak-gui.exe");
        let (_, rec) = offer(&mut st, None, prog, &exts(&["md", "txt", "log"]), false).unwrap();
        // Another program writes into a key Jarak created, after the offer.
        st.set(".log", "", Some("Other.log")).unwrap();
        let dry = remove(&mut st, &rec, true).unwrap();
        assert_eq!(dry.len(), 4, "three values and the ProgID: {dry:?}");
        assert!(st.key_exists(PROG_ID).unwrap(), "a dry run removes nothing");
        remove(&mut st, &rec, false).unwrap();
        assert!(!st.key_exists(PROG_ID).unwrap());
        assert_eq!(
            st.get(".md\\OpenWithProgids", "Other.md").unwrap(),
            Some(None)
        );
        assert!(!st.key_exists(".txt").unwrap(), "Jarak's own empty keys go");
        assert_eq!(
            st.get(".log", "").unwrap(),
            Some(Some("Other.log".into())),
            "a key another program uses now stays"
        );
        st.delete_tree(".log").unwrap();
        assert_eq!(st.keys, before, "everything else as it was");
    }

    #[test]
    fn the_record_round_trips() {
        let d = std::env::temp_dir().join(format!("jarak-assoc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let p = record_path(&d);
        assert_eq!(Record::read(&p), None);
        let r = Record {
            schema: "jarak-associations-v1".into(),
            program: r"C:\x\jarak-gui.exe".into(),
            extensions: exts(&["md"]),
            created: exts(&[".md"]),
        };
        r.write(&p).unwrap();
        assert_eq!(Record::read(&p), Some(r));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The real registry, under a private test key that is deleted afterwards.
    #[cfg(windows)]
    #[test]
    fn the_real_registry_offer_and_removal_under_a_test_key() {
        let base = format!("{}jarak-unit-{}\\Classes", TEST_PREFIX, std::process::id());
        assert!(Registry::test_base("Software\\Classes").is_none());
        let mut st = Registry::test_base(&base).unwrap();
        st.set(".md\\OpenWithProgids", "Other.md", None).unwrap();
        let prog = Path::new(r"C:\x\jarak-gui.exe");
        let (_, rec) = offer(&mut st, None, prog, &exts(&["md", "txt"]), false).unwrap();
        assert_eq!(offered(&st, &rec).unwrap(), ["md", "txt"]);
        assert_eq!(
            st.get(&format!("{PROG_ID}\\DefaultIcon"), "").unwrap(),
            Some(Some(r#""C:\x\jarak-gui.exe",0"#.into()))
        );
        remove(&mut st, &rec, false).unwrap();
        assert!(!st.key_exists(PROG_ID).unwrap());
        assert!(!st.key_exists(".txt").unwrap());
        assert_eq!(
            st.get(".md\\OpenWithProgids", "Other.md").unwrap(),
            Some(None)
        );
        let top = Registry::test_base(&format!("{}jarak-unit-{}", TEST_PREFIX, std::process::id()))
            .unwrap();
        top.delete_test_base().unwrap();
        assert!(!st.key_exists("").unwrap());
        assert!(Registry::user_classes().delete_test_base().is_err());
    }
}
