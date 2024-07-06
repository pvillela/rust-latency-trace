//! Compares the overhead for the measurement of latencies for [`dev_support::simple_fns::simple_real_sync`],
//! vs. the latency of [`dev_support::simple_fns::simple_real_sync_un`].

use criterion::black_box;
use dev_support::{
    bench_diff::bench_diff,
    simple_fns::{simple_real_sync, simple_real_sync_un},
};
use latency_trace::LatencyTrace;

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
    let args = cmd_line_args().unwrap_or((20, 10, 100, 5, 20_000));
    println!("\nargs: {args:?}");

    let (outer_loop, inner_loop, nrepeats, ntasks, extent) = args;

    let f_instrumented = || {
        let lt = LatencyTrace::activated_default().unwrap();
        let timings = lt.measure_latencies(|| simple_real_sync(nrepeats, ntasks, extent));
        black_box(timings);
    };

    let f_uninstrumented = || simple_real_sync_un(nrepeats, ntasks, extent);

    bench_diff(f_instrumented, f_uninstrumented, outer_loop, inner_loop);
}
