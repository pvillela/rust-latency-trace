#!/bin/bash

cargo bench --bench bench_deep_divan && \
cargo bench --bench bench_simple_divan && \
cargo bench --bench bench_simple_criterion

# Below is skipped because it crashes
# cargo bench --bench bench_deep_criterion

