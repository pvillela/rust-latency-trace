#!/bin/bash

OUTPUT=out/deep_sync_comparison.txt

rm -f $OUTPUT

for i in "100 0" "200 0" "400 0" "800 0"
do
    cargo run -r --example deep_sync $i >> $OUTPUT
    cargo run -r --example deep_sync $i >> $OUTPUT
    cargo run -r --example deep_sync $i >> $OUTPUT
    cargo run -r --example deep_sync $i >> $OUTPUT
    cargo run -r --example deep_sync $i >> $OUTPUT

    cargo run -r --example deep_sync_un $i >> $OUTPUT
    cargo run -r --example deep_sync_un $i >> $OUTPUT
    cargo run -r --example deep_sync_un $i >> $OUTPUT
    cargo run -r --example deep_sync_un $i >> $OUTPUT
    cargo run -r --example deep_sync_un $i >> $OUTPUT
done
