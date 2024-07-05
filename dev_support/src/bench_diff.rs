//! Compares the difference of total latency for two closures.

use hdrhistogram::Histogram;
use latency_trace::summary_stats;
use std::{
    hint::black_box,
    io::{self, Write},
    time::Instant,
};

/// Compares the difference of total latency for two closures `f1` and `f2` in ***microseconds***.
/// Differences (latency(f1) - latenchy(f2)) are collected in two [`Histogram`]s, one for positive differences and the
/// other for negative differences.
///
/// Arguments:
/// - `f1` - first target for comparison.
/// - `f2` - second target for comparison.
/// - `outer_loop` - number of outer loop repetitions. For each iteration, the inner loop (see below) is executed for
/// each of the target closures.
/// - `inner_loop` - number of inner loop repetitions. Within each outer loop iteration and for each of the target closures,
/// the target closure is executed `inner_loop times`, the total latency for the inner loop is measured for the
/// target closure for the inner loop. The mean difference `(total_latency(f1) - total_latency(f2)) / inner_loop` is
/// calculated. Depending on whether the mean difference is positive or negative, it is recorded on the histogram
/// `hist_pos` or `hist_neg`, respectively.
///
/// The benchmark is warmed-up with one additional initial outer loop iteration for which measurements are not collected.
pub fn bench_diff<U>(f1: impl Fn() -> U, f2: impl Fn() -> U, outer_loop: usize, inner_loop: usize) {
    let mut hist_neg = Histogram::<u64>::new_with_bounds(1, 20 * 1000 * 1000, 2).unwrap();
    let mut hist_pos = Histogram::<u64>::new_from(&hist_neg);

    let outer_core = || {
        let start1 = Instant::now();
        for _ in 0..inner_loop {
            black_box(f1());
        }
        let elapsed1 = Instant::now().duration_since(start1);
        let elapsed1_micros = elapsed1.as_micros() as i128;

        let start2 = Instant::now();
        for _ in 0..inner_loop {
            black_box(f2());
        }
        let elapsed2 = Instant::now().duration_since(start2);
        let elapsed2_micros = elapsed2.as_micros() as i128;

        (elapsed1_micros - elapsed2_micros) as i64
    };

    print!("\nExecuting bench_diff: ");
    io::stdout().flush().unwrap();

    // Warm-up
    // outer_core();

    for _ in 0..outer_loop {
        let diff = outer_core();

        if diff >= 0 {
            hist_pos
                .record((diff / (inner_loop as i64)) as u64)
                .unwrap();
        } else {
            hist_neg.record((-diff / inner_loop as i64) as u64).unwrap();
        }

        print!(".");
        io::stdout().flush().unwrap();
    }

    println!(" done\n");

    let summary_neg = summary_stats(&hist_neg);
    let summary_pos = summary_stats(&hist_pos);

    println!("summary_neg={summary_neg:?}");
    println!("summary_pos={summary_pos:?}");
    println!();
}
