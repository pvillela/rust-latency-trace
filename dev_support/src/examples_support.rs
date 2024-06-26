use latency_trace::{summary_stats, Timings};

pub fn print_summary(latencies: &Timings) {
    let sg_to_parent = latencies.span_group_to_parent();

    println!("\nSpan group parents:");

    for (span_group, parent) in sg_to_parent {
        println!("  * {:?} -> {:?}", span_group, parent);
    }

    println!("\nSummary statistics by span group:");

    for (span_group, v) in latencies.iter() {
        let summary = summary_stats(v);
        println!("  * {:?}, {:?}", span_group, summary);
    }
}
