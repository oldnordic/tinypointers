# Tinypointers API Manual

Complete reference for all public types, methods, and usage patterns.

## Table of Contents

- [Types](#types)
- [Construction](#construction)
- [Core Operations](#core-operations)
- [Query Methods](#query-methods)
- [Capacity Management](#capacity-management)
- [Error Handling](#error-handling)
- [Usage Patterns](#usage-patterns)
- [Performance Considerations](#performance-considerations)

## Types

### TinyPtrMap<V>

Main hash map type for u64 keys with compact pointer storage.

**Type Parameters:**
- `V: Handle` — Value type (typically `NonZeroU32` or similar handle type)

**Constraints:**
- Keys: Fixed as `u64` (optimized for integer keys)
- Values: Any handle type (arena indices, file descriptors, etc.)

**Example:**
```rust
use tinypointers::TinyPtrMap;
use std::num::NonZeroU32;

let mut map: TinyPtrMap<u64> = TinyPtrMap::new(1000);
```

### Handle

Type alias for the handle type stored in the map.

**Definition:**
```rust
pub type Handle = NonZeroU32;
```

**Usage:**
```rust
let handle = NonZeroU32::new(42).unwrap();
map.insert(12345, handle)?;
```

### Error

Error type for insert operations.

**Variants:**
- `Error::ContainerFull` — Container overflow (should not occur with proper sizing)

## Construction

### new()

Create a new map sized for n elements.

**Signature:**
```rust
pub fn new(n: usize) -> Self
```

**Parameters:**
- `n: usize` — Expected number of elements

**Returns:**
- `TinyPtrMap<V>` — New map instance

**Example:**
```rust
let map: TinyPtrMap<u64> = TinyPtrMap::new(1000);
```

**Sizing:**
- Creates ~n/log n containers
- Each container: s = 16·log n items
- Total capacity: ~n elements

### with_capacity()

Constructor alias for API familiarity.

**Signature:**
```rust
pub fn with_capacity(n: usize) -> Self
```

**Behavior:** Identical to `new(n)` — delegates internally.

**Example:**
```rust
let map = TinyPtrMap::with_capacity(1000);
```

## Core Operations

### insert()

Insert a key → handle mapping.

**Signature:**
```rust
pub fn insert(&mut self, key: u64, value: NonZeroU32) -> Result<(), Error>
```

**Parameters:**
- `key: u64` — Integer key (must be unique)
- `value: NonZeroU32` — Handle to store

**Returns:**
- `Result<(), Error>` — Ok(()) or Err(Error::ContainerFull)

**Behavior:**
- Uses power-of-two-choices load balancing
- O(1) expected time
- O(log log n) worst-case (negligible probability)

**Example:**
```rust
let handle = NonZeroU32::new(42).unwrap();
map.insert(12345, handle)?;

// Handle duplicates (will overwrite existing)
map.insert(12345, handle)?;
```

**Error Handling:**
```rust
match map.insert(key, handle) {
    Ok(()) => println!("Inserted"),
    Err(Error::ContainerFull) => eprintln!("Container overflow"),
}
```

### get()

Lookup handle by key.

**Signature:**
```rust
pub fn get(&self, key: &u64) -> Option<Handle>
```

**Parameters:**
- `key: &u64` — Key to lookup

**Returns:**
- `Option<Handle>` — Some(handle) if found, None if missing

**Behavior:**
- O(1) expected time
- Uses reverse index for fast validation
- Returns copy of handle (cheap for NonZeroU32)

**Example:**
```rust
if let Some(handle) = map.get(&12345) {
    println!("Found: {}", handle);
} else {
    println!("Not found");
}
```

### get_key_value()

Get key and handle together.

**Signature:**
```rust
pub fn get_key_value<'a>(&self, key: &'a u64) -> Option<(&'a u64, Handle)>
```

**Parameters:**
- `key: &'a u64` — Key to lookup (with lifetime 'a)

**Returns:**
- `Option<(&'a u64, Handle)>` — Some((key_ref, handle)) or None

**Lifetime Note:**
- Returned key reference lives as long as input parameter
- Useful for iteration patterns where you need both key and value

**Example:**
```rust
if let Some((key, handle)) = map.get_key_value(&12345) {
    println!("{} -> {}", key, handle);
}
```

### contains_key()

Check if key exists.

**Signature:**
```rust
pub fn contains_key(&self, key: &u64) -> bool
```

**Parameters:**
- `key: &u64` — Key to check

**Returns:**
- `bool` — true if key exists, false otherwise

**Behavior:**
- Semantic clarity over `get().is_some()`
- O(1) expected time (delegates to HashMap::contains_key)

**Example:**
```rust
if map.contains_key(&12345) {
    println!("Key exists");
}
```

### remove()

Remove key → handle mapping.

**Signature:**
```rust
pub fn remove(&mut self, key: &u64) -> Option<Handle>
```

**Parameters:**
- `key: &u64` — Key to remove

**Returns:**
- `Option<Handle>` — Some(handle) if removed, None if not found

**Behavior:**
- O(1) expected time
- Frees container slot and reverse index entry
- Returns removed handle (if found)

**Example:**
```rust
if let Some(handle) = map.remove(&12345) {
    println!("Removed: {}", handle);
} else {
    println!("Key not found");
}
```

## Query Methods

### len()

Get current element count.

**Signature:**
```rust
pub fn len(&self) -> usize
```

**Returns:**
- `usize` — Number of elements in map

**Example:**
```rust
println!("Elements: {}", map.len());
```

### is_empty()

Check if map is empty.

**Signature:**
```rust
pub fn is_empty(&self) -> bool
```

**Returns:**
- `bool` — true if len() == 0, false otherwise

**Example:**
```rust
if map.is_empty() {
    println!("Map is empty");
}
```

### capacity()

Get total capacity across all containers.

**Signature:**
```rust
pub fn capacity(&self) -> usize
```

**Returns:**
- `usize` — Total slots across all containers

**Note:**
- Sum of all container capacities
- Not the same as "remaining capacity"
- Use for resource monitoring

**Example:**
```rust
let total_capacity = map.capacity();
println!("Total capacity: {}", total_capacity);
```

## Capacity Management

### clear()

Clear all entries, preserve structure.

**Signature:**
```rust
pub fn clear(&mut self)
```

**Behavior:**
- Removes all key → handle mappings
- Preserves container allocation
- Preserves reverse index allocation
- O(1) time (just resets counters)

**Use Case:**
- Reset map without deallocation
- Reuse map for new dataset

**Example:**
```rust
map.clear();
assert!(map.is_empty());
```

## Debug Implementation

### Debug Trait

TinyPtrMap implements Debug for inspection.

**Output Format:**
```
TinyPtrMap {
    containers: [Container, ...],
    total_capacity: N,
    item_count: M,
    arena_size: K,
    reverse_index_size: L
}
```

**Example:**
```rust
println!("{:?}", map);
```

## Error Handling

### Error Type

**Variants:**
```rust
pub enum Error {
    ContainerFull,
}
```

**Handling:**
```rust
match map.insert(key, handle) {
    Ok(()) => /* success */,
    Err(Error::ContainerFull) => /* handle overflow */,
}
```

**Note:** ContainerFull should not occur with proper sizing (use n ≥ expected elements).

## Usage Patterns

### Pattern 1: Token → Edge Cache

```rust
use tinypointers::TinyPtrMap;
use std::num::NonZeroU32;

let mut cache: TinyPtrMap<u64> = TinyPtrMap::new(100_000);

// Insert token_id → edge_index
let edge_index = NonZeroU32::new(42).unwrap();
cache.insert(token_id, edge_index)?;

// Lookup edge index for token
if let Some(index) = cache.get(&token_id) {
    // Use edge index...
}
```

### Pattern 2: LRU Access Pattern

```rust
// Good for LRU (temporal locality)
for (token_id, edge_index) in token_stream {
    cache.insert(token_id, edge_index)?;
}

// Lookups benefit from cache locality
if let Some(index) = cache.get(&recent_token) {
    // Likely in cache (recent access)
}
```

### Pattern 3: Existence Check

```rust
// Clearer than get().is_some()
if cache.contains_key(&token_id) {
    println!("Token in cache");
}
```

### Pattern 4: Key-Value Iteration

```rust
// Get both key and handle together
if let Some((token_id, edge_index)) = cache.get_key_value(&token) {
    println!("Token {} -> Edge {}", token_id, edge_index);
}
```

### Pattern 5: Resource Monitoring

```rust
let total_capacity = cache.capacity();
let used = cache.len();
let utilization = (used as f64 / total_capacity as f64) * 100.0;

println!("Utilization: {:.1}%", utilization);
```

## Performance Considerations

### Time Complexity

**All operations: O(1) expected**

| Operation | Expected | Worst-Case |
|-----------|----------|------------|
| insert | O(1) | O(log log n) |
| get | O(1) | O(log log n) |
| remove | O(1) | O(log log n) |
| contains_key | O(1) | O(log log n) |
| len | O(1) | O(1) |
| is_empty | O(1) | O(1) |
| clear | O(1) | O(1) |
| capacity | O(1) | O(1) |

**Note:** Worst-case occurs with negligible probability due to power-of-two-choices.

### Space Complexity

**Per element:**
- Fixed-size: Θ(log log log n + log k) bits
- Variable-size: Θ(log k) expected bits

**Example (n=1M, k=10):**
- Traditional pointer: 20 bits
- Tiny pointer (fixed): ~10 bits
- Tiny pointer (variable): ~4 bits expected

### Cache Performance

**Sequential access:** ~10.1ns (cache-friendly)
**Random access:** ~12.5ns (still excellent)

**Tips:**
- Use for workloads with high temporal locality
- Insert size = expected elements for best load balancing
- Pre-size for known vocabularies

### Sizing Guidelines

**Rule of thumb:** n = expected elements × 1.1

```rust
// For 100K expected elements
let map = TinyPtrMap::new(110_000);
```

**Under-sizing:** May cause container overflow (unlikely in practice)
**Over-sizing:** Wastes memory but no correctness issues

## Common Pitfalls

### 1. Forgetting to Handle Insert Errors

**Wrong:**
```rust
map.insert(key, handle);  // Ignoring Result
```

**Right:**
```rust
map.insert(key, handle)?;
```

### 2. Using Wrong Key Type

**Wrong:**
```rust
map.insert(&key, handle);  // &u64 instead of u64
```

**Right:**
```rust
map.insert(key, handle);
```

### 3. Assuming Order Preservation

**Wrong:**
```rust
// Elements not stored in insertion order
for i in 0..10 {
    map.insert(i, handle);
}
// Iteration order is undefined
```

**Right:**
```rust
// Use external ordering if needed
let mut keys: Vec<u64> = map.keys().collect();
keys.sort();
```

### 4. Ignoring Capacity

**Wrong:**
```rust
// No check if container will overflow
for i in 0..1_000_000 {
    map.insert(i, handle)?;
}
```

**Right:**
```rust
// Pre-size for expected capacity
let mut map = TinyPtrMap::new(1_100_000);
for i in 0..1_000_000 {
    map.insert(i, handle)?;
}
```

## Integration Examples

### With Arena Allocation

```rust
use tinypointers::TinyPtrMap;
use std::num::NonZeroU32;

struct Arena {
    data: Vec<String>,
}

impl Arena {
    fn push(&mut self, value: String) -> NonZeroU32 {
        let index = self.data.len();
        self.data.push(value);
        NonZeroU32::new(index as u32 + 1).unwrap()
    }

    fn get(&self, index: NonZeroU32) -> &str {
        &self.data[index.get() as usize - 1]
    }
}

let mut arena = Arena { data: Vec::new() };
let mut map: TinyPtrMap<u64> = TinyPtrMap::new(1000);

// Store string in arena, index in map
let index = arena.push("Hello".to_string());
map.insert(12345, index)?;

// Lookup string via map
if let Some(index) = map.get(&12345) {
    println!("{}", arena.get(index));
}
```

### With LRU Cache

```rust
use tinypointers::TinyPtrMap;

struct LRUCache {
    map: TinyPtrMap<u64>,
    capacity: usize,
}

impl LRUCache {
    fn new(capacity: usize) -> Self {
        Self {
            map: TinyPtrMap::new(capacity),
            capacity,
        }
    }

    fn get(&mut self, key: u64) -> Option<NonZeroU32> {
        self.map.get(&key)
    }

    fn put(&mut self, key: u64, value: NonZeroU32) -> Result<(), Error> {
        if self.map.len() >= self.capacity {
            // Evict oldest (simplified)
            self.map.clear();
        }
        self.map.insert(key, value)
    }
}
```

## Reference Implementation

See `src/lib.rs` for complete implementation details.

## See Also

- [README](https://github.com/oldnordic/tinypointers/blob/main/README.md) — Overview and quick start
- [ARCHITECTURE](https://github.com/oldnordic/tinypointers/blob/main/docs/ARCHITECTURE.md) — Design and internals
- [MATHEMATICAL_FRAMEWORK](https://github.com/oldnordic/tinypointers/blob/main/docs/MATHEMATICAL_FRAMEWORK.md) — Theoretical bounds