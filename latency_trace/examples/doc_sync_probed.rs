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
    let probed = LatencyTrace::activated_default()
        .unwrap()
        .measure_latencies_probed(f)
        .unwrap();

    // Let the function run for some time before probing latencies.
    thread::sleep(Duration::from_micros(16000));

    let latencies1 = probed.probe_latencies();
    let latencies2 = probed.wait_and_report();

    println!("\nlatencies1 in microseconds");
    for (span_group, stats) in latencies1.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    println!("\nlatencies2 in microseconds");
    for (span_group, stats) in latencies2.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }
}
