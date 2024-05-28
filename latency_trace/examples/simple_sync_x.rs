//! Example of latency measurement without tracing, for comparison with `simple_sync.rs`.

use std::{
    hint::black_box,
    thread,
    time::{Duration, Instant},
};

/// Returns command line argument or default
fn arg() -> u64 {
    match std::env::args().nth(1) {
        Some(v) => u64::from_str_radix(&v, 10).expect("argument must be integer"),
        None => 2000,
    }
}

fn f() {
    for i in 0..1000 {
        // Simulated work
        thread::sleep(Duration::from_micros(arg() * 3));

        black_box(g(i));
    }
}

fn g(i: i32) -> i32 {
    // Simulated work
    black_box(i);
    thread::sleep(Duration::from_micros(arg() * 2));
    black_box(i)
}

fn main() {
    let start = Instant::now();
    f();
    println!(
        "\n=== {} {} ===========================================================",
        std::env::args().nth(0).unwrap(),
        arg()
    );
    println!("Elapsed time: {:?}", Instant::now().duration_since(start));
}
