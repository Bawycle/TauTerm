// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use parking_lot::RwLock;
use std::hint::black_box;
use tau_term_lib::vt::VtProcessor;

/// Measures raw VT parser throughput on 1 MB of printable ASCII.
///
/// The `VtProcessor` is created in the setup phase (not counted) so only
/// `process()` latency is measured.  `black_box` prevents the compiler from
/// discarding the output.
fn bench_write_char_throughput(c: &mut Criterion) {
    let data: Vec<u8> = (0u8..128)
        .cycle()
        .filter(|b| b.is_ascii_graphic() || *b == b' ')
        .take(1_024 * 1_024)
        .collect();

    let mut group = c.benchmark_group("vt_throughput");
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("write_1mb_ascii", |b| {
        b.iter_batched(
            || VtProcessor::new(220, 50, 1_000, 0, false),
            |mut proc| {
                black_box(proc.process(black_box(&data)));
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

/// Measures scroll throughput: 1 000 `\r\n` on a full 80×24 buffer.
///
/// Buffer fill is done in the setup phase; only the 1 000 scrolls are timed.
fn bench_scroll_throughput(c: &mut Criterion) {
    let row: Vec<u8> = std::iter::repeat(b'X').take(80).collect();

    let mut group = c.benchmark_group("scroll_throughput");
    group.bench_function("scroll_up_1000x", |b| {
        b.iter_batched(
            || {
                let mut proc = VtProcessor::new(80, 24, 1_000, 0, false);
                for _ in 0..24 {
                    proc.process(&row);
                    proc.process(b"\r\n");
                }
                proc
            },
            |mut proc| {
                for _ in 0..1_000 {
                    black_box(proc.process(black_box(b"\r\n" as &[u8])));
                }
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

/// Measures a single mark+iterate cycle on `DirtyRows`.
///
/// `black_box` on the sum prevents the compiler from eliminating the bitfield
/// traversal as dead code.
fn bench_dirty_rows_mark_and_iterate(c: &mut Criterion) {
    use tau_term_lib::vt::screen_buffer::DirtyRows;

    let mut group = c.benchmark_group("dirty_rows");
    group.bench_function("mark_50_rows_and_iterate", |b| {
        b.iter(|| {
            let mut dirty = DirtyRows::default();
            for row in 0u16..50 {
                dirty.set(row);
            }
            let sum: u16 = dirty.iter().sum();
            dirty.clear();
            black_box(sum);
        })
    });
    group.finish();
}

/// Measures the cost of reading 5 dirty rows directly via `get_row()`.
///
/// `black_box` on the cell count prevents elision.
fn bench_partial_update_event(c: &mut Criterion) {
    let content: Vec<u8> = (0u8..128)
        .cycle()
        .filter(|b| b.is_ascii_graphic())
        .take(220 * 50)
        .collect();
    let mut proc = VtProcessor::new(220, 50, 1_000, 0, false);
    proc.process(&content);

    let mut group = c.benchmark_group("partial_update");
    group.bench_function("5_dirty_rows_220x50", |b| {
        b.iter(|| {
            let meta = proc.get_screen_meta();
            let buf = proc.active_buf_ref();
            let mut cell_count = 0usize;
            for row in 0u16..5 {
                if let Some(cells) = buf.get_row(row) {
                    cell_count += cells.len();
                }
            }
            black_box((meta.cols, cell_count));
        })
    });
    group.finish();
}

/// Measures VT throughput on ~200 KB of realistic mixed content:
/// SGR sequences, cursor movement, OSC title, CJK wide chars, plain ASCII.
fn bench_realistic_vt_content(c: &mut Criterion) {
    // Build one ~572-byte repeating block:
    // - SGR bold green filename (30%)
    // - CSI cursor move + erase (20%)
    // - OSC window title (5%)
    // - CJK wide chars (10%)
    // - Plain ASCII fill (35%)
    let block = {
        let mut b = Vec::new();
        // SGR bold green — ~30% of block
        b.extend_from_slice(b"\x1b[1;32mfilename.txt\x1b[0m ");
        b.extend_from_slice(b"\x1b[1;32mfilename.txt\x1b[0m ");
        b.extend_from_slice(b"\x1b[1;32mfilename.txt\x1b[0m ");
        // CSI cursor + erase — ~20% of block
        b.extend_from_slice(b"\x1b[3;5H\x1b[K");
        b.extend_from_slice(b"\x1b[3;5H\x1b[K");
        // OSC title — ~5%
        b.extend_from_slice(b"\x1b]0;htop - 4 processes\x07");
        // CJK wide chars — ~10% (3 bytes each)
        b.extend_from_slice("中文日本語".as_bytes());
        b.extend_from_slice("中文日本語".as_bytes());
        // Plain ASCII fill — ~35%
        b.extend_from_slice(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do.");
        b.extend_from_slice(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do.");
        b.extend_from_slice(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do.");
        b
    };

    // Repeat block until we reach ~200 KB
    let target = 200 * 1024;
    let mut payload = Vec::with_capacity(target + block.len());
    while payload.len() < target {
        payload.extend_from_slice(&block);
    }
    payload.truncate(target);

    let mut group = c.benchmark_group("vt_throughput");
    group.throughput(Throughput::Bytes(payload.len() as u64));
    group.bench_function("write_realistic_vt_content", |b| {
        b.iter_batched(
            || VtProcessor::new(220, 50, 10_000, 0, false),
            |mut proc| {
                black_box(proc.process(black_box(&payload)));
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

/// Measures VT throughput on unicode-heavy content:
/// CJK wide chars, regional indicator emoji pairs, emoji + variation selectors.
fn bench_unicode_emoji_hotpath(c: &mut Criterion) {
    let mut group = c.benchmark_group("unicode_emoji_hotpath");

    // Sub-bench 1: 5000 CJK codepoints cycling through U+4E00..=U+9FFF
    {
        let cjk_chars: Vec<u8> = (0u32..5000)
            .map(|i| char::from_u32(0x4E00 + (i % (0x9FFF - 0x4E00 + 1))).unwrap_or('中'))
            .flat_map(|c| {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf).as_bytes().to_vec()
            })
            .collect();

        group.bench_function("wide_cjk_5000", |b| {
            b.iter_batched(
                || VtProcessor::new(220, 50, 1_000, 0, false),
                |mut proc| {
                    black_box(proc.process(black_box(&cjk_chars)));
                },
                BatchSize::SmallInput,
            )
        });
    }

    // Sub-bench 2: 500 Regional Indicator pairs U+1F1E6 + U+1F1FA (🇦🇺)
    // Each codepoint is 4 bytes UTF-8, so each pair is 8 bytes.
    {
        let ri_a = '\u{1F1E6}'; // Regional Indicator A
        let ri_u = '\u{1F1FA}'; // Regional Indicator U
        let mut ri_bytes = Vec::with_capacity(500 * 8);
        for _ in 0..500 {
            let mut buf = [0u8; 4];
            ri_bytes.extend_from_slice(ri_a.encode_utf8(&mut buf).as_bytes());
            ri_bytes.extend_from_slice(ri_u.encode_utf8(&mut buf).as_bytes());
        }

        group.bench_function("regional_indicators_500_pairs", |b| {
            b.iter_batched(
                || VtProcessor::new(220, 50, 1_000, 0, false),
                |mut proc| {
                    black_box(proc.process(black_box(&ri_bytes)));
                },
                BatchSize::SmallInput,
            )
        });
    }

    // Sub-bench 3: 2000 sequences of U+2764 (❤, 3 bytes) + U+FE0F (variation selector 16, 3 bytes)
    {
        let heart = '\u{2764}'; // ❤ — 3 bytes UTF-8 (E2 9D A4)
        let vs16 = '\u{FE0F}'; // Variation Selector 16 — 3 bytes UTF-8 (EF B8 8F)
        let mut emoji_bytes = Vec::with_capacity(2000 * 6);
        for _ in 0..2000 {
            let mut buf = [0u8; 4];
            emoji_bytes.extend_from_slice(heart.encode_utf8(&mut buf).as_bytes());
            emoji_bytes.extend_from_slice(vs16.encode_utf8(&mut buf).as_bytes());
        }
        assert_eq!(emoji_bytes.len(), 12_000);

        group.bench_function("emoji_variation_2000", |b| {
            b.iter_batched(
                || VtProcessor::new(220, 50, 1_000, 0, false),
                |mut proc| {
                    black_box(proc.process(black_box(&emoji_bytes)));
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

/// Measures scrollback eviction cost: every scroll triggers pop_front + push_back.
///
/// Setup: VtProcessor filled to exactly 1000 scrollback lines.
/// Bench: 1000 additional `\r\n` forcing continuous eviction.
fn bench_scrollback_eviction(c: &mut Criterion) {
    let row: Vec<u8> = std::iter::repeat(b'X').take(80).collect();
    let newline = b"\r\n";

    let mut group = c.benchmark_group("scrollback_eviction");
    group.bench_function("evict_1000_lines", |b| {
        b.iter_batched(
            || {
                let mut proc = VtProcessor::new(80, 24, 1_000, 0, false);
                // Fill scrollback to exactly 1000 lines.
                //
                // On an 80×24 terminal the first 23 newlines only advance the
                // cursor through the visible screen (rows 0→23); no line is
                // pushed to scrollback until the cursor reaches the last row and
                // a further newline triggers scroll_up.  We therefore need
                // `scrollback_limit + (rows - 1) = 1 000 + 23 = 1 023`
                // row+newline pairs to saturate the scrollback ring, so that
                // *every* iteration in the bench loop triggers a pop_front +
                // push_back eviction.
                for _ in 0..1_023 {
                    proc.process(&row);
                    proc.process(newline);
                }
                proc
            },
            |mut proc| {
                // 1000 additional \r\n — each triggers pop_front + push_back.
                for _ in 0..1_000 {
                    black_box(proc.process(black_box(newline)));
                }
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

/// Measures the cost of merging 50 DirtyRegion values into an accumulator.
///
/// Uses a deterministic row pattern to avoid rand dependency.
fn bench_dirty_region_merge_burst(c: &mut Criterion) {
    use tau_term_lib::vt::screen_buffer::DirtyRegion;

    let regions: Vec<DirtyRegion> = (0u16..50)
        .map(|i| {
            let mut r = DirtyRegion::default();
            r.mark_row(i);
            r.mark_row((i * 3) % 50);
            r.mark_row((i * 7) % 50);
            r
        })
        .collect();

    let mut group = c.benchmark_group("dirty_region_merge_burst");
    group.sample_size(500);
    group.bench_function("merge_50_regions", |b| {
        b.iter(|| {
            let mut acc = DirtyRegion::default();
            for r in &regions {
                acc.merge(black_box(r));
            }
            black_box(acc);
        })
    });
    group.finish();
}

/// Measures the full process → build_screen_update_event cycle on a 4 KB mixed payload.
///
/// Represents the hot path in Task 2 of the PTY read loop.
fn bench_full_process_emit_cycle(c: &mut Criterion) {
    use tau_term_lib::session::{ids::PaneId, pty_task::build_screen_update_event};

    // Build a 4 KB mixed VT payload.
    let block = {
        let mut b = Vec::new();
        b.extend_from_slice(b"\x1b[1;32mfilename.txt\x1b[0m ");
        b.extend_from_slice(b"\x1b[3;5H\x1b[K");
        b.extend_from_slice(b"\x1b]0;htop - 4 processes\x07");
        b.extend_from_slice("中文日本語".as_bytes());
        b.extend_from_slice(b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do.");
        b
    };
    let target = 4 * 1024;
    let mut data = Vec::with_capacity(target + block.len());
    while data.len() < target {
        data.extend_from_slice(&block);
    }
    data.truncate(target);

    let mut group = c.benchmark_group("full_process_emit_cycle");
    group.measurement_time(std::time::Duration::from_secs(10));
    group.bench_function("process_4kb_then_build_event", |b| {
        b.iter_batched(
            || {
                let vt = Arc::new(RwLock::new(VtProcessor::new(220, 50, 10_000, 0, false)));
                let pane_id = PaneId("bench".into());
                (vt, pane_id)
            },
            |(vt, pane_id)| {
                let dirty = {
                    let mut w = vt.write();
                    w.process(black_box(&data))
                };
                let event = build_screen_update_event(&pane_id, &vt, black_box(&dirty));
                black_box(event);
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_write_char_throughput,
    bench_scroll_throughput,
    bench_dirty_rows_mark_and_iterate,
    bench_partial_update_event,
    bench_realistic_vt_content,
    bench_unicode_emoji_hotpath,
    bench_scrollback_eviction,
    bench_dirty_region_merge_burst,
    bench_full_process_emit_cycle,
);
criterion_main!(benches);
