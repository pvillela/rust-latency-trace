//! Example of latency measurement for a simple async function.

use latency_trace::{histogram_summary, LatencyTrace};
use rand::Rng;
use std::{thread, time::Duration};
use tracing::{instrument, trace_span};

fn gen_random_to(max: u64) -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(0..=max)
}

#[instrument(level = "trace")]
async fn f() {
    for _ in 0..100 {
        trace_span!("empty").in_scope(|| {
            // Empty span used to measure tracing overhead.
        });

        // Simulated work
        thread::sleep(Duration::from_millis(gen_random_to(12)));

        g().await;
    }
}

#[instrument(level = "trace")]
async fn g() {
    // Simulated work
    thread::sleep(Duration::from_millis(gen_random_to(8)));
}

fn main() {
    // set_var("RUST_LOG", "latency_trace=trace");
    // _ = env_logger::try_init();

    let latencies = LatencyTrace::new().measure_latencies_tokio(f);
    println!("Latency stats below are in microseconds");
    for (span_group, v) in latencies.timings() {
        let summary = v.map(histogram_summary);
        println!("  * {:?}, {:?}", span_group, summary);
    }
}
