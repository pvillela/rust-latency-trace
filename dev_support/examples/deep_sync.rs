//! Example of latency measurement for a simple sync function that does nothing,
//! to demonstrate the overhead associated with tracing and the framework.
//!
//! The nested spans with no other significant executable code, other than the loop and function call,
//! provide visibility to the overhead of span creation and processing.

use dev_support::{deep_fns::deep_sync, examples_support::cmd_line_args};
use latency_trace::{summary_stats, LatencyTrace};
use std::time::Instant;

fn main() {
    // std::env::set_var("RUST_LOG", "latency_trace=trace");
    // _ = env_logger::try_init();

    // Get args from command line or use defaults below.
    let (nrepeats, ntasks, _) = cmd_line_args().unwrap_or((100, 0, None));

    println!(
        "\n=== {} {:?} ===========================================================",
        std::env::args().next().unwrap(),
        cmd_line_args()
    );

    let start = Instant::now();
    let latencies = LatencyTrace::default()
        .measure_latencies(|| deep_sync(nrepeats, ntasks))
        .unwrap();
    println!("Elapsed time: {:?}", Instant::now().duration_since(start));

    println!("\nDebug print of `latencies.map_values(summary_stats)`:");
    println!("{:?}", latencies.map_values(summary_stats));
}
