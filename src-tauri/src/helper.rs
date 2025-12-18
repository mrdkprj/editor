use crate::{
    fgrep::{self, GrepRequest},
    session::Session,
    watcher, WatchTx, WriteFileInfo,
};
use encoding_rs::Encoding;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub fn setup(app: &mut tauri::App, args: Vec<String>) {
    let id = &app.config().identifier;
    if let Ok(session) = crate::session::start(id) {
        app.manage(session);
    }

    let (tx_cmd, rx_cmd) = tokio::sync::mpsc::unbounded_channel();
    app.manage(WatchTx(tx_cmd));
    watcher::spwan_watcher(app.app_handle(), rx_cmd).unwrap();

    if args.len() == 1 {
        app.manage(FileArg::default());
        return;
    }

    if args[1] == "-g" {
        let req = fgrep::GrepRequest {
            condition: args[2].to_string(),
            start_directory: args[3].to_string(),
            file_type: args[4].to_string(),
            match_by_word: args.contains(&"-m".to_string()),
            case_sensitive: args.contains(&"-c".to_string()),
            regexp: args.contains(&"-r".to_string()),
            recursive: args.contains(&"-s".to_string()),
        };
        app.manage(req);
    } else {
        let file = FileArg {
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
        app.manage(file);
    }
}

pub fn exit(app: &tauri::AppHandle) {
    if let Some(session) = app.try_state::<Session>() {
        crate::session::end(session.inner());
    }
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

pub fn get_init_args(app: AppHandle) -> Result<InitArgs, String> {
    let locale = zouni::shell::get_locale();
    let restore_position = app.try_state::<Session>().is_some();
    let mut args = InitArgs {
        locales: vec![locale],
        restore_position,
        app_data_dir: app.path().app_data_dir().unwrap_or_default().to_string_lossy().to_string(),
        ..Default::default()
    };

    if let Some(file_args) = app.try_state::<FileArg>() {
        let mut file = file_args.inner().clone();
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
        file.content = content;
        file.encoding = encoding;
        args.file = Some(file);
        return Ok(args);
    }

    if let Some(reg) = app.try_state::<GrepRequest>() {
        args.grep = Some(reg.inner().clone());
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
