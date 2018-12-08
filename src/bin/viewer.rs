extern crate gdk_pixbuf;
extern crate glib;
extern crate gtk;
extern crate image;
extern crate mm_map_tools;
use gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Builder, CellRendererText, ComboBox, FileChooserAction, FileChooserDialog,
    Image, ListStore, ResponseType, TreeView, Window,
};
use mm_map_tools::decompress::read_decompressed;
use mm_map_tools::map_section::MapSection;
use mm_map_tools::render::render_map_section;
use mm_map_tools::sprite_file::SpriteFile;
use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

fn map_section_path(mm_path: &Path, map_group: &str, map_section: &str) -> PathBuf {
    mm_path
        .join("Realms")
        .join(&map_group)
        .join(&map_section)
        .with_extension("map")
}

fn render_map(mm_path: &Path, map_group: &str, map_section: &str) -> image::RgbaImage {
    let map_section_path_1 = map_section_path(&mm_path, &map_group, &map_section);
    println!("{:?}", &map_section_path_1);
    let sprites_path = map_section_path_1
        .parent()
        .unwrap()
        .join(Path::new("Terrain.spr"));

    let map_section = MapSection::from_contents(read_decompressed(map_section_path_1).unwrap());
    let sprites = SpriteFile::parse(File::open(sprites_path).unwrap());

    render_map_section(&map_section, &sprites)
}

fn image_to_pixbuf(image: image::RgbaImage) -> Pixbuf {
    let width = image.width() as i32;
    let height = image.height() as i32;
    let raw = image.into_raw();
    Pixbuf::new_from_vec(raw, Colorspace::Rgb, true, 8, width, height, width * 4)
}

fn create_map_section_list(mm_path: &Path, map_group: &str) -> ListStore {
    // TODO: error handling
    let map_section_dir = mm_path.join("Realms").join(map_group);
    let store = ListStore::new(&[String::static_type()]);
    for entry in map_section_dir.read_dir().unwrap() {
        let entry_path = entry.unwrap().path();
        if entry_path.extension() == Some(OsStr::new("map")) {
            let name = entry_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            store.insert_with_values(None, &[0], &[&name]);
        }
    }
    store
}

fn create_map_group_list(mm_path: &Path) -> ListStore {
    // TODO: error handling
    let store = ListStore::new(&[String::static_type()]);
    let map_groups_dir = mm_path.join("Realms");
    for realm_entry in map_groups_dir.read_dir().unwrap() {
        if !realm_entry.as_ref().unwrap().file_type().unwrap().is_dir() {
            continue;
        }
        for subrealm_entry in realm_entry.unwrap().path().read_dir().unwrap() {
            if !subrealm_entry
                .as_ref()
                .unwrap()
                .file_type()
                .unwrap()
                .is_dir()
            {
                continue;
            }
            let name: String = subrealm_entry
                .unwrap()
                .path()
                .strip_prefix(&map_groups_dir)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            store.insert_with_values(None, &[0], &[&name]);
        }
    }
    store
}

fn map_group_selector_init(map_group_selector: &ComboBox) {
    let cell_renderer_map_group = CellRendererText::new();
    map_group_selector.pack_start(&cell_renderer_map_group, true);
    map_group_selector.add_attribute(&cell_renderer_map_group, "text", 0);
}

fn map_section_selector_init(map_section_selector: &TreeView) {
    let column = map_section_selector.get_column(0).unwrap();
    let cell_renderer = CellRendererText::new();
    column.pack_start(&cell_renderer, true);
    column.add_attribute(&cell_renderer, "text", 0);
}

fn create_main_window(mm_path: &Path) -> ApplicationWindow {
    let glade_src = include_str!("viewer.glade");
    let builder = Builder::new();
    builder.add_from_string(glade_src).unwrap();

    let window: ApplicationWindow = builder.get_object("main_window").unwrap();
    let image: Image = builder.get_object("map_image").unwrap();

    let map_group_selector: ComboBox = builder.get_object("map_group_selector").unwrap();
    map_group_selector_init(&map_group_selector);
    let map_group_store = create_map_group_list(&mm_path);
    map_group_selector.set_model(&map_group_store);

    let map_section_selector: TreeView = builder.get_object("map_section_selector").unwrap();
    map_section_selector_init(&map_section_selector);
    let section_store = create_map_section_list(&mm_path, "Celtic/Forest");
    map_section_selector.set_model(&section_store);

    let current_group = Rc::new(RefCell::new("Celtic/Forest".to_string()));
    let current_section = Rc::new(RefCell::new("CFsec01".to_string()));

    let map_image = render_map(mm_path, &current_group.borrow(), &current_section.borrow());
    let pixbuf = image_to_pixbuf(map_image);
    image.set_from_pixbuf(Some(&pixbuf));

    // TODO: find ways to manage these nasty GObject clones to use in closures
    let mm_path_2 = mm_path.to_path_buf();
    let mm_path_3 = mm_path.to_path_buf();
    let map_section_selector_2 = map_section_selector.clone();
    let current_group_2 = current_group.clone();
    map_group_selector.connect_changed(move |map_group_selector| {
        let iter = map_group_selector.get_active_iter().unwrap();
        let group_segment = map_group_store.get_value(&iter, 0).get::<String>().unwrap();
        let section_store = create_map_section_list(&mm_path_2, &group_segment);
        map_section_selector_2.set_model(&section_store);
        current_group.replace(group_segment.to_string());
    });

    map_section_selector.connect_cursor_changed(move |map_section_selector| {
        let selection = map_section_selector.get_selection();
        if let Some((model, iter)) = selection.get_selected() {
            let section_segment = model.get_value(&iter, 0).get::<String>().unwrap();
            current_section.replace(section_segment.to_string());

            let mm_path_4 = mm_path_3.clone();

            let (images_channel_tx, images_channel_rx) = mpsc::channel();
            let image_2 = image.clone();

            gtk::timeout_add(100, move || {
                let mut has_images = false;
                while let Ok(image) = images_channel_rx.try_recv() {
                    let pixbuf = image_to_pixbuf(image);
                    image_2.set_from_pixbuf(Some(&pixbuf));
                    has_images = true
                }
                Continue(!has_images)
            });

            let group_copy = current_group_2.borrow().clone();
            let section_copy = section_segment.clone();

            thread::spawn(move || {
                let map_image = render_map(&mm_path_4, &group_copy, &section_copy);
                images_channel_tx.send(map_image).unwrap();
            });
        }
    });

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
        let mm_path_str = env::var("MM_PATH").unwrap();
        let mm_path = Path::new(&mm_path_str);
        //dir_chooser.destroy();
        let window = create_main_window(mm_path);
        window.show_all();
        gtk::main();
    }
}
