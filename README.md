# latency-trace

Supports latency measurement for functions and code blocks, both sync and async.

Given code instrumented wth the Rust [tracing](https://crates.io/crates/tracing) library, this library uses the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture both total and active span timings, where:

- total timings include suspend time and are based on span creation and closing;
- active timings exclude suspend time and are based on span entry and exit.

## TODOs

- Add core concepts and examples to lib docs.
- Support filtering of spans
