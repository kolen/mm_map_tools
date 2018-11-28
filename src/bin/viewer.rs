extern crate gdk_pixbuf;
extern crate gtk;
extern crate image;
extern crate mm_map_tools;
use gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::prelude::*;
use gtk::{Image, ScrolledWindow, Window, WindowType};
use mm_map_tools::decompress::read_decompressed;
use mm_map_tools::map_section::MapSection;
use mm_map_tools::render::render_map_section;
use mm_map_tools::sprite_file::SpriteFile;
use std::env;
use std::fs::File;
use std::path::Path;

fn render_map() -> image::RgbaImage {
    let map_section_path1 = env::var("MAP_SECTION").unwrap();
    let map_section_path = Path::new(&map_section_path1);
    let sprites_path = map_section_path
        .parent()
        .unwrap()
        .join(Path::new("Terrain.spr"));

    let map_section = MapSection::from_contents(read_decompressed(map_section_path).unwrap());
    let sprites = SpriteFile::parse(File::open(sprites_path).unwrap());

    render_map_section(&map_section, &sprites)
}

fn pixbuf() -> Pixbuf {
    let map_image = render_map();
    let width = map_image.width() as i32;
    let height = map_image.height() as i32;
    let raw = map_image.into_raw();
    Pixbuf::new_from_vec(raw, Colorspace::Rgb, true, 8, width, height, width * 4)
}

fn main() {
    gtk::init().unwrap();

    let window = Window::new(WindowType::Toplevel);
    window.set_title("Magic & Mayhem map section viewer");
    window.set_default_size(1024, 768);

    let pb = pixbuf();

    let scroller = ScrolledWindow::new(None, None);
    let image = Image::new_from_pixbuf(&pb);
    scroller.add(&image);
    window.add(&scroller);

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}
