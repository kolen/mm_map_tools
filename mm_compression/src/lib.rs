//! Support for compression and obfuscation formats used in Magic &
//! Mayhem
//!
//! Some of Magic & Mayhem files are compressed and obfuscated (seems
//! that compression and obfuscation are always used together,
//! i.e. there's no just compressed but not obfuscated files and vice
//! versa). Files are, obviously, first compressed and then
//! obfuscated.

mod compression;
mod obfuscation;
pub mod test_utils;

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
    LZSS = 2,
    Unknown,
}

#[derive(Debug)]
struct Header {
    unpacked_size: u32,
    checksum_deobfuscated: u32,
    checksum_uncompressed: u32,
    compression: CompressionType,
}

const HEADER_SIZE: usize = 4 * 4;

impl Header {
    pub fn from_bytes(input: &[u8]) -> Result<Header, DecompressError> {
        Ok(Header {
            unpacked_size: u32::from_le_bytes(input[0..0x4].try_into()?),
            checksum_deobfuscated: u32::from_le_bytes(input[0x4..0x8].try_into()?),
            checksum_uncompressed: u32::from_le_bytes(input[0x8..0xc].try_into()?),
            compression: match u32::from_le_bytes(input[0xc..0x10].try_into()?) {
                x if x == CompressionType::Uncompressed as u32 => CompressionType::Uncompressed,
                x if x == CompressionType::RLE as u32 => CompressionType::RLE,
                x if x == CompressionType::LZSS as u32 => CompressionType::LZSS,
                _ => CompressionType::Unknown,
            },
        })
    }
}

#[derive(Debug)]
pub enum DecompressError {
    DeobfuscateChecksumNotMatch,
    ObfuscateFileTooSmall,
    DecompressChecksumNonMatch,
    InvalidCompressionType,
    CompressionNotSupported,
    ContentTooSmall,
    FileError { error: std::io::Error },
    PrematureEnd { context: Option<u32> },
}

impl fmt::Display for DecompressError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecompressError::DeobfuscateChecksumNotMatch => {
                write!(f, "deobfuscation checksum does not match")
            }
            DecompressError::DecompressChecksumNonMatch => {
                write!(f, "decompression checksum does not match")
            }
            DecompressError::ObfuscateFileTooSmall => {
                write!(f, "file too small for obfuscation/deobfuscation")
            }
            DecompressError::InvalidCompressionType => write!(f, "invalid compression type"),
            DecompressError::CompressionNotSupported => write!(f, "compression not supported"),
            DecompressError::ContentTooSmall => write!(f, "file contents are too small"),
            DecompressError::FileError { error: e } => write!(f, "file reading error: {}", e),
            DecompressError::PrematureEnd { context: None } => write!(f, "premature end of file"),
            DecompressError::PrematureEnd {
                context: Some(line),
            } => write!(
                f,
                "premature end of file when lz unpacking, lz.unpack.rs line {}",
                line
            ),
        }
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

impl From<obfuscation::InputTooSmall> for DecompressError {
    fn from(_error: obfuscation::InputTooSmall) -> Self {
        DecompressError::ObfuscateFileTooSmall
    }
}

struct ChecksummingReader<R: Read> {
    reader: R,
    pub checksum: u32,
    odd: bool,
    current_word: u32,
    current_word_fill: u32,
}

impl<R: Read> Read for ChecksummingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let result = self.reader.read(buf);
        if let Ok(bytes_read) = result {
            for &byte in &buf[0..bytes_read] {
                self.current_word |= (byte as u32) << (8 * self.current_word_fill);
                self.current_word_fill += 1;
                if self.current_word_fill >= 4 {
                    if self.odd {
                        self.checksum = self.checksum.wrapping_add(self.current_word);
                    } else {
                        self.checksum ^= self.current_word;
                    }
                    self.current_word = 0;
                    self.current_word_fill = 0;
                    self.odd = !self.odd;
                }
            }
        }
        result
    }
}

/// Wraps reader with a reader that calculates checksum
fn checksummed<R: Read>(reader: R) -> ChecksummingReader<R> {
    ChecksummingReader {
        reader,
        checksum: 0,
        odd: false,
        current_word: 0,
        current_word_fill: 0,
    }
}

#[deprecated]
fn checksum<R: Read>(reader: R) -> u32 {
    let mut checksummer = checksummed(reader);
    std::io::copy(&mut checksummer, &mut std::io::sink()).unwrap();
    checksummer.checksum
}

fn deobfuscate(input: &mut [u8]) -> Result<Vec<u8>, DecompressError> {
    let deobfuscated = obfuscation::deobfuscate(input)?;
    let header = Header::from_bytes(&deobfuscated)?;
    if header.checksum_deobfuscated == checksum(&deobfuscated[HEADER_SIZE..]) {
        Ok(deobfuscated)
    } else {
        Err(DecompressError::DeobfuscateChecksumNotMatch)
    }
}

fn lzss_decompress(input: &[u8]) -> Result<Vec<u8>, DecompressError> {
    let header = Header::from_bytes(input)?;

    let mut buffer = Vec::with_capacity(header.unpacked_size as usize);
    let mut compressed_reader = checksummed(
        compression::decompress(&input[HEADER_SIZE..]).take(header.unpacked_size as u64),
    );
    compressed_reader.read_to_end(&mut buffer)?;

    if header.checksum_uncompressed == compressed_reader.checksum {
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
        CompressionType::LZSS => lzss_decompress(&output),
        _ => Err(DecompressError::CompressionNotSupported),
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
