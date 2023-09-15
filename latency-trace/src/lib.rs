//! This library supports latency measurement for functions and code blocks, both sync and async.
//!
//! Given code instrumented with the Rust [`tracing`](https://crates.io/crates/tracing) library, this library
//! uses histograms from the [`hdrhistogram`](https://crates.io/crates/hdrhistogram) library to capture both total and active
//! span timings, where:
//! - total timings include suspend time and are based on span creation and closing;
//! - active timings exclude suspend time and are based on span entry and exit.
//!
//! Latencies are collected in **microseconds** for all spans with level `trace` or higher.

mod lib_core;
pub use lib_core::*;

mod span_groupers;
pub use span_groupers::*;

mod pub_itf_ext;
pub use pub_itf_ext::*;

mod histogram_summary;
pub use histogram_summary::*;

mod wrapper;
pub use wrapper::*;

mod btreemap_ext;
pub use btreemap_ext::*;
