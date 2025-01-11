This library supports the reliable comparison of the difference in latency between two functions/closures.

One could simply run tools like [Criterion](https://crates.io/crates/criterion) or [Divan](https://crates.io/crates/divan) on each function and compare the results. However, a couple of challenges arise:

- Order bias -- When running two benchmarks, one after the other, the first one may get an edge over the second one. This can be empirically observed. I speculate that this observation may be due to the following:

  - If the two benchmarks are run in the same process, the bias may be due to memory allocation order.
  - Even if they are run in separate processes, the second one may run with a hotter CPU that may be throttled down by the hardware/OS to stay within allowable hardware thermal limits.

- Random noise -- Random noise can confound results when the two functions have latencies that are close to each other.

These challenges can be addressed by running the individual benchmarks multiple times, interspersed with cooling-down intervals, and using appropriate statistical methods to compare the two observed latency distributions. This is, however, time consuming and requires careful planning and analysis.

The present library provides a quick and convenient alternative to the above cumbersome methodology. It addresses order bias and random noise as follows:

- Let `f1` be the first target function and `f2` be the second one.
- It contains an outer loop that executes benchmarks for both functions multiple times for statistical stability.
- Each execution of the outer loop includes two inner loops -- one executes `f1` multiple times and the other executes `f2` the same number of times.
- The overall comparison benchmark execution is preceded by a warm-up execution of one iteration of the outer loop, which is not tallied.
- It maintains four histograms:
  - `hist_f1`: captures the mean latency of `f1` over each inner loop.
  - `hist_f2`: captures the mean latency of `f2` over each inner loop.
  - `hist_f1_ge_f2`: captures the mean latency of `f1` minus the mean latency of `f2` over each inner loop, when the difference is greater than or equal to zero.
  - `hist_f1_lt_f2`: captures the mean latency of `f1` minus the mean latency of `f2` over each inner loop, when the difference is negative.
- Due to the execution structure with the outer and inner loops, groups of executions of `f1` and `f2` alternate repeatedly, effectively eliminating order bias.
- The inner loops address the noise problem by computing the respective means over a substantial number of iterations and assigning the resulting difference to either `hist_f1_ge_f2` or `hist_f1_lt_f2`.
- A simple comparison of the number of observations in `hist_f1_ge_f2` and `hist_f1_lt_f2` yields a reliable indicator of which function is faster, provided that the number of outer and inner loop iterations are reasonably large. If the count in `hist_f1_lt_f2` is higher then `f1` is faster, and vice-versa.
- The summary statistics for the four histograms are generated as well, but the key hypothesis testing purpose of this library is addressed by the above point.
