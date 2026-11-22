//! Step 87 (#106): decoding, encoding and atomic saves. `JARAK-FR-020…023`, `JARAK-FR-027`,
//! `JARAK-SEC-001`, `JARAK-SEC-002`.

use super::*;
use crate::document::Eol;

const ANSI: TextEncoding = TextEncoding::Legacy(encoding_rs::WINDOWS_1252);
const KOREAN: TextEncoding = TextEncoding::Legacy(encoding_rs::EUC_KR);

fn utf16(s: &str, le: bool, bom: bool) -> Vec<u8> {
    encode(
        s,
        Format {
            encoding: if le {
                TextEncoding::Utf16Le
            } else {
                TextEncoding::Utf16Be
            },
            bom,
        },
    )
    .unwrap()
}

/// The corpus: every case must decode to the expected text and encode back byte for byte.
#[test]
fn every_format_in_the_corpus_round_trips_byte_for_byte() {
    let euc_kr = encoding_rs::EUC_KR.encode("한글 메모\r\n").0.into_owned();
    let cp1252 = encoding_rs::WINDOWS_1252
        .encode("café — naïve\n")
        .0
        .into_owned();
    let cases: Vec<(&str, Vec<u8>, TextEncoding, &str, bool)> = vec![
        ("empty", vec![], ANSI, "", false),
        (
            "utf-8",
            "plain ascii\n".into(),
            ANSI,
            "plain ascii\n",
            false,
        ),
        ("utf-8 hangul", "한글\r\n".into(), ANSI, "한글\r\n", false),
        (
            "utf-8 bom",
            b"\xEF\xBB\xBFbom\n".to_vec(),
            ANSI,
            "bom\n",
            true,
        ),
        (
            "utf-16le bom",
            utf16("가a\r\n", true, true),
            ANSI,
            "가a\r\n",
            true,
        ),
        (
            "utf-16be bom",
            utf16("가a\n", false, true),
            ANSI,
            "가a\n",
            true,
        ),
        (
            "utf-16le no bom",
            utf16("hello world\r\n", true, false),
            ANSI,
            "hello world\r\n",
            false,
        ),
        ("euc-kr", euc_kr, KOREAN, "한글 메모\r\n", false),
        ("windows-1252", cp1252, ANSI, "café — naïve\n", false),
        (
            "mixed eol, no final newline",
            "a\r\nb\nc\rd".into(),
            ANSI,
            "a\r\nb\nc\rd",
            false,
        ),
        (
            "trailing spaces",
            "x  \t\n  \n".into(),
            ANSI,
            "x  \t\n  \n",
            false,
        ),
    ];
    for (name, bytes, fallback, text, bom) in cases {
        let d = decode(&bytes, fallback);
        assert_eq!(d.text, text, "{name}");
        assert_eq!(d.format.bom, bom, "{name}");
        assert!(d.lossless, "{name} is lossless");
        assert!(!d.read_only(), "{name}");
        assert_eq!(
            encode(&d.text, d.format).unwrap(),
            bytes,
            "{name} round trip"
        );
    }
}

#[test]
fn detection_names_the_encoding() {
    assert_eq!(decode(b"abc", ANSI).format.encoding, TextEncoding::Utf8);
    assert_eq!(
        decode(&utf16("ab\n", true, false), ANSI).format.encoding,
        TextEncoding::Utf16Le
    );
    assert_eq!(
        decode(&utf16("ab\n", false, false), ANSI).format.encoding,
        TextEncoding::Utf16Be
    );
    let latin = encoding_rs::WINDOWS_1252.encode("é").0.into_owned();
    assert_eq!(decode(&latin, ANSI).format.encoding, ANSI);
    assert_eq!(TextEncoding::Utf8.label(true), "UTF-8 BOM");
    assert_eq!(TextEncoding::Utf16Le.label(true), "UTF-16 LE BOM");
    assert_eq!(KOREAN.name(), "EUC-KR");
    assert_eq!(encoding_for_codepage(949), KOREAN);
    assert_eq!(encoding_for_codepage(1252), ANSI);
    assert_eq!(encoding_for_codepage(65001), TextEncoding::Utf8);
    assert_eq!(encoding_for_codepage(12345), TextEncoding::Utf8);
    assert_eq!(encoding_for_label("euc-kr"), Some(KOREAN));
    assert_eq!(encoding_for_label("cp949"), Some(KOREAN));
    assert_eq!(encoding_for_label("UTF-8"), Some(TextEncoding::Utf8));
    assert_eq!(encoding_for_label("utf-16le"), None);
    assert_eq!(encoding_for_label("klingon"), None);
    assert_eq!(encoding_for_label("windows-949"), Some(KOREAN));
    assert_eq!(encoding_for_label("1252"), Some(ANSI));
    assert_eq!(encoding_for_label("cp12345"), None);
}

#[test]
fn lossy_and_binary_files_open_read_only() {
    // Invalid UTF-8 with a UTF-8 fallback: replacement characters would be written back.
    let d = decode(b"ok \xFF\xFE broken", TextEncoding::Utf8);
    assert!(!d.lossless && d.read_only());
    // A byte sequence EUC-KR does not define.
    let d = decode(b"\xC8\xFF", KOREAN);
    assert!(!d.lossless && d.read_only());
    // An unpaired surrogate in UTF-16.
    let d = decode(b"\xFF\xFE\x00\xD8a\x00", ANSI);
    assert!(!d.lossless);
    // An odd number of bytes after a UTF-16 BOM.
    let d = decode(b"\xFF\xFEa\x00b", ANSI);
    assert!(!d.lossless);
    // Zero bytes in a UTF-8 file: binary.
    let d = decode(b"PK\x03\x04\x00\x00data\x00", ANSI);
    assert!(d.binary && d.read_only());
}

#[test]
fn text_an_encoding_cannot_hold_is_refused_not_substituted() {
    let e = encode(
        "abc 한",
        Format {
            encoding: ANSI,
            bom: false,
        },
    )
    .unwrap_err();
    assert_eq!((e.ch, e.at), ('한', 4));
    assert!(e.to_string().contains("U+D55C"), "{e}");
}

#[test]
fn line_ending_statistics() {
    let c = EolCounts::of("a\r\nb\r\nc\nd\re");
    assert_eq!((c.crlf, c.lf, c.cr), (2, 1, 1));
    assert!(c.mixed());
    assert_eq!(c.dominant(), Some(Eol::CrLf));
    assert_eq!(c.name(), "mixed");
    assert_eq!(EolCounts::of("a\nb\n").name(), "lf");
    assert_eq!(EolCounts::of("one line").name(), "none");
    assert_eq!(EolCounts::of("a\rb").dominant(), Some(Eol::Cr));
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("jarak-file-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn entries(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn saves_create_and_replace_and_leave_nothing_behind() {
    let d = temp("save");
    let p = d.join("note.md");
    save_atomic(&p, b"first").unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), b"first");
    save_atomic(&p, "두 번째\r\n".as_bytes()).unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), "두 번째\r\n".as_bytes());
    assert_eq!(entries(&d), ["note.md"], "no temporary file left");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_failing_save_never_touches_the_original() {
    let d = temp("fail");
    let p = d.join("keep.txt");
    std::fs::write(&p, b"original").unwrap();
    for stage in [Stage::Create, Stage::Write, Stage::Flush, Stage::Replace] {
        let e = save_atomic_failing(&p, b"new content", Some(stage)).unwrap_err();
        assert!(e.to_string().contains("injected"), "{stage:?}: {e}");
        assert_eq!(std::fs::read(&p).unwrap(), b"original", "{stage:?}");
        assert_eq!(
            entries(&d),
            ["keep.txt"],
            "{stage:?}: temporary file removed"
        );
    }
    // A new file that fails is not created at all.
    let q = d.join("new.txt");
    assert!(save_atomic_failing(&q, b"x", Some(Stage::Flush)).is_err());
    assert!(!q.exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn read_only_files_are_refused_and_attributes_survive_a_save() {
    let d = temp("attrs");
    let p = d.join("ro.txt");
    std::fs::write(&p, b"locked").unwrap();
    let mut perm = std::fs::metadata(&p).unwrap().permissions();
    perm.set_readonly(true);
    std::fs::set_permissions(&p, perm.clone()).unwrap();
    let e = save_atomic(&p, b"changed").unwrap_err();
    assert_eq!(e.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(std::fs::read(&p).unwrap(), b"locked");
    #[allow(clippy::permissions_set_readonly_false)]
    perm.set_readonly(false);
    std::fs::set_permissions(&p, perm).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const HIDDEN: u32 = 0x2;
        let h = d.join("hidden.txt");
        std::fs::write(&h, b"h").unwrap();
        let ok = std::process::Command::new("attrib")
            .args(["+h", h.to_str().unwrap()])
            .status()
            .unwrap()
            .success();
        assert!(ok);
        save_atomic(&h, b"saved").unwrap();
        assert_eq!(std::fs::read(&h).unwrap(), b"saved");
        let attrs = std::fs::metadata(&h).unwrap().file_attributes();
        assert_ne!(attrs & HIDDEN, 0, "ReplaceFileW keeps the hidden attribute");
    }
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn reading_refuses_folders_and_oversized_files() {
    let d = temp("read");
    assert!(matches!(
        read(&d, ANSI, DEFAULT_SIZE_LIMIT),
        Err(OpenError::Directory)
    ));
    let p = d.join("big.txt");
    std::fs::write(&p, vec![b'a'; 100]).unwrap();
    assert!(matches!(
        read(&p, ANSI, 10),
        Err(OpenError::TooLarge {
            size: 100,
            limit: 10
        })
    ));
    assert_eq!(read(&p, ANSI, 1000).unwrap().text.len(), 100);
    assert!(matches!(
        read(&d.join("missing.txt"), ANSI, 10),
        Err(OpenError::Io(e)) if e.kind() == io::ErrorKind::NotFound
    ));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn file_documents_open_save_save_as_copy_and_reload() {
    let d = temp("filedoc");
    let p = d.join("memo.txt");
    let euc = encoding_rs::EUC_KR.encode("한글\r\n둘째 줄").0.into_owned();
    std::fs::write(&p, &euc).unwrap();
    let mut f = FileDocument::open(&p, KOREAN, DEFAULT_SIZE_LIMIT).unwrap();
    assert_eq!(f.format.encoding, KOREAN);
    assert_eq!(f.doc.eol, Eol::CrLf);
    assert_eq!(f.title(), "memo.txt");
    assert!(!f.read_only && !f.doc.is_dirty());
    // Saving without an edit writes the same bytes.
    f.save().unwrap();
    assert_eq!(std::fs::read(&p).unwrap(), euc);
    // An edit in Korean is saved in the file's own encoding.
    f.doc.doc_end(false);
    f.doc.type_text("!");
    assert!(f.doc.is_dirty());
    f.save().unwrap();
    assert!(!f.doc.is_dirty());
    assert_eq!(
        std::fs::read(&p).unwrap(),
        encoding_rs::EUC_KR
            .encode("한글\r\n둘째 줄!")
            .0
            .into_owned()
    );
    // A character EUC-KR cannot hold is refused; the file keeps its last content.
    f.doc.type_text("🙂");
    assert!(matches!(f.save(), Err(SaveError::Unmappable(_))));
    assert!(f.doc.is_dirty());
    f.doc.undo();
    // Save a Copy keeps the document's file; Save As moves it.
    let copy = d.join("copy.txt");
    f.save_copy(&copy).unwrap();
    assert_eq!(f.path.as_deref(), Some(p.as_path()));
    let moved = d.join("moved.txt");
    f.save_as(&moved).unwrap();
    assert_eq!(f.title(), "moved.txt");
    // Reload reads the file again.
    std::fs::write(&moved, encoding_rs::EUC_KR.encode("바뀜").0).unwrap();
    f.reload(KOREAN, DEFAULT_SIZE_LIMIT).unwrap();
    assert_eq!(f.doc.text().to_string(), "바뀜");
    // A missing file is a new document created on the first save.
    let new = d.join("new.md");
    let mut n = FileDocument::open(&new, KOREAN, DEFAULT_SIZE_LIMIT).unwrap();
    assert!(!new.exists() && n.doc.len_chars() == 0);
    n.doc.type_text("x");
    n.save().unwrap();
    assert_eq!(std::fs::read(&new).unwrap(), b"x");
    // An untitled document needs Save As.
    assert!(matches!(
        FileDocument::untitled().save(),
        Err(SaveError::NoPath)
    ));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn lossy_files_are_read_only_and_say_why() {
    let d = temp("lossy");
    let p = d.join("broken.txt");
    std::fs::write(&p, b"ok \xFF\xFE").unwrap();
    let mut f = FileDocument::open(&p, TextEncoding::Utf8, DEFAULT_SIZE_LIMIT).unwrap();
    assert!(f.read_only);
    assert!(
        f.read_only_reason
            .as_deref()
            .unwrap()
            .contains("cannot reproduce")
    );
    assert!(matches!(f.save(), Err(SaveError::ReadOnly)));
    assert!(matches!(
        f.save_as(&d.join("x.txt")),
        Err(SaveError::ReadOnly)
    ));
    assert_eq!(std::fs::read(&p).unwrap(), b"ok \xFF\xFE");
    let _ = std::fs::remove_dir_all(&d);
}

// ---- Step 103: every character set of the Encoding menu, reinterpreting, lossy conversion ----

/// A sample in each character set's script (what the set can hold).
fn sample(label: &str) -> &'static str {
    match label {
        "euc-kr" => "한글 텍스트, 漢字 ABC 123",
        "shift_jis" | "euc-jp" | "iso-2022-jp" => "日本語のテキスト、カタカナ ABC",
        "gbk" => "中文文本 简体 ABC",
        "gb18030" => "中文 简体 € ABC",
        "big5" => "中文繁體 ABC",
        "windows-1251" | "iso-8859-5" | "ibm866" | "koi8-r" => "Привет, мир! ABC",
        "koi8-u" => "Привіт, світе! ABC",
        "windows-1252" => "Café, déjà vu — €",
        "iso-8859-15" => "Café € œ Ÿ",
        "windows-1250" | "iso-8859-2" => "Žluťoučký kůň úpěl",
        "windows-1253" | "iso-8859-7" => "Καλημέρα κόσμε",
        "windows-1254" => "Türkçe ğüşiöç İ",
        "windows-1255" | "iso-8859-8" => "שלום עולם",
        "windows-1256" | "iso-8859-6" => "مرحبا بالعالم",
        "windows-1257" | "iso-8859-4" | "iso-8859-13" => "Ąčęėįšųūž",
        "windows-1258" => "Viêt Đa ơ ư",
        "windows-874" => "ภาษาไทย ABC",
        _ => panic!("no sample for {label}"),
    }
}

#[test]
fn every_listed_character_set_round_trips_and_is_reinterpreted() {
    for (_, _, name, label) in CHARSETS {
        let enc = encoding_for_label(label).unwrap_or_else(|| panic!("{label}"));
        let text = format!("{}\r\nline 2\r\n", sample(label));
        let fmt = Format {
            encoding: enc,
            bom: false,
        };
        let bytes = encode(&text, fmt).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_ne!(bytes, text.as_bytes(), "{name}: not just UTF-8");
        let d = decode_as(&bytes, fmt);
        assert_eq!(d.text, text, "{name}");
        assert!(d.lossless && !d.binary, "{name}");
        assert_eq!(unmappable(&text, enc, 3).0, 0, "{name}");
    }
    assert_eq!(CHARSETS.len(), 28);
}

#[test]
fn utf8_and_utf16_with_and_without_bom_are_reinterpreted() {
    let text = "자락 ABC\n";
    let utf8 = Format::utf8();
    // As UTF-8: a BOM in the file is taken off and kept.
    let with = [b"\xEF\xBB\xBF".as_slice(), text.as_bytes()].concat();
    let d = decode_as(&with, utf8);
    assert_eq!(
        (d.text.as_str(), d.format.bom, d.lossless),
        (text, true, true)
    );
    // As UTF-8-BOM without one in the file: same text, saving adds it.
    let d = decode_as(text.as_bytes(), Format { bom: true, ..utf8 });
    assert_eq!((d.text.as_str(), d.format.bom), (text, true));
    for (enc, mark) in [
        (TextEncoding::Utf16Le, b"\xFF\xFE".as_slice()),
        (TextEncoding::Utf16Be, b"\xFE\xFF".as_slice()),
    ] {
        let fmt = Format {
            encoding: enc,
            bom: true,
        };
        let bytes = encode(text, fmt).unwrap();
        assert!(bytes.starts_with(mark));
        let d = decode_as(
            &bytes,
            Format {
                encoding: enc,
                bom: false,
            },
        );
        assert_eq!(
            (d.text.as_str(), d.format.bom, d.lossless),
            (text, true, true),
            "{enc:?}"
        );
    }
    // The wrong reading of EUC-KR bytes as Windows-1252 is lossless but garbled; the right one
    // gives the Korean back (what Reinterpret is for).
    let kr = encoding_for_label("euc-kr").unwrap();
    let bytes = encode(
        "한글",
        Format {
            encoding: kr,
            bom: false,
        },
    )
    .unwrap();
    let wrong = decode_as(
        &bytes,
        Format {
            encoding: encoding_for_label("windows-1252").unwrap(),
            bom: false,
        },
    );
    assert_ne!(wrong.text, "한글");
    assert_eq!(
        decode_as(
            &bytes,
            Format {
                encoding: kr,
                bom: false
            }
        )
        .text,
        "한글"
    );
    // Bytes that are not UTF-8 read as UTF-8: replacement characters, not lossless.
    let d = decode_as(&bytes, utf8);
    assert!(!d.lossless && d.text.contains('\u{FFFD}'));
}

#[test]
fn lossy_conversions_are_found_before_saving() {
    let w1252 = encoding_for_label("windows-1252").unwrap();
    let (n, first) = unmappable("abc 한글 é", w1252, 1);
    assert_eq!((n, first), (2, vec![(4, '한')]));
    assert_eq!(
        unmappable("한글", TextEncoding::Utf8, 5).0,
        0,
        "UTF-8 holds everything"
    );
    assert_eq!(
        unmappable("Привет", encoding_for_label("koi8-r").unwrap(), 5).0,
        0
    );
    assert!(
        encode(
            "한글",
            Format {
                encoding: w1252,
                bom: false
            }
        )
        .is_err(),
        "saving refuses it"
    );
}

#[test]
fn reinterpret_reads_the_file_again_in_another_encoding() {
    let dir = std::env::temp_dir().join(format!("jarak-reinterpret-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("a.txt");
    let kr = encoding_for_label("euc-kr").unwrap();
    std::fs::write(
        &p,
        encode(
            "한글\n",
            Format {
                encoding: kr,
                bom: false,
            },
        )
        .unwrap(),
    )
    .unwrap();
    // Opened with a Western fallback: the text is garbled.
    let mut f =
        FileDocument::open(&p, encoding_for_label("windows-1252").unwrap(), 1 << 20).unwrap();
    assert_ne!(f.doc.text().to_string(), "한글\n");
    f.reinterpret(
        Format {
            encoding: kr,
            bom: false,
        },
        1 << 20,
    )
    .unwrap();
    assert_eq!(f.doc.text().to_string(), "한글\n");
    assert_eq!(f.format.encoding, kr);
    assert!(!f.read_only && !f.doc.is_dirty());
    assert_eq!(
        std::fs::read(&p).unwrap(),
        encode(
            "한글\n",
            Format {
                encoding: kr,
                bom: false
            }
        )
        .unwrap(),
        "the file is untouched"
    );
    let mut u = FileDocument::untitled();
    assert!(
        u.reinterpret(Format::utf8(), 1 << 20).is_err(),
        "no file to read"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Step 104 (`JARAK-FR-120`): the document knows when another program changed, deleted or created
/// its file; its own saves do not count as changes.
#[test]
fn the_disk_state_tells_changes_deletions_and_new_files() {
    let d = temp("disk-state");
    let f = d.join("a.txt");
    std::fs::write(&f, "one\r\n").unwrap();
    let mut doc = FileDocument::open(&f, TextEncoding::Utf8, DEFAULT_SIZE_LIMIT).unwrap();
    assert!(doc.disk.is_some());
    assert_eq!(disk_state(&f, doc.disk), DiskState::Same);
    std::fs::write(&f, "changed by another program\r\n").unwrap();
    let DiskState::Changed(now) = disk_state(&f, doc.disk) else {
        panic!("a change is seen");
    };
    assert_eq!(now.len, 28);
    // Its own save is what it last saw.
    doc.doc.paste("x");
    doc.save().unwrap();
    assert_eq!(disk_state(&f, doc.disk), DiskState::Same);
    std::fs::remove_file(&f).unwrap();
    assert_eq!(disk_state(&f, doc.disk), DiskState::Missing);
    // A document for a file that does not exist yet saw none; one appearing is a change.
    let g = d.join("new.txt");
    let fresh = FileDocument::open(&g, TextEncoding::Utf8, DEFAULT_SIZE_LIMIT).unwrap();
    assert_eq!(fresh.disk, None);
    assert_eq!(disk_state(&g, fresh.disk), DiskState::Same);
    std::fs::write(&g, "appeared").unwrap();
    assert!(matches!(disk_state(&g, fresh.disk), DiskState::Changed(_)));
    let _ = std::fs::remove_dir_all(&d);
}

/// Step 104: a reload is one undo step that leaves the document saved; Ctrl+Z brings back the
/// text in memory before it (unsaved); bookmarks outside the change stay; the format follows the
/// file; the user's own read-only choice stays.
#[test]
fn an_undoable_reload_keeps_the_previous_text_one_undo_away() {
    let d = temp("reload-undo");
    let f = d.join("a.txt");
    std::fs::write(&f, "keep\r\nold line\r\ntail\r\n").unwrap();
    let mut doc = FileDocument::open(&f, TextEncoding::Utf8, DEFAULT_SIZE_LIMIT).unwrap();
    doc.doc.bookmark_lines([0, 2]);
    doc.doc
        .set_selection(&crate::selection::Selection::point(0));
    doc.doc.paste("mine ");
    assert!(doc.doc.is_dirty());
    std::fs::write(&f, "\u{feff}keep\r\nnew line from outside\r\ntail\r\n").unwrap();
    assert!(
        doc.reload_undoable(TextEncoding::Utf8, DEFAULT_SIZE_LIMIT)
            .unwrap()
    );
    assert_eq!(
        doc.doc.text().to_string(),
        "keep\r\nnew line from outside\r\ntail\r\n"
    );
    assert!(!doc.doc.is_dirty(), "the reloaded text is the saved one");
    assert!(doc.format.bom, "the format follows the file");
    assert_eq!(disk_state(&f, doc.disk), DiskState::Same);
    assert_eq!(
        doc.doc.bookmarked_lines(),
        [0, 2],
        "lines outside the change keep their bookmarks"
    );
    assert!(doc.doc.undo());
    assert_eq!(
        doc.doc.text().to_string(),
        "mine keep\r\nold line\r\ntail\r\n"
    );
    assert!(doc.doc.is_dirty(), "the text from before is unsaved");
    assert!(doc.doc.redo());
    assert!(!doc.doc.is_dirty());
    // Nothing differs: no undo step.
    assert!(
        !doc.reload_undoable(TextEncoding::Utf8, DEFAULT_SIZE_LIMIT)
            .unwrap()
    );
    // The user's read-only choice stays across a reload.
    doc.read_only = true;
    std::fs::write(&f, "again\r\n").unwrap();
    doc.reload_undoable(TextEncoding::Utf8, DEFAULT_SIZE_LIMIT)
        .unwrap();
    assert!(doc.read_only && doc.read_only_reason.is_none());
    let _ = std::fs::remove_dir_all(&d);
}
