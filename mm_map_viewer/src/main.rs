use gdk_pixbuf::{Colorspace, Pixbuf};
use glib::MainContext;
use gtk::{
    prelude::*, Application, ButtonsType, FileChooser, FileChooserAction, FileChooserDialog, Label,
    ListBox, MessageDialog, MessageType, Window,
};
use gtk::{
    ApplicationWindow, Builder, CellRendererText, ComboBox, Image, ListStore, ResponseType, Spinner,
};
use gtk4 as gtk;
use mm_map_rendering::{utils::Renderer, RenderOptions};
use std::cell::RefCell;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

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
    let bytes = glib::Bytes::from_owned(raw);
    Pixbuf::from_bytes(&bytes, Colorspace::Rgb, true, 8, width, height, width * 4)
}

fn debounced(timeout: Duration, action: impl Fn() + 'static) -> impl Fn() + 'static {
    let action_rc = Rc::new(action);
    let last_invokation_id: Rc<RefCell<u32>> = Rc::new(RefCell::new(0));
    move || {
        *last_invokation_id.borrow_mut() += 1;
        let current_invokation: u32 = *last_invokation_id.borrow();

        glib::timeout_add_local_once(
            timeout,
            clone!(action_rc, last_invokation_id => move || {
                if *last_invokation_id.borrow() == current_invokation {
                    action_rc();
                }
            }),
        );
    }
}

fn fill_map_section_list(
    map_section_list: ListBox,
    mm_path: &Path,
    map_group: &str,
) -> std::io::Result<()> {
    let map_section_dir = mm_path.join("Realms").join(map_group);

    while let Some(row) = map_section_list.row_at_index(0) {
        map_section_list.remove(&row);
    }

    for entry in map_section_dir.read_dir()? {
        let entry_path = entry?.path();
        if entry_path.extension() == Some(OsStr::new("map")) {
            if let Some(name) = entry_path.file_stem() {
                let label = Label::new(Some(&name.to_string_lossy()));
                map_section_list.append(&label);
            }
        }
    }

    Ok(())
}

fn create_map_group_list(mm_path: &Path) -> std::io::Result<ListStore> {
    let store = ListStore::new(&[String::static_type()]);
    let map_groups_dir = mm_path.join("Realms");
    for realm_entry in map_groups_dir.read_dir()? {
        let realm_entry_ = realm_entry?;
        if !realm_entry_.file_type()?.is_dir() {
            continue;
        }
        for subrealm_entry in realm_entry_.path().read_dir().unwrap() {
            let subrealm_entry_ = subrealm_entry?;
            if !subrealm_entry_.file_type()?.is_dir() {
                continue;
            }
            let name = subrealm_entry_
                .path()
                .strip_prefix(&map_groups_dir)
                .unwrap() // Should always have that prefix
                .to_string_lossy()
                .to_string();
            store.insert_with_values(None, &[(0, &name)]);
        }
    }
    Ok(store)
}

fn map_group_selector_init(map_group_selector: &ComboBox) {
    let cell_renderer_map_group = CellRendererText::new();
    map_group_selector.pack_start(&cell_renderer_map_group, true);
    map_group_selector.add_attribute(&cell_renderer_map_group, "text", 0);
}

fn update_map_display(
    application: Application,
    window: Window,
    image_view: gtk::Image,
    map_rendering_spinner: gtk::Spinner,
    renderer: Arc<Renderer>,
    map_group: &str,
    map_section: &str,
    max_layer: u32,
) {
    let (images_sender, images_receiver) =
        MainContext::channel::<Result<image::RgbaImage, String>>(Default::default());

    images_receiver.attach(
        None,
        clone!(map_rendering_spinner, image_view => move |image_result| {
            map_rendering_spinner.start();

            match image_result {
                Ok(image) => {
                    let pixbuf = image_to_pixbuf(image);
                    image_view.set_from_pixbuf(Some(&pixbuf));
                    map_rendering_spinner.stop();
                }
                Err(error_message) => {
                    // FIXME: clos button does not close the dialog
                    let msg_box = MessageDialog::builder()
                        .text(&error_message)
                        .buttons(ButtonsType::Close)
                        .message_type(MessageType::Error)
                        .modal(true)
                        .transient_for(&window)
                        .application(&application)
                        .build();
                    msg_box.present();
                    map_rendering_spinner.stop();
                }
            }
            Continue(true)
        }),
    );

    let (map_group_1, map_section_1) = (map_group.to_owned(), map_section.to_owned());
    thread::Builder::new()
        .name("map render".into())
        .spawn(move || {
            // Errors itself don't implement Send, so we'll send strings
            let time = Instant::now();
            let render_options = RenderOptions { max_layer };
            let map_image = renderer
                .render(&map_group_1, &map_section_1, &render_options)
                .map_err(|e| format!("Error loading map section:\n{}", e));
            eprintln!("Rendering took {:?}", time.elapsed());
            images_sender.send(map_image).expect("send rendered image");
        })
        .expect("create map rendering thread");
}

fn create_main_window(mm_path: &Path, application: Application) -> ApplicationWindow {
    let glade_src = include_str!("viewer.glade");
    let builder = Builder::new();
    builder.add_from_string(glade_src).unwrap();

    let window: ApplicationWindow = builder.object("main_window").unwrap();
    window.set_application(Some(&application));
    let image: Image = builder.object("map_image").unwrap();

    let map_group_selector: ComboBox = builder.object("map_group_selector").unwrap();
    map_group_selector_init(&map_group_selector);
    // FIXME: unwrap, should handle failure for initial load
    let map_group_store = create_map_group_list(&mm_path).expect("Map group list failed");
    map_group_selector.set_model(Some(&map_group_store));

    let map_section_selector: ListBox = builder.object("map_section_selector").unwrap();

    let max_layer_adjustment: gtk::Adjustment = builder.object("max_layer").unwrap();

    let current_group = Rc::new(RefCell::new("Celtic/Forest".to_string()));
    let current_section = Rc::new(RefCell::new("CFsec01".to_string()));
    let current_max_layer = Rc::new(RefCell::new(max_layer_adjustment.value() as u32));

    let renderer = Arc::new(Renderer::new(mm_path));

    let mm_path_buf = mm_path.to_path_buf();
    map_group_selector.connect_changed(
        clone!(current_group, map_section_selector => move |map_group_selector| {
            let iter = map_group_selector.active_iter().unwrap();
            let group_segment = map_group_store.get_value(&iter, 0).get::<String>().unwrap();
            fill_map_section_list(map_section_selector.clone(), &mm_path_buf, &group_segment).expect("fill map section list"); // TODO: handle error
            current_group.replace(group_segment);
        }),
    );

    let map_rendering_spinner: Spinner = builder.object("map_rendering_spinner").unwrap();

    map_section_selector.connect_row_selected(
        clone!(application, window, image, map_rendering_spinner, renderer, current_group, current_section, current_max_layer => move |_selector, opt_row| {
            if let Some(row) = opt_row {
                let section_segment = row.child().expect("get ListBoxRow child").downcast::<Label>().expect("downcast to Label").label().to_string();

                current_section.replace(section_segment);

                update_map_display(
                    application.clone(),
                    window.clone().upcast(),
                    image.clone(),
                    map_rendering_spinner.clone(),
                    renderer.clone(),
                    &current_group.borrow().clone(),
                    &current_section.borrow().clone(),
                    *current_max_layer.borrow(),
                );
            };
        })
    );

    let update_map_display_on_max_level = debounced(Duration::from_millis(500), {
        let max_layer_adjustment = max_layer_adjustment.clone();

        clone!(window => move || {
            let max_layer = max_layer_adjustment.value() as u32;

            eprintln!("Max layer: {}", max_layer);
            current_max_layer.clone().replace(max_layer);

            update_map_display(
                application.clone(),
                window.clone().upcast(),
                image.clone(),
                map_rendering_spinner.clone(),
                renderer.clone(),
                &current_group.borrow().clone(),
                &current_section.borrow().clone(),
                max_layer,
            );
        })
    });

    max_layer_adjustment.connect_value_changed(move |_adj| {
        update_map_display_on_max_level();
    });

    window
}

fn run_window(mm_path: &Path, application: Application) {
    let window = create_main_window(mm_path, application);
    window.show();
}

fn main() {
    let application = Application::builder().build();
    application.connect_activate(|app| {
        if let Ok(mm_path) = env::var("MM_PATH") {
            let mm_path_1 = Path::new(&mm_path).to_path_buf();
            run_window(&mm_path_1, app.clone());
        } else {
            let dir_chooser = FileChooserDialog::builder()
                .application(&app.clone())
                .title("Select Magic & Mayhem directory")
                .action(FileChooserAction::SelectFolder)
                .build();

            dir_chooser.connect_response(clone!(app => move |dialog, response_type| {
                if response_type == ResponseType::Accept {
                    let chooser: FileChooser = dialog.clone().upcast();
                    let file = chooser.file().expect("get selected directory");
                    run_window(&file.path().expect("get file path"), app.clone());
                }
            }));

            dir_chooser.present();
        }
    });
    application.run();
}
