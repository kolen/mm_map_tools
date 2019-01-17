// Based on:
//
// Magic & Mayhem decryption and unpacking routines
// Author: Nikita Sadkov
// License: GPL2

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct LZInput {
    pub ptr: *const u8,
    pub bit_ptr: u8,
    pub value: u32,
}

pub unsafe fn lz_unpack(input: *const u8, mut output: *mut u8, unpacked_size: usize) {
    let mut lz_dict: [u8; 4096] = [0; 4096];
    let mut lz_input: LZInput = LZInput {
        ptr: 0 as *const u8,
        bit_ptr: 0,
        value: 0,
    };
    let mut ptr_inc: *const u8 = 0 as *const u8;
    let mut value: i8 = 0;

    let lz = &mut lz_input;
    (*lz).ptr = input;
    (*lz).bit_ptr = 0x80i32 as u8;
    (*lz).value = 0i32 as u32;
    let mut count = 0;
    let mut count_save = 0;
    let mut dict_index: i32 = 1;
    loop {
        let mut bit: u8 = (*lz).bit_ptr;
        if bit as i32 == 0x80i32 {
            ptr_inc = (*lz).ptr.offset(1isize);
            (*lz).value = *(*lz).ptr as u32;
            (*lz).ptr = ptr_inc
        }
        let value_bit: i32 = ((*lz).value & bit as u32) as i32;
        let next_bit: i8 = (bit as i32 >> 1i32) as i8;
        (*lz).bit_ptr = next_bit as u8;
        if 0 == next_bit {
            (*lz).bit_ptr = 0x80i32 as u8
        }
        if 0 != value_bit {
            let mut high_bit: u32 = 0x80i32 as u32;
            value = 0i32 as i8;
            loop {
                bit = (*lz).bit_ptr;
                if bit as i32 == 0x80i32 {
                    ptr_inc = (*lz).ptr.offset(1isize);
                    (*lz).value = *(*lz).ptr as u32;
                    (*lz).ptr = ptr_inc
                }
                if 0 != bit as u32 & (*lz).value {
                    value = (value as u32 | high_bit) as i8
                }
                high_bit >>= 1i32;
                let next_bit_2: i8 = (bit as i32 >> 1i32) as i8;
                (*lz).bit_ptr = next_bit_2 as u8;
                if 0 == next_bit_2 {
                    (*lz).bit_ptr = 0x80i32 as u8
                }
                if !(0 != high_bit) {
                    break;
                }
            }
            let fresh0 = output;
            output = output.offset(1);
            *fresh0 = value as u8;
            count += 1;
            lz_dict[dict_index as usize] = value as u8;
            count_save = count;
            dict_index = dict_index as u16 as i32 + 1i32 & 0xfffi32
        } else {
            let mut back_ref_bit: u32 = 0x800i32 as u32;
            let mut back_ref_off: i32 = 0;
            loop {
                bit = (*lz).bit_ptr;
                if bit as i32 == 0x80i32 {
                    ptr_inc = (*lz).ptr.offset(1isize);
                    (*lz).value = *(*lz).ptr as u32;
                    (*lz).ptr = ptr_inc
                }
                if 0 != bit as u32 & (*lz).value {
                    back_ref_off = (back_ref_off as u32 | back_ref_bit) as i32
                }
                back_ref_bit >>= 1i32;
                let next_bit_3: i8 = (bit as i32 >> 1i32) as i8;
                (*lz).bit_ptr = next_bit_3 as u8;
                if 0 == next_bit_3 {
                    (*lz).bit_ptr = 0x80i32 as u8
                }
                if !(0 != back_ref_bit) {
                    break;
                }
            }
            if 0 == back_ref_off {
                return;
            }
            let mut low_bit: u32 = 8;
            let mut back_ref_len: i32 = 0;
            loop {
                bit = (*lz).bit_ptr;
                if bit as i32 == 0x80i32 {
                    ptr_inc = (*lz).ptr.offset(1isize);
                    (*lz).value = *(*lz).ptr as u32;
                    count = count_save;
                    (*lz).ptr = ptr_inc
                }
                if 0 != bit as u32 & (*lz).value {
                    back_ref_len = (back_ref_len as u32 | low_bit) as i32
                }
                low_bit >>= 1i32;
                let next_bit_4: i8 = (bit as i32 >> 1i32) as i8;
                (*lz).bit_ptr = next_bit_4 as u8;
                if 0 == next_bit_4 {
                    (*lz).bit_ptr = 0x80i32 as u8
                }
                if !(0 != low_bit) {
                    break;
                }
            }
            let mut back_ref_i: i32 = 0i32;
            if back_ref_len + 1i32 >= 0i32 {
                loop {
                    let value_2 = lz_dict
                        [(back_ref_off as u16 as i32 + back_ref_i as u16 as i32 & 0xfffi32) as usize]
                        as i32;
                    let fresh1 = output;
                    output = output.offset(1);
                    *fresh1 = value_2 as u8;
                    count += 1;
                    count_save = count;
                    if count == unpacked_size {
                        return;
                    }
                    lz_dict[dict_index as usize] = value_2 as u8;
                    back_ref_i += 1;
                    dict_index = dict_index as u16 as i32 + 1i32 & 0xfffi32;
                    if !(back_ref_i < back_ref_len + 2i32) {
                        break;
                    }
                }
            }
        }
        if count == unpacked_size {
            return;
        }
    }
}
