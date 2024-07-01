//! Executes benchmarks with [`dev_support::long_compute`].

use dev_support::work_fns::real_work;

#[divan::bench(args = [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, 20_000_000, 40_000_000, 80_000_000 ])]
fn long_compute_bench(extent: u64) {
    real_work(extent);
}

fn main() {
    divan::main();
}
