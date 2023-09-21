#!/bin/bash

OUTPUT=out/simple_sync_pausable.txt

rm -f $OUTPUT

for i in 0 200 400 800 1600 3200 6400
do
    cargo run -r --example simple_sync_pausable $i >> $OUTPUT
    cargo run -r --example simple_sync_pausable $i >> $OUTPUT
    cargo run -r --example simple_sync_pausable $i >> $OUTPUT
    cargo run -r --example simple_sync_pausable $i >> $OUTPUT
    cargo run -r --example simple_sync_pausable $i >> $OUTPUT
done
