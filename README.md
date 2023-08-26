# latency-trace

A little library that supports latency measurement for functions and code blocks, both sync and async.

Given code instrumented wth the Rust [tracing](https://crates.io/crates/tracing) library, `latency-trace`` uses the [hdrhistogram](https://crates.io/crates/hdrhistogram) library to capture both total and sync span timings, where:

- total timings include suspend time and are based on span creation and closing;
- active timings exclude suspend time and are based on span entry and exit.
