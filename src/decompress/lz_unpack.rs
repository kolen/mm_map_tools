// Based on the "mmdecrypt.c" from SAU: Sprite and Archive Utility
// project:
//
// https://github.com/saniv/sau/blob/master/unsorted/scraps/mmdecrypt.c
//
// Magic & Mayhem decryption and unpacking routines
// Author: Nikita Sadkov
// License: GPL2

use std::iter::Iterator;
use std::result::Result;

pub struct PrematureEnd {
    pub context_line: u32,
}

struct DictTraverse {
    back_ref_off: i32,
    back_ref_i: i32,
    back_ref_len: i32,
}

struct LZReader<I>
where
    I: Iterator<Item = u8>,
{
    input: I,
    lz_dict: [u8; 4096],
    bit_ptr: u8,
    lz_value: u32,
    count: usize,
    count_save: usize,
    dict_index: i32,
    unpacked_size: usize,
    dict_traverse: Option<DictTraverse>,
}

impl<I> LZReader<I>
where
    I: Iterator<Item = u8>,
{
    pub fn new(input: I, unpacked_size: usize) -> Self {
        LZReader {
            input,
            lz_dict: [0; 4096],
            bit_ptr: 0x80,
            lz_value: 0,
            count: 0,
            count_save: 0,
            dict_index: 1,
            unpacked_size,
            dict_traverse: None,
        }
    }

    fn read(&mut self) -> bool {
        if let Some(value) = self.input.next() {
            self.lz_value = value as u32;
            true
        } else {
            false
        }
    }

    fn traverse_dict(&mut self) -> Option<Result<u8, PrematureEnd>> {
        let mut dt = &mut self.dict_traverse.as_mut().unwrap();

        let value_2 = self.lz_dict[(dt.back_ref_off + dt.back_ref_i & 0xfff) as usize];
        self.count += 1;
        self.count_save = self.count;
        if self.count == self.unpacked_size {
            return None;
        }
        self.lz_dict[self.dict_index as usize] = value_2;
        dt.back_ref_i += 1;
        self.dict_index = self.dict_index as u16 as i32 + 1 & 0xfffi32;
        if !(dt.back_ref_i < dt.back_ref_len + 2) {
            self.dict_traverse = None;
        }

        Some(Ok(value_2))
    }
}

impl<I> Iterator for LZReader<I>
where
    I: Iterator<Item = u8>,
{
    type Item = Result<u8, PrematureEnd>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut value: i8;
        let mut bit: u8 = self.bit_ptr;
        if bit as i32 == 0x80 {
            if !self.read() {
                return Some(Err(PrematureEnd {
                    context_line: line!(),
                }));
            }
        }
        let value_bit: i32 = (self.lz_value & bit as u32) as i32;
        let next_bit: i8 = (bit as i32 >> 1) as i8;
        self.bit_ptr = next_bit as u8;
        if 0 == next_bit {
            self.bit_ptr = 0x80u8
        }
        assert!(self.count <= self.unpacked_size);
        if self.count == self.unpacked_size {
            return None;
        }
        if 0 != value_bit {
            let mut high_bit: u32 = 0x80 as u32;
            value = 0 as i8;
            loop {
                bit = self.bit_ptr;
                if bit as i32 == 0x80 {
                    if !self.read() {
                        return Some(Err(PrematureEnd {
                            context_line: line!(),
                        }));
                    }
                }
                if 0 != bit as u32 & self.lz_value {
                    value = (value as u32 | high_bit) as i8
                }
                high_bit >>= 1;
                let next_bit_2: i8 = (bit as i32 >> 1) as i8;
                self.bit_ptr = next_bit_2 as u8;
                if 0 == next_bit_2 {
                    self.bit_ptr = 0x80 as u8
                }
                if !(0 != high_bit) {
                    break;
                }
            }
            self.count += 1;
            self.lz_dict[self.dict_index as usize] = value as u8;
            self.count_save = self.count;
            self.dict_index = self.dict_index as u16 as i32 + 1 & 0xfff;
            Some(Ok(value as u8))
        } else {
            if self.dict_traverse.is_some() {
                return self.traverse_dict();
            }

            let mut back_ref_bit: u32 = 0x800;
            let mut back_ref_off: i32 = 0;
            loop {
                bit = self.bit_ptr;
                if bit as i32 == 0x80 {
                    if !self.read() {
                        return Some(Err(PrematureEnd {
                            context_line: line!(),
                        }));
                    }
                }
                if 0 != bit as u32 & self.lz_value {
                    back_ref_off = (back_ref_off as u32 | back_ref_bit) as i32
                }
                back_ref_bit >>= 1;
                let next_bit_3: i8 = (bit as i32 >> 1) as i8;
                self.bit_ptr = next_bit_3 as u8;
                if 0 == next_bit_3 {
                    self.bit_ptr = 0x80 as u8
                }
                if !(0 != back_ref_bit) {
                    break;
                }
            }
            if 0 == back_ref_off {
                return None;
            }
            let mut low_bit: u32 = 8;
            let mut back_ref_len: i32 = 0;
            loop {
                bit = self.bit_ptr;
                if bit as i32 == 0x80 {
                    if !self.read() {
                        return Some(Err(PrematureEnd {
                            context_line: line!(),
                        }));
                    }
                    self.count = self.count_save;
                }
                if 0 != bit as u32 & self.lz_value {
                    back_ref_len = (back_ref_len as u32 | low_bit) as i32
                }
                low_bit >>= 1;
                let next_bit_4: i8 = (bit as i32 >> 1) as i8;
                self.bit_ptr = next_bit_4 as u8;
                if 0 == next_bit_4 {
                    self.bit_ptr = 0x80 as u8
                }
                if !(0 != low_bit) {
                    break;
                }
            }
            let back_ref_i: i32 = 0;
            if back_ref_len + 1 >= 0 {
                self.dict_traverse = Some(DictTraverse {
                    back_ref_off,
                    back_ref_i,
                    back_ref_len,
                });

                self.traverse_dict()
            } else {
                unimplemented!();
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.unpacked_size))
    }
}

pub fn lz_unpack(
    input: impl IntoIterator<Item = u8>,
    unpacked_size: usize,
) -> Result<Vec<u8>, PrematureEnd> {
    let reader = LZReader::new(input.into_iter(), unpacked_size);
    reader.collect()
}
