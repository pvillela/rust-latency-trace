//! Example using custom grouping by given fields.

use env_logger;
use latency_trace::{group_by_given_fields, measure_latencies_with_custom_grouping_tokio};
use std::env::set_var;

mod examples_grouping_helper;
use examples_grouping_helper::f;

fn main() {
    // Set below value to "trace" to enable full library tracing.
    set_var("RUST_LOG", "info");

    let latencies =
        measure_latencies_with_custom_grouping_tokio(group_by_given_fields(&["foo"]), || async {
            // Set env_logger only if `tracing_subsriber` hasn't pulled in `tracing_log` and already set a logger.
            // Otherwise, setting a second logger would panic.
            _ = env_logger::try_init();

            let h1 = tokio::spawn(f());
            let h2 = tokio::spawn(f());
            _ = h1.await;
            _ = h2.await;
        });

    latencies.print_mean_timings();

    latencies.with(|info| {
        println!("\nMedian timings by span group:");

        let parents = &info.parents;

        for (span_group, v) in info.timings.iter() {
            let median_total_time = v.total_time.value_at_percentile(50.0);
            let median_active_time = v.active_time.value_at_percentile(50.0);
            let total_time_count = v.total_time.len();
            let active_time_count = v.active_time.len();
            let parent = parents.get(span_group.callsite_id()).unwrap();
            println!(
                "  * span_group={:?}, parent={:?}, median_total_time={}μs, total_time_count={}, median_active_time={}μs, active_time_count={}",
                span_group, parent, median_total_time, total_time_count, median_active_time,active_time_count
            );
        }
    });
}
