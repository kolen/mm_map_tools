// Based on:
//
// Magic & Mayhem decryption and unpacking routines
// Author: Nikita Sadkov
// License: GPL2

#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_mut
)]

static mut PRNG_State: u32 = 0;
static mut PRNG_Map: [u32; 256] = [0; 256];
static mut PRNG_MapReady: i32 = 0;

unsafe fn prng(mut table: *mut u32) -> u32 {
    let mut a: u32 = 0;
    let mut b: u32 = 0;
    let mut c: u32 = 0;
    a = *table.offset(0isize);
    *table.offset(0isize) = PRNG_Map[a as usize];
    b = *table.offset(1isize);
    *table.offset(1isize) = PRNG_Map[b as usize];
    c = *table.offset(b.wrapping_add(2i32 as u32) as isize)
        ^ *table.offset(a.wrapping_add(2i32 as u32) as isize);
    *table.offset(a.wrapping_add(2i32 as u32) as isize) = c;
    return c;
}

pub unsafe fn prng_init(mut table: *mut u32, mut seed: u32) {
    let mut p: *mut u32 = 0 as *mut u32;
    let mut i: i32 = 0;
    let mut count: i32 = 0;
    let mut t: i64 = 0;
    let mut a: u32 = 0;
    let mut k: u32 = 0;
    let mut q: *mut i32 = 0 as *mut i32;
    let mut b: i32 = 0;
    if 0 == PRNG_MapReady {
        i = 0i32;
        while i < 0xf9i32 {
            PRNG_Map[i as usize] = (i + 1i32) as u32;
            i += 1
        }
        PRNG_Map[0xf9i32 as usize] = 0i32 as u32
    }
    PRNG_State = seed;
    *table.offset(0isize) = 0i32 as u32;
    *table.offset(1isize) = 103i32 as u32;
    p = table.offset(251isize);
    count = 250i32;
    i = 0i32;
    while i < 250i32 {
        t = 0x41c64e6du64.wrapping_mul(PRNG_State as u64) as i64;
        *(&mut t as *mut i64 as *mut u32).offset(1isize) <<= 16i32;
        t = (t as u64).wrapping_add(0xffff00003039u64) as i64 as i64;
        PRNG_State = t as u32;
        *p = *(&mut t as *mut i64 as *mut u32).offset(1isize) & 0xffff0000u32 | t as u32 >> 16i32;
        p = p.offset(-1isize);
        i += 1
    }
    a = 0xffffffffu32;
    k = 0x80000000u32;
    q = table.offset(5isize) as *mut i32;
    loop {
        b = *q;
        *q = (k | a & b as u32) as i32;
        q = q.offset(7isize);
        k >>= 1i32;
        a >>= 1i32;
        if !(0 != k) {
            break;
        }
    }
}
// it seeds random number generator with a key
// then it uses generated random numbers to XOR the input
pub unsafe fn decrypt(mut Input: *mut u8, mut Size: i32) {
    let mut I: i32 = 0;
    let mut Len: i32 = 0;
    let mut Table: [u32; 256] = [0; 256];
    let mut P: *mut u32 = Input as *mut u32;
    prng_init(Table.as_mut_ptr(), 1234567890u32);
    Table[254usize] = 0i32 as u32;
    let fresh0 = P;
    P = P.offset(1);
    prng_init(Table.as_mut_ptr(), *fresh0);
    Len = (Size - 4i32) / 4i32;
    I = 0i32;
    while I < Len {
        let fresh1 = P;
        P = P.offset(1);
        *fresh1 ^= prng(Table.as_mut_ptr());
        I += 1
    }
    Len = (Size - 4i32) % 4i32;
    I = 0i32;
    while I < Len {
        let ref mut fresh2 = *(P as *mut u8);
        *fresh2 = (*fresh2 as u32 ^ prng(Table.as_mut_ptr())) as u8;
        I += 1
    }
}
