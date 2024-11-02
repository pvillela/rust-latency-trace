//! This example does not use [`LatencyTrace`](latency_trace::LatencyTrace) but is included for comparison with
//! examples `doc_sync_layered` and `doc_sync`.

use std::{thread, time::Duration};
use tracing::{info, instrument, trace, trace_span};
use tracing_subscriber::fmt::format::FmtSpan;

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
    // Instantiate a `tracing_subscriber::fmt::Subscriber`.
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_span_events(FmtSpan::FULL)
        .with_max_level(tracing::Level::TRACE)
        .finish();

    // Set the subscriber as the default
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    // Trace.
    f();
}
