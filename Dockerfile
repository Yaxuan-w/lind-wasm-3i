# Use an official Ubuntu as a parent image
FROM --platform=linux/amd64 ubuntu:22.04

# Set the working directory to home
WORKDIR /home

# Install all the required dependencies 
RUN apt-get update && \
    apt-get install -y build-essential git wget gcc-i686-linux-gnu g++-i686-linux-gnu \
    bison gawk vim libxml2 python3 curl gcc

# Clone the Lind-wasm repository
RUN git clone --recurse-submodules https://github.com/Lind-Project/lind-wasm.git

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    . "$HOME/.cargo/env" && \
    rustup install nightly && \
    rustup default nightly

# Ensure the Rust environment is available in future RUN instructions
ENV PATH="/root/.cargo/bin:${PATH}"

# Install clang-16 for compiling the code
RUN wget https://github.com/llvm/llvm-project/releases/download/llvmorg-16.0.4/clang+llvm-16.0.4-x86_64-linux-gnu-ubuntu-22.04.tar.xz && \
    tar -xf clang+llvm-16.0.4-x86_64-linux-gnu-ubuntu-22.04.tar.xz

# Go inside Lind-wasm repository and clone rustposix
WORKDIR /home/lind-wasm
RUN git clone https://github.com/yzhang71/safeposix-rust.git 

# Go inside lind-wasm/glibc and switch to main branch
WORKDIR /home/lind-wasm/glibc
RUN git switch main

# Move wasi directory
RUN mv /home/lind-wasm/glibc/wasi /home/clang+llvm-16.0.4-x86_64-linux-gnu-ubuntu-22.04/lib/clang/16/lib

# Configure glibc for WASI
RUN echo '#!/bin/bash\n\
    set -e\n\
    BUILDDIR=build\n\
    mkdir -p $BUILDDIR\n\
    cd $BUILDDIR\n\
    ../configure --disable-werror --disable-hidden-plt --with-headers=/usr/i686-linux-gnu/include --prefix=/home/lind-wasm/glibc/target --host=i686-linux-gnu --build=i686-linux-gnu\
        CFLAGS=" -O2 -g" \
        CC="/home/clang+llvm-16.0.4-x86_64-linux-gnu-ubuntu-22.04/bin/clang-16 --target=wasm32-unknown-wasi -v -Wno-int-conversion"\n' > config.sh && \
    unset LD_LIBRARY_PATH && \
    chmod +x config.sh

# Build glibc
RUN ./config.sh && \
    cd build && \
    make --keep-going -j4 || true

RUN make install --keep-going || true

# Goto Lind-wasm/safeposix directory and build the project
WORKDIR /home/lind-wasm/safeposix-rust
RUN git switch 3i-dev && \
    cargo build

# Build and run wasmtime
WORKDIR /home/lind-wasm/wasmtime
RUN git switch add-lind && \
    cp /home/lind-wasm/safeposix-rust/target/debug/librustposix.so /home/lind-wasm/wasmtime/crates/rustposix && \
    git submodule update --init && \
    cd /home/lind-wasm/wasmtime/crates/rustposix/src && \
    rm -r build.rs && \
    echo 'fn main() { println!("cargo:rustc-link-search=native=/home/lind-wasm/wasmtime/crates/rustposix"); println!("cargo:rustc-link-lib=dylib=rustposix"); }' > build.rs && \
    chmod +x build.rs && \
    cd /home/lind-wasm/wasmtime && \
    export LD_LIBRARY_PATH=/home/lind-wasm/wasmtime/crates/rustposix:$LD_LIBRARY_PATH && \
    cargo build

# Modify stubs.h
RUN cd /home/lind-wasm/glibc/target/include/gnu && \
    rm -r stubs.h && \
    echo '/* This file is automatically generated. This file selects the right generated file of `__stub_FUNCTION` macros based on the architecture being compiled for. */\n#if defined __x86_64__ && defined __LP64__\n# include <gnu/stubs-64.h>\n#endif\n#if defined __x86_64__ && defined __ILP32__\n# include <gnu/stubs-x32.h>\n#endif\n' > stubs.h && \
    chmod +x stubs.h

# Get glibc as working directory
WORKDIR /home/lind-wasm/glibc

# Generate wasm sysroot
RUN echo '#!/bin/bash\n\nsrc_dir="./build"\n\ninclude_source_dir="/home/lind-wasm/glibc/target/include"\ncrt1_source_path="/home/lind-wasm/glibc/lind_syscall/crt1.o"\nlind_syscall_path="/home/lind-wasm/glibc/lind_syscall/lind_syscall.o"\n\noutput_archive="sysroot/lib/wasm32-wasi/libc.a"\nsysroot_dir="sysroot"\n\nrm -rf "$sysroot_dir"\n\nobject_files=$(find "$src_dir" -type f -name "*.o" ! \\( -name "stamp.o" -o -name "argp-pvh.o" -o -name "repertoire.o" \\))\nobject_files="$object_files $lind_syscall_path"\n\nif [ -z "$object_files" ]; then\n  echo "No suitable .o files found in '$src_dir'."\n  exit 1\nfi\n\nmkdir -p "$sysroot_dir/include/wasm32-wasi" "$sysroot_dir/lib/wasm32-wasi"\n\n/home/clang+llvm-16.0.4-x86_64-linux-gnu-ubuntu-22.04/bin/llvm-ar rcs "$output_archive" $object_files\n\nif [ $? -eq 0 ]; then\n  echo "Successfully created $output_archive with the following .o files:"\n  echo "$object_files"\nelse\n  echo "Failed to create the archive."\n  exit 1\nfi\n\ncp -r "$include_source_dir"/* "$sysroot_dir/include/wasm32-wasi/"\n\ncp "$crt1_source_path" "$sysroot_dir/lib/wasm32-wasi/"\n' > gen_sysroot.sh && \
    chmod +x gen_sysroot.sh && \
    ./gen_sysroot.sh