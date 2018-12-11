extern crate gdk_pixbuf;
extern crate glib;
extern crate gtk;
extern crate image;
extern crate mm_map_tools;
use gdk_pixbuf::{Colorspace, Pixbuf};
use gtk::prelude::*;
use gtk::{
    ApplicationWindow, Builder, CellRendererText, ComboBox, FileChooserAction, FileChooserDialog,
    Image, ListStore, ResponseType, Spinner, TreeView, Window,
};
use mm_map_tools::render::utils::Renderer;
use mm_map_tools::render::RenderOptions;
use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn image_to_pixbuf(image: image::RgbaImage) -> Pixbuf {
    let width = image.width() as i32;
    let height = image.height() as i32;
    let raw = image.into_raw();
    Pixbuf::new_from_vec(raw, Colorspace::Rgb, true, 8, width, height, width * 4)
}

fn debounced(timeout: u32, action: impl Fn() + 'static) -> impl Fn() + 'static {
    let action_rc = Rc::new(action);
    let last_invokation_id: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
    let last_invokation_id_1 = last_invokation_id.clone();
    move || {
        *last_invokation_id_1.borrow_mut() += 1;
        let current_invokation: u32 = *last_invokation_id_1.borrow();

        let action_rc_1 = action_rc.clone();
        let last_invokation_id_2 = last_invokation_id_1.clone();
        gtk::timeout_add(timeout, move || {
            if *last_invokation_id_2.borrow() == current_invokation {
                action_rc_1();
            }
            gtk::Continue(false)
        });
    }
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

fn update_map_display(
    window: ApplicationWindow,
    image_view: gtk::Image,
    map_rendering_spinner: gtk::Spinner,
    renderer: Arc<Renderer>,
    map_group: &str,
    map_section: &str,
    max_layer: u32,
) {
    let (images_channel_tx, images_channel_rx) =
        mpsc::channel::<Result<image::RgbaImage, String>>();

    gtk::timeout_add(100, move || {
        let mut has_images = false;
        map_rendering_spinner.start();

        while let Ok(image_result) = images_channel_rx.try_recv() {
            match image_result {
                Ok(image) => {
                    let pixbuf = image_to_pixbuf(image);
                    image_view.set_from_pixbuf(Some(&pixbuf));
                    map_rendering_spinner.stop();
                }
                Err(error_message) => {
                    let msg_box = gtk::MessageDialog::new(
                        Some(&window),
                        gtk::DialogFlags::MODAL,
                        gtk::MessageType::Error,
                        gtk::ButtonsType::Ok,
                        &error_message,
                    );
                    msg_box.run();
                    msg_box.destroy();
                    map_rendering_spinner.stop();
                }
            }
            has_images = true
        }
        Continue(!has_images)
    });

    let (map_group_1, map_section_1) = (map_group.to_owned(), map_section.to_owned());
    thread::spawn(move || {
        // Errors itself don't implement Send, so we'll send strings
        let time = SystemTime::now();
        let render_options = RenderOptions {
            max_layer: max_layer,
        };
        let map_image = renderer
            .render(&map_group_1, &map_section_1, &render_options)
            .map_err(|e| format!("Error loading map section:\n{}", e));
        eprintln!("Rendering took {:?}", time.elapsed().unwrap());
        images_channel_tx.send(map_image).unwrap();
    });
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

    let max_layer_adjustment: gtk::Adjustment = builder.get_object("max_layer").unwrap();

    let current_group = Rc::new(RefCell::new("Celtic/Forest".to_string()));
    let current_section = Rc::new(RefCell::new("CFsec01".to_string()));
    let current_max_layer = Rc::new(RefCell::new(max_layer_adjustment.get_value() as u32));

    let renderer = Arc::new(Renderer::new(mm_path));

    let mm_path_buf = mm_path.to_path_buf();
    map_group_selector.connect_changed(
        clone!(current_group, map_section_selector => move |map_group_selector| {
            let iter = map_group_selector.get_active_iter().unwrap();
            let group_segment = map_group_store.get_value(&iter, 0).get::<String>().unwrap();
            let section_store = create_map_section_list(&mm_path_buf, &group_segment);
            map_section_selector.set_model(&section_store);
            current_group.replace(group_segment.to_string());
        }),
    );

    let map_rendering_spinner: Spinner = builder.get_object("map_rendering_spinner").unwrap();

    map_section_selector.connect_cursor_changed(
        clone!(window, image, map_rendering_spinner, renderer, current_group, current_section, current_max_layer => move |map_section_selector| {
            let selection = map_section_selector.get_selection();
            if let Some((model, iter)) = selection.get_selected() {
                let section_segment = model.get_value(&iter, 0).get::<String>().unwrap();
                current_section.replace(section_segment.to_string());

                update_map_display(
                    window.clone(),
                    image.clone(),
                    map_rendering_spinner.clone(),
                    renderer.clone(),
                    &current_group.borrow().clone(),
                    &current_section.borrow().clone(),
                    current_max_layer.borrow().clone(),
                );
            }
        }),
    );

    let update_map_display_on_max_level = debounced(500, {
        let window = window.clone();
        let image = image.clone();
        let map_rendering_spinner = map_rendering_spinner.clone();
        let renderer = renderer.clone();
        let current_group = current_group.clone();
        let current_section = current_section.clone();
        let current_max_layer = current_max_layer.clone();
        let max_layer_adjustment = max_layer_adjustment.clone();

        move || {
            let max_layer = max_layer_adjustment.get_value() as u32;

            eprintln!("Max layer: {}", max_layer);
            current_max_layer.clone().replace(max_layer);

            update_map_display(
                window.clone(),
                image.clone(),
                map_rendering_spinner.clone(),
                renderer.clone(),
                &current_group.borrow().clone(),
                &current_section.borrow().clone(),
                max_layer,
            );
        }
    });

    max_layer_adjustment.connect_value_changed(move |_adj| {
        update_map_display_on_max_level();
    });

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    window
}

fn run_window(mm_path: &Path) {
    let window = create_main_window(mm_path);
    window.show_all();
    gtk::main();
}

fn main() {
    gtk::init().unwrap();

    let dir_chooser = FileChooserDialog::with_buttons::<Window>(
        Some("Select Magic & Mayhem directory"),
        None,
        FileChooserAction::SelectFolder,
        &[("_Open", ResponseType::Accept)],
    );

    let mm_path = env::var("MM_PATH")
        .ok()
        .map(|path_s| Path::new(&path_s).to_path_buf())
        .or_else(|| {
            if dir_chooser.run() == ResponseType::Accept.into() {
                Some(
                    dir_chooser
                        .get_filename()
                        .expect("Can't get filename from dir chooser"),
                )
            } else {
                None
            }
        });
    if let Some(mm_path_1) = mm_path {
        run_window(&mm_path_1);
    }
}
