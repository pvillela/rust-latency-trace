use std::{thread, time::Duration};
use tracing::{instrument, trace, trace_span};
use tracing_subscriber::fmt::format::FmtSpan;

#[instrument(level = "trace")]
fn f() {
    trace!("in f");
    for _ in 0..10 {
        trace_span!("loop_body").in_scope(|| {
            trace!("in loop body");
            // Simulated work
            thread::sleep(Duration::from_micros(1200));

            g();
        });
    }
}

#[instrument(level = "trace")]
fn g() {
    trace!("in g");
    // Simulated work
    thread::sleep(Duration::from_micros(800));
}

fn main() {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_span_events(FmtSpan::FULL)
        .with_max_level(tracing::Level::TRACE)
        .finish();

    // Set the subscriber as the default
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    // Trace.
    f();
}
