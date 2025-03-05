#!/bin/bash

# This is only a simple helper compilation script for `getuid_grate` and `geteuid_grate`. Regular cage file will use 
# `lindtool.sh` to compile. 
# Future improvements could be:
# 1. Let user determine grate files which need to be compiled with multiple `-D` value

if [[ -z "$1" || -z "$2" ]]; then
    echo "Usage: $0 --UID_GRATE_VAL=<value> --EUID_GRATE_VAL=<value>"
    exit 1
fi

# Extract `UID_GRATE_VAL` from user input
if [[ "$1" =~ --UID_GRATE_VAL=([0-9]+) ]]; then
    UID_GRATE_VAL=${BASH_REMATCH[1]}
else
    echo "Error: incorrect input format! Should be --UID_GRATE_VAL=<value> --EUID_GRATE_VAL=<value>"
    exit 1
fi

if [[ "$2" =~ --EUID_GRATE_VAL=([0-9]+) ]]; then
    EUID_GRATE_VAL=${BASH_REMATCH[1]}
else
    echo "Error: incorrect input format! Should be --UID_GRATE_VAL=<value> --EUID_GRATE_VAL=<value>"
    exit 1
fi

# Compile `getuid_grate`
/home/lind/lind-wasm/clang+llvm-16.0.4-x86_64-linux-gnu-ubuntu-22.04/bin/clang -pthread --target=wasm32-unknown-wasi --sysroot /home/lind/lind-wasm/src/glibc/sysroot -DUID_GRATE_VAL=$UID_GRATE_VAL -Wl,--import-memory,--export-memory,--max-memory=67108864,--export=__stack_pointer,--export=__stack_low,--export=pass_fptr_to_wt,--export-table getuid_grate.c -g -O0 -o getuid_grate.wasm && wasm-opt --asyncify --debuginfo getuid_grate.wasm -o getuid_grate.wasm && /home/lind/lind-wasm/src/wasmtime/target/debug/wasmtime compile getuid_grate.wasm -o getuid_grate.cwasm

# Compile `geteuid_grate`
/home/lind/lind-wasm/clang+llvm-16.0.4-x86_64-linux-gnu-ubuntu-22.04/bin/clang -pthread --target=wasm32-unknown-wasi --sysroot /home/lind/lind-wasm/src/glibc/sysroot -DEUID_GRATE_VAL=$EUID_GRATE_VAL -Wl,--import-memory,--export-memory,--max-memory=67108864,--export=__stack_pointer,--export=__stack_low,--export=pass_fptr_to_wt,--export-table geteuid_grate.c -g -O0 -o geteuid_grate.wasm && wasm-opt --asyncify --debuginfo geteuid_grate.wasm -o geteuid_grate.wasm && /home/lind/lind-wasm/src/wasmtime/target/debug/wasmtime compile geteuid_grate.wasm -o geteuid_grate.cwasm
