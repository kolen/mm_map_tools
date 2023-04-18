// Based on the "mmdecrypt.c" from SAU: Sprite and Archive Utility
// project:
//
// https://github.com/saniv/sau/blob/master/unsorted/scraps/mmdecrypt.c
//
// Magic & Mayhem decryption and unpacking routines
// Author: Nikita Sadkov
// License: GPL2

#![allow(clippy::cast_lossless)]

use std::{convert::TryInto, error::Error, fmt::Display};

#[derive(Debug, PartialEq)]
pub struct InputTooSmall;

impl Display for InputTooSmall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "input should be at least 4 bytes")
    }
}
impl Error for InputTooSmall {}

struct PRNG {
    table: [u32; 250],
    i: usize,
    j: usize,
}

impl PRNG {
    pub fn new(seed: u32) -> Self {
        PRNG {
            table: Self::table_from_seed(seed),
            i: 0,
            j: 103,
        }
    }

    fn table_from_seed(seed: u32) -> [u32; 250] {
        let mut current_seed: u32 = seed;
        let mut table: [u32; 250] = [0; 250];

        for chunk in table.rchunks_mut(1) {
            let (new_seed, fill_value) = Self::seed_iterate(current_seed);
            current_seed = new_seed;
            chunk[0] = fill_value;
        }

        let mut mask: u32 = 0xffff_ffff;
        let mut bit: u32 = 0x8000_0000;
        let mut i = 3;
        while bit != 0 {
            table[i] = bit | table[i] & mask;
            i += 7;
            bit >>= 1;
            mask >>= 1;
        }

        table
    }

    fn seed_iterate(seed: u32) -> (u32, u32) {
        let value = (seed as u64).wrapping_mul(0x41c6_4e6d).wrapping_add(12345);
        let new_seed = value as u32;
        let table_entry = (value >> 16) as u32;
        (new_seed, table_entry)
    }
}

impl Iterator for PRNG {
    type Item = u32;

    fn next(self: &mut Self) -> Option<u32> {
        let value = self.table[self.i] ^ self.table[self.j];
        self.table[self.i] = value;

        self.i = (self.i + 1) % 250;
        self.j = (self.j + 1) % 250;

        Some(value)
    }
}

fn process(input: &[u8], seed: u32, output: &mut Vec<u8>) {
    let mut prng = PRNG::new(seed);
    let chunks_iter = input.chunks_exact(4);
    let remainder = chunks_iter.remainder();

    for chunk in chunks_iter {
        let current = u32::from_le_bytes(chunk.try_into().unwrap()) ^ prng.next().unwrap();
        output.extend_from_slice(&u32::to_le_bytes(current));
    }
    for chunk in remainder.iter() {
        output.push(*chunk ^ prng.next().unwrap() as u8);
    }
}

pub fn deobfuscate(input: &[u8]) -> Result<Vec<u8>, InputTooSmall> {
    let seed = u32::from_le_bytes(input.get(..4).ok_or(InputTooSmall)?.try_into().unwrap());
    let mut result = Vec::with_capacity(input.len());
    process(&input[4..], seed, &mut result);
    Ok(result)
}

pub fn obfuscate(input: &[u8], seed: u32) -> Vec<u8> {
    let mut result = Vec::with_capacity(input.len() + 4);
    result.extend(u32::to_le_bytes(seed));
    process(input, seed, &mut result);
    result
}

#[cfg(test)]
mod test {
    use super::{deobfuscate, obfuscate, InputTooSmall};

    #[test]
    fn test_obfuscate_then_deobfuscate() {
        let source: &[u8] = "The quick brown fox jumps over the lazy dog".as_bytes();
        let result = deobfuscate(&obfuscate(source, 123456)).unwrap();
        assert_eq!(source, &result);
    }

    #[test]
    fn test_deobfuscate_basic() {
        let source = (123..140).collect::<Vec<u8>>();
        let expected: &[u8] = &[22, 203, 42, 122, 69, 220, 114, 34, 148, 54, 160, 111, 66];
        let result = deobfuscate(&source[..]).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_deobfuscate_basic_no_remainder() {
        let source = (123..139).collect::<Vec<u8>>();
        let expected: &[u8] = &[22, 203, 42, 122, 69, 220, 114, 34, 148, 54, 160, 111];
        let result = deobfuscate(&source[..]).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_deobfuscate_empty() {
        assert_eq!(InputTooSmall, deobfuscate(&[]).unwrap_err());
    }

    #[test]
    fn test_deobfuscate_too_small() {
        assert_eq!(InputTooSmall, deobfuscate(&[1, 2]).unwrap_err());
    }
}
