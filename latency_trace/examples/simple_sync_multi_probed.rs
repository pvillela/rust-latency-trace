//! Example of probed latency measurement for a simple sync function.

use latency_trace::{summary_stats, LatencyTrace};
use std::{
    thread,
    time::{Duration, Instant},
};
use tracing::{instrument, trace_span};

/// Returns command line argument or default
fn arg() -> u64 {
    match std::env::args().nth(1) {
        Some(v) => v.parse::<u64>().expect("argument must be integer"),
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

    let probed = LatencyTrace::default().measure_latencies_probed(|| {
        thread::scope(|s| {
            for _ in 0..5 {
                s.spawn(f);
            }
        });
    });
    thread::sleep(Duration::from_micros(arg() * 12));
    let latencies1 = probed.probe_latencies();
    let latencies2 = probed.wait_and_report();

    println!(
        "\n=== {} {} ===========================================================",
        std::env::args().next().unwrap(),
        arg()
    );
    println!("Elapsed time: {:?}", Instant::now().duration_since(start));

    println!("\nlatencies1 in microseconds");
    for (span_group, stats) in latencies1.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    println!("\nlatencies2 in microseconds");
    for (span_group, stats) in latencies2.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }
}
