## Benchmarks

My execution of the benchmarks defined in this repo on my laptop indicates that there is great variability in the latency tracing overhead. With different target functions, the overhead per span varied from less than 1 µs up to 32 µs. The overhead percentage relative to the total latency of the uninstrumented version of the target function varied from less than 1% to 7%. Furthermore, given a target function, multiple runs of the Divan or Criterion bencmarks can produce significantly varying results.
