// SPDX-License-Identifier: MPL-2.0

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
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
            || VtProcessor::new(220, 50, 1_000),
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
                let mut proc = VtProcessor::new(80, 24, 1_000);
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
    let mut proc = VtProcessor::new(220, 50, 1_000);
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

criterion_group!(
    benches,
    bench_write_char_throughput,
    bench_scroll_throughput,
    bench_dirty_rows_mark_and_iterate,
    bench_partial_update_event,
);
criterion_main!(benches);
