//! Example showing latency measurement of [dev_utils::target_fns::target_fn] using the span grouper
//! [latency_trace::group_by_given_fields].

use dev_utils::target_fns::target_fn;
use env_logger;
use latency_trace::{group_by_given_fields, LatencyTrace};
use std::env::set_var;

mod examples_support;
use examples_support::print_summary;

fn main() {
    // Set below value to "trace" to enable full library tracing.
    set_var("RUST_LOG", "info");

    let latencies = LatencyTrace::new()
        .with_span_grouper(group_by_given_fields(&["foo"]))
        .measure_latencies_tokio(|| async {
            // Set env_logger only if `tracing_subsriber` hasn't pulled in `tracing_log` and already set a logger.
            // Otherwise, setting a second logger would panic.
            _ = env_logger::try_init();

            target_fn().await;
        });

    print_summary(&latencies);
}
