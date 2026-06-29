# Changelog

All notable changes to tinypointers are documented in this file.

## [0.1.0] - 2026-06-29

### Added
- Initial Tiny Pointers implementation (Bender et al., 2021)
- Container hierarchy with n/log n containers, s = c·log n capacity
- Load-balancing table with power-of-two-choices allocation
- Overflow arrays for deterministic fallback
- Reverse index (HashMap<u64, TinyPointer>) for get/remove operations
- Battle test suite (30 tests): adversarial inputs, collision analysis, Miri UB detection

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

### Known Limitations
- Hash independence approximated via level-specific seeds (inherent to construction)
- Bit-packed handle encoding not yet implemented (uses u16/u32)

### Incomplete Features (Blocking Production)
These features are NOT yet implemented. This section will be removed ONLY when all features are complete and tested.

**Missing Essential API:**
- ❌ capacity() method - cannot query max capacity
- ❌ clear() method - cannot reset without reallocating
- ❌ Debug impl - cannot inspect state during debugging

**Missing Nice-to-Have API:**
- ❌ get_key_value() - cannot retrieve key+handle together
- ❌ contains_key() - must use get().is_some() workaround
- ❌ with_capacity() - must use new() directly

**Implementation Status:**
- See `TODO.md` for complete task breakdown
- See `API_SPEC.md` for detailed requirements
- Phase 1 (Essential): 0/3 tasks complete
- Phase 2 (Nice-to-Have): 0/3 tasks complete
- Phase 3 (Validation): 0/3 tasks complete

**Quality Gates:**
- Zero `todo!()` or `unimplemented!()` macros allowed
- Zero stub functions or placeholder implementations
- Zero "TODO/FIXME/HACK/XXX" comments in code
- All features must be fully implemented or not at all

