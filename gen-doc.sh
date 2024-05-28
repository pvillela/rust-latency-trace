#!/bin/bash

pushd latency_trace
cargo makedocs -e log -e thread_local_collect -e tokio -e tracing-subscriber -e sha2 -e base64ct -i tracing-core
popd
cargo doc -p latency_trace --no-deps

cat readme0.md readme1.md readme2.md > README.md

git status
