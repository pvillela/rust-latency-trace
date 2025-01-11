//! Compares the overhead for the measurement of latencies for [`dev_support::simple_fns::simple_real_sync`],
//! vs. the latency of [`dev_support::simple_fns::simple_real_sync_un`].

use bench_diff::bench_diff_print;
use dev_support::{
    bench_support::bench_diff::print_diff_out,
    simple_fns::{simple_real_sync, simple_real_sync_un},
};
use latency_trace::LatencyTrace;
use std::hint::black_box;

/// Returns command line arguments (`outer_repeats`, `inner_repeats`, `ntasks`, `extent`).
fn cmd_line_args() -> Option<(usize, usize, usize, u64)> {
    let mut args = std::env::args();

    let arg1 = match args.nth(1) {
        Some(arg1) if &arg1 != "--bench" => arg1,
        _ => return None,
    };

    let exec_count = arg1
        .parse::<usize>()
        .expect("1st argument (`exec_count`), must be integer");

    let nrepeats = args
        .next()
        .expect("3 more integer arguments must be provided")
        .parse::<usize>()
        .expect("2nd argument (`nrepeats`), must be integer");

    let ntasks = args
        .next()
        .expect("2 more integer arguments must be provided")
        .parse::<usize>()
        .expect("3rd argument (`ntasks`), must be integer");

    let extent = args
        .next()
        .expect("1 more integer argument must be provided")
        .parse::<u64>()
        .expect("4th argument (`extent`), must be integer");

    Some((exec_count, nrepeats, ntasks, extent))
}

fn main() {
    let (exec_count, nrepeats, ntasks, extent) = cmd_line_args().unwrap_or((200, 100, 5, 20_000));

    let f_instrumented = || {
        let lt = LatencyTrace::activated_default().unwrap();
        let timings = lt.measure_latencies(|| simple_real_sync(nrepeats, ntasks, extent));
        black_box(timings);
    };

    let f_uninstrumented = || simple_real_sync_un(nrepeats, ntasks, extent);

    let print_sub_header = || {
        println!("simple_real_sync -- nrepeats={nrepeats}, ntasks={ntasks}, extent={extent}");
        println!("simple_real_sync_un -- nrepeats={nrepeats}, ntasks={ntasks}, extent={extent}");
    };

    bench_diff_print(
        f_instrumented,
        f_uninstrumented,
        exec_count,
        &print_sub_header,
        print_diff_out,
    );

    bench_diff_print(
        f_instrumented,
        f_uninstrumented,
        exec_count,
        &print_sub_header,
        print_diff_out,
    );
}
