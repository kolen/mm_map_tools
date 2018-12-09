use byteorder::{ByteOrder, LittleEndian};
use std::error;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::sync::{Mutex, Once};

extern "C" {
    fn init_prng_map();
    fn decrypt(input: *mut u8, size: usize);
    fn lz_unpack(input: *const u8, output: *mut u8, unpacked_size: isize);
}

#[derive(Debug)]
enum CompressionType {
    Uncompressed = 0,
    RLE = 1,
    LZ77 = 2,
}

#[derive(Debug)]
struct Header {
    seed: u32,
    unpacked_size: u32,
    checksum_deobfuscated: u32,
    checksum_uncompressed: u32,
    compression: CompressionType,
}

const HEADER_SIZE: usize = 4 * 5;

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
                _ => panic!("Invalid compression type"), // TODO: return error
            },
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
    FileError { error: std::io::Error },
}

impl fmt::Display for DecompressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let suberror = match self {
            DecompressError::DeobfuscateChecksumNotMatch => {
                "Deobfuscation checksum does not match".into()
            }
            DecompressError::DecompressChecksumNonMatch => {
                "Decompression checksum does not match".into()
            }
            DecompressError::InvalidCompressionType => "Invalid compression type".into(),
            DecompressError::CompressionNotSupported => "Compression not supported".into(),
            DecompressError::ContentTooSmall => "File contents are too small".into(),
            DecompressError::FileError { error: e } => format!("File reading error: {}", e),
        };
        write!(f, "Decompression error: {}", suberror)
    }
}

impl error::Error for DecompressError {}

impl From<std::io::Error> for DecompressError {
    fn from(error: std::io::Error) -> Self {
        DecompressError::FileError { error: error }
    }
}

static PRNG_INITIALIZED: Once = Once::new();
lazy_static! {
    static ref EXTERNAL_LIB_LOCK: Mutex<()> = Mutex::new(());
}

fn init_prng_once() {
    PRNG_INITIALIZED.call_once(|| unsafe {
        init_prng_map();
    });
}

fn checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    let mut odd: bool = false;
    // Change to exact_chunks when it stabilize
    for chunk in data.chunks(4) {
        let element: u32 = if chunk.len() == 4 {
            LittleEndian::read_u32(chunk)
        } else {
            0 // Incomplete 32-bit uint is treated as zero
        };

        if odd {
            sum = sum.wrapping_add(element);
        } else {
            sum ^= element;
        }
        odd = !odd;
    }
    sum
}

fn deobfuscate(input: &mut [u8]) -> Result<(), DecompressError> {
    init_prng_once();
    unsafe {
        decrypt(input.as_mut_ptr(), input.len());
    }
    let header = Header::from_bytes(input)?;
    if header.checksum_deobfuscated == checksum(&input[HEADER_SIZE..]) {
        Ok(())
    } else {
        Err(DecompressError::DeobfuscateChecksumNotMatch)
    }
}

fn lz77_decompress(input: &[u8]) -> Result<Vec<u8>, DecompressError> {
    let header = Header::from_bytes(input)?;
    // TODO: Why * 2? Seen this in mmdecrypt.c
    let mut buffer = vec![0; (header.unpacked_size * 2) as usize];
    unsafe {
        lz_unpack(
            input[HEADER_SIZE..].as_ptr(),
            buffer.as_mut_ptr(),
            header.unpacked_size as isize,
        );
    }

    buffer.resize(header.unpacked_size as usize, 0);
    if header.checksum_uncompressed == checksum(&buffer) {
        Ok(buffer)
    } else {
        Err(DecompressError::DecompressChecksumNonMatch)
    }
}

pub fn decompress(input: &mut [u8]) -> Result<Vec<u8>, DecompressError> {
    if input.len() <= 20 {
        return Err(DecompressError::ContentTooSmall);
    }
    // mmdecrypt.c is not thread-safe
    let _lock = EXTERNAL_LIB_LOCK.lock().unwrap();
    deobfuscate(input)?;
    let header = Header::from_bytes(input)?;
    match header.compression {
        CompressionType::Uncompressed => Ok(input[HEADER_SIZE..].to_vec()),
        CompressionType::RLE => Err(DecompressError::CompressionNotSupported),
        CompressionType::LZ77 => lz77_decompress(input),
    }
}

pub fn read_decompressed<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, DecompressError> {
    let mut f = File::open(&path)?;
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    decompress(&mut buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils::*;
    #[test]
    fn test_decompress() {
        let decoded = read_decompressed(&test_file_path("Realms/Celtic/Forest/CFsec50.map"));
        assert!(decoded.is_ok(), "Decompress failed: {:?}", decoded);
        assert_eq!(6, LittleEndian::read_u32(&decoded.unwrap()));
    }

    #[test]
    fn test_too_short() {
        let decoded = decompress(&mut vec![0; 10]);
        match decoded.unwrap_err() {
            DecompressError::ContentTooSmall => (),
            x => panic!("Invalid error {:?}", x),
        }
    }
}
