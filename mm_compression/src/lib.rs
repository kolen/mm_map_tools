mod obfuscation;
mod lz_unpack;
pub mod test_utils;

use self::obfuscation::process;
use self::lz_unpack::{lz_unpack, PrematureEnd};
use std::convert::TryInto;
use std::error;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

#[derive(Debug)]
enum CompressionType {
    Uncompressed = 0,
    RLE = 1,
    LZ77 = 2,
}

#[derive(Debug)]
struct Header {
    #[allow(dead_code)]
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
            seed: u32::from_le_bytes(input[0x0..0x4].try_into()?),
            unpacked_size: u32::from_le_bytes(input[0x4..0x8].try_into()?),
            checksum_deobfuscated: u32::from_le_bytes(input[0x8..0xc].try_into()?),
            checksum_uncompressed: u32::from_le_bytes(input[0xc..0x10].try_into()?),
            compression: match u32::from_le_bytes(input[0x10..0x14].try_into()?) {
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
    PrematureEnd { context: Option<u32> },
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
            DecompressError::PrematureEnd { context: None } => "Premature end of file".into(),
            DecompressError::PrematureEnd {
                context: Some(line),
            } => format!(
                "Premature end of file when lz unpacking, lz.unpack.rs line {}",
                line
            ),
        };
        write!(f, "Decompression error: {}", suberror)
    }
}

impl error::Error for DecompressError {}

impl From<std::io::Error> for DecompressError {
    fn from(error: std::io::Error) -> Self {
        DecompressError::FileError { error }
    }
}

impl From<core::array::TryFromSliceError> for DecompressError {
    fn from(_error: core::array::TryFromSliceError) -> Self {
        DecompressError::PrematureEnd { context: None }
    }
}

impl From<PrematureEnd> for DecompressError {
    fn from(error: PrematureEnd) -> Self {
        DecompressError::PrematureEnd {
            context: Some(error.context_line),
        }
    }
}

fn checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    let mut odd: bool = false;

    // Last incomplete chunk of bytes (<4) is ignored
    for chunk in data.chunks_exact(4) {
        let element = u32::from_le_bytes(chunk.try_into().unwrap());

        if odd {
            sum = sum.wrapping_add(element);
        } else {
            sum ^= element;
        }
        odd = !odd;
    }
    sum
}

fn deobfuscate(input: &mut [u8]) -> Result<Vec<u8>, DecompressError> {
    let deobfuscated = process(input);
    let header = Header::from_bytes(&deobfuscated)?;
    if header.checksum_deobfuscated == checksum(&deobfuscated[HEADER_SIZE..]) {
        Ok(deobfuscated)
    } else {
        Err(DecompressError::DeobfuscateChecksumNotMatch)
    }
}

fn lz77_decompress(input: &[u8]) -> Result<Vec<u8>, DecompressError> {
    let header = Header::from_bytes(input)?;
    let buffer = lz_unpack(
        (&input[HEADER_SIZE..]).iter().cloned(),
        header.unpacked_size as usize,
    )?;

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
    let output = deobfuscate(input)?;
    let header = Header::from_bytes(&output)?;
    match header.compression {
        CompressionType::Uncompressed => Ok(output[HEADER_SIZE..].to_vec()),
        CompressionType::RLE => Err(DecompressError::CompressionNotSupported),
        CompressionType::LZ77 => lz77_decompress(&output),
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
    use crate::test_utils::*;
    #[test]
    #[ignore]
    fn test_decompress() {
        let decoded = read_decompressed(&test_file_path("Realms/Celtic/Forest/CFsec50.map"));
        assert!(decoded.is_ok(), "Decompress failed: {:?}", decoded);
        assert_eq!(
            6,
            u32::from_le_bytes(decoded.unwrap()[..4].try_into().unwrap())
        );
    }

    #[test]
    #[ignore]
    fn test_too_short() {
        let decoded = decompress(&mut vec![0; 10]);
        match decoded.unwrap_err() {
            DecompressError::ContentTooSmall => (),
            x => panic!("Invalid error {:?}", x),
        }
    }
}
