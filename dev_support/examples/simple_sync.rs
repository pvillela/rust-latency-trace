//! Example of latency measurement for a simple sync function.

use dev_support::{examples_support::cmd_line_args, simple_fns::simple_sync};
use latency_trace::{summary_stats, LatencyTrace};
use std::time::Instant;

fn main() {
    // std::env::set_var("RUST_LOG", "latency_trace=trace");
    // _ = env_logger::try_init();

    // Get args from command line or use defaults below.
    let (nrepeats, ntasks, sleep_micros) = cmd_line_args().unwrap_or((100, 0, Some(100)));

    println!(
        "\n=== {} {:?} ===========================================================",
        std::env::args().next().unwrap(),
        cmd_line_args()
    );

    let start = Instant::now();
    let latencies = LatencyTrace::default()
        .measure_latencies(|| simple_sync(nrepeats, ntasks, sleep_micros.unwrap()))
        .unwrap();
    println!("Elapsed time: {:?}", Instant::now().duration_since(start));

    println!("\nDebug print of `latencies.map_values(summary_stats)`:");
    println!("{:?}", latencies.map_values(summary_stats));
}
