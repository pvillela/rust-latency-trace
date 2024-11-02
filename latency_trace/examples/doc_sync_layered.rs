//! Demonstrates how to use [`LatencyTrace`] as a [`Layer`](tracing_subscriber::layer::Layer)
//! that can be composed with other layers to make a [`Subscriber`](tracing::Subscriber).
//!
//! Compare with examples `doc_sync` and `doc_sync_fmt`.

use latency_trace::{summary_stats, LatencyTrace};
use std::{thread, time::Duration};
use tracing::{info, instrument, level_filters::LevelFilter, trace, trace_span};
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*, Registry};

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
    // LatencyTrace instance from which latency statistics will be extracted later.
    let lt = LatencyTrace::default();

    // Clone of the above that will be used as a `tracing_subscriber::layer::Layer` that can be
    // composed with other tracing layers.
    let ltl = lt.clone().with_filter(LevelFilter::INFO);

    // `tracing_subscriber::fmt::Layer` that can be composed with the above `LatencyTrace` layer.
    let tfmt = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::FULL)
        .with_filter(LevelFilter::INFO);

    // Instantiate a layered subscriber and set it as the global default.
    let layered = Registry::default().with(ltl).with(tfmt);
    layered.init();

    // Measure latencies.
    let latencies = lt.measure_latencies(f);

    println!("\nLatency stats below are in microseconds");
    for (span_group, stats) in latencies.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    // A shorter way to print the summary stats, with uglier formatting.
    println!("\nDebug print of `latencies.map_values(summary_stats)`:");
    println!("{:?}", latencies.map_values(summary_stats));
}
