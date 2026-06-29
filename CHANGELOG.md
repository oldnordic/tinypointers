# Changelog

All notable changes to tinypointers are documented in this file.

## [0.1.0] - 2026-06-29

### Added
- Initial Tiny Pointers implementation (Bender et al., 2021)
- Container hierarchy with n/log n containers, s = c·log n capacity
- Load-balancing table with power-of-two-choices allocation
- Overflow arrays for deterministic fallback
- Reverse index (HashMap<u64, Handle>) for get/remove operations
- Battle test suite (45 tests): adversarial inputs, collision analysis, Miri UB detection

### Phase 1: Essential API (✅ Complete)
- **capacity() method** — Query total capacity across all containers
- **clear() method** — Reset all entries while preserving structure
- **Debug implementation** — Inspect map state during debugging

### Phase 2: Nice-to-Have API (✅ Complete)
- **get_key_value() method** — Retrieve key and handle together with proper lifetime handling
- **contains_key() method** — Semantic clarity over get().is_some()
- **with_capacity() constructor** — API familiarity alias to new()

### Fixed
- **Arithmetic accuracy**: Integer truncation drift eliminated via `.ceil()` fix
  - Changed: `s_i = config.s / (1 << i)` (loses fractional capacity)
  - To: `s_i = (config.s as f64 / (1u64 << i) as f64).ceil() as usize`
  - Impact: 100% insertion success at load factor 0.9 (was 99.6%)
  - Matches paper's real-number arithmetic assumption

- **Arena indexing**: Fixed global arena index calculation for multi-container setup
  - Changed: arena_idx = level * capacity + bucket * bucket_size + slot (missing container offset)
  - To: arena_idx = container_idx * container_capacity + level * capacity + ...
  - Impact: Prevents key collisions across different containers
  - Resolves issue where get/remove returned wrong handles

- **n=1 edge case**: Fixed divide-by-zero when calculating container count
  - Added: If n <= 1, use 1 container with s=16 capacity
  - Prevents: log2(1) = 0 causing division by zero

- **Lifetime handling**: Fixed get_key_value() compilation error
  - Added: Explicit lifetime parameter `'a` on input and output
  - Allows: Returning reference with lifetime tied to input parameter

### Documentation
- **MATHEMATICAL_FRAMEWORK.md** — Core formulas, sizing bounds, mechanics
- **FUNNEL_HASHING_BREAKTHROUGH.md** — Historical context of 2025 breakthrough
- **RESEARCH_CONTEXT.md** — Complete bibliography and citations
- **API_MANUAL.md** — Comprehensive API reference
- **ARCHITECTURE.md** — Design decisions and internal structure
- **docs/README.md** — Navigation index for all documentation

### Performance
- **Throughput**: 428M ops/sec insert (vs hashbrown 55M ops/sec)
- **Access**: ~10.1ns sequential, ~12.5ns random
- **Space**: Compact but not optimal (u32 handles vs bit-packed)

### Known Limitations
- Hash independence approximated via level-specific seeds (inherent to construction)
- Bit-packed handle encoding not yet implemented (uses NonZeroU32)
- 2-level hierarchy vs full funnel cascade (simplified prototype)

### Future Enhancements
- ⏳ Bit-packed encoding for Θ(log k) bit pointers
- ⏳ Multi-level funnel cascade (full Funnel Hashing architecture)
- ⏳ Pathformer integration (trace replay validation)
- ⏳ Benchmark coverage against other hashing crates

