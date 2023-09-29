use latency_trace::{histogram_summary, Latencies};

pub fn print_summary(latencies: &Latencies) {
    println!("\nSpan group parents:");

    for span_group in latencies.span_groups() {
        let parent = span_group
            .parent_idx()
            .map(|pidx| &latencies.span_groups()[pidx]);
        println!("  * {:?} -> {:?}", span_group, parent);
    }

    println!("\nSummary statistics by span group:");

    for (span_group, v) in latencies.timings() {
        let summary = histogram_summary(v);
        println!("  * {:?}, {:?}", span_group, summary);
    }
}

#[allow(unused)]
fn main() {}
