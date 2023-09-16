//! Example of latency measurement without tracing, for comparison purposes.

use std::{
    hint::black_box,
    thread,
    time::{Duration, Instant},
};

fn f() {
    for i in 0..1000 {
        // Simulated work
        thread::sleep(Duration::from_millis(6));

        black_box(g(i));
    }
}

fn g(i: i32) -> i32 {
    // Simulated work
    black_box(i);
    thread::sleep(Duration::from_millis(4));
    black_box(i)
}

fn main() {
    let start = Instant::now();
    f();
    println!(
        "*** Elapsed time: {:?}",
        Instant::now().duration_since(start)
    );
}
