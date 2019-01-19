// Based on:
//
// Magic & Mayhem decryption and unpacking routines
// Author: Nikita Sadkov
// License: GPL2

use byteorder::{ByteOrder, LittleEndian};

fn prng_map_lookup(x: u32) -> u32 {
    if x >= 0xf9 {
        0
    } else {
        x + 1
    }
}

fn prng(table: &mut [u32; 256]) -> u32 {
    let a = table[0];
    table[0] = prng_map_lookup(a);
    let b = table[1];
    table[1] = prng_map_lookup(b);
    let c =
        table[b.wrapping_add(2i32 as u32) as usize] ^ table[a.wrapping_add(2i32 as u32) as usize];
    table[a.wrapping_add(2i32 as u32) as usize] = c;
    return c;
}

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

fn prng_init(table: &mut [u32; 256], mut seed: u32) {
    let mut p: usize = 251;
    let mut q: usize = 5;
    let mut prng_state: u32 = seed;

    table[0] = 0;
    table[1] = 103;
    let mut i: i32 = 0;
    while i < 250 {
        match prng_state_iterate(prng_state) {
            (new_prng_state, fill_value) => {
                prng_state = new_prng_state;
                table[p] = fill_value;
            }
        }
        p -= 1;
        i += 1
    }
    let mut a: u32 = 0xffffffff;
    let mut k: u32 = 0x80000000;
    loop {
        let b = table[q];
        table[q] = (k | a & b) as u32;
        q += 7;
        k >>= 1i32;
        a >>= 1i32;
        if !(0 != k) {
            break;
        }
    }
}

pub fn decrypt(input: &mut [u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(input.len());
    let mut table: [u32; 256] = [0; 256];

    prng_init(&mut table, LittleEndian::read_u32(input));
    result.extend_from_slice(&input[0..4]);

    let chunks_iter = input[4..].chunks_exact(4);
    let remainder = chunks_iter.remainder();
    for chunk in chunks_iter {
        let mut out: [u8; 4] = [0; 4];
        LittleEndian::write_u32(&mut out, LittleEndian::read_u32(&chunk) ^ prng(&mut table));
        result.extend_from_slice(&out);
    }
    for chunk in remainder.iter() {
        result.push(*chunk ^ prng(&mut table) as u8);
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
