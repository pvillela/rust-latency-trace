use latency_trace::LatencyTrace;
use std::time::Duration;
use tracing::{instrument, trace_span, Instrument};

#[instrument(level = "trace")]
async fn f() {
    for _ in 0..1000 {
        async {
            trace_span!("empty").in_scope(|| {
                // Empty span used to show some of the tracing overhead.
            });

            // Simulated work
            tokio::time::sleep(Duration::from_micros(6000)).await;

            g().await;
        }
        .instrument(trace_span!("loop_body"))
        .await
    }
}

#[instrument(level = "trace")]
async fn g() {
    // Simulated work
    tokio::time::sleep(Duration::from_micros(4000)).await;
}

fn main() {
    let latencies = LatencyTrace::default().measure_latencies_tokio(f);

    println!("\nLatency stats below are in microseconds");
    for (span_group, stats) in latencies.summary_stats() {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    // A shorter way to print the summary stats, with uglier formatting.
    println!("\nDebug print of `latencies.summary_stats()`:");
    println!("{:?}", latencies.summary_stats());
}
