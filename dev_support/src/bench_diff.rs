//! Module to compare the difference in latency between two closures.

use hdrhistogram::Histogram;
use latency_trace::{summary_stats, SummaryStats, Timing};
use std::{
    hint::black_box,
    io::{stdout, Write},
    time::Instant,
};

const WARMUP_COUNT: usize = 10;

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
    shift: usize,
) -> (u64, u64) {
    if i % 2 != shift % 2 {
        let l1 = latency(f1, inner_count);
        let l2 = latency(f2, inner_count);
        (l1, l2)
    } else {
        let l2 = latency(f2, inner_count);
        let l1 = latency(f1, inner_count);
        (l1, l2)
    }
}

struct ChainedOutput {
    fn_idx: usize,
    latency: u64,
}

fn outer_core_chained<U, V>(
    i: usize,
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    inner_count: usize,
    shift: usize,
) -> ChainedOutput {
    if i % 2 != shift % 2 {
        let l1 = latency(f1, inner_count);
        ChainedOutput {
            fn_idx: 1,
            latency: l1,
        }
    } else {
        let l2 = latency(f2, inner_count);
        ChainedOutput {
            fn_idx: 2,
            latency: l2,
        }
    }
}

struct BenchDiffHists {
    hist_f1: Timing,
    hist_f2: Timing,
    hist_f1_lt_f2: Timing,
    hist_f1_ge_f2: Timing,
}

pub struct BenchDiffStats {
    pub stats_f1: SummaryStats,
    pub stats_f2: SummaryStats,
    pub stats_f1_lt_f2: SummaryStats,
    pub stats_f1_ge_f2: SummaryStats,
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
fn bench_diff_hists_x<U, V>(
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    outer_count: usize,
    inner_count: usize,
    shift: usize,
    outer_loop_pre: impl Fn(),
    outer_loop_tail: impl Fn(usize),
) -> BenchDiffHists {
    let mut hist_f1_lt_f2 = Histogram::<u64>::new_with_bounds(1, 20 * 1000 * 1000, 2).unwrap();
    let mut hist_f1_ge_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f1 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);

    // Warm-up
    for i in 0..WARMUP_COUNT {
        outer_core(i, &f1, &f2, inner_count, shift);
    }

    outer_loop_pre();

    for i in 1..=outer_count {
        let (elapsed1, elapsed2) = outer_core(i, &f1, &f2, inner_count, shift);

        hist_f1.record(elapsed1).unwrap();
        hist_f2.record(elapsed2).unwrap();

        let diff = elapsed1 as i64 - elapsed2 as i64;

        if diff >= 0 {
            hist_f1_ge_f2
                .record((diff / (inner_count as i64)) as u64)
                .unwrap();
        } else {
            hist_f1_lt_f2
                .record((-diff / (inner_count as i64)) as u64)
                .unwrap();
        }

        outer_loop_tail(i);
    }

    BenchDiffHists {
        hist_f1,
        hist_f2,
        hist_f1_lt_f2,
        hist_f1_ge_f2,
    }
}

pub fn bench_diff_stats_x<U, V>(
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    outer_count: usize,
    inner_count: usize,
    shift: usize,
    outer_loop_pre: impl Fn(),
    outer_loop_tail: impl Fn(usize),
) -> BenchDiffStats {
    let BenchDiffHists {
        hist_f1,
        hist_f2,
        hist_f1_lt_f2,
        hist_f1_ge_f2,
    } = bench_diff_hists_x(
        f1,
        f2,
        outer_count,
        inner_count,
        shift,
        outer_loop_pre,
        outer_loop_tail,
    );

    let stats_f1 = summary_stats(&hist_f1);
    let stats_f2 = summary_stats(&hist_f2);
    let stats_f1_lt_f2 = summary_stats(&hist_f1_lt_f2);
    let stats_f1_ge_f2 = summary_stats(&hist_f1_ge_f2);

    BenchDiffStats {
        stats_f1,
        stats_f2,
        stats_f1_lt_f2,
        stats_f1_ge_f2,
    }
}

pub fn bench_diff_stats<U, V>(
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    outer_count: usize,
    inner_count: usize,
    shift: usize,
) -> BenchDiffStats {
    bench_diff_stats_x(f1, f2, outer_count, inner_count, shift, || (), |_| ())
}

pub fn bench_diff_stats_print<U, V>(
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    outer_count: usize,
    inner_count: usize,
    shift: usize,
    f1_str: &str,
    f2_str: &str,
) {
    println!("\nbench_diff: outer_count={outer_count}, inner_count={inner_count}, shift={shift}");
    println!("f1: {f1_str}");
    println!("f2: {f2_str}");
    println!();
    print!("Warming up ...");
    stdout().flush().unwrap();

    let outer_loop_pre = || {
        println!(" ready to execute");
        print!("Executing bench_diff: ");
        stdout().flush().unwrap();
    };

    let outer_loop_tail = |i| {
        if i % 20 == 0 {
            print!("{i}/{outer_count}");
        } else {
            print!(".");
        }
        stdout().flush().unwrap();
    };

    let BenchDiffStats {
        stats_f1,
        stats_f2,
        stats_f1_lt_f2,
        stats_f1_ge_f2,
    } = bench_diff_stats_x(
        f1,
        f2,
        outer_count,
        inner_count,
        shift,
        outer_loop_pre,
        outer_loop_tail,
    );

    println!(" done\n");

    println!("stats_f1={stats_f1:?}");
    println!("\nstats_f2={stats_f2:?}");
    println!("\nstats_f1_lt_f2={stats_f1_lt_f2:?}");
    println!("\nstats_f1_ge_f2={stats_f1_ge_f2:?}");
    println!();
}

fn bench_diff_chained_hists_x<U, V>(
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    outer_count: usize,
    inner_count: usize,
    shift: usize,
    outer_loop_pre: impl Fn(),
    outer_loop_tail: impl Fn(usize),
) -> BenchDiffHists {
    let mut hist_f1_lt_f2 = Histogram::<u64>::new_with_bounds(1, 20 * 1000 * 1000, 2).unwrap();
    let mut hist_f1_ge_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f1 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);

    let (mut elapsed1, mut elapsed2): (u64, u64) = (0, 0);

    // Warm-up
    for i in 0..WARMUP_COUNT {
        let ChainedOutput { fn_idx, latency } = outer_core_chained(i, &f1, &f2, inner_count, shift);
        let elapseds = [&mut elapsed1, &mut elapsed2];
        *elapseds[fn_idx - 1] = latency;
    }

    outer_loop_pre();

    for i in 1..=outer_count {
        let ChainedOutput { fn_idx, latency } = outer_core_chained(i, &f1, &f2, inner_count, shift);

        let hs_f = [&mut hist_f1, &mut hist_f2];
        hs_f[fn_idx - 1].record(latency).unwrap();
        let elapseds = [&mut elapsed1, &mut elapsed2];
        *elapseds[fn_idx - 1] = latency;

        let diff = elapsed1 as i64 - elapsed2 as i64;

        if diff >= 0 {
            hist_f1_ge_f2
                .record((diff / (inner_count as i64)) as u64)
                .unwrap();
        } else {
            hist_f1_lt_f2
                .record((-diff / (inner_count as i64)) as u64)
                .unwrap();
        }

        outer_loop_tail(i);
    }

    BenchDiffHists {
        hist_f1,
        hist_f2,
        hist_f1_lt_f2,
        hist_f1_ge_f2,
    }
}

pub fn bench_diff_chained_stats_x<U, V>(
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    outer_count: usize,
    inner_count: usize,
    shift: usize,
    outer_loop_pre: impl Fn(),
    outer_loop_tail: impl Fn(usize),
) -> BenchDiffStats {
    let BenchDiffHists {
        hist_f1,
        hist_f2,
        hist_f1_lt_f2,
        hist_f1_ge_f2,
    } = bench_diff_chained_hists_x(
        f1,
        f2,
        outer_count,
        inner_count,
        shift,
        outer_loop_pre,
        outer_loop_tail,
    );

    let stats_f1 = summary_stats(&hist_f1);
    let stats_f2 = summary_stats(&hist_f2);
    let stats_f1_lt_f2 = summary_stats(&hist_f1_lt_f2);
    let stats_f1_ge_f2 = summary_stats(&hist_f1_ge_f2);

    BenchDiffStats {
        stats_f1,
        stats_f2,
        stats_f1_lt_f2,
        stats_f1_ge_f2,
    }
}

pub fn bench_diff_chained_stats<U, V>(
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    outer_count: usize,
    inner_count: usize,
    shift: usize,
) -> BenchDiffStats {
    bench_diff_chained_stats_x(f1, f2, outer_count, inner_count, shift, || (), |_| ())
}

pub fn bench_diff_chained_stats_print<U, V>(
    f1: impl Fn() -> U,
    f2: impl Fn() -> V,
    outer_count: usize,
    inner_count: usize,
    shift: usize,
    f1_str: &str,
    f2_str: &str,
) {
    println!(
        "\nbench_diff_chained: outer_count={outer_count}, inner_count={inner_count}, shift={shift}"
    );
    println!("f1: {f1_str}");
    println!("f2: {f2_str}");
    println!();
    print!("Warming up ...");
    stdout().flush().unwrap();

    let outer_loop_pre = || {
        println!(" ready to execute");
        print!("Executing bench_diff_chained: ");
        stdout().flush().unwrap();
    };

    let outer_loop_tail = |i| {
        if i % 20 == 0 {
            print!("{i}/{outer_count}");
        } else {
            print!(".");
        }
        stdout().flush().unwrap();
    };

    let BenchDiffStats {
        stats_f1,
        stats_f2,
        stats_f1_lt_f2,
        stats_f1_ge_f2,
    } = bench_diff_chained_stats_x(
        f1,
        f2,
        outer_count,
        inner_count,
        shift,
        outer_loop_pre,
        outer_loop_tail,
    );

    println!(" done\n");

    println!("stats_f1={stats_f1:?}");
    println!("\nstats_f2={stats_f2:?}");
    println!("\nstats_f1_lt_f2={stats_f1_lt_f2:?}");
    println!("\nstats_f1_ge_f2={stats_f1_ge_f2:?}");
    println!();
}
