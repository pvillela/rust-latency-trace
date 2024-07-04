use dev_support::simple_fns::simple_real_sync;
use hdrhistogram::Histogram;
use latency_trace::{summary_stats, LatencyTraceE};
use std::{hint::black_box, time::Instant};

/// Returns command line arguments (`outer_repeats`, `inner_repeats`, `ntasks`, `extent`).
fn cmd_line_args() -> Option<(usize, usize, usize, usize, u64)> {
    let mut args = std::env::args();

    let outer_loop = args
        .nth(1)?
        .parse::<usize>()
        .expect("1st argument (`outer_repeats`), must be integer");

    let inner_loop = args
        .next()
        .expect("4 more integer arguments must be provided")
        .parse::<usize>()
        .expect("2nd argument (`inner_repeats`), must be integer");

    let nrepeats = args
        .next()
        .expect("3 more integer arguments must be provided")
        .parse::<usize>()
        .expect("3rd argument (`inner_repeats`), must be integer");

    let ntasks = args
        .next()
        .expect("2 more integer arguments must be provided")
        .parse::<usize>()
        .expect("4th argument (`ntasks`), must be integer");

    let extent = args
        .next()
        .expect("1 more integer argument must be provided")
        .parse::<u64>()
        .expect("5th argument (`extent`), must be integer");

    Some((outer_loop, inner_loop, nrepeats, ntasks, extent))
}

fn main() {
    let (outer_loop, inner_loop, nrepeats, ntasks, extent) =
        cmd_line_args().unwrap_or((20, 10, 100, 5, 20_000));

    let mut hist_neg = Histogram::<u64>::new_with_bounds(1, 20 * 1000 * 1000, 2).unwrap();
    let mut hist_pos = Histogram::<u64>::new_from(&hist_neg);

    let lt = LatencyTraceE::activated_default().unwrap();

    for _ in 0..outer_loop {
        LatencyTraceE::select_probed();
        let start_probed = Instant::now();
        for _ in 0..inner_loop {
            let timings = lt.measure_latencies(|| simple_real_sync(nrepeats, ntasks, extent));
            black_box(timings);
        }
        let elapsed_probed = Instant::now().duration_since(start_probed);
        let elapsed_probed_micros = elapsed_probed.as_micros() as i128;

        LatencyTraceE::select_joined();
        let start_joined = Instant::now();
        for _ in 0..inner_loop {
            let timings = lt.measure_latencies(|| simple_real_sync(nrepeats, ntasks, extent));
            black_box(timings);
        }
        let elapsed_joined = Instant::now().duration_since(start_joined);
        let elapsed_joined_micros = elapsed_joined.as_micros() as i128;

        let diff = elapsed_probed_micros - elapsed_joined_micros;
        if diff >= 0 {
            hist_pos.record(diff as u64).unwrap();
        } else {
            hist_neg.record(-diff as u64).unwrap();
        }
    }

    let summary_neg = summary_stats(&hist_neg);
    let summary_pos = summary_stats(&hist_pos);

    println!("summary_neg={summary_neg:?}");
    println!("summary_pos={summary_pos:?}");
}
