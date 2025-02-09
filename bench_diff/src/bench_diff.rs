//! Module to compare the difference in latency between two closures.

use crate::{new_timing, summary_stats, SummaryStats, Timing};
use hdrhistogram::Histogram;
use statrs::distribution::{ContinuousCDF, StudentsT};
use std::{
    io::{stdout, Write},
    time::Instant,
};

const WARMUP_COUNT: usize = 10;

fn latency(f: impl Fn()) -> u64 {
    let start = Instant::now();
    f();
    let elapsed = Instant::now().duration_since(start);
    elapsed.as_micros() as u64
}

fn quad_exec(f1: impl Fn(), f2: impl Fn()) -> [(u64, u64); 4] {
    let l01 = latency(&f1);
    let l02 = latency(&f2);

    let l11 = latency(&f1);
    let l12 = latency(&f2);

    let l22 = latency(&f2);
    let l21 = latency(&f1);

    let l32 = latency(&f2);
    let l31 = latency(&f1);

    [(l01, l02), (l11, l12), (l21, l22), (l31, l32)]
}

pub struct BenchDiffOut {
    hist_f1: Timing,
    hist_f2: Timing,
    hist_f1_lt_f2: Timing, //todo: replace with count, sum and sum of squares of ratios
    hist_f1_ge_f2: Timing, //todo: replace with count, sum and sum of squares of ratios
    sum_ln_f1: f64,
    sum2_ln_f1: f64,
    sum_ln_f2: f64,
    sum2_ln_f2: f64,
}

impl BenchDiffOut {
    pub fn summary_f1(&self) -> SummaryStats {
        summary_stats(&self.hist_f1)
    }

    pub fn summary_f2(&self) -> SummaryStats {
        summary_stats(&self.hist_f2)
    }

    pub fn count_f1_lt_f2(&self) -> u64 {
        self.hist_f1_lt_f2.len()
    }

    pub fn count_f1_ge_f2(&self) -> u64 {
        self.hist_f1_ge_f2.len()
    }

    pub fn mean_ln_f1(&self) -> f64 {
        self.sum2_ln_f1 / self.hist_f1.len() as f64
    }

    pub fn stdev_ln_f1(&self) -> f64 {
        let n = self.hist_f1.len() as f64;
        (self.sum2_ln_f1 - self.sum_ln_f1.powi(2) / n) / (n - 1.0)
    }

    pub fn mean_ln_f2(&self) -> f64 {
        self.sum2_ln_f2 / self.hist_f2.len() as f64
    }

    pub fn stdev_ln_f2(&self) -> f64 {
        let n = self.hist_f2.len() as f64;
        (self.sum2_ln_f2 - self.sum_ln_f2.powi(2) / n) / (n - 1.0)
    }

    /// Welch's t statistic for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm),
    ///
    /// See [Welch's t-test](https://en.wikipedia.org/wiki/Welch%27s_t-test)
    pub fn welch_ln_t(&self) -> f64 {
        let n = self.hist_f1.len() as f64;
        let dx = self.mean_ln_f1() - self.mean_ln_f2();
        let s2_x1 = self.stdev_ln_f1().powi(2) / n;
        let s2_x2 = self.stdev_ln_f2().powi(2) / n;
        let s_dx = (s2_x1 + s2_x2).sqrt();
        dx / s_dx
    }

    /// Degrees of freedom for Welch's t-test for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm),
    ///
    /// See [Welch's t-test](https://en.wikipedia.org/wiki/Welch%27s_t-test)
    pub fn welch_ln_deg_freedom(&self) -> f64 {
        let n = self.hist_f1.len() as f64;
        let s2_x1 = self.stdev_ln_f1().powi(2) / n;
        let s2_x2 = self.stdev_ln_f2().powi(2) / n;
        let s2_dx = s2_x1 + s2_x2;
        (n - 1.0) * s2_dx.powi(2) / (s2_x1.powi(2) + s2_x2.powi(2))
    }

    /// Confidence interval for
    /// `mean(ln(latency(f1))) - mean(ln(latency(f2)))` (where `ln` is the natural logarithm),
    /// with confidence level of `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are log-normal. This assumption is widely supported by
    /// performance analysis theory and empirical data.
    ///
    /// This is also the confidence interval for the difference of medians of logarithms under the above assumption.
    pub fn welch_ln_ci(&self, alpha: f64) -> (f64, f64) {
        let n = self.hist_f1.len() as f64;
        let dx = self.mean_ln_f1() - self.mean_ln_f2();
        let s2_x1 = self.stdev_ln_f1().powi(2) / n;
        let s2_x2 = self.stdev_ln_f2().powi(2) / n;
        let nu = self.welch_ln_deg_freedom();

        let stud = StudentsT::new(0.0, 1.0, nu)
            .expect("can't happen: degrees of freedom is always >= 3 by construction");
        let t = -stud.inverse_cdf(alpha / 2.0);

        let mid = dx;
        let radius = (s2_x1 + s2_x2).sqrt() * t;

        (mid - radius, mid + radius)
    }

    /// Confidence interval for
    /// `median(latency(f1)) / median(latency(f2))`,
    /// with confidence level of `(1 - alpha)`.
    ///
    /// Assumes that both `latency(f1)` and `latency(f2)` are log-normal. This assumption is widely supported by
    /// performance analysis theory and empirical data.
    pub fn welch_ratio_ci(&self, alpha: f64) -> (f64, f64) {
        let (log_low, log_high) = self.welch_ln_ci(alpha);
        let low = log_low.exp();
        let high = log_high.exp();
        (low, high)
    }
}

/// Compares the difference of total latency for two closures `f1` and `f2` in ***microseconds***.
/// Differences (latency(f1) - latency(f2)) are collected in two [`Histogram`]s, one for positive differences and the
/// other for negative differences.
///
/// Arguments:
/// - `f1` - first target for comparison.
/// - `f2` - second target for comparison.
/// - `exec_count` - number of outer loop repetitions. For each iteration, the inner loop (see below) is executed for
///   each of the target closures.
/// - `inner_count` - number of inner loop repetitions. Within each outer loop iteration and for each of the target closures,
///   the target closure is executed `inner_count times`, the total latency for the inner loop is measured for the
///   target closure for the inner loop. The mean difference `(total_latency(f1) - total_latency(f2)) / inner_count` is
///   calculated. Depending on whether the mean difference is positive or negative, it is recorded on the histogram
///   `hist_f1_ge_f2` or `hist_f1_lt_f2`, respectively.
/// - `f_args_str` - string that documents relevant arguments enclosed by the closures `f1` and `f2` (e.g., using the
///   `format!` macro). It is printed together with `exec_count` and `inner_count` to provide context for the benchmark.
///
/// The benchmark is warmed-up with one additional initial outer loop iteration for which measurements are not collected.
pub fn bench_diff_x(
    f1: impl Fn(),
    f2: impl Fn(),
    exec_count: usize,
    outer_loop_pre: impl Fn(),
    outer_loop_tail: impl Fn(usize),
) -> BenchDiffOut {
    let mut hist_f1_lt_f2 = new_timing(20 * 1000 * 1000, 2);
    let mut hist_f1_ge_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f1 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut hist_f2 = Histogram::<u64>::new_from(&hist_f1_lt_f2);
    let mut sum_ln_f1 = 0.0_f64;
    let mut sum2_ln_f1 = 0.0_f64;
    let mut sum_ln_f2 = 0.0_f64;
    let mut sum2_ln_f2 = 0.0_f64;

    // Warm-up
    for _ in 0..WARMUP_COUNT {
        quad_exec(&f1, &f2);
    }

    outer_loop_pre();

    for i in 1..=exec_count / 4 {
        let pairs = quad_exec(&f1, &f2);

        for (elapsed1, elapsed2) in pairs {
            hist_f1
                .record(elapsed1)
                .expect("can't happen: histogram is auto-resizable");
            hist_f2
                .record(elapsed2)
                .expect("can't happen: histogram is auto-resizable");

            let diff = elapsed1 as i64 - elapsed2 as i64;

            if diff >= 0 {
                hist_f1_ge_f2
                    .record(diff as u64)
                    .expect("can't happen: histogram is auto-resizable");
            } else {
                hist_f1_lt_f2
                    .record(-diff as u64)
                    .expect("can't happen: histogram is auto-resizable");
            }

            let ln_f1 = (elapsed1 as f64).ln();
            sum_ln_f1 += ln_f1;
            sum2_ln_f1 += ln_f1 * ln_f1;

            let ln_f2 = (elapsed1 as f64).ln();
            sum_ln_f2 += ln_f2;
            sum2_ln_f2 += ln_f2 * ln_f2;
        }

        outer_loop_tail(i * 4);
    }

    BenchDiffOut {
        hist_f1,
        hist_f2,
        hist_f1_lt_f2,
        hist_f1_ge_f2,
        sum_ln_f1,
        sum2_ln_f1,
        sum_ln_f2,
        sum2_ln_f2,
    }
}

pub fn bench_diff(f1: impl Fn(), f2: impl Fn(), exec_count: usize) -> BenchDiffOut {
    bench_diff_x(f1, f2, exec_count, || (), |_| ())
}

pub fn bench_diff_print(
    f1: impl Fn(),
    f2: impl Fn(),
    exec_count: usize,
    print_sub_header: impl Fn(),
    print_stats: impl Fn(BenchDiffOut),
) {
    println!("\nbench_diff: exec_count={exec_count}");
    print_sub_header();
    println!();
    print!("Warming up ...");
    stdout().flush().expect("unexpected I/O error");

    let outer_loop_pre = || {
        println!(" ready to execute");
        print!("Executing bench_diff: ");
        stdout().flush().expect("unexpected I/O error");
    };

    let outer_loop_tail = |i| {
        if i % 20 == 0 {
            print!("{i}/{exec_count}");
        } else {
            print!(".");
        }
        stdout().flush().expect("unexpected I/O error");
    };

    let diff_out = bench_diff_x(f1, f2, exec_count, outer_loop_pre, outer_loop_tail);

    println!(" done\n");

    print_stats(diff_out);
}
