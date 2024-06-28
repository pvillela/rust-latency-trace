//! Example of latency measurement of uninstrumented simple sync function, for comparison with `simple_sync.rs`.

use dev_support::{examples_support::cmd_line_args, simple_fns::simple_sync_un};
use std::time::Instant;

fn main() {
    // Get args from command line or use defaults below.
    let (nrepeats, ntasks, sleep_micros) = cmd_line_args().unwrap_or((100, 0, Some(100)));

    println!(
        "\n=== {} {:?} ===========================================================",
        std::env::args().next().unwrap(),
        cmd_line_args()
    );

    let start = Instant::now();
    simple_sync_un(nrepeats, ntasks, sleep_micros.unwrap());
    println!("Elapsed time: {:?}", Instant::now().duration_since(start));
}
