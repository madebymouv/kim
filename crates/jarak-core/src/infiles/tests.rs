//! Find and Replace in Files on a fixture tree (`JARAK-FR-073`, Step 95): filters, recursion,
//! hidden folders, binary and large files skipped, and Replace in Files changing only confirmed
//! matches while keeping every file's encoding, byte-order mark and line endings byte for byte.

use super::*;
use crate::search::{Mode, Query};
use std::sync::atomic::AtomicBool;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("jarak-infiles-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn utf16le_bom(s: &str) -> Vec<u8> {
    let mut v = vec![0xFF, 0xFE];
    for u in s.encode_utf16() {
        v.extend_from_slice(&u.to_le_bytes());
    }
    v
}

fn cp949(s: &str) -> Vec<u8> {
    let (b, _, bad) = encoding_rs::EUC_KR.encode(s);
    assert!(!bad);
    b.into_owned()
}

#[cfg(windows)]
fn hide(p: &Path) {
    use std::os::windows::ffi::OsStrExt;
    let w: Vec<u16> = p.as_os_str().encode_wide().chain([0]).collect();
    #[allow(unsafe_code)]
    // SAFETY: a NUL-terminated wide path; FILE_ATTRIBUTE_HIDDEN | DIRECTORY attributes.
    unsafe {
        windows_sys::Win32::Storage::FileSystem::SetFileAttributesW(w.as_ptr(), 0x2 | 0x10);
    }
}

/// The fixture: text in several encodings and line endings, a binary file, a sub-folder, a hidden
/// folder and an excluded build folder.
fn tree(name: &str) -> PathBuf {
    let d = temp(name);
    std::fs::write(d.join("a.txt"), b"one cat\r\ntwo cats\r\n").unwrap();
    std::fs::write(d.join("b.md"), b"# cat\n\nconcat cat\n").unwrap();
    std::fs::write(d.join("c.txt"), utf16le_bom("UTF-16 cat\r\n")).unwrap();
    std::fs::write(d.join("d.txt"), cp949("한글 cat 고양이\r\n")).unwrap();
    std::fs::write(d.join("e.bin"), b"cat\0\0\x01binary").unwrap();
    std::fs::write(d.join("mixed.txt"), b"cat\nmixed cat\r\nend").unwrap();
    std::fs::create_dir_all(d.join("sub")).unwrap();
    std::fs::write(d.join("sub").join("f.txt"), b"deep cat\n").unwrap();
    std::fs::create_dir_all(d.join("build")).unwrap();
    std::fs::write(d.join("build").join("g.txt"), b"built cat\n").unwrap();
    std::fs::create_dir_all(d.join("sub").join("build")).unwrap();
    std::fs::write(
        d.join("sub").join("build").join("h.txt"),
        b"nested built cat\n",
    )
    .unwrap();
    std::fs::create_dir_all(d.join(".hidden")).unwrap();
    std::fs::write(d.join(".hidden").join("i.txt"), b"hidden cat\n").unwrap();
    #[cfg(windows)]
    hide(&d.join(".hidden"));
    d
}

fn scope(dir: &Path, filters: &str) -> Scope {
    Scope {
        dir: dir.to_path_buf(),
        filters: Filters::parse(filters),
        recursive: true,
        hidden: false,
        fallback: TextEncoding::Legacy(encoding_rs::EUC_KR),
    }
}

fn matcher(text: &str, whole: bool) -> Matcher {
    Query {
        text: text.into(),
        whole_word: whole,
        ..Query::default()
    }
    .compile()
    .unwrap()
}

fn names(f: &Found, root: &Path) -> Vec<String> {
    f.files
        .iter()
        .map(|h| {
            h.path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

#[test]
fn filters_parse_like_notepad_plus_plus() {
    let f = Filters::parse("*.txt *.md !*.min.md !\\build !+\\node_modules");
    assert_eq!(f.include, ["*.txt", "*.md"]);
    assert_eq!(f.exclude, ["*.min.md"]);
    assert_eq!(
        f.exclude_dirs,
        [
            ("build".to_string(), false),
            ("node_modules".to_string(), true)
        ]
    );
    assert!(f.file("A.TXT") && f.file("x.md") && !f.file("x.min.md") && !f.file("x.rs"));
    assert!(f.dir_excluded("build", 1) && !f.dir_excluded("build", 2));
    assert!(f.dir_excluded("node_modules", 3));
    assert!(Filters::parse("").file("anything"));
    assert!(Filters::parse("*.*").file("Makefile"));
    assert!(glob("a?c*", "abcdef") && !glob("a?c", "ac"));
}

#[test]
fn find_in_files_walks_filters_and_skips_binary_and_hidden() {
    let d = tree("find");
    let cancel = AtomicBool::new(false);
    let m = matcher("cat", true);
    let f = find_in_files(&scope(&d, ""), &m, &cancel, &|_, _| {});
    assert_eq!(
        names(&f, &d),
        [
            "a.txt",
            "b.md",
            "c.txt",
            "d.txt",
            "mixed.txt",
            "build/g.txt",
            "sub/f.txt",
            "sub/build/h.txt"
        ],
        "each folder's files, then its sub-folders, in name order; the hidden folder left out"
    );
    assert!(
        f.skipped
            .iter()
            .any(|(p, s)| p.ends_with("e.bin") && *s == Skip::Binary)
    );
    let a = &f.files[0];
    assert_eq!(
        (
            a.hits[0].line,
            a.hits[0].column,
            a.hits[0].line_text.as_str()
        ),
        (1, 5, "one cat")
    );
    assert_eq!(a.hits.len(), 1, "whole word: not cats");
    let k = f.files.iter().find(|h| h.path.ends_with("d.txt")).unwrap();
    assert_eq!(
        (k.hits[0].column, k.hits[0].line_text.as_str()),
        (4, "한글 cat 고양이"),
        "CP949 decoded"
    );
    // Filters: only .txt, not the build folders (top level only with !\, anywhere with !+\).
    let f = find_in_files(&scope(&d, "*.txt !\\build"), &m, &cancel, &|_, _| {});
    assert_eq!(
        names(&f, &d),
        [
            "a.txt",
            "c.txt",
            "d.txt",
            "mixed.txt",
            "sub/f.txt",
            "sub/build/h.txt"
        ]
    );
    let f = find_in_files(&scope(&d, "*.txt !+\\build"), &m, &cancel, &|_, _| {});
    assert_eq!(
        names(&f, &d),
        ["a.txt", "c.txt", "d.txt", "mixed.txt", "sub/f.txt"]
    );
    // Not into sub-folders; into hidden ones.
    let mut s = scope(&d, "*.txt");
    s.recursive = false;
    assert_eq!(
        names(&find_in_files(&s, &m, &cancel, &|_, _| {}), &d),
        ["a.txt", "c.txt", "d.txt", "mixed.txt"]
    );
    let mut s = scope(&d, "i.txt");
    s.hidden = true;
    #[cfg(windows)]
    assert_eq!(
        names(&find_in_files(&s, &m, &cancel, &|_, _| {}), &d),
        [".hidden/i.txt"]
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn replace_in_files_keeps_encodings_line_endings_and_untouched_bytes() {
    let d = tree("replace");
    let before: Vec<(PathBuf, Vec<u8>)> = files(&scope(&d, ""), &AtomicBool::new(false))
        .into_iter()
        .map(|p| {
            let b = std::fs::read(&p).unwrap();
            (p, b)
        })
        .collect();
    let m = matcher("cat", true);
    let (plans, found) = plan(
        &scope(&d, "*.txt *.md !+\\build"),
        &m,
        "dog",
        &AtomicBool::new(false),
    );
    assert_eq!(plans.len(), 6);
    assert!(found.skipped.is_empty());
    let b = plans.iter().find(|p| p.path.ends_with("b.md")).unwrap();
    assert_eq!(b.changes.len(), 2, "# cat and the last cat; not concat");
    assert_eq!(b.changes[1].new_line, "concat dog");
    // Leave one change out (b.md's first) and one whole file (sub/f.txt).
    let journal = d.join("journal");
    let out = apply(
        &plans,
        &|p, k| !(p.ends_with("b.md") && k == 0) && !p.ends_with("f.txt"),
        TextEncoding::Legacy(encoding_rs::EUC_KR),
        &journal,
        "cat",
        "dog",
    )
    .unwrap();
    assert_eq!(out.applied.len(), 5);
    assert!(out.skipped.is_empty());
    let rd = |n: &str| std::fs::read(d.join(n)).unwrap();
    assert_eq!(
        rd("a.txt"),
        b"one dog\r\ntwo cats\r\n",
        "CRLF kept, cats untouched"
    );
    assert_eq!(
        rd("b.md"),
        b"# cat\n\nconcat dog\n",
        "only the confirmed change"
    );
    assert_eq!(
        rd("c.txt"),
        utf16le_bom("UTF-16 dog\r\n"),
        "UTF-16LE with its BOM"
    );
    assert_eq!(rd("d.txt"), cp949("한글 dog 고양이\r\n"), "CP949 kept");
    assert_eq!(
        rd("mixed.txt"),
        b"dog\nmixed dog\r\nend",
        "mixed line endings kept"
    );
    assert_eq!(
        rd("sub/f.txt"),
        b"deep cat\n",
        "a file left out is untouched"
    );
    assert_eq!(rd("build/g.txt"), b"built cat\n", "excluded");
    assert_eq!(rd("e.bin"), b"cat\0\0\x01binary", "binary untouched");
    // The journal: the originals, then undo restores every byte.
    let batch = out.batch.unwrap();
    assert_eq!(batches(&journal)[0].id, batch.id);
    let u = undo(&journal, &batch.id, false).unwrap();
    assert_eq!(u.applied.len(), 5);
    for (p, b) in &before {
        assert_eq!(&std::fs::read(p).unwrap(), b, "{} restored", p.display());
    }
    assert!(batches(&journal)[0].undone);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_file_changed_after_the_preview_or_unencodable_is_skipped_and_undo_leaves_later_edits() {
    let d = temp("safety");
    std::fs::write(d.join("a.txt"), b"cat\n").unwrap();
    std::fs::write(d.join("b.txt"), b"cat\n").unwrap();
    std::fs::write(d.join("k.txt"), cp949("cat 한글\r\n")).unwrap();
    let m = matcher("cat", false);
    let s = scope(&d, "");
    let (plans, _) = plan(&s, &m, "😺", &AtomicBool::new(false));
    assert_eq!(plans.len(), 3);
    std::fs::write(d.join("b.txt"), b"cat edited elsewhere\n").unwrap();
    let journal = d.join("j");
    let out = apply(
        &plans,
        &|_, _| true,
        TextEncoding::Legacy(encoding_rs::EUC_KR),
        &journal,
        "cat",
        "😺",
    )
    .unwrap();
    assert_eq!(out.applied.len(), 1, "only a.txt");
    assert!(
        out.skipped
            .iter()
            .any(|(p, s)| p.ends_with("b.txt") && *s == Skip::Changed)
    );
    assert!(
        out.skipped
            .iter()
            .any(|(p, s)| p.ends_with("k.txt") && matches!(s, Skip::Unmappable(_)))
    );
    assert_eq!(
        std::fs::read(d.join("b.txt")).unwrap(),
        b"cat edited elsewhere\n"
    );
    assert_eq!(
        std::fs::read(d.join("k.txt")).unwrap(),
        cp949("cat 한글\r\n")
    );
    assert_eq!(std::fs::read_to_string(d.join("a.txt")).unwrap(), "😺\n");
    // Edited after the batch: undo leaves it.
    std::fs::write(d.join("a.txt"), "😺 and more\n").unwrap();
    let id = out.batch.unwrap().id;
    let dry = undo(&journal, &id, true).unwrap();
    assert!(dry.applied.is_empty() && dry.skipped.len() == 1);
    let u = undo(&journal, &id, false).unwrap();
    assert!(u.skipped.iter().any(|(_, s)| *s == Skip::Changed));
    assert_eq!(
        std::fs::read_to_string(d.join("a.txt")).unwrap(),
        "😺 and more\n"
    );
    assert!(
        !batches(&journal)[0].undone,
        "not marked undone while a file is left"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn regex_replacements_in_files_and_the_journal_keeps_twenty_batches() {
    let d = temp("regex");
    std::fs::write(d.join("a.txt"), b"x=1\ny=22\n").unwrap();
    let m = Query {
        text: r"(\w)=(\d+)".into(),
        mode: Mode::Regex,
        ..Query::default()
    }
    .compile()
    .unwrap();
    let journal = d.join("j");
    let (plans, _) = plan(&scope(&d, "*.txt"), &m, r"\2:$1", &AtomicBool::new(false));
    assert_eq!(plans[0].changes[1].new_line, "22:y");
    apply(
        &plans,
        &|_, _| true,
        TextEncoding::Legacy(encoding_rs::EUC_KR),
        &journal,
        "",
        "",
    )
    .unwrap();
    assert_eq!(std::fs::read(d.join("a.txt")).unwrap(), b"1:x\n22:y\n");
    // Twenty-two more batches: the oldest go.
    let m = matcher("1", false);
    for i in 0..22 {
        std::fs::write(d.join("a.txt"), b"1\n").unwrap();
        let (plans, _) = plan(
            &scope(&d, "*.txt"),
            &m,
            &i.to_string(),
            &AtomicBool::new(false),
        );
        apply(
            &plans,
            &|_, _| true,
            TextEncoding::Legacy(encoding_rs::EUC_KR),
            &journal,
            "1",
            "",
        )
        .unwrap();
    }
    assert_eq!(batches(&journal).len(), JOURNAL_KEEP);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn cancelling_stops_the_search() {
    let d = tree("cancel");
    let cancel = AtomicBool::new(true);
    let f = find_in_files(&scope(&d, ""), &matcher("cat", false), &cancel, &|_, _| {});
    assert!(f.cancelled);
    assert!(f.files.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}
