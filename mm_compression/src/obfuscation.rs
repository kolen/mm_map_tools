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
        let mut t = 0x41c6_4e6du64.wrapping_mul(seed as u64) as u64;

        let mut t_hi: u32 = (t >> 32) as u32;
        t_hi = t_hi.wrapping_shl(16);

        let t_lo: u32 = t as u32;

        t = ((t_hi as u64) << 32) | (t_lo as u64);
        t = t.wrapping_add(0xffff_0000_3039);

        let new_seed = t as u32;
        let table_entry: u32 = (((t >> 32) as u32) & 0xffff_0000) | ((t as u32) >> 16);

        debug_assert_eq!(
            (new_seed & 0xffff_0000) >> 16,
            table_entry & 0xffff,
            "Returned data words not match: {:x}, {:x}",
            new_seed,
            table_entry
        );
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

/// Deobfuscates obfuscated data or obfuscates plaintext data (it's
/// the same operation).
pub fn process(input: &[u8]) -> Result<Vec<u8>, InputTooSmall> {
    let mut prng = PRNG::new(u32::from_le_bytes(
        input.get(..4).ok_or(InputTooSmall)?.try_into().unwrap(),
    ));

    let mut result = Vec::with_capacity(input.len());
    result.extend_from_slice(&input[0..4]);

    let chunks_iter = input[4..].chunks_exact(4);
    let remainder = chunks_iter.remainder();

    for chunk in chunks_iter {
        let current = u32::from_le_bytes(chunk.try_into().unwrap()) ^ prng.next().unwrap();
        result.extend_from_slice(&u32::to_le_bytes(current));
    }
    for chunk in remainder.iter() {
        result.push(*chunk ^ prng.next().unwrap() as u8);
    }
    Ok(result)
}

#[cfg(test)]
mod test {
    use super::{process, InputTooSmall};

    #[test]
    fn test_decrypt_two_times_returns_original() {
        let source: &[u8] = "The quick brown fox jumps over the lazy dog".as_bytes();
        let result = process(&process(source).unwrap()).unwrap();
        assert_eq!(source, &result);
    }

    #[test]
    fn test_basic() {
        let source = (123..140).collect::<Vec<u8>>();
        let expected: &[u8] = &[
            123, 124, 125, 126, 22, 203, 42, 122, 69, 220, 114, 34, 148, 54, 160, 111, 66,
        ];
        let result = process(&source[..]).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_basic_no_remainder() {
        let source = (123..139).collect::<Vec<u8>>();
        let expected: &[u8] = &[
            123, 124, 125, 126, 22, 203, 42, 122, 69, 220, 114, 34, 148, 54, 160, 111,
        ];
        let result = process(&source[..]).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_empty() {
        assert_eq!(InputTooSmall, process(&[]).unwrap_err());
    }

    #[test]
    fn test_too_small() {
        assert_eq!(InputTooSmall, process(&[1, 2]).unwrap_err());
    }
}