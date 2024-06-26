# latency_trace

This library supports latency measurement for functions and code blocks, both sync and async. Given code instrumented with the Rust [tracing](https://crates.io/crates/tracing) library, this library uses histograms from the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture span latency timings. Latencies are collected in **microseconds** for all spans with level `trace` or higher.

See the crate [documentation](https://docs.rs/thread_local_collect/latest/latency_trace/) for a comprehensive overview and usage examples. The source [repo](https://github.com/pvillela/rust-latency-trace/tree/main) also contains benchmarks and additional examples.

## Rust version requirements

This version of this library can be compiled with rustc 1.79.0 or higher. It may work with earlier rustc versions but that is not guaranteed.

## Related work

[tracing-timing](https://crates.io/crates/tracing-timing/0.2.8) also collects latency information for code instrumented with the [tracing](https://crates.io/crates/tracing) library, using histograms from [hdrhistogram](https://crates.io/crates/hdrhistogram). _tracing-timing_ collects latencies for [events](https://docs.rs/tracing/0.1.37/tracing/#events) within [spans](https://docs.rs/tracing/0.1.37/tracing/#spans). This provides more flexibility but also requires events to be defined within spans in order to measure latencies. Interpreting the latency results associated with events can be challenging for async code. By contrast, this framework simply measures span latencies and ignores events.

I am grateful to the author of `tracing-timing` for creating a high-quality, well-documented library which introduced me to the `hdrhistogram` crate and provided key insights into latency tracing concepts and mechanisms.

## License

This library is distributed under the terms of the MIT license, with copyright retained by the author.

See [LICENSE](https://github.com/pvillela/rust-latency-trace/tree/main/LICENSE) for details.
