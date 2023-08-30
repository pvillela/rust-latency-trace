use latency_trace::Latencies;
use log;
use std::{thread, time::Duration};
use tracing::{info, instrument, warn, Instrument};

#[instrument(level = "trace")]
pub async fn f() {
    let mut foo: u64 = 1;

    for i in 0..8 {
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
        .instrument(tracing::trace_span!(
            "my_great_span",
            foo = i % 2,
            bar = i % 4
        ))
        .await
    }
}

pub fn print_mean_timings(latencies: &Latencies) {
    latencies.with(|info| {
        println!("\nMean timing values by span group:");

        let parents = &info.parents;

        for (span_group, v) in info.timings.iter() {
            let mean_total_time = v.total_time.mean();
            let mean_active_time = v.active_time.mean();
            let total_time_count = v.total_time.len();
            let active_time_count = v.active_time.len();
            let parent = parents.get(span_group.callsite_id()).unwrap();
            println!(
                "  * span_group={:?}, parent={:?}, mean_total_time={}μs, total_time_count={}, mean_active_time={}μs, active_time_count={}",
                span_group, parent, mean_total_time, total_time_count, mean_active_time,active_time_count
            );
        }
    });
}

pub fn print_median_timings(latencies: &Latencies) {
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

#[allow(unused)]
fn main() {}