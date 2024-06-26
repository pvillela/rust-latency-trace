//! Executes benchmarks with [`dev_utils::deep_fns`].

mod common_deep;

#[allow(unused)]
use common_deep::sync_completion;
use common_deep::{set_up, sync_all_in, sync_un_direct, Params, ARR_PARAMS};
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function(&format!("set-up"), |b| b.iter(set_up));

    for params in ARR_PARAMS {
        let Params {
            nrepeats,
            ntasks,
            span_count,
        } = params;

        // Below commented-out because it crashes when ntasks > 0. It may be something with `std::hint::black_box` which
        // is used in `sync_completion` instead of `criterion::black_box`.
        // c.bench_function(&format!("sync_completion({params})"), |b| {
        //     b.iter(|| sync_completion(nrepeats, ntasks))
        // });

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
