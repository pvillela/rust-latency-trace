//! Executes benchmarks with [`dev_support::simple_fns::simple_real_sync`].

use dev_support::bench_support::{
    common::index_range,
    simple_real::{
        set_up, sync_all_in, sync_completion, sync_un_all_in, sync_un_completion, sync_un_direct,
        Params, ARR_PARAMS,
    },
};

#[divan::bench]
fn set_up_bench() {
    set_up()
}

#[allow(unused)]
// #[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_completion_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        extent,
    } = ARR_PARAMS[idx];
    sync_completion(nrepeats, ntasks, extent)
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_all_in_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        extent,
    } = ARR_PARAMS[idx];
    sync_all_in(nrepeats, ntasks, extent);
}

#[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_direct_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        extent,
    } = ARR_PARAMS[idx];
    sync_un_direct(nrepeats, ntasks, extent)
}

#[allow(unused)]
// #[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_completion_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        extent,
    } = ARR_PARAMS[idx];
    sync_un_completion(nrepeats, ntasks, extent)
}

#[allow(unused)]
// #[divan::bench(args = index_range(&ARR_PARAMS))]
fn sync_un_all_in_bench(idx: usize) {
    let Params {
        nrepeats,
        ntasks,
        extent,
    } = ARR_PARAMS[idx];
    sync_un_all_in(nrepeats, ntasks, extent);
}

fn main() {
    for i in index_range(&ARR_PARAMS) {
        let Params {
            nrepeats,
            ntasks,
            extent: _,
        } = ARR_PARAMS[i];
        let timings = sync_all_in(nrepeats, ntasks, 0);
        let span_count = timings.values().fold(0, |acc, hist| acc + hist.len());
        println!("idx={i}, params={}, span_count={span_count}", ARR_PARAMS[i]);
    }

    // Run benchmarks:
    divan::main();
}
