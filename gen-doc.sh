#!/bin/bash

pushd latency-trace
cargo makedocs -e log -e thread-local-drop -e tokio -e tracing-subscriber -i tracing-core
popd
cargo doc -p latency-trace --no-deps
