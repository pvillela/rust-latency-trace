//! This library supports latency measurement for functions and code blocks, both sync and async.
//!
//! Given code instrumented with the Rust [tracing](https://crates.io/crates/tracing) library, this library uses histograms from the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture span latency timings.
//!
//! Latencies are collected in **microseconds** for all spans with level `trace` or higher.
//!
//! ## Design goals
//!
//! This framework should:
//!
//! - Be **easy to use**. Is should only require a handful of framework lines of code to provide default latency metrics for code instrumented with the Rust [tracing](https://crates.io/crates/tracing) library.
//! - Be **self-contained**, i.e., should not depend on the use of external tools like OpenTelemetry collectors, Jaeger, Grafana, etc.
//! - **Support** both **sync and async** code.
//! - Have **low overhead**, i.e., the latency associated with the collection of latency information should be low.
//!
//! ## Core concepts
//!
//! This library collects latency information for [spans](https://docs.rs/tracing/0.1.37/tracing/#spans). Spans are defined in the code using macros and functions from the Rust [tracing](https://crates.io/crates/tracing) library which define span **_callsite_**s, i.e., the places in the code where spans are defined. As the code is executed, a span definition in the code may be executed multiple times -- each such execution is a span instance. Span instances arising from the same span definition are grouped into [`SpanGroup`]s for latency information collection, which is done using [Histogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/struct.Histogram.html)s from the [hdrhistogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/) library.
//!
//! The grouping of spans for latency collection is not exactly based on the span definitions in the code. Spans at runtime are structured as a set of [span trees](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships) that correspond to the nesting of spans from code execution paths. The grouping of runtime spans for latency collection should respect the runtime parent-child relationships among spans.
//!
//! Thus, [`SpanGroup`]s form a forest of trees where some pairs of span groups have a parent-child relationship, corresponding to the parent-child relationships of the spans associated with the span groups. This means that if `SpanGroup A` is the parent of `SpanGroup B` then, for each span that was assigned to group `B`, its parent span was assigned to group `A`.
//!
//! The coarsest-grained grouping of spans is characterized by a **_callsite path_** -- a callsite and the (possibly empty) list of its ancestor callsites based on the different runtime execution paths (see [Span relationships](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships)). This is the default `SpanGroup` definition. Finer-grained groupings of spans can differentiate groups of spans with the same callsite path by taking into account values computed at runtime from the spans' runtime [Attributes](https://docs.rs/tracing/0.1.37/tracing/span/struct.Attributes.html).
//!
//! While the granularity of latency information collection cannot be finer than a [`SpanGroup`], the collected latency information can be subsequently aggregated further by grouping `SpanGroup`s as needed (see [`TimingsAggregate::aggregate`].)
//!
//! ## Key design choices
//!
//! This framework uses [hdrhistogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/index.html)::[Histogram](https://docs.rs/hdrhistogram/latest/hdrhistogram/struct.Histogram.html#) to collect latency information as it provides an efficient data structure for high-fidelity data collection across wide latency value ranges.
//!
//! Two other design choices support the low overhead goal.
//!
//! - The _tracing_ library's [Registry](https://docs.rs/tracing-subscriber/0.3.17/tracing_subscriber/registry/struct.Registry.html#) is used to store temporary timing data at runtime. As noted in the documentation, "This registry is implemented using a [lock-free sharded slab](https://docs.rs/sharded-slab/0.1.4/x86_64-unknown-linux-gnu/sharded_slab/index.html), and is highly optimized for concurrent access."
//! - Runtime data collection takes place independently on each thread, overwhelmingly without the need for inter-thread coordination. The only inter-thread coordination involved is one mutex lock request per thread for the entire duration of the measurement, regardless of the number of spans executed. _After_ the test execution has completed, information is extracted from the various threads, with zero impact on the latency measurements. The [thread-local-drop] framework is used to support this design approach.
//!
//! ## Usage modes
//!
//! This framework is used to measure latencies for a sync or async function `f` that takes no arguments and contains code instrumented  with the *tracing* framework.  Any code to be measured can be wrapped by such a function.
//!
//! The following modes of latency information reporting are supported:
//!
//! - ***Direct*** -- Information is reported only after `f` terminates.
//! - ***Pausable*** -- Partial information can be reported during `f`'s execution. In this case, there are two sub-options:
//!   - ***Nonblocking*** -- `f`'s execution continues normally but latency information collection is paused while the previously collected data is extracted for reporting. In this case, some latency information is lost during the collection pause. This is the preferred option.
//!   - ***Blocking*** -- `f`'s execution is blocked while the previously collected data is extracted for reporting. In this case, there is no loss of latency information but there is distortion of latencies for the period during which `f`'s execution was paused.
//!
//! The *direct* mode has the lowest overhead -- see [Key design choices](#key-design-choices) above. It is suitable for code that runs to completion in a reasonable amount of time.
//!
//! The *pausable* modes are suitable for code that is expected to run for extended periods of time, including servers. The *pausable* modes add some overhead beyond the direct mode as a read is performed on an [RwLock](https://doc.rust-lang.org/stable/std/sync/struct.RwLock.html) for each span. Informal benchmarking performed by the author indicates that this additional overhead is small, but this depends on the use case and the user is encouraged to perform their own benchmarks.
//!
//! ## Async runtimes
//!
//! This framework supports [tokio](https://crates.io/crates/tokio) out-of-the-box (see [`LatencyTrace::measure_latencies_tokio`] and [`LatencyTrace::measure_latencies_pausable_tokio`]) but other async runtimes can be used as well by simply wrapping the async code with the chosen async runtime and using one of the sync methods ([`LatencyTrace::measure_latencies`] or [`LatencyTrace::measure_latencies_pausable`]). The source code for the above-mentioned *tokio* variants shows exactly how to do it.
//!
//! ## Example usage
//!
//! ### Simple sync example
//!
//! ```rust
//! use latency_trace::{summary_stats, BTreeMapExt, LatencyTrace};
//! use std::{thread, time::Duration};
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
//!     for (span_group, stats) in latencies.map_values(summary_stats) {
//!         println!("  * {:?}, {:?}", span_group, stats);
//!     }
//!
//!     // A shorter way to print the summary stats, with uglier formatting.
//!     println!("\nDebug print of `latencies.map_values(summary_stats)`:");
//!     println!("{:?}", latencies.map_values(summary_stats));
//! }
//! ```
//!
//! ### Simple async example
//!
//! ```rust
//! use latency_trace::{summary_stats, BTreeMapExt, LatencyTrace};
//! use std::time::Duration;
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
//!     for (span_group, stats) in latencies.map_values(summary_stats) {
//!         println!("  * {:?}, {:?}", span_group, stats);
//!     }
//!
//!     // A shorter way to print the summary stats, with uglier formatting.
//!     println!("\nDebug print of `latencies.map_values(summary_stats)`:");
//!     println!("{:?}", latencies.map_values(summary_stats));
//! }
//! ```
//!
//! ### Simple sync pausable example
//!
//! ```rust
//! use latency_trace::{summary_stats, BTreeMapExt, LatencyTrace, PausableMode};
//! use std::{thread, time::Duration};
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
//!     let pausable = LatencyTrace::default().measure_latencies_pausable(PausableMode::Nonblocking, f);
//!     thread::sleep(Duration::from_micros(24000));
//!     let latencies1 = pausable.pause_and_report();
//!     let latencies2 = pausable.wait_and_report();
//!
//!     println!("\nlatencies1 in microseconds");
//!     for (span_group, stats) in latencies1.map_values(summary_stats) {
//!         println!("  * {:?}, {:?}", span_group, stats);
//!     }
//!
//!     println!("\nlatencies2 in microseconds");
//!     for (span_group, stats) in latencies2.map_values(summary_stats) {
//!         println!("  * {:?}, {:?}", span_group, stats);
//!     }
//! }
//! ```
//!
//! **Async pausable** is similar to the above but uses [`LatencyTrace::measure_latencies_pausable_tokio`] instead.
//!
//! ## Related work
//!
//! [tracing-timing](https://crates.io/crates/tracing-timing/0.2.8) also collects latency information for code instrumented with the [tracing](https://crates.io/crates/tracing) library, using histograms from [hdrhistogram](https://crates.io/crates/hdrhistogram). _tracing-timing_ collects latencies for [events](https://docs.rs/tracing/0.1.37/tracing/#events) within [spans](https://docs.rs/tracing/0.1.37/tracing/#spans). This provides more flexibility but also requires events to be defined within spans in order to measure latencies. Interpreting the latency results associated with events can be challenging for async code. By contrast, this framework simply measures span latencies and ignores events.
//!
//! I am grateful to the author of _tracing-timing_ for creating a high-quality, well-documented library which introduced me to the _hdrhistogram_ crate and provided key insights into latency tracing concepts and mechanisms.

mod core_internals;
pub use core_internals::*;

mod span_groupers;
pub use span_groupers::*;

mod latency_trace;
pub use crate::latency_trace::*;

mod summary_stats;
pub use summary_stats::*;

mod btreemap_ext;
pub use btreemap_ext::*;

mod pausable_trace;
pub use pausable_trace::*;
