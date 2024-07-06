//! Compares total latency for the measurement of latencies for [`dev_support::simple_fns::simple_real_sync`]
//! with both the default [`latency_trace::LatencyTrace`], which uses
//! [`thread_local_collect::tlm::probed`](https://docs.rs/thread_local_collect/latest/thread_local_collect/tlm/probed/index.html)
//! and [`latency_trace::LatencyTraceJ`] which uses
//! [`thread_local_collect::tlm::joined`](https://docs.rs/thread_local_collect/latest/thread_local_collect/tlm/joined/index.html).
//!
//! Running `cargo bench --bench bench_diff_simple_real_sync_probed_vs_joined -- 200 10 100 5 20000` shows quite conclusively
//! that there is no measurable difference in overhead with one `thread_local_collect` module versus the other. That command
//! could take up to a couple of minutes to finish.

use dev_support::{bench_diff::bench_diff, simple_fns::simple_real_sync};
use latency_trace::LatencyTraceE;

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

    let lt = LatencyTraceE::activated_default().unwrap();

    let f_probed = || {
        LatencyTraceE::select_probed();
        lt.measure_latencies(|| simple_real_sync(nrepeats, ntasks, extent))
    };

    let f_joined = || {
        LatencyTraceE::select_joined();
        lt.measure_latencies(|| simple_real_sync(nrepeats, ntasks, extent))
    };

    bench_diff(f_probed, f_joined, outer_loop, inner_loop);
}
