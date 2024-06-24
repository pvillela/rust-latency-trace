//! Executes benchmarks with [`dev_utils::deep_fns`].

use dev_utils::deep_fns::{deep_sync, deep_sync_un};
use latency_trace::bench_support::{measure_latencies1, measure_latencies2};
use latency_trace::{LatencyTrace, Timings};
use std::{fmt::Display, hint::black_box, ops::Range};

fn set_up() {
    let lt = LatencyTrace::default();
    measure_latencies1(lt);
}

fn sync_direct(nrepeats: usize, ntasks: usize) {
    deep_sync(nrepeats, ntasks);
}

fn sync_completion(nrepeats: usize, ntasks: usize) {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, move || deep_sync(nrepeats, ntasks));
}

fn sync_all_in(nrepeats: usize, ntasks: usize) -> Timings {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || deep_sync(nrepeats, ntasks));
    black_box(timings)
}

fn sync_un_direct(nrepeats: usize, ntasks: usize) {
    deep_sync_un(nrepeats, ntasks);
}

fn sync_un_completion(nrepeats: usize, ntasks: usize) {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, move || deep_sync_un(nrepeats, ntasks));
}

fn sync_un_all_in(nrepeats: usize, ntasks: usize) -> Timings {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || deep_sync_un(nrepeats, ntasks));
    black_box(timings)
}

struct Params {
    nrepeats: usize,
    ntasks: usize,
}

impl Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Params { nrepeats, ntasks } = self;
        f.write_fmt(format_args!("(nrepeats={nrepeats}, ntasks={ntasks})"))
    }
}

const ARR_PARAMS: [Params; 6] = [
    Params {
        nrepeats: 10,
        ntasks: 0,
    },
    Params {
        nrepeats: 20,
        ntasks: 0,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
    },
    Params {
        nrepeats: 10,
        ntasks: 5,
    },
    Params {
        nrepeats: 20,
        ntasks: 5,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
    },
];

const fn index_range<T, const N: usize>(_arr: &[T; N]) -> Range<usize> {
    Range { start: 0, end: N }
}

#[divan::bench]
fn set_up_bench() {
    set_up()
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_direct_bench(idx: usize) {
    let Params { nrepeats, ntasks } = ARR_PARAMS[idx];
    sync_direct(nrepeats, ntasks)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_completion_bench(idx: usize) {
    let Params { nrepeats, ntasks } = ARR_PARAMS[idx];
    sync_completion(nrepeats, ntasks)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_all_in_bench(idx: usize) {
    let Params { nrepeats, ntasks } = ARR_PARAMS[idx];
    sync_all_in(nrepeats, ntasks);
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_direct_bench(idx: usize) {
    let Params { nrepeats, ntasks } = ARR_PARAMS[idx];
    sync_un_direct(nrepeats, ntasks)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_completion_bench(idx: usize) {
    let Params { nrepeats, ntasks } = ARR_PARAMS[idx];
    sync_un_completion(nrepeats, ntasks)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_all_in_bench(idx: usize) {
    let Params { nrepeats, ntasks } = ARR_PARAMS[idx];
    sync_un_all_in(nrepeats, ntasks);
}

fn main() {
    for i in index_range(&ARR_PARAMS) {
        let Params { nrepeats, ntasks } = ARR_PARAMS[i];

        // do it twice to make sure each execution starts clean
        for _ in 0..2 {
            // Print span count.
            let timings = sync_all_in(nrepeats, ntasks);
            let span_count = timings.values().fold(0, |acc, hist| acc + hist.len());
            println!("idx={i}, params={}, span_count={span_count}", ARR_PARAMS[i]);
        }
    }

    // Run benchmarks:
    divan::main();
}
