//! This captures both total and sync timings:
//! - total timings include suspend time and are based on span creation and closing;
//! - active timings exclude suspend time and are based on span entry and exit.

use dev_utils::target_fns::target_fn;
use env_logger;
use latency_trace::measure_latencies_tokio;
use std::env::set_var;

mod examples_support;
use examples_support::{print_mean_timings, print_median_timings};

fn main() {
    // Set below value to "trace" to enable full library tracing.
    set_var("RUST_LOG", "info");

    let latencies = measure_latencies_tokio(|| async {
        // Set env_logger only if `tracing_subsriber` hasn't pulled in `tracing_log` and already set a logger.
        // Otherwise, setting a second logger would panic.
        _ = env_logger::try_init();

        target_fn().await;
    });

    print_mean_timings(&latencies);

    print_median_timings(&latencies);
}
