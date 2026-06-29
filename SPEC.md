# Tiny Pointers — Implementation Specification

## API Completeness Status

**Current Phase:** Prototype with Core Operations

**Implemented:**
- ✅ Container hierarchy allocation
- ✅ Power-of-two-choices load balancing
- ✅ Overflow arrays for determinism
- ✅ Insert/get/remove operations
- ✅ Container::free() + helpers (free_level_slot, free_overflow_slot)
- ✅ Reverse index (HashMap<u64, TinyPointer>)

**Missing (Blocking Production):**
- ❌ capacity() query method
- ❌ clear() operation
- ❌ Debug implementation

**Missing (Nice-to-Have):**
- ❌ get_key_value() method
- ❌ contains_key() method
- ❌ with_capacity() constructor alias

**Out of Scope:**
- Generic values (design: fixed Handle type)
- Entry API (not needed for workload)
- Iterators (direct lookup, not traversal)
- Custom hashers (DefaultHasher sufficient)
- Bulk operations (extend, drain, retain)
- Serialization (runtime structure)
- Concurrent access (single-threaded workload)

## Optional Enhancements (Future Work)

### Bit-Packed Encoding

**Goal:** Implement Θ(log k) bit pointer compression

**Current:** 32-bit handles (NonZeroU32)
**Target:** 4-8 bit handles via bit manipulation

**Implementation:**
- Feature flag: `bit-packed`
- Approach: Custom bit encoding (no external deps)
- Space savings: 4-8× reduction
- Estimated effort: 1 week

**Research:** [docs/BIT_PACKED_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/BIT_PACKED_RESEARCH.md)
**Plan:** [docs/BIT_PACKED_PLAN.md](https://github.com/oldnordic/tinypointers/blob/main/docs/BIT_PACKED_PLAN.md)

### Funnel Cascade

**Goal:** Implement multi-level Funnel Hashing cascade

**Current:** 2-level hierarchy
**Target:** log n levels with dynamic bucket sizing

**Implementation:**
- Feature flag: `funnel-cascade`
- Approach: Based on [opthash](https://github.com/aaron-ang/opthash-rs) patterns
- Benefit: O(1) operations at 95%+ load factor
- Estimated effort: 2 weeks

**Research:** [docs/FUNNEL_CASCADE_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_CASCADE_RESEARCH.md)
**Plan:** [docs/FUNNEL_CASCADE_PLAN.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_CASCADE_PLAN.md)

**Documentation:**
- See `API_SPEC.md` for detailed API requirements
- See `TODO.md` for implementation task breakdown

## Problem Statement

Compress pointer storage from Θ(log n) → o(log n) bits for u64 integer keys with high hit-rate LRU access patterns.

**Target workload:** pathformer token_id → edge cache
- Keys: u64 token IDs
- Values: compact handles (indices into dense arena storage)
- Operations: insert, lookup, remove
- Access pattern: LRU-biased, high locality

## Mathematical Foundation

### Core Theorem (Bender et al., 2021)

For array at load factor (1 − δ), δ = 1/k:

- **Fixed-size tiny pointers:** Θ(log log log n + log k) bits
- **Variable-size tiny pointers:** Θ(log k) expected bits

**Key bounds:**
- Doubly-exponential tail: Pr[P ≥ k + j] ≤ 2⁻²Ω(j)
- Expected size: O(1 + log δ⁻¹)

### Construction: Container-Based Hierarchical Hash

**Structure:**
```
n/log n containers
  └─ s = c·log n items per container
      └─ log₂s levels
          ├─ Level i: s_i = s/2ⁱ buckets, b slots each
          └─ Overflow array: s_i slots per level
```

**Allocation algorithm (Allocate(k)):**
1. Hash k into container (if full → fail)
2. For i = 0 to log₂s − 1:
   - Increment L_i (count of items at level ≥ i)
   - Try allocate in level-i load-balancing table
   - If success: return (level i, slot j)
   - If L_{i+1} ≥ s_{i+1}: use overflow array

**Pointer encoding:**
- Load-balancing table: O(log i) bits (level + slot)
- Overflow array: O(log s_i) bits (level-from-back + slot index)

## Rust Architecture

### Design Constraints

**NOT a general-purpose hashmap.** Compact handle map for specific hot path.

**Type signature:**
```rust
pub struct TinyPtrMap<K, V> {
    // K: u64 (token_id)
    // V: compact handle (arena index)
}
```

**Separation of concerns:**
- Tiny pointer table: key → compact handle
- Dense arena: handle → actual payload (Vec<V> or slab)
- Keeps structure focused on placement/dereference

### Implementation Phases

#### Phase 1: Container Layer

- Hash keys into m = n / log n containers
- Each container independent
- Metadata contiguous per container

```rust
struct Container {
    levels: Vec<Level>,
    overflow_arrays: Vec<OverflowArray>,
    item_count: usize,
}
```

#### Phase 2: Per-Container Hierarchy

**Constants:**
- `s = c * log n` (container capacity)
- `b = large constant` (bucket size)
- `log₂s` levels per container

**Level structure:**
```rust
struct Level {
    buckets: Vec<Bucket>,  // s_i = s / 2^i buckets
    hash_seed: u64,        // independent hash per level
}

struct Bucket {
    slots: [Option<Key>; b],  // b slots, fixed
}
```

#### Phase 3: Overflow Path

**Critical: implement early for correctness.**

- Deterministic fallback guarantees L_i ≤ s_i
- Prevents probabilistic garbage
- Each level has overflow array with s_i slots

```rust
struct OverflowArray {
    slots: Vec<Option<(Key, Slot)>>,  // s_i slots
}
```

#### Phase 4: Handle Encoding

**Prototype: use u16/u32 first.**

```rust
enum TinyPointer {
    LoadBalancer { level: u8, slot: u16 },
    Overflow { level_from_back: u8, slot: u32 },
}
```

**Optimization: bit-packing later.**
- Level: O(log i) bits
- Slot: O(log b) = O(1) for load-balancing
- Slot: O(log s_i) for overflow

#### Phase 5: Dense Payload Arena

```rust
pub struct TinyPtrMap<K, V> {
    containers: Vec<Container>,
    arena: Vec<V>,  // dense storage
    _phantom: PhantomData<K>,
}
```

**Operations:**
- `insert(&mut self, key: K, value: V) -> Result<(), Error>`
- `get(&self, key: &K) -> Option<&V>`
- `remove(&mut self, key: &K) -> Option<V>`

### Reference Architecture

**Base: hashbrown crate**
- Modularity: separate raw_table → map
- Hash quality: ahash/std fallbacks
- Memory layout: contiguous metadata

**Adaptations for tiny pointers:**
- Replace single-level hash with container hierarchy
- Add overflow path determinism
- Compact handle encoding

## Operations

### Allocate(k)

```rust
fn allocate(&mut self, key: K) -> Result<TinyPointer, Error> {
    let container_idx = self.hash_to_container(&key);
    let container = &mut self.containers[container_idx];

    if container.item_count >= container.capacity() {
        return Err(Error::ContainerFull);
    }

    for level_idx in 0..container.levels.len() {
        container.increment_level_count(level_idx);

        match container.try_allocate_level(&key, level_idx) {
            Ok(slot) => return Ok(TinyPointer::load_balancer(level_idx, slot)),
            Err(FailureReason::BucketFull) => {
                if container.next_level_capacity(level_idx) >= container.capacity(level_idx + 1) {
                    let slot = container.allocate_overflow(&key, level_idx)?;
                    return Ok(TinyPointer::overflow(level_idx, slot));
                }
                // Continue to next level
            }
        }
    }

    Err(Error::AllocationFailed)
}
```

### Dereference(k, p)

```rust
fn dereference(&self, key: &K, ptr: &TinyPointer) -> Option<usize> {
    let container_idx = self.hash_to_container(key);
    let container = &self.containers[container_idx];

    match ptr {
        TinyPointer::LoadBalancer { level, slot } => {
            container.get_level_slot(key, *level, *slot)
        }
        TinyPointer::Overflow { level_from_back, slot } => {
            let level = container.levels.len() - 1 - *level_from_back;
            container.get_overflow_slot(key, level, *slot)
        }
    }
}
```

### Free(k, p)

```rust
fn free(&mut self, key: &K, ptr: TinyPointer) {
    let container_idx = self.hash_to_container(key);
    let container = &mut self.containers[container_idx];

    match ptr {
        TinyPointer::LoadBalancer { level, slot } => {
            container.free_level_slot(key, *level, *slot);
        }
        TinyPointer::Overflow { level_from_back, slot } => {
            let level = container.levels.len() - 1 - *level_from_back;
            container.free_overflow_slot(key, level, *slot);
        }
    }

    container.item_count -= 1;
}
```

## Benchmarking

### Tools

- **criterion**: statistical benchmarks, CI-friendly
- **perf**: hardware counters, cache misses
- **flamegraph**: hot path visualization

### Workloads

#### 1. Synthetic Map Benchmark

**Operations:**
- Random insert/lookup/remove
- High-load lookup-heavy regime

**Parameters:**
```
sizes: [10³, 10⁴, 10⁵, 10⁶]
load_factors: [0.5, 0.75, 0.9]
operations: [lookup-heavy, insert-heavy, mixed]
```

#### 2. Project Benchmark (Pathformer Trace)

**Replay actual access pattern:**
- Lazy-edge cache lookups
- Real LRU locality
- Actual key distribution

**Collection:**
```rust
// Instrument pathformer cache
struct CacheTracer {
    accesses: Vec<(u64, AccessType)>,
}

// Replay in benchmark
fn bench_trace(c: &mut Criterion) {
    let trace = load_pathformer_trace();
    c.bench_function("trace_replay", |b| {
        b.iter(|| {
            let mut map = TinyPtrMap::new();
            for (key, op) in &trace {
                match op {
                    AccessType::Lookup => { map.get(key); }
                    AccessType::Insert => { map.insert(*key, arena_index); }
                }
            }
        });
    });
}
```

### Comparators

**Baselines:**
1. `nohash_hasher::BuildNoHashHasher<u64>` — integer-key specialization
2. `hashbrown::HashMap<u64, V>` — state-of-the-art general hash
3. `sorted Vec<(u64, handle)>` — small-container baseline
4. Two-level array — clustered key ranges

**Metrics:**
- Throughput: ops/sec
- Memory: RSS, peak allocation
- Cache: L1/L2/LLC misses (perf)
- Profile: flamegraph hot paths

## Integration Path

### Phase 1: Prototype

**Goal:** Validate math → code mapping

- Fixed parameters (n, s, b)
- u16 handles (skip bit-packing)
- Single-threaded only
- Basic correctness tests

### Phase 2: Optimization

**Focus:** Real workload performance

- Profile-guided tuning
- Cache-aware layout
- Bit-packed handles
- Parameter sweep (c, b values)

### Phase 3: Pathformer Integration

**Drop-in replacement:**
```rust
// Before
use std::collections::HashMap;
let cache: HashMap<u64, EdgeHandle> = HashMap::new();

// After
use tinypointers::TinyPtrMap;
let cache: TinyPtrMap<u64, EdgeHandle> = TinyPtrMap::new();
```

**ABI compatibility:**
- Same API surface
- Same error types
- Benchmark-driven migration

## Success Criteria

**Functional:**
- ✓ All three operations (allocate/dereference/free)
- ✓ Deterministic overflow behavior
- ✓ No memory corruption (valgrind/miri clean)

**Performance:**
- ✓ Better than hashbrown on pathformer trace
- ✓ Competitive with nohash on integer keys
- ✓ < 10% overhead on sorted-Vec for tiny containers

**Space:**
- ✓ Measured pointer size: O(log k) expected
- ✓ Total overhead: < 2× baselines at load factor 0.9

## References

- Tiny Pointers (Bender et al., 2021): https://arxiv.org/abs/2111.12800
- hashbrown: https://github.com/rust-lang/hashbrown
- nohash: https://github.com/AustralianAnimals/nohash_hasher
