//! This library supports latency measurement for functions and code blocks, both sync and async.
//!
//! Given code instrumented with the Rust [tracing](https://crates.io/crates/tracing) library, this library
//! uses histograms from the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture span
//! latency timings.
//!
//! Latencies are collected in **microseconds** for all spans with level `trace` or higher.

mod core_internals;
pub use core_internals::*;

mod span_groupers;
pub use span_groupers::*;

mod latency_trace;
pub use latency_trace::*;

mod summary_stats;
pub use summary_stats::*;

mod wrapper;
pub use wrapper::*;

mod btreemap_ext;
pub use btreemap_ext::*;
