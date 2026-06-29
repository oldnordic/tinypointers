#!/usr/bin/env bash
# Battle test suite for Tiny Pointers
#
# Order:
# 1. Correctness under adversarial keys
# 2. Collision rate comparison vs ahash/fxhash
# 3. Miri undefined behavior detection
# 4. Fuzzing with structure-aware inputs
# 5. Flamegraph + perf profiling

set -e

echo "=== TINY POINTERS BATTLE TEST SUITE ==="
echo ""

# Phase 1: Adversarial correctness
echo "[1/5] Adversarial correctness tests..."
cargo test --test adversarial

# Phase 2: Collision rate comparison
echo ""
echo "[2/5] Collision rate comparison..."
cargo test --test collision_rate

# Phase 3: Miri undefined behavior
echo ""
echo "[3/5] Miri undefined behavior detection..."
if command -v miri &> /dev/null; then
    cargo miri test --miri-config-tests
else
    echo "Miri not found, skipping UB detection"
    echo "Install with: rustup component add miri"
fi

# Phase 4: Fuzzing
echo ""
echo "[4/5] Fuzzing with structure-aware inputs..."
echo "Running short fuzz campaigns (10s each)..."

if cargo fuzz --help &> /dev/null; then
    echo "  - insert_fuzz (10s)"
    timeout 10s cargo fuzz run insert_fuzz -- -max_total_time=10 || true

    echo "  - adversarial_fuzz (10s)"
    timeout 10s cargo fuzz run adversarial_fuzz -- -max_total_time=10 || true
else
    echo "cargo-fuzz not configured, skipping"
fi

# Phase 5: Profiling
echo ""
echo "[5/5] Flamegraph + perf profiling..."

echo "  - Criterion benchmarks"
cargo bench -- --sample-size 10

echo ""
echo "  - Flamegraph generation"
if cargo bench --bench tinypointers_bench --flamegraph 2>&1; then
    echo "Flamegraphs: target/criterion/*/flamegraph.svg"
else
    echo "Flamegraph generation failed"
fi

echo ""
echo "  - Hardware counters (if perf available)"
if command -v perf &> /dev/null; then
    perf stat -e cycles,instructions,cache-references,cache-misses,L1-dcache-load-misses \
        cargo bench -- --sample-size 10 2>&1 | tee perf_output.txt
    echo "Perf output: perf_output.txt"
else
    echo "perf not found, skipping hardware counters"
fi

echo ""
echo "=== BATTLE TEST COMPLETE ==="
echo ""
echo "Results summary:"
echo "  - Adversarial tests: cargo test --test adversarial"
echo "  - Collision rates: cargo test --test collision_rate"
echo "  - Miri: cargo miri test (if available)"
echo "  - Fuzzing: cargo fuzz run <target>"
echo "  - Profiling: cargo bench + target/criterion/"
