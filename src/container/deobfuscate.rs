// Based on the "mmdecrypt.c" from SAU: Sprite and Archive Utility
// project:
//
// https://github.com/saniv/sau/blob/master/unsorted/scraps/mmdecrypt.c
//
// Magic & Mayhem decryption and unpacking routines
// Author: Nikita Sadkov
// License: GPL2

use std::convert::TryInto;

/// For prng_state return new prng_state and next table entry. Used
/// only for initializing PRNG table.
fn prng_state_iterate(prng_state: u32) -> (u32, u32) {
    let mut t = 0x41c64e6du64.wrapping_mul(prng_state as u64) as i64;

    let mut t_hi: u32 = (t >> 32) as u32;
    t_hi = t_hi.wrapping_shl(16);

    let t_lo: u32 = t as u32;

    t = (((t_hi as u64) << 32) | (t_lo as u64)) as i64;
    t = (t as u64).wrapping_add(0xffff00003039) as i64 as i64;

    let new_prng_state = t as u32;
    let table_entry: u32 = (((t >> 32) as u32) & 0xffff_0000u32) | ((t as u32) >> 16);

    assert_eq!(
        (new_prng_state & 0xffff_0000) >> 16,
        table_entry & 0xffff,
        "Returned data words not match: {:x}, {:x}",
        new_prng_state,
        table_entry
    );
    (new_prng_state, table_entry)
}

struct PRNG {
    table: [u32; 256],
}

impl PRNG {
    pub fn new(seed: u32) -> Self {
        let mut prng_state: u32 = seed;
        let mut table: [u32; 256] = [0; 256];

        table[0] = 0;
        table[1] = 103;

        for c in table[2..=251].rchunks_mut(1) {
            match prng_state_iterate(prng_state) {
                (new_prng_state, fill_value) => {
                    prng_state = new_prng_state;
                    c[0] = fill_value;
                }
            }
        }

        let mut mask: u32 = 0xffffffff;
        let mut bit: u32 = 0x80000000;
        let mut i = 5;
        while bit != 0 {
            table[i] = bit | table[i] & mask;
            i += 7;
            bit >>= 1;
            mask >>= 1;
        }
        PRNG { table }
    }

    pub fn next(self: &mut Self) -> u32 {
        let table = &mut self.table;
        let a = table[0];
        table[0] = Self::pseudo_map_lookup(a);
        let b = table[1];
        table[1] = Self::pseudo_map_lookup(b);
        let c = table[b.wrapping_add(2i32 as u32) as usize]
            ^ table[a.wrapping_add(2i32 as u32) as usize];
        table[a.wrapping_add(2i32 as u32) as usize] = c;
        c
    }

    fn pseudo_map_lookup(x: u32) -> u32 {
        if x >= 0xf9 {
            0
        } else {
            x + 1
        }
    }
}

pub fn decrypt(input: &mut [u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(input.len());

    let mut prng = PRNG::new(u32::from_le_bytes(input[..4].try_into().unwrap()));
    result.extend_from_slice(&input[0..4]);

    let chunks_iter = input[4..].chunks_exact(4);
    let remainder = chunks_iter.remainder();
    for chunk in chunks_iter {
        let current = u32::from_le_bytes(chunk.try_into().unwrap()) ^ prng.next();
        result.extend_from_slice(&u32::to_le_bytes(current));
    }
    for chunk in remainder.iter() {
        result.push(*chunk ^ prng.next() as u8);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prng_state_iterate() {
        assert_eq!(prng_state_iterate(0x7654_3210), (0xd17a_6109, 0x0a14_d17a));
        assert_eq!(prng_state_iterate(0x0000_0000), (12345, 0));
        assert_eq!(prng_state_iterate(0x0000_0001), (0x41c6_7ea6, 0x41c6));
        assert_eq!(prng_state_iterate(0xffff_ffff), (0xbe39e1cc, 0x4e6cbe39));
    }
}
