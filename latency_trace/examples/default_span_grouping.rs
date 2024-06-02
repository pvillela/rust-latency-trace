//! Example showing latency measurement of [dev_utils::target_fns::target_fn] with default span grouping.

use dev_utils::target_fns::target_fn;
use latency_trace::LatencyTrace;
use std::env::set_var;

mod examples_support;
use examples_support::print_summary;

fn main() {
    // Set below value to "trace" to enable full library tracing.
    set_var("RUST_LOG", "info");

    let latencies = LatencyTrace::default().measure_latencies_tokio(|| async {
        // Set env_logger only if `tracing_subsriber` hasn't pulled in `tracing_log` and already set a logger.
        // Otherwise, setting a second logger would panic.
        _ = env_logger::try_init();

        target_fn().await;
    });

    print_summary(&latencies);
}
