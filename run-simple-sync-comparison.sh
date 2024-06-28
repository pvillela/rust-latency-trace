#!/bin/bash

OUTPUT=out/simple_sync_comparison.txt

rm -f $OUTPUT

for i in "100 0 0" "100 0 100" "100 0 200" "100 0 400"
do
    cargo run -r --example simple_sync $i >> $OUTPUT
    cargo run -r --example simple_sync $i >> $OUTPUT
    cargo run -r --example simple_sync $i >> $OUTPUT
    cargo run -r --example simple_sync $i >> $OUTPUT
    cargo run -r --example simple_sync $i >> $OUTPUT

    cargo run -r --example simple_sync_un $i >> $OUTPUT
    cargo run -r --example simple_sync_un $i >> $OUTPUT
    cargo run -r --example simple_sync_un $i >> $OUTPUT
    cargo run -r --example simple_sync_un $i >> $OUTPUT
    cargo run -r --example simple_sync_un $i >> $OUTPUT
done
