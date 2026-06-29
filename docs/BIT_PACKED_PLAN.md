# Bit-Packed Encoding Implementation Plan

Implementation plan for optional bit-packed handle encoding feature.

## Overview

**Goal:** Implement Θ(log k) bit pointer compression as optional feature.

**Reference:** [docs/BIT_PACKED_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/BIT_PACKED_RESEARCH.md)

**Feature Flag:** `bit-packed`

**Estimated Effort:** 1 week

## Phase 1: Foundation (Day 1)

### Task 1.1: Add Feature Flag

**File:** `Cargo.toml`

**Changes:**
```toml
[features]
default = []
bit-packed = []
```

**Verification:**
```bash
cargo build --features bit-packed
cargo build --no-default-features
```

### Task 1.2: Create Handle Module Structure

**File:** `src/lib.rs`

**Changes:**
```rust
#[cfg(feature = "bit-packed")]
mod handle {
    pub type Handle = BitPackedHandle;

    pub struct BitPackedHandle(u8);

    impl BitPackedHandle {
        // Encoding logic
    }

    impl Debug for BitPackedHandle {
        // Debug implementation
    }
}

#[cfg(not(feature = "bit-packed"))]
mod handle {
    pub type Handle = std::num::NonZeroU32;
}
```

**Verification:**
```bash
cargo check --features bit-packed
cargo check --no-default-features
```

## Phase 2: Bit-Packed Implementation (Days 2-4)

### Task 2.1: Implement Encoding Logic

**File:** `src/lib.rs` (handle module)

**Requirements:**
```rust
impl BitPackedHandle {
    /// Encode level and slot into 8-bit handle
    ///
    /// Format: [level: 2 bits][slot: 6 bits]
    /// Range: level 0-3, slot 0-63
    pub fn encode(level: u8, slot: u16) -> Self {
        assert!(level < 4, "level must be 0-3");
        assert!(slot < 64, "slot must be 0-63");
        let data = ((level as u8) << 6) | ((slot as u8) & 0x3F);
        Self(data)
    }

    /// Decode handle into level and slot
    pub fn decode(&self) -> (u8, u16) {
        let level = (self.0 >> 6) & 0x03;
        let slot = (self.0 & 0x3F) as u16;
        (level, slot)
    }
}
```

**Test Requirements:**
- Round-trip encoding/decoding
- Edge cases: (0, 0), (3, 63)
- Invalid input handling

### Task 2.2: Update Container Operations

**File:** `src/lib.rs` (Container methods)

**Changes:**
- Replace `NonZeroU32` with generic `Handle`
- Update arena indexing to work with both handle types
- Ensure compatibility with current reverse index

**Verification:**
```bash
cargo test --features bit-packed
cargo test --no-default-features
```

### Task 2.3: Update TinyPtrMap Operations

**File:** `src/lib.rs` (TinyPtrMap methods)

**Changes:**
- Update `insert()` to use new Handle type
- Update `get()` to handle both handle types
- Update `remove()` accordingly

**Verification:**
```bash
cargo test --lib --features bit-packed
cargo test --lib --no-default-features
```

## Phase 3: Testing (Days 5-6)

### Task 3.1: Add Unit Tests

**File:** `src/lib.rs` (tests module)

**Tests:**
```rust
#[cfg(test)]
mod bit_packed_tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        for level in 0..4 {
            for slot in 0..64 {
                let handle = BitPackedHandle::encode(level, slot);
                let (decoded_level, decoded_slot) = handle.decode();
                assert_eq!(decoded_level, level);
                assert_eq!(decoded_slot, slot);
            }
        }
    }

    #[test]
    fn test_space_efficiency() {
        assert_eq!(std::mem::size_of::<BitPackedHandle>(), 1);
        assert_eq!(std::mem::size_of::<NonZeroU32>(), 4);
    }

    #[test]
    fn test_edge_cases() {
        let handle = BitPackedHandle::encode(0, 0);
        assert_eq!(handle.decode(), (0, 0));

        let handle = BitPackedHandle::encode(3, 63);
        assert_eq!(handle.decode(), (3, 63));
    }
}
```

### Task 3.2: Add Integration Tests

**File:** `tests/integration_bit_packed.rs`

**Tests:**
- Full API surface with bit-packed handles
- Insert/remove/get cycles
- Memory footprint comparison
- Performance regression tests

## Phase 4: Benchmarking (Day 7)

### Task 4.1: Space Benchmarks

**File:** `benches/bit_packed_space.rs`

**Metrics:**
```rust
#[bench]
fn bench_handle_size(b: &mut Criterion) {
    // Compare BitPackedHandle vs NonZeroU32
}

#[bench]
fn bench_map_memory_footprint(b: &mut Criterion) {
    // Compare total memory usage
}
```

### Task 4.2: Performance Benchmarks

**File:** `benches/bit_packed_perf.rs`

**Metrics:**
```rust
#[bench]
fn bench_insert_bit_packed(b: &mut Criterion) {
    // Measure insert throughput
}

#[bench]
fn bench_get_bit_packed(b: &mut Criterion) {
    // Measure lookup performance
}
```

## Success Criteria

**Functional:**
- ✅ All tests pass with both feature combinations
- ✅ No regressions in existing functionality
- ✅ Zero compiler warnings

**Space:**
- ✅ 4× reduction in handle storage (1 byte vs 4 bytes)
- ✅ Measurable reduction in total map footprint

**Performance:**
- ✅ CPU overhead <20% per operation
- ✅ No significant slowdown in workloads

**Documentation:**
- ✅ Feature documented in README
- ✅ Implementation details in ARCHITECTURE
- ✅ Usage examples in API_MANUAL

## Verification Plan

**Pre-commit:**
```bash
cargo test --features bit-packed
cargo test --no-default-features
cargo clippy --all-targets -- -D warnings
```

**Post-commit:**
```bash
cargo bench --features bit-packed
cargo bench --no-default-features
# Compare results
```

## Rollout Strategy

**Phase 1:** Feature flag only (default disabled)
**Phase 2:** Benchmark validation
**Phase 3:** Enable if beneficial, else keep optional

## References

- Research: [docs/BIT_PACKED_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/BIT_PACKED_RESEARCH.md)
- bitpacking crate: [GitHub: quickwit-oss/bitpacking](https://github.com/quickwit-oss/bitpacking)
- packed_struct crate: [crates.io/crates/packed_struct](https://crates.io/crates/packed_struct)