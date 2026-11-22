//! Golden tests of the exports (`JARAK-FR-141`, Step 106): the HTML, RTF and published folder of
//! two sample documents (the Markdown one is `note.markdown`: its broken links are deliberate, and
//! the docs check reads only `*.md`) are compared byte for byte with the files in `tests/golden/export/`
//! (`JARAK_WRITE_GOLDEN=1` writes them again after a deliberate change; review the diff). The
//! documents themselves must come out unchanged.

use jarak_core::export::{self, Images};
use jarak_core::syntax;
use std::path::{Path, PathBuf};

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/export")
}

/// Compares `got` with the golden file `name` (line endings ignored), or writes it.
fn golden(name: &str, got: &str) {
    let p = golden_dir().join("expected").join(name);
    if std::env::var_os("JARAK_WRITE_GOLDEN").is_some() {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("{}: {e} (JARAK_WRITE_GOLDEN=1 writes it)", p.display()));
    let norm = |s: &str| s.replace("\r\n", "\n");
    assert!(
        norm(&want) == norm(got),
        "{name} differs from its golden file {}; JARAK_WRITE_GOLDEN=1 writes it again",
        p.display()
    );
}

fn read(name: &str) -> (PathBuf, String) {
    let p = golden_dir().join(name);
    let t = std::fs::read_to_string(&p).unwrap();
    (p, t)
}

#[test]
fn highlighted_code_as_html_and_rtf() {
    let (p, text) = read("sample.rs");
    let rust = syntax::detect("sample.rs", "", &[]);
    assert_eq!(rust.map(|l| l.name.as_str()), Some("Rust"));
    golden(
        "sample.rs.html",
        &export::code_html(&text, rust, "sample.rs", false),
    );
    golden(
        "sample.rs.lines.html",
        &export::code_html(&text, rust, "sample.rs", true),
    );
    golden("sample.rs.rtf", &export::code_rtf(&text, rust, true));
    golden(
        "sample.rs.clipboard.html",
        &export::cf_html(&export::code_fragment(
            &text,
            rust,
            false,
            export::Styling::Inline,
        )),
    );
    assert_eq!(std::fs::read_to_string(&p).unwrap(), text);
}

#[test]
fn rendered_markdown_embedded_linked_and_published() {
    let (doc, text) = read("note.markdown");
    let before = std::fs::read(&doc).unwrap();
    let out = golden_dir().join("expected").join("note.html");
    let embedded = export::markdown_html(&text, Some(&doc), "note.markdown", Images::Embed, &out);
    golden("note.html", &embedded.html);
    let used: Vec<(&str, &str)> = embedded
        .images
        .iter()
        .map(|i| (i.reference.as_str(), i.used.name()))
        .collect();
    assert_eq!(
        used,
        [
            ("assets/dot.png", "embedded"),
            ("https://example.com/r.png", "remote"),
            ("assets/missing.png", "missing"),
            ("../outside.png", "refused"),
        ]
    );
    let linked = export::markdown_html(&text, Some(&doc), "note.markdown", Images::Link, &out);
    golden("note.linked.html", &linked.html);
    assert!(linked.html.contains("src=\"../assets/dot.png\""));

    let site = std::env::temp_dir().join(format!("jarak-golden-site-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&site);
    let p = export::publish(&doc, &text, &site, false, false).unwrap();
    golden(
        "published/note.html",
        &std::fs::read_to_string(&p.page).unwrap(),
    );
    assert_eq!(
        std::fs::read(site.join("assets/dot.png")).unwrap(),
        std::fs::read(golden_dir().join("assets/dot.png")).unwrap()
    );
    let _ = std::fs::remove_dir_all(&site);
    assert_eq!(
        std::fs::read(&doc).unwrap(),
        before,
        "the document is unchanged"
    );
}
