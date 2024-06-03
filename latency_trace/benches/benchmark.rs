//! Executes benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dev_utils::simple_fns::{
    simple_fn_async, simple_fn_async_un, simple_fn_sync, simple_fn_sync_un,
};
use latency_trace::bench_support::{
    measure_latencies1, measure_latencies2, measure_latencies2_tokio,
};
use latency_trace::LatencyTrace;

const REPEATS: usize = 100;
const SLEEP_MICROS: u64 = 100;

fn set_up_bench() {
    let lt = LatencyTrace::default();
    measure_latencies1(lt);
}

fn sync_completion_bench() {
    let lt = LatencyTrace::default();
    measure_latencies2(lt, || simple_fn_sync(REPEATS, SLEEP_MICROS));
}

fn sync_all_in_bench() {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(|| simple_fn_sync(REPEATS, SLEEP_MICROS));
    black_box(timings);
}

fn sync_un_bench() {
    simple_fn_sync_un(REPEATS, SLEEP_MICROS);
}

fn async_completion_bench() {
    let lt = LatencyTrace::default();
    measure_latencies2_tokio(lt, || simple_fn_async(REPEATS, SLEEP_MICROS));
}

fn async_all_in_bench() {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies_tokio(|| simple_fn_async(REPEATS, SLEEP_MICROS));
    black_box(timings);
}

fn async_un_bench() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(simple_fn_async_un(REPEATS, SLEEP_MICROS));
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("set-up", |b| b.iter(set_up_bench));
    c.bench_function("sync_completion", |b| b.iter(sync_completion_bench));
    c.bench_function("sync_all_in", |b| b.iter(sync_all_in_bench));
    c.bench_function("sync_un", |b| b.iter(sync_un_bench));
    c.bench_function("async_completion", |b| b.iter(async_completion_bench));
    c.bench_function("async_all_in", |b| b.iter(async_all_in_bench));
    c.bench_function("async_un", |b| b.iter(async_un_bench));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
