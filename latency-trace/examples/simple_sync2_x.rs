//! Example of latency measurement for a simple sync function tha does nothing,
//! to demonstrate the overhead associated with tracing and the framework.
//!
//! This example has the same structure as `simple_sync2.rs` but without tracing.
//! It runs in ~0.5 microseconds on my 2022 Dell Inspiron 16.

use std::{hint::black_box, time::Instant};

fn f() {
    for i in 0..1000 {
        black_box(i);

        black_box(g(i));
    }
}

fn g(x: i32) -> i32 {
    black_box(x)
}

fn main() {
    let start = Instant::now();
    f();
    println!(
        "*** Elapsed time: {:?}",
        Instant::now().duration_since(start)
    );
}
