#!/bin/bash

OUTPUT=out/simple_sync_comparison.txt

rm -f $OUTPUT

for i in 0 200 400 800 1600 3200
do
    cargo run -r --example simple_sync $i >> $OUTPUT
    cargo run -r --example simple_sync $i >> $OUTPUT
    cargo run -r --example simple_sync_x $i >> $OUTPUT
    cargo run -r --example simple_sync_x $i >> $OUTPUT
done
