//! Example of latency measurement for a simple sync function.

use latency_trace::LatencyTrace;
use std::{
    thread,
    time::{Duration, Instant},
};
use tracing::{instrument, trace_span};

/// Returns command line argument or default
fn arg() -> u64 {
    match std::env::args().nth(1) {
        Some(v) => u64::from_str_radix(&v, 10).expect("argument must be integer"),
        None => 2000,
    }
}

#[instrument(level = "trace")]
fn f() {
    for _ in 0..1000 {
        trace_span!("loop_body").in_scope(|| {
            trace_span!("empty").in_scope(|| {
                // Empty span used to show some of the tracing overhead.
            });

            // Simulated work
            thread::sleep(Duration::from_micros(arg() * 3));

            g();
        });
    }
}

#[instrument(level = "trace")]
fn g() {
    // Simulated work
    thread::sleep(Duration::from_micros(arg() * 2));
}

fn main() {
    // std::env::set_var("RUST_LOG", "latency_trace=trace");
    // _ = env_logger::try_init();

    let start = Instant::now();

    let pausable = LatencyTrace::default().measure_latencies_pausable(|| {
        thread::scope(|s| {
            for _ in 0..5 {
                s.spawn(f);
            }
        });
    });
    thread::sleep(Duration::from_micros(arg() * 12));
    let latencies1 = pausable.pause_and_collect();
    let latencies2 = pausable.wait_and_collect();

    println!(
        "\n=== {} {} ===========================================================",
        std::env::args().nth(0).unwrap(),
        arg()
    );
    println!("Elapsed time: {:?}", Instant::now().duration_since(start));

    println!("\nlatencies1 in microseconds");
    for (span_group, stats) in latencies1.summary_stats() {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    println!("\nlatencies2 in microseconds");
    for (span_group, stats) in latencies2.summary_stats() {
        println!("  * {:?}, {:?}", span_group, stats);
    }
}
