#!/bin/bash

pushd latency_trace
cargo package $1
popd
