//! Executes benchmarks.

use std::fmt::Display;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dev_utils::simple_fns::{
    simple_fn_async, simple_fn_async_un, simple_fn_sync, simple_fn_sync_un,
};
use latency_trace::bench_support::{
    measure_latencies1, measure_latencies2, measure_latencies2_tokio,
};
use latency_trace::LatencyTrace;

fn set_up_bench() {
    let lt = LatencyTrace::default();
    measure_latencies1(lt);
}

fn sync_completion_bench(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, move || simple_fn_sync(nrepeats, ntasks, sleep_micros));
}

fn sync_all_in_bench(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || simple_fn_sync(nrepeats, ntasks, sleep_micros));
    black_box(timings);
}

fn sync_un_bench(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    simple_fn_sync_un(nrepeats, ntasks, sleep_micros);
}

fn async_completion_bench(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::default();
    measure_latencies2_tokio(lt, move || simple_fn_async(nrepeats, ntasks, sleep_micros));
}

fn async_all_in_bench(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    let lt = LatencyTrace::default();
    let timings =
        lt.measure_latencies_tokio(move || simple_fn_async(nrepeats, ntasks, sleep_micros));
    black_box(timings);
}

fn async_un_bench(nrepeats: usize, ntasks: usize, sleep_micros: u64) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(simple_fn_async_un(nrepeats, ntasks, sleep_micros));
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

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function(&format!("set-up"), |b| b.iter(set_up_bench));

    for params in [
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
    ] {
        let Params {
            nrepeats,
            ntasks,
            sleep_micros,
        } = params;

        c.bench_function(&format!("sync_completion({params})"), |b| {
            b.iter(|| sync_completion_bench(nrepeats, ntasks, sleep_micros))
        });
        c.bench_function(&format!("sync_all_in({params})"), |b| {
            b.iter(|| sync_all_in_bench(nrepeats, ntasks, sleep_micros))
        });
        c.bench_function(&format!("sync_un({params})"), |b| {
            b.iter(|| sync_un_bench(nrepeats, ntasks, sleep_micros))
        });
        c.bench_function(&format!("async_completion({params})"), |b| {
            b.iter(|| async_completion_bench(nrepeats, ntasks, sleep_micros))
        });
        c.bench_function(&format!("async_all_in({params})"), |b| {
            b.iter(|| async_all_in_bench(nrepeats, ntasks, sleep_micros))
        });
        c.bench_function(&format!("async_un({params})"), |b| {
            b.iter(|| async_un_bench(nrepeats, ntasks, sleep_micros))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
