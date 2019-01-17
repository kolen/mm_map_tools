// Based on:
//
// Magic & Mayhem decryption and unpacking routines
// Author: Nikita Sadkov
// License: GPL2

#![allow(non_snake_case, non_upper_case_globals, unused_mut)]

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct lz_input {
    pub Ptr: *const u8,
    pub BitPtr: u8,
    pub Value: u32,
}

pub unsafe fn lz_unpack(Input: *const u8, mut Output: *mut u8, UnpackedSize: usize) {
    let mut LZDict: [u8; 4096] = [0; 4096];
    let mut LZInput: lz_input = lz_input {
        Ptr: 0 as *const u8,
        BitPtr: 0,
        Value: 0,
    };
    let mut lz: *mut lz_input = 0 as *mut lz_input;
    let mut count: usize = 0;
    let mut bit: u8 = 0;
    let mut ptr_inc: *const u8 = 0 as *const u8;
    let mut value_bit: i32 = 0;
    let mut next_bit: i8 = 0;
    let mut value: i8 = 0;
    let mut next_bit_2: i8 = 0;
    let mut back_ref_bit: u32 = 0;
    let mut back_ref_off: i32 = 0;
    let mut back_ref_i: i32 = 0;
    let mut back_ref_len: i32 = 0;
    let mut next_bit_3: i8 = 0;
    let mut low_bit: u32 = 0;
    let mut high_bit: u32 = 0;
    let mut next_bit_4: i8 = 0;
    let mut value_2: i32 = 0;
    let mut dict_index: i32 = 0;
    let mut count_save: usize = 0;

    lz = &mut LZInput;
    (*lz).Ptr = Input;
    (*lz).BitPtr = 0x80i32 as u8;
    (*lz).Value = 0i32 as u32;
    count = 0;
    count_save = 0;
    dict_index = 1i32;
    loop {
        bit = (*lz).BitPtr;
        if bit as i32 == 0x80i32 {
            ptr_inc = (*lz).Ptr.offset(1isize);
            (*lz).Value = *(*lz).Ptr as u32;
            (*lz).Ptr = ptr_inc
        }
        value_bit = ((*lz).Value & bit as u32) as i32;
        next_bit = (bit as i32 >> 1i32) as i8;
        (*lz).BitPtr = next_bit as u8;
        if 0 == next_bit {
            (*lz).BitPtr = 0x80i32 as u8
        }
        if 0 != value_bit {
            high_bit = 0x80i32 as u32;
            value = 0i32 as i8;
            loop {
                bit = (*lz).BitPtr;
                if bit as i32 == 0x80i32 {
                    ptr_inc = (*lz).Ptr.offset(1isize);
                    (*lz).Value = *(*lz).Ptr as u32;
                    (*lz).Ptr = ptr_inc
                }
                if 0 != bit as u32 & (*lz).Value {
                    value = (value as u32 | high_bit) as i8
                }
                high_bit >>= 1i32;
                next_bit_2 = (bit as i32 >> 1i32) as i8;
                (*lz).BitPtr = next_bit_2 as u8;
                if 0 == next_bit_2 {
                    (*lz).BitPtr = 0x80i32 as u8
                }
                if !(0 != high_bit) {
                    break;
                }
            }
            let fresh0 = Output;
            Output = Output.offset(1);
            *fresh0 = value as u8;
            count += 1;
            LZDict[dict_index as usize] = value as u8;
            count_save = count;
            dict_index = dict_index as u16 as i32 + 1i32 & 0xfffi32
        } else {
            back_ref_bit = 0x800i32 as u32;
            back_ref_off = 0i32;
            loop {
                bit = (*lz).BitPtr;
                if bit as i32 == 0x80i32 {
                    ptr_inc = (*lz).Ptr.offset(1isize);
                    (*lz).Value = *(*lz).Ptr as u32;
                    (*lz).Ptr = ptr_inc
                }
                if 0 != bit as u32 & (*lz).Value {
                    back_ref_off = (back_ref_off as u32 | back_ref_bit) as i32
                }
                back_ref_bit >>= 1i32;
                next_bit_3 = (bit as i32 >> 1i32) as i8;
                (*lz).BitPtr = next_bit_3 as u8;
                if 0 == next_bit_3 {
                    (*lz).BitPtr = 0x80i32 as u8
                }
                if !(0 != back_ref_bit) {
                    break;
                }
            }
            if 0 == back_ref_off {
                return;
            }
            low_bit = 8i32 as u32;
            back_ref_len = 0i32;
            loop {
                bit = (*lz).BitPtr;
                if bit as i32 == 0x80i32 {
                    ptr_inc = (*lz).Ptr.offset(1isize);
                    (*lz).Value = *(*lz).Ptr as u32;
                    count = count_save;
                    (*lz).Ptr = ptr_inc
                }
                if 0 != bit as u32 & (*lz).Value {
                    back_ref_len = (back_ref_len as u32 | low_bit) as i32
                }
                low_bit >>= 1i32;
                next_bit_4 = (bit as i32 >> 1i32) as i8;
                (*lz).BitPtr = next_bit_4 as u8;
                if 0 == next_bit_4 {
                    (*lz).BitPtr = 0x80i32 as u8
                }
                if !(0 != low_bit) {
                    break;
                }
            }
            back_ref_i = 0i32;
            if back_ref_len + 1i32 >= 0i32 {
                loop {
                    value_2 = LZDict
                        [(back_ref_off as u16 as i32 + back_ref_i as u16 as i32 & 0xfffi32) as usize]
                        as i32;
                    let fresh1 = Output;
                    Output = Output.offset(1);
                    *fresh1 = value_2 as u8;
                    count += 1;
                    count_save = count;
                    if count == UnpackedSize {
                        return;
                    }
                    LZDict[dict_index as usize] = value_2 as u8;
                    back_ref_i += 1;
                    dict_index = dict_index as u16 as i32 + 1i32 & 0xfffi32;
                    if !(back_ref_i < back_ref_len + 2i32) {
                        break;
                    }
                }
            }
        }
        if count == UnpackedSize {
            return;
        }
    }
}
