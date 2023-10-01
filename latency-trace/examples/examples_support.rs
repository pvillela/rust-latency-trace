use std::collections::BTreeMap;

use latency_trace::{summary_stats, SpanGroup, Timings};

pub fn print_summary(latencies: &Timings) {
    let id_to_span_group: BTreeMap<u64, SpanGroup> =
        latencies.keys().map(|k| (k.id(), k.clone())).collect();

    println!("\nSpan group parents:");

    for span_group in latencies.keys() {
        let parent = span_group
            .parent_id()
            .map(|pid| id_to_span_group.get(&pid).unwrap());
        println!("  * {:?} -> {:?}", span_group, parent);
    }

    println!("\nSummary statistics by span group:");

    for (span_group, v) in latencies {
        let summary = summary_stats(v);
        println!("  * {:?}, {:?}", span_group, summary);
    }
}

#[allow(unused)]
fn main() {}
