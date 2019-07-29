// Based on the "mmdecrypt.c" from SAU: Sprite and Archive Utility
// project:
//
// https://github.com/saniv/sau/blob/master/unsorted/scraps/mmdecrypt.c
//
// Magic & Mayhem decryption and unpacking routines
// Author: Nikita Sadkov
// License: GPL2

use std::result::Result;

pub struct PrematureEnd {
    pub context_line: u32,
}

struct PackedDataReader<I: Iterator<Item = u8>> {
    iter: I,
}

impl<I: Iterator<Item = u8>> PackedDataReader<I> {
    fn read(&mut self, context_line: u32) -> Result<u32, PrematureEnd> {
        self.iter
            .next()
            .map(|a| a as u32)
            .ok_or(PrematureEnd { context_line })
    }
}

pub fn lz_unpack(
    input: impl IntoIterator<Item = u8>,
    unpacked_size: usize,
) -> Result<Vec<u8>, PrematureEnd> {
    let mut output: Vec<u8> = Vec::with_capacity(unpacked_size);

    let mut reader = PackedDataReader {
        iter: input.into_iter(),
    };

    let mut lz_dict: [u8; 4096] = [0; 4096];
    let mut bit_ptr: u8 = 0x80;
    let mut lz_value: u32 = 0;

    let mut count = 0;
    let mut count_save = 0;
    let mut dict_index: i32 = 1;
    loop {
        let mut value: i8;
        let mut bit: u8 = bit_ptr;
        if bit as i32 == 0x80 {
            lz_value = reader.read(line!())?;
        }
        let value_bit: i32 = (lz_value & bit as u32) as i32;
        let next_bit: i8 = (bit as i32 >> 1) as i8;
        bit_ptr = next_bit as u8;
        if 0 == next_bit {
            bit_ptr = 0x80 as u8
        }
        if 0 != value_bit {
            let mut high_bit: u32 = 0x80 as u32;
            value = 0 as i8;
            loop {
                bit = bit_ptr;
                if bit as i32 == 0x80 {
                    lz_value = reader.read(line!())?;
                }
                if 0 != bit as u32 & lz_value {
                    value = (value as u32 | high_bit) as i8
                }
                high_bit >>= 1;
                let next_bit_2: i8 = (bit as i32 >> 1) as i8;
                bit_ptr = next_bit_2 as u8;
                if 0 == next_bit_2 {
                    bit_ptr = 0x80 as u8
                }
                if !(0 != high_bit) {
                    break;
                }
            }
            output.push(value as u8);
            count += 1;
            lz_dict[dict_index as usize] = value as u8;
            count_save = count;
            dict_index = dict_index as u16 as i32 + 1 & 0xfff
        } else {
            let mut back_ref_bit: u32 = 0x800;
            let mut back_ref_off: i32 = 0;
            loop {
                bit = bit_ptr;
                if bit as i32 == 0x80 {
                    lz_value = reader.read(line!())?;
                }
                if 0 != bit as u32 & lz_value {
                    back_ref_off = (back_ref_off as u32 | back_ref_bit) as i32
                }
                back_ref_bit >>= 1;
                let next_bit_3: i8 = (bit as i32 >> 1) as i8;
                bit_ptr = next_bit_3 as u8;
                if 0 == next_bit_3 {
                    bit_ptr = 0x80 as u8
                }
                if !(0 != back_ref_bit) {
                    break;
                }
            }
            if 0 == back_ref_off {
                return Ok(output);
            }
            let mut low_bit: u32 = 8;
            let mut back_ref_len: i32 = 0;
            loop {
                bit = bit_ptr;
                if bit as i32 == 0x80 {
                    lz_value = reader.read(line!())?;
                    count = count_save;
                }
                if 0 != bit as u32 & lz_value {
                    back_ref_len = (back_ref_len as u32 | low_bit) as i32
                }
                low_bit >>= 1;
                let next_bit_4: i8 = (bit as i32 >> 1) as i8;
                bit_ptr = next_bit_4 as u8;
                if 0 == next_bit_4 {
                    bit_ptr = 0x80 as u8
                }
                if !(0 != low_bit) {
                    break;
                }
            }
            let mut back_ref_i: i32 = 0;
            if back_ref_len + 1 >= 0 {
                loop {
                    let value_2 = lz_dict[(back_ref_off + back_ref_i & 0xfff) as usize];
                    output.push(value_2);
                    count += 1;
                    count_save = count;
                    if count == unpacked_size {
                        return Ok(output);
                    }
                    lz_dict[dict_index as usize] = value_2;
                    back_ref_i += 1;
                    dict_index = dict_index as u16 as i32 + 1 & 0xfffi32;
                    if !(back_ref_i < back_ref_len + 2) {
                        break;
                    }
                }
            }
        }
        if count == unpacked_size {
            return Ok(output);
        }
    }
}
