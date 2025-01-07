//! Module to compare the difference in latency between two closures.

use hdrhistogram::Histogram;
use latency_trace::summary_stats;
use std::{
    hint::black_box,
    io::{stdout, Write},
    time::Instant,
};

fn latency<U>(f: impl Fn() -> U, inner_count: usize) -> u64 {
    let start = Instant::now();
    for _ in 0..inner_count {
        black_box(f());
    }
    let elapsed = Instant::now().duration_since(start);
    elapsed.as_micros() as u64
}

fn outer_core<U, V>(
    i: usize,
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    inner_count: usize,
) -> (u64, u64) {
    if i % 2 == 1 {
        let l1 = latency(f1, inner_count);
        let l2 = latency(f2, inner_count);
        (l1, l2)
    } else {
        let l2 = latency(f2, inner_count);
        let l1 = latency(f1, inner_count);
        (l1, l2)
    }
}

/// Compares the difference of total latency for two closures `f1` and `f2` in ***microseconds***.
/// Differences (latency(f1) - latency(f2)) are collected in two [`Histogram`]s, one for positive differences and the
/// other for negative differences.
///
/// Arguments:
/// - `f1` - first target for comparison.
/// - `f2` - second target for comparison.
/// - `outer_count` - number of outer loop repetitions. For each iteration, the inner loop (see below) is executed for
///   each of the target closures.
/// - `inner_count` - number of inner loop repetitions. Within each outer loop iteration and for each of the target closures,
///   the target closure is executed `inner_count times`, the total latency for the inner loop is measured for the
///   target closure for the inner loop. The mean difference `(total_latency(f1) - total_latency(f2)) / inner_count` is
///   calculated. Depending on whether the mean difference is positive or negative, it is recorded on the histogram
///   `hist_f1_ge_f2` or `hist_f1_lt_f2`, respectively.
/// - `f_args_str` - string that documents relevant arguments enclosed by the closures `f1` and `f2` (e.g., using the
///   `format!` macro). It is printed together with `outer_count` and `inner_count` to provide context for the benchmark.
///
/// The benchmark is warmed-up with one additional initial outer loop iteration for which measurements are not collected.
pub fn bench_diff<U>(
    f1: impl Fn() -> U,
    f2: impl Fn() -> U,
    outer_count: usize,
    inner_count: usize,
    f_args_str: &str,
) {
    let mut hist_f1_lt_f2 = Histogram::<u64>::new_with_bounds(1, 20 * 1000 * 1000, 2).unwrap();
    let mut hist_f1_ge_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f1 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);

    println!(
        "\nContext: (outer_count={outer_count}, inner_count={inner_count}, f_args=[{f_args_str}])"
    );
    println!();

    // Warm-up
    print!("Warming up ...");
    stdout().flush().unwrap();
    outer_core(0, &f1, &f2, inner_count);
    println!(" ready to execute");

    print!("Executing bench_diff: ");
    stdout().flush().unwrap();

    for i in 1..=outer_count {
        let (elapsed1, elapsed2) = outer_core(i, &f1, &f2, inner_count);

        hist_f1.record(elapsed1).unwrap();
        hist_f2.record(elapsed2).unwrap();

        let diff = elapsed1 as i64 - elapsed2 as i64;

        if diff >= 0 {
            hist_f1_ge_f2
                .record((diff / (inner_count as i64)) as u64)
                .unwrap();
        } else {
            hist_f1_lt_f2
                .record((-diff / inner_count as i64) as u64)
                .unwrap();
        }

        if i % 20 == 0 {
            print!("{i}/{outer_count}");
        } else {
            print!(".");
        }
        stdout().flush().unwrap();
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
