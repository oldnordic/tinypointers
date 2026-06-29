# Funnel Cascade Implementation Plan

Implementation plan for optional multi-level Funnel Hashing cascade feature.

## Overview

**Goal:** Implement full Funnel Hashing cascade with log n levels as optional feature.

**Reference:** [docs/FUNNEL_CASCADE_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_CASCADE_RESEARCH.md)

**Feature Flag:** `funnel-cascade`

**Estimated Effort:** 2 weeks

## Phase 1: Research & Study (Days 1-2)

### Task 1.1: Study opthash Implementation

**Reference:** [GitHub: aaron-ang/opthash-rs](https://github.com/aaron-ang/opthash-rs)

**Focus Areas:**
- Cascade structure and coordination
- Level-specific hash function design
- Bucket sizing calculation (β = 2·log ε⁻¹)
- Overflow table management
- Insert/get/remove algorithms

**Deliverable:** Research notes on implementation patterns

### Task 1.2: Analyze Current 2-Level Structure

**File:** `src/lib.rs`

**Analysis:**
- Document current 2-level approach
- Identify extension points for multi-level
- Map opthash patterns to current code

**Deliverable:** Extension point documentation

## Phase 2: Foundation (Days 3-4)

### Task 2.1: Add Feature Flag

**File:** `Cargo.toml`

**Changes:**
```toml
[features]
default = []
funnel-cascade = []
full = ["funnel-cascade", "bit-packed"]
```

**Verification:**
```bash
cargo build --features funnel-cascade
cargo build --no-default-features
```

### Task 2.2: Create Funnel Module Structure

**File:** `src/lib.rs`

**Changes:**
```rust
#[cfg(feature = "funnel-cascade")]
mod funnel {
    use super::*;

    pub struct FunnelConfig {
        pub bucket_size: usize,
        pub num_levels: usize,
        pub epsilon: f64,
    }

    impl FunnelConfig {
        pub fn new(n: usize, epsilon: f64) -> Self {
            let bucket_size = (2.0 * epsilon.log2()).ceil() as usize;
            let num_levels = (n as f64).log2().ceil() as usize;
            Self {
                bucket_size,
                num_levels,
                epsilon,
            }
        }
    }

    // Multi-level operations
    pub fn insert_funnel(/* ... */) -> Result<(), Error> { }
    pub fn get_funnel(/* ... */) -> Option<Handle> { }
    pub fn remove_funnel(/* ... */) -> Option<Handle> { }
}

#[cfg(not(feature = "funnel-cascade"))]
mod funnel {
    // Current 2-level implementation
    pub fn insert_funnel(/* ... */) -> Result<(), Error> { }
    // ...
}
```

**Verification:**
```bash
cargo check --features funnel-cascade
cargo check --no-default-features
```

## Phase 3: Implementation (Days 5-10)

### Task 3.1: Implement FunnelConfig

**File:** `src/lib.rs` (funnel module)

**Requirements:**
```rust
impl FunnelConfig {
    /// Calculate optimal bucket size β = 2·log ε⁻¹
    fn calculate_bucket_size(epsilon: f64) -> usize {
        (2.0 * (1.0 / epsilon).log2()).ceil() as usize
    }

    /// Calculate number of levels (log₂ n)
    fn calculate_num_levels(n: usize) -> usize {
        (n as f64).log2().ceil() as usize
    }

    /// Calculate level capacity (progressive sizing)
    fn level_capacity(&self, level: usize) -> usize {
        if level == 0 {
            self.bucket_size
        } else {
            self.bucket_size * (1 << level)
        }
    }
}
```

**Tests:**
- Verify bucket size formula matches paper
- Test level capacity progression
- Validate config for various n, ε values

### Task 3.2: Implement Multi-Level Insert

**File:** `src/lib.rs` (funnel module)

**Requirements:**
```rust
pub fn insert_funnel(
    containers: &mut Vec<Container>,
    config: &FunnelConfig,
    key: u64,
    value: Handle
) -> Result<(), Error> {
    // Level-specific hash function
    for level in 0..config.num_levels {
        let bucket_size = config.level_capacity(level);
        
        // Try primary bucket
        if try_insert_level(containers, key, value, level, bucket_size)? {
            return Ok(());
        }

        // Try overflow at this level
        if try_insert_overflow(containers, key, value, level)? {
            return Ok(());
        }
    }

    // All levels exhausted
    Err(Error::ContainerFull)
}
```

**Tests:**
- Insert succeeds at low load factors
- Insert succeeds at 95% load factor
- Proper overflow cascade behavior
- Error handling when full

### Task 3.3: Implement Level-Specific Hashing

**File:** `src/lib.rs` (funnel module)

**Requirements:**
```rust
struct LevelHasher {
    level: usize,
    seed: u64,
    hash_fn: fn(u64, u64) -> u64,
}

impl LevelHasher {
    fn new(level: usize) -> Self {
        Self {
            level,
            seed: level as u64 * 0x9E3779B9744C7A5, // Unique per level
            hash_fn: DefaultHasher::default,
        }
    }

    fn hash(&self, key: u64, capacity: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        self.seed.hash(&mut hasher);
        key.hash(&mut hasher);
        (hasher.finish() as usize) % capacity
    }
}
```

**Tests:**
- Hash independence validation
- Different levels produce different hashes
- Uniform distribution verification

### Task 3.4: Implement Multi-Level Get

**File:** `src/lib.rs` (funnel module)

**Requirements:**
```rust
pub fn get_funnel(
    containers: &Vec<Container>,
    config: &FunnelConfig,
    key: u64
) -> Option<Handle> {
    // Check reverse index first (O(1))
    if let Some(handle) = reverse_index.get(&key) {
        return Some(*handle);
    }

    // Cascade through levels to find key
    for level in 0..config.num_levels {
        if let Some(handle) = search_level(containers, key, level) {
            return Some(handle);
        }
    }

    None
}
```

**Tests:**
- Get returns correct handle
- Get returns None for missing keys
- Reverse index lookup works
- Cascade search works when reverse index miss

### Task 3.5: Implement Multi-Level Remove

**File:** `src/lib.rs` (funnel module)

**Requirements:**
```rust
pub fn remove_funnel(
    containers: &mut Vec<Container>,
    reverse_index: &mut HashMap<u64, Handle>,
    key: u64
) -> Option<Handle> {
    // Remove from reverse index
    if let Some(handle) = reverse_index.remove(&key) {
        // Free from container (no scan needed)
        // Note: Does not free container slot (known limitation)
        Some(handle)
    } else {
        None
    }
}
```

**Tests:**
- Remove returns correct handle
- Remove returns None for missing keys
- Reverse index cleanup
- Subsequent get returns None

## Phase 4: Integration (Days 11-12)

### Task 4.1: Update TinyPtrMap

**File:** `src/lib.rs` (TinyPtrMap)

**Changes:**
- Add `FunnelConfig` field to struct
- Update `new()` to create config
- Route operations through funnel module

**Verification:**
```bash
cargo test --features funnel-cascade
cargo test --no-default-features
```

### Task 4.2: Update Reverse Index

**File:** `src/lib.rs` (TinyPtrMap)

**Changes:**
- Ensure reverse index compatible with funnel
- Handle multi-level pointer encoding
- Update `get()` to use funnel lookup

**Verification:**
```bash
cargo test --features funnel-cascade
```

## Phase 5: Testing (Days 13-14)

### Task 5.1: Add Unit Tests

**File:** `src/lib.rs` (tests module)

**Tests:**
```rust
#[cfg(test)]
mod funnel_tests {
    use super::*;

    #[test]
    fn test_funnel_config() {
        let config = FunnelConfig::new(1000, 0.1);
        assert_eq!(config.num_levels, 10); // log₂10
        assert!(config.bucket_size > 0);
    }

    #[test]
    fn test_high_load_factor() {
        // Test 95% load factor
        let n = 1000;
        let epsilon = 0.05;
        let config = FunnelConfig::new(n, epsilon);
        // Insert 950 items, verify >90% success
    }

    #[test]
    fn test_cascade_overflow() {
        // Fill primary, verify cascade to level 1
        // Fill level 1, verify cascade to level 2
    }

    #[test]
    fn test_level_specific_hashing() {
        // Verify different levels produce different hashes
        // Verify uniform distribution
    }
}
```

### Task 5.2: Add Integration Tests

**File:** `tests/integration_funnel.rs`

**Tests:**
- Full API with multi-level cascade
- High load factor operations
- Performance vs 2-level approach
- Memory footprint comparison

## Phase 6: Benchmarking (Days 15-17)

### Task 6.1: High Load Factor Benchmarks

**File:** `benches/funnel_high_load.rs`

**Metrics:**
```rust
#[bench]
fn bench_funnel_95_load(b: &mut Criterion) {
    // Compare vs 2-level at 95% load
}

#[bench]
fn bench_funnel_99_load(b: &mut Criterion) {
    // Test at 99% load (theoretically O(1))
}
```

### Task 6.2: Cascade Overhead Benchmarks

**File:** `benches/funnel_cascade.rs`

**Metrics:**
```rust
#[bench]
fn bench_multi_level_iterations(b: &mut Criterion) {
    // Measure iteration count vs 2-level
}

#[bench]
fn bench_level_specific_hashing(b: &mut Criterion) {
    // Compare hash computation overhead
}
```

## Success Criteria

**Functional:**
- ✅ All tests pass with both feature combinations
- ✅ No regressions in existing functionality
- ✅ O(1) operations at 95%+ load factor

**Performance:**
- ✅ Competitive with 2-level at low load factors
- ✅ Superior to 2-level at high load factors (90%+)
- ✅ Acceptable overhead from multiple levels

**Space:**
- ✅ Better space utilization than 2-level
- ✅ Reduced overflow probability
- ✅ Measurable capacity increase

**Documentation:**
- ✅ Feature documented in README
- ✅ Implementation details in ARCHITECTURE
- ✅ Funnel Hashing theory preserved

## Verification Plan

**Pre-commit:**
```bash
cargo test --features funnel-cascade
cargo test --no-default-features
cargo clippy --all-targets -- -D warnings
```

**Post-commit:**
```bash
cargo bench --features funnel-cascade
cargo bench --no-default-features
# Compare results
```

## Risk Mitigation

**Complexity Risk:**
- Study opthash thoroughly before coding
- Prototype cascade logic separately
- Test incrementally per level

**Performance Risk:**
- Benchmark early and often
- Compare against 2-level baseline
- Be prepared to keep as optional feature

**Compatibility Risk:**
- Ensure cfg isolation complete
- Test both feature combinations
- Document tradeoffs clearly

## Rollout Strategy

**Phase 1:** Research-only (no code)
**Phase 2:** Prototype and validate
**Phase 3:** Full implementation (optional)
**Phase 4:** Enable if beneficial, else keep optional

## References

- Research: [docs/FUNNEL_CASCADE_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_CASCADE_RESEARCH.md)
- opthash implementation: [GitHub: aaron-ang/opthash-rs](https://github.com/aaron-ang/opthash-rs)
- Funnel Hashing paper: [docs/RESEARCH_CONTEXT.md](https://github.com/oldnordic/tinypointers/blob/main/docs/RESEARCH_CONTEXT.md)
- Breakthrough documentation: [docs/FUNNEL_HASHING_BREAKTHROUGH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_HASHING_BREAKTHROUGH.md)