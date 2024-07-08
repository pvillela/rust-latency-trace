# latency_trace

This library supports latency measurement for functions and code blocks, both sync and async.

Given code instrumented with the Rust [`tracing`](https://crates.io/crates/tracing) library, this library uses histograms from the [`hdrhistogram`](https://crates.io/crates/hdrhistogram) library to capture span latency timings.

Latencies are collected in **microseconds** for all spans with level `trace` or higher.

## Design goals

This framework should:

- Be **easy to use**. It should only require a handful of lines of code to get latency metrics for code instrumented with the Rust [`tracing`](https://crates.io/crates/tracing) library.
- Be **self-contained**, i.e., should not depend on the use of external tools like OpenTelemetry collectors, Jaeger, Grafana, etc.
- **Support** both **sync and async** code.
- Have **low overhead**, i.e., the latency associated with the collection of latency information should be low.

## Core concepts

This library collects latency information for [spans](https://docs.rs/tracing/0.1.37/tracing/#spans). Span definitions are created in the code using macros and functions from the Rust [`tracing`](https://crates.io/crates/tracing) library which define span [callsites](https://docs.rs/tracing-core/latest/tracing_core/callsite/index.html), i.e., the places in the code where spans are defined. As the code is executed, a span definition in the code may be executed multiple times -- each such execution is a span instance. Span instances arising from the same span definition are grouped into [`SpanGroup`]s for latency information collection. Latencies are collected using [`Histogram`s](https://docs.rs/hdrhistogram/latest/hdrhistogram/struct.Histogram.html) from the [`hdrhistogram`](https://docs.rs/hdrhistogram/latest/hdrhistogram/) library.

The grouping of spans for latency collection is not exactly based on the span definitions in the code. Spans at runtime are structured as a set of [span trees](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships) that correspond to the nesting of spans from code execution paths. The grouping of runtime spans for latency collection should respect the runtime parent-child relationships among spans.

Thus, [`SpanGroup`]s form a forest of trees where some pairs of span groups have a parent-child relationship, corresponding to the parent-child relationships of the spans associated with the span groups. This means that if `SpanGroup A` is the parent of `SpanGroup B` then, for each span that was assigned to group `B`, its parent span was assigned to group `A`.

The coarsest-grained grouping of spans is characterized by a **_callsite path_** -- a callsite and the (possibly empty) list of its ancestor callsites based on the different runtime execution paths (see [span relationships](https://docs.rs/tracing/0.1.37/tracing/span/index.html#span-relationships)). This is the default `SpanGroup` definition. Finer-grained groupings of spans can differentiate groups of spans with the same callsite path by taking into account values computed at runtime from the spans' runtime [`Attributes`](https://docs.rs/tracing/0.1.37/tracing/span/struct.Attributes.html).

While the granularity of latency information collection cannot be finer than a [`SpanGroup`], the collected latency information can be subsequently aggregated further by grouping `SpanGroup`s as needed (see [`Timings::aggregate`].)

## Key design choices

This framework uses [`hdrhistogram`](https://docs.rs/hdrhistogram/latest/hdrhistogram/index.html)::[`Histogram`](https://docs.rs/hdrhistogram/latest/hdrhistogram/struct.Histogram.html#) to collect latency information as it provides an efficient data structure for high-fidelity data collection across wide latency value ranges.

Two other design choices support the low overhead goal.

- The `tracing` library's [`Registry`](https://docs.rs/tracing-subscriber/0.3.17/tracing_subscriber/registry/struct.Registry.html#) is used to store temporary timing data at runtime. As noted in the documentation, "This registry is implemented using a [lock-free sharded slab](https://docs.rs/sharded-slab/0.1.4/x86_64-unknown-linux-gnu/sharded_slab/index.html), and is highly optimized for concurrent access."
- Runtime data collection takes place independently on each thread, overwhelmingly without the need for inter-thread coordination. The only inter-thread coordination involves two mutex lock request per thread (at the beginning of data collection for the thread and at the end) for the entire duration of the measurement, regardless of the number of spans executed. _After_ the test execution has completed, information is post-processed for presentation purposes, with zero impact on the latency measurements. The [`thread_local_collect`](https://crates.io/crates/thread_local_collect) library is used to support this design approach.

## Usage modes

This framework is used to measure latencies for a sync or async function `f` that takes no arguments and contains code instrumented with the [`tracing`](https://crates.io/crates/tracing) framework. Any code to be measured can be wrapped by such a function.

The following modes of latency information reporting are supported:

- **_Direct_** -- Executes the instrumented function `f` and, after `f` terminates, returns with the reporting of latencies.
- **_Probed_** -- Executes the instrumented function `f`and immediately returns an object that can be used to report on partial latency information while `f` is still running. The returned object also provides a method that waits until `f` terminates and reports the final latency information.

## Async runtimes

This framework supports [`tokio`](https://crates.io/crates/tokio) out-of-the-box (see [`LatencyTrace::measure_latencies_tokio`] and [`LatencyTrace::measure_latencies_probed_tokio`]) but other async runtimes can be used as well by simply wrapping the async code with the chosen async runtime and using one of the sync methods ([`LatencyTrace::measure_latencies`] or [`LatencyTrace::measure_latencies_probed`]). The source code for the above-mentioned `tokio` variants shows exactly how to do it.

## Rust version requirements

This version of this library can be compiled with `rustc` 1.79.0 or higher. It may work with earlier `rustc` versions but that is not guaranteed.

## Default cargo feature

To include this library as a dependency without optional features in your Cargo.toml:

```toml
[dependencies]
latency_trace = "0.5"
```

## Optional cargo features

The optional feature flag "tokio" enables the [`tokio`](https://crates.io/crates/tokio)-related methods of [`LatencyTrace`](https://docs.rs/latency_trace/latest/latency_trace/struct.LatencyTrace.html).

```toml
[dependencies]
latency_trace = { version = "0.5", features = ["tokio"] }
```

To run the `doc_async_*` examples from the `latency_trace` crate (see source [repo](https://github.com/pvillela/rust-latency-trace/tree/main)), specify `--features tokio` or `--all-features` when invoking `cargo run`. For the example, to run `doc_async_probed.rs`, do as follows:

```bash
cargo run --features tokio --example doc_async_probed
```

or

```bash
cargo run --all-features --example doc_async_probed
```

## Usage examples
