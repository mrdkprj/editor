use crate::{
    fgrep::{self, GrepRequest},
    menu::{self, AppMenu},
    tab,
    watcher::{self, WatchTx},
    WriteFileInfo,
};
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU16, Ordering::Relaxed},
        Mutex, OnceLock,
    },
};
use tauri::{AppHandle, Emitter, Manager};

static UUID: AtomicU16 = AtomicU16::new(0);
static RESTORE_POSITION: OnceLock<bool> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CurrentTheme {
    is_dark: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Selection {
    pub column: u64,
    pub row: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileArg {
    pub file_path: Option<String>,
    pub content: Option<String>,
    pub encoding: Option<String>,
    pub start_line: Option<Selection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitArgs {
    file: Option<FileArg>,
    grep: Option<GrepRequest>,
    locales: Vec<String>,
    app_data_dir: String,
    restore_position: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct WindowLabels {
    pub(crate) labels: HashMap<String, WindowTitle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowTitle {
    pub label: String,
    pub title: String,
    pub path: String,
}

pub fn handle_second_instance(app: &tauri::AppHandle, argv: Vec<String>) {
    let opening_file_path = setup(app, argv);
    create_new_window(app, opening_file_path);
}

pub fn start(app: &tauri::AppHandle) {
    let (tx_cmd, rx_cmd) = smol::channel::unbounded();
    app.manage(WatchTx(tx_cmd));
    watcher::spwan_watcher(app.app_handle(), rx_cmd).unwrap();

    app.manage(Mutex::new(InitArgs::default()));
    app.manage(Mutex::new(CurrentTheme::default()));
    app.manage(Mutex::new(WindowLabels::default()));
    app.manage(smol::lock::Mutex::new(AppMenu::default()));
    tab::init(app);

    // #[cfg(target_os = "linux")]
    // {
    //     use gtk::traits::{BoxExt, ContainerExt, WidgetExt};

    //     let host = app.get_webview_window("Main").unwrap();
    //     let vbox = host.default_vbox().unwrap();
    //     let host_children = vbox.children();

    //     let host_webview = host_children.first().unwrap();
    //     host_webview.set_size_request(-1, 0);
    //     host_webview.set_vexpand(false);
    //     vbox.set_child_packing(host_webview, false, false, 0, gtk::PackType::Start);

    //     let overlay = gtk::Overlay::new();
    //     overlay.set_hexpand(true);
    //     overlay.set_vexpand(true);
    //     vbox.pack_start(&overlay, true, true, 0);
    // }
}

fn update_init_arg(app: &tauri::AppHandle, args: Option<InitArgs>) {
    let state = app.state::<Mutex<InitArgs>>();
    let mut state = state.lock().unwrap();
    if let Some(args) = args {
        *state = args;
    } else {
        *state = InitArgs::default();
    }
}

pub fn setup(app: &tauri::AppHandle, args: Vec<String>) -> Option<String> {
    /* Reset first */
    update_init_arg(app, None);

    let locale = zouni::shell::get_locale();
    let restore_position = if RESTORE_POSITION.get().is_none() {
        *RESTORE_POSITION.get_or_init(|| true)
    } else {
        false
    };

    let mut init_args = InitArgs {
        locales: vec![locale],
        restore_position,
        app_data_dir: app.path().app_data_dir().unwrap_or_default().to_string_lossy().to_string(),
        ..Default::default()
    };

    let mut opening_file = None;
    if args.len() > 1 {
        if args[1] == "-g" {
            let grep_request = fgrep::GrepRequest {
                condition: args[2].to_string(),
                start_directory: args[3].to_string(),
                file_type: args[4].to_string(),
                match_by_word: args.contains(&"-m".to_string()),
                case_sensitive: args.contains(&"-c".to_string()),
                regexp: args.contains(&"-r".to_string()),
                recursive: args.contains(&"-s".to_string()),
            };
            init_args.grep = Some(grep_request);
        } else {
            let file_arg = FileArg {
                file_path: Some(args[1].to_string()),
                content: None,
                encoding: None,
                start_line: if args.len() > 2 {
                    Some(Selection {
                        column: args[2].parse().unwrap(),
                        row: args[3].parse().unwrap(),
                    })
                } else {
                    None
                },
            };
            opening_file = Some(args[1].to_string());
            init_args.file = Some(file_arg);
        }
    }

    update_init_arg(app, Some(init_args));

    opening_file
}

/* Check if the file is already opened. If any, returns the window label */
pub fn is_file_opened(app: &tauri::AppHandle, opening_file_path: Option<String>) -> Option<String> {
    let opening_file_path = opening_file_path.unwrap_or_default();
    let state = app.state::<Mutex<WindowLabels>>();
    let state = state.lock().unwrap();
    let already_opened = state.labels.iter().filter(|(_, title)| !title.path.is_empty() && title.path == opening_file_path).map(|(label, _)| label).collect::<Vec<&String>>();
    if already_opened.is_empty() {
        None
    } else {
        Some(already_opened[0].to_string())
    }
}

pub fn change_theme(app: &tauri::AppHandle, is_dark: bool) {
    let state = app.state::<Mutex<CurrentTheme>>();
    let mut state = state.lock().unwrap();
    state.is_dark = is_dark;
}

pub fn create_new_window(app: &tauri::AppHandle, opening_file_path: Option<String>) {
    if let Some(label) = is_file_opened(app, opening_file_path) {
        app.emit_to(
            tauri::EventTarget::WebviewWindow {
                label,
            },
            "bring_to_frong",
            (),
        )
        .unwrap();
        return;
    }
    let id = UUID.fetch_add(1, Relaxed);
    let config = &app.config().app.windows[1];
    let mut config = config.clone();
    let label = format!("{}-{:?}", config.label, id);
    config.label = label;
    let current_theme = app.state::<Mutex<CurrentTheme>>();
    let current_theme = current_theme.lock().unwrap();
    config.theme = if current_theme.is_dark {
        Some(tauri::Theme::Dark)
    } else {
        Some(tauri::Theme::Light)
    };
    tauri::WebviewWindowBuilder::from_config(app, &config).unwrap().build().unwrap();
}

pub fn get_init_args(app: AppHandle) -> Result<InitArgs, String> {
    let state = app.state::<Mutex<InitArgs>>();
    let mut init_args = state.lock().unwrap();

    if let Some(file) = init_args.file.as_mut() {
        let (content, encoding) = if let Some(file_path) = &file.file_path {
            let bytes = std::fs::read(file_path).map_err(|e| e.to_string())?;
            if bytes.is_empty() {
                (None, None)
            } else {
                let mut detector = chardetng::EncodingDetector::new();
                if detector.feed(&bytes, true) {
                    let result = detector.guess(None, true).decode(&bytes);
                    (Some(result.0.to_string()), Some(result.1.name().to_string()))
                } else {
                    (Some(unsafe { String::from_utf8_unchecked(bytes) }), None)
                }
            }
        } else {
            (None, None)
        };
        file.content = content;
        file.encoding = encoding;
    }

    Ok(init_args.clone())
}

pub fn update_title(app: &tauri::AppHandle, title: WindowTitle) {
    let state = app.state::<Mutex<WindowLabels>>();
    let mut state = state.lock().unwrap();
    state.labels.insert(title.label.to_string(), title.clone());
    tab::platform_impl::update(app, &title.label, &title.title, &title.path);
}

pub fn remove_window(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Mutex<WindowLabels>>();
    let mut state = state.lock().unwrap();
    state.labels.remove(label);

    /* Remove from Menu map too */
    menu::remove(app, label);
    /* Remove from tab  */
    tab::platform_impl::remove(app, label);

    if state.labels.is_empty() {
        if let Some(main) = app.get_webview_window("Main") {
            let _ = main.hide();
            let _ = main.destroy();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReadResult {
    pub content: String,
    pub encoding: String,
}
pub fn read_to_string(file_path: &str) -> Result<ReadResult, String> {
    let bytes = std::fs::read(file_path).map_err(|e| e.to_string())?;
    let (content, encoding) = if bytes.is_empty() {
        (String::new(), encoding_rs::UTF_8.name().to_string())
    } else {
        let mut detector = chardetng::EncodingDetector::new();
        if detector.feed(&bytes, true) {
            let result = detector.guess(None, true).decode(&bytes);
            (result.0.to_string(), result.1.name().to_string())
        } else {
            (unsafe { String::from_utf8_unchecked(bytes) }, encoding_rs::UTF_8.name().to_string())
        }
    };

    Ok(ReadResult {
        content,
        encoding,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EncodeArg {
    pub file_path: String,
    pub encoding: String,
}
pub fn encode(arg: EncodeArg) -> Result<String, String> {
    if let Some(encoding) = Encoding::for_label(arg.encoding.as_bytes()) {
        let bytes = std::fs::read(&arg.file_path).map_err(|e| e.to_string())?;
        if bytes.is_empty() {
            return Ok(String::new());
        }
        let encoded = encoding.decode(&bytes).0.to_string();
        Ok(encoded)
    } else {
        Err("Requested encoding is invalid".to_string())
    }
}

pub fn write_to_file(info: WriteFileInfo) -> Result<(), String> {
    if let Some(encoding_label) = &info.encoding {
        let encoding = Encoding::for_label(encoding_label.as_bytes()).unwrap_or(encoding_rs::UTF_8);
        if encoding == encoding_rs::UTF_8 {
            return write_raw(info);
        }

        // Encode if not UTF-8
        let encoded = encoding.encode(&info.data);
        std::fs::write(info.fullPath, encoded.0).map_err(|e| e.to_string())
    } else {
        write_raw(info)
    }
}

fn write_raw(info: WriteFileInfo) -> Result<(), String> {
    std::fs::write(info.fullPath, info.data.as_bytes()).map_err(|e| e.to_string())
}
