//! Executes benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dev_utils::simple_fns::{
    simple_fn_async, simple_fn_async_un, simple_fn_sync, simple_fn_sync_un,
};
use latency_trace::bench_support::{
    measure_latencies1, measure_latencies2, measure_latencies2_tokio,
};
use latency_trace::LatencyTrace;

const NREPEATS: usize = 100;
const SLEEP_MICROS: u64 = 100;

fn set_up_bench() {
    let lt = LatencyTrace::default();
    measure_latencies1(lt);
}

fn sync_completion_bench(ntasks: usize) {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, move || simple_fn_sync(NREPEATS, ntasks, SLEEP_MICROS));
}

fn sync_all_in_bench(ntasks: usize) {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || simple_fn_sync(NREPEATS, ntasks, SLEEP_MICROS));
    black_box(timings);
}

fn sync_un_bench(ntasks: usize) {
    simple_fn_sync_un(NREPEATS, ntasks, SLEEP_MICROS);
}

fn async_completion_bench(ntasks: usize) {
    let lt = LatencyTrace::default();
    measure_latencies2_tokio(lt, move || simple_fn_async(NREPEATS, ntasks, SLEEP_MICROS));
}

fn async_all_in_bench(ntasks: usize) {
    let lt = LatencyTrace::default();
    let timings =
        lt.measure_latencies_tokio(move || simple_fn_async(NREPEATS, ntasks, SLEEP_MICROS));
    black_box(timings);
}

fn async_un_bench(ntasks: usize) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(simple_fn_async_un(NREPEATS, ntasks, SLEEP_MICROS));
}

fn criterion_benchmark(c: &mut Criterion) {
    for ntasks in [0, 5] {
        c.bench_function(&format!("set-up(ntasks={ntasks})"), |b| {
            b.iter(set_up_bench)
        });
        c.bench_function(&format!("sync_completion(ntasks={ntasks})"), |b| {
            b.iter(|| sync_completion_bench(ntasks))
        });
        c.bench_function(&format!("sync_all_in(ntasks={ntasks})"), |b| {
            b.iter(|| sync_all_in_bench(ntasks))
        });
        c.bench_function(&format!("sync_un(ntasks={ntasks})"), |b| {
            b.iter(|| sync_un_bench(ntasks))
        });
        c.bench_function(&format!("async_completion(ntasks={ntasks})"), |b| {
            b.iter(|| async_completion_bench(ntasks))
        });
        c.bench_function(&format!("async_all_in(ntasks={ntasks})"), |b| {
            b.iter(|| async_all_in_bench(ntasks))
        });
        c.bench_function(&format!("async_un(ntasks={ntasks})"), |b| {
            b.iter(|| async_un_bench(ntasks))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
