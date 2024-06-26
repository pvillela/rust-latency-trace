//! Executes benchmarks with [`dev_support::deep_fns`].

use dev_support::bench_support::{
    common::index_range,
    deep::{
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
        nrepeats, ntasks, ..
    } = ARR_PARAMS[idx];
    sync_completion(nrepeats, ntasks)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_all_in_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        span_count,
    } = ARR_PARAMS[idx];
    sync_all_in(nrepeats, ntasks, span_count);
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_direct_bench(idx: usize) {
    let Params {
        nrepeats, ntasks, ..
    } = ARR_PARAMS[idx];
    sync_un_direct(nrepeats, ntasks)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_completion_bench(idx: usize) {
    let Params {
        nrepeats, ntasks, ..
    } = ARR_PARAMS[idx];
    sync_un_completion(nrepeats, ntasks)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_all_in_bench(idx: usize) {
    let Params {
        nrepeats, ntasks, ..
    } = ARR_PARAMS[idx];
    sync_un_all_in(nrepeats, ntasks);
}

fn main() {
    for i in index_range(&ARR_PARAMS) {
        let Params {
            nrepeats,
            ntasks,
            span_count,
        } = ARR_PARAMS[i];

        // do it twice to make sure each execution starts clean
        for _ in 0..2 {
            // Print span count.
            let timings = sync_all_in(nrepeats, ntasks, span_count);
            let span_count = timings.values().fold(0, |acc, hist| acc + hist.len());
            println!("idx={i}, params={}, span_count={span_count}", ARR_PARAMS[i]);
        }
    }

    // Run benchmarks:
    divan::main();
}
