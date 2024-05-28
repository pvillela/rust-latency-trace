#!/bin/bash

pushd latency_trace
cargo makedocs -e log -e thread-local-collect -e tokio -e tracing-subscriber -e sha2 -e base64ct -i tracing-core
popd
cargo doc -p latency_trace --no-deps
