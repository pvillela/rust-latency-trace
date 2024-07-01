//! Module that provides functions to simulate work, used to support benchmarks.

use sha2::{Digest, Sha256};
use std::{hint::black_box, thread, time::Duration};

/// Function that sleeps to simulate work to support benchmarks.
pub fn lazy_work(extent: u64) {
    thread::sleep(Duration::from_micros(extent));
}

/// Function that does a significant amount of computation to support benchmarks.
pub fn real_work(extent: u64) {
    let buf = black_box(extent.to_be_bytes());
    let mut hasher = Sha256::new();
    for _ in 0..extent {
        hasher.update(buf);
    }
    let hash = hasher.finalize();
    black_box(hash);
}
