# Bit-Packed Encoding Research

Research findings for implementing Θ(log k) bit pointer compression in tinypointers.

## Current State Analysis

**Current Implementation:**
```rust
// 32-bit handles
pub type Handle = NonZeroU32;

// TinyPointer enum (not optimized)
pub enum TinyPointer {
    LoadBalancer { level: u8, slot: u16 },    // 24 bits
    Overflow { level_from_back: u8, slot: u32 } // 40 bits
}
```

**Space Analysis:**
- `NonZeroU32`: 32 bits per handle
- `TinyPointer`: 24-40 bits per handle (discriminant overhead)
- Target: Θ(log k) bits → For k=10: ~4 bits per handle

**Space Savings Potential:**
- Current: 32 bits per handle
- Target: 4-8 bits per handle
- **Potential reduction: 4-8×**

## Research Findings

### 1. bitpacking Crate (quickwit-oss)

**Source:** [GitHub: quickwit-oss/bitpacking](https://github.com/quickwit-oss/bitpacking)

**Capabilities:**
- SIMD-accelerated integer compression
- Throughput: >4 billion integers per second
- Best for: Sequential integer compression
- Works with: Groups of 32/128/256 integers

**Relevance to Tinypointers:**
- ✅ High-performance encoding/decoding
- ❌ Designed for sequential data (not random access)
- ❌ Batch-oriented (not single-pointer operations)

**Verdict:** Not directly suitable for random-access handle encoding.

### 2. packed_struct Crate

**Source:** [crates.io: packed_struct](https://crates.io/crates/packed_struct)

**Capabilities:**
- Bit-level struct packing via attributes
- Meta-programming approach
- Best for: Protocol/file format structures

**Example:**
```rust
use packed_struct::PackedStruct;

#[derive(PackedStruct)]
#[packed_struct(endian = "lsb")]
struct BitPackedHandle {
    #[packed_struct(bits = "2")]
    level: u8,
    #[packed_struct(bits = "6")]
    slot: u8,
}
// Total: 8 bits
```

**Relevance to Tinypointers:**
- ✅ Precise bit control
- ✅ Compile-time safety
- ✅ Suitable for random access
- ❌ Adds dependency
- ❌ May have encoding/decoding overhead

**Verdict:** Good candidate for cfg-based implementation.

### 3. Custom #[repr(u8)] Enum Approach

**Source:** [Stack Overflow: Minimal enum size](https://stackoverflow.com/questions/77377069/how-to-pack-a-rust-enum-into-its-minimal-size)

**Technique:**
```rust
#[repr(u8)]
enum TinyPointer {
    Inline(u8),        // 0-255: 1 bit discriminant + 8 bits data
    Short(u16),        // 256-65535: 2 bit discriminant + 16 bits data
    Medium(u32),       // 64K-4B: 4 bit discriminant + 32 bits data
}
```

**Space Analysis:**
- `Inline(u8)`: 9 bits total (1 discriminant + 8 data)
- `Short(u16)`: 18 bits total (2 discriminant + 16 data)
- `Medium(u32)`: 36 bits total (4 discriminant + 32 data)

**Relevance to Tinypointers:**
- ✅ Standard Rust (no external dependencies)
- ✅ Clear semantics
- ❌ Still larger than target Θ(log k) bits
- ❌ Enum discriminant overhead unavoidable

**Verdict:** Better than current, but not optimal.

### 4. Custom Bit Manipulation

**Source:** [Blog: Engineering Fixed-Width Bit-Packed Integer Vector](https://lukefleed.xyz/posts/compressed-fixedvec/)

**Technique:**
```rust
struct BitPackedHandle {
    data: u64,  // Packed with custom encoding
}

impl BitPackedHandle {
    // Encode: level (2 bits) + slot (6 bits) = 8 bits
    fn encode(level: u8, slot: u16) -> u64 {
        ((level as u64) << 6) | (slot as u64 & 0x3F)
    }

    // Decode: extract level and slot
    fn decode(data: u64) -> (u8, u16) {
        let level = (data >> 6) as u8;
        let slot = (data & 0x3F) as u16;
        (level, slot)
    }
}
```

**Space Analysis:**
- Custom encoding: Exact bit count
- For k=10 (log₂10 ≈ 3.3 bits): Can use 4 bits per handle
- **9× space reduction vs NonZeroU32**

**Relevance to Tinypointers:**
- ✅ Zero dependencies
- ✅ Exact bit control
- ✅ CPU-efficient (bit operations)
- ❌ Manual encoding/decoding
- ❌ Error-prone if not careful

**Verdict:** Best approach for cfg-based optional feature.

## Implementation Strategy

### Proposed Design: cfg-Based Bit Packing

**Cargo.toml:**
```toml
[features]
default = []
bit-packed = []
```

**Implementation (src/lib.rs):**
```rust
#[cfg(feature = "bit-packed")]
mod handle {
    // Bit-packed handles: 4-8 bits
    pub type Handle = BitPackedHandle;

    pub struct BitPackedHandle(u8);

    impl BitPackedHandle {
        pub fn encode(level: u8, slot: u16) -> Self {
            // Custom encoding logic
            Self((level << 6) | (slot as u8 & 0x3F))
        }

        pub fn decode(&self) -> (u8, u16) {
            let level = self.0 >> 6;
            let slot = (self.0 & 0x3F) as u16;
            (level, slot)
        }
    }
}

#[cfg(not(feature = "bit-packed"))]
mod handle {
    // Current: 32-bit handles
    pub type Handle = std::num::NonZeroU32;
}
```

**Tradeoffs:**
- **Space:** 4-8 bits vs 32 bits (4-8× reduction)
- **CPU:** Bit operations vs direct access (small overhead)
- **Complexity:** Encoding/decoding logic (maintenance cost)

### Performance Considerations

**Encoding Cost:**
- Bit shifts and masks: ~1-2 CPU cycles
- Comparable to HashMap hash computation

**Decoding Cost:**
- Bit operations on every get/remove
- May offset space savings in CPU-bound workloads

**Memory Benefit:**
- 4-8× reduction in pointer storage
- Better cache utilization
- Potential overall performance win

### Testing Strategy

**Validation Tests:**
```rust
#[cfg(test)]
mod bit_packed_tests {
    #[test]
    fn test_encode_decode_roundtrip() {
        let handle = BitPackedHandle::encode(5, 42);
        let (level, slot) = handle.decode();
        assert_eq!(level, 5);
        assert_eq!(slot, 42);
    }

    #[test]
    fn test_space_efficiency() {
        let handles: Vec<BitPackedHandle> = (0..1000)
            .map(|i| BitPackedHandle::encode(i % 4, i as u16))
            .collect();
        assert_eq!(std::mem::size_of_val::<BitPackedHandle>(), 1);
    }
}
```

**Benchmark Comparison:**
```rust
#[cfg(feature = "bit-packed")]
#[bench]
fn bench_bit_packed_insert(b: &mut Criterion) {
    // Compare vs NonZeroU32 baseline
}

#[cfg(not(feature = "bit-packed"))]
#[bench]
fn bench_standard_insert(b: &mut Criterion) {
    // Current implementation
}
```

## Research Summary

**Best Approach:** Custom bit manipulation (cfg-gated)

**Reasoning:**
1. Zero dependencies
2. Exact bit control (Θ(log k) achievable)
3. CPU-efficient (simple bit ops)
4. Easy to toggle via features

**Implementation Priority:** High (clear payoff, low risk)

**Estimated Effort:** 1 week
- Research: 1 day (complete)
- Implementation: 3-4 days
- Testing/Benchmarking: 2 days

**Success Criteria:**
- 4-8× space reduction measured
- CPU overhead <20% per operation
- All tests pass with both features

## References

- [bitpacking crate (SIMD compression)](https://github.com/quickwit-oss/bitpacking)
- [packed_struct crate](https://crates.io/crates/packed_struct)
- [Stack Overflow: Enum size minimization](https://stackoverflow.com/questions/77377069/how-to-pack-a-rust-enum-into-its-minimal-size)
- [Bit-packed integer vector blog](https://lukefleed.xyz/posts/compressed-fixedvec/)