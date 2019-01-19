// Based on:
//
// Magic & Mayhem decryption and unpacking routines
// Author: Nikita Sadkov
// License: GPL2

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

// it seeds random number generator with a key
// then it uses generated random numbers to XOR the input
pub fn decrypt(Input: &mut [u8]) {
    let mut I: usize = 0;
    let mut Len: usize = 0;
    let mut Table: [u32; 256] = [0; 256];
    let mut P: *mut u32 = Input.as_mut_ptr() as *mut u32;
    prng_init(&mut Table, 1234567890u32);
    Table[254usize] = 0i32 as u32;
    let fresh0 = P;
    unsafe {
        P = P.offset(1);
    }
    unsafe {
        prng_init(&mut Table, *fresh0);
    }
    Len = (Input.len() - 4) / 4;
    I = 0;
    while I < Len {
        let fresh1 = P;
        unsafe {
            P = P.offset(1);
            *fresh1 ^= prng(&mut Table);
        }
        I += 1
    }
    Len = (Input.len() - 4) % 4;
    I = 0;
    while I < Len {
        unsafe {
            let ref mut fresh2 = *(P as *mut u8);
            *fresh2 = (*fresh2 as u32 ^ prng(&mut Table)) as u8;
        }
        I += 1
    }
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
