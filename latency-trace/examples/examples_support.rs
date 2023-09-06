use latency_trace::Latencies;

pub fn print_mean_timings(latencies: &Latencies) {
    println!("\nMean timing values by span group:");

    for (span_group, v) in latencies {
        let mean_total_time = v.total_time().mean();
        let mean_active_time = v.active_time().mean();
        let total_time_count = v.total_time().len();
        let active_time_count = v.active_time().len();
        let parent = v.parent();
        println!(
                "  * {:?}, parent={:?}, mean_total_time={}μs, total_time_count={}, mean_active_time={}μs, active_time_count={}",
                span_group, parent, mean_total_time, total_time_count, mean_active_time,active_time_count
            );
    }
}

pub fn print_median_timings(latencies: &Latencies) {
    println!("\nMedian timings by span group:");

    for (span_group, v) in latencies {
        let median_total_time = v.total_time().value_at_percentile(50.0);
        let median_active_time = v.active_time().value_at_percentile(50.0);
        let total_time_count = v.total_time().len();
        let active_time_count = v.active_time().len();
        let parent = v.parent();
        println!(
                "  * {:?}, parent={:?}, median_total_time={}μs, total_time_count={}, median_active_time={}μs, active_time_count={}",
                span_group, parent, median_total_time, total_time_count, median_active_time,active_time_count
            );
    }
}

#[allow(unused)]
fn main() {}
