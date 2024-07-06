## Benchmarks

Execution on my laptop of the benchmarks defined in the source [repo](https://github.com/pvillela/rust-latency-trace/tree/main) indicates that there can be significant variability in latency tracing overhead measurements. There can be so much variability across runs that sometimes nonsensical negative overheads were observed (using both the Divan and Criterion benchmarking frameworks), i.e., the median total latency of an instrumented function was less than that of the uninstrumented version of the same function.

To more reliably compare latency differences between two functions, given the above-mentioned variability, I created a utility that separates positive and negative differences so that nonsensical results can be easily excluded. See [`bench_diff.rs`](https://github.com/pvillela/rust-latency-trace/blob/main/dev_support/src/bench_diff.rs).

Running the benchark [`bench_overhead_simple_real_sync`](https://github.com/pvillela/rust-latency-trace/blob/main/dev_support/benches/bench_overhead_simple_real_sync.rs) with a target function (`simple_real_sync`) that does heavy in-memory computation, the overhead per span varied from less than 1 µs up to 3 µs, depending on the intensity of the computation. The high end of the aforementioned overhead range was with computations that took 800 µs to 1,200 µs per span, in a multi-threaded scenario with 6 threads and 100 iterations (instantiating 2 spans each) per thread.

Where the target function had a `thread::sleep()` in lieu of real computation, higher overheads were observed, especially as the sleep time increased. It appears that thread context switches have a significant adverse impact on overheads, possibly due to the invalidation of processor caches.
