use crate::{
    fgrep::{self, GrepRequest},
    menu::{self, AppMenu},
    watcher::{self, WatchTx},
    WriteFileInfo,
};
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicU16, Mutex, OnceLock},
};
use tauri::{AppHandle, Emitter, Manager};

static UUID: AtomicU16 = AtomicU16::new(0);
static RESTORE_POSITION: OnceLock<bool> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenedWebview {
    webviews: HashMap<String, WebviewTitle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebviewTitle {
    label: String,
    title: String,
    path: String,
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

pub fn handle_second_instance(app: &tauri::AppHandle, argv: Vec<String>) {
    let opening_file_path = setup(app, argv);
    create_new_window(app, opening_file_path);
}

pub fn start(app: &tauri::AppHandle) {
    let (tx_cmd, rx_cmd) = smol::channel::unbounded();
    app.manage(WatchTx(tx_cmd));
    watcher::spwan_watcher(app.app_handle(), rx_cmd).unwrap();

    app.manage(Mutex::new(FileArg::default()));
    app.manage(Mutex::new(fgrep::GrepRequest::default()));
    app.manage(Mutex::new(OpenedWebview::default()));
    app.manage(smol::lock::Mutex::new(AppMenu::default()));

    #[cfg(target_os = "linux")]
    {
        use gtk::traits::{BoxExt, ContainerExt, WidgetExt};

        let host = app.get_webview_window("Main").unwrap();
        let vbox = host.default_vbox().unwrap();
        let host_children = vbox.children();

        let host_webview = host_children.first().unwrap();
        host_webview.set_size_request(-1, 0);
        host_webview.set_vexpand(false);
        vbox.set_child_packing(host_webview, false, false, 0, gtk::PackType::Start);

        let overlay = gtk::Overlay::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        vbox.pack_start(&overlay, true, true, 0);
    }
}

fn update_file_arg(app: &tauri::AppHandle, file_arg: FileArg) {
    let state = app.state::<Mutex<FileArg>>();
    let mut state = state.lock().unwrap();
    *state = file_arg;
}

fn update_grep_request(app: &tauri::AppHandle, grep_request: fgrep::GrepRequest) {
    let state = app.state::<Mutex<fgrep::GrepRequest>>();
    let mut state = state.lock().unwrap();
    *state = grep_request;
}

pub fn setup(app: &tauri::AppHandle, args: Vec<String>) -> Option<String> {
    let mut opening_file = None;
    if args.len() == 1 {
        update_file_arg(app, FileArg::default());
        return opening_file;
    }

    if args[1].is_empty() {
        update_file_arg(app, FileArg::default());
    } else if args[1] == "-g" {
        let grep_request = fgrep::GrepRequest {
            condition: args[2].to_string(),
            start_directory: args[3].to_string(),
            file_type: args[4].to_string(),
            match_by_word: args.contains(&"-m".to_string()),
            case_sensitive: args.contains(&"-c".to_string()),
            regexp: args.contains(&"-r".to_string()),
            recursive: args.contains(&"-s".to_string()),
        };
        update_grep_request(app, grep_request);
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
        update_file_arg(app, file_arg);
    }

    opening_file
}

// Check if the file is already opened. If any, returns the window label
pub fn is_file_opened(app: &tauri::AppHandle, opening_file_path: Option<String>) -> Option<String> {
    let opening_file_path = opening_file_path.unwrap_or_default();
    let state = app.state::<Mutex<OpenedWebview>>();
    let state = state.lock().unwrap();
    let already_opened = state.webviews.iter().filter(|webview| !webview.1.path.is_empty() && webview.1.path == opening_file_path).map(|webview| webview.0).collect::<Vec<&String>>();
    if already_opened.is_empty() {
        None
    } else {
        Some(already_opened[0].to_string())
    }
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
    let id = UUID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let config = &app.config().app.windows[1];
    let mut config = config.clone();
    config.label = format!("{}-{:?}", config.label, id);
    tauri::WebviewWindowBuilder::from_config(app, &config).unwrap().build().unwrap();
}

pub fn get_init_args(app: AppHandle) -> Result<InitArgs, String> {
    let locale = zouni::shell::get_locale();
    let restore_position = if RESTORE_POSITION.get().is_none() {
        *RESTORE_POSITION.get_or_init(|| true)
    } else {
        false
    };
    let mut args = InitArgs {
        locales: vec![locale],
        restore_position,
        app_data_dir: app.path().app_data_dir().unwrap_or_default().to_string_lossy().to_string(),
        ..Default::default()
    };

    if let Some(file_args_state) = app.try_state::<Mutex<FileArg>>() {
        let mut file_args = file_args_state.lock().unwrap();
        let (content, encoding) = if let Some(file_path) = &file_args.file_path {
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
        file_args.content = content;
        file_args.encoding = encoding;
        args.file = Some(file_args.clone());
        return Ok(args);
    }

    if let Some(grep_request_state) = app.try_state::<Mutex<GrepRequest>>() {
        args.grep = Some(grep_request_state.lock().unwrap().clone());
        return Ok(args);
    }

    Ok(args)
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

pub fn get_webview_labels(app: tauri::AppHandle) -> OpenedWebview {
    let state = app.state::<Mutex<OpenedWebview>>();
    let state = state.lock().unwrap();
    state.clone()
}

pub fn update_webview_label(app: &tauri::AppHandle, webview_title: WebviewTitle) {
    let state = app.state::<Mutex<OpenedWebview>>();
    let mut state = state.lock().unwrap();
    let _ = state.webviews.insert(webview_title.label.clone(), webview_title);
}

pub fn remove_from_webview_label(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Mutex<OpenedWebview>>();
    let mut state = state.lock().unwrap();

    if state.webviews.is_empty() {
        return;
    }

    let _ = state.webviews.remove(label);
    /* Remove from Menu map too */
    menu::remove(app, label);

    if state.webviews.is_empty() {
        if let Some(main) = app.get_webview_window("Main") {
            let _ = main.destroy();
        }
    }
}
