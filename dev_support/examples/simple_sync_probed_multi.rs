//! Example of probed latency measurement for a simple sync function. Same as example `simple_sync_probed`,
//! but the default args set `ntasks > 0` (single thread).

use dev_support::{examples_support::cmd_line_args, simple_fns::simple_sync};
use latency_trace::{summary_stats, LatencyTrace};
use std::{
    thread,
    time::{Duration, Instant},
};

fn main() {
    // std::env::set_var("RUST_LOG", "latency_trace=trace");
    // _ = env_logger::try_init();

    // Get args from command line or use defaults below.
    let (nrepeats, ntasks, sleep_micros) = cmd_line_args().unwrap_or((100, 5, Some(100)));

    println!(
        "\n=== {} {:?} ===========================================================",
        std::env::args().next().unwrap(),
        cmd_line_args()
    );

    let start = Instant::now();

    let probed = LatencyTrace::default()
        .measure_latencies_probed(move || simple_sync(nrepeats, ntasks, sleep_micros.unwrap()));

    // Let the function run for some time before probing latencies.
    thread::sleep(Duration::from_micros(
        sleep_micros.unwrap() * (nrepeats / 3) as u64,
    ));

    let latencies1 = probed.probe_latencies();
    let latencies2 = probed.wait_and_report();

    println!("Elapsed time: {:?}", Instant::now().duration_since(start));

    println!("\nlatencies1 in microseconds");
    for (span_group, stats) in latencies1.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }

    println!("\nlatencies2 in microseconds");
    for (span_group, stats) in latencies2.map_values(summary_stats) {
        println!("  * {:?}, {:?}", span_group, stats);
    }
}
