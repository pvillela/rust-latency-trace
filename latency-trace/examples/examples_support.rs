use latency_trace::Latencies;

pub fn print_parents_means_medians(latencies: &Latencies) {
    println!("\nSpan group parents:");

    for span_group in latencies.span_groups() {
        let parent = span_group
            .parent_idx()
            .map(|pidx| &latencies.span_groups()[pidx]);
        println!("  * {:?} -> {:?}", span_group, parent);
    }

    println!("\nMean timing values by span group:");

    for (span_group, v) in latencies.timings() {
        let mean_total_time = v.total_time().mean();
        let mean_active_time = v.active_time().mean();
        let total_time_count = v.total_time().len();
        let active_time_count = v.active_time().len();
        println!(
                "  * {:?}, mean_total_time={}μs, total_time_count={}, mean_active_time={}μs, active_time_count={}",
                span_group, mean_total_time, total_time_count, mean_active_time,active_time_count
            );
    }

    println!("\nMedian timings by span group:");

    for (span_group, v) in latencies.timings() {
        let median_total_time = v.total_time().value_at_percentile(50.0);
        let median_active_time = v.active_time().value_at_percentile(50.0);
        let total_time_count = v.total_time().len();
        let active_time_count = v.active_time().len();
        println!(
                "  * {:?}, median_total_time={}μs, total_time_count={}, median_active_time={}μs, active_time_count={}",
                span_group, median_total_time, total_time_count, median_active_time,active_time_count
            );
    }
}

#[allow(unused)]
fn main() {}
