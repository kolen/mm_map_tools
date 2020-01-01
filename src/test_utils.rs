use crate::container::read_decompressed;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

pub fn test_file_path(filename: &str) -> PathBuf {
    let prefix = env::var("MM_PATH").expect("MM_PATH must be specified to run tests");
    Path::new(&prefix).join(Path::new(filename))
}

#[allow(dead_code)]
pub fn test_file_contents(filename: &str) -> Vec<u8> {
    let mut buffer = Vec::new();
    let mut file = File::open(&test_file_path(filename)).unwrap();
    file.read_to_end(&mut buffer).unwrap();
    buffer
}

#[allow(dead_code)]
pub fn test_file_compressed_contents(filename: &str) -> Vec<u8> {
    read_decompressed(&test_file_path(filename)).unwrap()
}
