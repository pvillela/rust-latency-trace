//! Executes benchmarks with [`dev_support::simple_fns`].

mod common_simple;

#[allow(unused_imports)]
use common_simple::{
    async_all_in, async_completion, async_un_direct, set_up, sync_all_in, sync_completion,
    sync_un_direct, Params, ARR_PARAMS,
};
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function(&format!("set-up"), |b| b.iter(set_up));

    for params in ARR_PARAMS {
        let Params {
            nrepeats,
            ntasks,
            sleep_micros,
        } = params;

        c.bench_function(&format!("sync_completion({params})"), |b| {
            b.iter(|| sync_completion(nrepeats, ntasks, sleep_micros))
        });
        c.bench_function(&format!("sync_all_in({params})"), |b| {
            b.iter(|| sync_all_in(nrepeats, ntasks, sleep_micros))
        });
        c.bench_function(&format!("sync_un_direct({params})"), |b| {
            b.iter(|| sync_un_direct(nrepeats, ntasks, sleep_micros))
        });
        // c.bench_function(&format!("async_completion({params})"), |b| {
        //     b.iter(|| async_completion(nrepeats, ntasks, sleep_micros))
        // });
        // c.bench_function(&format!("async_all_in({params})"), |b| {
        //     b.iter(|| async_all_in(nrepeats, ntasks, sleep_micros))
        // });
        // c.bench_function(&format!("async_un_direct({params})"), |b| {
        //     b.iter(|| async_un_direct(nrepeats, ntasks, sleep_micros))
        // });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
