#!/bin/bash

rm -r target/doc

pushd latency_trace

cat README.md > ../README.md

cargo makedocs \
    -e log \
    -e thread_local_collect \
    -e tokio \
    -e sha2 \
    -e base64ct \
    -i tracing-core
    # -e hdrhistogram \
    # -e tracing \
    # -e tracing-core
    # -e tracing-subscriber \

popd

cargo doc -p latency_trace --no-deps

git status
