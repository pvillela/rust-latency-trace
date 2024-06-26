//! Example of latency measurement without tracing, for comparison with `deep_sync.rs`.

use dev_support::{deep_fns::deep_sync_un, examples_support::cmd_line_args};
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
    deep_sync_un(nrepeats, ntasks);
    println!("Elapsed time: {:?}", Instant::now().duration_since(start));
}
