## Benchmarks

My execution of the benchmarks defined in this repo indicates that, for a simple function that spawns five threads, each of which executes 100 times a loop containing two nested trace spans, for a total of 1,000 span creations and corresponding latency measurements (with a parallelism of 5), the total execution time of the instrumented function is ~64.8 ms versus ~62.8 ms for the uninstrumented version of the same function. Taking into account the parallelism, this amounts to ~2 ms / 200 spans, which is ~10 microseconds per span. Your mileage may vary. You are encouraged to run the benchmarks I have defined as well as your own benchmarks.

## Related work

[tracing-timing](https://crates.io/crates/tracing-timing/0.2.8) also collects latency information for code instrumented with the [tracing](https://crates.io/crates/tracing) library, using histograms from [hdrhistogram](https://crates.io/crates/hdrhistogram). _tracing-timing_ collects latencies for [events](https://docs.rs/tracing/0.1.37/tracing/#events) within [spans](https://docs.rs/tracing/0.1.37/tracing/#spans). This provides more flexibility but also requires events to be defined within spans in order to measure latencies. Interpreting the latency results associated with events can be challenging for async code. By contrast, this framework simply measures span latencies and ignores events.

I am grateful to the author of _tracing-timing_ for creating a high-quality, well-documented library which introduced me to the _hdrhistogram_ crate and provided key insights into latency tracing concepts and mechanisms.
