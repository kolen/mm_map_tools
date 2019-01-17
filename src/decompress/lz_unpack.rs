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
    let mut LZ: *mut lz_input = 0 as *mut lz_input;
    let mut Count: usize = 0;
    let mut Bit: u8 = 0;
    let mut PtrInc: *const u8 = 0 as *const u8;
    let mut ValueBit: i32 = 0;
    let mut NextBit: i8 = 0;
    let mut Value: i8 = 0;
    let mut NextBit2: i8 = 0;
    let mut BackRefBit: u32 = 0;
    let mut BackRefOff: i32 = 0;
    let mut BackRefI: i32 = 0;
    let mut BackRefLen: i32 = 0;
    let mut NextBit3: i8 = 0;
    let mut LowBit: u32 = 0;
    let mut HighBit: u32 = 0;
    let mut NextBit4: i8 = 0;
    let mut Value2: i32 = 0;
    let mut DictIndex: i32 = 0;
    let mut CountSave: usize = 0;

    LZ = &mut LZInput;
    (*LZ).Ptr = Input;
    (*LZ).BitPtr = 0x80i32 as u8;
    (*LZ).Value = 0i32 as u32;
    Count = 0;
    CountSave = 0;
    DictIndex = 1i32;
    loop {
        Bit = (*LZ).BitPtr;
        if Bit as i32 == 0x80i32 {
            PtrInc = (*LZ).Ptr.offset(1isize);
            (*LZ).Value = *(*LZ).Ptr as u32;
            (*LZ).Ptr = PtrInc
        }
        ValueBit = ((*LZ).Value & Bit as u32) as i32;
        NextBit = (Bit as i32 >> 1i32) as i8;
        (*LZ).BitPtr = NextBit as u8;
        if 0 == NextBit {
            (*LZ).BitPtr = 0x80i32 as u8
        }
        if 0 != ValueBit {
            HighBit = 0x80i32 as u32;
            Value = 0i32 as i8;
            loop {
                Bit = (*LZ).BitPtr;
                if Bit as i32 == 0x80i32 {
                    PtrInc = (*LZ).Ptr.offset(1isize);
                    (*LZ).Value = *(*LZ).Ptr as u32;
                    (*LZ).Ptr = PtrInc
                }
                if 0 != Bit as u32 & (*LZ).Value {
                    Value = (Value as u32 | HighBit) as i8
                }
                HighBit >>= 1i32;
                NextBit2 = (Bit as i32 >> 1i32) as i8;
                (*LZ).BitPtr = NextBit2 as u8;
                if 0 == NextBit2 {
                    (*LZ).BitPtr = 0x80i32 as u8
                }
                if !(0 != HighBit) {
                    break;
                }
            }
            let fresh0 = Output;
            Output = Output.offset(1);
            *fresh0 = Value as u8;
            Count += 1;
            LZDict[DictIndex as usize] = Value as u8;
            CountSave = Count;
            DictIndex = DictIndex as u16 as i32 + 1i32 & 0xfffi32
        } else {
            BackRefBit = 0x800i32 as u32;
            BackRefOff = 0i32;
            loop {
                Bit = (*LZ).BitPtr;
                if Bit as i32 == 0x80i32 {
                    PtrInc = (*LZ).Ptr.offset(1isize);
                    (*LZ).Value = *(*LZ).Ptr as u32;
                    (*LZ).Ptr = PtrInc
                }
                if 0 != Bit as u32 & (*LZ).Value {
                    BackRefOff = (BackRefOff as u32 | BackRefBit) as i32
                }
                BackRefBit >>= 1i32;
                NextBit3 = (Bit as i32 >> 1i32) as i8;
                (*LZ).BitPtr = NextBit3 as u8;
                if 0 == NextBit3 {
                    (*LZ).BitPtr = 0x80i32 as u8
                }
                if !(0 != BackRefBit) {
                    break;
                }
            }
            if 0 == BackRefOff {
                return;
            }
            LowBit = 8i32 as u32;
            BackRefLen = 0i32;
            loop {
                Bit = (*LZ).BitPtr;
                if Bit as i32 == 0x80i32 {
                    PtrInc = (*LZ).Ptr.offset(1isize);
                    (*LZ).Value = *(*LZ).Ptr as u32;
                    Count = CountSave;
                    (*LZ).Ptr = PtrInc
                }
                if 0 != Bit as u32 & (*LZ).Value {
                    BackRefLen = (BackRefLen as u32 | LowBit) as i32
                }
                LowBit >>= 1i32;
                NextBit4 = (Bit as i32 >> 1i32) as i8;
                (*LZ).BitPtr = NextBit4 as u8;
                if 0 == NextBit4 {
                    (*LZ).BitPtr = 0x80i32 as u8
                }
                if !(0 != LowBit) {
                    break;
                }
            }
            BackRefI = 0i32;
            if BackRefLen + 1i32 >= 0i32 {
                loop {
                    Value2 = LZDict
                        [(BackRefOff as u16 as i32 + BackRefI as u16 as i32 & 0xfffi32) as usize]
                        as i32;
                    let fresh1 = Output;
                    Output = Output.offset(1);
                    *fresh1 = Value2 as u8;
                    Count += 1;
                    CountSave = Count;
                    if Count == UnpackedSize {
                        return;
                    }
                    LZDict[DictIndex as usize] = Value2 as u8;
                    BackRefI += 1;
                    DictIndex = DictIndex as u16 as i32 + 1i32 & 0xfffi32;
                    if !(BackRefI < BackRefLen + 2i32) {
                        break;
                    }
                }
            }
        }
        if Count == UnpackedSize {
            return;
        }
    }
}
