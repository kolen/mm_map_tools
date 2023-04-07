use std::io::Read;

use bitstream_io::{BigEndian, BitRead, BitReader};

const WINDOW_SIZE: usize = 0x1000;

pub struct CompressedReader<R: Read> {
    bit_reader: BitReader<R, BigEndian>,
    window: [u8; WINDOW_SIZE],
    window_pointer: usize,
    output_pointer: usize,
    output_size: usize,
    bytes_outputted: usize,
}

impl<R: Read> CompressedReader<R> {
    fn flush_output_buffer(&mut self, out_buf: &mut [u8]) -> std::io::Result<usize> {
        debug_assert!(self.output_size > 0);
        let mut outputted_size: usize = 0;
        let mut out = out_buf.iter_mut();

        while self.output_size > 0 {
            if let Some(out_byte) = out.next() {
                let value = self.window[self.output_pointer];
                *out_byte = value;
                self.write_to_window(value);

                self.output_pointer += 1;
                self.output_pointer %= WINDOW_SIZE;
                self.output_size -= 1;
                self.bytes_outputted += 1;
                outputted_size += 1;
            } else {
                break;
            }
        }
        Ok(outputted_size)
    }

    fn write_to_window(&mut self, value: u8) {
        self.window[self.window_pointer] = value;
        self.window_pointer += 1;
        self.window_pointer %= WINDOW_SIZE;
    }
}

impl<R: Read> Read for CompressedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        if self.output_size > 0 {
            self.flush_output_buffer(buf)
        } else {
            match self.bit_reader.read_bit()? {
                true => {
                    self.bit_reader.read_bytes(&mut buf[0..1])?;
                    self.write_to_window(buf[0]);
                    self.bytes_outputted += 1;
                    Ok(1)
                }
                false => {
                    let offset: i32 = self.bit_reader.read(12)?;
                    let size: i32 = self.bit_reader.read(4)?;
                    self.output_pointer = offset as usize;
                    self.output_size = (size as usize) + 2;
                    self.flush_output_buffer(buf)
                }
            }
        }
    }
}

pub fn decompress<R>(source: R) -> CompressedReader<R>
where
    R: Read,
{
    let bit_reader = BitReader::new(source);
    CompressedReader {
        bit_reader,
        window: [0; WINDOW_SIZE],
        window_pointer: 1,
        output_pointer: 0,
        output_size: 0,
        bytes_outputted: 0,
    }
}
