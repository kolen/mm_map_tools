use nom::{le_i32, le_u32, le_u8, rest};

use image::Pixel;
use image::{ImageBuffer, Rgb, Rgba, RgbaImage};
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::iter;

type Rgb8 = Rgb<u8>;
type Rgba8 = Rgba<u8>;
type Pallette = Vec<Rgb8>;

#[derive(Debug)]
pub struct SpriteFile {
    pub pallettes: Vec<Pallette>,
    pub frames: Vec<Frame>,
}

struct SpriteFileHeader {
    pallettes: Vec<Pallette>,
    frame_offsets: Vec<u32>,
}

pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub center_x: i32,
    pub center_y: i32,
    pub unknown1: u32,
    pub unknown2: u32,
    pub name: String,
    pub image: ImageBuffer<Rgba8, Vec<u8>>,
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Frame '{}' {}x{} {}, {}",
            self.name, self.width, self.height, self.unknown1, self.unknown2
        )
    }
}

fn indexed_to_rgba(pixel: Option<u8>, pallette: Pallette) -> Rgba8 {
    match pixel {
        Some(index) => pallette[index as usize].to_rgba(),
        None => Rgba8 { data: [0, 0, 0, 0] },
    }
}

impl SpriteFile {
    pub fn parse(mut file: File) -> SpriteFile {
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).expect("Can't read sprite file");

        let (payload, header) = header(&buf[..]).expect("Can't parse header");

        let frames = header
            .frame_offsets
            .iter()
            .filter_map(|&offset| {
                Some(
                    frame(&payload[offset as usize..], &header.pallettes)
                        .expect(&format!("Can't decode frame at {}", offset))
                        .1,
                )
            }).collect();

        SpriteFile {
            pallettes: header.pallettes,
            frames: frames,
        }
    }
}

struct IterPixelRow<'a> {
    runs: &'a [u8],
    pixels: &'a [u8],
    is_skip: bool,
    pixels_left: u8,
    pallette: &'a Pallette,
}

impl<'a> Iterator for IterPixelRow<'a> {
    type Item = Rgba8;
    fn next(&mut self) -> Option<Rgba8> {
        while self.pixels_left == 0 {
            self.is_skip = !self.is_skip;
            self.pixels_left = self.runs[0];
            self.runs = &self.runs[1..];
        }
        self.pixels_left -= 1;
        if self.is_skip {
            Some(Rgba8 { data: [0, 0, 0, 0] })
        } else {
            let pixel = self.pixels[0];
            self.pixels = &self.pixels[1..];
            Some(self.pallette[pixel as usize].to_rgba())
        }
    }
}

struct LineOffsets {
    runs_offset: u32,
    pixels_offset: u32,
}

fn pixels(
    input: &[u8],
    lines: Vec<LineOffsets>,
    width: u32,
    height: u32,
    pallette: &Pallette,
) -> RgbaImage {
    let mut bytes: Vec<u8> = Vec::with_capacity(width as usize * height as usize * 4);
    let iter_rgba = lines.into_iter().flat_map(|offsets| {
        IterPixelRow {
            runs: &input[offsets.runs_offset as usize..],
            pixels: &input[offsets.pixels_offset as usize..],
            is_skip: false,
            pixels_left: 0,
            pallette: pallette,
        }.chain(iter::repeat(Rgba8 { data: [0, 0, 0, 0] }))
        .take(width as usize)
    });
    for pixel in iter_rgba {
        bytes.extend_from_slice(&pixel.data)
    }

    debug_assert!(bytes.len() == (width as usize) * (height as usize) * 4);

    RgbaImage::from_raw(width, height, bytes).expect("Can't construct RgbaImage")
}

named_args!(
    frame<'a>(pallettes: &'a Vec<Pallette>) <&'a [u8], Frame>,
    do_parse!(content: peek!(rest) >>
              _size: le_u32 >>
              width: le_u32 >>
              height: le_u32 >>
              center_x: le_i32 >>
              center_y: le_i32 >>
              name: take_str!(8) >>
              pallette_index: le_u32 >>
              unknown1: le_u32 >>
              unknown2: le_u32 >>
              rows: count!(
                  do_parse!(r: le_u32 >>
                            p: le_u32 >>
                            (LineOffsets { runs_offset: r,
                                           pixels_offset: p })),
                  height as usize) >>
              (Frame {
                  width: width,
                  height: height,
                  center_x: center_x,
                  center_y: center_y,
                  unknown1: unknown1,
                  unknown2: unknown2,
                  name: name.trim_right_matches('\0').to_string(),
                  image: pixels(content, rows, width, height,
                                &pallettes[pallette_index as usize])
              })));

named!(pallette<&[u8], Vec<Rgb8> >,
       count!(do_parse!(r: le_u8 >>
                        g: le_u8 >>
                        b: le_u8 >>
                        (Rgb{data: [r, g, b]})),
              256));

named!(header<&[u8], SpriteFileHeader>,
    do_parse!(tag!("SPR\0") >>
              take!(4) >>
              take!(4) >>
              num_frames: le_u32 >>
              num_pallettes: le_u32 >>
              take!(4) >>
              pallettes: count!(pallette, num_pallettes as usize) >>
              frame_offsets: count!(le_u32, num_frames as usize) >>
              (SpriteFileHeader {
                  pallettes: pallettes,
                  frame_offsets: frame_offsets
              })
    )
);

#[cfg(test)]
mod tests {
    use super::*;
    use test_utils::*;

    #[test]
    fn test_load() {
        let f = File::open(test_file_path("Realms/Celtic/Forest/Terrain.spr")).unwrap();
        SpriteFile::parse(f);
    }
}
