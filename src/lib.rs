// Tiny Pointers — o(log n) bit pointer compression
// Based on Bender et al., 2021: https://arxiv.org/abs/2111.12800

//! Compact pointer compression for u64 integer keys with LRU access patterns
//!
//! This crate implements tiny pointers, achieving o(log n) bit pointer storage
//! for the target workload of pathformer token_id → edge cache lookups.

#![warn(missing_docs)]
#![warn(clippy::all)]

use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::num::NonZeroU32;

/// Compact handle for dense arena storage
pub type Handle = NonZeroU32;

/// Tiny pointer encoding (prototype: use u16/u32, optimize later)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TinyPointer {
    /// Allocated in load-balancing table at (level, slot)
    LoadBalancer {
        /// Level index in container hierarchy
        level: u8,
        /// Slot index within bucket
        slot: u16,
    },
    /// Allocated in overflow array at (level_from_back, slot)
    Overflow {
        /// Level index counted from back of hierarchy
        level_from_back: u8,
        /// Slot index within overflow array
        slot: u32,
    },
}

/// Errors during tiny pointer operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Container at capacity
    ContainerFull,
    /// Allocation failed after probing all levels
    AllocationFailed,
    /// Invalid tiny pointer for key
    InvalidPointer,
}

/// Load-balancing table bucket (prototype: fixed-size array)
#[derive(Debug)]
struct Bucket {
    slots: Vec<Option<u64>>, // Store keys directly for prototype
}

impl Bucket {
    fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity],
        }
    }

    fn try_allocate(&mut self, key: u64) -> Option<usize> {
        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(key);
                return Some(idx);
            }
        }
        None
    }
}

/// Level in container hierarchy
#[derive(Debug)]
struct Level {
    buckets: Vec<Bucket>,
    hash_seed: u64,
    count: usize, // L_i: items at level >= i
}

impl Level {
    fn new(num_buckets: usize, bucket_capacity: usize, hash_seed: u64) -> Self {
        Self {
            buckets: (0..num_buckets)
                .map(|_| Bucket::new(bucket_capacity))
                .collect(),
            hash_seed,
            count: 0,
        }
    }

    fn hash_key(&self, key: u64) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.hash_seed.hash(&mut hasher);
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.buckets.len()
    }

    fn try_allocate(&mut self, key: u64) -> Result<usize, Error> {
        let bucket_idx = self.hash_key(key);
        self.buckets[bucket_idx]
            .try_allocate(key)
            .ok_or(Error::AllocationFailed)
    }
}

/// Overflow array for determinism
#[derive(Debug)]
struct OverflowArray {
    slots: Vec<Option<(u64, usize)>>, // (key, slot_index)
}

impl OverflowArray {
    fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity],
        }
    }

    fn allocate(&mut self, key: u64) -> Result<usize, Error> {
        for (idx, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some((key, idx));
                return Ok(idx);
            }
        }
        Err(Error::ContainerFull)
    }
}

/// Container in partitioned hierarchy
#[derive(Debug)]
struct Container {
    levels: Vec<Level>,
    overflow_arrays: Vec<OverflowArray>,
    item_count: usize,
    capacity: usize,
}

impl Container {
    fn new(config: &ContainerConfig) -> Self {
        let num_levels = (config.s as f64).log2().ceil() as usize;

        let mut levels = Vec::with_capacity(num_levels);
        let mut overflow_arrays = Vec::with_capacity(num_levels);

        for i in 0..num_levels {
            let s_i = (config.s as f64 / (1u64 << i) as f64).ceil() as usize;
            let num_buckets = s_i / config.b;

            levels.push(Level::new(
                num_buckets.max(1),
                config.b,
                config.base_hash_seed + i as u64,
            ));

            overflow_arrays.push(OverflowArray::new(s_i));
        }

        Self {
            levels,
            overflow_arrays,
            item_count: 0,
            capacity: config.s,
        }
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn item_count(&self) -> usize {
        self.item_count
    }

    fn try_allocate(&mut self, key: u64) -> Result<TinyPointer, Error> {
        if self.item_count >= self.capacity() {
            return Err(Error::ContainerFull);
        }

        for level_idx in 0..self.levels.len() {
            // Calculate next level capacity before borrowing mutably
            let next_level_capacity = if level_idx + 1 < self.levels.len() {
                self.levels[level_idx + 1].buckets.len()
                    * self.levels[level_idx + 1].buckets[0].slots.len()
            } else {
                0
            };

            let level = &mut self.levels[level_idx];

            match level.try_allocate(key) {
                Ok(slot) => {
                    self.item_count += 1;
                    // Increment counts for all levels <= level_idx
                    for j in 0..=level_idx {
                        self.levels[j].count += 1;
                    }
                    return Ok(TinyPointer::LoadBalancer {
                        level: level_idx as u8,
                        slot: slot as u16,
                    });
                }
                Err(Error::AllocationFailed) => {
                    if level.count >= next_level_capacity {
                        // Use overflow array
                        let overflow = &mut self.overflow_arrays[level_idx];
                        match overflow.allocate(key) {
                            Ok(slot) => {
                                self.item_count += 1;
                                // Increment counts for all levels <= level_idx
                                for j in 0..=level_idx {
                                    self.levels[j].count += 1;
                                }
                                let level_from_back = (self.levels.len() - 1 - level_idx) as u8;
                                return Ok(TinyPointer::Overflow {
                                    level_from_back,
                                    slot: slot as u32,
                                });
                            }
                            Err(_) => {
                                // Continue to next level
                                continue;
                            }
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Err(Error::AllocationFailed)
    }

    fn dereference(&self, key: u64, ptr: &TinyPointer) -> Option<usize> {
        match ptr {
            TinyPointer::LoadBalancer {
                level: level_num,
                slot,
            } => {
                let level = &self.levels[*level_num as usize];
                let bucket_idx = level.hash_key(key);
                let bucket = &level.buckets[bucket_idx];
                if bucket
                    .slots
                    .get(*slot as usize)?
                    .map(|k| k == key)
                    .unwrap_or(false)
                {
                    // Return global index: level * max_capacity + bucket_idx * bucket_size + slot
                    let bucket_size = bucket.slots.len();
                    let max_capacity = self.capacity;
                    Some(
                        (*level_num as usize) * max_capacity
                            + bucket_idx * bucket_size
                            + *slot as usize,
                    )
                } else {
                    None
                }
            }
            TinyPointer::Overflow {
                level_from_back,
                slot,
            } => {
                let level_idx = self.levels.len() - 1 - *level_from_back as usize;
                let overflow = &self.overflow_arrays[level_idx];
                overflow.slots.get(*slot as usize)?.and_then(|(k, _idx)| {
                    if k == key {
                        // Return global index: level * max_capacity + max_level_capacity + slot
                        let max_capacity = self.capacity;
                        Some(level_idx * max_capacity + max_capacity + *slot as usize)
                    } else {
                        None
                    }
                })
            }
        }
    }

    fn free_level_slot(&mut self, key: u64, level: u8, slot: u16) {
        if let Some(level_obj) = self.levels.get_mut(level as usize) {
            let bucket_idx = level_obj.hash_key(key);
            if let Some(bucket) = level_obj.buckets.get_mut(bucket_idx) {
                if let Some(slot_ref) = bucket.slots.get_mut(slot as usize) {
                    *slot_ref = None;
                }
            }
        }
    }

    fn free_overflow_slot(&mut self, key: u64, level_idx: usize, slot: u32) {
        if let Some(overflow) = self.overflow_arrays.get_mut(level_idx) {
            if let Some(slot_ref) = overflow.slots.get_mut(slot as usize) {
                if slot_ref.and_then(|(k, _)| Some(k == key)).unwrap_or(false) {
                    *slot_ref = None;
                }
            }
        }
    }

    fn free(&mut self, key: u64, ptr: &TinyPointer) {
        let stored_level = match ptr {
            TinyPointer::LoadBalancer { level, slot } => {
                self.free_level_slot(key, *level, *slot);
                *level as usize
            }
            TinyPointer::Overflow {
                level_from_back,
                slot,
            } => {
                let level_idx = self.levels.len() - 1 - *level_from_back as usize;
                self.free_overflow_slot(key, level_idx, *slot);
                level_idx
            }
        };

        // Decrement count for all levels <= stored_level
        for j in 0..=stored_level {
            if let Some(level_obj) = self.levels.get_mut(j) {
                level_obj.count = level_obj.count.saturating_sub(1);
            }
        }

        self.item_count = self.item_count.saturating_sub(1);
    }

    fn clear(&mut self) {
        // Reset item count
        self.item_count = 0;

        // Clear all bucket slots in all levels
        for level in &mut self.levels {
            level.count = 0;
            for bucket in &mut level.buckets {
                for slot in &mut bucket.slots {
                    *slot = None;
                }
            }
        }

        // Clear all overflow slots
        for overflow in &mut self.overflow_arrays {
            for slot in &mut overflow.slots {
                *slot = None;
            }
        }
    }
}

/// Container configuration
#[derive(Debug, Clone)]
struct ContainerConfig {
    s: usize, // Capacity per container: c * log n
    b: usize, // Bucket size
    base_hash_seed: u64,
}

/// Tiny pointer map: u64 keys → compact handles
pub struct TinyPtrMap<K = u64> {
    containers: Vec<Container>,
    arena: Vec<Handle>,
    key_to_ptr: HashMap<u64, TinyPointer>,
    _phantom: PhantomData<K>,
}

impl<K> fmt::Debug for TinyPtrMap<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let item_count: usize = self.containers.iter().map(|c| c.item_count()).sum();
        f.debug_struct("TinyPtrMap")
            .field("containers", &self.containers.len())
            .field(
                "total_capacity",
                &self.containers.iter().map(|c| c.capacity()).sum::<usize>(),
            )
            .field("item_count", &item_count)
            .field("arena_size", &self.arena.len())
            .field("reverse_index_size", &self.key_to_ptr.len())
            .finish()
    }
}

impl TinyPtrMap<u64> {
    /// Create new tiny pointer map
    pub fn new(n: usize) -> Self {
        let num_containers = if n <= 1 {
            1
        } else {
            n / (n as f64).log2().ceil() as usize
        };
        let s = if n <= 1 {
            16
        } else {
            2 * (n as f64).log2().ceil() as usize
        }; // c = 2 for prototype

        let config = ContainerConfig {
            s,
            b: 16, // Fixed bucket size for prototype
            base_hash_seed: 0x5728_9a3c,
        };

        let containers = (0..num_containers)
            .map(|_| Container::new(&config))
            .collect();

        Self {
            containers,
            arena: Vec::new(),
            key_to_ptr: HashMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Hash key to container index
    fn hash_to_container(&self, key: &u64) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.containers.len()
    }

    /// Insert key → handle mapping
    pub fn insert(&mut self, key: u64, value: Handle) -> Result<(), Error> {
        let container_idx = self.hash_to_container(&key);
        let container = &mut self.containers[container_idx];

        let ptr = container.try_allocate(key)?;

        let local_arena_idx = if let Some(idx) = container.dereference(key, &ptr) {
            idx
        } else {
            return Err(Error::InvalidPointer);
        };

        let container_capacity = container.capacity();
        let arena_idx = container_idx * container_capacity + local_arena_idx;

        if arena_idx >= self.arena.len() {
            self.arena.resize(arena_idx + 1, Handle::MIN);
        }

        self.arena[arena_idx] = value;
        self.key_to_ptr.insert(key, ptr);
        Ok(())
    }

    /// Lookup key → handle
    pub fn get(&self, key: &u64) -> Option<Handle> {
        let ptr = self.key_to_ptr.get(key)?;
        let container_idx = self.hash_to_container(key);
        let container = &self.containers[container_idx];

        let local_arena_idx = container.dereference(*key, ptr)?;
        let container_capacity = container.capacity();
        let arena_idx = container_idx * container_capacity + local_arena_idx;
        self.arena.get(arena_idx).copied()
    }

    /// Remove key → handle mapping
    pub fn remove(&mut self, key: &u64) -> Option<Handle> {
        let ptr = self.key_to_ptr.remove(key)?;
        let container_idx = self.hash_to_container(key);
        let container = &mut self.containers[container_idx];

        let local_arena_idx = container.dereference(*key, &ptr)?;
        let container_capacity = container.capacity();
        let arena_idx = container_idx * container_capacity + local_arena_idx;

        container.free(*key, &ptr);
        self.arena.get(arena_idx).copied()
    }

    /// Number of items stored
    pub fn len(&self) -> usize {
        self.containers.iter().map(|c| c.item_count()).sum()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns total capacity across all containers
    ///
    /// Formula: num_containers * container_capacity
    /// Includes overflow array slots in calculation
    pub fn capacity(&self) -> usize {
        self.containers.iter().map(|c| c.capacity()).sum()
    }

    /// Clears all entries while preserving allocated structure
    ///
    /// Resets all item counts to 0
    /// Clears all bucket slots to None
    /// Clears all overflow slots to None
    /// Clears reverse index lookup
    /// Resets arena to default values
    pub fn clear(&mut self) {
        // Clear each container
        for container in &mut self.containers {
            container.clear();
        }
        // Clear reverse index
        self.key_to_ptr.clear();
        // Reset arena to default values
        for handle in &mut self.arena {
            *handle = Handle::MIN;
        }
    }

    /// Returns key and handle if key exists
    ///
    /// Useful for iteration patterns where both key and value needed
    pub fn get_key_value<'a>(&self, key: &'a u64) -> Option<(&'a u64, Handle)> {
        let handle = self.get(key)?;
        Some((key, handle))
    }

    /// Returns true if key exists in map
    ///
    /// Semantic clarity over get().is_some()
    pub fn contains_key(&self, key: &u64) -> bool {
        self.key_to_ptr.contains_key(key)
    }

    /// Creates map with specified capacity
    ///
    /// Alias for new() to provide API familiarity with HashMap
    pub fn with_capacity(n: usize) -> Self {
        Self::new(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_basic() {
        let config = ContainerConfig {
            s: 64,
            b: 4,
            base_hash_seed: 0,
        };

        let mut container = Container::new(&config);

        let key1 = 12345;
        let key2 = 67890;

        let ptr1 = container.try_allocate(key1).unwrap();
        let ptr2 = container.try_allocate(key2).unwrap();

        assert_eq!(container.item_count(), 2);

        let idx1 = container.dereference(key1, &ptr1).unwrap();
        let idx2 = container.dereference(key2, &ptr2).unwrap();

        assert_ne!(idx1, idx2);
    }

    #[test]
    fn test_map_basic() {
        let mut map = TinyPtrMap::new(1024);

        let handle = Handle::MIN;
        assert!(map.insert(12345, handle).is_ok());
        assert_eq!(map.len(), 1);

        let retrieved = map.get(&12345);
        assert_eq!(retrieved, Some(handle));
    }

    #[test]
    fn test_get_remove() {
        let mut map = TinyPtrMap::new(1024);

        let handle1 = Handle::MIN;
        let handle2 = NonZeroU32::new(42).unwrap();

        assert!(map.insert(111, handle1).is_ok());
        assert!(map.insert(222, handle2).is_ok());

        assert_eq!(map.get(&111), Some(handle1));
        assert_eq!(map.get(&222), Some(handle2));
        assert_eq!(map.get(&333), None);

        assert_eq!(map.remove(&111), Some(handle1));
        assert_eq!(map.get(&111), None);
        assert_eq!(map.get(&222), Some(handle2));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_capacity() {
        let map = TinyPtrMap::new(1024);
        // Test capacity exists and is non-zero
        assert!(map.capacity() > 0);
        // Test capacity is at least as much as requested
        assert!(map.capacity() >= 1024);
    }

    #[test]
    fn test_capacity_after_insert() {
        let mut map = TinyPtrMap::new(1024);
        let initial_capacity = map.capacity();
        let handle = Handle::MIN;

        // Insert items
        for i in 0..100 {
            assert!(map.insert(i, handle).is_ok());
        }

        // Capacity should remain constant after insert
        assert_eq!(map.capacity(), initial_capacity);
    }

    #[test]
    fn test_capacity_after_remove() {
        let mut map = TinyPtrMap::new(1024);
        let handle = Handle::MIN;

        // Insert items
        for i in 0..100 {
            assert!(map.insert(i, handle).is_ok());
        }

        let initial_capacity = map.capacity();

        // Remove items
        for i in 0..50 {
            map.remove(&i);
        }

        // Capacity should remain constant after remove
        assert_eq!(map.capacity(), initial_capacity);
    }

    #[test]
    fn test_capacity_small_map() {
        let map = TinyPtrMap::new(10);
        // Test with small n
        assert!(map.capacity() > 0);
        assert!(map.capacity() >= 10);
    }

    #[test]
    fn test_capacity_single_item_map() {
        let map = TinyPtrMap::new(1);
        // Test with n=1 edge case
        assert!(map.capacity() >= 1);
    }

    #[test]
    fn test_clear() {
        let mut map = TinyPtrMap::new(1024);
        let handle = Handle::MIN;

        // Insert items
        for i in 0..100 {
            assert!(map.insert(i, handle).is_ok());
        }

        assert_eq!(map.len(), 100);

        // Clear map
        map.clear();

        // Verify empty but same capacity
        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
        assert!(map.capacity() > 0);
    }

    #[test]
    fn test_clear_capacity_unchanged() {
        let mut map = TinyPtrMap::new(1024);
        let handle = Handle::MIN;

        // Insert items
        for i in 0..50 {
            assert!(map.insert(i, handle).is_ok());
        }

        let capacity_before = map.capacity();
        map.clear();
        let capacity_after = map.capacity();

        // Capacity should remain unchanged
        assert_eq!(capacity_before, capacity_after);
    }

    #[test]
    fn test_clear_then_insert() {
        let mut map = TinyPtrMap::new(1024);
        let handle1 = Handle::MIN;
        let handle2 = NonZeroU32::new(42).unwrap();

        // Insert, clear, then insert again
        assert!(map.insert(111, handle1).is_ok());
        map.clear();
        assert!(map.insert(222, handle2).is_ok());

        // Verify new insert works after clear
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&222), Some(handle2));
        assert_eq!(map.get(&111), None);
    }

    #[test]
    fn test_clear_on_empty_map() {
        let mut map = TinyPtrMap::new(1024);

        // Clear on empty map should be safe
        map.clear();
        map.clear();

        assert_eq!(map.len(), 0);
        assert!(map.is_empty());
    }

    #[test]
    fn test_clear_then_get_remove() {
        let mut map = TinyPtrMap::new(1024);
        let handle = Handle::MIN;

        // Insert items
        for i in 0..50 {
            assert!(map.insert(i, handle).is_ok());
        }

        map.clear();

        // get() should return None after clear
        for i in 0..50 {
            assert_eq!(map.get(&i), None);
        }

        // remove() should return None after clear
        for i in 0..50 {
            assert_eq!(map.remove(&i), None);
        }
    }

    #[test]
    fn test_debug_impl() {
        let mut map = TinyPtrMap::new(1024);

        // Test Debug output compiles
        let debug_str = format!("{:?}", map);

        // Verify output contains expected fields
        assert!(debug_str.contains("TinyPtrMap"));
        assert!(debug_str.contains("containers"));

        // Verify Debug shows expected information
        let handle = Handle::MIN;
        for i in 0..10 {
            assert!(map.insert(i, handle).is_ok());
        }

        let debug_str_with_data = format!("{:?}", map);
        assert!(debug_str_with_data.contains("item_count"));
        assert!(debug_str_with_data.contains("arena_size"));
    }

    #[test]
    fn test_debug_output_readable() {
        let mut map = TinyPtrMap::new(100);
        let handle = Handle::MIN;

        for i in 0..20 {
            assert!(map.insert(i, handle).is_ok());
        }

        // Debug output should be human-readable
        let debug_str = format!("{:?}", map);
        assert!(!debug_str.is_empty());
        assert!(!debug_str.contains("\n\n\n")); // Not excessive newlines
    }

    #[test]
    fn test_get_key_value() {
        let mut map = TinyPtrMap::new(1024);
        let handle1 = Handle::MIN;
        let handle2 = NonZeroU32::new(42).unwrap();

        // Test returns Some when key exists
        assert!(map.insert(111, handle1).is_ok());
        let result = map.get_key_value(&111);
        assert!(result.is_some());
        let (key_ref, handle) = result.unwrap();
        assert_eq!(*key_ref, 111);
        assert_eq!(handle, handle1);

        // Test returns None when key missing
        let result = map.get_key_value(&222);
        assert!(result.is_none());

        // Test with multiple keys
        assert!(map.insert(222, handle2).is_ok());
        let result = map.get_key_value(&222);
        assert!(result.is_some());
        let (key_ref, handle) = result.unwrap();
        assert_eq!(*key_ref, 222);
        assert_eq!(handle, handle2);
    }

    #[test]
    fn test_contains_key() {
        let mut map = TinyPtrMap::new(1024);
        let handle = Handle::MIN;

        // Test returns true for existing keys
        assert!(map.insert(111, handle).is_ok());
        assert!(map.contains_key(&111));
        assert!(!map.contains_key(&222));

        // Test returns false for missing keys
        assert!(!map.contains_key(&333));

        // Test behavior matches get().is_some()
        assert_eq!(map.contains_key(&111), map.get(&111).is_some());
        assert_eq!(map.contains_key(&222), map.get(&222).is_some());
    }

    #[test]
    fn test_contains_key_after_remove() {
        let mut map = TinyPtrMap::new(1024);
        let handle = Handle::MIN;

        assert!(map.insert(111, handle).is_ok());
        assert!(map.contains_key(&111));

        map.remove(&111);
        assert!(!map.contains_key(&111));
    }

    #[test]
    fn test_with_capacity() {
        // Test with_capacity(n) == new(n)
        let map1 = TinyPtrMap::with_capacity(1024);
        let map2 = TinyPtrMap::new(1024);

        // Both should have same capacity
        assert_eq!(map1.capacity(), map2.capacity());

        // Both should have same container count
        assert_eq!(map1.containers.len(), map2.containers.len());
    }

    #[test]
    fn test_with_capacity_functionality() {
        let mut map = TinyPtrMap::with_capacity(100);
        let handle = Handle::MIN;

        // Should work exactly like new()
        assert!(map.insert(42, handle).is_ok());
        assert!(map.contains_key(&42));
        assert_eq!(map.len(), 1);

        // Capacity should match parameter
        assert!(map.capacity() >= 100);
    }

    #[test]
    fn test_container_level_count_leak() {
        let config = ContainerConfig {
            s: 64,
            b: 4,
            base_hash_seed: 0,
        };
        let mut container = Container::new(&config);
        let mut keys_and_ptrs = Vec::new();

        // Insert items until we allocate at level >= 1
        let mut allocated_at_level_1_or_more = false;
        for i in 0..64 {
            let key = i as u64;
            if let Ok(ptr) = container.try_allocate(key) {
                match ptr {
                    TinyPointer::LoadBalancer { level, .. } if level >= 1 => {
                        allocated_at_level_1_or_more = true;
                    }
                    TinyPointer::Overflow { .. } => {
                        allocated_at_level_1_or_more = true;
                    }
                    _ => {}
                }
                keys_and_ptrs.push((key, ptr));
            }
        }

        assert!(allocated_at_level_1_or_more, "Should have allocated at level >= 1");

        // Now free all items
        for (key, ptr) in keys_and_ptrs {
            container.free(key, &ptr);
        }

        // Verify all level counts are 0
        for (i, level) in container.levels.iter().enumerate() {
            assert_eq!(level.count, 0, "Level {} count is leaked: {}", i, level.count);
        }
    }
}
