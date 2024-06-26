//! Executes benchmarks with [`dev_support::simple_fns`].

use dev_support::bench_support::{
    common::index_range,
    simple::{
        async_all_in, async_completion, async_un_all_in, async_un_completion, async_un_direct,
        set_up, sync_all_in, sync_completion, sync_un_all_in, sync_un_completion, sync_un_direct,
        Params, ARR_PARAMS,
    },
};

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

// #[divan::bench(args = index_range(&ARR_PARAMS))]
#[allow(unused)]
fn async_completion_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    async_completion(nrepeats, ntasks, sleep_micros)
}

// #[divan::bench(args = index_range(&ARR_PARAMS))]
#[allow(unused)]
fn async_all_in_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    async_all_in(nrepeats, ntasks, sleep_micros);
}

// #[divan::bench(args = index_range(&ARR_PARAMS))]
#[allow(unused)]
fn async_un_direct_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    async_un_direct(nrepeats, ntasks, sleep_micros)
}

// #[divan::bench(args = index_range(&ARR_PARAMS))]
#[allow(unused)]
fn async_un_completion_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        sleep_micros,
    } = ARR_PARAMS[idx];
    async_un_completion(nrepeats, ntasks, sleep_micros)
}

// #[divan::bench(args = index_range(&ARR_PARAMS))]
#[allow(unused)]
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
