#!/usr/bin/bash
rm .autoparallelise
cargo build
for i in {1..5}
    do echo "================================================================="
done
cargo build
