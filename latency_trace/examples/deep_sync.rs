//! Example of latency measurement for a simple sync function that does nothing,
//! to demonstrate the overhead associated with tracing and the framework.
//!
//! The nested spans with no other significant executable code, other than the loop and function call,
//! provide visibility to the overhead of span creation and processing, which is ~0.5-1 microseconds
//! per span instance on my 2022 Dell Inspiron 16.

use criterion::black_box;
use dev_support::deep_fns::deep_sync;
use latency_trace::LatencyTrace;
use std::time::Instant;

fn sync_all_in_bench(nrepeats: usize, ntasks: usize) {
    let lt = LatencyTrace::default();
    let timings = lt.measure_latencies(move || deep_sync(nrepeats, ntasks));
    black_box(timings);
}

fn main() {
    // std::env::set_var("RUST_LOG", "latency_trace=trace");
    // _ = env_logger::try_init();

    let start = Instant::now();

    // let latencies = LatencyTrace::default().measure_latencies(|| deep_sync(1000, 5));
    sync_all_in_bench(1000, 5);

    println!(
        "*** Elapsed time: {:?}",
        Instant::now().duration_since(start)
    );

    // println!("\nLatency stats below are in microseconds");
    // for (span_group, stats) in latencies.map_values(summary_stats) {
    //     println!("  * {:?}, {:?}", span_group, stats);
    // }
}
