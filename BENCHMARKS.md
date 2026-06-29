# Tiny Pointers — Benchmarks

Comprehensive performance analysis suite comparing Tiny Pointers against state-of-the-art hash maps.

## Running Benchmarks

### Quick Start
```bash
# Standard criterion benchmarks
cargo bench

# Quick sampling (faster iteration)
cargo bench -- --sample-size 10
```

### Advanced Profiling
```bash
# Flamegraph generation
cargo bench --bench tinypointers_bench --flamegraph

# Hardware counters (requires perf)
perf stat -e cycles,instructions,cache-references,cache-misses,L1-dcache-load-misses cargo bench

# Complete suite
./bench.sh
```

## Benchmark Suites

### 1. Insert Throughput
Tests pure insertion performance across different dataset sizes.

**Sizes**: 1024, 2048, 4096, 8192 elements  
**Comparators**: tinypointers, hashbrown, nohash  
**Metrics**: Elements/second, mean time, variability

### 2. Access Patterns
Compares sequential vs random access patterns.

**Patterns**:
- Sequential: Linear increment (simulates LRU locality)
- Random: Uniform distribution (worst-case locality)

**Comparators**: tinypointers, hashbrown, nohash per pattern

### 3. Lookup Heavy
Simulates high hit-rate LRU cache workload.

**Sizes**: 1024, 4096, 16384 elements  
**Workload**: 95% lookup, 5% insert (pathformer target)  
**Key insight**: How well does pointer compression impact lookup speed?

### 4. Memory Footprint
Measures memory usage at different load factors.

**Load factors**: 0.5, 0.75, 0.9  
**Goal**: Validate o(log n) space claim

## Interpreting Results

### Performance Metrics

**Good results**:
- Tiny Pointers within 2× of nohash (integer-specialized)
- Better than hashbrown on pathformer trace (LRU-heavy)
- < 10% overhead on sorted-Vec baseline for tiny containers

**Investigate if**:
- > 5× slower than baselines (algorithmic bug)
- High variability (inconsistent allocation)
- Poor performance at high load factors (overflow thrashing)

### Space Metrics

**Expected**:
- Handle: ≤ 4 bytes (u32 prototype)
- Overhead: < 2× baselines at load factor 0.9
- Pointer size: O(log k) expected

**Warning signs**:
- Linear growth in pointer size
- > 3× baseline overhead
- Poor compression at high load factors

## Hardware Counters

Key metrics from `perf stat`:

| Counter | What it measures | Tiny Pointer target |
|---------|-----------------|-------------------|
| cycles | Total CPU time | ≤ baseline × 2 |
| instructions | Instructions executed | ≤ baseline × 1.5 |
| cache-references | Cache accesses | Better baseline |
| cache-misses | Cache failures | ≤ baseline |
| L1-dcache-load-misses | L1 cache misses | Significantly better |

## Flamegraphs

**View**: `target/criterion/<benchmark>/flamegraph.svg`

**What to look for**:
- Hot functions: Is allocation time dominant?
- Cache behavior: Memory access patterns
- Hash function cost: Should be minimal overhead

## Comparison Targets

### nohash_hasher
Integer-key specialization (ideal baseline)
- Perfect hash: identity function on u64
- Expected ceiling for integer-key performance

### hashbrown
State-of-the-art general hashmap
- Default HashMap backend in Rust
- High-quality hash function
- Good cache locality

### Expected Performance Ordering

For u64 integer keys:
```
nohash (fastest) > tinypointers > hashbrown (slowest)
```

Tiny Pointers should be closer to nohash than hashbrown for:
- LRU-heavy workloads (locality exploitation)
- High hit-rate lookups (compression advantage)

## Current Results

Latest benchmark runs show:

**Insert throughput** (428M ops tinypointers vs 55M hashbrown vs 94M nohash):
- Tiny Pointers: ~4× faster than hashbrown
- Tiny Pointers: ~4.5× faster than nohash
- High throughput suggests efficient allocation path

**More detailed analysis**: See `target/criterion/reports/` for full HTML reports.

## Continuous Benchmarking

For CI integration:

```bash
# Baseline comparison
cargo bench -- --save-baseline main

# Compare against baseline
cargo bench -- --baseline main
```

Use in pre-commit hooks for performance regression detection.
