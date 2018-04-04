#!/usr/bin/bash
tempfile=$(mktemp)
echo "Using ${tempfile}"
playground=~/Projects/FYP/src/playground/
echo "Removing .autoparallelise"
rm .autoparallelise
echo "Running Stage 1"
RUST_BACKTRACE=full cargo build
for i in {1..5}
    do (>&2 echo "=================================================================")
done
echo "Extracting Imports"
grep -e '^#!' src/main.rs | grep -v 'plugin' >> ${tempfile}
echo "" >> $tempfile
grep -e "^extern" src/main.rs >> ${tempfile}
echo "" >> ${tempfile}
grep -e "^use" src/main.rs >> ${tempfile}
echo "Running Stage 2"
RUST_BACKTRACE=full cargo build >> ${tempfile}
for i in {1..5}
    do (>&2 echo "=================================================================")
done
cat ${tempfile} > ${playground}/src/main.rs
rm ${tempfile}
echo "Compiling Playground"
(cd ${playground} && cargo build)
