//! This captures both total and sync timings:
//! - total timings include suspend time and are based on span creation and closing;
//! - active timings exclude suspend time and are based on span entry and exit.

use env_logger;
use latency_trace::measure_latencies_tokio;
use log;
use std::{env::set_var, thread, time::Duration};
use tracing::{info, instrument, warn, Instrument};

#[instrument(level = "trace")]
async fn f() {
    let mut foo: u64 = 1;

    for i in 0..4 {
        log::trace!("Before my_great_span");

        async {
            thread::sleep(Duration::from_millis(3));
            tokio::time::sleep(Duration::from_millis(100)).await;
            foo += 1;
            info!(yak_shaved = true, yak_count = 2, "hi from inside my span");
            log::trace!("Before my_other_span");
            async {
                thread::sleep(Duration::from_millis(2));
                tokio::time::sleep(Duration::from_millis(25)).await;
                warn!(yak_shaved = false, yak_count = -1, "failed to shave yak");
            }
            .instrument(tracing::trace_span!("my_other_span", foo = i % 2))
            .await;
        }
        .instrument(tracing::trace_span!("my_great_span", bar = (i + 1) % 2))
        .await
    }
}

fn main() {
    // Set below value to "trace" to enable full library tracing.
    set_var("RUST_LOG", "info");

    let latencies = measure_latencies_tokio(|| async {
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
                "  span_group={:?}, parent={:?}, median_total_time={}μs, total_time_count={}, median_active_time={}μs, active_time_count={}",
                span_group, parent, median_total_time, total_time_count, median_active_time,active_time_count
            );
        }
    });
}
