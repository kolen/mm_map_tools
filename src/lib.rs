#[macro_use]
extern crate nom;
extern crate piston_window;
extern crate image;

use piston_window::{PistonWindow, WindowSettings, clear, OpenGL, Texture};

use nom::{le_u8, le_u32, le_i32};
use nom::IResult;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::str;
use std::fmt;
use std::iter;
use image::Pixel;
use image::{Rgba, Rgb, ImageBuffer, RgbaImage};

type Rgb8 = Rgb<u8>;
type Rgba8 = Rgba<u8>;
type Pallette = Vec<Rgb8>;

#[derive(Debug)]
struct SpriteFile {
    num_frames: u32,
    num_pallettes: u32,
    pallettes: Vec<Vec<Rgb8>>,
    frames: Vec<Frame>
}

struct Frame {
    width: u32,
    height: u32,
    center_x: i32,
    center_y: i32,
    name: String,
    size: u32,
    image: ImageBuffer<Rgba8, Vec<u8>>
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Frame {}", self.name)
    }
}

fn indexed_to_rgba(pixel: Option<u8>, pallette: Pallette) -> Rgba8 {
    match pixel {
        Some(index) => pallette[index as usize].to_rgba(),
        None => Rgba8 { data: [0, 0, 0, 0] }
    }
}

struct IterPixelRow<'a> {
    runs: &'a [u8],
    pixels: &'a [u8],
    is_skip: bool,
    pixels_left: u8,
    pallette: &'a Pallette,
}

impl <'a>Iterator for IterPixelRow<'a> {
    type Item = Rgba8;
    fn next(&mut self) -> Option<Rgba8> {
        if self.pixels_left == 0 {
            self.is_skip = !self.is_skip;
            self.pixels_left = self.runs[0];
            self.runs = &self.runs[1..];
        }
        self.pixels_left -= 1;
        if self.is_skip {
            Some(Rgba8 {data: [0, 0, 0, 0]})
        } else {
            Some(self.pallette[self.pixels[0] as usize].to_rgba())
        }
    }
}

fn pixels(input: &[u8], lines: &[(u32, u32)], width: u32, height: u32,
          pallette: &Pallette) -> RgbaImage {
    let mut bytes:Vec<u8> = Vec::with_capacity(
        width as usize * height as usize * 4);
    let iter_rgba =
        lines.into_iter().flat_map(|&(runs_offset, pixels_offset)| {
            IterPixelRow {
                runs: &input[runs_offset as usize ..],
                pixels: &input[pixels_offset as usize ..],
                is_skip: false,
                pixels_left: 0,
                pallette: pallette
            }
        });
    for pixel in iter_rgba {
        bytes.extend_from_slice(&pixel.data)
    }

    RgbaImage::from_raw(width, height, bytes).unwrap()
}

fn frame<'a>(input: &'a [u8], pallettes: &Vec<Pallette>)
             -> IResult<&'a [u8], Frame> {
    chain!(input,
           size: le_u32 ~
           width: le_u32 ~
           height: le_u32 ~
           center_x: le_i32 ~
           center_y: le_i32 ~
           name: take_str!(8) ~
           pallette_index: le_u32 ~
           le_u32 ~
           le_u32 ~
           rows: count!(tuple!(le_u32, le_u32), height as usize) ,

    || {
        Frame {
            size: size,
            width: width,
            height: height,
            center_x: center_x,
            center_y: center_y,
            name: name.trim_right_matches('\0').to_string(),
            image: pixels(input, &rows, width, height,
                          &pallettes[pallette_index as usize])
        }
    })
}

macro_rules! frames(
    ($i:expr, $offsets:expr, $pallettes:expr) => ({
        let offsets: Vec<u32> = $offsets;
        let input: &[u8] = $i;
        let frames = offsets.iter()
            .map(|offset|
                 match frame(&input[*offset as usize ..], $pallettes) {
                     IResult::Done(_, frame) => frame,
                     IResult::Error(e) => panic!("Error {:?}", e),
                     IResult::Incomplete(i) => panic!("Incomplete: {:?}", i)
                 }).collect::<Vec<Frame>>();
        IResult::Done($i, frames)
    })
);

named!(pallette<&[u8], Vec<Rgb8> >,
       count!(chain!(r: le_u8 ~
                     g: le_u8 ~
                     b: le_u8, || { Rgb{data: [r, g, b]} }),
              256));

fn header(input: &[u8]) -> IResult<&[u8], SpriteFile> {
    chain!(input,
           tag!("SPR\0") ~
           take!(4) ~
           take!(4) ~
           num_frames: le_u32 ~
           num_pallettes: le_u32 ~
           take!(4) ~

           pallettes: count!(pallette, num_pallettes as usize) ~
           frame_offsets: count!(le_u32, num_frames as usize) ~
           frames: frames!(frame_offsets, &pallettes) ,

           || {
               SpriteFile {
                   num_frames: num_frames,
                   num_pallettes: num_pallettes,
                   pallettes: pallettes,
                   frames: frames
               }
           }
    )
}

fn parse() -> io::Result<()> {
    let mut f = try!(File::open("/Volumes/data/games/Magic and Mayhem/Realms/Greek/Labyrinth/Terrain.spr"));
    let mut buf: Vec<u8> = Vec::new();
    let read = try!(f.read_to_end(&mut buf));

    match header(&buf[..]) {
        IResult::Done(_, spritefile) => {
            println!("Frames: {}, pallettes: {}, {:?}", spritefile.num_frames, spritefile.num_pallettes, spritefile);
        }
        IResult::Error(e)      => panic!("Error: {:?}", e),
        IResult::Incomplete(i) => panic!("Incomplete: {:?}", i),
    };

    Ok(())
}

fn main() {
    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow =
        WindowSettings::new("piston: image", [300, 300])
        .exit_on_esc(true)
        .opengl(opengl)
        .build()
        .unwrap();

    // let assets = find_folder::Search::ParentsThenKids(3, 3)
    //     .for_folder("assets").unwrap();
    // let rust_logo = assets.join("rust.png");
    // let rust_logo = Texture::from_path(
    //     &mut window.factory,
    //     &rust_logo,
    //     Flip::None,
    //     &TextureSettings::new()
    // ).unwrap();
    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g| {
            clear([1.0; 4], g);
            // image(&rust_logo, c.transform, g);
        });
    }

    match parse() {
        Err(e) => println!("Error: {}", e),
        Ok(()) => {}
    }
}
