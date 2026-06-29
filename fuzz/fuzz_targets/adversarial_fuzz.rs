#![no_main]
use libfuzzer_sys::fuzz_target;
use std::num::NonZeroU32;
use tinypointers::TinyPtrMap;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 {
        return;
    }

    let seed = u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]);

    let pattern_type = data[8];
    let iteration_count = (data[9] as usize).min(100);
    let base_key = u64::from_le_bytes([
        data[10], data[11], data[12], data[13], data[14], data[15], 0, 0,
    ]);

    let size = iteration_count.max(10) * 2;
    let mut map = TinyPtrMap::new(size);

    for i in 0..iteration_count {
        let key = match pattern_type % 5 {
            0 => base_key.wrapping_add(i as u64),
            1 => (base_key.wrapping_add(i as u64)) * 31,
            2 => (base_key.wrapping_add(i as u64)) << 8,
            3 => 0xFFFF_FFFF_FFFF_F000u64.wrapping_add(base_key).wrapping_add(i as u64),
            4 => base_key ^ (i as u64),
            _ => base_key.wrapping_add(i as u64),
        };

        let handle = NonZeroU32::new((i as u32) % 1_000_000 + 1);
        if let Some(h) = handle {
            let _ = map.insert(key, h);
        }
    }

    let _ = map.len();
});
