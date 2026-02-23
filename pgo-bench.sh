#!/bin/bash
set -e
PGODIR=$(mktemp -d -t pgo-data.XXXX)
trap 'rm -rf $PGODIR' EXIT

# Find out the current toolchain.
TOOLCHAIN=$(rustup show active-toolchain | cut -d " " -f1)
echo "Building instrumented binary for ${TOOLCHAIN}..."

# Find the llvm-profdata command for this toolchain.
CARGODIR=$(rustup which cargo | sed "s#bin/cargo#lib/rustlib#")
LLVMPROFDATA=$(find "${CARGODIR}" -name llvm-profdata)

if [ -z "${LLVMPROFDATA}" ]; then
    echo "Could not find llvm-profdata, make sure it is installed:"
    echo "> rustup component add llvm-tools-preview"
    exit 1
fi

# Build an instrumented binary for the bench example.
RUSTFLAGS="-C profile-generate=${PGODIR}" cargo build -q --release --example bench

# Run the instrumented binary multiple times to generate profile data.
echo "Running the instrumented program..."
./target/release/examples/bench >/dev/null
./target/release/examples/bench >/dev/null
./target/release/examples/bench >/dev/null
./target/release/examples/bench >/dev/null
./target/release/examples/bench >/dev/null
./target/release/examples/bench >/dev/null
./target/release/examples/bench >/dev/null
./target/release/examples/bench >/dev/null

# Merge the profile data.
${LLVMPROFDATA} merge -o "${PGODIR}/merged.profdata" "${PGODIR}"

# LLVMCOV=$(find "${CARGODIR}" -name llvm-cov)
# ${LLVMCOV} show -Xdemangler=rustfilt target/release/wfeusk \
#      -instr-profile="${PGODIR}"/merged.profdata \
#      -show-line-counts-or-regions \
#      -show-instantiations

# Build the optimized binary.
echo "Building optimized binary..."

# LLVM options, one per line:
LLVM_OPTS=(
    -inline-threshold=1024
    # -inlinehint-threshold=2048
    # -enable-gvn-hoist
    # -enable-gvn-memdep
    # -enable-gvn-sink
    # -aarch64-use-aa
)
LLVM_OPTS_STR=$(printf ' -C llvm-args=%s' "${LLVM_OPTS[@]}")
echo "$LLVM_OPTS_STR"

RUSTFLAGS="-C profile-use=${PGODIR}/merged.profdata $LLVM_OPTS_STR" cargo bench "$@"
