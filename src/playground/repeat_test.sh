#!/bin/bash
for complexity in {1..10}
do
    for rep in {0..50}
    do
        echo "complexity=${complexity}, rep=${rep}"
        ./generated-test.sh ${complexity}
    done
done
