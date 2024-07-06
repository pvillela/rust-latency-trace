#!/bin/bash

for i in "200 10 100 0 10000" "200 10 100 0 20000" "200 10 100 0 40000" "200 10 100 0 80000"
do
    cargo bench --bench bench_overhead_simple_real_sync -- $i
    cargo bench --bench bench_overhead_simple_real_sync -- $i
    cargo bench --bench bench_overhead_simple_real_sync -- $i
    cargo bench --bench bench_overhead_simple_real_sync -- $i
    cargo bench --bench bench_overhead_simple_real_sync -- $i
done

for i in "200 10 100 5 10000" "200 10 100 5 20000" "200 10 100 5 40000" "200 10 100 5 80000"
do
    cargo bench --bench bench_overhead_simple_real_sync -- $i
    cargo bench --bench bench_overhead_simple_real_sync -- $i
    cargo bench --bench bench_overhead_simple_real_sync -- $i
    cargo bench --bench bench_overhead_simple_real_sync -- $i
    cargo bench --bench bench_overhead_simple_real_sync -- $i
done
