#!/bin/bash

cargo makedocs -e hdrhistogram
cargo doc -p bench_diff --no-deps --all-features
