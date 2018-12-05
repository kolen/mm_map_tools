extern crate gdk_pixbuf;
extern crate gtk;
extern crate image;
extern crate mm_map_tools;
use gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Builder, CellRendererText, FileChooserAction, FileChooserDialog, Image,
    ListStore, ResponseType, TreeView, Window,
};
use mm_map_tools::decompress::read_decompressed;
use mm_map_tools::map_section::MapSection;
use mm_map_tools::render::render_map_section;
use mm_map_tools::sprite_file::SpriteFile;
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};

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

fn create_map_section_list() -> ListStore {
    let store = ListStore::new(&[String::static_type()]);
    store.insert_with_values(None, &[0], &[&"Lol"]);
    store
}

fn create_main_window() -> ApplicationWindow {
    let glade_src = include_str!("viewer.glade");
    let builder = Builder::new();
    builder.add_from_string(glade_src).unwrap();

    let window: ApplicationWindow = builder.get_object("main_window").unwrap();
    let image: Image = builder.get_object("map_image").unwrap();
    let pixbuf = pixbuf();
    image.set_from_pixbuf(Some(&pixbuf));

    let map_section_selector: TreeView = builder.get_object("map_section_selector").unwrap();
    let section_store = create_map_section_list();
    map_section_selector.set_model(&section_store);

    let column = map_section_selector.get_column(0).unwrap();
    let cell_renderer = CellRendererText::new();
    column.pack_start(&cell_renderer, true);
    column.add_attribute(&cell_renderer, "text", 0);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    window
}

fn main() {
    gtk::init().unwrap();

    let dir_chooser = FileChooserDialog::with_buttons::<Window>(
        Some("Select Magic & Mayhem directory"),
        None,
        FileChooserAction::SelectFolder,
        &[("_Open", ResponseType::Accept)],
    );
    if true
    /* dir_chooser.run() == ResponseType::Accept.into() */
    {
        //let mm_path = dir_chooser.get_filename().unwrap();
        //dir_chooser.destroy();
        let window = create_main_window();
        window.show_all();
        gtk::main();
    }
}
