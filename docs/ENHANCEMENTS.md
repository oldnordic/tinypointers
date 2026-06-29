# Optional Enhancements Overview

Overview of optional future enhancements for tinypointers, including research findings and implementation plans.

## Available Enhancements

### 1. Bit-Packed Encoding

**Goal:** Implement Θ(log k) bit pointer compression

**Current State:**
- Uses `NonZeroU32` (32 bits per handle)
- Simple, fast, but not space-optimal

**Target:**
- 4-8 bits per handle
- Space savings: 4-8× reduction
- Example: For k=10 → ~4 bits (vs 32 bits)

**Implementation:**
- Feature flag: `bit-packed`
- Approach: Custom bit manipulation
- Dependencies: None
- Estimated effort: 1 week

**Research Status:** ✅ Complete
- [BIT_PACKED_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/BIT_PACKED_RESEARCH.md) — Full research findings

**Implementation Plan:** ✅ Ready
- [BIT_PACKED_PLAN.md](https://github.com/oldnordic/tinypointers/blob/main/docs/BIT_PACKED_PLAN.md) — Step-by-step implementation guide

### 2. Funnel Cascade

**Goal:** Implement multi-level Funnel Hashing cascade

**Current State:**
- 2-level hierarchy (primary + overflow)
- Fixed bucket size: b=16
- Simple level-specific seeding

**Target:**
- log n levels with progressive overflow
- Dynamic bucket sizing: β = 2·log ε⁻¹
- Independent hash functions per level
- O(1) operations at 95%+ load factor

**Implementation:**
- Feature flag: `funnel-cascade`
- Approach: Based on [opthash](https://github.com/aaron-ang/opthash-rs) patterns
- Dependencies: None
- Estimated effort: 2 weeks

**Research Status:** ✅ Complete
- [FUNNEL_CASCADE_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_CASCADE_RESEARCH.md) — Full research findings

**Implementation Plan:** ✅ Ready
- [FUNNEL_CASCADE_PLAN.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_CASCADE_PLAN.md) — Step-by-step implementation guide

## Comparison

| Aspect | Bit-Packed | Funnel Cascade | Current |
|--------|-------------|----------------|---------|
| **Space per pointer** | 4-8 bits | 32 bits | 32 bits |
| **Levels** | 2 (unchanged) | log n | 2 |
| **Bucket sizing** | Fixed | Dynamic | Fixed |
| **Hash independence** | Approximated | Improved | Approximated |
| **Load factor** | Good (90%) | Excellent (95%+) | Good (90%) |
| **Complexity** | Low | High | Low |
| **Effort** | 1 week | 2 weeks | - |
| **Dependencies** | None | None | - |

## Implementation Strategy

Both enhancements use cfg feature flags for optional compilation:

```toml
[features]
default = []
bit-packed = []
funnel-cascade = []
full = ["funnel-cascade", "bit-packed"]
```

**Usage:**
```bash
# Current (default)
cargo build

# Bit-packed handles only
cargo build --features bit-packed

# Funnel cascade only
cargo build --features funnel-cascade

# Both optimizations
cargo build --features full
```

## Research Sources

### Bit-Packed Encoding Research

**Crates Studied:**
- [bitpacking crate](https://github.com/quickwit-oss/bitpacking) — SIMD compression (not suitable for random access)
- [packed_struct crate](https://crates.io/crates/packed_struct) — Bit-level struct packing
- [Stack Overflow: Enum packing](https://stackoverflow.com/questions/77377069/how-to-pack-a-rust-enum-into-its-minimal-size)
- [Bit-packed integer vector blog](https://lukefleed.xyz/posts/compressed-fixedvec/)

**Key Findings:**
- Custom bit manipulation preferred for random access
- No external dependencies needed
- Simple bit operations: encoding/decoding

### Funnel Cascade Research

**Implementation Studied:**
- [opthash crate](https://github.com/aaron-ang/opthash-rs) — Production Funnel HashMap
- [opthash on Hacker News](https://news.ycombinator.com/item?id=48365720) — Community discussion

**Key Findings:**
- Reference implementation available
- Production-tested patterns
- Clear mapping from theory to code

## Implementation Order

**Recommended Sequence:**

1. **Bit-Packed Encoding (1 week)**
   - Lower risk, clear payoff
   - Simpler implementation
   - Validates cfg approach

2. **Funnel Cascade (2 weeks)**
   - Higher complexity, uncertain payoff
   - Requires studying opthash implementation
   - Only if bit-packed shows benefit

**Combined Approach:**
- Can implement independently
- Can benchmark separately
- Can enable/disable via features

## Success Criteria

### Bit-Packed Encoding

**Space:**
- 4× reduction in handle storage (1 byte vs 4 bytes)
- Measurable reduction in total map footprint

**Performance:**
- CPU overhead <20% per operation
- No significant slowdown in workloads

### Funnel Cascade

**Performance:**
- O(1) operations at 95%+ load factor
- Competitive with 2-level at low loads
- Better space utilization

**Compatibility:**
- All existing tests pass
- No regressions in functionality

## Documentation Map

**Research:**
- [BIT_PACKED_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/BIT_PACKED_RESEARCH.md)
- [FUNNEL_CASCADE_RESEARCH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_CASCADE_RESEARCH.md)

**Plans:**
- [BIT_PACKED_PLAN.md](https://github.com/oldnordic/tinypointers/blob/main/docs/BIT_PACKED_PLAN.md)
- [FUNNEL_CASCADE_PLAN.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_CASCADE_PLAN.md)

**Integration:**
- [SPEC.md](https://github.com/oldnordic/tinypointers/blob/main/SPEC.md) — Updated with optional enhancements
- [README.md](https://github.com/oldnordic/tinypointers/blob/main/README.md) — User-facing overview
- [docs/README.md](https://github.com/oldnordic/tinypointers/blob/main/docs/README.md) — Documentation index

## Next Steps

**To implement bit-packed encoding:**
1. Review [BIT_PACKED_PLAN.md](https://github.com/oldnordic/tinypointers/blob/main/docs/BIT_PACKED_PLAN.md)
2. Add `bit-packed` feature to Cargo.toml
3. Follow implementation phases (1 week)
4. Benchmark and validate

**To implement funnel cascade:**
1. Study [opthash implementation](https://github.com/aaron-ang/opthash-rs)
2. Review [FUNNEL_CASCADE_PLAN.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_CASCADE_PLAN.md)
3. Add `funnel-cascade` feature to Cargo.toml
4. Follow implementation phases (2 weeks)
5. Benchmark and validate at high load factors

**For both enhancements:**
- All research documented (no guessing required)
- Implementation plans ready (step-by-step guides)
- cfg flags prevent disruption (optional features)
- Can validate independently before committing

## Summary

**Research Phase:** ✅ Complete
- All web searches performed
- All implementations studied
- All findings documented

**Planning Phase:** ✅ Complete
- Implementation plans created
- Task breakdowns defined
- Success criteria established

**Implementation Phase:** ⏳ Ready
- Research complete (no guessing)
- Plans detailed (clear roadmap)
- Optional via features (no disruption)

Both enhancements are ready for implementation when needed. All research is preserved in docs/, no need to guess or re-search.