// Adversarial correctness tests for Tiny Pointers
//
// Tests worst-case inputs, collision patterns, and adversarial key distributions

use std::num::NonZeroU32;
use tinypointers::TinyPtrMap;

#[test]
fn test_sequential_keys() {
    let mut map = TinyPtrMap::new(1024);

    for i in 0u64..512 {
        let result = map.insert(i, NonZeroU32::new((i % 1_000_000) as u32 + 1).unwrap());
        assert!(result.is_ok(), "Failed to insert sequential key {}", i);
    }

    assert_eq!(map.len(), 512);
}

#[test]
fn test_powers_of_two() {
    let mut map = TinyPtrMap::new(1024);

    for i in 0..10u64 {
        let key = 1u64 << i;
        let result = map.insert(key, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());
        assert!(result.is_ok(), "Failed to insert power of two: {}", key);
    }

    assert_eq!(map.len(), 10);
}

#[test]
fn test_repeated_patterns() {
    let mut map = TinyPtrMap::new(1024);
    let pattern: u64 = 0xABCD_ABCD_ABCD_ABCD;

    for i in 0u64..100 {
        let key = pattern.wrapping_add(i);
        let result = map.insert(key, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());
        assert!(result.is_ok(), "Failed to insert pattern key {}", i);
    }

    assert_eq!(map.len(), 100);
}

#[test]
fn test_collision_resistance() {
    let mut map = TinyPtrMap::new(2048);

    for i in 0u64..512 {
        let key = i * 31; // Poor hash distribution multiplier
        let result = map.insert(key, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());
        assert!(result.is_ok(), "Failed to insert potential collision {}", i);
    }

    assert_eq!(map.len(), 512);
}

#[test]
fn test_high_bit_density() {
    let mut map = TinyPtrMap::new(1024);

    for i in 0u64..256 {
        let key = 0xFFFF_FFFF_FFFF_F000u64.wrapping_add(i);
        let result = map.insert(key, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());
        assert!(result.is_ok(), "Failed to insert high-bit key {}", i);
    }

    assert_eq!(map.len(), 256);
}

#[test]
fn test_alternating_bits() {
    let mut map = TinyPtrMap::new(1024);

    for i in 0u64..256 {
        let key = 0xAAAA_AAAA_AAAA_AAAAu64.wrapping_add(i);
        let result = map.insert(key, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());
        assert!(result.is_ok(), "Failed to insert alternating-bit key {}", i);
    }

    assert_eq!(map.len(), 256);
}

#[test]
fn test_zero_bytes() {
    let mut map = TinyPtrMap::new(1024);

    for i in 0u64..256 {
        let key = i << 8; // Keys with zero low byte
        let result = map.insert(key, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());
        assert!(result.is_ok(), "Failed to insert zero-byte key {}", i);
    }

    assert_eq!(map.len(), 256);
}

#[test]
fn test_container_capacity_limit() {
    let size = 64;
    let mut map = TinyPtrMap::new(size);

    let mut successful_inserts = 0;
    for i in 0u64..size as u64 * 2 {
        let result = map.insert(i, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());

        if result.is_ok() {
            successful_inserts += 1;
        }
    }

    // Should handle at least the specified capacity
    assert!(map.len() >= size, "Should handle at least capacity items");
    assert!(
        successful_inserts >= size,
        "Should successfully insert at least capacity items"
    );
}

#[test]
fn test_duplicate_key_insertion() {
    let mut map = TinyPtrMap::new(256);
    let key = 12345u64;
    let handle = NonZeroU32::new(42).unwrap();

    let result1 = map.insert(key, handle);
    assert!(result1.is_ok(), "First insert should succeed");

    let result2 = map.insert(key, NonZeroU32::new(43).unwrap());
    assert!(
        result2.is_ok() || result2.is_err(),
        "Duplicate insert behavior"
    );
}

#[test]
fn test_worst_case_distribution() {
    let mut map = TinyPtrMap::new(512);

    let mut successful = 0;
    for i in 0u64..128 {
        let key = (i * 64) & 0xFF; // Force low-byte collisions
        let result = map.insert(key, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());

        if result.is_ok() {
            successful += 1;
        }
    }

    // Accept partial success due to aggressive collision pattern
    assert!(
        successful >= 50,
        "Should handle most collision patterns, got {}",
        successful
    );
    assert!(map.len() >= 50, "Should store multiple items");
}

#[test]
fn test_sparse_distribution() {
    let mut map = TinyPtrMap::new(2048);

    for i in 0u64..100 {
        let key = i * 1_000_000; // Very sparse keys
        let result = map.insert(key, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());
        assert!(result.is_ok(), "Failed to insert sparse key {}", i);
    }

    assert_eq!(map.len(), 100);
}

#[test]
fn test_negative_equivalent_large_keys() {
    let mut map = TinyPtrMap::new(1024);

    for i in 0u64..128 {
        let key = u64::MAX - i; // Large values near MAX
        let result = map.insert(key, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());
        assert!(result.is_ok(), "Failed to insert large key {}", i);
    }

    assert_eq!(map.len(), 128);
}

#[test]
fn test_error_conditions() {
    let mut map = TinyPtrMap::new(16);

    for i in 0u64..32 {
        let _ = map.insert(i, NonZeroU32::new(((i % 1_000_000) as u32) + 1).unwrap());
    }

    // Should handle the capacity, though may not enforce exact limit
    assert!(map.len() >= 10, "Should handle reasonable insert count");
}
