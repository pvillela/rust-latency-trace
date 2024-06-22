use latency_trace::{summary_stats, LatencyTrace};
use std::time::Duration;
use tracing::{instrument, trace_span, Instrument};

#[instrument(level = "trace")]
async fn f() {
    for _ in 0..1000 {
        async {
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
    let latencies = LatencyTrace::default().measure_latencies_tokio(f);

    println!("\nLatency stats below are in microseconds");
    for (span_group, stats) in latencies.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    // A shorter way to print the summary stats, with uglier formatting.
    println!("\nDebug print of `latencies.map_values(summary_stats)`:");
    println!("{:?}", latencies.map_values(summary_stats));
}
