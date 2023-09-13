//! Example of latency measurement for a simple sync function.

use latency_trace::{histogram_summary, LatencyTrace};
use rand::Rng;
use std::{thread, time::Duration};
use tracing::{instrument, trace_span};

#[instrument(level = "trace")]
fn f() {
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        trace_span!("empty").in_scope(|| {
            // Empty span used to measure tracing overhead.
        });

        // Simulated work
        thread::sleep(Duration::from_millis(rng.gen_range(0..=12)));

        g();
    }
}

#[instrument(level = "trace")]
fn g() {
    let mut rng = rand::thread_rng();
    // Simulated work
    thread::sleep(Duration::from_millis(rng.gen_range(0..=8)));
}

fn main() {
    let latencies = LatencyTrace::new().measure_latencies(f);
    println!("Latency stats below are in microseconds");
    for (span_group, v) in latencies.timings() {
        let summary = v.map(histogram_summary);
        println!("  * {:?}, {:?}", span_group, summary);
    }
}
