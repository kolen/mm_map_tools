use base64::Engine;
use mm_file_formats::sprites::{Frame, Sprites};
use std::env;
use std::fs::File;
use std::io::{self, stdout, Cursor, Write};

fn write_frame_img<W: Write>(out: &mut W, frame: &Frame) -> io::Result<()> {
    if frame.image.width() == 0 || frame.image.height() == 0 {
        return Ok(());
    }

    let mut png: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(1024 * 64));
    frame
        .image
        .write_to(&mut png, image::ImageOutputFormat::Png)
        .expect("Write png file");
    let base64_png = base64::engine::general_purpose::STANDARD_NO_PAD.encode(png.get_ref());
    let image_url = format!("data:image/png;base64,{base64_png}");
    let width = frame.width;
    let height = frame.height;

    writeln!(
        out,
        "<img src=\"{image_url}\" width=\"{width}\" height=\"{height}\" />"
    )?;
    Ok(())
}

fn write_frame<W: Write>(out: &mut W, frame: &Frame, index: i32) -> io::Result<()> {
    let name = &frame.name;
    writeln!(out, "<li>")?;
    write_frame_img(out, frame)?;
    writeln!(out, "<div class=\"sprite-number\">{index}</div>")?;
    writeln!(out, "<div class=\"sprite-name\">{name}</div>")?;
    writeln!(out, "</li>")?;
    Ok(())
}

fn main() -> io::Result<()> {
    let filename = env::args().nth(1).expect("input file argument required");
    let sprites = Sprites::parse(File::open(filename).expect("open sprite file"));

    let mut out = stdout();

    writeln!(&out, "<!DOCTYPE html>")?;
    writeln!(&out, "<html>")?;
    writeln!(&out, "<head>")?;
    writeln!(&out, "<style>\n{}\n</style>", include_str!("style.css"))?;
    writeln!(&out, "</head>")?;
    writeln!(&out, "<body>")?;

    writeln!(&out, "<ul class=\"sprites\">")?;

    let mut i = 0;
    for frame in sprites.frames {
        write_frame(&mut out, &frame, i)?;
        i += 1;
    }

    writeln!(&out, "</ul>")?;

    writeln!(&out, "</body>")?;
    writeln!(&out, "</html>")?;

    Ok(())
}
