use latency_trace::{summary_stats, LatencyTrace};
use std::{thread, time::Duration};
use tracing::{info, instrument, trace, trace_span};

#[instrument(level = "info")]
fn f() {
    trace!("in f");
    for _ in 0..10 {
        trace_span!("loop_body").in_scope(|| {
            info!("in loop body");
            // Simulated work
            thread::sleep(Duration::from_micros(1200));

            g();
        });
    }
}

#[instrument(level = "info")]
fn g() {
    trace!("in g");
    // Simulated work
    thread::sleep(Duration::from_micros(800));
}

fn main() {
    let latencies = LatencyTrace::activated_default()
        .unwrap()
        .measure_latencies(f);

    println!("\nLatency stats below are in microseconds");
    for (span_group, stats) in latencies.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    // A shorter way to print the summary stats, with uglier formatting.
    println!("\nDebug print of `latencies.map_values(summary_stats)`:");
    println!("{:?}", latencies.map_values(summary_stats));
}
