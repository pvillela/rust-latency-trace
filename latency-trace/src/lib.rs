//! supports latency measurement for functions and code blocks, both sync and async.
//!
//! Given code instrumented wth the Rust [tracing](https://crates.io/crates/tracing) library, this library
//! uses the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture both total and active
//! span timings, where:
//! - total timings include suspend time and are based on span creation and closing;
//! - active timings exclude suspend time and are based on span entry and exit.

mod latency_trace;
pub use latency_trace::*;

mod span_groupers;
pub use span_groupers::*;

pub mod map;

pub mod measure;
pub use measure::*;

mod histogram_summary;
pub use histogram_summary::*;
