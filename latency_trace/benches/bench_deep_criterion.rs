//! Executes benchmarks with [`dev_utils::deep_fns`].

use std::fmt::Display;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dev_utils::deep_fns::{deep_sync, deep_sync_un};
use latency_trace::bench_support::{measure_latencies1, measure_latencies2};
use latency_trace::LatencyTrace;

fn set_up_bench() {
    let lt = LatencyTrace::default();
    measure_latencies1(lt);
}

fn sync_completion_bench(nrepeats: usize, ntasks: usize) {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, move || deep_sync(nrepeats, ntasks));
}

fn sync_all_in_bench(nrepeats: usize, ntasks: usize) {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || deep_sync(nrepeats, ntasks));
    black_box(timings);
}

fn sync_un_bench(nrepeats: usize, ntasks: usize) {
    deep_sync_un(nrepeats, ntasks);
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

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function(&format!("set-up"), |b| b.iter(set_up_bench));

    for params in [
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
    ] {
        let Params { nrepeats, ntasks } = params;

        c.bench_function(&format!("sync_completion({params})"), |b| {
            b.iter(|| sync_completion_bench(nrepeats, ntasks))
        });
        c.bench_function(&format!("sync_all_in({params})"), |b| {
            b.iter(|| sync_all_in_bench(nrepeats, ntasks))
        });
        c.bench_function(&format!("sync_un({params})"), |b| {
            b.iter(|| sync_un_bench(nrepeats, ntasks))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
