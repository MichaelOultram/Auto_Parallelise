#!/bin/bash
for complexity in {25..200..25}
do
    for rep in {0..50}
    do
        echo "complexity=${complexity}, rep=${rep}"
        ./generated-test.sh ${complexity}
    done
done
