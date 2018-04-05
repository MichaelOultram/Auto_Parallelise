#!/bin/bash
set -x
tempfile=$(mktemp)
sourcefile="$(pwd)/src/main.rs"
seq_no_o=$(mktemp)
seq_o=$(mktemp)
seq_log=$(mktemp)
par_source=$(mktemp)
par_log=$(mktemp)
par_no_o=$(mktemp)
par_o=$(mktemp)
stdin_params="1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20"
repetitions=10

# Generate a sequential program
sequential-program-generator $1 > ${tempfile}

######################
# Sequential compile #
######################
cat ${tempfile} > ${sourcefile}

# No optimisations
echo "+ cargo build" >> ${seq_log}
cargo build 2>> ${seq_log}
if [ $? -ne 0 ]; then
    echo "Failed to sequential compile no optimisations"
    echo "View log at ${seq_log}"
    exit 1
fi
start=`date +%s.%N`
for ((n=0;n<${repetitions};n++)); do
    cargo run -- ${stdin_params} >> ${seq_no_o}
done
end=`date +%s.%N`
seq_no_o_runtime=$( echo "$end - $start" | bc -l )

# With optimisations
echo "+ cargo build --release" >> ${seq_log}
cargo build --release 2>> ${seq_log}
if [ $? -ne 0 ]; then
    echo "Failed to sequential compile with optimisations"
    echo "View log at ${seq_log}"
    exit 1
fi
start=`date +%s.%N`
for ((n=0;n<${repetitions};n++)); do
    cargo run --release -- ${stdin_params} >> ${seq_o}
done
end=`date +%s.%N`
seq_o_runtime=$( echo "$end - $start" | bc -l )

# Check that optimisations have not changed the program
sort ${seq_no_o} -o ${seq_no_o}
sort ${seq_o} -o ${seq_o}
diff ${seq_no_o} ${seq_o}
if [ $? -ne 0 ]; then
    echo "There was a difference between seq_no_o and seq_o"
    exit 1
fi

####################
# Parallel compile #
####################
echo "#![feature(plugin)]
#![plugin(auto_parallelise)]
#[autoparallelise]" > ${sourcefile}
cat ${tempfile} >> ${sourcefile}

# Running Stage 1
rm .autoparallelise
echo "+ cargo build" >> ${par_log}
RUST_BACKTRACE=full cargo build 2>> ${par_log}

# Running Stage 2
echo "+ cargo build" >> ${par_log}
RUST_BACKTRACE=full cargo build >> ${par_source} 2>> ${par_log}
if [ ! -s ${par_source} ]; then
    echo "Failed to generate parallel source code"
    echo "View log at ${par_log}"
    exit 1
fi

# Copy into playground
cat ${par_source} > ${sourcefile}

# Quick and Dirty fixes
sed -i 's/stdin_receive_0) =/stdin_receive_0): (::std::sync::mpsc::Sender<(Vec<i32>,)>, ::std::sync::mpsc::Receiver<(Vec<i32>,)>) =/g' ${sourcefile}

# No optimisations
echo "+ cargo build" >> ${par_log}
cargo build 2>> ${par_log}
if [ $? -ne 0 ]; then
    echo "Failed to parallel compile no optimisations"
    echo "View log at ${par_log}"
    exit 1
fi
start=`date +%s.%N`
for ((n=0;n<${repetitions};n++)); do
    cargo run -- ${stdin_params} >> ${par_no_o}
done
end=`date +%s.%N`
par_no_o_runtime=$( echo "$end - $start" | bc -l )

# With optimisations
echo "+ cargo build --release" >> ${par_log}
cargo build --release 2>> ${par_log}
if [ $? -ne 0 ]; then
    echo "Failed to parallel compile with optimisations"
    echo "View log at ${par_log}"
    exit 1
fi
start=`date +%s.%N`
for ((n=0;n<${repetitions};n++)); do
    cargo run --release -- ${stdin_params} >> ${par_o}
done
end=`date +%s.%N`
par_o_runtime=$( echo "$end - $start" | bc -l )

# Check that optimisations have not changed the program
sort ${par_no_o} -o ${par_no_o}
sort ${par_o} -o ${par_o}
diff ${par_no_o} ${par_o}
if [ $? -ne 0 ]; then
    echo "There was a difference between par_no_o and par_o"
    echo "View log at ${par_log}"
    exit 1
fi

# Check that parallelisation has not changed the program
diff ${seq_o} ${par_o}
if [ $? -ne 0 ]; then
    echo "There was a difference between seq_o and par_o"
    echo "View log at ${par_log}"
    exit 1
fi

cat ${seq_o}
cat ${par_o}

echo "${seq_no_o_runtime},${seq_o_runtime},${par_no_o_runtime},${par_o_runtime}"

# Remove tempfiles
rm ${tempfile} ${seq_no_o} ${seq_o} ${seq_log} ${par_source} ${par_no_o} ${par_o} ${par_log}
