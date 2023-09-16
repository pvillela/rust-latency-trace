//! Example of latency measurement for a simple sync function.

use latency_trace::{histogram_summary, LatencyTrace};
use std::{hint::black_box, thread, time::Duration};
use tracing::{instrument, trace_span};

#[instrument(level = "trace")]
fn f() {
    for i in 0..1000 {
        trace_span!("loop_body").in_scope(|| {
            trace_span!("empty").in_scope(|| {
                // Empty span used to measure tracing overhead.
                black_box(i);
            });

            // Simulated work
            thread::sleep(Duration::from_millis(6));

            black_box(g(i));
        });
    }
}

#[instrument(level = "trace")]
fn g(i: i32) -> i32 {
    // Simulated work
    black_box(i);
    thread::sleep(Duration::from_millis(4));
    black_box(i)
}

fn main() {
    let latencies = LatencyTrace::new().measure_latencies(f);
    println!("Latency stats below are in microseconds");
    for (span_group, v) in latencies.timings() {
        let summary = v.map(histogram_summary);
        println!("  * {:?}, {:?}", span_group, summary);
    }
}
