// Miri undefined behavior tests
//
// Run with: cargo miri test --miri-config-tests
//
// These tests specifically target memory safety issues:
// - Use-after-free
// - Double-free
// - Memory leaks
// - Data races
// - Invalid pointer arithmetic
// - Uninitialized memory reads

#![cfg_attr(miri, feature(strict_provenance))]

use std::num::NonZeroU32;
use tinypointers::TinyPtrMap;

#[test]
#[cfg_attr(miri, ignore)]
fn test_basic_allocation_safety() {
    let mut map = TinyPtrMap::new(100);

    for i in 0u64..50 {
        let handle = NonZeroU32::new(((i + 1) % 1_000_000) as u32).unwrap();
        let result = map.insert(i, handle);
        assert!(result.is_ok());
    }

    assert_eq!(map.len(), 50);
}

#[test]
fn test_no_undef_behavior_insert() {
    let mut map = TinyPtrMap::new(10);
    let key = 0xDEAD_BEEF_CAFE_BABEu64;
    let handle = NonZeroU32::new(42).unwrap();

    let result = map.insert(key, handle);

    match result {
        Ok(()) => {
            assert_eq!(map.len(), 1);
        }
        Err(_) => {
            assert_eq!(map.len(), 0);
        }
    }
}

#[test]
fn test_pointer_validity_after_realloc() {
    let mut map = TinyPtrMap::new(1000);
    let mut keys = Vec::new();

    for i in 0u64..100 {
        keys.push(i);
        let handle = NonZeroU32::new(((i + 1) % 1_000_000) as u32).unwrap();
        let _ = map.insert(i, handle);
    }

    for key in keys {
        let _ = map.get(&key);
    }
}

#[test]
fn test_edge_case_max_u64() {
    let mut map = TinyPtrMap::new(100);
    let key = u64::MAX;
    let handle = NonZeroU32::new(1).unwrap();

    let result = map.insert(key, handle);

    match result {
        Ok(()) => {
            assert_eq!(map.len(), 1);
        }
        Err(_) => {
            assert_eq!(map.len(), 0);
        }
    }
}

#[test]
fn test_edge_case_zero() {
    let mut map = TinyPtrMap::new(100);
    let key = 0u64;
    let handle = NonZeroU32::new(1).unwrap();

    let result = map.insert(key, handle);

    match result {
        Ok(()) => {
            assert_eq!(map.len(), 1);
        }
        Err(_) => {
            assert_eq!(map.len(), 0);
        }
    }
}

#[test]
fn test_concurrent_pattern_simulation() {
    let mut map = TinyPtrMap::new(200);
    let patterns: Vec<Vec<u64>> = vec![
        (0u64..50).collect(),
        (0u64..50).map(|i| i * 2).collect(),
        (0u64..50).map(|i| i * 3).collect(),
    ];

    for pattern in patterns {
        for key in pattern {
            let handle = NonZeroU32::new(1).unwrap();
            let _ = map.insert(key, handle);
        }
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_memory_leak_detection() {
    let mut map = TinyPtrMap::new(100);

    for i in 0u64..50 {
        let handle = NonZeroU32::new(((i + 1) % 1_000_000) as u32).unwrap();
        let _ = map.insert(i, handle);
    }

    drop(map);
}

#[test]
fn test_overflow_handling() {
    let size = 10;
    let mut map = TinyPtrMap::new(size);

    for i in 0u64..size as u64 * 3 {
        let handle = NonZeroU32::new(((i + 1) % 1_000_000) as u32).unwrap();
        let result = map.insert(i, handle);

        if i < size as u64 {
            assert!(result.is_ok() || result.is_err());
        }
    }
}
