## Benchmarks

My execution of the benchmarks defined in this repo on my machine indicates the following:

- For a single-threaded function that does nothing but has lots of instrumentation, the median latency was 705.1 µs with 904 spans created at runtime. The same function without instrumentation had a median latency of 70.69 ns. Thus, average overhead latency per span was 0.78 µs.
- For a multi-threaded function (6 threads) that does nothing but has lots of instrumentation, the median latency was 1.379 ms with 5,419 spans created at runtime. The same function without instrumentation had a median latency of 55.2 µs. Thus, average overhead latency per span was 0.25. The lower latency overhead compared to the above is due to parallelism.
- For an instrumented single-threaded function that has does simulated work with some thread::sleep calls, the median latency was 494.5 µs with 904 spans created at runtime. The same function without instrumentation had a median latency of 70.69 ns. Thus, average overhead latency per span was 0.47 µs.

indicates that, for a simple function that spawns five threads, each of which executes 100 times a loop containing two nested trace spans, for a total of 1,000 span creations and corresponding latency measurements (with a parallelism of 5), the total execution time of the instrumented function is ~64.8 ms versus ~62.8 ms for the uninstrumented version of the same function. Taking into account the parallelism, this amounts to ~2 ms / 200 spans, which is ~10 microseconds per span. Your mileage may vary. You are encouraged to run the benchmarks I have defined as well as your own benchmarks.
