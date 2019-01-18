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

fn prng(mut table: &mut [u32; 256]) -> u32 {
    let a = table[0];
    table[0] = prng_map_lookup(a);
    let b = table[1];
    table[1] = prng_map_lookup(b);
    let c =
        table[b.wrapping_add(2i32 as u32) as usize] ^ table[a.wrapping_add(2i32 as u32) as usize];
    table[a.wrapping_add(2i32 as u32) as usize] = c;
    return c;
}

fn prng_init(table: &mut [u32; 256], mut seed: u32) {
    let mut p: *mut u32 = 0 as *mut u32;
    let mut i: i32 = 0;
    let mut count: i32 = 0;
    let mut t: i64 = 0;
    let mut a: u32 = 0;
    let mut k: u32 = 0;
    let mut q: *mut i32 = 0 as *mut i32;
    let mut b: i32 = 0;
    let mut prng_state: u32 = seed;

    table[0] = 0;
    table[1] = 103;
    unsafe {
        p = table.as_mut_ptr().offset(251);
    }
    count = 250;
    i = 0;
    while i < 250 {
        unsafe {
            t = 0x41c64e6du64.wrapping_mul(prng_state as u64) as i64;
            *(&mut t as *mut i64 as *mut u32).offset(1isize) <<= 16i32;
            t = (t as u64).wrapping_add(0xffff00003039u64) as i64 as i64;
            prng_state = t as u32;
            *p = *(&mut t as *mut i64 as *mut u32).offset(1isize) & 0xffff0000u32
                | t as u32 >> 16i32;
            p = p.offset(-1isize);
        }
        i += 1
    }
    a = 0xffffffffu32;
    k = 0x80000000u32;
    unsafe {
        q = table.as_mut_ptr().offset(5) as *mut i32;
    }
    loop {
        unsafe {
            b = *q;
            *q = (k | a & b as u32) as i32;
            q = q.offset(7isize);
        }
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
