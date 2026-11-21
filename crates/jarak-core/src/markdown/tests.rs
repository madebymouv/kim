//! Step 89 (#108): the Markdown render model. `JARAK-FR-040`, `JARAK-SEC-001`.

use super::*;

fn t(s: &str) -> Inline {
    Inline::Text(s.into())
}

#[test]
fn headings_paragraphs_emphasis_and_code() {
    let r = parse("# Title 자락\n\nSome *em* **strong** ~~gone~~ `code`.\n");
    assert_eq!(r.blocks.len(), 2);
    assert_eq!(
        r.blocks[0].block,
        Block::Heading {
            level: 1,
            inlines: vec![t("Title 자락")]
        }
    );
    assert_eq!(
        r.blocks[1].block,
        Block::Paragraph(vec![
            t("Some "),
            Inline::Emph(vec![t("em")]),
            t(" "),
            Inline::Strong(vec![t("strong")]),
            t(" "),
            Inline::Strike(vec![t("gone")]),
            t(" "),
            Inline::Code("code".into()),
            t("."),
        ])
    );
    assert_eq!(
        (r.blocks[0].lines.clone(), r.blocks[1].lines.clone()),
        (0..1, 2..3)
    );
}

#[test]
fn task_lists_nested_lists_and_quotes() {
    let r =
        parse("- [ ] open\n- [x] done\n  - nested\n\n3. third\n4. fourth\n\n> quoted **text**\n");
    let Block::List { start, items } = &r.blocks[0].block else {
        panic!("{:?}", r.blocks[0])
    };
    assert_eq!(*start, None);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].task, Some(false));
    assert_eq!(items[1].task, Some(true));
    assert_eq!(items[0].blocks, vec![Block::Paragraph(vec![t("open")])]);
    assert!(
        matches!(&items[1].blocks[1], Block::List { items, .. } if items.len() == 1),
        "{:?}",
        items[1]
    );
    assert!(
        matches!(&r.blocks[1].block, Block::List { start: Some(3), items } if items.len() == 2)
    );
    assert!(matches!(&r.blocks[2].block, Block::Quote(b) if b.len() == 1));
    assert_eq!(r.blocks[2].lines, 7..8);
}

#[test]
fn code_blocks_tables_rules_and_html() {
    let src = "```rust\nfn main() {}\n```\n\n| a | b |\n|:--|--:|\n| 1 | **2** |\n\n---\n\n<div>raw</div>\n";
    let r = parse(src);
    assert_eq!(
        r.blocks[0].block,
        Block::Code {
            lang: "rust".into(),
            text: "fn main() {}\n".into()
        }
    );
    assert_eq!(r.blocks[0].lines, 0..3);
    let Block::Table {
        align,
        header,
        rows,
    } = &r.blocks[1].block
    else {
        panic!("{:?}", r.blocks[1])
    };
    assert_eq!(align, &vec![Align::Left, Align::Right]);
    assert_eq!(header, &vec![vec![t("a")], vec![t("b")]]);
    assert_eq!(
        rows,
        &vec![vec![vec![t("1")], vec![Inline::Strong(vec![t("2")])]]]
    );
    assert_eq!(r.blocks[2].block, Block::Rule);
    assert_eq!(r.blocks[3].block, Block::Html("<div>raw</div>".into()));
}

#[test]
fn links_images_autolinks_and_footnotes() {
    let r = parse(
        "See [docs](https://x.org \"t\") and ![shot](assets/a.png), https://example.com/a(b). www.kim.dev!\nNote[^1].\n\n[^1]: The note.\n",
    );
    let Block::Paragraph(v) = &r.blocks[0].block else {
        panic!()
    };
    assert!(v.contains(&Inline::Link {
        url: "https://x.org".into(),
        children: vec![t("docs")]
    }));
    assert!(v.contains(&Inline::Image {
        url: "assets/a.png".into(),
        alt: "shot".into()
    }));
    assert!(
        v.contains(&Inline::Link {
            url: "https://example.com/a(b)".into(),
            children: vec![t("https://example.com/a(b)")]
        }),
        "{v:?}"
    );
    assert!(
        v.contains(&Inline::Link {
            url: "http://www.kim.dev".into(),
            children: vec![t("www.kim.dev")]
        }),
        "{v:?}"
    );
    assert!(v.contains(&Inline::FootnoteRef("1".into())));
    assert!(matches!(&r.blocks[1].block, Block::Footnote { label, .. } if label == "1"));
    assert_eq!(r.images(), vec!["assets/a.png".to_string()]);
    // Inside code and existing links nothing is linkified.
    let r = parse("`https://not.a.link` [https://x.org](https://y.org)\n");
    let Block::Paragraph(v) = &r.blocks[0].block else {
        panic!()
    };
    assert_eq!(v[0], Inline::Code("https://not.a.link".into()));
    assert!(matches!(&v[2], Inline::Link { url, .. } if url == "https://y.org"));
}

#[test]
fn source_lines_map_back_to_blocks() {
    let src = "# A\n\npara\none\n\n- x\n- y\n\n## B\n";
    let r = parse(src);
    let lines: Vec<_> = r.blocks.iter().map(|b| b.lines.clone()).collect();
    assert_eq!(lines, vec![0..1, 2..4, 5..7, 8..9]);
    assert_eq!(r.block_for_line(0), Some(0));
    assert_eq!(
        r.block_for_line(1),
        Some(1),
        "a blank line belongs to the next block"
    );
    assert_eq!(r.block_for_line(3), Some(1));
    assert_eq!(r.block_for_line(6), Some(2));
    assert_eq!(r.block_for_line(99), Some(3));
    assert_eq!(parse("").block_for_line(0), None);
}

#[test]
fn crlf_documents_and_plain_text() {
    let r = parse("# T\r\n\r\nbody *x*\r\n");
    assert_eq!(r.blocks.len(), 2);
    assert_eq!(r.blocks[1].lines, 2..3);
    let Block::Paragraph(v) = &r.blocks[1].block else {
        panic!()
    };
    assert_eq!(plain(v), "body x");
}

#[test]
fn a_large_document_parses_fast() {
    let unit = "## Section\n\nSome text with *emphasis*, `code` and a [link](https://x.org).\n\n- item one\n- item two\n\n```\ncode block\n```\n\n";
    let text = unit.repeat(1024 * 1024 / unit.len());
    let t0 = std::time::Instant::now();
    let r = parse(&text);
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    assert!(r.blocks.len() > 30_000);
    // Debug builds are much slower; the release budget is in JARAK_ARCHITECTURE.
    assert!(ms < 5000.0, "{ms} ms");
}
