use latency_trace::{summary_stats, LatencyTrace};
use std::{thread, time::Duration};
use tracing::{instrument, trace_span, Instrument};

#[instrument(level = "trace")]
async fn f() {
    for _ in 0..1000 {
        async {
            trace_span!("empty").in_scope(|| {
                // Empty span used to show some of the tracing overhead.
            });

            // Simulated work
            tokio::time::sleep(Duration::from_micros(1200)).await;

            g().await;
        }
        .instrument(trace_span!("loop_body"))
        .await
    }
}

#[instrument(level = "trace")]
async fn g() {
    // Simulated work
    tokio::time::sleep(Duration::from_micros(800)).await;
}

fn main() {
    let probed = LatencyTrace::default().measure_latencies_probed_tokio(f);
    thread::sleep(Duration::from_micros(48000));
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
