# Funnel Cascade Research

Research findings for implementing multi-level Funnel Hashing cascade in tinypointers.

## Current State Analysis

**Current Implementation (2-level):**
```rust
// Current: 2-level hierarchy
for level in 0..2 {
    try_insert(level, key, value);
}
```

**Theoretical Target (Funnel Hashing):**
- log n levels (not just 2)
- Dynamic bucket sizing: β = 2·log ε⁻¹
- Level-specific hash functions
- Cascading overflow with O(1) guarantees

**Current vs Target:**

| Aspect | Current | Target (Funnel) |
|--------|---------|------------------|
| Levels | 2 | log n |
| Bucket size | Fixed b=16 | Dynamic β = 2·log ε⁻¹ |
| Hash functions | Level-seeded | Independent per level |
| Overflow | Arrays | Cascading sub-tables |

## Research Findings

### 1. opthash Rust Implementation

**Source:** [GitHub: aaron-ang/opthash-rs](https://github.com/aaron-ang/opthash-rs)

**Capabilities:**
- Production FunnelHashMap implementation
- Based on Farach-Colton et al. 2025 paper
- Both Elastic Hashing and Funnel Hashing
- Full Rust implementation

**Architecture Analysis:**
```rust
// From opthash source (research summary)
pub struct FunnelHashMap<K, V> {
    // Primary table
    primary: Vec<Bucket>,
    // Overflow tables (cascading)
    overflows: Vec<Vec<Bucket>>,
    // Level-specific parameters
    config: FunnelConfig,
}

struct FunnelConfig {
    bucket_size: usize,     // β = 2·log ε⁻¹
    num_levels: usize,      // log n
    load_factor: f64,
}
```

**Key Insights:**
1. **Dynamic bucket sizing:** β depends on ε (empty fraction)
2. **Independent hashes:** Each level uses different hash function
3. **Cascade coordination:** Overflow flows through levels deterministically
4. **O(1) guarantees:** Even at 99%+ capacity

**Relevance to Tinypointers:**
- ✅ Reference implementation available
- ✅ Same target paper (Farach-Colton et al. 2025)
- ✅ Production-quality code to study
- ⚠️ Different design goals (general vs u64-specific)

**Verdict:** Excellent reference for implementation patterns.

### 2. Funnel Hashing Paper Details

**Source:** Our documentation: [FUNNEL_HASHING_BREAKTHROUGH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_HASHING_BREAKTHROUGH.md)

**Key Technical Details:**

**Bucket Sizing:**
```
β = 2·log ε⁻¹
```
Where:
- ε = empty fraction (e.g., 0.1 for 90% load)
- β = bucket capacity in primary table

**Example:**
- ε = 0.1 (90% load)
- β = 2·log(10) ≈ 6.6 → Round to 7 slots per bucket

**Block Splitting:**
```
b = δ⁻²·log ε⁻¹
```
Where:
- δ = √ε (scaling parameter)
- b = block size for overflow handling

**Cascade Structure:**
```
Primary Table (β buckets)
    ↓ (overflow cascade)
Level 1 Overflow (Θ(log log n) each)
    ↓ (overflow cascade)
Level 2 Overflow (larger buckets)
    ↓ (overflow cascade)
Level 3 Overflow (final resort)
```

### 3. Level-Specific Hash Functions

**Requirement:** Independent hash functions per level

**Current Approach (Level Seeds):**
```rust
fn hash_level(&self, key: &u64, level: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    level.hash(&mut hasher);  // Seed with level
    key.hash(&mut hasher);
    (hasher.finish() as usize) % capacity
}
```

**Funnel Approach (Independent Functions):**
```rust
struct LevelHasher {
    seed: u64,
    hash_fn: fn(u64, u64) -> u64,
}

impl LevelHasher {
    fn hash(&self, key: u64) -> usize {
        (self.hash_fn)(key, self.seed) as usize % self.capacity
    }
}
```

**Improvement:** Use truly independent hash functions (not just seeded)

## Implementation Strategy

### Proposed Design: cfg-Based Funnel Cascade

**Cargo.toml:**
```toml
[features]
default = []
funnel-cascade = []
full = ["funnel-cascade", "bit-packed"]
```

**Implementation (src/lib.rs):**
```rust
#[cfg(feature = "funnel-cascade")]
mod funnel {
    // Full Funnel Hashing cascade
    use super::*;

    pub struct FunnelConfig {
        pub bucket_size: usize,     // β = 2·log ε⁻¹
        pub num_levels: usize,      // log₂n
        pub epsilon: f64,           // Empty fraction
    }

    impl FunnelConfig {
        pub fn new(n: usize, epsilon: f64) -> Self {
            let bucket_size = (2.0 * epsilon.log2()).ceil() as usize;
            let num_levels = (n as f64).log2() as usize;
            Self {
                bucket_size,
                num_levels,
                epsilon,
            }
        }
    }

    // Multi-level insert with cascade
    pub fn insert_funnel(
        containers: &mut Vec<Container>,
        config: &FunnelConfig,
        key: u64,
        value: Handle
    ) -> Result<(), Error> {
        for level in 0..config.num_levels {
            let bucket_size = if level == 0 {
                config.bucket_size
            } else {
                // Progressive bucket sizing
                config.bucket_size * (1 << level)
            };

            if try_insert_level(containers, key, value, level, bucket_size)? {
                return Ok(());
            }
        }
        Err(Error::ContainerFull)
    }
}

#[cfg(not(feature = "funnel-cascade"))]
mod funnel {
    // Current 2-level approach
    pub fn insert_funnel(
        containers: &mut Vec<Container>,
        _config: &(),
        key: u64,
        value: Handle
    ) -> Result<(), Error> {
        for level in 0..2 {
            if try_insert_level(containers, key, value, level, 16)? {
                return Ok(());
            }
        }
        Err(Error::ContainerFull)
    }
}
```

### Performance Considerations

**Multi-Level Overhead:**
- Current: 2 iterations max
- Funnel: log n iterations (e.g., ~20 iterations for n=1M)

**Expected Benefit:**
- Better load balancing at high capacity (95%+)
- Reduced overflow probability
- O(1) operations even at extreme load factors

**Tradeoff:**
- More iterations per insert
- Better space utilization
- Deterministic fallback

### Testing Strategy

**Validation Tests:**
```rust
#[cfg(test)]
mod funnel_tests {
    #[test]
    fn test_high_load_factor() {
        // Test 95% load factor insertion success
        let n = 1000;
        let epsilon = 0.05;  // 95% load
        let config = FunnelConfig::new(n, epsilon);
        // Insert 950 items, verify success rate
    }

    #[test]
    fn test_o1_operations() {
        // Verify O(1) operations at high load
        let ops = 10000;
        let start = Instant::now();
        for _ in 0..ops {
            // Measure time complexity
        }
        // Verify linear in ops, not in n
    }

    #[test]
    fn test_cascade_overflow() {
        // Verify overflow flows through cascade correctly
        // Fill primary table, verify level 1 overflow works
    }
}
```

**Benchmark Comparison:**
```rust
#[cfg(feature = "funnel-cascade")]
#[bench]
fn bench_funnel_insert_95_load(b: &mut Criterion) {
    // Compare vs 2-level at 95% load factor
}

#[cfg(not(feature = "funnel-cascade"))]
#[bench]
fn bench_standard_insert_95_load(b: &mut Criterion) {
    // Current implementation
}
```

## Research Summary

**Best Approach:** Reference opthash implementation patterns

**Reasoning:**
1. Production-tested implementation available
2. Same theoretical foundation (Farach-Colton et al. 2025)
3. Clear mapping from paper to code
4. Can extract cascade logic patterns

**Implementation Priority:** Medium (higher complexity, uncertain payoff)

**Estimated Effort:** 2 weeks
- Research: 2 days (opthash source study)
- Implementation: 7-8 days
- Validation/Benchmarking: 3-4 days

**Success Criteria:**
- O(1) operations at 95%+ load factor
- Better space utilization than 2-level
- All tests pass with feature enabled

## Risk Assessment

**Complexity Risk:** High
- Multi-level coordination tricky
- Hash independence critical
- Cascade ordering matters

**Performance Risk:** Medium
- May not improve on current 2-level approach
- Overhead of multiple levels
- Unclear if benefit justifies complexity

**Recommendation:**
- Implement bit-packed encoding first (clearer payoff)
- Study opthash implementation thoroughly
- Prototype cascade before full commitment

## References

- [opthash implementation](https://github.com/aaron-ang/opthash-rs) — Production Funnel HashMap
- [opthash on Hacker News](https://news.ycombinator.com/item?id=48365720) — Community discussion
- [FUNNEL_HASHING_BREAKTHROUGH.md](https://github.com/oldnordic/tinypointers/blob/main/docs/FUNNEL_HASHING_BREAKTHROUGH.md) — Our detailed documentation
- [Farach-Colton et al. 2025 paper](https://github.com/oldnordic/tinypointers/blob/main/docs/RESEARCH_CONTEXT.md) — Full citation