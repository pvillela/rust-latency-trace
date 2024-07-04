#!/bin/bash

cargo bench --all-features --bench bench_deep_divan
cargo bench --all-features --bench bench_simple_divan
cargo bench --all-features --bench bench_deep_criterion
cargo bench --all-features --bench bench_simple_criterion
