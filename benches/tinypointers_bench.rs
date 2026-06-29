// Tiny Pointers benchmarks with criterion, flamegraph, and perf
//
// Run with:
// - cargo bench (standard criterion output)
// - cargo bench -- --profile-time 10 (longer runs)
// - cargo bench --bench tinypointers_bench --flamegraph (flamegraph)
// - perf stat -e cycles,instructions,cache-references,cache-misses,L1-dcache-load-misses cargo bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use nohash_hasher::BuildNoHashHasher;
use std::collections::HashMap;
use std::num::NonZeroU32;
use tinypointers::TinyPtrMap;

/// Benchmark map insert operations - throughput comparison
fn bench_insert_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_throughput");

    for size in [1024, 2048, 4096, 8192].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter("tinypointers"),
            size,
            |b, &size| {
                let mut map = TinyPtrMap::new(size);
                b.iter(|| {
                    let key: u64 = black_box(rand::random());
                    let handle = NonZeroU32::MIN;
                    let _ = black_box(map.insert(key, handle));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter("hashbrown"),
            size,
            |b, &size| {
                let mut map: HashMap<u64, NonZeroU32> = HashMap::with_capacity(size);
                b.iter(|| {
                    let key: u64 = black_box(rand::random());
                    let handle = NonZeroU32::MIN;
                    black_box(map.insert(key, handle));
                });
            },
        );

        group.bench_with_input(BenchmarkId::from_parameter("nohash"), size, |b, &size| {
            let mut map: HashMap<u64, NonZeroU32, BuildNoHashHasher<u64>> =
                HashMap::with_capacity_and_hasher(size, BuildNoHashHasher::default());
            b.iter(|| {
                let key: u64 = black_box(rand::random());
                let handle = NonZeroU32::MIN;
                black_box(map.insert(key, handle));
            });
        });
    }

    group.finish();
}

/// Benchmark sequential vs random access patterns
fn bench_access_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("access_patterns");

    for pattern in ["sequential", "random"].iter() {
        group.bench_function(format!("tinypointers_{}", pattern), |b| {
            let mut map = TinyPtrMap::new(2048);
            let mut base_key: u64 = rand::random();

            b.iter(|| {
                let key = if *pattern == "sequential" {
                    base_key = base_key.wrapping_add(1);
                    base_key
                } else {
                    rand::random()
                };

                let handle = NonZeroU32::MIN;
                let _ = black_box(map.insert(key, handle));
            });
        });

        group.bench_function(format!("hashbrown_{}", pattern), |b| {
            let mut map: HashMap<u64, NonZeroU32> = HashMap::with_capacity(2048);
            let mut base_key: u64 = rand::random();

            b.iter(|| {
                let key = if *pattern == "sequential" {
                    base_key = base_key.wrapping_add(1);
                    base_key
                } else {
                    rand::random()
                };

                let handle = NonZeroU32::MIN;
                black_box(map.insert(key, handle));
            });
        });

        group.bench_function(format!("nohash_{}", pattern), |b| {
            let mut map: HashMap<u64, NonZeroU32, BuildNoHashHasher<u64>> =
                HashMap::with_capacity_and_hasher(2048, BuildNoHashHasher::default());
            let mut base_key: u64 = rand::random();

            b.iter(|| {
                let key = if *pattern == "sequential" {
                    base_key = base_key.wrapping_add(1);
                    base_key
                } else {
                    rand::random()
                };

                let handle = NonZeroU32::MIN;
                black_box(map.insert(key, handle));
            });
        });
    }

    group.finish();
}

/// Benchmark lookup-heavy workload (high hit-rate LRU simulation)
fn bench_lookup_heavy(c: &mut Criterion) {
    let mut group = c.benchmark_group("lookup_heavy");

    for size in [1024, 4096, 16384].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter("tinypointers"),
            size,
            |b, &size| {
                let mut map = TinyPtrMap::new(size);
                let keys: Vec<u64> = (0..size).map(|_| rand::random()).collect();

                for &key in &keys {
                    let _ = map.insert(key, NonZeroU32::MIN);
                }

                b.iter(|| {
                    let key = black_box(keys[rand::random::<usize>() % keys.len()]);
                    black_box(map.get(&key));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::from_parameter("hashbrown"),
            size,
            |b, &size| {
                let mut map: HashMap<u64, NonZeroU32> = HashMap::with_capacity(size);
                let keys: Vec<u64> = (0..size).map(|_| rand::random()).collect();

                for &key in &keys {
                    map.insert(key, NonZeroU32::MIN);
                }

                b.iter(|| {
                    let key = black_box(keys[rand::random::<usize>() % keys.len()]);
                    black_box(map.get(&key));
                });
            },
        );

        group.bench_with_input(BenchmarkId::from_parameter("nohash"), size, |b, &size| {
            let mut map: HashMap<u64, NonZeroU32, BuildNoHashHasher<u64>> =
                HashMap::with_capacity_and_hasher(size, BuildNoHashHasher::default());
            let keys: Vec<u64> = (0..size).map(|_| rand::random()).collect();

            for &key in &keys {
                map.insert(key, NonZeroU32::MIN);
            }

            b.iter(|| {
                let key = black_box(keys[rand::random::<usize>() % keys.len()]);
                black_box(map.get(&key));
            });
        });
    }

    group.finish();
}

/// Benchmark memory footprint at different load factors
fn bench_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_footprint");

    for load_factor in [0.5, 0.75, 0.9].iter() {
        group.bench_with_input(
            BenchmarkId::new("tinypointers", load_factor),
            load_factor,
            |b, &load_factor| {
                let size = 4096;
                let actual_size = (size as f64 / load_factor) as usize;
                let mut map = TinyPtrMap::new(actual_size);

                b.iter(|| {
                    let keys: Vec<u64> = (0..size).map(|_| rand::random()).collect();
                    for &key in &keys {
                        let _ = map.insert(key, NonZeroU32::MIN);
                    }
                    black_box(map.len());
                    map = TinyPtrMap::new(actual_size);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_insert_throughput,
    bench_access_patterns,
    bench_lookup_heavy,
    bench_memory_footprint
);
criterion_main!(benches);
