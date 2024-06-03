//! Example showing latency measurement of [dev_utils::target_fns::target_fn] with default span grouping.

use dev_utils::elab_fns::elab_fn_async;
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

        elab_fn_async().await;
    });

    print_summary(&latencies);
}
