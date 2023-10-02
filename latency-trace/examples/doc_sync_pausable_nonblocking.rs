use latency_trace::{summary_stats, BTreeMapExt, LatencyTrace, PausableMode};
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
            thread::sleep(Duration::from_micros(1200));

            g();
        });
    }
}

#[instrument(level = "trace")]
fn g() {
    // Simulated work
    thread::sleep(Duration::from_micros(800));
}

fn main() {
    let pausable = LatencyTrace::default().measure_latencies_pausable(PausableMode::Nonblocking, f);
    thread::sleep(Duration::from_micros(4800));
    let latencies1 = pausable.pause_and_report();
    let latencies2 = pausable.wait_and_report();

    println!("\nlatencies1 in microseconds");
    for (span_group, stats) in latencies1.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    println!("\nlatencies2 in microseconds");
    for (span_group, stats) in latencies2.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }
}