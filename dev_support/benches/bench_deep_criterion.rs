//! Executes benchmarks with [`dev_support::deep_fns`].

use criterion::{criterion_group, criterion_main, Criterion};
use dev_support::bench_support::deep::{
    set_up, sync_all_in, sync_completion, sync_un_direct, Params, ARR_PARAMS,
};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("set-up", |b| b.iter(set_up));

    for params in ARR_PARAMS {
        let Params {
            nrepeats,
            ntasks,
            span_count,
        } = params;

        c.bench_function(&format!("sync_completion({params})"), |b| {
            b.iter(|| sync_completion(nrepeats, ntasks))
        });

        c.bench_function(&format!("sync_all_in({params})"), |b| {
            b.iter(|| sync_all_in(nrepeats, ntasks, span_count))
        });
        c.bench_function(&format!("sync_un_direct({params})"), |b| {
            b.iter(|| sync_un_direct(nrepeats, ntasks))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
