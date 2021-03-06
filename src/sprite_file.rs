use image::Pixel;
use image::{ImageBuffer, Rgb, Rgba, RgbaImage};
use nom::{
    bytes::complete::{tag, take},
    combinator::{map, map_res},
    multi::count,
    number::complete::{le_i32, le_u32, le_u8},
    sequence::tuple,
    IResult,
};
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::iter;
use std::str::from_utf8;

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

impl SpriteFile {
    pub fn parse(mut file: File) -> SpriteFile {
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).expect("Can't read sprite file");

        let (payload, header) = header(&buf[..]).expect("Can't parse header");

        let frames = header
            .frame_offsets
            .iter()
            .map(|&offset| {
                frame(&header.pallettes)(&payload[offset as usize..])
                    .unwrap_or_else(|_| panic!("Can't decode frame at {}", offset))
                    .1
            })
            .collect();

        SpriteFile {
            pallettes: header.pallettes,
            frames,
        }
    }
}

struct IterPixelRow<'a> {
    runs: &'a [u8],
    pixels: &'a [u8],
    is_skip: bool,
    pixels_left: u8,
    pallette: &'a [Rgb8],
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
            Some(Rgba([0, 0, 0, 0]))
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
    lines: impl IntoIterator<Item = LineOffsets>,
    width: u32,
    height: u32,
    pallette: &[Rgb8],
) -> RgbaImage {
    let mut image = ImageBuffer::new(width, height);

    // rows_mut crashes on zero-width images:
    // https://github.com/image-rs/image/issues/994
    if width == 0 || height == 0 {
        return image;
    }

    for (offsets, output_row) in lines.into_iter().zip(image.rows_mut()) {
        let input_pixels = IterPixelRow {
            runs: &input[offsets.runs_offset as usize..],
            pixels: &input[offsets.pixels_offset as usize..],
            is_skip: false,
            pixels_left: 0,
            pallette,
        }
        .chain(iter::repeat(Rgba([0, 0, 0, 0])))
        .take(width as usize);

        for (input_pixel, output_pixel) in input_pixels.zip(output_row) {
            *output_pixel = input_pixel;
        }
    }

    image
}

fn frame(pallettes: &[Pallette]) -> impl Fn(&[u8]) -> IResult<&[u8], Frame> + '_ {
    move |i: &[u8]| {
        let (input, (_size, width, height, center_x, center_y)) =
            tuple((le_u32, le_u32, le_u32, le_i32, le_i32))(i)?;
        let (input, name) = map(map_res(take(8usize), from_utf8), String::from)(input)?;
        let (input, pallette_index) = le_u32(input)?;
        let (input, (unknown1, unknown2)) = tuple((le_u32, le_u32))(input)?;
        let (input, rows) = count(
            map(tuple((le_u32, le_u32)), |(runs_offset, pixels_offset)| {
                LineOffsets {
                    runs_offset,
                    pixels_offset,
                }
            }),
            height as usize,
        )(input)?;
        let image = pixels(i, rows, width, height, &pallettes[pallette_index as usize]);

        Ok((
            input,
            Frame {
                width,
                height,
                center_x,
                center_y,
                unknown1,
                unknown2,
                name,
                image,
            },
        ))
    }
}

fn pallette(i: &[u8]) -> IResult<&[u8], Pallette> {
    count(
        map(tuple((le_u8, le_u8, le_u8)), |(r, g, b)| Rgb([r, g, b])),
        256usize,
    )(i)
}

fn header(input: &[u8]) -> IResult<&[u8], SpriteFileHeader> {
    let (input, (_, _, _, num_frames, num_pallettes, _)) =
        tuple((tag("SPR\0"), le_u32, le_u32, le_u32, le_u32, le_u32))(input)?;
    let (input, pallettes) = count(pallette, num_pallettes as usize)(input)?;
    let (input, frame_offsets) = count(le_u32, num_frames as usize)(input)?;

    Ok((
        input,
        SpriteFileHeader {
            pallettes,
            frame_offsets,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[test]
    #[ignore]
    fn test_load() {
        let f = File::open(test_file_path("Realms/Celtic/Forest/Terrain.spr")).unwrap();
        SpriteFile::parse(f);
    }
}
