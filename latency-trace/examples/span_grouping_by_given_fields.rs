//! Example using custom grouping by given fields.

use dev_utils::target_fns::target_fn;
use env_logger;
use latency_trace::{group_by_given_fields, measure_latencies_with_custom_grouping_tokio};
use std::env::set_var;

mod examples_support;
use examples_support::print_parents_means_medians;

fn main() {
    // Set below value to "trace" to enable full library tracing.
    set_var("RUST_LOG", "info");

    let latencies =
        measure_latencies_with_custom_grouping_tokio(group_by_given_fields(&["foo"]), || async {
            // Set env_logger only if `tracing_subsriber` hasn't pulled in `tracing_log` and already set a logger.
            // Otherwise, setting a second logger would panic.
            _ = env_logger::try_init();

            target_fn().await;
        });

    print_parents_means_medians(&latencies);
}
