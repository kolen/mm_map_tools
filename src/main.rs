#[macro_use]
extern crate nom;
use nom::{le_u8, le_u32, le_i32};
use nom::IResult;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::str;

#[derive(Debug)]
struct SpriteFile {
    num_frames: u32,
    num_pallettes: u32,
    pallettes: Vec<Vec<RGB>>,
    frames: Vec<Frame>
}

#[derive(Debug)]
struct RGB {
    r: u8,
    g: u8,
    b: u8
}

#[derive(Debug)]
struct Frame {
    width: u32,
    height: u32,
    center_x: i32,
    center_y: i32,
    name: String,
    size: u32
}

fn frame(input: &[u8]) -> IResult<&[u8], Frame> {
    chain!(input,
           size: le_u32 ~
           width: le_u32 ~
           height: le_u32 ~
           center_x: le_i32 ~
           center_y: le_i32 ~
           name: take_str!(8) ,

    || {
        Frame {
            size: size,
            width: width,
            height: height,
            center_x: center_x,
            center_y: center_y,
            name: name.trim_right_matches('\0').to_string()
        }
    })
}


macro_rules! frames(
    ($i:expr, $offsets:expr) => ({
        let offsets: Vec<u32> = $offsets;
        let input: &[u8] = $i;
        let frames = offsets.iter()
            .map(|offset|
                 match frame(&input[*offset as usize ..]) {
                     IResult::Done(_, frame) => frame,
                     IResult::Error(e) => panic!("Error {:?}", e),
                     IResult::Incomplete(i) => panic!("Incomplete: {:?}", i)
                 }).collect::<Vec<Frame>>();
        IResult::Done($i, frames)
    })
);


named!(pallette<&[u8], Vec<RGB> >,
       count!(chain!(r: le_u8 ~
                     g: le_u8 ~
                     b: le_u8, || { RGB { r: r, g: g, b: b } } ),
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
           frames: frames!(frame_offsets) ,

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
    match parse() {
        Err(e) => println!("Error: {}", e),
        Ok(()) => {}
    }
}
