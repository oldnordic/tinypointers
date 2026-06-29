#![no_main]
use libfuzzer_sys::fuzz_target;
use std::num::NonZeroU32;
use tinypointers::TinyPtrMap;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let keys: Vec<u64> = data
        .chunks(8)
        .filter(|chunk| chunk.len() == 8)
        .map(|chunk| {
            let arr = [chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7]];
            u64::from_le_bytes(arr)
        })
        .collect();

    if keys.is_empty() {
        return;
    }

    let size = keys.len().max(10);
    let mut map = TinyPtrMap::new(size);

    for (i, &key) in keys.iter().enumerate() {
        let handle = NonZeroU32::new((i as u32) % 1_000_000 + 1);
        if let Some(handle) = handle {
            let _ = map.insert(key, handle);
        }
    }

    for &key in &keys {
        let _ = map.get(&key);
    }
});
