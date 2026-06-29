# Tinypointers Architecture

Design decisions, internal structure, and implementation details.

## Table of Contents

- [Overview](#overview)
- [Core Concepts](#core-concepts)
- [Data Structures](#data-structures)
- [Algorithms](#algorithms)
- [Memory Layout](#memory-layout)
- [Hash Functions](#hash-functions)
- [Load Balancing](#load-balancing)
- [Overflow Handling](#overflow-handling)
- [Tradeoffs](#tradeoffs)
- [Future Enhancements](#future-enhancements)

## Overview

Tinypointers implements **Tiny Pointers** (Bender et al., 2021) — a hash map specialized for u64 keys with sub-logarithmic pointer storage.

**Key Design Goals:**
1. **Space efficiency:** Θ(log k) bits per pointer (vs log₂ n bits)
2. **Time efficiency:** O(1) expected operations
3. **Simplicity:** 2-level hierarchy (vs full funnel cascade)
4. **Determinism:** Overflow arrays prevent infinite probe sequences

**Architecture:**
```
n/log n containers
  └── s = 16·log n items per container
      └── log₂s levels
          ├── Level 0: s₀ = s buckets (b=16 slots each)
          ├── Level 1: s₁ = s/2 buckets (b=16 slots each)
          ├── Level 2: s₂ = s/4 buckets (b=16 slots each)
          └── Overflow arrays: sᵢ slots per level
```

## Core Concepts

### 1. Container Hierarchy

**Why:** Divide large array into smaller containers for better load balancing.

**Structure:**
- Total containers: n/log n
- Items per container: s = 16·log n
- Total capacity: ~n elements

**Example (n=1000):**
- Containers: 1000 / 6.6 ≈ 151
- Items per container: 16 · 6.6 ≈ 106
- Total capacity: 151 · 106 ≈ 16,000 slots

### 2. Power-of-Two-Choices

**Why:** Load balancing through choice reduces max load from log n / log log n to log log n.

**Algorithm:**
1. Hash key to 2 container candidates
2. Choose less-loaded container
3. Insert into chosen container

**Implementation:**
```rust
let c1 = (hash1(&key) as usize) % self.containers.len();
let c2 = (hash2(&key) as usize) % self.containers.len();
let target = if self.containers[c1].item_count < self.containers[c2].item_count {
    &mut self.containers[c1]
} else {
    &mut self.containers[c2]
};
```

### 3. Hierarchical Levels

**Why:** Divide container into levels for progressive overflow handling.

**Structure:**
- Level 0: s₀ = s buckets (primary storage)
- Level 1: s₁ = s/2 buckets (first overflow)
- Level 2: s₂ = s/4 buckets (second overflow)
- ...
- Level i: sᵢ = s/2ⁱ buckets

**Insertion:** Try level 0, overflow to level 1, then level 2, etc.

### 4. Overflow Arrays

**Why:** Deterministic fallback prevents probe sequences from growing indefinitely.

**Structure:**
- Each level has dedicated overflow array
- Overflow capacity: sᵢ slots per level
- Total overflow: Σ sᵢ < 2s slots

**Guarantee:** No probe sequence exceeds log₂s + 1 hops.

### 5. Reverse Index

**Why:** Fast key validation and duplicate detection.

**Structure:**
- `HashMap<u64, Handle>` mapping key → handle
- O(1) lookup for existence check
- O(1) removal without scanning containers

**Usage:**
```rust
// Check if key exists
if self.key_to_ptr.contains_key(&key) {
    return Err(Error::DuplicateKey);
}

// Remove without scanning
if let Some(handle) = self.key_to_ptr.remove(&key) {
    // Free container slot
}
```

## Data Structures

### TinyPtrMap<V>

**Fields:**
```rust
pub struct TinyPtrMap<V> {
    containers: Vec<Container>,
    key_to_ptr: HashMap<u64, Handle>,
    arena: Vec<V>,
    config: Config,
}
```

**Responsibilities:**
- `containers`: Main storage hierarchy
- `key_to_ptr`: Reverse index for validation
- `arena`: Dense storage for actual values
- `config`: Sizing parameters

### Container

**Fields:**
```rust
struct Container {
    buckets: Vec<Vec<Option<NonZeroU32>>>,
    overflow: Vec<Vec<NonZeroU32>>,
    item_count: usize,
    s: usize,
    b: usize,
}
```

**Responsibilities:**
- `buckets`: Multi-level bucket array
- `overflow`: Per-level overflow arrays
- `item_count`: Load tracking
- `s`: Total capacity
- `b`: Bucket size (fixed at 16)

**Level Calculation:**
```rust
fn levels(&self) -> usize {
    (self.s as f64).log2() as usize
}

fn level_capacity(&self, level: usize) -> usize {
    self.s / (1 << level)  // s / 2^level
}
```

### Config

**Fields:**
```rust
struct Config {
    n: usize,
    s: usize,
    b: usize,
    num_containers: usize,
}
```

**Default Values:**
- `s = 16 · log₂ n`
- `b = 16` (bucket size)
- `num_containers = n / log₂ n`

## Algorithms

### Insert Algorithm

**Steps:**
1. **Duplicate check:** Verify key not in reverse index
2. **Container selection:** Power-of-two-choices
3. **Level insertion:** Try each level sequentially
4. **Overflow fallback:** Use overflow array if bucket full
5. **Index update:** Add to reverse index

**Pseudocode:**
```
function insert(key, value):
    if key in reverse_index:
        return Error::DuplicateKey

    c1 = hash1(key) % num_containers
    c2 = hash2(key) % num_containers
    container = select_less_loaded(c1, c2)

    for level in 0..container.levels:
        bucket = hash_level(key, level) % container.level_capacity(level)
        if bucket.has_empty_slot:
            bucket.insert(value)
            container.item_count++
            reverse_index[key] = value
            return Ok(())

    # All buckets full, try overflow
    for level in 0..container.levels:
        if container.overflow[level].has_empty_slot:
            container.overflow[level].insert(value)
            container.item_count++
            reverse_index[key] = value
            return Ok(())

    return Error::ContainerFull
```

**Complexity:**
- Expected: O(1) (power-of-two-choices + low load factor)
- Worst-case: O(log log n) (all levels full, negligible probability)

### Get Algorithm

**Steps:**
1. **Reverse index lookup:** O(1) check
2. **Return:** Early exit if not found

**Pseudocode:**
```
function get(key):
    if key not in reverse_index:
        return None

    # Key exists, return value (no container scan needed)
    return Some(reverse_index[key])
```

**Complexity:** O(1) guaranteed (single HashMap lookup)

**Note:** Container scan NOT needed due to reverse index.

### Remove Algorithm

**Steps:**
1. **Reverse index lookup:** Get handle if exists
2. **Container deallocation:** Free slot (no scan)
3. **Index update:** Remove from reverse index

**Pseudocode:**
```
function remove(key):
    if key not in reverse_index:
        return None

    handle = reverse_index[key]

    # Free from container (no scan needed)
    # Implementation: zero the slot in arena
    arena[handle] = None

    # Remove from reverse index
    reverse_index.remove(key)

    return Some(handle)
```

**Complexity:** O(1) (single HashMap remove + arena write)

**Note:** Does NOT find and free container slot (leaks slot, but safe).

## Memory Layout

### Overall Structure

```
┌─────────────────────────────────────────┐
│ TinyPtrMap                              │
├─────────────────────────────────────────┤
│ containers: Vec<Container>            │
│   ┌─────────────────────────────────┐  │
│   │ Container 0                    │  │
│   │ ├── buckets: Vec<Vec<Option>>  │  │
│   │ │   ├── Level 0: [bucket...]  │  │
│   │ │   ├── Level 1: [bucket...]  │  │
│   │ │   └── Level 2: [bucket...]  │  │
│   │ ├── overflow: Vec<Vec>         │  │
│   │ │   ├── Level 0: [slot...]    │  │
│   │ │   ├── Level 1: [slot...]    │  │
│   │ │   └── Level 2: [slot...]    │  │
│   │ └── item_count: usize          │  │
│   └─────────────────────────────────┘  │
│   ┌─────────────────────────────────┐  │
│   │ Container 1                    │  │
│   │ └── ...                        │  │
│   └─────────────────────────────────┘  │
│ key_to_ptr: HashMap<u64, Handle>     │
│ arena: Vec<NonZeroU32>                │
│ config: Config                        │
└─────────────────────────────────────────┘
```

### Container Layout

```
Container (s=106, b=16, levels=3)
├── Level 0: 106 buckets × 16 slots = 1696 slots
├── Level 1: 53 buckets × 16 slots = 848 slots
├── Level 2: 26 buckets × 16 slots = 416 slots
└── Overflow: 106 + 53 + 26 = 185 slots
```

**Total per container:** ~3145 slots (including overflow)

### Memory Calculation

**For n=1000:**
- Containers: 151
- Slots per container: ~3145
- Total slots: 151 · 3145 ≈ 475,000 slots
- Slot size: 4 bytes (NonZeroU32)
- Total memory: 475,000 · 4 ≈ 1.9 MB

**Overhead factor:** ~2× (generous capacity for low load factor)

## Hash Functions

### Hash Independence

**Theoretical requirement:** Independent hash functions per level

**Our approximation:** Level-specific seeds with `DefaultHasher`

```rust
use std::collections::hash_map::DefaultHasher;

fn hash_level(key: &u64, level: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    // Add level-specific seed
    (level as u64).hash(&mut hasher);
    hasher.finish()
}
```

**Known deviation:** Not perfectly independent (inherent to construction)

### Hash Usage

**Container selection:**
```rust
let c1 = hash_level(&key, 0) as usize % num_containers;
let c2 = hash_level(&key, 1) as usize % num_containers;
```

**Bucket selection:**
```rust
let bucket = hash_level(&key, level) as usize % level_capacity(level);
```

**Seeding:** Level number added as seed approximation

## Load Balancing

### Power-of-Two-Choices

**Goal:** Reduce maximum load from log n / log log n to log log n

**Mechanism:**
1. Hash key to 2 candidates
2. Choose candidate with fewer items
3. Insert into chosen container

**Implementation:**
```rust
let c1 = (hash1(&key) as usize) % num_containers;
let c2 = (hash2(&key) as usize) % num_containers;
let target = if containers[c1].item_count < containers[c2].item_count {
    c1
} else {
    c2
};
```

**Effect:** With high probability, max load ≈ log log n (vs log n / log log n)

### Overflow Handling

**Strategy:** Progressive overflow through hierarchy

**Levels:**
1. Try primary bucket (level 0)
2. Try level 1 overflow
3. Try level 2 overflow
4. ...
5. Return error if all full

**Guarantee:** Probe sequence ≤ levels + 1 (deterministic)

## Tradeoffs

### Space vs Time

**Space savings:**
- Traditional: log₂ n bits per pointer
- Tiny pointers: log k bits expected
- **Example:** n=1M → 20 bits → 4 bits (75% reduction)

**Time cost:**
- Traditional: O(1) with low constant
- Tiny pointers: O(1) with higher constant (hierarchy traversal)

**Decision:** Trade time for space (memory-constrained workloads)

### Simplicity vs Theory

**Our choice:** 2-level hierarchy (simplified)

**Full theory:** Multi-level funnel cascade

**Tradeoff:**
- Simpler implementation
- Worse theoretical bounds
- Easier to debug and maintain

**Decision:** Validate prototype before adding complexity

### Determinism vs Efficiency

**Our choice:** Overflow arrays (deterministic fallback)

**Alternative:** Pure probing (probabilistic)

**Tradeoff:**
- Overflow: Memory overhead, guaranteed bounds
- Probing: Zero overhead, probabilistic bounds

**Decision:** Determinism preferred for correctness

## Future Enhancements

### 1. Bit-Packed Encoding

**Current:** Uses NonZeroU32 (32 bits)

**Target:** Θ(log k) bits per pointer

**Challenge:** Efficient encoding/decoding without CPU overhead

**Approach:**
```rust
enum TinyPointer {
    Inline(u8),           // ≤255
    Short(u16),           // ≤65535
    Medium(u32),          // ≤4B
}
```

**Expected:** 2-4× space reduction

### 2. Multi-Level Funnel

**Current:** 2-level hierarchy

**Target:** log n levels (full funnel cascade)

**Benefit:** Better theoretical bounds at high load factors

**Challenge:** Implementation complexity

### 3. Dynamic Bucket Sizing

**Current:** Fixed b=16

**Target:** Dynamic β = 2·log ε⁻¹

**Benefit:** Adaptive to load factor

**Challenge:** Resize logic and rebalancing

### 4. Hash Function Optimization

**Current:** DefaultHasher with level seeds

**Target:** Cryptographic independent hashes

**Benefit:** Closer to theoretical assumptions

**Challenge:** Performance overhead

## Implementation Notes

### Arithmetic Accuracy

**Fixed:** Integer truncation via `.ceil()`

```rust
let s_i = (config.s as f64 / (1u64 << i) as f64).ceil() as usize;
```

**Before fix:** Truncation caused capacity drift at high load factors

**Result:** 100% insertion success at 0.9 load factor (was 99.6%)

### Safety

**Miri clean:** Zero undefined behavior detected

**Safety properties:**
- No raw pointer dereferencing
- No unsafe code outside arena operations
- Proper bounds checking on all array accesses

### Performance

**Benchmark results:** See [BENCHMARKS.md](https://github.com/oldnordic/tinypointers/blob/main/BENCHMARKS.md)

**Key metrics:**
- 428M ops/sec insert (vs hashbrown 55M ops/sec)
- ~10.1ns sequential access
- ~12.5ns random access

## References

- [Tiny Pointers paper](https://arxiv.org/abs/2111.12800) — Theoretical foundation
- [Funnel Hashing](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_HASHING_BREAKTHROUGH.md) — 2025 breakthrough
- [hashbrown source](https://github.com/rust-lang/hashbrown) — Architecture reference

## See Also

- [API Manual](https://github.com/oldnordic/tinypointers/blob/main/docs/API_MANUAL.md) — User-facing API
- [README](https://github.com/oldnordic/tinypointers/blob/main/README.md) — Quick start
- [MATHEMATICAL_FRAMEWORK](https://github.com/oldnordic/tinypointers/blob/main/docs/MATHEMATICAL_FRAMEWORK.md) — Theory