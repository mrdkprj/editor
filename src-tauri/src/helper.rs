use crate::{
    fgrep::{self, GrepRequest},
    menu::{self, AppMenu},
    tab::{self, WindowMode},
    watcher::{self, WatchTx},
    WriteFileInfo,
};
use clap::Parser;
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU16, Ordering::Relaxed},
        Mutex, OnceLock,
    },
};
use tauri::{AppHandle, Manager, WebviewWindow};

static UUID: AtomicU16 = AtomicU16::new(0);
static LOCALE: OnceLock<String> = OnceLock::new();
static RESTORE_POSITION: OnceLock<bool> = OnceLock::new();
const DEFAULT_WINDOW_LABEL: &str = "View";

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
    args: HashMap<String, InitArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitArg {
    file: Option<FileArg>,
    grep: Option<GrepRequest>,
    locales: Vec<String>,
    app_data_dir: String,
    restore_position: bool,
    opener: String,
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

/* ignore_errors is required to ignore args Tauri appends */
#[derive(Parser, Debug, Default)]
#[command(ignore_errors = true)]
pub struct Args {
    #[arg(short = 'g', num_args = 3, value_names = ["condition", "start_directory", "file_type"])]
    pub grep: Option<Vec<String>>,

    #[arg(short = 'c', requires = "grep")]
    pub case_sensitive: Option<bool>,

    #[arg(short = 'm', requires = "grep")]
    pub match_by_word: Option<bool>,

    #[arg(short = 'r', requires = "grep")]
    pub regexp: Option<bool>,

    #[arg(short = 's', requires = "grep")]
    pub recursive: Option<bool>,

    /// Optional filepaths
    #[arg(num_args = 0..)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct File {
    path: String,
    column: Option<u64>,
    row: Option<u64>,
}

pub fn start(app: &tauri::AppHandle) {
    let (tx_cmd, rx_cmd) = smol::channel::unbounded();
    app.manage(WatchTx(tx_cmd));
    watcher::spwan_watcher(app.app_handle(), rx_cmd).unwrap();

    app.manage(Mutex::new(InitArgs::default()));
    app.manage(Mutex::new(CurrentTheme::default()));
    app.manage(Mutex::new(WindowLabels::default()));
    app.manage(smol::lock::Mutex::new(AppMenu::default()));
    tab::init(app, "Main");
}

pub fn handle_second_instance(app: &tauri::AppHandle, argv: Vec<String>) {
    let labels = setup(app, argv, true, None);
    on_setup(app, labels);
}

pub fn new_window(window: &WebviewWindow, argv: Vec<String>) {
    let app = window.app_handle();
    let labels = setup(app, argv, true, None);
    on_setup(app, labels);
}

fn new_init_arg(app: &AppHandle, opener: Option<&str>, grep: Option<GrepRequest>, file: Option<FileArg>) -> InitArg {
    let locale = LOCALE.get_or_init(zouni::shell::get_locale).to_string();
    let restore_position = if RESTORE_POSITION.get().is_none() {
        *RESTORE_POSITION.get_or_init(|| true)
    } else {
        false
    };

    InitArg {
        locales: vec![locale],
        restore_position,
        app_data_dir: app.path().app_data_dir().unwrap_or_default().to_string_lossy().to_string(),
        opener: opener.unwrap_or_default().to_string(),
        grep,
        file,
    }
}

pub fn setup(app: &AppHandle, argv: Vec<String>, create_window: bool, opener: Option<&str>) -> HashMap<String, Option<String>> {
    let args = Args::try_parse_from(argv).unwrap_or_default();

    let state = app.state::<Mutex<InitArgs>>();
    let mut state = state.lock().unwrap();

    let mut pairs = HashMap::new();

    if let Some(grep) = args.grep {
        let grep_request = fgrep::GrepRequest {
            condition: grep[0].to_string(),
            start_directory: grep[1].to_string(),
            file_type: grep[2].to_string(),
            match_by_word: args.match_by_word.unwrap_or_default(),
            case_sensitive: args.case_sensitive.unwrap_or_default(),
            regexp: args.regexp.unwrap_or_default(),
            recursive: args.recursive.unwrap_or_default(),
        };
        let init_args = new_init_arg(app, opener, Some(grep_request), None);
        let label = get_new_window_label(create_window);
        state.args.insert(label.clone(), init_args);
        pairs.insert(label, None);
    } else if args.files.is_empty() {
        let label = get_new_window_label(create_window);
        let init_args = new_init_arg(app, opener, None, None);
        state.args.insert(label.clone(), init_args);
        pairs.insert(label, None);
    } else {
        let files = parse_files(&args.files);
        for (i, file) in files.iter().enumerate() {
            let file_arg = FileArg {
                file_path: Some(file.path.clone()),
                start_line: file.column.map(|column| Selection {
                    column,
                    row: file.row.unwrap(),
                }),
                ..Default::default()
            };
            let init_args = new_init_arg(app, opener, None, Some(file_arg));
            let label = if i == 0 {
                get_new_window_label(create_window)
            } else {
                get_new_window_label(true)
            };
            state.args.insert(label.clone(), init_args);
            pairs.insert(label, Some(file.path.clone()));
        }
    }

    pairs.remove(DEFAULT_WINDOW_LABEL);
    pairs
}

pub fn parse_files(raw: &[String]) -> Vec<File> {
    let mut locations = Vec::new();
    let mut iter = raw.iter().peekable();

    /* Loop as long as there are tokens left to process as filepaths */
    while let Some(path) = iter.next() {
        let mut column = None;
        let mut row = None;

        /* Check if the next token can be parsed as a u64 (column) */
        if let Some(next_token) = iter.peek() {
            if let Ok(col_val) = next_token.parse::<u64>() {
                column = Some(col_val);
                iter.next();

                /* If column exists, check if the *following* token is a u64 (row) */
                if let Some(next_token_2) = iter.peek() {
                    if let Ok(row_val) = next_token_2.parse::<u64>() {
                        row = Some(row_val);
                        iter.next();
                    }
                }
            }
        }

        locations.push(File {
            path: path.to_string(),
            column,
            row,
        });
    }

    locations
}

/* Check if the file is already opened. If any, bring the windwo to front */
pub fn is_file_opened(app: &tauri::AppHandle, opening_file_path: Option<String>) -> bool {
    let opening_file_path = opening_file_path.unwrap_or_default();
    let state = app.state::<Mutex<WindowLabels>>();
    let state = state.lock().unwrap();
    let already_opened = state.labels.iter().filter(|(_, title)| !title.path.is_empty() && title.path == opening_file_path).map(|(label, _)| label).collect::<Vec<&String>>();
    if already_opened.is_empty() {
        return false;
    }

    let label = already_opened[0];
    let window = app.get_webview_window(label).unwrap();
    let tab_mode = {
        let mode = app.state::<Mutex<WindowMode>>();
        let mode = mode.lock().unwrap();
        mode.is_tab_mode()
    };

    if tab_mode {
        crate::tab::handle_request(&window, tab::TabRequest::Select(label.to_string()));
    } else {
        let _ = app.get_webview_window(label).unwrap().set_focus();
    }

    true
}

fn get_new_window_label(new_window: bool) -> String {
    if new_window {
        let id = UUID.fetch_add(1, Relaxed);
        format!("{}-{:?}", DEFAULT_WINDOW_LABEL, id)
    } else {
        DEFAULT_WINDOW_LABEL.to_string()
    }
}

pub fn on_setup(app: &AppHandle, labels: HashMap<String, Option<String>>) {
    for (label, opening_file_path) in labels {
        create_new_window(app, label, opening_file_path);
    }
}

pub fn create_new_window(app: &tauri::AppHandle, label: String, opening_file_path: Option<String>) {
    if is_file_opened(app, opening_file_path) {
        return;
    }
    let config = &app.config().app.windows[1];
    let mut config = config.clone();
    config.label = label;
    let current_theme = app.state::<Mutex<CurrentTheme>>();
    let current_theme = current_theme.lock().unwrap();
    config.theme = if current_theme.is_dark {
        Some(tauri::Theme::Dark)
    } else {
        Some(tauri::Theme::Light)
    };
    {
        let mode = app.state::<Mutex<WindowMode>>();
        let mode = mode.lock().unwrap();
        if mode.is_tab_mode() {
            config.focus = false;
        }
    }
    if cfg!(target_os = "windows") {
        /*
           Must be async for tauri::WebviewWindowBuilder::from_config.
           On Windows, this function deadlocks when used in a synchronous command or event handlers, see the Webview2 issue. You should use async commands and separate threads when creating windows.
        */
        let app = app.clone();
        std::thread::spawn(move || {
            app.clone()
                .run_on_main_thread(move || {
                    tauri::WebviewWindowBuilder::from_config(&app, &config).unwrap().build().unwrap();
                })
                .unwrap();
        })
        .join()
        .unwrap();
    } else {
        tauri::WebviewWindowBuilder::from_config(app, &config).unwrap().build().unwrap();
    }
}

pub fn create_new_host_window(app: &tauri::AppHandle) -> String {
    let id = UUID.fetch_add(1, Relaxed);
    let config = &app.config().app.windows[0];
    let mut config = config.clone();
    let label = format!("{}-{:?}", config.label, id);
    config.label = label.clone();
    let current_theme = app.state::<Mutex<CurrentTheme>>();
    let current_theme = current_theme.lock().unwrap();
    config.theme = if current_theme.is_dark {
        Some(tauri::Theme::Dark)
    } else {
        Some(tauri::Theme::Light)
    };
    tauri::WebviewWindowBuilder::from_config(app, &config).unwrap().build().unwrap();
    label
}

pub fn get_init_args(window: &WebviewWindow) -> InitArg {
    let app = window.app_handle();
    let state = app.state::<Mutex<InitArgs>>();
    let mut init_args = state.lock().unwrap();

    if let Some(mut arg) = init_args.args.remove(window.label()) {
        if let Some(file) = arg.file.as_mut() {
            if let Some(file_path) = &file.file_path {
                if let Ok(bytes) = std::fs::read(file_path) {
                    let (content, encoding) = if bytes.is_empty() {
                        (None, None)
                    } else {
                        let mut detector = chardetng::EncodingDetector::new();
                        if detector.feed(&bytes, true) {
                            let result = detector.guess(None, true).decode(&bytes);
                            (Some(result.0.to_string()), Some(result.1.name().to_string()))
                        } else {
                            (Some(unsafe { String::from_utf8_unchecked(bytes) }), None)
                        }
                    };
                    file.content = content;
                    file.encoding = encoding;
                } else {
                    /* If file read fails, ignore path argument */
                    file.file_path = None;
                }
            }
        }
        arg.clone()
    } else {
        InitArg::default()
    }
}

pub fn change_theme(app: &tauri::AppHandle, is_dark: bool) {
    let state = app.state::<Mutex<CurrentTheme>>();
    let mut state = state.lock().unwrap();
    state.is_dark = is_dark;
}

pub fn update_title(app: &tauri::AppHandle, title: WindowTitle) {
    let state = app.state::<Mutex<WindowLabels>>();
    let mut state = state.lock().unwrap();
    state.labels.insert(title.label.to_string(), title.clone());
    tab::update(app, &title.label, &title.title, &title.path);
}

pub fn remove_window(app: &tauri::AppHandle, label: &str) {
    let state = app.state::<Mutex<WindowLabels>>();
    let mut state = state.lock().unwrap();
    state.labels.remove(label);

    /* Remove from Menu map too */
    menu::remove(app, label);
    /* Remove from tab  */
    tab::remove(app, label);

    if state.labels.is_empty() {
        for (_, host) in app.webview_windows() {
            let _ = host.hide();
            let _ = host.destroy();
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

        /* Encode if not UTF-8 */
        let encoded = encoding.encode(&info.data);
        std::fs::write(info.fullPath, encoded.0).map_err(|e| e.to_string())
    } else {
        write_raw(info)
    }
}

fn write_raw(info: WriteFileInfo) -> Result<(), String> {
    std::fs::write(info.fullPath, info.data.as_bytes()).map_err(|e| e.to_string())
}
