//! Compares the difference of total latency for two closures.

use hdrhistogram::Histogram;
use latency_trace::summary_stats;
use std::{
    hint::black_box,
    io::{self, Write},
    time::Instant,
};

/// Compares the difference of total latency for two closures `f1` and `f2` in ***microseconds***.
/// Differences (latency(f1) - latency(f2)) are collected in two [`Histogram`]s, one for positive differences and the
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
/// `hist_f1_ge_f2` or `hist_f1_lt_f2`, respectively.
///
/// The benchmark is warmed-up with one additional initial outer loop iteration for which measurements are not collected.
pub fn bench_diff<U>(f1: impl Fn() -> U, f2: impl Fn() -> U, outer_loop: usize, inner_loop: usize) {
    let mut hist_f1_lt_f2 = Histogram::<u64>::new_with_bounds(1, 20 * 1000 * 1000, 2).unwrap();
    let mut hist_f1_ge_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f1 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);

    let outer_core = || {
        let start1 = Instant::now();
        for _ in 0..inner_loop {
            black_box(f1());
        }
        let elapsed1 = Instant::now().duration_since(start1);
        let elapsed1_micros = elapsed1.as_micros() as u64;

        let start2 = Instant::now();
        for _ in 0..inner_loop {
            black_box(f2());
        }
        let elapsed2 = Instant::now().duration_since(start2);
        let elapsed2_micros = elapsed2.as_micros() as u64;

        (elapsed1_micros, elapsed2_micros)
    };

    // Warm-up
    print!("Warming up ...");
    io::stdout().flush().unwrap();
    outer_core();
    println!(" ready to execute");

    print!("Executing bench_diff: ");
    io::stdout().flush().unwrap();

    for i in 1..=outer_loop {
        let (elapsed1, elapsed2) = outer_core();

        hist_f1.record(elapsed1).unwrap();
        hist_f2.record(elapsed2).unwrap();

        let diff = elapsed1 as i64 - elapsed2 as i64;

        if diff >= 0 {
            hist_f1_ge_f2
                .record((diff / (inner_loop as i64)) as u64)
                .unwrap();
        } else {
            hist_f1_lt_f2
                .record((-diff / inner_loop as i64) as u64)
                .unwrap();
        }

        if i % 20 == 0 {
            print!("{i}/{outer_loop}");
        } else {
            print!(".");
        }
        io::stdout().flush().unwrap();
    }

    println!(" done\n");

    let summary_f1 = summary_stats(&hist_f1);
    let summary_f2 = summary_stats(&hist_f2);
    let summary_f1_lt_f2 = summary_stats(&hist_f1_lt_f2);
    let summary_f1_ge_f2 = summary_stats(&hist_f1_ge_f2);

    println!("summary_f1={summary_f1:?}");
    println!("\nsummary_f2={summary_f2:?}");
    println!("\nsummary_f1_lt_f2={summary_f1_lt_f2:?}");
    println!("\nsummary_f1_ge_f2={summary_f1_ge_f2:?}");
    println!();
}
