//! This library supports latency measurement for functions and code blocks, both sync and async.
//!
//! Given code instrumented with the Rust [tracing](https://crates.io/crates/tracing) library, this library
//! uses histograms from the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture span
//! latency timings.
//!
//! Latencies are collected in **microseconds** for all spans with level `trace` or higher.
//!
//! ## Core concepts
//!
//! **TODO:** discuss:
//! - [SpanGroup]
//! - [Latencies]
//!
//! ## Design goals and approach
//!
//! **TODO:** discuss:
//! - standalone
//! - easy to use
//! - work with both sync and async
//! - low overhead (mention only one mutex lock request per thread for the entire duration of the measurement,
//!   regardless of the number of spans executed).
//!
//! ## Example usage
//!
//! ### Simple sync example
//!
//! ```
//! use latency_trace::LatencyTrace;
//! use std::{
//!     thread,
//!     time::{Duration, Instant},
//! };
//! use tracing::{instrument, trace_span};
//!
//! #[instrument(level = "trace")]
//! fn f() {
//!     for _ in 0..1000 {
//!         trace_span!("loop_body").in_scope(|| {
//!             trace_span!("empty").in_scope(|| {
//!                 // Empty span used to show some of the tracing overhead.
//!             });
//!
//!             // Simulated work
//!             thread::sleep(Duration::from_micros(6000));
//!
//!             g();
//!         });
//!     }
//! }
//!
//! #[instrument(level = "trace")]
//! fn g() {
//!     // Simulated work
//!     thread::sleep(Duration::from_micros(4000));
//! }
//!
//! fn main() {
//!     let latencies = LatencyTrace::new().measure_latencies(f);
//!
//!     println!("\nLatency stats below are in microseconds");
//!     for (span_group, stats) in latencies.summary_stats() {
//!         println!("  * {:?}, {:?}", span_group, stats);
//!     }
//!
//!     // A shorter way to print the summary stats, with uglier formatting.
//!     println!("\nDebug print of `latencies.summary_stats()`:");
//!     println!("{:?}", latencies.summary_stats());
//! }
//! ```
//!
//! ## Simple async example
//!
//! ```
//! use latency_trace::LatencyTrace;
//! use std::time::{Duration, Instant};
//! use tracing::{instrument, trace_span, Instrument};
//!
//! #[instrument(level = "trace")]
//! async fn f() {
//!     for _ in 0..1000 {
//!         async {
//!             trace_span!("empty").in_scope(|| {
//!                 // Empty span used to show some of the tracing overhead.
//!             });
//!
//!             // Simulated work
//!             tokio::time::sleep(Duration::from_micros(6000)).await;
//!
//!             g().await;
//!         }
//!         .instrument(trace_span!("loop_body"))
//!         .await
//!     }
//! }
//!
//! #[instrument(level = "trace")]
//! async fn g() {
//!     // Simulated work
//!     tokio::time::sleep(Duration::from_micros(4000)).await;
//! }
//!
//! fn main() {
//!     let latencies = LatencyTrace::new().measure_latencies_tokio(f);
//!
//!     println!("\nLatency stats below are in microseconds");
//!     for (span_group, stats) in latencies.summary_stats() {
//!         println!("  * {:?}, {:?}", span_group, stats);
//!     }
//!
//!     // A shorter way to print the summary stats, with uglier formatting.
//!     println!("\nDebug print of `latencies.summary_stats()`:");
//!     println!("{:?}", latencies.summary_stats());
//! }
//! ```

mod core_internals;
pub use core_internals::*;

mod span_groupers;
pub use span_groupers::*;

mod latency_trace;
pub use crate::latency_trace::*;

mod summary_stats;
pub use summary_stats::*;

mod wrapper;
pub use wrapper::*;

mod btreemap_ext;
pub use btreemap_ext::*;
