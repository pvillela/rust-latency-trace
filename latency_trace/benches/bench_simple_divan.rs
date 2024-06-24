//! Executes benchmarks with [`dev_utils::simple_fns`].

use dev_utils::simple_fns::{simple_async, simple_async_un, simple_sync, simple_sync_un};
use latency_trace::bench_support::{
    measure_latencies1, measure_latencies2, measure_latencies2_tokio,
};
use latency_trace::{LatencyTrace, Timings};
use std::fmt::Display;
use std::hint::black_box;
use std::ops::Range;

fn set_up() {
    let lt = LatencyTrace::default();
    measure_latencies1(lt);
}

fn sync_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, move || simple_sync(nrepeats, ntasks, sleep_micros));
}

fn sync_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) -> Timings {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || simple_sync(nrepeats, ntasks, sleep_micros));
    black_box(timings)
}

fn sync_un_direct(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    simple_sync_un(nrepeats, ntasks, sleep_micros);
}

fn sync_un_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, move || simple_sync(nrepeats, ntasks, sleep_micros));
}

fn sync_un_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) -> Timings {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || simple_sync(nrepeats, ntasks, sleep_micros));
    black_box(timings)
}

fn async_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::default();
    measure_latencies2_tokio(lt, move || simple_async(nrepeats, ntasks, sleep_micros));
}

fn async_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies_tokio(move || simple_async(nrepeats, ntasks, sleep_micros));
    black_box(timings);
}

fn async_un_direct(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(simple_async_un(nrepeats, ntasks, sleep_micros));
}

fn async_un_completion(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, move || simple_sync(nrepeats, ntasks, sleep_micros));
}

fn async_un_all_in(nrepeats: usize, ntasks: usize, sleep_micros: u64) -> Timings {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || simple_sync(nrepeats, ntasks, sleep_micros));
    black_box(timings)
}

struct Params {
    nrepeats: usize,
    ntasks: usize,
    sleep_micros: u64,
}

impl Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Params {
            nrepeats,
            ntasks,
            sleep_micros,
        } = self;
        f.write_fmt(format_args!(
            "(nrepeats={nrepeats}, ntasks={ntasks}, sleep_micros={sleep_micros})"
        ))
    }
}

const ARR_PARAMS: [Params; 6] = [
    Params {
        nrepeats: 10,
        ntasks: 0,
        sleep_micros: 1000,
    },
    Params {
        nrepeats: 20,
        ntasks: 0,
        sleep_micros: 500,
    },
    Params {
        nrepeats: 100,
        ntasks: 0,
        sleep_micros: 100,
    },
    Params {
        nrepeats: 10,
        ntasks: 5,
        sleep_micros: 1000,
    },
    Params {
        nrepeats: 20,
        ntasks: 5,
        sleep_micros: 500,
    },
    Params {
        nrepeats: 100,
        ntasks: 5,
        sleep_micros: 100,
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
fn sync_completion_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    sync_completion(nrepeats, ntasks, sleep_micros)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_all_in_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    sync_all_in(nrepeats, ntasks, sleep_micros);
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_direct_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    sync_un_direct(nrepeats, ntasks, sleep_micros)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_completion_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    sync_un_completion(nrepeats, ntasks, sleep_micros)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_all_in_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    sync_un_all_in(nrepeats, ntasks, sleep_micros);
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn async_completion_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    async_completion(nrepeats, ntasks, sleep_micros)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn async_all_in_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    async_all_in(nrepeats, ntasks, sleep_micros);
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn async_un_direct_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    async_un_direct(nrepeats, ntasks, sleep_micros)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn async_un_completion_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    async_un_completion(nrepeats, ntasks, sleep_micros)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn async_un_all_in_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    async_un_all_in(nrepeats, ntasks, sleep_micros);
}

fn main() {
    for i in index_range(&ARR_PARAMS) {
        let Params {
            nrepeats,
            ntasks,
            sleep_micros: _,
        } = ARR_PARAMS[i];
        let timings = sync_all_in(nrepeats, ntasks, 0);
        let span_count = timings.values().fold(0, |acc, hist| acc + hist.len());
        println!("idx={i}, params={}, span_count={span_count}", ARR_PARAMS[i]);
    }

    // Run benchmarks:
    divan::main();
}
