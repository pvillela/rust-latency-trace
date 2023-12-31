use latency_trace::LatencyTrace;
use std::{thread, time::Duration};
use tracing::{instrument, trace_span};

#[instrument(level = "trace")]
fn f() {
    for _ in 0..1000 {
        trace_span!("loop_body").in_scope(|| {
            trace_span!("empty").in_scope(|| {
                // Empty span used to show some of the tracing overhead.
            });

            // Simulated work
            thread::sleep(Duration::from_micros(6000));

            g();
        });
    }
}

#[instrument(level = "trace")]
fn g() {
    // Simulated work
    thread::sleep(Duration::from_micros(4000));
}

fn main() {
    let latencies = LatencyTrace::default().measure_latencies(f);

    println!("\nLatency stats below are in microseconds");
    for (span_group, stats) in latencies.summary_stats() {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    // A shorter way to print the summary stats, with uglier formatting.
    println!("\nDebug print of `latencies.summary_stats()`:");
    println!("{:?}", latencies.summary_stats());
}
