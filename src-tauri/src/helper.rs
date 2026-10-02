use crate::{
    fgrep::{self, GrepRequest},
    manager::{self, create_new_window, get_new_window_label, TabRequest, WindowLabels, DEFAULT_WINDOW_LABEL},
    menu::{self, AppMenu},
    watcher::{self, WatchTx},
    WriteFileInfo,
};
use clap::Parser;
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
use tauri::{AppHandle, Manager, WebviewWindow};

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
    pub file: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct File {
    path: String,
    column: Option<u64>,
    row: Option<u64>,
}

struct PendingArgs {
    args: Vec<Vec<String>>,
}

static STARTED: OnceLock<bool> = OnceLock::new();
static LOCALE: OnceLock<String> = OnceLock::new();
static RESTORE_POSITION: OnceLock<bool> = OnceLock::new();

pub fn start(app: &tauri::AppHandle) {
    let (tx_cmd, rx_cmd) = smol::channel::unbounded();
    app.manage(WatchTx(tx_cmd));
    watcher::spwan_watcher(app.app_handle(), rx_cmd).unwrap();
    app.manage(Mutex::new(InitArgs::default()));
    app.manage(smol::lock::Mutex::new(AppMenu::default()));
    manager::init(app);
}

pub fn handle_second_instance(app: &AppHandle, argv: Vec<String>) {
    if procced(app, &argv) {
        let label = get_new_window_label();
        let opening_file_path = setup(app, argv, &label, None);
        if !is_file_opened(app, &label, opening_file_path) {
            create_new_window(app, label)
        }
    }
}

pub fn start_setup(app: &AppHandle, argv: Vec<String>) {
    setup(app, argv, DEFAULT_WINDOW_LABEL, None);
    STARTED.get_or_init(|| true);

    if let Some(state) = app.try_state::<Mutex<PendingArgs>>() {
        let mut state = state.lock().unwrap();
        for argv in state.args.clone() {
            let label = get_new_window_label();
            let opening_file_path = setup(app, argv, &label, None);
            if !is_file_opened(app, &label, opening_file_path) {
                create_new_window(app, label);
            }
        }
        state.args.clear();
    }
}

pub fn new_window(window: &WebviewWindow, argv: Vec<String>) {
    let app = window.app_handle();
    let label = get_new_window_label();
    let opening_file_path = setup(app, argv, &label, Some(window.label()));
    if !is_file_opened(app, &label, opening_file_path) {
        create_new_window(app, label);
    }
}

fn procced(app: &AppHandle, argv: &[String]) -> bool {
    let started = STARTED.get().unwrap_or(&false);

    if *started {
        true
    } else {
        if let Some(state) = app.try_state::<Mutex<PendingArgs>>() {
            let mut state = state.lock().unwrap();
            state.args.push(argv.to_vec());
        } else {
            app.manage(Mutex::new(PendingArgs {
                args: vec![argv.to_vec()],
            }));
        }

        false
    }
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

pub fn setup(app: &AppHandle, argv: Vec<String>, label: &str, opener: Option<&str>) -> Option<String> {
    let args = Args::try_parse_from(argv).unwrap_or_default();

    let state = app.state::<Mutex<InitArgs>>();
    let mut state = state.lock().unwrap();

    let mut opening_file_path = None;

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

        state.args.insert(label.to_string(), init_args);
    } else if args.file.is_empty() {
        let init_args = new_init_arg(app, opener, None, None);
        state.args.insert(label.to_string(), init_args);
    } else {
        let file = parse_files(&args.file);
        let file_arg = FileArg {
            file_path: Some(file.path.clone()),
            start_line: file.column.map(|column| Selection {
                column,
                row: file.row.unwrap(),
            }),
            ..Default::default()
        };
        let init_args = new_init_arg(app, opener, None, Some(file_arg));
        state.args.insert(label.to_string(), init_args);
        opening_file_path = Some(file.path);
    }

    opening_file_path
}

pub fn parse_files(raw: &[String]) -> File {
    let mut iter = raw.iter().peekable();

    /* Loop as long as there are tokens left to process as filepaths */
    if let Some(path) = iter.next() {
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

        File {
            path: path.to_string(),
            column,
            row,
        }
    } else {
        File::default()
    }
}

/* Check if the file is already opened. If any, bring the windwo to front */
pub fn is_file_opened(app: &tauri::AppHandle, label: &str, opening_file_path: Option<String>) -> bool {
    if opening_file_path.is_none() {
        return false;
    }

    let opening_file_path = opening_file_path.unwrap();
    let already_opened = {
        let state = app.state::<Mutex<WindowLabels>>();
        let state = state.lock().unwrap();
        state.labels.iter().filter(|(_, title)| !title.path.is_empty() && title.path == opening_file_path).map(|(label, _)| label.to_string()).collect::<Vec<String>>()
    };
    /* If not opened, insert immediately for possible consecutive window creation */
    if already_opened.is_empty() {
        manager::add_title(app, label, "", &opening_file_path);
        return false;
    }

    let label = &already_opened[0];
    let window = app.get_webview_window(label).unwrap();
    let tab_mode = manager::is_tab_mode(app);

    if tab_mode {
        manager::on_tab_request(&window, TabRequest::Select(label.to_string()));
    } else {
        let _ = app.get_webview_window(label).unwrap().set_focus();
    }

    true
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

pub fn on_destroy(app: &tauri::AppHandle, label: &str) {
    /* Remove from Menu map too */
    menu::remove(app, label);

    manager::remove_window(app, label);
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
