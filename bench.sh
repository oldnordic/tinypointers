#!/usr/bin/env bash
# Benchmark script with perf and flamegraph support

set -e

echo "=== Tiny Pointers Benchmark Suite ==="
echo ""

echo "[1/3] Standard Criterion benchmarks..."
cargo bench

echo ""
echo "[2/3] With perf counters (if available)..."
if command -v perf &> /dev/null; then
    perf stat -e cycles,instructions,cache-references,cache-misses,L1-dcache-load-misses \
        cargo bench 2>&1 | tee perf_output.txt
    echo "Perf stats saved to perf_output.txt"
else
    echo "perf not found, skipping hardware counters"
fi

echo ""
echo "[3/3] Flamegraph generation..."
if cargo bench --bench tinypointers_bench --flamegraph 2>&1; then
    echo "Flamegraphs saved in target/criterion/"
else
    echo "Flamegraph generation failed (pprof may need additional dependencies)"
fi

echo ""
echo "=== Benchmark Results ==="
echo "See: target/criterion/ directory for HTML reports"
