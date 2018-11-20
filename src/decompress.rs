use std::sync::Once;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use byteorder::{ByteOrder, LittleEndian};

extern "C" {
    fn init_prng_map();
    fn decrypt(input: *mut u8, size: usize);
    fn checksum(input: *const u8, size: isize) -> u32;
    fn lz_unpack(input: *const u8, output: *mut u8, unpacked_size: isize);
}

#[derive(Debug)]
enum CompressionType {
    Uncompressed = 0,
    RLE = 1,
    LZ77 = 2
}

#[derive(Debug)]
struct Header {
    seed: u32,
    unpacked_size: u32,
    checksum_deobfuscated: u32,
    checksum_uncompressed: u32,
    compression: CompressionType
}

const HEADER_SIZE:usize = 4 * 5;

impl Header {
    pub fn from_bytes(input: &[u8]) -> Result<Header, DecompressError> {
        Ok(Header {
            seed: LittleEndian::read_u32(&input[0x0..]),
            unpacked_size: LittleEndian::read_u32(&input[0x4..]),
            checksum_deobfuscated: LittleEndian::read_u32(&input[0x8..]),
            checksum_uncompressed: LittleEndian::read_u32(&input[0xc..]),
            compression: match LittleEndian::read_u32(&input[0x10..]) {
                x if x == CompressionType::Uncompressed as u32 => CompressionType::Uncompressed,
                x if x == CompressionType::RLE as u32 => CompressionType::RLE,
                x if x == CompressionType::LZ77 as u32 => CompressionType::LZ77,
                _ => panic!("Invalid compression type") // TODO: return error
            }
        })
    }
}

#[derive(Debug)]
pub enum DecompressError {
    DeobfuscateChecksumNotMatch,
    DecompressChecksumNonMatch,
    InvalidCompressionType,
    CompressionNotSupported,
    ContentTooSmall,
    FileError { error: std::io::Error } 
}

impl From<std::io::Error> for DecompressError {
    fn from(error: std::io::Error) -> Self {
        DecompressError::FileError { error: error }
    }
}

static prng_initialized:Once = Once::new();

fn init_prng_once() {
    prng_initialized.call_once(|| {
        unsafe { init_prng_map(); }
    });
}

fn checksum_(input: &[u8], size: isize) -> u32 {
    assert!(size > 0);
    assert!(size <= input.len() as isize);
    unsafe { checksum(input.as_ptr(), size) }
}

fn deobfuscate(input: &mut[u8], size: usize) -> Result<(), DecompressError> {
    init_prng_once();
    unsafe { decrypt(input.as_mut_ptr(), size); }
    let header = Header::from_bytes(input)?;
    if header.checksum_deobfuscated == checksum_(&input[HEADER_SIZE..], (size - HEADER_SIZE) as isize) {
        Ok(())
    } else {
        Err(DecompressError::DeobfuscateChecksumNotMatch)
    }
}

fn lz77_decompress(input: &mut[u8]) -> Result<Vec<u8>, DecompressError> {
    let header = Header::from_bytes(input)?;
     // TODO: Why * 2? Seen this in mmdecrypt.c
    let mut buffer = vec![0; (header.unpacked_size * 2) as usize];
    unsafe { lz_unpack(input[HEADER_SIZE..].as_ptr(), buffer.as_mut_ptr(), header.unpacked_size as isize); }

    if header.checksum_uncompressed == checksum_(&buffer, header.unpacked_size as isize) {
        Ok(buffer)
    } else {
        Err(DecompressError::DecompressChecksumNonMatch)
    }
}

pub fn decompress(input: &mut[u8], size: usize) -> Result<Vec<u8>, DecompressError> {
    if size <= 20 {
        return Err(DecompressError::ContentTooSmall);
    }
    deobfuscate(input, size)?;
    let header = Header::from_bytes(input)?;
    match header.compression {
        CompressionType::Uncompressed => Ok(input[HEADER_SIZE..].to_vec()),
        CompressionType::RLE => Err(DecompressError::CompressionNotSupported),
        CompressionType::LZ77 => lz77_decompress(input)
    }
}

pub fn read_decompressed<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, DecompressError> {
    let mut f = File::open(&path)?;
    let mut buffer = Vec::new();
    let size = fs::metadata(&path)?.len() as usize;
    f.read_to_end(&mut buffer)?;
    decompress(&mut buffer, size)
}

#[test]
fn test_decompress() {
    let decoded = read_decompressed("/Volumes/data/games/Magic and Mayhem/Realms/Celtic/Forest/CFsec50.map");
    assert!(decoded.is_ok());
    assert_eq!(6, LittleEndian::read_u32(&decoded.unwrap()));
}

#[test]
fn test_too_short() {
    let decoded = decompress(&mut vec![0; 10], 10);
    match decoded.unwrap_err() {
        DecompressError::ContentTooSmall => (),
        x => panic!("Invalid error {:?}", x)
    }
}
