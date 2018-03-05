#!/usr/bin/bash
playground=~/Projects/FYP/src/playground/
echo "Removing .autoparallelise"
rm .autoparallelise
echo "Running Stage 1"
cargo build
for i in {1..5}
    do (>&2 echo "=================================================================")
done
echo "Running Stage 2"
cargo build >${playground}/src/main.rs
for i in {1..5}
    do (>&2 echo "=================================================================")
done
echo "Compiling Playground"
(cd ${playground} && cargo build)
