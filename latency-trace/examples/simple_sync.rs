//! Example of latency measurement for a simple sync function.

use latency_trace::{histogram_summary, LatencyTrace};
use std::{
    thread,
    time::{Duration, Instant},
};
use tracing::{instrument, trace_span};

#[instrument(level = "trace")]
fn f() {
    for _ in 0..1000 {
        trace_span!("loop_body").in_scope(|| {
            trace_span!("empty").in_scope(|| {
                // Empty span used to show some of the tracing overhead.
            });

            // Simulated work
            thread::sleep(Duration::from_millis(6));

            g();
        });
    }
}

#[instrument(level = "trace")]
fn g() {
    // Simulated work
    thread::sleep(Duration::from_millis(4));
}

fn main() {
    // std::env::set_var("RUST_LOG", "latency_trace=trace");
    // _ = env_logger::try_init();

    let start = Instant::now();

    let latencies = LatencyTrace::new().measure_latencies(f);

    println!(
        "*** Elapsed time: {:?}",
        Instant::now().duration_since(start)
    );

    println!("\nLatency stats below are in microseconds");
    for (span_group, v) in latencies.timings() {
        let summary = v.map(histogram_summary);
        println!("  * {:?}, {:?}", span_group, summary);
    }

    // A shorter way to print the summary stats, with uglier formatting.
    println!("\nSummary stats in microseconds mapped directly from `latencies.timings():`");
    println!("{:?}", latencies.timings().map_values(histogram_summary));
}
