//! Example of latency measurement for a simple sync function.

use latency_trace::{histogram_summary, LatencyTrace};
use rand::Rng;
use std::{env::set_var, thread, time::Duration};
use tracing::{instrument, trace_span, Instrument};

fn gen_random_to(max: u64) -> u64 {
    // let mut rng = rand::thread_rng();
    // rng.gen_range(0..=max)
    max
}

#[instrument(level = "trace")]
async fn f() {
    for _ in 0..1 {
        trace_span!("empty").in_scope(|| {
            // Empty span used to measure tracing overhead.
        });

        // Simulated work
        thread::sleep(Duration::from_millis(gen_random_to(12)));

        // let h = tokio::spawn(g()).instrument(trace_span!("spawn_g_1"));
        // h.await.unwrap();

        // g().await;
        // g().await;

        let h = tokio::spawn(g()).instrument(trace_span!("spawn_g_2"));
        g().await;
        h.await.unwrap();

        // g().await;
    }
}

#[instrument(level = "trace")]
async fn g() {
    println!("g before tokio sleep");
    tokio::time::sleep(Duration::from_millis(gen_random_to(4))).await;
    println!("g after tokio sleep");
    // Simulated work
    thread::sleep(Duration::from_millis(gen_random_to(4)));
}

fn main() {
    set_var("RUST_LOG", "latency_trace=trace");
    _ = env_logger::try_init();

    let latencies = LatencyTrace::new().measure_latencies_tokio(f);
    println!("Latency stats below are in microseconds");
    for (span_group, v) in latencies.timings() {
        let summary = v.map(histogram_summary);
        println!("  * {:?}, {:?}", span_group, summary);
    }
}
