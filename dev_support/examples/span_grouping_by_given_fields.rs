//! Example showing latency measurement of [dev_support::target_fns::target_fn] using the span grouper
//! [latency_trace::group_by_given_fields].

use dev_support::{elab_fns::elab_async, examples_support::print_summary};
use latency_trace::{group_by_given_fields, LatencyTraceOld};
use std::env::set_var;

fn main() {
    // Set below value to "trace" to enable full library tracing.
    set_var("RUST_LOG", "info");

    let latencies = LatencyTraceOld::default()
        .with_span_grouper(group_by_given_fields(&["foo"]))
        .measure_latencies_tokio(|| async {
            // Set env_logger only if `tracing_subsriber` hasn't pulled in `tracing_log` and already set a logger.
            // Otherwise, setting a second logger would panic.
            _ = env_logger::try_init();

            elab_async().await;
        })
        .unwrap();

    print_summary(&latencies);
}
