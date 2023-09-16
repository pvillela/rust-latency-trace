//! Example of latency measurement for a simple async function.

use latency_trace::{histogram_summary, LatencyTrace};
use std::time::{Duration, Instant};
use tracing::{instrument, trace_span, Instrument};

#[instrument(level = "trace")]
async fn f() {
    for _ in 0..1000 {
        async {
            trace_span!("empty").in_scope(|| {
                // Empty span used to show some of the tracing overhead.
            });

            // Simulated work
            tokio::time::sleep(Duration::from_millis(6)).await;

            g().await;
        }
        .instrument(trace_span!("loop_body"))
        .await
    }
}

#[instrument(level = "trace")]
async fn g() {
    // Simulated work
    tokio::time::sleep(Duration::from_millis(4)).await;
}

fn main() {
    // std::env::set_var("RUST_LOG", "latency_trace=trace");
    // _ = env_logger::try_init();

    let start = Instant::now();

    let latencies = LatencyTrace::new().measure_latencies_tokio(f);

    println!(
        "*** Elapsed time: {:?}",
        Instant::now().duration_since(start)
    );

    println!("\nLatency stats below are in microseconds");
    for (span_group, v) in latencies.timings() {
        let summary = v.map(histogram_summary);
        println!("  * {:?}, {:?}", span_group, summary);
    }
}
