#![doc = include_str!("lib.md")]
#![deny(clippy::unwrap_used)]

mod bench_diff;
pub use bench_diff::*;

mod summary_stats;
pub use summary_stats::*;
