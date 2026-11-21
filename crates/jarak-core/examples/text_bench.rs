//! Step 86 (#105): edit latency of the text model on a 100 MB document (`JARAK-NFR-002`: the
//! edit itself ≤ 1 ms). Run with `cargo run --release -p jarak-core --example text_bench`; it
//! prints one line per measurement, and `-- --record <file>` also writes them as JSON (the
//! record in `docs/perf/jarak-text-model.json`). Nothing else is written.

use jarak_core::document::Document;
use jarak_core::selection::{Range, Selection};
use std::time::Instant;

fn main() {
    let record = std::env::args().skip_while(|a| a != "--record").nth(1);
    let line = "The quick brown fox 한글 문장 jumps over the lazy dog 0123456789\r\n";
    let target = 100 * 1024 * 1024;
    let text = line.repeat(target / line.len() + 1);
    let t = Instant::now();
    let mut d = Document::from_text(&text);
    drop(text);
    let load_ms = t.elapsed().as_secs_f64() * 1000.0;
    println!(
        "load: {} MB, {} lines, {} chars in {:.0} ms",
        target / (1024 * 1024),
        d.text().len_lines(),
        d.len_chars(),
        load_ms
    );
    let len = d.len_chars();
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let mut next = |n: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n as u64) as usize
    };
    let mut times = Vec::with_capacity(4000);
    for i in 0..4000 {
        let pos = next(len);
        let t = Instant::now();
        d.set_selection(&Selection::single(Range::point(pos)));
        match i % 4 {
            0 => d.type_text("a"),
            1 => d.type_text("한"),
            2 => d.delete_backward(),
            _ => d.newline(),
        }
        times.push(t.elapsed().as_secs_f64() * 1e6);
    }
    times.sort_by(f64::total_cmp);
    let p = |q: f64| times[((times.len() as f64 - 1.0) * q) as usize];
    println!(
        "edit at a random place (set caret + type/delete/newline): p50 {:.1} µs, p95 {:.1} µs, p99 {:.1} µs, max {:.1} µs",
        p(0.5),
        p(0.95),
        p(0.99),
        p(1.0)
    );
    let t = Instant::now();
    let mut n = 0;
    while d.undo() {
        n += 1;
    }
    let undo_us = t.elapsed().as_secs_f64() * 1e6 / n.max(1) as f64;
    println!("undo of {n} steps: {undo_us:.1} µs each");
    let mut lookups = Vec::with_capacity(10000);
    let lines = d.text().len_lines();
    for _ in 0..10000 {
        let l = next(lines);
        let t = Instant::now();
        let c = d.text().line_to_char(l);
        std::hint::black_box(d.line_col(c));
        lookups.push(t.elapsed().as_secs_f64() * 1e6);
    }
    lookups.sort_by(f64::total_cmp);
    println!(
        "line lookup + line/column: p50 {:.2} µs, max {:.1} µs",
        lookups[lookups.len() / 2],
        lookups[lookups.len() - 1]
    );
    if let Some(path) = record {
        let json = format!(
            "{{
  \"budgets\": [
    {{ \"limit\": 1000.0, \"path\": \"/edit_us/max\", \"requirement\": \"JARAK-NFR-002 (the edit itself <= 1 ms)\" }}
  ],
  \"document_mib\": 100,
  \"lines\": {},
  \"load_ms\": {:.0},
  \"edit_us\": {{ \"p50\": {:.1}, \"p95\": {:.1}, \"p99\": {:.1}, \"max\": {:.1} }},
  \"undo_us_per_step\": {:.1},
  \"line_lookup_us\": {{ \"p50\": {:.2}, \"max\": {:.1} }},
  \"command\": \"cargo run --release -p jarak-core --example text_bench -- --record docs/perf/jarak-text-model.json\"
}}
",
            d.text().len_lines(),
            load_ms,
            p(0.5),
            p(0.95),
            p(0.99),
            p(1.0),
            undo_us,
            lookups[lookups.len() / 2],
            lookups[lookups.len() - 1]
        );
        std::fs::write(&path, json).expect("write the record");
        println!("recorded {path}");
    }
}
