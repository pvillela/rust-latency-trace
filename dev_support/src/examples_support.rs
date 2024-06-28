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

/// Returns command line arguments (`nrepeats`, `ntasks`, `sleep_micros`) for use by example functions.
pub fn cmd_line_args() -> Option<(usize, usize, Option<u64>)> {
    let nrepeats = match std::env::args().nth(1) {
        Some(v) => v
            .parse::<usize>()
            .expect("1st argument, if provided, must be integer"),
        None => return None,
    };

    let ntasks = std::env::args()
        .nth(2)
        .expect("2nd argument must be provided")
        .parse::<usize>()
        .expect("2nd argument must be an integer");

    let sleep_micros = std::env::args().nth(3).map(|v| {
        v.parse::<u64>()
            .expect("3rd argument, if provided, must be integer")
    });

    Some((nrepeats, ntasks, sleep_micros))
}
