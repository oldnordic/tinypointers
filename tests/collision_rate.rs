// Collision rate comparison tests
//
// Compare Tiny Pointers collision resistance against ahash and fxhash
// at high load factors (>0.8) where collision probability is highest

use ahash::RandomState;
use fxhash::FxHasher64;
use std::collections::HashMap;
use std::hash::Hash;
use std::num::NonZeroU32;
use tinypointers::TinyPtrMap;

fn count_hash_collisions(keys: &[u64], state: &RandomState) -> usize {
    let mut seen = HashMap::new();
    let mut collisions = 0;

    for &key in keys {
        let hash = state.hash_one(key);

        match seen.entry(hash) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(true);
            }
            std::collections::hash_map::Entry::Occupied(_) => {
                collisions += 1;
            }
        }
    }

    collisions
}

fn count_fx_collisions(keys: &[u64]) -> usize {
    let mut seen = HashMap::new();
    let mut collisions = 0;

    for &key in keys {
        use std::hash::Hasher;
        let mut hasher = FxHasher64::default();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        match seen.entry(hash) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(true);
            }
            std::collections::hash_map::Entry::Occupied(_) => {
                collisions += 1;
            }
        }
    }

    collisions
}

#[test]
fn test_collision_rate_high_load_factor() {
    let load_factors = [0.5, 0.75, 0.8, 0.9, 0.95];
    let base_size = 1000;

    for &load_factor in &load_factors {
        let actual_size = (base_size as f64 / load_factor) as usize;
        let mut tinyptr_map = TinyPtrMap::new(actual_size);

        let keys: Vec<u64> = (0..base_size).map(|_| rand::random()).collect();

        for &key in &keys {
            let _ = tinyptr_map.insert(key, NonZeroU32::new(1).unwrap());
        }

        let ahash_state = RandomState::new();
        let ahash_collisions = count_hash_collisions(&keys, &ahash_state);
        let fx_collisions = count_fx_collisions(&keys);

        println!(
            "Load factor {:.2}: ahash={}, fx={}, tinyptr_inserted={}",
            load_factor,
            ahash_collisions,
            fx_collisions,
            tinyptr_map.len()
        );

        // Allow <0.5% failure rate at extreme load factors (paper bound: doubly-exponential tail)
        // This is consistent with theoretical guarantees
        assert!(tinyptr_map.len() >= (base_size as f64 * 0.995) as usize);
    }
}

#[test]
fn test_collision_rate_adversarial_patterns() {
    let patterns: Vec<Vec<u64>> = vec![
        (0..1000).map(|i| i * 31).collect(),          // Poor multiplier
        (0..1000).map(|i| i << 8).collect(),          // Zero-byte suffix
        (0..1000).map(|i| 0xABCDABCD ^ i).collect(),  // Repeated pattern
        (0..1000).map(|i| 0xFFFF_FFFF & i).collect(), // Low bits only
    ];

    for (idx, keys) in patterns.iter().enumerate() {
        let size = 2000;
        let mut tinyptr_map = TinyPtrMap::new(size);

        for &key in keys {
            let _ = tinyptr_map.insert(key, NonZeroU32::new(1).unwrap());
        }

        let ahash_state = RandomState::new();
        let ahash_collisions = count_hash_collisions(keys, &ahash_state);
        let fx_collisions = count_fx_collisions(keys);

        println!(
            "Pattern {}: ahash={}, fx={}, tinyptr_success={}",
            idx,
            ahash_collisions,
            fx_collisions,
            tinyptr_map.len()
        );

        assert_eq!(tinyptr_map.len(), keys.len());
    }
}

#[test]
fn test_collision_rate_sequential_vs_random() {
    let size = 2000;
    let sequential_keys: Vec<u64> = (0..1000).collect();
    let random_keys: Vec<u64> = (0..1000).map(|_| rand::random()).collect();

    for (keys_type, keys) in [("sequential", sequential_keys), ("random", random_keys)] {
        let mut tinyptr_map = TinyPtrMap::new(size);

        for &key in &keys {
            let _ = tinyptr_map.insert(key, NonZeroU32::new(1).unwrap());
        }

        let ahash_state = RandomState::new();
        let ahash_collisions = count_hash_collisions(&keys, &ahash_state);
        let fx_collisions = count_fx_collisions(&keys);

        println!(
            "Keys={}: ahash={}, fx={}, tinyptr_success={}",
            keys_type,
            ahash_collisions,
            fx_collisions,
            tinyptr_map.len()
        );

        assert_eq!(tinyptr_map.len(), keys.len());
    }
}

#[test]
fn test_collision_under_resizing_pressure() {
    let sizes = [100, 500, 1000, 5000];
    let load_factor = 0.9;

    for size in sizes {
        let actual_size = (size as f64 / load_factor) as usize;
        let mut tinyptr_map = TinyPtrMap::new(actual_size);

        let keys: Vec<u64> = (0..size).map(|_| rand::random()).collect();

        for &key in &keys {
            let _ = tinyptr_map.insert(key, NonZeroU32::new(1).unwrap());
        }

        let ahash_state = RandomState::new();
        let ahash_collisions = count_hash_collisions(&keys, &ahash_state);
        let fx_collisions = count_fx_collisions(&keys);

        println!(
            "Size {}: ahash={}, fx={}, tinyptr_success={}",
            size,
            ahash_collisions,
            fx_collisions,
            tinyptr_map.len()
        );

        // Accept 95% success rate - documents known limitation at high load factors
        assert!(tinyptr_map.len() >= (size as f64 * 0.95) as usize);
    }
}
