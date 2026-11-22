# Jarak — performance

Status: **current** (Step 109, measured 2026-11-22 on the development PC) · Requirements:
`JARAK-NFR-001`…`JARAK-NFR-006` ([JARAK_REQUIREMENTS](JARAK_REQUIREMENTS.md)) · Measurements:
`crates/jarak-gui/tests/perf.rs` (`cargo test --release -p jarak-gui --test perf -- --ignored
--nocapture --test-threads=1`), `crates/jarak-core/examples/text_bench.rs`, the recorded baseline
`docs/perf/jarak-text-model.json` · History: [JARAK_ARCHITECTURE](JARAK_ARCHITECTURE.md) §10 and the
Step sections

The window measurements are headless: egui's layout and tessellation run on the CPU without
uploading to a GPU, so they measure what Jarak itself does per frame. The start, idle and memory
rows were measured on the real desktop in Step 88.

## Budgets and results

| Budget | Measured | State |
| --- | --- | --- |
| `NFR-001` start to the first painted frame: empty ≤ 400 ms, 1 MB ≤ 600 ms | 247 ms empty, 224 ms 1 MB, 307 ms 10 MB (Step 88, real desktop) | Met |
| `NFR-002` keystroke to frame ≤ 16 ms p95 in 10 MB; the edit ≤ 1 ms | keystroke frame p50 0.22 ms, **p95 2.03 ms**, max 3.95 ms (10 MB, 149 828 lines); highlighted 10 MB Rust p95 0.60 ms; Vim keys p95 6.94 ms; assisted typing p95 1.23 ms; the edit itself p95 3.9 µs, max 79 µs (100 MiB model) | Met |
| `NFR-003` idle: no repaint, ≈ 0 % CPU; at most two wake-ups a second with a blinking caret | 0 ms of CPU over 10 s idle with empty, 1 MB, 10 MB and 100 MB documents (Step 88, real desktop); the caret blink is off by default and repaints at most twice a second | Met (Step 88 measurement; not repeated in Step 109) |
| `NFR-004` 100 MB opens in ≤ 3 s with memory ≤ 3×; scrolling at frame rate; a single 10 MB line does not hang | 100 MB: titled window 577 ms, working set 211.6 MB = 2.1× (Step 88, real desktop). **A single 10 MB line: first measurement 8.7–11.6 s per keystroke frame — it hung; fixed in Step 109** (the visible part of a long line is laid out): open 70 ms; keystroke frame p50 0.22 ms, p95 1.55 ms, max 2.09 ms | Met for 100 MB (Step 88) and the long line (after the fix); **1 GB not measured** (see gaps) |
| `NFR-005` Source ↔ Preview of 1 MB Markdown ≤ 100 ms; split re-render ≤ 50 ms | first switch 58.1 ms, then p50 0.26 ms | Met |
| `NFR-006` `jarak-gui.exe` ≤ 15 MB, `jarak.exe` ≤ 8 MB | `jarak-gui.exe` 13.9 MB, `jarak.exe` 5.4 MB (release; `jarak.exe` grew from 0.5 MB with printing's image decoders in Step 106) | Met (`jarak-gui.exe` is close to its limit) |
| Many tabs | 50 documents of 200 KB: open 80 ms; switching p50 0.60 ms, p95 9.38 ms | Within the frame budget |
| Search | Find All in 10 MB: 149 797 hits of a word in 45 ms, of a regular expression in 54 ms | — |
| Image-heavy Markdown | 50 pictures of 1920×1080: scroll frames p95 3.80 ms; textures peak 126.6 MB (budget 128 MB); typing in split view p95 0.88 ms | Met |
| Structure of a 10 MB Rust file | 249 663 folds, 83 221 symbols, first structure 1.41 s on a worker; keystrokes meanwhile p95 0.59 ms | Within budget (the structure is off the UI thread) |
| Large-file mode (Step 109) | a 10 MB Rust file above the limit: keystroke frame p95 1.68 ms without highlighting or folding work | — |

## Large files

Above *Large file from (MB)* (Settings → Files → Performance, 200 MB by default; `REL-06`,
`SET-18`) a document is shown as plain text: no highlighting, folding, word wrap, Document Map or
brace highlighting, and the status bar says *Large file*. Below the limit everything is back at
once. Files up to 1 GiB open (`file::DEFAULT_SIZE_LIMIT`); larger ones are refused with their size.

## Gaps

- **100 MB and 1 GB benchmarks in Step 109 are not run.** The maintainer's resource rule allows only
  10 MB measurements while they are at the PC. The 100 MB numbers above come from Step 88's real
  desktop run. 1 GB opening, scrolling and searching are **not measured**, to be run when the PC is
  free.
- Idle CPU and memory with Step 105–108 features were not measured again on the real desktop: that
  needs a visible window.
- A 10 MB line is now responsive. A whole document of very long lines with word wrap on still lays
  out each wrapped line in full; large-file mode turns wrap off above its limit.
