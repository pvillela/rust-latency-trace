//! This library supports latency measurement for functions and code blocks, both sync and async.
//!
//! Given code instrumented with the Rust [tracing](https://crates.io/crates/tracing) library, this library uses histograms from the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture span latency timings.
//!
//! Latencies are collected in **microseconds** for all spans with level `trace` or higher.
//!
//! ## Core concepts
//!
//! This library collects latency information for [spans](https://docs.rs/tracing/0.1.37/tracing/#spans). Spans are defined in the code using macros and functions from the Rust [tracing](https://crates.io/crates/tracing) library which define span ***callsite***s, i.e., the places in the code where spans are defined. As the code is executed, a span definition in the code may be executed multiple times -- each such execution is a span instance. Span instances arising from the same span definition are grouped into [`SpanGroup`]s for latency information collection, which is done using [Histogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/struct.Histogram.html)s from the [hdrhistogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/) library.
//!
//! The grouping of spans for latency collection is not exactly based on the span definitions in the code. Spans at runtime are structured as a set of [span trees](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships) that correspond to the nesting of spans from code execution paths. The grouping of runtime spans for latency collection should respect the runtime parent-child relationships among spans.
//!
//! Thus, [`SpanGroup`]s form a forest of trees where some pairs of span groups have a parent-child relationship, corresponding to the parent-child relationships of the spans associated with the span groups. This means that if `SpanGroup A` is the parent of `SpanGroup B` then, for each span that was assigned to group `B`, its parent span was assigned to group `A`.
//!
//! The coarsest-grained grouping of spans is characterized by a ***callsite path*** -- a callsite and the (possibly empty) list of its ancestor callsites based on the different runtime execution paths (see [Span relationships](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships)). This is the default `SpanGroup` definition. Finer-grained groupings of spans can differentiate groups of spans with the same callsite path by taking into account values computed at runtime from the spans' runtime [Attributes](https://docs.rs/tracing/0.1.37/tracing/span/struct.Attributes.html).
//!
//! While the granularity of latency information collection cannot be finer than a [`SpanGroup`], the collected latency information can be subsequently aggregated further by grouping `SpanGroup`s as needed (see  [`Timings::aggregate`].)
//!
//! ## Design goals and approach
//!
//! **TODO:** discuss:
//!
//! - standalone
//! - easy to use
//! - work with both sync and async
//! - low overhead (mention only one mutex lock request per thread for the entire duration of the measurement, regardless of the number of spans executed).
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
//!     let latencies = LatencyTrace::default().measure_latencies(f);
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
//!     let latencies = LatencyTrace::default().measure_latencies_tokio(f);
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
